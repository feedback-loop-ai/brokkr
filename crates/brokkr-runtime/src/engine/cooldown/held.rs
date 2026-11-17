//! A held wait's turn (decision 0106 ruling 2, slice 0106b1). A provider's
//! stated reset past the declared maximum is held, not parked: the run stays
//! `running` with its effect open, and the engine does not sleep it. The
//! drive that journals the held terminal ends there, releasing its lane and
//! process; `brokkr resume` is the wait's driver after that, and a resume
//! before the wait's end refuses by name and names the lift. At or after the
//! end the wait is taken as any other, and its probe runs on the same
//! effect.

use brokkr_core::envelope::EventType;
use brokkr_core::{provider_waits, ProviderWait};
use serde_json::Value;
use thiserror::Error;

use super::{Engine, EngineError};

/// Where this engine's drive stands on a held wait: none met, a held
/// terminal this drive journaled, or the turn that met it and ended the
/// drive. Only a drive that wrote the held terminal ends quietly; any other
/// turn that meets a held wait before its end is a resume, and refuses.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeldDrive {
    #[default]
    Unheld,
    Written,
    Ended,
}

/// The refusal a resume before a held wait's end gives (decision 0106
/// ruling 2): the provider's stated end, the maximum it passed, and the two
/// hands that move the run, the resume at the end and the operator's lift.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "run {run_id} is held until {until}: {provider}/{route} stated a reset past the declared \
     {max_seconds}s maximum, so nothing is parked and no attempt is spent; `brokkr resume` \
     probes at or after {until}, and the operator's lift, `brokkr operator retry --run \
     {run_id}`, ends the wait sooner"
)]
pub struct HeldWait {
    pub run_id: String,
    pub provider: String,
    pub route: String,
    /// The wait's end as RFC 3339: the latest of the recorded `until` and
    /// the provider and route's unexpired rows.
    pub until: String,
    pub max_seconds: u64,
}

/// Does this terminal payload record a held wait? The append's one reading,
/// so the drive that journals it knows to end.
pub(in crate::engine) fn holds_a_wait(event_type: EventType, payload: &Value) -> bool {
    let event = super::sites::record_event(event_type, payload);
    provider_waits(&event)
        .ok()
        .flatten()
        .is_some_and(|waits| waits.iter().any(|wait| wait.held.is_some()))
}

impl Engine {
    /// The held turn, before any wait is taken: `Ok(false)` where no
    /// recorded wait is held or the latest held end has come, so the wait
    /// is taken as any other; `Ok(true)` where this turn ended the drive
    /// that journaled the held terminal; and the typed refusal for a resume
    /// before the end.
    ///
    /// The end is the provider's own stated `until`, the latest across the
    /// held sites. The store's aggregate is not read here: the ordinary
    /// wait that follows at the end takes its rows, the second its row is
    /// rounded up to included, in bounded slices that judge the maximum, so
    /// a later row another run recorded can never make this turn sleep.
    pub(in crate::engine) fn held_turn(
        &mut self,
        waits: &[ProviderWait],
    ) -> Result<bool, EngineError> {
        let Some((wait, held)) = waits
            .iter()
            .filter_map(|wait| wait.held.map(|held| (wait, held)))
            .max_by_key(|(wait, _)| wait.until)
        else {
            return Ok(false);
        };
        let end = wait.until;
        if end <= self.now() {
            return Ok(false);
        }
        if std::mem::take(&mut self.cooldown.held) == HeldDrive::Written {
            self.cooldown.held = HeldDrive::Ended;
            return Ok(true);
        }
        Err(EngineError::HeldWait(Box::new(HeldWait {
            run_id: self.run_id.clone(),
            provider: wait.provider.clone(),
            route: wait.route.clone(),
            until: crate::admission::spelled(end),
            max_seconds: held.max_seconds,
        })))
    }
}
