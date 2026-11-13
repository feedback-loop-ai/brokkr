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
use super::{
    episode, maximum_reason, past_maximum, CooldownError, Engine, EngineError, Waited, SLICE,
};

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
        self.hold_the_aggregate(events, state, effect_id, seat, waits)
    }

    /// The wait's rounds: each round waits the record's own end and the
    /// store's aggregate, probes every cooled site, then re-reads the
    /// aggregate before deciding to retry. A rate-limited refusal's next row
    /// extends the wait and the loop; the episode maximum bounds it, the
    /// remainders waited on another run's rows included (decision 0103
    /// ruling 2(a), choice 3).
    fn hold_the_aggregate(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        waits: &[ProviderWait],
    ) -> Result<Waited, EngineError> {
        let mut answers = Vec::with_capacity(waits.len());
        // Seconds waited beyond what the episode already counts: another
        // run's row has no row of this episode, so its remainder is summed
        // here rather than read back.
        let mut unrecorded = 0u64;
        // The latest own or probe-round end the episode counts. A probe's
        // row is counted once by `episode`, never again as a remainder.
        let mut counted = waits
            .iter()
            .map(|wait| wait.until)
            .max()
            .expect("a wait to take");
        loop {
            match self.wait_round(events, effect_id, waits, &mut unrecorded, counted)? {
                WaitRound::Stopped => return Ok(Waited::Stopped),
                WaitRound::Parked(reason) => return Ok(Waited::Parked(reason)),
                WaitRound::Waited => {}
            }
            match self.probe_each(events, state, effect_id, seat, waits)? {
                ProbeRound::Stopped => return Ok(Waited::Stopped),
                ProbeRound::Parked(reason) => return Ok(Waited::Parked(reason)),
                ProbeRound::Extended(until) => {
                    counted = counted.max(until);
                    continue;
                }
                ProbeRound::Answered(round) => merge_answers(&mut answers, round),
            }
            // Re-read AFTER the probes and BEFORE the retry: another run's
            // row, or one recorded while the probe ran, holds the route
            // again, so the wait and the probes repeat. The answers already
            // taken are kept; the retry's `effect/started` records the last
            // answer for each cooled site.
            match self.aggregate_in_force(waits) {
                Ok(false) => return Ok(Waited::Proceed(answers)),
                Ok(true) => continue,
                Err(error) => return Ok(Waited::Parked(error.to_string())),
            }
        }
    }

    /// One round's wait: a stop, the park reason the maximum gives, or the
    /// wait taken. `unrecorded` gains the seconds this round waits past what
    /// the episode already counts.
    pub(super) fn wait_round(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        unrecorded: &mut u64,
        counted: OffsetDateTime,
    ) -> Result<WaitRound, EngineError> {
        if self.moved_from_the_effect()? {
            return Ok(WaitRound::Stopped);
        }
        let end = match self.cool_down_end(waits) {
            Ok(end) => end,
            Err(error) => return Ok(WaitRound::Parked(error.to_string())),
        };
        let additional = beyond_the_record(counted, end, self.now());
        if let Some(reason) = self.extension_park_reason(
            events,
            effect_id,
            waits,
            unrecorded.saturating_add(additional),
        ) {
            return Ok(WaitRound::Parked(reason));
        }
        *unrecorded = unrecorded.saturating_add(additional);
        if self.wait_in_slices(end)? {
            return Ok(WaitRound::Stopped);
        }
        Ok(WaitRound::Waited)
    }

    /// Does any cooled site still stand in an aggregate at `now`: the
    /// re-read after the probes and before the retry, and a store refusal
    /// parks by its own words.
    fn aggregate_in_force(&self, waits: &[ProviderWait]) -> Result<bool, CooldownError> {
        Ok(!self.cool_down_aggregates(waits)?.is_empty())
    }

    /// One round's probes in declared order: every answer, the first
    /// extension's new end (later members wait for it), a stop, or the park
    /// reason a probe end gives.
    fn probe_each(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        waits: &[ProviderWait],
    ) -> Result<ProbeRound, EngineError> {
        let mut answers = Vec::new();
        for wait in waits {
            if self.moved_from_the_effect()? {
                return Ok(ProbeRound::Stopped);
            }
            match self.probe_wait(events, state, effect_id, seat, wait)? {
                Probe::Answered(answer) => answers.push(answer),
                Probe::Extended(until) => return Ok(ProbeRound::Extended(until)),
                Probe::Parked(reason) => return Ok(ProbeRound::Parked(reason)),
            }
        }
        Ok(ProbeRound::Answered(answers))
    }

    /// The maximum judged over the episode through this round, `extension`
    /// seconds included: the reason to park, or `None` when every cooled
    /// route may wait. The episode reads its probe rounds from the table, so
    /// a row written while a probe ran is counted; a remainder waited on
    /// another run's row is not a row of this episode and is passed in.
    pub(super) fn extension_park_reason(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        extension: u64,
    ) -> Option<String> {
        let total = match episode(&self.store, &self.run_id, events, effect_id) {
            Ok((_, total)) => total,
            Err(error) => {
                let wait = &waits[0];
                return Some(format!(
                    "a cool-down wait for {}/{} cannot read its episode: {error}",
                    wait.provider, wait.route
                ));
            }
        };
        for wait in waits {
            let max = match self.cooldown_bounds(&wait.provider, &wait.route) {
                Ok(Some(bounds)) => u64::from(bounds.max_seconds.get()),
                Ok(None) => continue,
                Err(error) => {
                    return Some(format!(
                        "a cool-down wait for {}/{} cannot read its bounds: {error}",
                        wait.provider, wait.route
                    ))
                }
            };
            let through = total.saturating_add(extension);
            if past_maximum(extension, through, max) {
                let pending = ProviderWait {
                    site: wait.site.clone(),
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    words: wait.words.clone(),
                    retry_after_seconds: None,
                    seconds: extension,
                    until: self.now(),
                    rounds: wait.rounds,
                };
                return Some(maximum_reason(&pending, through, max));
            }
        }
        None
    }

    /// The wait's end: the latest of every recorded wait's own `until` and
    /// the store's aggregate for its provider and route. A store refusal
    /// parks by its own words, never as an absent aggregate (decision 0103
    /// ruling 2(a)); a version 1, 2 or 3 journal answers none, so the
    /// event's own `until` stands.
    fn cool_down_end(&self, waits: &[ProviderWait]) -> Result<OffsetDateTime, CooldownError> {
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
    ) -> Result<Vec<CooldownAggregate>, CooldownError> {
        let mut aggregates = Vec::new();
        for wait in waits {
            match self.store.cooldown(&wait.provider, &wait.route, self.now()) {
                Ok(Some(aggregate)) => aggregates.push(aggregate),
                Ok(None) => {}
                Err(source) => {
                    return Err(CooldownError::AggregateUnreadable {
                        provider: wait.provider.clone(),
                        route: wait.route.clone(),
                        source: Box::new(source),
                    })
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

/// What one round's wait left.
pub(super) enum WaitRound {
    Waited,
    Stopped,
    Parked(String),
}

/// What one round of probes left.
enum ProbeRound {
    Answered(Vec<ProbeAnswer>),
    /// The new end a rate-limited refusal's row records.
    Extended(OffsetDateTime),
    Stopped,
    Parked(String),
}

/// The seconds from `end` that lie past what the episode already counts:
/// `counted` is the latest own or probe-round end it sums, and an instant
/// already reached is no wait at all.
fn beyond_the_record(counted: OffsetDateTime, end: OffsetDateTime, now: OffsetDateTime) -> u64 {
    (end - counted.max(now)).whole_seconds().max(0) as u64
}

/// Keep one answer per cooled site: a later round's answer replaces the
/// earlier one, so the retry's `effect/started` records the final answers
/// (decision 0103 ruling 4(a)).
fn merge_answers(answers: &mut Vec<ProbeAnswer>, round: Vec<ProbeAnswer>) {
    for answer in round {
        record_answer(answers, answer);
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
