//! A running run, advanced by its cursor (#288): [`Engine::advance_running`]
//! is the match over [`Cursor`], and each arm that does more than call one
//! method is a named helper beside it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, ParkReason, RunState};
use brokkr_core::EventEnvelope;
use serde_json::{json, Value};

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

    /// The `ExecuteEffect` arm: the seat's retry budget read, then the
    /// exhausted-retry park or the attempt itself.
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
        if failed_attempts >= limits.max_attempts {
            // Bounded retry exhausted (decision 0006): park with the
            // last recorded error.
            let last_error = last_error(events, effect_id);
            self.append(
                EventType::RunParked,
                json!({
                    "reason": format!(
                        "effect {effect_id} failed {failed_attempts} of {} attempt(s); \
                         last error: {last_error}",
                        limits.max_attempts
                    ),
                    "evidence": {},
                }),
                None,
            )?;
        } else {
            // `execute` is the one place a gate head is armed, for
            // every body shape, and an attempt that leaves it
            // through an `Err` — a checkpoint that would not
            // journal, say — appends no terminal event and so
            // never reaches the compare. The observation dies with
            // the span that took it: a stale head here would be
            // spent on the NEXT effect's terminal event, which
            // could then read a park no gate earned.
            let dispatched = self.execute(events, state, effect_id, seat);
            if dispatched.is_err() {
                self.active_gate_head = None;
            }
            dispatched?
        }
        Ok(())
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
