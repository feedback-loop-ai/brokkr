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
use crate::policy::STRATEGIES;

/// Results that universally count toward the consecutive-failure counter,
/// matching the production table's retry/hard-stop rules. A strategy-local
/// result such as SDD's `fail` counts only when its decision records that
/// phase's engine-computed counter.
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
    /// How many times the run has ENTERED each phase — the count the
    /// graph already renders as `×N`, now readable by the table as the
    /// phase-visit predicate that bounds reforging (decision 0022).
    pub visits: BTreeMap<String, u64>,
    /// The last successful triage ruling in this run. It is derived from
    /// the journal and never accepted from a seat as an evaluation input.
    pub strategy: Option<String>,
    /// The raw result object of the most recent succeeded effect. A seat
    /// the run RETURNS to receives it (decision 0022): the finding
    /// travels with the run, not just the boolean it was reduced to.
    pub last_result: Option<Value>,
    /// Typed forward handoff: the last ruled successful result per phase,
    /// reduced to its verdict and filtered inputs by transition/decided.
    pub phase_results: BTreeMap<String, Value>,
    pub reviewed_heads: Option<Value>,
    pub last_decision: Option<Value>,
    pub park_reason: Option<String>,
    pub feature: Option<String>,
    /// Last operator command awaiting disposition: (command_id, command).
    pub pending_command: Option<(String, String)>,
    /// An operator stop ACCEPTED while an attempt was in flight. The
    /// command rides — the attempt is untouched and keeps journaling —
    /// and the effect's boundary concludes the run per the command.
    pub riding_stop: bool,
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

impl FoldError {
    /// The journal position the fold refused at — the citation a reader
    /// can check, and the fact a quarantined row states about itself. An
    /// empty journal has no event to cite, so it cites the position
    /// before the first one.
    pub fn seq(&self) -> u64 {
        match self {
            FoldError::Empty => 0,
            FoldError::FirstEventNotRunStarted { seq }
            | FoldError::OutOfPlace { seq, .. }
            | FoldError::BadPayload { seq, .. }
            | FoldError::AfterTerminal { seq }
            | FoldError::NoMatchingCommand { seq }
            | FoldError::UnknownCommand { seq, .. } => *seq,
        }
    }
}

/// The commands an operator gives a live run, through `brokkr operator`
/// or the Looper bridge, and the word the journal records for each. A
/// word outside the set can still reach the journal as an
/// `operator/commanded` (the bridge journals the word it was sent, and
/// `supersede` is an annotation), and an acceptance of one folds to
/// [`FoldError::UnknownCommand`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorCommand {
    /// Moves a parked run back to running, into the phase it parked in.
    Retry,
    /// Concludes the run wherever it stands, riding an in-flight attempt
    /// to its boundary first.
    Stop,
}

impl OperatorCommand {
    /// Every command, in the order `brokkr operator` names them.
    pub const ALL: [OperatorCommand; 2] = [OperatorCommand::Retry, OperatorCommand::Stop];

    /// The word `operator/commanded` records.
    pub const fn as_str(self) -> &'static str {
        match self {
            OperatorCommand::Retry => "retry",
            OperatorCommand::Stop => "stop",
        }
    }

    /// The command a typed or journaled word names, or `None` for a word
    /// outside the set.
    pub fn parse(word: &str) -> Option<OperatorCommand> {
        Self::ALL
            .into_iter()
            .find(|command| command.as_str() == word)
    }
}

/// Why an operator command was refused: the word `operator/rejected`
/// journals as its `reason`. Two kinds of refusal share it. A command
/// the run could never have taken names its condition; a command that
/// was legal when asked and illegal by the time its disposition was
/// written is [`Refusal::LostFence`], a run that moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Not a word in [`OperatorCommand`].
    CommandNotAllowed,
    /// The run had already reached `Completed`/`Stopped`. Named for the
    /// [`FoldError`] an acceptance there would mint, since that error is
    /// what the refusal exists to prevent.
    AfterTerminal,
    /// `retry` asked of a run that is not parked.
    RunNotAwaitingOperator,
    /// `retry` asked of a run parked before it entered any phase (a
    /// selector with no strategy and no default parks at `Start`), so
    /// there is no phase to return to.
    NoPhaseToRetry,
    /// The run moved beneath the command: legal when the operator asked,
    /// illegal by the time the disposition was written.
    LostFence,
    /// The caller's cursor no longer describes the run's head: the
    /// fenced path's word for a race, `LostFence`'s counterpart on the
    /// side that HAS a cursor to be stale.
    StaleCursor,
    /// A replayed command id whose `operator/commanded` is journaled with
    /// no disposition after it.
    IncompleteCommandReplay,
    /// A replayed rejection whose journaled payload carries no reason.
    PreviouslyRejected,
}

impl Refusal {
    /// The word the journal records.
    pub const fn word(self) -> &'static str {
        match self {
            Refusal::CommandNotAllowed => "command_not_allowed",
            Refusal::AfterTerminal => "after_terminal",
            Refusal::RunNotAwaitingOperator => "run_not_awaiting_operator",
            Refusal::NoPhaseToRetry => "no_phase_to_retry",
            Refusal::LostFence => "lost_fence",
            Refusal::StaleCursor => "stale_cursor",
            Refusal::IncompleteCommandReplay => "incomplete_command_replay",
            Refusal::PreviouslyRejected => "previously_rejected",
        }
    }
}

fn terminal(status: Status) -> bool {
    matches!(status, Status::Completed | Status::Stopped)
}

/// Would an `operator/accepted` for `command` fold against this state,
/// and if not, which condition refuses it? The one rule both sides read:
/// [`fold`] refuses an acceptance this names, and the engine asks it
/// before writing one, because events are immutable and an acceptance
/// that lands where fold refuses it is a journal that stops folding from
/// that seq on.
///
/// Whether the acceptance disposes of the command still pending is the
/// other half of fold's check, and is the writer's to fence: this rule
/// reads the run, not the command queue.
/// - A run that has gone `Completed`/`Stopped` takes no acceptance.
/// - `retry` moves a run from parked back to running, so it needs the
///   run still parked, and a phase to return to.
/// - `stop` is a live kill switch and lands wherever a live run stands.
pub fn acceptance_refusal(state: &RunState, command: OperatorCommand) -> Option<Refusal> {
    if terminal(state.status) {
        return Some(Refusal::AfterTerminal);
    }
    match command {
        OperatorCommand::Retry if state.status != Status::AwaitingOperator => {
            Some(Refusal::RunNotAwaitingOperator)
        }
        OperatorCommand::Retry if state.phase.is_none() => Some(Refusal::NoPhaseToRetry),
        OperatorCommand::Retry | OperatorCommand::Stop => None,
    }
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

/// The step taken at an in-flight effect's boundary. An operator stop
/// accepted mid-flight rides untouched to exactly here: the run then
/// concludes per the command (`Cursor::Stop`, which the engine turns
/// into run/stopped) instead of taking the boundary's normal next step.
/// The riding command is spent once it is honoured.
fn conclude(state: &mut RunState, normal: Cursor) {
    state.cursor = match state.riding_stop {
        true => Cursor::Stop,
        false => normal,
    };
    state.riding_stop = false;
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
        visits: BTreeMap::new(),
        strategy: None,
        last_result: None,
        phase_results: BTreeMap::new(),
        reviewed_heads: None,
        last_decision: None,
        park_reason: None,
        feature: first
            .payload
            .get("feature")
            .and_then(Value::as_str)
            .map(str::to_string),
        pending_command: None,
        riding_stop: false,
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

#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn apply(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    use EventType::*;

    if terminal(state.status) {
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
            *state.visits.entry(phase.clone()).or_insert(0) += 1;
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
                    state.last_result = Some(result.clone());
                    conclude(state, Cursor::Decide { effect_id, result });
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
                    let normal = Cursor::ExecuteEffect {
                        effect_id,
                        seat: seat.clone(),
                        failed_attempts: failed_attempts + 1,
                    };
                    conclude(state, normal);
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
            let normal = Cursor::Park {
                reason: if detail.starts_with("GATE-MOVED-HEAD ") {
                    "GATE-MOVED-HEAD".to_string()
                } else {
                    format!("effect {effect_id} indeterminate: {detail}")
                },
            };
            conclude(state, normal);
            Ok(())
        }

        TransitionDecided => {
            if !matches!(&state.cursor, Cursor::Decide { .. }) {
                return Err(out_of_place(state));
            }
            let from = payload_str(event, "from")?;
            let result = payload_str(event, "result")?;
            state.phase_results.insert(
                from.clone(),
                serde_json::json!({
                    "result": result.clone(),
                    "inputs": event.payload.get("inputs").cloned().unwrap_or_else(|| serde_json::json!({})),
                }),
            );
            if from == "triage" && STRATEGIES.contains(&result.as_str()) {
                state.strategy = Some(result.clone());
            }
            let scoped_failure = result == "fail"
                && event
                    .payload
                    .pointer("/inputs/consecutive_failures")
                    .is_some();
            if FAILURE_RESULTS.contains(&result.as_str()) || scoped_failure {
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
                    // A named rule reached this position on purpose — a
                    // rule-driven park (decision 0022) or a rule whose
                    // artifact gate blocked it. Saying "no ruling" there
                    // would be the exact mislabeling the ruling forbids.
                    state.cursor = Cursor::Park {
                        reason: match event.payload.get("rule_id").and_then(Value::as_str) {
                            Some(rule_id) => format!("{rule_id} for ({from}, {result}): {problem}"),
                            None => format!("no ruling for ({from}, {result}): {problem}"),
                        },
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
            let Some(command) = OperatorCommand::parse(&command) else {
                return Err(FoldError::UnknownCommand {
                    seq: event.seq,
                    command,
                });
            };
            // `acceptance_refusal`'s terminal answer never reaches here:
            // the top of `apply` refuses every event after a terminal
            // first. What is left is a retry with nothing to return to.
            if acceptance_refusal(state, command).is_some() {
                return Err(out_of_place(state));
            }
            match command {
                OperatorCommand::Retry => {
                    state.status = Status::Running;
                    state.park_reason = None;
                    state.cursor = Cursor::RequestEffect;
                }
                // `brokkr operator` is a live kill switch: it journals
                // the command AND its acceptance without reading the
                // run's cursor first, so a stop legitimately lands
                // wherever the run stands. It always reaches a
                // conclusion; only WHEN differs. Mid-flight it rides —
                // the attempt is untouched, its checkpoints keep
                // applying, and the effect's own boundary (result,
                // failure, or indeterminate close) concludes the run per
                // it, so decision 0006's attempt bounds are never
                // truncated. Anywhere else there is no attempt to wait
                // for, so the run concludes where it stands: parked (the
                // operator answering a park with "stop") and running
                // between effects are the same sentence, finished at the
                // first position the journal offers.
                OperatorCommand::Stop => {
                    if matches!(state.cursor, Cursor::EffectInFlight { .. }) {
                        state.riding_stop = true;
                    } else {
                        state.status = Status::Running;
                        state.park_reason = None;
                        state.cursor = Cursor::Stop;
                    }
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
            // exhausted the seat's attempt limit (decision 0006). A
            // selector with no journal strategy and no default also parks
            // while entering the phase, before an effect can be requested.
            if !matches!(
                &state.cursor,
                Cursor::Start
                    | Cursor::EnterPhase { .. }
                    | Cursor::Park { .. }
                    | Cursor::ExecuteEffect { .. }
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
    if let Some(strategy) = &state.strategy {
        inputs.insert("strategy".to_string(), Value::from(strategy.clone()));
    }
    inputs
}

#[cfg(test)]
mod tests;
