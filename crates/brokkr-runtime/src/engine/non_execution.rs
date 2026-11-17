//! Decision 0106 ruling 4's predicate, derived once from the journal's typed
//! evidence: a park is `retryable` only when its last attempt is PROVEN
//! never to have executed. It stands beside 0053's [`super::failed_to_start`],
//! which writes the `start_failure` field this reads, and the spend-time
//! re-read of slice 0106d calls the same function, so the writer and the
//! dispatcher cannot disagree.
//!
//! The `start_failure` field alone is not the proof. A panel writes it when
//! ANY member failed to start while the others ran, a sequence writes it
//! from the failing step's own start failures after earlier steps were
//! accepted and checkpointed, and a typed rate-limit refusal writes it and
//! then records a wait. So the field is one conjunct of five, and the rest
//! are read from the events beside it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, Run, RunState};
use brokkr_core::EventEnvelope;
use serde_json::Value;

use super::broker::settlement::reservation_stands;
use super::cooldown::terminal;
use super::Engine;

/// What the effect's newest terminal event says about its last attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LastAttempt {
    /// An `effect/indeterminate`: the attempt may have executed.
    Indeterminate,
    /// A failure that records a typed refusal (`refused` or `waits`): the
    /// provider answered, and the wait it recorded is the retry's gate.
    Refused,
    /// A failure to start with nothing the journal holds of the attempt
    /// having run: `start_failure`, no checkpoint of that attempt before it.
    FailedToStart,
    /// Any other failure: the attempt may have executed.
    Failed,
}

/// The facts of one effect the journal cannot say for itself.
#[derive(Debug, Clone, Copy)]
pub(super) struct Standing {
    /// The effect's failed attempts, as the fold counts them.
    pub failed_attempts: u64,
    /// 0006's cap for the seat.
    pub max_attempts: u64,
    /// Whether a writer reservation stands for the run, or is not proven
    /// absent: read from the coordination root ([`reservation_stands`]),
    /// never from the engine's lease, which settle takes off the engine
    /// before it can fail.
    pub reservation: bool,
}

/// The attempt a journal event belongs to: the envelope's, else the
/// payload's. `None` is an event that names none.
fn attempt_of(event: &EventEnvelope) -> Option<&str> {
    event
        .attempt_id
        .as_deref()
        .or_else(|| event.payload.get("attempt_id").and_then(Value::as_str))
}

fn is_effect(event: &EventEnvelope, effect_id: &str) -> bool {
    event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
}

/// What the effect's last terminal says of its last attempt, from the
/// events alone. `None` is an effect with no terminal event, or a terminal
/// that names no attempt, which proves nothing.
pub(super) fn last_attempt(events: &[EventEnvelope], effect_id: &str) -> Option<LastAttempt> {
    let last = terminal(events, effect_id)?;
    if last.event_type == EventType::EffectIndeterminate {
        return Some(LastAttempt::Indeterminate);
    }
    if last.payload.get("refused").is_some() || last.payload.get("waits").is_some() {
        return Some(LastAttempt::Refused);
    }
    let started = last
        .payload
        .get("start_failure")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let attempt = attempt_of(last)?;
    // A checkpoint of the attempt before its terminal is the journal's mark
    // that the attempt got as far as journaling something: a sequence
    // checkpoints each accepted step, a seat each turn it streamed, and a
    // panel EVERY member's outcome as `panel-member-finished` (panel.rs
    // `journal_panel_members`), a member that failed to start included.
    // So a panel never reads `FailedToStart`: fail-closed for `retryable`,
    // and a cap whose members all failed to start reads `determinate`.
    let ran = events.iter().any(|event| {
        event.event_type == EventType::EffectCheckpointed
            && event.seq < last.seq
            && is_effect(event, effect_id)
            && attempt_of(event) == Some(attempt)
    });
    Some(if started && !ran {
        LastAttempt::FailedToStart
    } else {
        LastAttempt::Failed
    })
}

/// Whether every attempt the journal started for the effect has a terminal
/// event of its own.
fn every_attempt_settled(events: &[EventEnvelope], effect_id: &str) -> bool {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectStarted && is_effect(event, effect_id))
        .all(|opened| {
            events.iter().any(|event| {
                matches!(
                    event.event_type,
                    EventType::EffectSucceeded
                        | EventType::EffectFailed
                        | EventType::EffectIndeterminate
                ) && event.seq > opened.seq
                    && is_effect(event, effect_id)
                    && attempt_of(event) == attempt_of(opened)
            })
        })
}

/// Ruling 4's predicate: the effect's last attempt never executed, the
/// effect is under its cap, every attempt is settled and no writer
/// reservation stands. A stored `retryable: true` is never trusted over
/// this.
pub(super) fn never_executed(
    events: &[EventEnvelope],
    effect_id: &str,
    standing: Standing,
) -> bool {
    last_attempt(events, effect_id) == Some(LastAttempt::FailedToStart)
        && standing.failed_attempts < standing.max_attempts
        && !standing.reservation
        && every_attempt_settled(events, effect_id)
}

impl Engine {
    /// Whether a contention park carries ruling 4's evidence: the run is at
    /// an effect's execution, and [`never_executed`] holds for it.
    pub(super) fn contention_retryable(&self, state: &RunState) -> bool {
        let Run::Running(Cursor::ExecuteEffect {
            effect_id,
            seat,
            failed_attempts,
        }) = &state.run
        else {
            return false;
        };
        let standing = Standing {
            failed_attempts: *failed_attempts,
            max_attempts: self.seat_limits(Some(seat)).max_attempts,
            reservation: reservation_stands(&self.run_id),
        };
        never_executed(&self.replay.events, effect_id, standing)
    }
}

#[cfg(test)]
mod tests;
