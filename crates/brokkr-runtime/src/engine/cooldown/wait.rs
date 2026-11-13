//! The engine's wait (decision 0103 slice 4c1): the maximum judged before
//! anything sleeps, the recorded remainder waited in bounded slices that
//! honour an operator `stop`, and the probe of each cooled route before the
//! retry.

use brokkr_core::fold::{fold, Cursor, Run, RunState};
use brokkr_core::{EventEnvelope, ProviderWait};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::probe::Probe;
use super::{episode, maximum_reason, past_maximum, Engine, EngineError, Waited, SLICE};

impl Engine {
    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop`, then probe the cooled route. The maximum is judged from the
    /// record before anything sleeps; the remainder only sets how long to
    /// sleep. A stop already journaled is seen before the first slice and
    /// again immediately before the probe.
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
        let latest = waits
            .iter()
            .map(|wait| wait.until)
            .max()
            .expect("a wait to take");
        if self.wait_in_slices(latest)? {
            return Ok(Waited::Stopped);
        }
        let mut answers = Vec::with_capacity(waits.len());
        for wait in waits {
            if self.moved_from_the_effect()? {
                return Ok(Waited::Stopped);
            }
            match self.probe_outcome(events, state, effect_id, seat, wait)? {
                Waited::Proceed(mut answered) => answers.append(&mut answered),
                Waited::Parked(reason) => return Ok(Waited::Parked(reason)),
                Waited::Stopped => return Ok(Waited::Stopped),
            }
        }
        Ok(Waited::Proceed(answers))
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
                let total = match episode(events, effect_id) {
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

    /// Probe the cooled route and turn its answer into what the caller does
    /// next: the retry's answer, or the park's reason.
    fn probe_outcome(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
    ) -> Result<Waited, EngineError> {
        match self.probe_wait(events, state, effect_id, seat, wait)? {
            Probe::Answered(answer) => Ok(Waited::Proceed(vec![answer])),
            Probe::Parked(reason) => Ok(Waited::Parked(reason)),
        }
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
