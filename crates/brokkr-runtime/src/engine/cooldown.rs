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
use brokkr_core::fold::{fold, Cursor, Run, RunState};
use brokkr_core::{
    provider_refusal, provider_waits, EventEnvelope, ProbeAnswer, ProviderRefusal,
    ProviderRefusalError, ProviderWait, ProviderWaitError, RefusalCause,
};
use brokkr_protocol::{PreSessionRefusal, RefusalClass};
use serde_json::{json, Value};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::{Duration as TimeDuration, OffsetDateTime};

use super::{Engine, EngineError};
use crate::admission::{read_host, Bounds, HostError, Hosting};
use crate::bundle::Limits;

mod probe;
use probe::Probe;

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
/// and sleep the wait reads, and the probe's answer the retry journals.
pub(crate) struct Seam {
    pub(crate) host: Option<PathBuf>,
    pub(crate) clock: Clock,
    pub(crate) answer: Option<ProbeAnswer>,
}

impl Default for Seam {
    fn default() -> Seam {
        Seam {
            host: None,
            clock: Clock::real(),
            answer: None,
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
    Proceed(ProbeAnswer),
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
fn effect_seat<'a>(events: &'a [EventEnvelope], effect_id: &str) -> Option<&'a str> {
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
    let mut rounds = 0u64;
    let mut seconds = 0u64;
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
            rounds = rounds.saturating_add(1);
            seconds = seconds.saturating_add(wait.seconds);
        }
    }
    Ok((rounds, seconds))
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
    format!(
        "cool-down for {}/{} would wait {}s in round {} (episode {}s), past the declared {max}s \
         maximum: {}",
        wait.provider, wait.route, wait.seconds, wait.rounds, episode_seconds, wait.words
    )
}

/// The park reason a recorded refusal gives (decision 0103 slice 4c1, this
/// fix round): one function the terminal path, the cap and the resume all
/// call, so the record's text cannot drift apart from the live one. Every
/// fact it names is one the failed event carries, so a resume that never saw
/// the first park says exactly what that park said.
fn refusal_text(refused: &ProviderRefusal, episode: Option<u64>) -> String {
    let route = refused.route.as_deref().unwrap_or("an unresolved route");
    let retry = refused
        .retry_after_seconds
        .map_or_else(String::new, |retry| {
            format!(" (provider retry-after {retry}s)")
        });
    match refused.cause {
        RefusalCause::RouteUnresolved => format!(
            "provider {} refused before the first turn with a rate limit, but its route could \
             not be resolved; no cool-down is recorded because a provider-wide hold would hold \
             unrelated routes{retry}: {}",
            refused.provider, refused.words
        ),
        RefusalCause::HostUnreadable => format!(
            "refusing to retry {}/{route} without a cool-down: the host configuration could not \
             be read or is not a valid host file{retry}: {}",
            refused.provider, refused.words
        ),
        RefusalCause::EpisodeUnreadable => format!(
            "refusing to retry {}/{route}: an earlier wait does not read{retry}: {}",
            refused.provider, refused.words
        ),
        RefusalCause::PastMaximum => {
            let (seconds, max) = recorded_length(refused);
            match episode {
                Some(episode) => format!(
                    "cool-down for {}/{route} would wait {seconds}{retry} and the episode \
                     {episode}s is past the declared {max} maximum: {}",
                    refused.provider, refused.words
                ),
                None => format!(
                    "cool-down for {}/{route} would wait {seconds}{retry}, past the declared \
                     {max} maximum: {}",
                    refused.provider, refused.words
                ),
            }
        }
        RefusalCause::NoEnd => {
            let (seconds, max) = recorded_length(refused);
            let given = refused.retry_after_seconds.map_or_else(
                || seconds.clone(),
                |retry| format!("retry-after {retry}s ({seconds})"),
            );
            // The numbers as they are: the end the wire cannot spell, the
            // length computed, and the maximum declared. Only a length the
            // record itself shows past that maximum says it was passed.
            let maximum = match recorded_past(refused) {
                true => format!(", past the declared {max} maximum"),
                false => format!(", and the declared {max} maximum"),
            };
            format!(
                "cool-down for {}/{route}: {given} has no representable end; the computed wait \
                 is {seconds}{maximum}: {}",
                refused.provider, refused.words
            )
        }
    }
}

/// Whether the record's own computed length is past its declared maximum,
/// as the numbers it carries show it: the no-end arm's one comparison.
fn recorded_past(refused: &ProviderRefusal) -> bool {
    let seconds = refused
        .seconds
        .expect("a computed refusal records its seconds");
    let max = refused
        .max_seconds
        .expect("a computed refusal records its maximum");
    seconds > max
}

/// A computed length and the declared maximum as the refusal's text spells
/// them. Only a cause that reached the maximum asks for them, and the reader
/// requires both on that cause, so the expects are the record's invariant,
/// as `waiter`'s RFC 3339 expect is the wire's.
fn recorded_length(refused: &ProviderRefusal) -> (String, String) {
    let seconds = refused
        .seconds
        .expect("a computed refusal records its seconds");
    let max = refused
        .max_seconds
        .expect("a computed refusal records its maximum");
    (format!("{seconds}s"), format!("{max}s"))
}

/// The refused record a typed rate-limit refusal parks on: the facts the
/// failed event keeps, whatever the park that follows says.
fn refusal_record(
    cause: RefusalCause,
    refusal: &PreSessionRefusal,
    route: Option<&str>,
    seconds: Option<u64>,
    max: Option<u64>,
) -> ProviderRefusal {
    ProviderRefusal {
        cause,
        provider: refusal.provider().to_string(),
        route: route.map(str::to_string),
        words: refusal.words().to_string(),
        retry_after_seconds: refusal.retry_after_seconds(),
        seconds,
        max_seconds: max,
    }
}

/// Write the refusal on the failed payload the terminal event will journal.
fn record_refusal(payload: &mut Value, refused: &ProviderRefusal) {
    payload["refused"] = serde_json::to_value(refused).expect("a refusal record serializes");
}

/// The end the journal can spell: RFC 3339 carries a four-digit year, and a
/// `time` build with large dates holds years past it. An end the wire cannot
/// carry is no wait at all, the same as one `checked_add` could not form
/// (step 4's no-`until` case), so the reader's one format decides it.
fn representable_end(until: OffsetDateTime) -> Option<OffsetDateTime> {
    until.format(&Rfc3339).ok().map(|_| until)
}

/// A sealed wait, as the terminal event's JSON spells it.
fn waiter(wait: &ProviderWait, until: OffsetDateTime) -> Value {
    let mut value = json!({
        "provider": wait.provider,
        "route": wait.route,
        "words": wait.words,
        "seconds": wait.seconds,
        "until": until.format(&Rfc3339).expect("an RFC 3339 time formats as RFC 3339"),
        "rounds": wait.rounds,
    });
    if let Some(seconds) = wait.retry_after_seconds {
        value["retry_after_seconds"] = json!(seconds);
    }
    value
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

    /// The last terminal event's wait, or why a present one does not read.
    /// A single seat carries exactly one; more refuses by name.
    pub(super) fn recorded_wait(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<ProviderWait>, CooldownError> {
        let Some(event) = terminal(events, effect_id) else {
            return Ok(None);
        };
        let Some(mut waits) = provider_waits(event).map_err(CooldownError::LastWaitUnreadable)?
        else {
            return Ok(None);
        };
        match waits.len() {
            1 => Ok(waits.pop()),
            count => Err(CooldownError::TooManyWaits {
                seq: event.seq,
                count,
            }),
        }
    }

    /// The refusal the effect's last terminal event carries, or why a
    /// present record does not read. Absent is the old journal's no-refusal;
    /// a present one is never read as absent (decision 0103 slice 4c1, this
    /// fix round).
    pub(super) fn recorded_refusal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<ProviderRefusal>, CooldownError> {
        let Some(event) = terminal(events, effect_id) else {
            return Ok(None);
        };
        provider_refusal(event).map_err(CooldownError::RefusedUnreadable)
    }

    /// The reason a crash-before-the-park resume gives: the same function
    /// the terminal path used, over the episode the journal still holds.
    pub(super) fn refused_park_reason(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        refused: &ProviderRefusal,
    ) -> String {
        let episode = episode(events, effect_id).ok().map(|(_, seconds)| seconds);
        refusal_text(refused, episode)
    }

    /// The failed payload, with the wait a typed rate-limit refusal records
    /// or the park that ends the run before any retry. A non-rate-limit
    /// refusal, and one with no declared cool-down, keeps today's 0006
    /// path; the absent declaration is named in the error so the cap's own
    /// park carries it (the recommended option (a)).
    pub(super) fn rate_limit_terminal(
        &mut self,
        effect_id: &str,
        mut payload: Value,
        refusal: Option<&PreSessionRefusal>,
    ) -> Result<Terminal, EngineError> {
        let Some(refusal) = refusal.filter(|refusal| refusal.class() == RefusalClass::RateLimit)
        else {
            return Ok(Terminal::Plain(payload));
        };
        let events = self.store.load(&self.run_id)?;
        let (provider, route) = (refusal.provider(), refusal.route());
        let Some(route) = route else {
            let refused = refusal_record(RefusalCause::RouteUnresolved, refusal, None, None, None);
            record_refusal(&mut payload, &refused);
            return Ok(self.refusal_terminal(&events, effect_id, payload, refused, None));
        };
        let bounds = match self.cooldown_bounds(provider, route) {
            Ok(Some(bounds)) => bounds,
            Ok(None) => {
                let note = format!(
                    "no cool-down is declared for {provider}/{route} in {}",
                    self.host_spelling()
                );
                note_error(&mut payload, &note);
                return Ok(Terminal::Plain(payload));
            }
            Err(_) => {
                let refused = refusal_record(
                    RefusalCause::HostUnreadable,
                    refusal,
                    Some(route),
                    None,
                    None,
                );
                record_refusal(&mut payload, &refused);
                return Ok(self.refusal_terminal(&events, effect_id, payload, refused, None));
            }
        };
        Ok(self.recorded_terminal(&events, effect_id, payload, route, refusal, bounds))
    }

    /// The wait the record supports and the park it may end with, once the
    /// refusal's route and bounds are known. Kept apart from
    /// [`Engine::rate_limit_terminal`] so the reading of the journal and
    /// the park it decides each stay one small function (ruling 4).
    fn recorded_terminal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        mut payload: Value,
        route: &str,
        refusal: &PreSessionRefusal,
        bounds: Bounds,
    ) -> Terminal {
        let provider = refusal.provider();
        let (prior_rounds, prior_seconds) = match episode(events, effect_id) {
            Ok(episode) => episode,
            Err(_) => {
                let refused = refusal_record(
                    RefusalCause::EpisodeUnreadable,
                    refusal,
                    Some(route),
                    None,
                    None,
                );
                record_refusal(&mut payload, &refused);
                return self.refusal_terminal(events, effect_id, payload, refused, None);
            }
        };
        let rounds = prior_rounds.saturating_add(1);
        let retry_after = refusal.retry_after_seconds();
        let seconds = retry_after.unwrap_or_else(|| {
            u64::from(bounds.default_seconds.get())
                .saturating_mul(2u64.saturating_pow((rounds - 1).min(32) as u32))
        });
        let max = u64::from(bounds.max_seconds.get());
        let total = prior_seconds.saturating_add(seconds);
        // The maximum is judged from the computed length BEFORE any `until`
        // is formed, so no huge retry-after can overflow here (step 4).
        let past = past_maximum(seconds, total, max);
        let until = i64::try_from(seconds).ok().and_then(|seconds| {
            self.now()
                .checked_add(TimeDuration::seconds(seconds))
                .and_then(representable_end)
        });
        let wait = until.map(|until| ProviderWait {
            site: None,
            provider: provider.to_string(),
            route: route.to_string(),
            words: refusal.words().to_string(),
            retry_after_seconds: retry_after,
            seconds,
            until,
            rounds,
        });
        if let Some(wait) = &wait {
            payload["waits"] = json!([waiter(wait, wait.until)]);
        }
        // The refusal that ends in a park is written on the failed event, not
        // only on the `run/parked` after it: a crash between the two appends
        // must not turn the refusal into today's 0006 retry. A wait with no
        // `until` is exactly such a park, so it writes no `waits` entry.
        if past || wait.is_none() {
            let (cause, episode) = match wait.is_some() {
                true => (RefusalCause::PastMaximum, Some(total)),
                false => (RefusalCause::NoEnd, None),
            };
            let refused = refusal_record(cause, refusal, Some(route), Some(seconds), Some(max));
            record_refusal(&mut payload, &refused);
            return self.refusal_terminal(events, effect_id, payload, refused, episode);
        }
        Terminal::Plain(payload)
    }

    /// The terminal a refusal that would park writes. At the cap the failed
    /// event alone is journaled, so the cursor's 0006 branch writes the one
    /// park with the record's facts beside it (decision 0103 ruling 7(c));
    /// below the cap the record's own text is the park.
    fn refusal_terminal(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        payload: Value,
        refused: ProviderRefusal,
        episode: Option<u64>,
    ) -> Terminal {
        if self.at_attempt_cap(events, effect_id) {
            return Terminal::Plain(payload);
        }
        Terminal::Park {
            payload,
            reason: refusal_text(&refused, episode),
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

    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop`, then probe the cooled route. The maximum is judged from the
    /// record before anything sleeps; the remainder only sets how long to
    /// sleep. A stop already journaled is seen before the first slice and
    /// again immediately before the probe.
    pub(super) fn take_cooldown(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
    ) -> Result<Waited, EngineError> {
        match self.cooldown_bounds(&wait.provider, &wait.route) {
            Ok(Some(bounds)) => {
                let max = u64::from(bounds.max_seconds.get());
                let total = match episode(events, effect_id) {
                    Ok((_, total)) => total,
                    Err(reason) => {
                        return Ok(Waited::Parked(format!(
                            "a resume waits for {}/{} without reading its episode: {reason}",
                            wait.provider, wait.route
                        )));
                    }
                };
                if past_maximum(wait.seconds, total, max) {
                    return Ok(Waited::Parked(maximum_reason(wait, total, max)));
                }
            }
            Ok(None) => {}
            Err(reason) => {
                return Ok(Waited::Parked(format!(
                    "a resume waits for {}/{} without reading its bounds: {reason}",
                    wait.provider, wait.route
                )));
            }
        }
        // Sample BEFORE the first stop check: an event journaled between
        // the caller's fold and this sample is then still a change, and the
        // check below sees it now rather than the sample swallowing it.
        let mut seen = self.store.high_water()?;
        if self.moved_from_the_effect()? {
            return Ok(Waited::Stopped);
        }
        loop {
            let remaining = wait.until - self.now();
            if remaining <= TimeDuration::ZERO {
                break;
            }
            (self.cooldown.clock.sleep)(SLICE);
            let high = self.store.high_water()?;
            if high != seen {
                seen = high;
                if self.moved_from_the_effect()? {
                    return Ok(Waited::Stopped);
                }
            }
        }
        if self.moved_from_the_effect()? {
            return Ok(Waited::Stopped);
        }
        match self.probe(events, state, effect_id, seat, wait)? {
            Probe::Answered(answer) => Ok(Waited::Proceed(answer)),
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

    fn now(&self) -> OffsetDateTime {
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
