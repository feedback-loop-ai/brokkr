//! A held wait's turn (decision 0106 ruling 2, slice 0106b1). A provider's
//! stated reset past the declared maximum is held, not parked: the run stays
//! `running` with its effect open, and the engine does not sleep it. The
//! drive that journals the held terminal ends there, releasing its lane and
//! process; `brokkr resume` is the wait's driver after that, and a resume
//! before the wait's end refuses by name and names the lift. At or after the
//! end the wait is taken as any other, and its probe runs on the same
//! effect.

use brokkr_core::envelope::EventType;
use brokkr_core::{provider_waits, EventEnvelope, Held, ProviderWait};
use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;

use super::{episode_by_site, Engine, EngineError, Episode};
use crate::engine::replay::Replay;
use crate::engine::DriveEnd;

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
    pub(crate) run_id: String,
    pub(crate) provider: String,
    pub(crate) route: String,
    /// The provider's stated end, the latest across the held sites, as
    /// RFC 3339.
    pub(crate) until: String,
    pub(crate) max_seconds: u64,
}

/// How one wait stands against its declared maximum (decision 0106 ruling
/// 2). The terminal's single seat, a panel's member and a probe's next
/// round all take this one decision (0071 ruling 5), so the terminal and the
/// probe cannot hold two maximum policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Bound {
    /// The provider's stated reset alone passes the maximum: the wait is
    /// held, its length is not judged, and it adds nothing to the episode.
    Held(Held),
    /// An ordinary wait within the maximum, and the episode through it.
    Within(u64),
    /// A computed wait, or the episode through an ordinary one, past the
    /// maximum: the provider's limit parks, and the episode through it.
    Past(u64),
}

impl Bound {
    /// The held kind, where the wait is held.
    pub(super) fn held(self) -> Option<Held> {
        match self {
            Bound::Held(held) => Some(held),
            Bound::Within(_) | Bound::Past(_) => None,
        }
    }
}

/// Judge a wait of `seconds`, whose provider stated `retry_after`, after an
/// episode of `before` counted seconds, against the declared `max`. A
/// stated reset that alone passes `max` is held whatever the episode before
/// it: the maximum bounds the engine's own computation, never the
/// provider's word. Every other wait parks where it, or the episode through
/// it, passes `max`.
pub(super) fn bound(retry_after: Option<u64>, seconds: u64, before: u64, max: u64) -> Bound {
    if retry_after == Some(seconds) && seconds > max {
        return Bound::Held(Held { max_seconds: max });
    }
    let through = before.saturating_add(seconds);
    if super::past_maximum(seconds, through, max) {
        Bound::Past(through)
    } else {
        Bound::Within(through)
    }
}

/// The latest end a held round of the episode stands at, and its route and
/// kind: what a resume before it refuses naming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::engine) struct HeldEnd {
    pub(super) until: OffsetDateTime,
    pub(super) provider: String,
    pub(super) route: String,
    pub(super) held: Held,
}

impl Episode {
    /// Keep the held round that ends last.
    pub(super) fn hold_to(&mut self, row: &brokkr_store::Cooldown, held: Held) {
        if self
            .held
            .as_ref()
            .is_some_and(|end| end.until >= row.until_at)
        {
            return;
        }
        self.held = Some(HeldEnd {
            until: row.until_at,
            provider: row.provider.clone(),
            route: row.route.clone(),
            held,
        });
    }
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
    /// The end of the drive whose turn met the held wait it journaled: the
    /// run still `running`, its lane and process released. `None` for every
    /// other turn, which the drive goes on from.
    pub(in crate::engine) fn held_drive_end(
        &mut self,
        replay: &mut Replay,
    ) -> Result<Option<DriveEnd>, EngineError> {
        if self.cooldown.held != HeldDrive::Ended {
            return Ok(None);
        }
        self.cooldown.held = HeldDrive::Unheld;
        let state = replay.caught_up(&mut self.store, &self.run_id)?;
        Ok(Some(DriveEnd {
            state,
            gaps: Vec::new(),
        }))
    }

    /// The held turn, before any wait is taken: `Ok(false)` where no
    /// recorded wait is held or the latest held end has come, so the wait
    /// is taken as any other; `Ok(true)` where this turn ended the drive
    /// that journaled the held terminal; and the typed refusal for a resume
    /// before the end.
    ///
    /// The end is the provider's own stated `until`, the latest across the
    /// held sites and the episode's held probe rounds (a probe's repeated
    /// held round is written to the table, not to an event). The store's
    /// aggregate is not read here: the ordinary wait that follows at the end
    /// takes its rows, the second its row is rounded up to included, in
    /// bounded slices that judge the maximum, so a later row another run
    /// recorded can never make this turn sleep. An episode that does not
    /// read names no held round here, and the wait that follows reads it
    /// again and parks by name, so nothing sleeps or retries on it.
    pub(in crate::engine) fn held_turn(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
    ) -> Result<bool, EngineError> {
        let recorded = waits.iter().filter_map(|wait| {
            wait.held.map(|held| HeldEnd {
                until: wait.until,
                provider: wait.provider.clone(),
                route: wait.route.clone(),
                held,
            })
        });
        let rounds = episode_by_site(&self.store, &self.run_id, events, effect_id)
            .ok()
            .and_then(|episode| episode.held);
        let Some(end) = recorded.chain(rounds).max_by_key(|end| end.until) else {
            return Ok(false);
        };
        if end.until <= self.now() {
            return Ok(false);
        }
        if std::mem::take(&mut self.cooldown.held) == HeldDrive::Written {
            self.cooldown.held = HeldDrive::Ended;
            return Ok(true);
        }
        Err(EngineError::HeldWait(Box::new(HeldWait {
            run_id: self.run_id.clone(),
            provider: end.provider,
            route: end.route,
            until: crate::admission::spelled(end.until),
            max_seconds: end.held.max_seconds,
        })))
    }
}
