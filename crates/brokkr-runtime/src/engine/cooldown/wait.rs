//! The engine's wait (decision 0103 slice 4c1, the aggregate and the probe's
//! extension in slice 4d2b): the maximum judged before anything sleeps, the
//! aggregate taken as the wait's end in bounded slices that honour an
//! operator `stop`, the probe of each cooled route, and a rate-limited
//! probe's next round waited and probed again before the retry.

use brokkr_core::fold::{fold, Cursor, Run, RunState};
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderWait};
use brokkr_store::CooldownAggregate;
use time::{Duration as TimeDuration, OffsetDateTime};

use super::probe::Probe;
use super::{episode, maximum_reason, past_maximum, Engine, EngineError, Waited, SLICE};

impl Engine {
    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop`, then probe the cooled route. The wait's end is the latest of
    /// each recorded wait's own `until` and the store's aggregate for its
    /// provider and route (decision 0103 ruling 2(a)); after the probes
    /// answer, the aggregate is re-read before the retry, so a row recorded
    /// while the probe ran holds the route again. The maximum is judged from
    /// the record before anything sleeps. A stop already journaled is seen
    /// before the first slice and again immediately before every probe.
    pub(in crate::engine) fn take_cooldown(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        waits: &[ProviderWait],
    ) -> Result<Waited, EngineError> {
        for wait in waits {
            if let Some(reason) = self.resume_park_reason(events, effect_id, wait) {
                return Ok(Waited::Parked(reason));
            }
        }
        let mut answers = Vec::with_capacity(waits.len());
        loop {
            if self.moved_from_the_effect()? {
                return Ok(Waited::Stopped);
            }
            let end = match self.cool_down_end(waits) {
                Ok(end) => end,
                Err(reason) => return Ok(Waited::Parked(reason)),
            };
            if self.wait_in_slices(end)? {
                return Ok(Waited::Stopped);
            }
            // A rate-limited refusal extends the wait and this round's
            // remaining probes wait for the new end: the loop is bounded by
            // the episode maximum, judged in `probe_end`, not by the cap.
            let mut extended = false;
            for wait in waits {
                if self.moved_from_the_effect()? {
                    return Ok(Waited::Stopped);
                }
                match self.probe_wait(events, state, effect_id, seat, wait)? {
                    Probe::Answered(answer) => record_answer(&mut answers, answer),
                    Probe::Extended => {
                        extended = true;
                        break;
                    }
                    Probe::Parked(reason) => return Ok(Waited::Parked(reason)),
                }
            }
            if extended {
                continue;
            }
            // Re-read AFTER the probes and BEFORE the retry: another run's
            // row, or one recorded while the probe ran, holds the route
            // again, so the wait and the probes repeat. The answers already
            // taken are kept; the retry's `effect/started` records the last
            // answer for each cooled site.
            match self.cool_down_aggregates(waits) {
                Ok(aggregates) if aggregates.is_empty() => return Ok(Waited::Proceed(answers)),
                Ok(_) => continue,
                Err(reason) => return Ok(Waited::Parked(reason)),
            }
        }
    }

    /// The wait's end: the latest of every recorded wait's own `until` and
    /// the store's aggregate for its provider and route. A store refusal
    /// parks by its own words, never as an absent aggregate (decision 0103
    /// ruling 2(a)); a version 1, 2 or 3 journal answers none, so the
    /// event's own `until` stands.
    fn cool_down_end(&self, waits: &[ProviderWait]) -> Result<OffsetDateTime, String> {
        let mut end = waits
            .iter()
            .map(|wait| wait.until)
            .max()
            .expect("a wait to take");
        for aggregate in self.cool_down_aggregates(waits)? {
            end = end.max(aggregate.until);
        }
        Ok(end)
    }

    /// Every unexpired aggregate the cooled sites stand at, at `now`, or
    /// the park reason a store refusal gives.
    fn cool_down_aggregates(
        &self,
        waits: &[ProviderWait],
    ) -> Result<Vec<CooldownAggregate>, String> {
        let mut aggregates = Vec::new();
        for wait in waits {
            match self.store.cooldown(&wait.provider, &wait.route, self.now()) {
                Ok(Some(aggregate)) => aggregates.push(aggregate),
                Ok(None) => {}
                Err(error) => {
                    return Err(format!(
                        "the cool-down aggregate for {}/{} cannot be read: {error}",
                        wait.provider, wait.route
                    ))
                }
            }
        }
        Ok(aggregates)
    }

    /// The bounds and episode the resume reads before it sleeps: the reason
    /// to park, or `None` when the wait may proceed. The maximum is judged
    /// from the record before anything sleeps; a bound or episode that does
    /// not read parks by name, never as no declaration (decision 0103
    /// ruling 3).
    fn resume_park_reason(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        wait: &ProviderWait,
    ) -> Option<String> {
        match self.cooldown_bounds(&wait.provider, &wait.route) {
            Ok(Some(bounds)) => {
                let max = u64::from(bounds.max_seconds.get());
                let total = match episode(&self.store, &self.run_id, events, effect_id) {
                    Ok((_, total)) => total,
                    Err(reason) => {
                        return Some(format!(
                            "a resume waits for {}/{} without reading its episode: {reason}",
                            wait.provider, wait.route
                        ));
                    }
                };
                past_maximum(wait.seconds, total, max).then(|| maximum_reason(wait, total, max))
            }
            Ok(None) => None,
            Err(reason) => Some(format!(
                "a resume waits for {}/{} without reading its bounds: {reason}",
                wait.provider, wait.route
            )),
        }
    }

    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop` through the journal's high-water mark. `true` is a stop:
    /// nothing may probe or retry after it.
    fn wait_in_slices(&mut self, until: OffsetDateTime) -> Result<bool, EngineError> {
        // Sample BEFORE the first stop check: an event journaled between
        // the caller's fold and this sample is then still a change, and the
        // check below sees it now rather than the sample swallowing it.
        let mut seen = self.store.high_water()?;
        if self.moved_from_the_effect()? {
            return Ok(true);
        }
        loop {
            let remaining = until - self.now();
            if remaining <= TimeDuration::ZERO {
                break;
            }
            (self.cooldown.clock.sleep)(SLICE);
            let high = self.store.high_water()?;
            if high != seen {
                seen = high;
                if self.moved_from_the_effect()? {
                    return Ok(true);
                }
            }
        }
        self.moved_from_the_effect()
    }

    /// Has the run left this effect's executable position: an operator
    /// `stop` during the wait lands as `Cursor::Stop`, and nothing may
    /// probe or retry after it.
    fn moved_from_the_effect(&self) -> Result<bool, EngineError> {
        let events = self.store.load(&self.run_id)?;
        let state = fold(&events)?;
        Ok(!matches!(
            state.run,
            Run::Running(Cursor::ExecuteEffect { .. })
        ))
    }
}

/// Keep one answer per cooled site: a later round's answer replaces the
/// earlier one, so the retry's `effect/started` records the final answers
/// (decision 0103 ruling 4(a)).
fn record_answer(answers: &mut Vec<ProbeAnswer>, answer: ProbeAnswer) {
    match answers.iter_mut().find(|seen| seen.site == answer.site) {
        Some(slot) => *slot = answer,
        None => answers.push(answer),
    }
}
