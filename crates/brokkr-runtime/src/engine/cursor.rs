//! A running run, advanced by its cursor (#288): [`Engine::advance_running`]
//! is the match over [`Cursor`], and each arm that does more than call one
//! method is a named helper beside it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, ParkReason, RunState};
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderRefusal, ProviderWait};
use serde_json::{json, Value};

use super::cooldown::{CooldownError, Waited};
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
            self.park_with_reason(reason)?;
            return Ok(());
        }
        match self.take_cooldown(events, state, &wait_effect, seat, &waits)? {
            Waited::Proceed(answers) => self.run_attempt(events, state, effect_id, seat, answers),
            // The stop's own arm writes the ending next turn.
            Waited::Stopped => Ok(()),
            Waited::Parked(reason) => self.park_with_reason(reason),
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
            let reason = format!(
                "effect {effect_id} failed {failed_attempts} of {max_attempts} attempt(s); \
                 last error: {}",
                last_error(events, effect_id)
            );
            self.park_at_cap(events, effect_id, reason, &waits, &refused, strategy)?;
            return Ok(Recorded::Parked);
        }
        if self.park_on_refused(events, effect_id, &refused)? {
            return Ok(Recorded::Parked);
        }
        self.record_waits(events, effect_id, waits)
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
        refused: &Result<Option<ProviderRefusal>, CooldownError>,
    ) -> Result<bool, EngineError> {
        match refused {
            Ok(Some(refused)) => {
                let reason = self.refused_park_reason(events, effect_id, refused);
                self.park_with_reason(reason)?;
                Ok(true)
            }
            Ok(None) => Ok(false),
            Err(detail) => {
                let reason = format!("refusing to execute effect {effect_id}: {detail}");
                self.park_with_reason(reason)?;
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
                    let reason = format!("refusing to execute effect {effect_id}: {detail}");
                    self.park_with_reason(reason)?;
                    Ok(Recorded::Parked)
                }
            },
            Err(detail) => {
                let reason = format!("refusing to execute effect {effect_id}: {detail}");
                self.park_with_reason(reason)?;
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
    /// order (decision 0103 slice 4c2a).
    fn park_at_cap(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        mut reason: String,
        waits: &Result<Option<Vec<ProviderWait>>, CooldownError>,
        refused: &Result<Option<ProviderRefusal>, CooldownError>,
        strategy: Option<&str>,
    ) -> Result<(), EngineError> {
        reason.push_str(&self.cap_facts(events, effect_id, waits, refused, strategy));
        self.park_with_reason(reason)
    }

    /// The one `run/parked` write the refusal and wait parks share: the
    /// reason and an empty evidence envelope. `park_at_cursor` keeps its
    /// own write because a moved head carries evidence.
    pub(super) fn park_with_reason(&mut self, reason: String) -> Result<(), EngineError> {
        self.append(
            EventType::RunParked,
            json!({"reason": reason, "evidence": {}}),
            None,
        )?;
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
