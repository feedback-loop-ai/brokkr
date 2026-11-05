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

use std::path::PathBuf;
use std::time::Duration;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Cursor, Run, RunState};
use brokkr_core::{provider_waits, EventEnvelope, ProviderWait};
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, PreSessionRefusal, RefusalClass};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use super::{argv_for, spawn_site, Door, Engine, EngineError, Selection};
use crate::admission::{read_host, Bounds, Hosting};
use crate::bundle::{ExecutableBody, Invocation, ENGINE_VERSION};

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
    pub(crate) answer: Option<Value>,
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

/// What the engine does with a wait it read: proceed to the retry, leave
/// a stopped run to its `Stop` arm, or park with this reason.
pub(super) enum Waited {
    Proceed(Value),
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

/// The effect's own episode: how many waits it has recorded and how many
/// seconds they add, read from the terminal events the journal holds. A
/// wait's length is the entry's own formula — the provider's retry-after
/// when it gave one, else the declared default doubled by its `rounds` —
/// which is the two facts the entry carries. A malformed earlier entry is
/// skipped here; the last terminal event is the one the engine refuses by
/// name before it acts.
fn episode(events: &[EventEnvelope], effect_id: &str, bounds: Bounds) -> (u64, u64) {
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
        for wait in provider_waits(event).ok().flatten().unwrap_or_default() {
            rounds = rounds.saturating_add(1);
            seconds = seconds.saturating_add(wait_seconds(&wait, bounds));
        }
    }
    (rounds, seconds)
}

/// One wait's length under the declared bounds.
fn wait_seconds(wait: &ProviderWait, bounds: Bounds) -> u64 {
    wait.retry_after_seconds.unwrap_or_else(|| {
        u64::from(bounds.default_seconds.get())
            .saturating_mul(2u64.saturating_pow((wait.rounds - 1).min(32) as u32))
    })
}

/// A sealed wait, as the terminal event's JSON spells it.
fn waiter(wait: &ProviderWait, until: OffsetDateTime) -> Value {
    let mut value = json!({
        "provider": wait.provider,
        "route": wait.route,
        "words": wait.words,
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
    ) -> Result<Option<Bounds>, String> {
        let Some(file) = &self.cooldown.host else {
            return Ok(None);
        };
        match read_host(file) {
            Ok(Hosting::Absent) => Ok(None),
            Ok(Hosting::Unreadable(kind)) => Err(format!(
                "the host configuration at {} cannot be read: {kind}",
                file.display()
            )),
            Ok(Hosting::Declared(config)) => Ok(config.cooldown(provider, Some(route))),
            Err(error) => Err(error.to_string()),
        }
    }

    /// The last terminal event's wait, or why a present one does not read.
    /// A single seat carries exactly one; more refuses by name.
    pub(super) fn recorded_wait(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<ProviderWait>, String> {
        let Some(event) = terminal(events, effect_id) else {
            return Ok(None);
        };
        let mut waits = provider_waits(event)
            .map_err(|error| error.to_string())?
            .unwrap_or_default();
        match waits.len() {
            0 => Ok(None),
            1 => Ok(waits.pop()),
            _ => Err(format!(
                "event {}: a single seat carries at most one wait, not {}",
                event.seq,
                waits.len()
            )),
        }
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
        let (provider, route) = (refusal.provider(), refusal.route());
        let Some(route) = route else {
            return Ok(Terminal::Park {
                payload,
                reason: format!(
                    "provider {provider} refused before the first turn with a rate limit, but \
                     its route could not be resolved; no cool-down is recorded because a \
                     provider-wide hold would hold unrelated routes"
                ),
            });
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
            Err(reason) => {
                return Ok(Terminal::Park {
                    payload,
                    reason: format!(
                        "refusing to retry {provider}/{route} without a cool-down: {reason}"
                    ),
                });
            }
        };
        let events = self.store.load(&self.run_id)?;
        let (prior_rounds, prior_seconds) = episode(&events, effect_id, bounds);
        let rounds = prior_rounds.saturating_add(1);
        let seconds = refusal.retry_after_seconds().unwrap_or_else(|| {
            u64::from(bounds.default_seconds.get())
                .saturating_mul(2u64.saturating_pow((rounds - 1).min(32) as u32))
        });
        let until = self.now() + TimeDuration::seconds(seconds as i64);
        let wait = ProviderWait {
            site: None,
            provider: provider.to_string(),
            route: route.to_string(),
            words: refusal.words().to_string(),
            retry_after_seconds: refusal.retry_after_seconds(),
            until,
            rounds,
        };
        payload["waits"] = json!([waiter(&wait, until)]);
        let max = u64::from(bounds.max_seconds.get());
        let total = prior_seconds.saturating_add(seconds);
        if seconds > max || total > max {
            return Ok(Terminal::Park {
                payload,
                reason: format!(
                    "cool-down for {provider}/{route} would wait {seconds}s in round {rounds} \
                     (episode {total}s), past the declared {max}s maximum: {}",
                    refusal.words()
                ),
            });
        }
        Ok(Terminal::Plain(payload))
    }

    /// Wait the recorded remainder in bounded slices, honouring an operator
    /// `stop`, then probe the cooled route. A wait whose remainder passes
    /// the declared maximum on a resume, or whose bounds can no longer be
    /// read, parks without a retry.
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
                let remaining = (wait.until - self.now()).max(TimeDuration::ZERO);
                let max = u64::from(bounds.max_seconds.get());
                if remaining.whole_seconds().max(0) as u64 > max {
                    return Ok(Waited::Parked(format!(
                        "a resume waits {}s for {}/{}, past the declared {max}s maximum: {}",
                        remaining.whole_seconds().max(0),
                        wait.provider,
                        wait.route,
                        wait.words
                    )));
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
        let mut seen = self.store.high_water()?;
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

    /// A dedicated one-turn invocation of the cooled site's own adapter and
    /// launch path (decision 0103 ruling 4(a)): never the retried effect, at
    /// the route the entry names, with the refused candidate's model pin and
    /// effort and the same boundary and credential facts.
    fn probe(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
    ) -> Result<Probe, EngineError> {
        let seat_def = &self.bundle.seats[seat];
        let (body, case) = seat_def
            .body
            .selected(state.strategy.as_deref())
            .expect("seat input resolved the selector");
        let ExecutableBody::Invocation(Invocation::Single {
            command,
            candidates,
            ..
        }) = body
        else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: a wait is recorded at a body that is not a \
                 single seat",
                wait.provider, wait.route
            )));
        };
        let (command, candidates, timeout) = (
            command.clone(),
            candidates.clone(),
            seat_def.limits.timeout_seconds,
        );
        let gate = seat_def
            .body
            .selected_is_gate(state.strategy.as_deref(), seat_def.has_gate());
        let site_name = case
            .map(|case| format!("{seat}:{case}"))
            .unwrap_or_else(|| seat.to_string());
        let mut selection = Selection::new();
        if !candidates.is_empty() {
            let index = refused_index(events, effect_id, candidates.len());
            selection.insert(None, candidates[index].clone());
        }
        let phase = super::phase_of(state, "probe without a phase")?;
        let mut input = self.seat_input(state, &phase, effect_id)?;
        let hands = self.runtime_hands(&site_name);
        let probe_attempt = Uuid::new_v4().to_string();
        let mut spawn = self.compose_at(
            Some(&site_name),
            &probe_attempt,
            gate,
            argv_for(&selection, &None, &command).to_vec(),
            hands.as_ref(),
            selection.get(&None),
            input["result_path"].as_str().unwrap_or_default(),
        );
        self.mark_serving(
            &site_name,
            selection.get(&None),
            (&[], hands.as_ref()),
            Some(&mut spawn),
            &mut input,
        );
        let begun = self.begun(&spawn)?;
        let door = self
            .held
            .as_ref()
            .map(|held| Door::of(held, &site_name, begun.as_ref()));
        let deadline = Duration::from_secs(timeout);
        let workdir = self.workdir();
        let (process, input) =
            match spawn_site(&self.bundle, &spawn, &input, &workdir, deadline, door) {
                Ok(started) => started,
                Err(reason) => {
                    return Ok(Probe::Parked(format!(
                        "the cool-down probe for {}/{} did not spawn: {reason}",
                        wait.provider, wait.route
                    )));
                }
            };
        let report = process.probing().run_attempt_resuming(
            ENGINE_VERSION,
            effect_id,
            &probe_attempt,
            &site_name,
            input,
            None,
            |_| {},
        );
        let report = self.folded(
            [effect_id, &probe_attempt, &site_name],
            &spawn,
            begun,
            report,
        )?;
        Ok(self.probe_end(wait, report))
    }

    /// What the probe's report leaves: an answer permits the retry, and
    /// every other end parks, cleanup's refusal first (decision 0103
    /// ruling 4(a)). A typed rate-limit refusal parks in 4c1 because the
    /// table that records its next round is 4d's.
    fn probe_end(&self, wait: &ProviderWait, report: AttemptReport) -> Probe {
        if let Cleanup::Unresolved { reason } = &report.cleanup {
            return Probe::Parked(format!(
                "the cool-down probe for {}/{} did not prove its process tree gone, so it is a \
                 cleanup failure: {reason}",
                wait.provider, wait.route
            ));
        }
        if let Some(refusal) = &report.refusal {
            let said = match refusal.class() {
                RefusalClass::RateLimit => "was refused again by the provider",
                RefusalClass::Other => "was refused",
            };
            return Probe::Parked(format!(
                "the cool-down probe for {}/{} {said}: {}",
                wait.provider,
                wait.route,
                refusal.words()
            ));
        }
        match report.settled_outcome() {
            AttemptOutcome::Succeeded { .. } => match report.checkpoints.first() {
                Some(_) => Probe::Answered(json!({
                    "provider": wait.provider,
                    "route": wait.route,
                    "answered_at": self
                        .now()
                        .format(&Rfc3339)
                        .expect("an RFC 3339 time formats as RFC 3339"),
                })),
                None => Probe::Parked(format!(
                    "the cool-down probe for {}/{} reported success with no answer checkpoint",
                    wait.provider, wait.route
                )),
            },
            AttemptOutcome::Failed { error } => Probe::Parked(format!(
                "the cool-down probe for {}/{} ended without an answer: {error}",
                wait.provider, wait.route
            )),
            AttemptOutcome::Indeterminate { reason } => Probe::Parked(format!(
                "the cool-down probe for {}/{} ended without an answer: {reason}",
                wait.provider, wait.route
            )),
        }
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

/// The chain link the refused attempt used: the start failures recorded
/// before the effect's last terminal event, clamped as `chain_index` does.
fn refused_index(events: &[EventEnvelope], effect_id: &str, chain: usize) -> usize {
    let before = terminal(events, effect_id)
        .and_then(|last| events.iter().position(|event| event.seq == last.seq))
        .map_or(events, |at| &events[..at]);
    super::chain_index(before, effect_id, &None, chain)
}

/// What a probe leaves.
enum Probe {
    Answered(Value),
    Parked(String),
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
