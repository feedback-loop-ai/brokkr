//! A held wait's turn (decision 0106 ruling 2, slice 0106b1). A provider's
//! stated reset past the declared maximum is held, not parked: the run stays
//! `running` with its effect open, and the engine does not sleep it. The
//! drive that journals the held terminal ends there, releasing its lane and
//! process; `brokkr resume` is the wait's driver after that, and a resume
//! before the wait's end refuses by name and names the lift. At or after the
//! end the wait is taken as any other, and its probe runs on the same
//! effect.

use brokkr_core::{EventEnvelope, Held, ProviderWait};
use thiserror::Error;
use time::OffsetDateTime;

use super::probe::NextRound;
use super::{episode_by_site, CooldownError, Engine, EngineError, Episode, MaximumFacts};
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

/// The ordinary seconds a site's own maximum judges besides its own wait
/// (fix round 0106b1r4, F2): the episode's, where the site recorded an
/// ordinary length, and none where every wait it recorded is held. A held
/// site contributes no seconds, so its maximum never decides another
/// site's; the resume, the round's wait and a probe's round all judge a
/// site through this and [`bound`], as the terminal does.
pub(super) fn judged_episode(ordinary: bool, episode: u64) -> u64 {
    if ordinary {
        episode
    } else {
        0
    }
}

/// One cooled site's declared maximum as a round's wait read it, and
/// whether the site recorded an ordinary length the episode counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SiteMax {
    pub(super) max: u64,
    pub(super) ordinary: bool,
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

    /// Judge one probe's next round before anything is written: its held
    /// kind (`None` is ordinary) and its own site's maximum, or the park.
    /// The refused site's OWN route maximum classifies its reset, through
    /// the terminal's one decision ([`bound`]), before any episode sum, as
    /// each panel member's own plan classifies its own at the terminal: no
    /// other site's maximum classifies it (fix round 0106b1r3). A held round
    /// adds nothing to the episode, so nothing further judges it. An
    /// ordinary round's episode through it is then judged against every
    /// other cooled site's maximum that counts seconds of it, and passing
    /// any writes no row (finding 4, 0103 choice 3): another site's maximum
    /// bounds the shared episode, never the reset's kind, and a site whose
    /// every wait is held bounds none of it ([`judged_episode`], fix round
    /// 0106b1r4, F2). The maxima are the ones the round's wait read before
    /// the probes; a site the wait found no declaration for judges nothing,
    /// so the round parks rather than writing unjudged (finding 1).
    /// Each probe plans its round from one of these waits and copies its
    /// site. Keep the typed refusal if a caller breaks that invariant.
    pub(super) fn judge_round(
        &self,
        waits: &[ProviderWait],
        maxima: &[Option<SiteMax>],
        round: &NextRound,
        waited: u64,
    ) -> Result<(Option<Held>, u64), CooldownError> {
        let unbounded = |wait: &ProviderWait| CooldownError::RoundUnbounded {
            provider: wait.provider.clone(),
            route: wait.route.clone(),
            words: round.words.clone(),
        };
        let mut bounded = Vec::with_capacity(waits.len());
        for (wait, max) in waits.iter().zip(maxima) {
            bounded.push((wait, max.ok_or_else(|| unbounded(wait))?));
        }
        let Some(&(own_wait, SiteMax { max: own, .. })) =
            bounded.iter().find(|(wait, _)| wait.site == round.site)
        else {
            return Err(CooldownError::RoundUnbounded {
                provider: round.provider.clone(),
                route: round.route.clone(),
                words: round.words.clone(),
            });
        };
        let before = round.episode.saturating_add(waited);
        let through = match bound(round.retry_after_seconds, round.seconds, before, own) {
            Bound::Held(kind) => return Ok((Some(kind), own)),
            Bound::Past(through) => return Err(self.round_past(round, own_wait, through, own)),
            Bound::Within(through) => through,
        };
        for (wait, site) in bounded {
            let before = judged_episode(site.ordinary, through);
            if wait.site != round.site && matches!(bound(None, 0, before, site.max), Bound::Past(_))
            {
                return Err(self.round_past(round, wait, through, site.max));
            }
        }
        Ok((None, own))
    }

    /// The park a probe's round past `wait`'s maximum `max` gives: the
    /// round that would be written, the episode through it, and the cooled
    /// site whose maximum it passed where that is not the round's own.
    fn round_past(
        &self,
        round: &NextRound,
        wait: &ProviderWait,
        through: u64,
        max: u64,
    ) -> CooldownError {
        let pending = ProviderWait {
            site: round.site.clone(),
            provider: round.provider.clone(),
            route: round.route.clone(),
            words: round.words.clone(),
            retry_after_seconds: round.retry_after_seconds,
            seconds: round.seconds,
            until: self.now(),
            rounds: round.round,
            held: None,
        };
        let passed = (wait.site != round.site).then(|| wait.site.clone().unwrap_or_default());
        CooldownError::Maximum(Box::new(MaximumFacts {
            wait: pending,
            episode: through,
            max,
            passed,
        }))
    }
}
