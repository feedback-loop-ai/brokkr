//! A running run, advanced by its cursor (#288): [`Engine::advance_running`]
//! is the match over [`Cursor`], and each arm that does more than call one
//! method is a named helper beside it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, ParkReason, RunState};
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderRefusal, ProviderWait};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

use super::cooldown::{CooldownError, Waited};
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
        let limits = self.seat_limits(Some(seat));
        let wait = self.recorded_wait(events, effect_id);
        let refused = self.recorded_refusal(events, effect_id);
        if failed_attempts >= limits.max_attempts {
            return self.park_at_cap(
                events,
                effect_id,
                failed_attempts,
                limits.max_attempts,
                &wait,
                &refused,
            );
        }
        // A crash between the failed event and its park: fold returns
        // `ExecuteEffect` only when no `run/parked` followed, so the record
        // itself ends the run here, before any wait, probe or driver
        // invocation (decision 0103 slice 4c1, this fix round).
        match refused {
            Ok(Some(refused)) => {
                let reason = self.refused_park_reason(events, effect_id, &refused);
                self.park_with_reason(reason)?;
                return Ok(());
            }
            Ok(None) => {}
            Err(detail) => {
                let reason = format!("refusing to execute effect {effect_id}: {detail}");
                self.park_with_reason(reason)?;
                return Ok(());
            }
        }
        let wait = match wait {
            Ok(Some(wait)) => wait,
            Ok(None) => return self.run_attempt(events, state, effect_id, seat, None),
            Err(detail) => {
                // A present wait that does not read refuses by name, never
                // as no wait (decision 0103 ruling 2(a)).
                let reason = format!("refusing to execute effect {effect_id}: {detail}");
                self.park_with_reason(reason)?;
                return Ok(());
            }
        };
        // Before the wait's probe and before the retry's admission, the
        // run's own reservation may name a crashed probe's attempt that no
        // journal event names. It is released only where every process it
        // witnessed is proven settled; otherwise the run parks with the
        // cleanup refusal naming it, spending no effect attempt and starting
        // no probe or retry (decision 0103 ruling 4, this fix round).
        if let Err(reason) = self.recovered_probe(events, &wait, &crate::engine::broker::observed())
        {
            self.park_with_reason(reason)?;
            return Ok(());
        }
        match self.take_cooldown(events, state, effect_id, seat, &wait)? {
            Waited::Proceed(answer) => {
                self.run_attempt(events, state, effect_id, seat, Some(answer))
            }
            // The stop's own arm writes the ending next turn.
            Waited::Stopped => Ok(()),
            Waited::Parked(reason) => self.park_with_reason(reason),
        }
    }

    /// The cap's park (decision 0006), with the refusal the last failure
    /// recorded beside it (decision 0103 ruling 7(c), this fix round): the
    /// 0006 text speaks first, then the refused record's facts, or the wait
    /// entry's where the journal carries no record.
    fn park_at_cap(
        &mut self,
        events: &[EventEnvelope],
        effect_id: &str,
        failed_attempts: u64,
        max_attempts: u64,
        wait: &Result<Option<ProviderWait>, CooldownError>,
        refused: &Result<Option<ProviderRefusal>, CooldownError>,
    ) -> Result<(), EngineError> {
        let mut reason = format!(
            "effect {effect_id} failed {failed_attempts} of {max_attempts} attempt(s); \
             last error: {}",
            last_error(events, effect_id)
        );
        match refused {
            Ok(Some(refused)) => {
                reason.push_str(&cap_refusal_facts(
                    refused,
                    wait.as_ref().ok().and_then(Option::as_ref),
                ));
                match wait {
                    Ok(Some(wait)) => reason.push_str(&self.cap_maximum(events, effect_id, wait)),
                    Ok(None) => reason.push_str("; no until exists for it"),
                    Err(detail) => reason.push_str(&format!("; {detail}")),
                }
            }
            Ok(None) => {
                if let Ok(Some(wait)) = wait {
                    reason.push_str(&cap_wait_facts(wait));
                    reason.push_str(&self.cap_maximum(events, effect_id, wait));
                } else if let Err(detail) = wait {
                    reason.push_str(&format!("; {detail}"));
                }
            }
            Err(detail) => {
                reason.push_str(&format!("; {detail}"));
                if let Ok(Some(wait)) = wait {
                    reason.push_str(&cap_wait_facts(wait));
                }
            }
        }
        self.park_with_reason(reason)?;
        Ok(())
    }

    /// The one `run/parked` write the refusal and wait parks share: the
    /// reason and an empty evidence envelope. `park_at_cursor` keeps its
    /// own write because a moved head carries evidence.
    fn park_with_reason(&mut self, reason: String) -> Result<(), EngineError> {
        self.append(
            EventType::RunParked,
            json!({"reason": reason, "evidence": {}}),
            None,
        )?;
        Ok(())
    }

    /// The cap's sentence for the declared maximum: the episode past it, the
    /// declaration when the wait is still within it, or why the episode
    /// cannot be read.
    fn cap_maximum(
        &self,
        events: &[EventEnvelope],
        effect_id: &str,
        wait: &ProviderWait,
    ) -> String {
        match self.wait_maximum(events, effect_id, wait) {
            Ok(Some((total, max, true))) => {
                format!("; the episode {total}s is past the declared {max}s maximum")
            }
            Ok(Some((_, max, false))) => format!("; the declared {max}s maximum"),
            Ok(None) => String::new(),
            Err(detail) => format!("; {detail}"),
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

/// The cap's facts about a recorded refusal: provider, route, the `until`
/// the wait entry beside it may carry, the provider's words, its retry-after
/// and the declared maximum (decision 0103 ruling 7(c), this fix round).
fn cap_refusal_facts(refused: &ProviderRefusal, wait: Option<&ProviderWait>) -> String {
    let route = refused.route.as_deref().unwrap_or("an unresolved route");
    let mut facts = match wait {
        Some(wait) => format!(
            "; the last refusal waits for {}/{} until {}: {}",
            refused.provider,
            route,
            wait.until
                .format(&Rfc3339)
                .expect("an RFC 3339 time formats as RFC 3339"),
            refused.words
        ),
        None => format!(
            "; the last refusal names {}/{} with no until: {}",
            refused.provider, route, refused.words
        ),
    };
    if let Some(retry) = refused.retry_after_seconds {
        facts.push_str(&format!("; the provider gave retry-after {retry}s"));
    }
    if let (Some(seconds), Some(max)) = (refused.seconds, refused.max_seconds) {
        facts.push_str(&format!(
            "; the computed wait was {seconds}s; the declared maximum was {max}s"
        ));
    }
    facts
}

/// The cap's facts from the wait entry alone, for a journal whose terminal
/// event carries no `refused` record: its provider, route, `until`, words
/// and the retry-after it recorded.
fn cap_wait_facts(wait: &ProviderWait) -> String {
    let mut facts = format!(
        "; the last refusal waits for {}/{} until {}: {}",
        wait.provider,
        wait.route,
        wait.until
            .format(&Rfc3339)
            .expect("an RFC 3339 time formats as RFC 3339"),
        wait.words
    );
    if let Some(retry) = wait.retry_after_seconds {
        facts.push_str(&format!("; the provider gave retry-after {retry}s"));
    }
    facts
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
