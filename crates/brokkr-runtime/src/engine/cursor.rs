//! A running run, advanced by its cursor (#288): [`Engine::advance_running`]
//! is the match over [`Cursor`], and each arm that does more than call one
//! method is a named helper beside it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, ParkReason, RunState};
use brokkr_core::{EventEnvelope, ProbeAnswer};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

use super::cooldown::Waited;
use super::{gate_head_evidence, operator_stop_reason, Engine, EngineError};

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
            Cursor::Park { reason } => self.park_at_cursor(events, reason),
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
        let limits = self
            .bundle
            .seats
            .get(seat)
            .map(|seat| seat.limits)
            .unwrap_or_default();
        let wait = self.recorded_wait(events, effect_id);
        if failed_attempts >= limits.max_attempts {
            // Bounded retry exhausted (decision 0006): park with the last
            // recorded error, and the cool-down the last refusal recorded
            // beside it (decision 0103 ruling 7(c)). No probe, no retry.
            let mut reason = format!(
                "effect {effect_id} failed {failed_attempts} of {} attempt(s); \
                 last error: {}",
                limits.max_attempts,
                last_error(events, effect_id)
            );
            match &wait {
                Ok(Some(wait)) => {
                    reason.push_str(&format!(
                        "; the last refusal waits for {}/{} until {}: {}",
                        wait.provider,
                        wait.route,
                        wait.until
                            .format(&Rfc3339)
                            .expect("an RFC 3339 time formats as RFC 3339"),
                        wait.words
                    ));
                    // Decision 0103 ruling 7(c): the cap speaks first, and
                    // the maximum rides beside it where it has passed too.
                    match self.wait_maximum(events, effect_id, wait) {
                        Ok(Some((total, max))) => reason.push_str(&format!(
                            "; the episode {total}s is past the declared {max}s maximum"
                        )),
                        Ok(None) => {}
                        Err(detail) => reason.push_str(&format!("; {detail}")),
                    }
                }
                Err(detail) => reason.push_str(&format!("; {detail}")),
                Ok(None) => {}
            }
            self.append(
                EventType::RunParked,
                json!({"reason": reason, "evidence": {}}),
                None,
            )?;
            return Ok(());
        }
        let wait = match wait {
            Ok(Some(wait)) => wait,
            Ok(None) => return self.run_attempt(events, state, effect_id, seat, None),
            Err(detail) => {
                // A present wait that does not read refuses by name, never
                // as no wait (decision 0103 ruling 2(a)).
                self.append(
                    EventType::RunParked,
                    json!({
                        "reason": format!("refusing to execute effect {effect_id}: {detail}"),
                        "evidence": {},
                    }),
                    None,
                )?;
                return Ok(());
            }
        };
        match self.take_cooldown(events, state, effect_id, seat, &wait)? {
            Waited::Proceed(answer) => {
                self.run_attempt(events, state, effect_id, seat, Some(answer))
            }
            // The stop's own arm writes the ending next turn.
            Waited::Stopped => Ok(()),
            Waited::Parked(reason) => {
                self.append(
                    EventType::RunParked,
                    json!({"reason": reason, "evidence": {}}),
                    None,
                )?;
                Ok(())
            }
        }
    }

    /// The attempt itself, with the probe's answer (if the wait ended) set
    /// for its `effect/started` and cleared before anything else runs.
    fn run_attempt(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        answer: Option<ProbeAnswer>,
    ) -> Result<(), EngineError> {
        self.cooldown.answer = answer;
        // `execute` is the one place a gate head is armed, for every body
        // shape, and an attempt that leaves it through an `Err` — a
        // checkpoint that would not journal, say — appends no terminal
        // event and so never reaches the compare. The observation dies with
        // the span that took it: a stale head here would be spent on the
        // NEXT effect's terminal event, which could then read a park no
        // gate earned.
        let dispatched = self.execute(events, state, effect_id, seat);
        self.cooldown.answer = None;
        if dispatched.is_err() {
            self.active_gate_head = None;
        }
        dispatched
    }

    /// The `Park` arm: the parked row, with the moved head's observations
    /// as its evidence when that is the reason.
    fn park_at_cursor(
        &mut self,
        events: &[EventEnvelope],
        reason: ParkReason,
    ) -> Result<(), EngineError> {
        let evidence = match reason {
            ParkReason::GateMovedHead => gate_head_evidence(events),
            ParkReason::Stated(_) => json!({}),
        };
        self.append(
            EventType::RunParked,
            json!({"reason": reason.as_str(), "evidence": evidence}),
            None,
        )?;
        Ok(())
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
