//! A running run, advanced by its cursor (#288): [`Engine::advance_running`]
//! is the match over [`Cursor`], and each arm that does more than call one
//! method is a named helper beside it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, ParkReason, RunState};
use brokkr_core::park::{FailureClass, Park, ParkCause};
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderRefusal, ProviderWait};
use serde_json::{json, Value};

use super::cooldown::{CooldownError, Waited};
use super::non_execution::{self, LastAttempt};
use super::{gate_head_evidence, operator_stop_reason, Engine, EngineError};

/// What the record says about an effect before its attempt: the wait a
/// rate-limit refusal left to take, no record at all, or a park the record
/// already asks for.
enum Recorded {
    Wait {
        effect_id: String,
        waits: Vec<ProviderWait>,
    },
    Attempt,
    Parked,
}

impl Engine {
    pub(super) fn advance_running(
        &mut self,
        events: &[EventEnvelope],
        cursor: Cursor,
        state: RunState,
    ) -> Result<(), EngineError> {
        match cursor {
            Cursor::Start => {
                let initial = crate::bundle::entry::first_phase(&self.bundle, self.entry.as_ref())?
                    .to_string();
                self.enter_phase(&initial, &state)
            }
            Cursor::EnterPhase { phase } => self.enter_phase(&phase, &state),
            Cursor::RequestEffect => self.request_or_finish(&state),
            Cursor::ExecuteEffect {
                effect_id,
                seat,
                failed_attempts,
            } => self.execute_effect(events, &state, &effect_id, &seat, failed_attempts),
            // Fresh process, no live driver: recovered only where every
            // driver is proven settled, never re-paid (U8e2).
            Cursor::EffectInFlight {
                effect_id,
                attempt_id,
                ..
            } => self.restarted(&effect_id, &attempt_id),
            Cursor::Decide { result, .. } => self.decide(&state, result),
            Cursor::Park { reason } => self.park_at_cursor(events, &state, reason),
            Cursor::Stop => self.stop_at_cursor(events),
        }
    }

    /// The `ExecuteEffect` arm: decision 0006's cap first, then the wait
    /// and probe a recorded rate-limit refusal asks for, then the attempt.
    fn execute_effect(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        failed_attempts: u64,
    ) -> Result<(), EngineError> {
        let max_attempts = self.seat_limits(Some(seat)).max_attempts;
        let (wait_effect, waits) = match self.park_on_record(
            events,
            effect_id,
            failed_attempts,
            max_attempts,
            state.strategy.as_deref(),
        )? {
            Recorded::Wait { effect_id, waits } => (effect_id, waits),
            Recorded::Attempt => {
                return self.run_attempt(events, state, effect_id, seat, Vec::new())
            }
            Recorded::Parked => return Ok(()),
        };
        // Before the wait's probe and before the retry's admission, the
        // run's own reservation may name a crashed probe's attempt that no
        // journal event names. It is released only where every process it
        // witnessed is proven settled; otherwise the run parks with the
        // cleanup refusal naming it, spending no effect attempt and starting
        // no probe or retry (decision 0103 ruling 4, this fix round).
        if let Some(reason) =
            self.orphaned_probe_refusal(seat, state.strategy.as_deref(), events, &waits)
        {
            let cause = ParkCause::infrastructure(false, reason.clone());
            self.park(&Park::new(reason, cause))?;
            return Ok(());
        }
        match self.take_cooldown(events, state, &wait_effect, seat, &waits)? {
            Waited::Proceed(answers) => self.run_attempt(events, state, effect_id, seat, answers),
            // The stop's own arm writes the ending next turn.
            Waited::Stopped => Ok(()),
            Waited::Parked(park) => self.park(&park),
        }
    }

    /// The record's parks before any wait or attempt: the 0006 cap first,
    /// then a crash-before-the-park `refused` record, then a recorded wait
    /// that does not read. `Wait` carries the wait the cool-down takes,
    /// `Attempt` the old journal's no-wait retry, and `Parked` a park this
    /// function already wrote.
    fn park_on_record(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        failed_attempts: u64,
        max_attempts: u64,
        strategy: Option<&str>,
    ) -> Result<Recorded, EngineError> {
        let waits = self.recorded_waits(events, effect_id);
        let refused = self.recorded_refusal(events, effect_id);
        if failed_attempts >= max_attempts {
            let attempts = (failed_attempts, max_attempts);
            self.park_at_cap(events, effect_id, attempts, &waits, &refused, strategy)?;
            return Ok(Recorded::Parked);
        }
        if self.park_on_refused(events, effect_id, &waits, &refused)? {
            return Ok(Recorded::Parked);
        }
        self.record_waits(events, effect_id, waits)
    }

    /// Park an effect record that does not read, naming it: an engine
    /// refusal, never a guessed cause.
    fn park_unreadable(
        &mut self,
        effect_id: &str,
        detail: impl std::fmt::Display,
    ) -> Result<(), EngineError> {
        let reason = format!("refusing to execute effect {effect_id}: {detail}");
        self.park(&Park::ruled(reason))
    }

    /// A crash between the failed event and its park: fold returns
    /// `ExecuteEffect` only when no `run/parked` followed, so the record
    /// itself ends the run here, before any wait, probe or driver
    /// invocation (decision 0103 slice 4c1, this fix round). `true` is the
    /// park this wrote.
    fn park_on_refused(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: &Result<Option<Vec<ProviderWait>>, CooldownError>,
        refused: &Result<Option<ProviderRefusal>, CooldownError>,
    ) -> Result<bool, EngineError> {
        match refused {
            Ok(Some(refused)) => {
                let reason = self.refused_park_reason(events, effect_id, refused);
                let until = recorded_until(waits, refused);
                let cause = ParkCause::provider_limit(refused, until);
                self.park(&Park::new(reason, cause))?;
                Ok(true)
            }
            Ok(None) => Ok(false),
            Err(detail) => {
                let reason = format!("refusing to execute effect {effect_id}: {detail}");
                self.park(&Park::ruled(reason))?;
                Ok(true)
            }
        }
    }

    /// The wait the record leaves, or the operator retry's indeterminate one,
    /// or today's no-wait attempt. A present wait that does not read refuses
    /// by name, never as no wait (decision 0103 ruling 2(a)).
    ///
    /// A present `waits` array is never empty: `provider_waits` refuses one
    /// by name, the invariant `the_provider_wait_extension_refuses_by_name`
    /// is the removal control for (decision 0071 ruling 9).
    fn record_waits(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        waits: Result<Option<Vec<ProviderWait>>, CooldownError>,
    ) -> Result<Recorded, EngineError> {
        match waits {
            Ok(Some(waits)) => Ok(Recorded::Wait {
                effect_id: effect_id.to_string(),
                waits,
            }),
            Ok(None) => match self.indeterminate_waits(events, effect_id) {
                Ok(Some((waited, waits))) => Ok(Recorded::Wait {
                    effect_id: waited,
                    waits,
                }),
                Ok(None) => Ok(Recorded::Attempt),
                Err(detail) => {
                    self.park_unreadable(effect_id, detail)?;
                    Ok(Recorded::Parked)
                }
            },
            Err(detail) => {
                self.park_unreadable(effect_id, detail)?;
                Ok(Recorded::Parked)
            }
        }
    }

    /// The waits an operator retry of an indeterminate panel stands on: the
    /// newest terminal event is that panel's and it carries them, while the
    /// effect now executing has no terminal of its own yet (decision 0103
    /// ruling 7). Any other effect has no wait in force here.
    fn indeterminate_waits(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
    ) -> Result<Option<(String, Vec<ProviderWait>)>, CooldownError> {
        if events.iter().any(|event| {
            matches!(
                event.event_type,
                EventType::EffectFailed | EventType::EffectIndeterminate
            ) && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
        }) {
            return Ok(None);
        }
        let Some(last) = events.iter().rev().find(|event| {
            matches!(
                event.event_type,
                EventType::EffectSucceeded
                    | EventType::EffectFailed
                    | EventType::EffectIndeterminate
            )
        }) else {
            return Ok(None);
        };
        if last.event_type != EventType::EffectIndeterminate {
            return Ok(None);
        }
        // `payload::EffectIndeterminate` reads this field before the fold
        // places the event (`a_malformed_event_out_of_place_is_refused_for_
        // its_payload_first`), so every journal that reached this turn
        // carries it; the default keeps a journal that cannot fold from
        // inventing a wait, and is that test's removal control.
        let waited = last
            .payload
            .get("effect_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(waits) = self.recorded_waits(events, waited)? else {
            return Ok(None);
        };
        Ok((!waits.is_empty()).then(|| (waited.to_string(), waits)))
    }

    /// The run's own reservation may name a crashed probe's attempt that no
    /// journal event names. `Some` is the cleanup refusal to park with,
    /// after which nothing may probe or retry; `None` is every process it
    /// witnessed proven settled, so the wait may go on (decision 0103
    /// ruling 4, this fix round).
    fn orphaned_probe_refusal(
        &self,
        seat: &str,
        strategy: Option<&str>,
        events: &[EventEnvelope],
        waits: &[ProviderWait],
    ) -> Option<String> {
        let label = self.selected_label(seat, strategy);
        self.recovered_probe(&label, events, waits, &crate::engine::broker::observed())
            .err()
            .map(|refusal| refusal.to_string())
    }

    /// The cap's park (decision 0006), with the facts the last failure
    /// recorded beside it (decision 0103 ruling 7(c)): 0006's text speaks
    /// first, then [`Engine::cap_facts`] names each member's own in declared
    /// order (decision 0103 slice 4c2a). `attempts` is the effect's failed
    /// attempts and 0006's cap for its seat.
    ///
    /// The reason is the operator's prose, and holds the last error and
    /// each wait's and refusal's words as the provider spoke them. The cause
    /// is the engine's own sentence, built from the typed counts and the
    /// class ([`cap_cause`]): the provider's words stay in the reason and
    /// the typed `refused` record, so no readout paints them twice (decision
    /// 0106 ruling 1, "the cause does not copy them").
    fn park_at_cap(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        attempts: (u64, u64),
        waits: &Result<Option<Vec<ProviderWait>>, CooldownError>,
        refused: &Result<Option<ProviderRefusal>, CooldownError>,
        strategy: Option<&str>,
    ) -> Result<(), EngineError> {
        let (failed, max) = attempts;
        let mut reason = format!(
            "effect {effect_id} failed {failed} of {max} attempt(s); last error: {}",
            last_error(events, effect_id)
        );
        reason.push_str(&self.cap_facts(events, effect_id, waits, refused, strategy));
        let cause = cap_cause(events, effect_id, attempts);
        self.park(&Park::new(reason, cause))
    }

    /// The one `run/parked` writer (decision 0106 ruling 1): a [`Park`]
    /// states its reason and its cause together, so a writer without a cause
    /// does not compile, and [`Engine::append`] refuses a raw `run/parked`.
    /// The evidence envelope is empty; a moved head's is
    /// [`Engine::park_with_evidence`].
    pub(super) fn park(&mut self, park: &Park) -> Result<(), EngineError> {
        self.park_with_evidence(park, json!({}))
    }

    /// [`Engine::park`] with the park's evidence beside it.
    pub(super) fn park_with_evidence(
        &mut self,
        park: &Park,
        evidence: Value,
    ) -> Result<(), EngineError> {
        self.append_raw(EventType::RunParked, park.payload(evidence), None)?;
        Ok(())
    }

    /// The attempt itself, with the probe's answer (if the wait ended) set
    /// for its `effect/started` and cleared before anything else runs.
    fn run_attempt(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        answers: Vec<ProbeAnswer>,
    ) -> Result<(), EngineError> {
        // The probe answers set before an operator retry's new effect are
        // kept: that request carries no wait of its own.
        self.cooldown.answers.extend(answers);
        // `execute` is the one place a gate head is armed, for every body
        // shape, and an attempt that leaves it through an `Err` — a
        // checkpoint that would not journal, say — appends no terminal
        // event and so never reaches the compare. The observation dies with
        // the span that took it: a stale head here would be spent on the
        // NEXT effect's terminal event, which could then read a park no
        // gate earned.
        let dispatched = self.execute(events, state, effect_id, seat);
        self.cooldown.answers.clear();
        self.stderr.active = None;
        if dispatched.is_err() {
            self.active_gate_head = None;
        }
        dispatched
    }

    /// Append any park the terminal record already requires before stderr
    /// can stall the engine. A retry itself belongs to the next turn.
    pub(super) fn park_after_terminal(&mut self) -> Result<(), EngineError> {
        let mut replay = std::mem::take(&mut self.replay);
        let state = replay.caught_up(&mut self.store, &self.run_id)?;
        (self.current_cause, self.fence) = super::fence::tip_of(&replay.events);
        let result = match &state.run {
            brokkr_core::fold::Run::Running(Cursor::Park { reason }) => {
                self.park_at_cursor(&replay.events, &state, reason.clone())
            }
            brokkr_core::fold::Run::Running(Cursor::ExecuteEffect {
                effect_id,
                seat,
                failed_attempts,
            }) => {
                let max = self.seat_limits(Some(seat)).max_attempts;
                self.park_on_record(
                    &replay.events,
                    effect_id,
                    *failed_attempts,
                    max,
                    state.strategy.as_deref(),
                )
                .map(drop)
            }
            _ => Ok(()),
        };
        self.replay = replay;
        result
    }

    /// The `Park` arm: the parked row, with the moved head's observations
    /// as its evidence when that is the reason.
    fn park_at_cursor(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        reason: ParkReason,
    ) -> Result<(), EngineError> {
        let evidence = match reason {
            ParkReason::GateMovedHead => gate_head_evidence(events),
            ParkReason::Stated(_) => json!({}),
        };
        let cause = match reason {
            // The moved head arrives through `effect/indeterminate`, so no
            // rule matched: `state.last_decision` is the ruling that routed
            // INTO the gate, not this park's.
            ParkReason::GateMovedHead => ParkCause::ruled(None, reason.as_str()),
            ParkReason::Stated(_) if closed_indeterminate(events) => {
                ParkCause::infrastructure(false, reason.as_str())
            }
            ParkReason::Stated(_) => ParkCause::ruled(rule_id(state), reason.as_str()),
        };
        self.park_with_evidence(&Park::new(reason.as_str(), cause), evidence)
    }

    /// The `Stop` arm: the stopped row, with the operator's reason.
    fn stop_at_cursor(&mut self, events: &[EventEnvelope]) -> Result<(), EngineError> {
        self.append(
            EventType::RunStopped,
            json!({"reason": operator_stop_reason(events)}),
            None,
        )?;
        Ok(())
    }
}

/// The rule id the run's last ruling matched, where it named one.
fn rule_id(state: &RunState) -> Option<String> {
    state
        .last_decision
        .as_ref()
        .and_then(|decision| decision.rule_id.present())
        .map(|rule| rule.to_string())
}

/// Whether the park was opened by an indeterminate close: the fold's
/// `Cursor::Park` follows the `effect/indeterminate` that reached it. An
/// operator command or its receipt may land between the close and the park
/// and moves nothing, so the close is the newest event that is neither.
fn closed_indeterminate(events: &[EventEnvelope]) -> bool {
    events
        .iter()
        .rev()
        .find(|event| {
            !matches!(
                event.event_type,
                EventType::OperatorCommanded
                    | EventType::OperatorAccepted
                    | EventType::OperatorRejected
            )
        })
        .is_some_and(|event| event.event_type == EventType::EffectIndeterminate)
}

/// The recorded end a `past-maximum` refusal's wait carries, where the
/// failed event still holds it.
fn recorded_until(
    waits: &Result<Option<Vec<ProviderWait>>, CooldownError>,
    refused: &ProviderRefusal,
) -> Option<time::OffsetDateTime> {
    let waits = waits.as_ref().ok()?.as_ref()?;
    waits
        .iter()
        .find(|wait| {
            wait.provider == refused.provider && Some(&wait.route) == refused.route.as_ref()
        })
        .map(|wait| wait.until)
}

/// The cap's cause (decision 0106 ruling 1), from [`non_execution`]'s one
/// reading of the effect's last terminal event, never from raw payload keys
/// read a second time here: `park_at_cap`'s typed `waits` and `refused` are
/// read from that same terminal, so they are the same fact.
///
/// A recorded `waits` or `refused`, readable or not, is a typed rate-limit
/// refusal: `provider-limit`. Otherwise a failure to start with no
/// checkpoint of the attempt is `start-failure`, and any failure that may
/// have executed is `determinate`. An indeterminate last terminal is not a
/// failure class at all: ruling 1's table gives an indeterminate end to
/// `infrastructure`, and nothing proves it never executed, so it is not
/// retryable. [`FailureClass::ProviderRefusal`] has no writer until 0106e
/// (#1037) types a non-rate-limit refusal; it is reserved, not dead.
///
/// Its detail is the engine's own sentence: 0006's cap as the counts say it,
/// and the class. It carries none of the seat's error and none of the
/// provider's words, which only the park's reason holds.
fn cap_cause(events: &[EventEnvelope], effect_id: &str, (failed, max): (u64, u64)) -> ParkCause {
    let cap = format!("effect {effect_id} failed {failed} of {max} attempt(s)");
    let class = match non_execution::last_attempt(events, effect_id) {
        Some(LastAttempt::Indeterminate) => {
            let detail = format!("{cap}; the last attempt ended indeterminate");
            return ParkCause::infrastructure(false, detail);
        }
        Some(LastAttempt::Refused) => FailureClass::ProviderLimit,
        Some(LastAttempt::FailedToStart) => FailureClass::StartFailure,
        Some(LastAttempt::Failed) | None => FailureClass::Determinate,
    };
    let detail = format!("{cap}; the last failure class is {}", class.word());
    ParkCause::attempt_cap(class, detail)
}

/// The error the effect's last `EffectFailed` recorded, read from the
/// newest event back, or "no error recorded".
fn last_error(events: &[EventEnvelope], effect_id: &str) -> String {
    events
        .iter()
        .rev()
        .find(|event| {
            event.event_type == EventType::EffectFailed
                && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
        })
        .and_then(|event| event.payload.get("error").and_then(Value::as_str))
        .unwrap_or("no error recorded")
        .to_string()
}
