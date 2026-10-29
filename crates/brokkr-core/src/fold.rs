//! `fold(events) -> RunState`: state is derived, never mutated. The fold
//! is bundle-independent — it derives the run's protocol position (the
//! `Cursor`) and journal-computed counters; the runtime combines cursor
//! with the pinned policy to pick the next action. Any event that is
//! impossible at the current cursor fails the fold closed: a journal
//! that violates the protocol is corrupt, not reinterpretable.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::decision::{Decision, PhaseResult, ReviewedHeads};
use crate::envelope::{EventEnvelope, EventType};
use crate::gate_moved_head;
use crate::payload;
use crate::policy::{CONSECUTIVE_FAILURES, REVIEWED_HEADS, STRATEGIES, STRATEGY, STRATEGY_PHASE};

mod operator;
use operator::{accept, command};
mod relation;
use relation::{Arm, Position, Transition};
mod run;
pub use run::{ParkReason, Run};

/// Results that universally count toward the consecutive-failure counter,
/// matching the production table's retry/hard-stop rules. A strategy-local
/// result such as SDD's `fail` counts only when its decision records that
/// phase's engine-computed counter.
pub const FAILURE_RESULTS: [&str; 2] = ["failed", "broken"];

/// The strategy-local failure word (SDD's verify), which counts toward
/// the consecutive-failure counter only where the counter is in play.
const SCOPED_FAILURE: &str = "fail";

/// The consecutive-failure rule, once (#345): does a decided `result`
/// count toward its phase's counter? [`FAILURE_RESULTS`] always do; the
/// strategy-local `fail` counts only when `counter_read` — at decide
/// time, when the phase's rules read the counter; on replay, when the
/// decision recorded it, which the engine does exactly then.
pub(crate) fn counts_as_failure(result: &str, counter_read: bool) -> bool {
    FAILURE_RESULTS.contains(&result) || (counter_read && result == SCOPED_FAILURE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    AwaitingOperator,
    Completed,
    Stopped,
}

impl Status {
    /// The word every readout and refusal names this status by.
    pub const fn as_str(self) -> &'static str {
        match self {
            Status::Running => "running",
            Status::AwaitingOperator => "awaiting_operator",
            Status::Completed => "completed",
            Status::Stopped => "stopped",
        }
    }

    /// The status [`Status::as_str`] words as `word`, or `None`.
    pub fn named(word: &str) -> Option<Status> {
        let all = [
            Self::Running,
            Self::AwaitingOperator,
            Self::Completed,
            Self::Stopped,
        ];
        all.into_iter().find(|status| status.as_str() == word)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cursor {
    /// Run started; the engine enters its admitted entry, else the table's initial phase.
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
    /// `stop_rides` is an operator stop ACCEPTED while it flies: the
    /// command rides — the attempt is untouched and keeps journaling —
    /// and the effect's boundary concludes the run per the command.
    EffectInFlight {
        effect_id: String,
        attempt_id: String,
        seat: String,
        failed_attempts: u64,
        stop_rides: bool,
    },
    /// A succeeded result awaits its policy decision.
    Decide { effect_id: String, result: Value },
    /// The engine must append run/parked (failed/indeterminate effect or
    /// a decision that produced no ruling).
    Park { reason: ParkReason },
    /// The engine must append run/stopped (operator stop accepted).
    Stop,
}

impl Cursor {
    /// The seat hired for the current effect and how many of its attempts
    /// have failed, while the effect is requested or in flight: the one
    /// reading the summary row and the fleet's hire both make.
    pub fn hire(&self) -> Option<(&str, u64)> {
        match self {
            Cursor::ExecuteEffect {
                seat,
                failed_attempts,
                ..
            }
            | Cursor::EffectInFlight {
                seat,
                failed_attempts,
                ..
            } => Some((seat, *failed_attempts)),
            Cursor::Start
            | Cursor::EnterPhase { .. }
            | Cursor::RequestEffect
            | Cursor::Decide { .. }
            | Cursor::Park { .. }
            | Cursor::Stop => None,
        }
    }
}

/// The cursor as readouts and refusals name it: the variant and the ids
/// and count that place it, never a seat's result or a park's free text.
/// `Debug` stays the whole value, for the replay check.
impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            Cursor::Start => "Start".to_string(),
            Cursor::EnterPhase { phase } => format!("EnterPhase {{ phase: {phase:?} }}"),
            Cursor::RequestEffect => "RequestEffect".to_string(),
            Cursor::ExecuteEffect {
                effect_id,
                seat,
                failed_attempts,
            } => format!(
                "ExecuteEffect {{ effect_id: {effect_id:?}, seat: {seat:?}, \
                 failed_attempts: {failed_attempts} }}"
            ),
            Cursor::EffectInFlight {
                effect_id,
                attempt_id,
                seat,
                failed_attempts,
                ..
            } => format!(
                "EffectInFlight {{ effect_id: {effect_id:?}, attempt_id: {attempt_id:?}, \
                 seat: {seat:?}, failed_attempts: {failed_attempts} }}"
            ),
            Cursor::Decide { effect_id, .. } => format!("Decide {{ effect_id: {effect_id:?} }}"),
            Cursor::Park { .. } => "Park".to_string(),
            Cursor::Stop => "Stop".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RunState {
    pub run_id: String,
    pub seq: u64,
    pub last_hash: String,
    /// Where the run stands: its status, cursor and park reason, at once.
    pub run: Run,
    pub phase: Option<String>,
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
    pub phase_results: BTreeMap<String, PhaseResult>,
    /// The heads the last decision that recorded any recorded: what the
    /// ship gate's drift check compares the tree against.
    pub reviewed_heads: Option<ReviewedHeads>,
    pub last_decision: Option<Decision>,
    pub feature: Option<String>,
    /// Last operator command awaiting disposition: (command_id, command).
    pub pending_command: Option<(String, String)>,
    /// The seq of an operator stop accepted while no `run/stopped` has
    /// been written yet: the run is stopping, not running.
    pub stop_accepted_seq: Option<u64>,
}

impl RunState {
    /// The seq of an accepted stop no ending has closed, while that is
    /// what the cursor says: the run is stopping, not running. A stop
    /// rides an in-flight effect to its boundary, so the cursor still
    /// states the attempt; the accepted seq is kept beside it.
    pub fn stopping_seq(&self) -> Option<u64> {
        match &self.run {
            Run::Running(Cursor::Stop) => self.stop_accepted_seq,
            Run::Running(Cursor::EffectInFlight {
                stop_rides: true, ..
            }) => self.stop_accepted_seq,
            Run::Running(_) | Run::AwaitingOperator { .. } | Run::Completed | Run::Stopped => None,
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq)]
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
    /// A `transition/decided` payload outside its closed contract: a
    /// field mistyped or not in the contract at all.
    #[error("event {seq}: transition/decided payload does not read: {detail}")]
    BadDecision { seq: u64, detail: String },
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
            | FoldError::BadDecision { seq, .. }
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
    /// Releases the writer reservation of a run's ended attempts on the
    /// operator's attestation that every process they owned has ended
    /// (decision 0065 slice two, U8e3). It moves nothing: the attempts stay
    /// as they ended, and the run where it stands.
    Release,
}

impl OperatorCommand {
    /// Every command, in the order `brokkr operator` names them.
    pub const ALL: [OperatorCommand; 3] = [
        OperatorCommand::Retry,
        OperatorCommand::Stop,
        OperatorCommand::Release,
    ];

    /// The word `operator/commanded` records.
    pub const fn as_str(self) -> &'static str {
        match self {
            OperatorCommand::Retry => "retry",
            OperatorCommand::Stop => "stop",
            OperatorCommand::Release => "release",
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
/// - `stop` is a live kill switch and lands wherever a live run stands, and
///   `release`, which moves nothing, lands wherever one stands too.
pub fn acceptance_refusal(state: &RunState, command: OperatorCommand) -> Option<Refusal> {
    if terminal(state.run.status()) {
        return Some(Refusal::AfterTerminal);
    }
    match command {
        OperatorCommand::Retry if state.run.status() != Status::AwaitingOperator => {
            Some(Refusal::RunNotAwaitingOperator)
        }
        OperatorCommand::Retry if state.phase.is_none() => Some(Refusal::NoPhaseToRetry),
        OperatorCommand::Retry | OperatorCommand::Stop | OperatorCommand::Release => None,
    }
}

/// The status a run's conclusion already reached, or `None` for a live
/// run `conclude` would close. The one rule `conclude` refuses by and the
/// view reads to say so before the verb is typed: a second `run/stopped`
/// would fail the fold as an event after terminal.
pub fn concluded(state: &RunState) -> Option<Status> {
    let status = state.run.status();
    terminal(status).then_some(status)
}

/// The words `conclude` refuses a concluded run with, the run named in them.
pub fn concluded_text(run_id: &str, status: Status) -> String {
    format!(
        "run '{run_id}' is already concluded ({}); conclude appends nothing",
        status.as_str()
    )
}

/// The step taken at an in-flight effect's boundary. An operator stop
/// accepted mid-flight rides untouched to exactly here: the run then
/// concludes per the command (`Cursor::Stop`, which the engine turns
/// into run/stopped) instead of taking the boundary's normal next step.
/// The riding command is spent once it is honoured.
fn conclude(state: &mut RunState, normal: Cursor) {
    let rides = matches!(
        state.run,
        Run::Running(Cursor::EffectInFlight {
            stop_rides: true,
            ..
        })
    );
    state.run = Run::Running(match rides {
        true => Cursor::Stop,
        false => normal,
    });
}

/// Fold a verified journal into state. Callers verify the hash chain
/// first (`envelope::verify_chain`); fold checks protocol shape only.
///
/// It is the state `run/started` opens, carried by [`fold_onto`] over the
/// whole journal, so `fold(prefix ++ suffix)` and
/// `fold_onto(fold(prefix)?, suffix)` are one state, byte for byte.
pub fn fold(events: &[EventEnvelope]) -> Result<RunState, FoldError> {
    let first = events.first().ok_or(FoldError::Empty)?;
    if first.event_type != EventType::RunStarted {
        return Err(FoldError::FirstEventNotRunStarted { seq: first.seq });
    }
    let state = RunState {
        run_id: first.run_id.clone(),
        seq: 0,
        last_hash: String::new(),
        run: Run::Running(Cursor::Start),
        phase: None,
        consecutive_failures: BTreeMap::new(),
        visits: BTreeMap::new(),
        strategy: None,
        last_result: None,
        phase_results: BTreeMap::new(),
        reviewed_heads: None,
        last_decision: None,
        feature: payload::RunStarted::read(first).feature.map(str::to_string),
        pending_command: None,
        stop_accepted_seq: None,
    };
    fold_onto(state, events)
}

/// Carry a fold forward over what landed after it: `state` is what
/// [`fold`] or `fold_onto` returned for the journal up to `state.seq`,
/// and `suffix` is the events after it, so a reader that already folded
/// a prefix pays only for the suffix. Callers verify the suffix against
/// the head the state names first (`envelope::verify_chain_after` with
/// `state.seq` and `state.last_hash`); like [`fold`], this checks
/// protocol shape only.
pub fn fold_onto(mut state: RunState, suffix: &[EventEnvelope]) -> Result<RunState, FoldError> {
    for event in suffix {
        state.seq = event.seq;
        state.last_hash = event.event_hash.clone();
        if event.seq == 1 {
            continue; // run/started opened the state in `fold`
        }
        apply(&mut state, event)?;
    }
    Ok(state)
}

/// Apply one event: the relation ([`relation::arm`]) answers for the
/// event at the run's position, and a transition it names is taken.
fn apply(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    match relation::arm(event.event_type, Position::of(state)) {
        Arm::Moves(transition) => take(transition, state, event),
        Arm::OutOfPlace => Err(payload::unplaced(event, out_of_place(state, event))),
        // Terminal runs accept only operator annotations that change nothing.
        Arm::Annotates => Ok(()),
        Arm::AfterTerminal => Err(FoldError::AfterTerminal { seq: event.seq }),
    }
}

fn out_of_place(state: &RunState, event: &EventEnvelope) -> FoldError {
    FoldError::OutOfPlace {
        seq: event.seq,
        event: format!("{:?}", event.event_type),
        cursor: state.run.to_string(),
    }
}

fn take(
    transition: Transition,
    state: &mut RunState,
    event: &EventEnvelope,
) -> Result<(), FoldError> {
    match transition {
        Transition::Enter => enter(state, event),
        Transition::Request => request(state, event),
        Transition::Start => start(state, event),
        Transition::Checkpoint => checkpoint(state, event),
        Transition::Succeed => succeed(state, event),
        Transition::Fail => fail(state, event),
        Transition::Close => close(state, event),
        Transition::Rule => rule(state, event),
        Transition::Command => command(state, event),
        Transition::Accept => accept(state, event),
        Transition::Reject => {
            state.pending_command = None;
            Ok(())
        }
        Transition::Park => park(state, event),
        Transition::Complete => conclude_run(state, Run::Completed),
        Transition::Stop => conclude_run(state, Run::Stopped),
    }
}

/// Is `effect_id` the effect the cursor holds open, requested or in
/// flight? The relation has already placed the event at one of the two.
fn names_the_open_effect(state: &RunState, effect_id: &str) -> bool {
    matches!(
        state.run.cursor(),
        Some(Cursor::ExecuteEffect { effect_id: open, .. }
            | Cursor::EffectInFlight { effect_id: open, .. }) if open == effect_id
    )
}

/// At `Start`, or at `EnterPhase` for the phase it names.
fn enter(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let phase = payload::PhaseEntered::read(event)?.phase.to_string();
    if matches!(state.run.cursor(), Some(Cursor::EnterPhase { phase: expected }) if *expected != phase)
    {
        return Err(out_of_place(state, event));
    }
    *state.visits.entry(phase.clone()).or_insert(0) += 1;
    state.phase = Some(phase);
    state.run = Run::Running(Cursor::RequestEffect);
    Ok(())
}

fn request(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let requested = payload::EffectRequested::read(event)?;
    state.run = Run::Running(Cursor::ExecuteEffect {
        effect_id: requested.effect_id.to_string(),
        seat: requested.seat.to_string(),
        failed_attempts: 0,
    });
    Ok(())
}

/// At `ExecuteEffect`, for the effect it holds.
fn start(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let started = payload::EffectStarted::read(event)?;
    match state.run.cursor() {
        Some(Cursor::ExecuteEffect {
            effect_id: open,
            seat,
            failed_attempts,
        }) if *open == started.effect_id => {
            state.run = Run::Running(Cursor::EffectInFlight {
                effect_id: started.effect_id.to_string(),
                attempt_id: started.attempt_id.to_string(),
                seat: seat.clone(),
                failed_attempts: *failed_attempts,
                stop_rides: false,
            });
            Ok(())
        }
        // Another effect's start: the relation placed the event here.
        _ => Err(out_of_place(state, event)),
    }
}

/// `event`, naming `effect_id`, refused as out of place unless that is the
/// effect the cursor holds open.
fn the_open_effect(
    state: &RunState,
    event: &EventEnvelope,
    effect_id: &str,
) -> Result<(), FoldError> {
    match names_the_open_effect(state, effect_id) {
        true => Ok(()),
        false => Err(out_of_place(state, event)),
    }
}

/// At `EffectInFlight`, for the effect it holds.
fn checkpoint(state: &RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let checkpointed = payload::EffectCheckpointed::read(event)?;
    the_open_effect(state, event, checkpointed.effect_id)
}

/// At `EffectInFlight`, for the effect it holds.
fn succeed(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let succeeded = payload::EffectSucceeded::read(event, |effect_id| {
        the_open_effect(state, event, effect_id)
    })?;
    let (effect_id, result) = (succeeded.effect_id.to_string(), succeeded.result.clone());
    state.last_result = Some(result.clone());
    conclude(state, Cursor::Decide { effect_id, result });
    Ok(())
}

/// At `EffectInFlight`, for the effect it holds. A determinate failure
/// returns the effect to the executable position; the ENGINE decides
/// retry-or-park against the seat's declared attempt limit (decision
/// 0006).
fn fail(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let effect_id = payload::EffectFailed::read(event)?.effect_id;
    match state.run.cursor() {
        Some(Cursor::EffectInFlight {
            effect_id: open,
            seat,
            failed_attempts,
            ..
        }) if *open == effect_id => {
            let normal = Cursor::ExecuteEffect {
                effect_id: effect_id.to_string(),
                seat: seat.clone(),
                failed_attempts: failed_attempts + 1,
            };
            conclude(state, normal);
            Ok(())
        }
        // Another effect's failure: the relation placed the event here.
        _ => Err(out_of_place(state, event)),
    }
}

/// At `EffectInFlight` or `ExecuteEffect`, for the effect it holds.
/// Completion could not be established: NEVER auto-retried — a retry
/// could silently re-pay for or duplicate completed work. Always parks
/// into operator judgment.
fn close(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let closed = payload::EffectIndeterminate::read(event)?;
    let effect_id = closed.effect_id;
    the_open_effect(state, event, effect_id)?;
    let detail = closed.reason.unwrap_or("no detail recorded");
    let normal = Cursor::Park {
        reason: if gate_moved_head::decode::<Value>(detail).is_some() {
            ParkReason::GateMovedHead
        } else {
            ParkReason::read(format!("effect {effect_id} indeterminate: {detail}"))
        },
    };
    conclude(state, normal);
    Ok(())
}

/// At `Decide`: the policy's ruling on the succeeded result.
fn rule(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let decision = payload::decided(event)?;
    let (from, result) = (&decision.from, &decision.result);
    state
        .phase_results
        .insert(from.clone(), PhaseResult::of(&decision));
    if from == STRATEGY_PHASE && STRATEGIES.contains(&result.as_str()) {
        state.strategy = Some(result.clone());
    }
    let counter_read = decision.input(CONSECUTIVE_FAILURES).is_some();
    let failures = state.consecutive_failures.entry(from.clone()).or_insert(0);
    *failures = match counts_as_failure(result, counter_read) {
        true => *failures + 1,
        false => 0,
    };
    if let Some(heads) = decision.input(REVIEWED_HEADS) {
        state.reviewed_heads = Some(ReviewedHeads::read(heads));
    }
    state.run = Run::Running(match decision.next.present() {
        Some(next) => Cursor::EnterPhase {
            phase: next.clone(),
        },
        None => {
            let problem = decision
                .problem
                .present()
                .map_or("no rule matched", String::as_str);
            // A named rule reached this position on purpose — a
            // rule-driven park (decision 0022) or a rule whose
            // artifact gate blocked it. Saying "no ruling" there
            // would be the exact mislabeling the ruling forbids.
            Cursor::Park {
                reason: ParkReason::read(match decision.rule_id.present() {
                    Some(rule_id) => format!("{rule_id} for ({from}, {result}): {problem}"),
                    None => format!("no ruling for ({from}, {result}): {problem}"),
                }),
            }
        }
    });
    state.last_decision = Some(decision);
    Ok(())
}

/// At `Park`, and at `ExecuteEffect` when the engine has exhausted the
/// seat's attempt limit (decision 0006). A selector with no journal
/// strategy and no default also parks while entering the phase, before
/// an effect can be requested.
fn park(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    state.run = Run::AwaitingOperator {
        reason: ParkReason::read(payload::RunParked::read(event)?.reason.to_string()),
    };
    Ok(())
}

/// `run/completed` or `run/stopped`, at any position of a running run.
fn conclude_run(state: &mut RunState, run: Run) -> Result<(), FoldError> {
    state.run = run;
    Ok(())
}

/// The evaluation inputs the ENGINE computes from the journal (never
/// accepted from a seat): the consecutive-failure counter including the
/// failure being decided, matching the referee's counting, when the
/// consecutive-failure rule (`counts_as_failure`) counts `result`.
/// `counter_read` says whether the phase's rules read the counter.
pub fn computed_inputs(
    state: &RunState,
    phase: &str,
    result: &str,
    counter_read: bool,
) -> Map<String, Value> {
    let mut inputs = Map::new();
    if counts_as_failure(result, counter_read) {
        let prior = state.consecutive_failures.get(phase).copied().unwrap_or(0);
        inputs.insert(CONSECUTIVE_FAILURES.to_string(), Value::from(prior + 1));
    }
    if let Some(strategy) = &state.strategy {
        inputs.insert(STRATEGY.to_string(), Value::from(strategy.clone()));
    }
    inputs
}

#[cfg(test)]
mod tests;
