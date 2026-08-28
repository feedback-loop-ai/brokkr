//! `fold(events) -> RunState`: state is derived, never mutated. The fold
//! is bundle-independent — it derives the run's protocol position (the
//! `Cursor`) and journal-computed counters; the runtime combines cursor
//! with the pinned policy to pick the next action. Any event that is
//! impossible at the current cursor fails the fold closed: a journal
//! that violates the protocol is corrupt, not reinterpretable.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::envelope::{EventEnvelope, EventType};

/// Results that count toward the consecutive-failure counter, matching
/// the production table's retry/hard-stop rules.
pub const FAILURE_RESULTS: [&str; 2] = ["failed", "broken"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    AwaitingOperator,
    Completed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cursor {
    /// Run started; the engine must enter the policy's initial phase.
    Start,
    /// The engine must append phase/entered for this phase.
    EnterPhase { phase: String },
    /// The engine must request the current phase's effect (or finish the
    /// run when the current phase is terminal in the pinned policy).
    RequestEffect,
    /// An effect is requested and durable with no attempt in flight.
    /// `failed_attempts` counts terminally failed attempts so far; the
    /// engine retries up to the seat's declared limit (decision 0006) or
    /// appends run/parked here when the limit is exhausted.
    ExecuteEffect {
        effect_id: String,
        seat: String,
        failed_attempts: u64,
    },
    /// An attempt is in flight (or was, when the process last died).
    EffectInFlight {
        effect_id: String,
        attempt_id: String,
        seat: String,
        failed_attempts: u64,
    },
    /// A succeeded result awaits its policy decision.
    Decide { effect_id: String, result: Value },
    /// The engine must append run/parked (failed/indeterminate effect or
    /// a decision that produced no ruling).
    Park { reason: String },
    /// The engine must append run/stopped (operator stop accepted).
    Stop,
    /// Parked or terminal: nothing to do without an operator event.
    Idle,
}

#[derive(Debug, Clone)]
pub struct RunState {
    pub run_id: String,
    pub seq: u64,
    pub last_hash: String,
    pub status: Status,
    pub phase: Option<String>,
    pub cursor: Cursor,
    /// Journal-computed, never accepted from a caller (README law 2).
    pub consecutive_failures: BTreeMap<String, u64>,
    pub reviewed_heads: Option<Value>,
    pub last_decision: Option<Value>,
    pub park_reason: Option<String>,
    pub feature: Option<String>,
    /// Last operator command awaiting disposition: (command_id, command).
    pub pending_command: Option<(String, String)>,
}

#[derive(Debug, Error, PartialEq)]
pub enum FoldError {
    #[error("journal is empty")]
    Empty,
    #[error("event {seq}: first event must be run/started")]
    FirstEventNotRunStarted { seq: u64 },
    #[error("event {seq}: {event} is impossible at cursor {cursor}")]
    OutOfPlace {
        seq: u64,
        event: String,
        cursor: String,
    },
    #[error("event {seq}: payload missing or mistyped '{field}'")]
    BadPayload { seq: u64, field: String },
    #[error("event {seq}: event after terminal status")]
    AfterTerminal { seq: u64 },
    #[error("event {seq}: operator/accepted without a matching command")]
    NoMatchingCommand { seq: u64 },
    #[error("event {seq}: unknown operator command '{command}'")]
    UnknownCommand { seq: u64, command: String },
}

fn payload_str(event: &EventEnvelope, field: &str) -> Result<String, FoldError> {
    event
        .payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| FoldError::BadPayload {
            seq: event.seq,
            field: field.to_string(),
        })
}

fn cursor_name(cursor: &Cursor) -> String {
    format!("{cursor:?}")
}

/// Fold a verified journal into state. Callers verify the hash chain
/// first (`envelope::verify_chain`); fold checks protocol shape only.
pub fn fold(events: &[EventEnvelope]) -> Result<RunState, FoldError> {
    let first = events.first().ok_or(FoldError::Empty)?;
    if first.event_type != EventType::RunStarted {
        return Err(FoldError::FirstEventNotRunStarted { seq: first.seq });
    }
    let mut state = RunState {
        run_id: first.run_id.clone(),
        seq: 0,
        last_hash: String::new(),
        status: Status::Running,
        phase: None,
        cursor: Cursor::Start,
        consecutive_failures: BTreeMap::new(),
        reviewed_heads: None,
        last_decision: None,
        park_reason: None,
        feature: first
            .payload
            .get("feature")
            .and_then(Value::as_str)
            .map(str::to_string),
        pending_command: None,
    };

    for event in events {
        state.seq = event.seq;
        state.last_hash = event.event_hash.clone();
        if event.seq == 1 {
            continue; // run/started consumed above
        }
        apply(&mut state, event)?;
    }
    Ok(state)
}

fn apply(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    use EventType::*;

    if matches!(state.status, Status::Completed | Status::Stopped) {
        // Terminal runs accept only operator annotations that change nothing.
        return match event.event_type {
            OperatorCommanded | OperatorRejected => Ok(()),
            _ => Err(FoldError::AfterTerminal { seq: event.seq }),
        };
    }

    let out_of_place = |state: &RunState| FoldError::OutOfPlace {
        seq: event.seq,
        event: format!("{:?}", event.event_type),
        cursor: cursor_name(&state.cursor),
    };

    match event.event_type {
        RunStarted => Err(out_of_place(state)),

        PhaseEntered => {
            let phase = payload_str(event, "phase")?;
            match &state.cursor {
                Cursor::Start => {}
                Cursor::EnterPhase { phase: expected } if *expected == phase => {}
                _ => return Err(out_of_place(state)),
            }
            state.phase = Some(phase);
            state.cursor = Cursor::RequestEffect;
            Ok(())
        }

        EffectRequested => {
            if state.cursor != Cursor::RequestEffect {
                return Err(out_of_place(state));
            }
            state.cursor = Cursor::ExecuteEffect {
                effect_id: payload_str(event, "effect_id")?,
                seat: payload_str(event, "seat")?,
                failed_attempts: 0,
            };
            Ok(())
        }

        EffectStarted => {
            let effect_id = payload_str(event, "effect_id")?;
            let attempt_id = payload_str(event, "attempt_id")?;
            match &state.cursor {
                Cursor::ExecuteEffect {
                    effect_id: open,
                    seat,
                    failed_attempts,
                } if *open == effect_id => {
                    state.cursor = Cursor::EffectInFlight {
                        effect_id,
                        attempt_id,
                        seat: seat.clone(),
                        failed_attempts: *failed_attempts,
                    };
                    Ok(())
                }
                _ => Err(out_of_place(state)),
            }
        }

        EffectCheckpointed => match &state.cursor {
            Cursor::EffectInFlight { effect_id, .. }
                if *effect_id == payload_str(event, "effect_id")? =>
            {
                Ok(())
            }
            _ => Err(out_of_place(state)),
        },

        EffectSucceeded => {
            let effect_id = payload_str(event, "effect_id")?;
            match &state.cursor {
                Cursor::EffectInFlight {
                    effect_id: open, ..
                } if *open == effect_id => {
                    let result =
                        event
                            .payload
                            .get("result")
                            .cloned()
                            .ok_or(FoldError::BadPayload {
                                seq: event.seq,
                                field: "result".into(),
                            })?;
                    state.cursor = Cursor::Decide { effect_id, result };
                    Ok(())
                }
                _ => Err(out_of_place(state)),
            }
        }

        EffectFailed => {
            // A determinate failure returns the effect to the executable
            // position; the ENGINE decides retry-or-park against the
            // seat's declared attempt limit (decision 0006).
            let effect_id = payload_str(event, "effect_id")?;
            match &state.cursor {
                Cursor::EffectInFlight {
                    effect_id: open,
                    seat,
                    failed_attempts,
                    ..
                } if *open == effect_id => {
                    state.cursor = Cursor::ExecuteEffect {
                        effect_id,
                        seat: seat.clone(),
                        failed_attempts: failed_attempts + 1,
                    };
                    Ok(())
                }
                _ => Err(out_of_place(state)),
            }
        }

        EffectIndeterminate => {
            // Completion could not be established: NEVER auto-retried —
            // a retry could silently re-pay for or duplicate completed
            // work. Always parks into operator judgment.
            let effect_id = payload_str(event, "effect_id")?;
            let matches_open = matches!(
                &state.cursor,
                Cursor::EffectInFlight { effect_id: open, .. } if *open == effect_id
            ) || matches!(
                // A requested-but-never-started effect can be closed as
                // indeterminate during crash recovery.
                &state.cursor,
                Cursor::ExecuteEffect { effect_id: open, .. } if *open == effect_id
            );
            if !matches_open {
                return Err(out_of_place(state));
            }
            let detail = event
                .payload
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("no detail recorded");
            state.cursor = Cursor::Park {
                reason: format!("effect {effect_id} indeterminate: {detail}"),
            };
            Ok(())
        }

        TransitionDecided => {
            if !matches!(&state.cursor, Cursor::Decide { .. }) {
                return Err(out_of_place(state));
            }
            let from = payload_str(event, "from")?;
            let result = payload_str(event, "result")?;
            if FAILURE_RESULTS.contains(&result.as_str()) {
                *state.consecutive_failures.entry(from.clone()).or_insert(0) += 1;
            } else {
                state.consecutive_failures.insert(from.clone(), 0);
            }
            if let Some(inputs) = event.payload.get("inputs").and_then(Value::as_object) {
                if let Some(heads) = inputs.get("reviewed_heads") {
                    state.reviewed_heads = Some(heads.clone());
                }
            }
            state.last_decision = Some(event.payload.clone());
            match event.payload.get("next").and_then(Value::as_str) {
                Some(next) => {
                    state.cursor = Cursor::EnterPhase {
                        phase: next.to_string(),
                    };
                }
                None => {
                    let problem = event
                        .payload
                        .get("problem")
                        .and_then(Value::as_str)
                        .unwrap_or("no rule matched");
                    state.cursor = Cursor::Park {
                        reason: format!("no ruling for ({from}, {result}): {problem}"),
                    };
                }
            }
            Ok(())
        }

        OperatorCommanded => {
            state.pending_command = Some((
                payload_str(event, "command_id")?,
                payload_str(event, "command")?,
            ));
            Ok(())
        }

        OperatorAccepted => {
            let command_id = payload_str(event, "command_id")?;
            let Some((pending_id, command)) = state.pending_command.take() else {
                return Err(FoldError::NoMatchingCommand { seq: event.seq });
            };
            if pending_id != command_id {
                return Err(FoldError::NoMatchingCommand { seq: event.seq });
            }
            if state.status != Status::AwaitingOperator {
                return Err(out_of_place(state));
            }
            match command.as_str() {
                "retry" => {
                    if state.phase.is_none() {
                        return Err(out_of_place(state));
                    }
                    state.status = Status::Running;
                    state.park_reason = None;
                    state.cursor = Cursor::RequestEffect;
                }
                "stop" => {
                    state.status = Status::Running;
                    state.park_reason = None;
                    state.cursor = Cursor::Stop;
                }
                other => {
                    return Err(FoldError::UnknownCommand {
                        seq: event.seq,
                        command: other.to_string(),
                    })
                }
            }
            Ok(())
        }

        OperatorRejected => {
            state.pending_command = None;
            Ok(())
        }

        RunParked => {
            // Legal at Park, and at ExecuteEffect when the engine has
            // exhausted the seat's attempt limit (decision 0006).
            if !matches!(
                &state.cursor,
                Cursor::Park { .. } | Cursor::ExecuteEffect { .. }
            ) {
                return Err(out_of_place(state));
            }
            state.status = Status::AwaitingOperator;
            state.park_reason = Some(payload_str(event, "reason")?);
            state.cursor = Cursor::Idle;
            Ok(())
        }

        RunCompleted => {
            if state.status != Status::Running {
                return Err(out_of_place(state));
            }
            state.status = Status::Completed;
            state.cursor = Cursor::Idle;
            Ok(())
        }

        RunStopped => {
            if state.status != Status::Running {
                return Err(out_of_place(state));
            }
            state.status = Status::Stopped;
            state.cursor = Cursor::Idle;
            Ok(())
        }
    }
}

/// The evaluation inputs the ENGINE computes from the journal (never
/// accepted from a seat): the consecutive-failure counter including the
/// failure being decided, matching the referee's counting.
pub fn computed_inputs(state: &RunState, phase: &str, result: &str) -> Map<String, Value> {
    let mut inputs = Map::new();
    if FAILURE_RESULTS.contains(&result) {
        let prior = state.consecutive_failures.get(phase).copied().unwrap_or(0);
        inputs.insert("consecutive_failures".to_string(), Value::from(prior + 1));
    }
    inputs
}

#[cfg(test)]
mod tests;
