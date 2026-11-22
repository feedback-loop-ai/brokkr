//! A provider's pre-session rate limit becomes a cool-down (decision
//! 0103 slice 4c1, #430): the terminal event records the wait, the engine
//! waits in-process holding the run, probes the cooled route, and retries
//! only after the probe answers, inside decision 0006's attempt cap.
//!
//! The wait and the probe read `now` and sleep through [`Clock`], the seam
//! tests drive (decision 0071 ruling 1: time is an argument). The bounds
//! are the machine's own, read from the host file the launch names at
//! this one place; a file that cannot be read parks and never falls back
//! to a constant (decision 0068 ruling 3).

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use brokkr_core::envelope::EventType;
use brokkr_core::park::{Park, ParkCause, ProviderLimitRefusal};
use brokkr_core::{
    provider_waits, EventEnvelope, ProbeAnswer, ProviderRefusalError, ProviderWait,
    ProviderWaitError,
};
use serde_json::Value;
use thiserror::Error;
use time::Duration as TimeDuration;
use time::OffsetDateTime;

use super::{Engine, EngineError};
use crate::admission::{read_host, Bounds, HostError, Hosting};
use crate::bundle::Limits;

mod held;
pub(crate) use held::HeldDrive;
use held::HeldEnd;
pub use held::HeldWait;
mod probe;
pub(in crate::engine) use probe::cleanup_failure;
mod terminal;
#[cfg(test)]
use probe::Probe;
use terminal::{record_refusal, refusal_record, refusal_text, representable_end, waiter};
mod wait;

mod sites;
pub(in crate::engine) use sites::{cooldown_rows, refused_members, RefusedMember};

/// One slice of an in-process wait: the wait re-reads the run's journal
/// after each, so an operator `stop` is honoured within a second.
const SLICE: Duration = Duration::from_secs(1);

/// Time as the engine's argument (decision 0071 ruling 1). Production's
/// closure is the wall clock and a real sleep; a test's advances a shared
/// time, so no test sleeps a real cool-down.
pub(crate) struct Clock {
    pub(crate) now: Box<dyn Fn() -> OffsetDateTime + Send + Sync>,
    pub(crate) sleep: Box<dyn FnMut(Duration) + Send + Sync>,
}

impl Clock {
    pub(crate) fn real() -> Clock {
        Clock {
            now: Box::new(OffsetDateTime::now_utc),
            sleep: Box::new(std::thread::sleep),
        }
    }
}

/// The engine's cool-down seam: the host file its launch found, the clock
/// and sleep the wait reads, and the probes' answers the retry journals,
/// one per answered cooled member (decision 0103 ruling 4(a), slice 4c2a).
pub(crate) struct Seam {
    pub(crate) host: Option<PathBuf>,
    pub(crate) clock: Clock,
    pub(crate) answers: Vec<ProbeAnswer>,
    /// Where this drive stands on a held wait (decision 0106 ruling 2).
    pub(crate) held: HeldDrive,
    /// The hold the probe in flight took on its route (fix round 0106b1r4).
    pub(crate) hold: Option<brokkr_store::ProbeHoldId>,
}

impl Default for Seam {
    fn default() -> Seam {
        Seam {
            host: None,
            clock: Clock::real(),
            answers: Vec::new(),
            held: HeldDrive::Unheld,
            hold: None,
        }
    }
}

/// Why a wait cannot be read or bounded: the host file, the terminal
/// event's own wait, or an earlier wait the episode sums. Every reason the
/// cool-down path parks with is one variant's `Display`.
#[derive(Debug, Error)]
pub(super) enum CooldownError {
    #[error("the host configuration at {} cannot be read: {kind}", path.display())]
    HostUnreadable { path: PathBuf, kind: ErrorKind },
    #[error("{source}")]
    HostInvalid { source: HostError },
    #[error("{0}")]
    LastWaitUnreadable(ProviderWaitError),
    #[error("event {seq}: a single seat carries at most one wait, not {count}")]
    TooManyWaits { seq: u64, count: usize },
    #[error("an earlier wait does not read: {source}")]
    EarlierWaitUnreadable { seq: u64, source: ProviderWaitError },
    #[error("the failed event's refusal does not read: {0}")]
    RefusedUnreadable(ProviderRefusalError),
    #[error("the journal's cool-down rounds cannot be read: {source}")]
    RoundsUnreadable {
        source: Box<brokkr_store::StoreError>,
    },
    #[error("the cool-down aggregate for {provider}/{route} cannot be read: {source}")]
    AggregateUnreadable {
        provider: String,
        route: String,
        source: Box<brokkr_store::StoreError>,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again by the provider, and no \
         cool-down is declared for it: {words}"
    )]
    ProbeUndeclared {
        provider: String,
        route: String,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again, and its bounds cannot be \
         read: {source}: {words}"
    )]
    ProbeBoundsUnreadable {
        provider: String,
        route: String,
        source: Box<CooldownError>,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again, and its journal cannot be \
         read: {source}: {words}"
    )]
    ProbeJournalUnreadable {
        provider: String,
        route: String,
        source: Box<brokkr_store::StoreError>,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again with no terminal event to \
         extend: {words}"
    )]
    ProbeNoTerminal {
        provider: String,
        route: String,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again, and its rounds cannot be \
         read: {source}: {words}"
    )]
    ProbeRoundsUnreadable {
        provider: String,
        route: String,
        source: Box<brokkr_store::StoreError>,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again, and its episode cannot be \
         read: {source}: {words}"
    )]
    ProbeEpisodeUnreadable {
        provider: String,
        route: String,
        source: Box<CooldownError>,
        words: String,
    },
    #[error("a cool-down wait for {provider}/{route} cannot read its episode: {source}")]
    WaitEpisodeUnreadable {
        provider: String,
        route: String,
        source: Box<CooldownError>,
    },
    #[error("a cool-down wait for {provider}/{route} cannot read its bounds: {source}")]
    WaitBoundsUnreadable {
        provider: String,
        route: String,
        source: Box<CooldownError>,
    },
    #[error("{0}")]
    Maximum(Box<MaximumFacts>),
    #[error(
        "the cool-down probe for {provider}/{route} would write a round its wait found no \
         cool-down declared for: {words}"
    )]
    RoundUnbounded {
        provider: String,
        route: String,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again by the provider: {words}"
    )]
    RoundNoTable {
        provider: String,
        route: String,
        words: String,
    },
    #[error(
        "the cool-down probe for {provider}/{route} was refused again with a wait of {seconds}s, \
         {}, whose end the journal cannot spell",
        against(*.seconds, *.max_seconds)
    )]
    RoundNoEnd {
        provider: String,
        route: String,
        seconds: u64,
        max_seconds: u64,
    },
    #[error(
        "the cool-down probe for {provider}/{route} could not record its next round: {source}"
    )]
    RoundUnwritable {
        provider: String,
        route: String,
        source: Box<brokkr_store::StoreError>,
    },
    #[error("the journal refused the attempt's outcome: {0}")]
    WaitUnwritten(Box<brokkr_store::QueueRefusal>),
}

/// What the engine does with a wait it read: proceed to the retry, leave
/// a stopped run to its `Stop` arm, or park with this reason and cause.
pub(super) enum Waited {
    Proceed(Vec<ProbeAnswer>),
    /// A probe wrote another held round, and the drive ends with the run
    /// `running` (decision 0106 ruling 2).
    Held,
    Stopped,
    Parked(Park),
}

/// The park a cool-down error ends the run with (decision 0106 ruling 1):
/// the error's words are the reason, and its typed facts name the cause. A
/// maximum the wait or its episode passed is `provider-limit` with its
/// `past-maximum` refusal, and a bound or episode that does not read is
/// `host-unreadable` or `episode-unreadable`. Every other error is the
/// engine's own refusal: `ruled`, never retried; the probe's own refusals
/// carry the provider's words in their text, so their cause is the engine's
/// sentence and the words stay in the reason.
pub(super) fn park_of(error: &CooldownError) -> Park {
    let reason = error.to_string();
    let cause = match said(error) {
        Said::Limit(provider, refusal) => ParkCause::refusal(provider, refusal),
        Said::Probe(provider, route, what) => ParkCause::ruled(
            None,
            format!("the cool-down probe for {provider}/{route} {what}"),
        ),
        Said::Own => ParkCause::ruled(None, reason.clone()),
    };
    Park::new(reason, cause)
}

/// How a no-end round's length stands against its site's own maximum: a
/// held round is past it, and an ordinary one from a held end near year
/// 9999 is within it (fix round 0106b1r3, R3).
fn against(seconds: u64, max_seconds: u64) -> String {
    let side = if seconds > max_seconds {
        "past"
    } else {
        "within"
    };
    format!("{side} the declared {max_seconds}s maximum")
}

/// A probe's round whose end the journal cannot spell, held or ordinary,
/// as the provider's typed limit: the terminal's `no-end` cause.
fn no_end(route: &str, seconds: u64, max_seconds: u64) -> ProviderLimitRefusal {
    ProviderLimitRefusal::NoEnd {
        route: route.to_string(),
        seconds,
        max_seconds,
    }
}

/// What a cool-down error says of its park's cause.
enum Said<'a> {
    /// The provider's limit, in its typed refusal.
    Limit(&'a str, ProviderLimitRefusal),
    /// A probe's refusal, whose text carries the provider's words: the
    /// engine's sentence is built from its provider and route and what
    /// failed (decision 0106 ruling 1: the words stay in the park's reason
    /// and never in the cause).
    Probe(&'a str, &'a str, &'static str),
    /// An error whose text carries no provider's words: the reason is the
    /// engine's own sentence.
    Own,
}

/// [`Said`] for `error`, in one match that lists every variant, so a new one
/// is a compile error here, where it must say whether its text carries the
/// provider's words (decision 0071 ruling 2).
fn said(error: &CooldownError) -> Said<'_> {
    match error {
        CooldownError::Maximum(facts) => Said::Limit(
            &facts.wait.provider,
            ProviderLimitRefusal::PastMaximum {
                route: facts.wait.route.clone(),
                until: facts.wait.until,
                seconds: facts.wait.seconds,
                max_seconds: facts.max,
            },
        ),
        CooldownError::RoundNoEnd {
            provider,
            route,
            seconds,
            max_seconds,
        } => Said::Limit(provider, no_end(route, *seconds, *max_seconds)),
        CooldownError::WaitEpisodeUnreadable {
            provider, route, ..
        } => Said::Limit(
            provider,
            ProviderLimitRefusal::EpisodeUnreadable {
                route: route.clone(),
            },
        ),
        CooldownError::WaitBoundsUnreadable {
            provider, route, ..
        } => Said::Limit(
            provider,
            ProviderLimitRefusal::HostUnreadable {
                route: route.clone(),
            },
        ),
        CooldownError::ProbeUndeclared {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "was refused again, with no cool-down declared",
        ),
        CooldownError::ProbeBoundsUnreadable {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "was refused again, and its bounds cannot be read",
        ),
        CooldownError::ProbeJournalUnreadable {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "was refused again, and its journal cannot be read",
        ),
        CooldownError::ProbeNoTerminal {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "was refused again with no terminal event to extend",
        ),
        CooldownError::ProbeRoundsUnreadable {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "was refused again, and its rounds cannot be read",
        ),
        CooldownError::ProbeEpisodeUnreadable {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "was refused again, and its episode cannot be read",
        ),
        CooldownError::RoundUnbounded {
            provider, route, ..
        } => Said::Probe(
            provider,
            route,
            "would write a round with no cool-down declared",
        ),
        CooldownError::RoundNoTable {
            provider, route, ..
        } => Said::Probe(provider, route, "was refused again by the provider"),
        CooldownError::HostUnreadable { .. }
        | CooldownError::HostInvalid { .. }
        | CooldownError::LastWaitUnreadable(_)
        | CooldownError::TooManyWaits { .. }
        | CooldownError::EarlierWaitUnreadable { .. }
        | CooldownError::RefusedUnreadable(_)
        | CooldownError::RoundsUnreadable { .. }
        | CooldownError::AggregateUnreadable { .. }
        | CooldownError::RoundUnwritable { .. }
        | CooldownError::WaitUnwritten(_) => Said::Own,
    }
}

/// The terminal event a `waits` array stands on: the newest failed or
/// indeterminate one for this effect.
pub(super) fn terminal<'a>(
    events: &'a [EventEnvelope],
    effect_id: &str,
) -> Option<&'a EventEnvelope> {
    events.iter().rev().find(|event| {
        matches!(
            event.event_type,
            EventType::EffectFailed | EventType::EffectIndeterminate
        ) && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
    })
}

/// The seat the journal requested this effect for, when it names one.
pub(super) fn effect_seat<'a>(events: &'a [EventEnvelope], effect_id: &str) -> Option<&'a str> {
    events
        .iter()
        .rev()
        .find(|event| {
            event.event_type == EventType::EffectRequested
                && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
        })
        .and_then(|event| event.payload.get("seat").and_then(Value::as_str))
}

/// The episode's record through one effect: the rounds each site has
/// waited and their seconds (decision 0103 ruling 3: the sum is across
/// sites, the round is per site), and the latest instant a stored row ends.
/// The terminal events are the attempt's record; the probe rounds a
/// terminal's episode gained are read back from the table, one row per
/// round, and counted by the length the row itself records (decision 0103
/// ruling 2(a)). The terminal event's own row is not counted twice; any
/// other row is a probe round, including the fill at round one of a
/// terminal whose own row never landed. An earlier entry that does not read
/// refuses by name, never as nothing.
pub(super) struct Episode {
    pub(super) rounds: BTreeMap<Option<String>, u64>,
    pub(super) seconds: u64,
    pub(super) latest: Option<OffsetDateTime>,
    /// The latest round length each site recorded: the terminal's own
    /// wait, or the probe row that superseded it. The resume's park words
    /// read this, so a park after a written probe round names that round's
    /// length rather than the first wait's (0103 ruling 3).
    pub(super) lengths: BTreeMap<Option<String>, u64>,
    /// The probe's held round that ends last, where the episode has one
    /// (decision 0106 ruling 2). A terminal's own held wait is read from its
    /// event's `waits` entry, never from its row.
    pub(super) held: Option<HeldEnd>,
}

pub(super) fn episode_by_site(
    store: &brokkr_store::Store,
    run_id: &str,
    events: &[EventEnvelope],
    effect_id: &str,
) -> Result<Episode, CooldownError> {
    let mut episode = Episode {
        rounds: BTreeMap::new(),
        seconds: 0,
        latest: None,
        lengths: BTreeMap::new(),
        held: None,
    };
    for event in events {
        if !matches!(
            event.event_type,
            EventType::EffectFailed | EventType::EffectIndeterminate
        ) || event.payload.get("effect_id").and_then(Value::as_str) != Some(effect_id)
        {
            continue;
        }
        let Some(waits) =
            provider_waits(event).map_err(|source| CooldownError::EarlierWaitUnreadable {
                seq: event.seq,
                source,
            })?
        else {
            continue;
        };
        for wait in waits {
            *episode.rounds.entry(wait.site.clone()).or_insert(0u64) += 1;
            // A held wait contributes its round and its `until`, but no
            // remainder, and its own length is never judged against the
            // maximum (decision 0106 ruling 2).
            if wait.held.is_none() {
                episode.seconds = episode.seconds.saturating_add(wait.seconds);
                episode.lengths.insert(wait.site.clone(), wait.seconds);
            }
            episode.fold_rounds(store, run_id, event.seq, &wait)?;
        }
    }
    Ok(episode)
}

impl Episode {
    /// Fold one wait's stored rows into the episode: every row's end into
    /// `latest`, and every probe round into its site's rounds. An ordinary
    /// round adds its length; a probe's held round, the provider's word,
    /// adds its round and its end but no remainder, and its end is one a
    /// resume does not sleep to (decision 0106 ruling 2).
    fn fold_rounds(
        &mut self,
        store: &brokkr_store::Store,
        run_id: &str,
        event_seq: u64,
        wait: &ProviderWait,
    ) -> Result<(), CooldownError> {
        let site = wait.site.clone().unwrap_or_default();
        let unreadable = |source| CooldownError::RoundsUnreadable {
            source: Box::new(source),
        };
        let rows = store
            .cooldown_rounds(run_id, event_seq, &site)
            .map_err(unreadable)?;
        let kinds = store
            .cooldown_held_rounds(run_id, event_seq, &site)
            .map_err(unreadable)?;
        for row in rows {
            self.latest = Some(
                self.latest
                    .map_or(row.until_at, |latest| latest.max(row.until_at)),
            );
            if the_waits_own(&row, wait) {
                continue;
            }
            *self.rounds.entry(wait.site.clone()).or_insert(0u64) += 1;
            if let Some(held) = kinds.get(&row.rounds).copied() {
                self.hold_to(&row, held);
                continue;
            }
            let length = round_length(&row);
            self.seconds = self.seconds.saturating_add(length);
            self.lengths.insert(wait.site.clone(), length);
        }
        Ok(())
    }
}

/// Whether `row` is the terminal event's own row for `wait`: the store
/// wrote that wait's end, rounded up to its stored whole second, at the
/// wait's own round when the event landed. A probe's fill at the same round
/// ends later and is a round the episode counts.
///
/// The one-second window is exact because of the invariant its callers
/// keep: a probe runs only at or after every own `until`, and a round is at
/// least one second, so a probe's row ends at least one whole second after
/// the wait's own `until`. A row inside that window is therefore the
/// terminal's own (the store's round-up), never a fill. The removal control
/// `a_fill_after_the_own_rows_second_is_counted_as_a_round` breaks the
/// invariant (a fill inside the window) and the episode stops counting it.
fn the_waits_own(row: &brokkr_store::Cooldown, wait: &ProviderWait) -> bool {
    row.rounds == wait.rounds && row.until_at - wait.until < TimeDuration::seconds(1)
}

/// A probe round's length, read from the row's own two times: the store
/// writes `recorded_at` as the round's start and `until_at` as its end, so
/// the episode never recomputes an earlier round under a later bound.
fn round_length(row: &brokkr_store::Cooldown) -> u64 {
    (row.until_at - row.recorded_at).whole_seconds().max(0) as u64
}

/// The effect's own episode: how many waits it has recorded and how many
/// seconds they add, read from the terminal events the journal holds and
/// the probe rounds the table appended (decision 0103 ruling 2(a)). Each
/// waits contributes the `seconds` it recorded under its own route's bounds
/// (decision 0103 ruling 3); each probe round contributes the length its
/// row records, so a round written while a probe ran is counted too. An
/// earlier entry that does not read refuses by name, never as nothing.
fn episode(
    store: &brokkr_store::Store,
    run_id: &str,
    events: &[EventEnvelope],
    effect_id: &str,
) -> Result<(u64, u64), CooldownError> {
    let episode = episode_by_site(store, run_id, events, effect_id)?;
    Ok((episode.rounds.values().copied().sum(), episode.seconds))
}

/// The one maximum check the terminal event and a resume both apply: the
/// wait's own recorded length, or the episode through it, past the declared
/// maximum.
fn past_maximum(seconds: u64, episode_seconds: u64, max: u64) -> bool {
    seconds > max || episode_seconds > max
}

/// The facts a maximum park names: the round that would be written and the
/// cooled site whose maximum it passed, typed rather than prebuilt prose so
/// a test can match them and a reader reads one record (finding 6).
#[derive(Debug)]
pub(super) struct MaximumFacts {
    /// The round's own site, route, round and length, so a park names the
    /// round it refuses, not another member's (finding 5).
    pub(super) wait: ProviderWait,
    pub(super) episode: u64,
    pub(super) max: u64,
    /// The cooled site whose maximum the round passed, when that site is not
    /// the round's own: the extension judges every member.
    pub(super) passed: Option<String>,
}

impl std::fmt::Display for MaximumFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            maximum_reason(&self.wait, self.episode, self.max, self.passed.as_deref())
        )
    }
}

/// The park reason a maximum refusal gives: the provider's words and the
/// maximum, with the round and the episode the record supplies. `passed`
/// names the cooled site whose maximum the round passed when that is not the
/// round's own site, so a panel's park never reads one member's round as
/// another's (finding 5).
fn maximum_reason(
    wait: &ProviderWait,
    episode_seconds: u64,
    max: u64,
    passed: Option<&str>,
) -> String {
    let member = wait
        .site
        .as_deref()
        .map_or_else(String::new, |site| format!("member {site}: "));
    // `passed` is only ever a member's own non-empty site. The journal
    // refuses a present empty `site` (`an_empty_site_is_not_an_absent_one`),
    // and `recorded_waits` refuses a panel mixing a site-less wait with a
    // member's (`recorded_waits_refuses_an_absent_site_beside_a_member`).
    // `NextRound.site` is the probing wait's own site, so a site-less wait
    // forms a site-less round and `passed` — `Some` only where the round's
    // site and the wait's differ — stays `None`. The old emptiness guard's
    // `Some("")` therefore has no caller (fix round 430s4d2d).
    let declared = match passed {
        Some(site) => format!("member {site}'s declared {max}s maximum"),
        None => format!("the declared {max}s maximum"),
    };
    format!(
        "{member}cool-down for {}/{} would wait {}s in round {} (episode {}s), past {declared}: {}",
        wait.provider, wait.route, wait.seconds, wait.rounds, episode_seconds, wait.words
    )
}

impl Engine {
    /// The cool-down bounds one route obeys, from the host file the launch
    /// named: the block's, else the provider's, else the route's own.
    /// `None` is no declaration at all; `Err` is a file that cannot be
    /// read or is invalid, and the run parks naming it (0068's outcome).
    pub(super) fn cooldown_bounds(
        &self,
        provider: &str,
        route: &str,
    ) -> Result<Option<Bounds>, CooldownError> {
        let Some(file) = &self.cooldown.host else {
            return Ok(None);
        };
        match read_host(file) {
            Ok(Hosting::Absent) => Ok(None),
            Ok(Hosting::Unreadable(kind)) => Err(CooldownError::HostUnreadable {
                path: file.clone(),
                kind,
            }),
            Ok(Hosting::Declared(config)) => Ok(config.cooldown(provider, Some(route))),
            Err(source) => Err(CooldownError::HostInvalid { source }),
        }
    }

    /// Does this failure bring `effect_id` to its seat's attempt cap?
    fn at_attempt_cap(&self, events: &[EventEnvelope], effect_id: &str) -> bool {
        let failed = events
            .iter()
            .filter(|event| {
                event.event_type == EventType::EffectFailed
                    && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
            })
            .count() as u64;
        let max = self
            .seat_limits(effect_seat(events, effect_id))
            .max_attempts;
        failed.saturating_add(1) >= max
    }

    /// The limits the seat `seat` runs under: the bundle's own, else
    /// [`Limits::default`]. One reading, shared by the cursor's cap check
    /// and this one, so the fallback is spelled once (decision 0006).
    pub(super) fn seat_limits(&self, seat: Option<&str>) -> Limits {
        seat.and_then(|seat| self.bundle.seats.get(seat))
            .map(|seat| seat.limits)
            .unwrap_or_default()
    }

    /// The episode through `wait`, the declared maximum, and whether the
    /// wait or the episode passed it, or why an earlier wait does not read.
    /// The cap's reason and the resume path share this one reading of the
    /// record; the maximum is reported whether or not it was passed, so the
    /// cap can always name it.
    pub(super) fn wait_maximum(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        wait: &ProviderWait,
    ) -> Result<Option<(u64, u64, bool)>, CooldownError> {
        let (_, total) = episode(&self.store, &self.run_id, events, effect_id)?;
        let Some(bounds) = self.cooldown_bounds(&wait.provider, &wait.route)? else {
            return Ok(None);
        };
        let max = u64::from(bounds.max_seconds.get());
        Ok(Some((total, max, past_maximum(wait.seconds, total, max))))
    }
}

/// The failed payload with the provider's refusal and the engine's own
/// note kept apart: the provider's words stay first, and the note is what
/// the cap's 0006 park reads back.
fn note_error(payload: &mut Value, note: &str) {
    let error = payload
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("no error recorded")
        .to_string();
    payload["error"] = Value::String(format!("{error}; {note}"));
}

/// The terminal event a rate-limit refusal writes, and the park that may
/// end the run with it.
pub(super) enum Terminal {
    Plain(Value),
    Park { payload: Value, park: Park },
}

impl Terminal {
    /// Journal the failed event, and after it the park when the wait is
    /// past the declared maximum: the failed event is the attempt's
    /// record, and the park ends the run without a retry.
    pub(super) fn journal(self, engine: &mut Engine, attempt_id: &str) -> Result<(), EngineError> {
        let (payload, park) = match self {
            Terminal::Plain(payload) => (payload, None),
            Terminal::Park { payload, park } => (payload, Some(park)),
        };
        engine.append(
            EventType::EffectFailed,
            payload,
            Some(attempt_id.to_string()),
        )?;
        if let Some(park) = park {
            engine.park(&park)?;
        }
        Ok(())
    }
}

impl Engine {
    fn host_spelling(&self) -> String {
        self.cooldown.host.as_ref().map_or_else(
            || "no host file".to_string(),
            |path| path.display().to_string(),
        )
    }

    pub(super) fn now(&self) -> OffsetDateTime {
        (self.cooldown.clock.now)()
    }
}

// A test's clock: one shared instant, advanced by each slice, so a wait is
// exercised with no real sleep. The stop test builds its own sleep that
// also journals the operator's command.
#[cfg(test)]
impl Clock {
    pub(crate) fn advancing(
        start: OffsetDateTime,
    ) -> (Clock, std::sync::Arc<std::sync::Mutex<OffsetDateTime>>) {
        let shared = std::sync::Arc::new(std::sync::Mutex::new(start));
        let read = std::sync::Arc::clone(&shared);
        let write = std::sync::Arc::clone(&shared);
        (
            Clock {
                now: Box::new(move || *read.lock().unwrap()),
                sleep: Box::new(move |slice| {
                    *write.lock().unwrap() += TimeDuration::seconds(slice.as_secs() as i64)
                }),
            },
            shared,
        )
    }
}

/// The host file a launch names, or nothing where the environment cannot
/// place one: an unplaced host is no declaration, never a constant.
pub(crate) fn host_file_here() -> Option<PathBuf> {
    crate::admission::host_file(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
    .ok()
}

#[cfg(test)]
mod tests;
