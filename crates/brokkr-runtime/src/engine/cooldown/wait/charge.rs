//! What a cool-down wait charges against its maximum (fix round 0106b1r15,
//! M1): the time it observes passing beyond what its episode's own rows
//! count, sampled on the engine's clock after every sleep, never the slice
//! it planned. The judge adds the wait it is about to take to what is
//! charged, but only an observation is ever charged, so a sleep that
//! overshoots its slice, a probe hold's contention between two polls, and a
//! start a hold refused all count, and a hold that never ends parks the run
//! at its maximum, within one slice.

use brokkr_core::{EventEnvelope, ProviderWait};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::super::held::SiteMax;
use super::super::{episode_by_site, CooldownError, Engine, Episode};

impl Engine {
    /// The effect's episode, and the latest end it counts: `counted`, or
    /// a later end of the episode's own stored rows, so a round written
    /// before this wait is never charged as time waited on another's.
    pub(super) fn counted_episode(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        counted: OffsetDateTime,
    ) -> Result<(Episode, OffsetDateTime), CooldownError> {
        let episode =
            episode_by_site(&self.store, &self.run_id, events, effect_id).map_err(|source| {
                let wait = &waits[0];
                CooldownError::WaitEpisodeUnreadable {
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    source: Box::new(source),
                }
            })?;
        let counted = counted.max(episode.latest.unwrap_or(counted));
        Ok((episode, counted))
    }

    /// `charge` carried to the sample `now`, past the end the episode
    /// counts, with nothing judged: the sample that found a wait's end
    /// passed charges the last wait's time this way.
    pub(super) fn charged_to(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &[ProviderWait],
        (counted, now): (OffsetDateTime, OffsetDateTime),
        charge: Charge,
    ) -> Result<Charge, CooldownError> {
        let (_, counted) = self.counted_episode(events, effect_id, waits, counted)?;
        Ok(charge.to(counted, now))
    }
}

/// The time a wait has charged, kept whole to the fraction, and the clock
/// sample it was charged through: `None` before the wait's first sample,
/// for the time before it is no wait this drive observed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::engine) struct Charge {
    pub(super) unrecorded: TimeDuration,
    pub(super) last: Option<OffsetDateTime>,
}

impl Charge {
    /// This charge carried to the sample `now`: the time from its last
    /// sample to `now` that lies past what the episode already counts
    /// (`counted`, the latest end of its own rows) is added, and `now` is
    /// the sample it is charged through.
    pub(super) fn to(self, counted: OffsetDateTime, now: OffsetDateTime) -> Charge {
        let observed = (self.last).map_or(TimeDuration::ZERO, |last| {
            beyond_the_record(counted, now, last)
        });
        Charge {
            unrecorded: self.unrecorded.saturating_add(observed),
            last: Some(now),
        }
    }
}

/// A wait's charge as it outlives the turn that took it: the engine's seam
/// keeps it with the waits it was charged on, so the turn a start refused
/// over a held route ends (`QueueRefusal::StartHeld`) hands the next
/// turn's wait on the same waits what was already charged, and that wait
/// charges from the same last sample. A wait on other waits starts afresh.
/// It is not journaled: a drive in a new process starts a new observation,
/// as a resume after its `until` probes rather than parks.
pub(crate) struct Carried {
    pub(super) waits: Vec<ProviderWait>,
    pub(super) charge: Charge,
}

/// One round's charge and each cooled site's declared maximum, as the
/// round's one bounds read left them.
#[derive(Debug)]
pub(in crate::engine::cooldown) struct WaitBudget {
    pub(super) charged: Charge,
    pub(super) maxima: Vec<Option<SiteMax>>,
}

/// One wait as its judge reads it: the end it waits to, the latest end the
/// episode already counts, and the one clock sample the end was read at.
#[derive(Debug, Clone, Copy)]
pub(in crate::engine::cooldown) struct Span {
    pub(in crate::engine::cooldown) end: OffsetDateTime,
    pub(in crate::engine::cooldown) counted: OffsetDateTime,
    pub(in crate::engine::cooldown) now: OffsetDateTime,
}

/// The time from `end` that lies past what the episode already counts,
/// kept to the fraction: `counted` is the latest own or probe-round end it
/// sums, and an instant already reached is no wait at all.
pub(super) fn beyond_the_record(
    counted: OffsetDateTime,
    end: OffsetDateTime,
    now: OffsetDateTime,
) -> TimeDuration {
    (end - counted.max(now)).max(TimeDuration::ZERO)
}

/// The whole seconds of `waited`, a judge's and a record's unit: the
/// fraction is dropped once, from the total, never from each slice.
pub(super) fn whole(waited: TimeDuration) -> u64 {
    waited.whole_seconds().max(0) as u64
}
