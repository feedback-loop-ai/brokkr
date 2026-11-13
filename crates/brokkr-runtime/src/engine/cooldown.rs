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

use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use brokkr_core::envelope::EventType;
use brokkr_core::{
    EventEnvelope, ProbeAnswer, ProviderRefusalError, ProviderWait, ProviderWaitError,
};
use serde_json::{json, Value};
use thiserror::Error;
#[cfg(test)]
use time::Duration as TimeDuration;
use time::OffsetDateTime;

use super::{Engine, EngineError};
use crate::admission::{read_host, Bounds, HostError, Hosting};
use crate::bundle::Limits;

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
}

impl Default for Seam {
    fn default() -> Seam {
        Seam {
            host: None,
            clock: Clock::real(),
            answers: Vec::new(),
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
}

/// What the engine does with a wait it read: proceed to the retry, leave
/// a stopped run to its `Stop` arm, or park with this reason.
pub(super) enum Waited {
    Proceed(Vec<ProbeAnswer>),
    Stopped,
    Parked(String),
}

/// The terminal event a `waits` array stands on: the newest failed or
/// indeterminate one for this effect.
fn terminal<'a>(events: &'a [EventEnvelope], effect_id: &str) -> Option<&'a EventEnvelope> {
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

/// The effect's own episode: how many waits it has recorded and how many
/// seconds they add, read from the terminal events the journal holds. Each
/// wait contributes the `seconds` it recorded under its own route's bounds
/// (decision 0103 ruling 3); an earlier entry that does not read refuses by
/// name, never as nothing.
fn episode(events: &[EventEnvelope], effect_id: &str) -> Result<(u64, u64), CooldownError> {
    let (sites, seconds) = sites::episode_by_site(events, effect_id)?;
    Ok((sites.values().copied().sum(), seconds))
}

/// The one maximum check the terminal event and a resume both apply: the
/// wait's own recorded length, or the episode through it, past the declared
/// maximum.
fn past_maximum(seconds: u64, episode_seconds: u64, max: u64) -> bool {
    seconds > max || episode_seconds > max
}

/// The park reason a maximum refusal gives: the provider's words and the
/// maximum, with the round and the episode the record supplies.
fn maximum_reason(wait: &ProviderWait, episode_seconds: u64, max: u64) -> String {
    let member = wait
        .site
        .as_deref()
        .map_or_else(String::new, |site| format!("member {site}: "));
    format!(
        "{member}cool-down for {}/{} would wait {}s in round {} (episode {}s), past the declared \
         {max}s maximum: {}",
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
        let (_, total) = episode(events, effect_id)?;
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
    Park { payload: Value, reason: String },
}

impl Terminal {
    /// Journal the failed event, and after it the park when the wait is
    /// past the declared maximum: the failed event is the attempt's
    /// record, and the park ends the run without a retry.
    pub(super) fn journal(self, engine: &mut Engine, attempt_id: &str) -> Result<(), EngineError> {
        let (payload, park) = match self {
            Terminal::Plain(payload) => (payload, None),
            Terminal::Park { payload, reason } => (payload, Some(reason)),
        };
        engine.append(
            EventType::EffectFailed,
            payload,
            Some(attempt_id.to_string()),
        )?;
        if let Some(reason) = park {
            engine.append(
                EventType::RunParked,
                json!({"reason": reason, "evidence": {}}),
                None,
            )?;
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
