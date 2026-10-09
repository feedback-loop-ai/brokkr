//! The operator's verbs over a run's journal: `retry` and `stop`
//! (unfenced and through the Looper bridge), `conclude`, and `supersede`
//! ([`supersede`]). Free functions over [`Store`] that share no
//! [`super::Engine`] state. Every write that could land where `fold`
//! refuses it is fenced on the head the deciding fold read
//! ([`fenced_append`]), and the rule for whether an acceptance folds at
//! all is `fold`'s own ([`acceptance_refusal`]).

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{
    acceptance_refusal, fold, Cursor, OperatorCommand, Refusal, RunState, Status,
};
use brokkr_core::EventEnvelope;
use brokkr_store::{Store, StoreError};
use serde_json::{json, Value};
use uuid::Uuid;

use super::{EngineError, OPERATOR_STOP_RULE};

mod receipt;
mod supersede;
#[cfg(test)]
pub(super) use supersede::operator_supersede_racing;
pub use supersede::{operator_supersede, Supersede};

/// How many times [`operator_command`] re-decides against a moving head
/// before it refuses. Each turn is spent only when a peer appended in the
/// microseconds between the deciding fold and the fenced write, and costs
/// one load and one fold. A command that loses four in a row is not
/// racing a burn, and a refusal is always the safe answer — `fold` reads
/// `operator/rejected` back in every state there is.
pub(super) const FENCE_ATTEMPTS: usize = 4;

/// The head a fenced write must still find: the seq and hash the store
/// compares, and the event id a write that follows it names as its cause.
struct Head {
    seq: u64,
    hash: String,
    event_id: String,
}

impl Head {
    fn of(event: &EventEnvelope) -> Head {
        Head {
            seq: event.seq,
            hash: event.event_hash.clone(),
            event_id: event.event_id.clone(),
        }
    }

    /// The head of a journal that has already folded. `fold` refuses an
    /// empty journal, so there always is one.
    fn tip(events: &[EventEnvelope]) -> Head {
        Head::of(events.last().expect("a journal that folded has a head"))
    }
}

/// Append `event_type` only if the run's head is still `head`
/// ([`Store::append_next_if_head`]): a peer that appended in between
/// takes the head away, and nothing is written.
fn fenced_append(
    store: &mut Store,
    run_id: &str,
    head: &Head,
    cause: &str,
    event_type: EventType,
    payload: Value,
    attempt_id: Option<String>,
) -> Result<EventEnvelope, StoreError> {
    let cause = Some(cause.to_string());
    store.append_next_if_head(
        run_id, head.seq, &head.hash, event_type, payload, cause, attempt_id,
    )
}

/// The proof every verb takes before it answers: what it just wrote
/// reads back.
fn read_back(store: &Store, run_id: &str) -> Result<RunState, EngineError> {
    Ok(fold(&store.load(run_id)?)?)
}

/// The run as a deciding fold reads it, and the head a write decided on
/// that reading must still find.
fn folded(store: &Store, run_id: &str) -> Result<(RunState, Head), EngineError> {
    let events = store.load(run_id)?;
    Ok((fold(&events)?, Head::tip(&events)))
}

/// The command id an operator event carries.
fn command_id_of(event: &EventEnvelope) -> Option<&str> {
    event.payload.get("command_id").and_then(Value::as_str)
}

/// The conclusion an accepted operator stop is journaled with: the rule
/// id [`OPERATOR_STOP_RULE`], the command it names, and the operator who
/// gave it with the reason they recorded. `run/stopped`'s v1 payload is
/// closed at `{reason}` (`contracts/README.md`, additionalProperties
/// false), so the citation lives INSIDE the reason string — no new field,
/// no second vocabulary — exactly as `request_or_finish`'s policy-driven
/// hard stop cites its `rule_id` there. The cause is read back from the
/// journal that `operator_command` wrote: `fold` spends the pending
/// command when it accepts it, so the events are the only place it
/// survives.
pub(super) fn operator_stop_reason(events: &[EventEnvelope]) -> String {
    let accepted = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::OperatorAccepted);
    let accepted_field = |field: &str| {
        accepted
            .and_then(|event| event.payload.get(field))
            .and_then(Value::as_str)
    };
    // The acceptance carries only the command's id; the command itself
    // is on the `operator/commanded` it disposes of.
    let command_id = accepted_field("command_id").unwrap_or("unrecorded");
    let command = events
        .iter()
        .find(|event| {
            event.event_type == EventType::OperatorCommanded
                && command_id_of(event) == Some(command_id)
        })
        .and_then(|event| event.payload.get("command"))
        .and_then(Value::as_str)
        .unwrap_or(OperatorCommand::Stop.as_str());
    format!(
        "{OPERATOR_STOP_RULE}: operator '{}' commanded {command} ({command_id}): {}",
        accepted_field("operator").unwrap_or("unrecorded"),
        accepted_field("reason").unwrap_or("no reason recorded"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FencedCommandOutcome {
    Accepted {
        head_seq: u64,
        head_hash: String,
    },
    Rejected {
        reason: String,
        head_seq: u64,
        head_hash: String,
    },
}

/// The word a Looper command carries, parsed once at the bridge's edge: a
/// verb of the closed set, or a word outside it, kept as sent so the
/// journal records what was asked before the door refuses it by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandWord<'a> {
    Known(OperatorCommand),
    Unknown(&'a str),
}

impl<'a> CommandWord<'a> {
    pub fn parse(word: &'a str) -> CommandWord<'a> {
        OperatorCommand::parse(word).map_or(CommandWord::Unknown(word), CommandWord::Known)
    }

    /// The word `operator/commanded` journals: the verb's own, or the one
    /// Looper sent.
    fn as_str(self) -> &'a str {
        match self {
            CommandWord::Known(command) => command.as_str(),
            CommandWord::Unknown(word) => word,
        }
    }
}

/// A command as Looper sent it, each wire field named: its id, its word,
/// who asked and why, and the head (`expected_seq`, `expected_hash`) the
/// operator saw when they asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FencedCommand<'a> {
    pub command_id: &'a str,
    pub command: CommandWord<'a>,
    pub operator: &'a str,
    pub reason: &'a str,
    pub expected_seq: u64,
    pub expected_hash: &'a str,
}

/// An acceptance that just landed, reported at the run's head.
fn accepted(store: &Store, run_id: &str) -> Result<FencedCommandOutcome, EngineError> {
    let (head_seq, head_hash) = store.head_hash(run_id)?;
    Ok(FencedCommandOutcome::Accepted {
        head_seq,
        head_hash,
    })
}

/// Journal a refusal and report it. A rejection needs no fence of its
/// own: `fold` reads `operator/rejected` back in every state there is,
/// terminal included, which is exactly why refusing is always the safe
/// answer to a race. The bridge's refusals carry an id a redelivery
/// shares, so they land on the head their receipt lookup read instead
/// ([`receipt::refuse_undisposed`]).
fn refuse(
    store: &mut Store,
    run_id: &str,
    rejection: Rejection,
) -> Result<FencedCommandOutcome, EngineError> {
    let (payload, cause) = (rejection.payload(), Some(rejection.cause.to_string()));
    let rejected = store.append_next(run_id, EventType::OperatorRejected, payload, cause, None)?;
    Ok(rejection.landed(rejected))
}

/// A refusal as `operator/rejected` journals it: the command it disposes
/// of, who asked, the refusal, and the event it is caused by.
struct Rejection<'a> {
    command_id: &'a str,
    operator: &'a str,
    refusal: Refusal,
    cause: &'a str,
}

impl Rejection<'_> {
    fn payload(&self) -> Value {
        let (id, operator, reason) = (self.command_id, self.operator, self.refusal.word());
        json!({"command_id": id, "operator": operator, "reason": reason})
    }

    /// The refusal reported at the rejection that just landed.
    fn landed(&self, rejected: EventEnvelope) -> FencedCommandOutcome {
        FencedCommandOutcome::Rejected {
            reason: self.refusal.word().into(),
            head_seq: rejected.seq,
            head_hash: rejected.event_hash,
        }
    }
}

/// Append an operator command and its disposition (the CLI is the
/// operator's console; approval is a signed journal entry, not prose).
///
/// Unfenced in the sense that the caller supplies no cursor — an
/// operator at a terminal has not read a head hash — but not unfenced
/// against the run. An engine process driving this same run is appending
/// concurrently, and between the operator's decision and this write it
/// can conclude the run or un-park it. The old code appended
/// `operator/accepted` unconditionally into that window, which on a run
/// that had gone terminal wrote a journal that no longer folds: silent
/// at write time, irreversible afterward, and surfacing only the next
/// time anyone read the run.
///
/// So the state is re-established here, as close to the write as the
/// store API allows — which, with [`Store::append_next_if_head`], is all
/// the way. `operator/commanded` is appended first (fold exempts it even
/// after a terminal, so it is safe anywhere and it records that the
/// operator asked). Then the run is folded again, and what THAT fold says
/// — not what the operator saw — decides the disposition. An acceptance
/// is written with the head that fold read as its fence, so it can only
/// land on the state it was decided against: a peer that appended in
/// between takes the head away, nothing is written, and the decision is
/// made again against what the journal now says. Decide-and-append is
/// atomic, not merely narrow, and no acceptance this function writes can
/// be one `fold` later refuses.
///
/// A refusal is written unfenced, because `fold` reads
/// `operator/rejected` back in every state there is.
pub fn operator_command(
    store: &mut Store,
    run_id: &str,
    command: OperatorCommand,
    operator: &str,
    reason: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    operator_command_racing(store, run_id, command, operator, reason, |_| {})
}

/// Which refusal a command meets against the state the deciding fold
/// read, if any. The condition [`acceptance_refusal`] names means two
/// things depending on WHEN it first held: true already when the
/// operator asked (`illegal_when_asked`), it is an illegal request and
/// the journal names it; true only once the command had landed, it is
/// [`Refusal::LostFence`], a run that moved.
///
/// An acceptance disposes of the command still pending, and `fold` reads
/// it back only as disposing of THAT one. A second operator commanding
/// in this window takes the pending place, and an acceptance written
/// into it is `NoMatchingCommand` for every reader afterwards — the same
/// irreversible shape as an acceptance after a terminal, arriving from a
/// peer at another terminal rather than from an engine.
fn standing_refusal(
    state: &RunState,
    command: OperatorCommand,
    command_id: &str,
    illegal_when_asked: bool,
) -> Option<Refusal> {
    if let Some(condition) = acceptance_refusal(state, command) {
        return Some(if illegal_when_asked {
            condition
        } else {
            Refusal::LostFence
        });
    }
    let pending = matches!(&state.pending_command, Some((pending, _)) if pending == command_id);
    (!pending).then_some(Refusal::LostFence)
}

/// [`operator_command`], with the window it fences made reachable.
///
/// `between` is called at the one instant the fence exists for: after the
/// deciding fold has read the journal, before the disposition is written.
/// Production passes a no-op. The tests pass an engine's append, because
/// a race proved by two real threads could only be asserted on when it
/// happened to interleave, and a fence is either always there or it is
/// not a fence.
pub(super) fn operator_command_racing(
    store: &mut Store,
    run_id: &str,
    command: OperatorCommand,
    operator: &str,
    reason: &str,
    mut between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    let command_id = Uuid::new_v4().to_string();
    // A journal that does not fold cannot host a legal acceptance at
    // all, so refuse before writing anything rather than adding to it.
    let (asked, head) = folded(store, run_id)?;
    // What the run already refused before the command was even journaled
    // is not a race, and is not reported as one.
    let illegal_when_asked = acceptance_refusal(&asked, command).is_some();
    let commanded = store.append_next(
        run_id,
        EventType::OperatorCommanded,
        json!({"command_id": command_id, "command": command.as_str(), "args": {}, "operator": operator}),
        Some(head.event_id),
        None,
    )?;

    let mut lost = 0;
    // One line per refusal callsite: the exact-coverage gate reads a
    // multi-line call's `?` edges as their own regions, and a refusal
    // whose failure edge cannot be reached deterministically would read
    // as an uncovered line forever. Sharing the line makes the gate see
    // what is actually exercised.
    let refusal_of = |store: &mut Store, why: Refusal| {
        let (command_id, cause) = (&command_id, &commanded.event_id);
        refuse(
            store,
            run_id,
            Rejection {
                command_id,
                operator,
                refusal: why,
                cause,
            },
        )
    };
    let disposition = loop {
        let (state, head) = folded(store, run_id)?;
        between(store);
        if let Some(refusal) = standing_refusal(&state, command, &command_id, illegal_when_asked) {
            break refusal_of(store, refusal)?;
        }
        let payload = json!({"command_id": command_id, "operator": operator, "reason": reason});
        let cause = &commanded.event_id;
        match fenced_append(
            store,
            run_id,
            &head,
            cause,
            EventType::OperatorAccepted,
            payload,
            None,
        ) {
            Ok(_) => break accepted(store, run_id)?,
            // A peer appended between the fold and the write. The
            // acceptance was never written; decide again against what it
            // wrote, and refuse rather than spin forever.
            Err(StoreError::HeadMoved { .. }) if lost < FENCE_ATTEMPTS => lost += 1,
            Err(StoreError::HeadMoved { .. }) => break refusal_of(store, Refusal::LostFence)?,
            Err(error) => return Err(error.into()),
        }
    };
    // The pair that just landed must read back. Same proof the fenced
    // path takes before it acknowledges anything to Looper.
    read_back(store, run_id)?;
    Ok(disposition)
}

/// Close a run from its journal alone: no bundle, no recipe, no process.
///
/// `resume` is bundle-in, bundle-out — it compiles the exact pinned
/// recipe and refuses on any drift (`ManifestMismatch`) before it looks
/// at the cursor, because the branches it drives (`RequestEffect`,
/// `ExecuteEffect`, `Decide`) SPEND money against a pinned policy and
/// must not spend it against a different one. That gate is correct, and
/// it is also why a run journaled under an engine that has since moved
/// can never reach a lawful conclusion: the door it needs is behind a
/// lock that exists for other doors.
///
/// This is the other door. An operator stop conclusion appends nothing
/// but bookkeeping — `operator/commanded`, `operator/accepted`,
/// at most one `effect/indeterminate`, and `run/stopped` — and reads no
/// policy to append any of it: `fold`'s `"stop"` arm lands at any cursor
/// (riding an in-flight attempt to its boundary, concluding where it
/// stands otherwise), and the boundary close is the same event
/// `advance_running` writes at `Cursor::EffectInFlight` on a fresh
/// process, which consults no bundle either. A closure that spawns
/// nothing needs no pinned recipe to be honest about what it wrote.
///
/// Deterministic throughout (law 2): the caller supplies a run id, an
/// operator identity, and a reason — never a cursor or a status. Every
/// position is re-derived by re-folding the journal after each append,
/// and an unexpected one is an error rather than a guess.
///
/// Every write is FENCED (the operator's park ruling, 2026-09-01,
/// applying the compare-and-append the concurrent-writers slice
/// landed): the stop command re-decides on a moved head and its
/// refusal ends the conclusion, and both closing appends land only on
/// the exact head this process just folded. A run something else is
/// still driving therefore refuses instead of being closed over: ANY
/// movement of the head is evidence the run is not dead, and a
/// conclusion is for a run believed dead. `resume`'s fresh-process
/// branch is fenced the same way, by the engine's own fold (decision
/// 0029). `brokkr runs` remains the way to look first.
pub fn conclude(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
) -> Result<RunState, EngineError> {
    conclude_racing(store, run_id, operator, reason, |_| {})
}

/// [`conclude`] with the windows held open: `between` runs before the
/// stop command and inside each fence — after the head is taken, before
/// the append lands on it. Production passes a no-op; tests pass the
/// live driver the fences exist to refuse.
pub(super) fn conclude_racing(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
    mut between: impl FnMut(&mut Store),
) -> Result<RunState, EngineError> {
    // `load` verifies the hash chain and never returns a partial journal,
    // so a broken chain refuses the whole conclusion here — before any
    // append. No second verification, and the error is never swallowed.
    let state = read_back(store, run_id)?;
    let status = match state.status {
        Status::Completed => Some("completed"),
        Status::Stopped => Some("stopped"),
        Status::Running | Status::AwaitingOperator => None,
    };
    if let Some(status) = status {
        return Err(EngineError::AlreadyConcluded {
            run_id: run_id.to_string(),
            status: status.to_string(),
        });
    }

    // A stop already in force is not re-commanded: the operator who
    // typed `brokkr operator stop` is the cause the journal already
    // names, and a second command would put a second name on the
    // conclusion. Only a run with no stop pending gets one, naming the
    // operator invoking `conclude`.
    between(store);
    if state.cursor != Cursor::Stop && !state.riding_stop {
        command_stop(store, run_id, operator, reason)?;
    }
    let events = close_riding_attempt(store, run_id, &mut between)?;

    // `riding_attempt` answering None IS the statement that the cursor is
    // `Cursor::Stop`; it refuses anything else rather than letting a
    // `run/stopped` be appended somewhere it does not belong.
    let head = Head::tip(&events);
    between(store);
    let payload = json!({"reason": operator_stop_reason(&events)});
    let stopped = fenced_append(
        store,
        run_id,
        &head,
        &head.event_id,
        EventType::RunStopped,
        payload,
        None,
    );
    concluded_or_alive(run_id, stopped)?;
    read_back(store, run_id)
}

/// The stop `conclude` commands on a run that has none in force, naming
/// the operator invoking it. A refusal ends the conclusion: the journal
/// moved beneath it.
fn command_stop(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
) -> Result<(), EngineError> {
    match operator_command(store, run_id, OperatorCommand::Stop, operator, reason)? {
        FencedCommandOutcome::Accepted { .. } => Ok(()),
        FencedCommandOutcome::Rejected {
            reason: refusal, ..
        } => Err(EngineError::Other(format!(
            "conclude: run '{run_id}' refused the stop ({refusal}); the \
             journal moved beneath the conclusion, so something may still \
             be driving this run — look with `brokkr runs` before closing"
        ))),
    }
}

/// The accepted stop rides an in-flight attempt to its boundary. This
/// process holds no driver for that attempt, so completion cannot be
/// established: close it indeterminate, exactly as a fresh drive
/// would. Closing the boundary is what SPENDS the ride (fold's
/// `conclude`), so the loop turns at most once — but what ends it is
/// the re-folded cursor, never a count kept here. Returns the journal
/// the ride left, standing at `Cursor::Stop`.
fn close_riding_attempt(
    store: &mut Store,
    run_id: &str,
    between: &mut impl FnMut(&mut Store),
) -> Result<Vec<EventEnvelope>, EngineError> {
    let mut events = store.load(run_id)?;
    while let Some((effect_id, attempt_id)) = riding_attempt(run_id, &fold(&events)?)? {
        let head = Head::tip(&events);
        between(store);
        let payload = json!({
            "effect_id": effect_id,
            "attempt_id": attempt_id,
            "reason": "the run was concluded from its journal while the attempt \
                       was in flight; completion cannot be established",
        });
        let event_type = EventType::EffectIndeterminate;
        let closed = fenced_append(
            store,
            run_id,
            &head,
            &head.event_id,
            event_type,
            payload,
            Some(attempt_id),
        );
        concluded_or_alive(run_id, closed)?;
        events = store.load(run_id)?;
    }
    Ok(events)
}

/// The fence's verdict, read as `conclude` must read it: a head that
/// moved beneath a conclusion is not a race to win but evidence the run
/// is alive, so it refuses with the look-first instruction instead of
/// retrying against a journal something else is writing.
fn concluded_or_alive(
    run_id: &str,
    written: Result<EventEnvelope, StoreError>,
) -> Result<EventEnvelope, EngineError> {
    match written {
        Err(StoreError::HeadMoved { .. }) => Err(EngineError::Other(format!(
            "conclude: the journal moved beneath the conclusion of run '{run_id}', \
             so something may still be driving it — a conclusion is for a run \
             believed dead; look with `brokkr runs` before closing"
        ))),
        other => Ok(other?),
    }
}

/// Where a run with an accepted stop stands, read off the cursor a
/// re-fold produced rather than predicted: `None` at `Cursor::Stop` —
/// the position `run/stopped` belongs at — and the in-flight attempt the
/// ride must close first otherwise. Under an accepted stop the fold
/// admits no third position, so any other cursor is a fold or engine
/// defect and refuses rather than guessing at a conclusion.
pub(super) fn riding_attempt(
    run_id: &str,
    state: &RunState,
) -> Result<Option<(String, String)>, EngineError> {
    match &state.cursor {
        Cursor::Stop => Ok(None),
        Cursor::EffectInFlight {
            effect_id,
            attempt_id,
            ..
        } if state.riding_stop => Ok(Some((effect_id.clone(), attempt_id.clone()))),
        cursor => Err(EngineError::Other(format!(
            "conclude: run '{run_id}' stands at {cursor:?} with an accepted stop; \
             a stop reaches Stop or rides an in-flight attempt and nothing else"
        ))),
    }
}

/// Apply a command received through the Looper producer bridge. The command id
/// is supplied by Looper, the expected cursor/hash fences concurrent operator
/// activity, and both acceptance and rejection become Brokkr journal evidence
/// before any control-state effect is possible. The command is written
/// against the head the deciding fold read and the cursor check covered,
/// and the acceptance against the command ([`Store::append_next_if_head`]),
/// so an engine append between that check and either write loses the
/// fence instead of slipping under it.
///
/// The bridge parsed Looper's word into a [`CommandWord`] at its edge: a
/// word outside the set is journaled as sent and refused as
/// [`Refusal::CommandNotAllowed`], so the evidence says what was asked.
pub fn apply_fenced_operator_command(
    store: &mut Store,
    run_id: &str,
    command: &FencedCommand,
) -> Result<FencedCommandOutcome, EngineError> {
    apply_fenced_windows(store, run_id, command, |_| {}, |_| {})
}

/// The refusal a bridge command meets before it is journaled, if any, in
/// the order the bridge has always answered: an unknown word, a stale
/// cursor, a run that is not parked, and then `fold`'s own rule. A
/// parked run is non-terminal, so what [`acceptance_refusal`] can still
/// add is a retry with no phase to return to.
fn fenced_refusal(state: &RunState, command: CommandWord, cursor_holds: bool) -> Option<Refusal> {
    let CommandWord::Known(command) = command else {
        return Some(Refusal::CommandNotAllowed);
    };
    if !cursor_holds {
        return Some(Refusal::StaleCursor);
    }
    if state.status != Status::AwaitingOperator {
        return Some(Refusal::RunNotAwaitingOperator);
    }
    acceptance_refusal(state, command)
}

/// [`apply_fenced_operator_command`] with the acceptance's window held
/// open: `between` runs after `operator/commanded` lands and before the
/// acceptance is written against it.
#[cfg(test)]
pub(super) fn apply_fenced_racing(
    store: &mut Store,
    run_id: &str,
    wire: &FencedCommand,
    between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    apply_fenced_windows(store, run_id, wire, |_| {}, between)
}

/// [`apply_fenced_operator_command`] with both windows held open — the
/// instants [`Store::append_next_if_head`]'s fence exists for:
/// `before_command` runs after the deciding fold and before
/// `operator/commanded` is written on its head, and `between` after the
/// command lands and before the acceptance is written against it.
/// Production passes no-ops; tests pass a peer.
pub(super) fn apply_fenced_windows(
    store: &mut Store,
    run_id: &str,
    wire: &FencedCommand,
    mut before_command: impl FnMut(&mut Store),
    mut between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    let (command_id, operator) = (wire.command_id, wire.operator);
    let events = store.load(run_id)?;
    if let Some(receipt) = receipt::journaled(store, run_id, &events, command_id, operator)? {
        return Ok(receipt);
    }
    let state = fold(&events)?;
    // The head this fold read: the one the wire's cursor is checked
    // against, and the one the command must still find (decision 0029).
    let head = Head::tip(&events);
    let cursor_holds = head.seq == wire.expected_seq && head.hash == wire.expected_hash;
    let rejection = fenced_refusal(&state, wire.command, cursor_holds);
    before_command(store);
    // A peer appended after the fold, so what was decided on it stands on
    // a head that is gone: nothing of the command is written, no winner is
    // picked, and the caller re-reads and re-issues.
    let Some(commanded) = fenced_commanded(store, run_id, wire, &head)? else {
        let refused = stale_refusal(store, run_id, wire, &head.event_id)?;
        read_back(store, run_id)?;
        return Ok(refused);
    };
    between(store);
    let cause = &commanded.event_id;
    let disposition = match rejection {
        Some(refusal) => {
            let rejection = Rejection {
                command_id,
                operator,
                refusal,
                cause,
            };
            receipt::refuse_undisposed(store, run_id, rejection)?
        }
        None => fenced_acceptance(store, run_id, wire, &commanded)?,
    };
    // Prove the newly appended pair does not corrupt fold semantics before the
    // bridge acknowledges it to Looper.
    read_back(store, run_id)?;
    Ok(disposition)
}

/// `operator/commanded` written on the head its deciding fold read, or
/// `None` when a peer has moved that head and nothing was written.
fn fenced_commanded(
    store: &mut Store,
    run_id: &str,
    wire: &FencedCommand,
    head: &Head,
) -> Result<Option<EventEnvelope>, StoreError> {
    let (id, word, operator) = (wire.command_id, wire.command.as_str(), wire.operator);
    let payload = json!({"command_id": id, "command": word, "args": {}, "operator": operator});
    let (cause, kind) = (&head.event_id, EventType::OperatorCommanded);
    match fenced_append(store, run_id, head, cause, kind, payload, None) {
        Err(StoreError::HeadMoved { .. }) => Ok(None),
        written => written.map(Some),
    }
}

/// The refusal a fenced write that lost its head journals, caused by
/// `cause` — unless what moved the head disposed of this same command id,
/// a redelivery racing this one: that disposition is this delivery's
/// receipt too, so it answers with it and writes nothing. The refusal
/// lands only on the head that lookup read ([`receipt::refuse_undisposed`]).
fn stale_refusal(
    store: &mut Store,
    run_id: &str,
    wire: &FencedCommand,
    cause: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    let rejection = Rejection {
        command_id: wire.command_id,
        operator: wire.operator,
        refusal: Refusal::StaleCursor,
        cause,
    };
    receipt::refuse_undisposed(store, run_id, rejection)
}

/// The acceptance, written against the head the cursor check covers — the
/// command itself — so a peer's append in between cannot slip under it.
/// It is not re-decided the way the unfenced path re-decides: this caller
/// HOLDS a cursor, and deciding against a head they never saw is what the
/// fence exists to prevent. They re-read and re-issue; the journal says why.
fn fenced_acceptance(
    store: &mut Store,
    run_id: &str,
    wire: &FencedCommand,
    commanded: &EventEnvelope,
) -> Result<FencedCommandOutcome, EngineError> {
    let head = Head::of(commanded);
    let (cause, kind) = (&commanded.event_id, EventType::OperatorAccepted);
    let payload =
        json!({"command_id": wire.command_id, "operator": wire.operator, "reason": wire.reason});
    match fenced_append(store, run_id, &head, cause, kind, payload, None) {
        Ok(_) => accepted(store, run_id),
        Err(StoreError::HeadMoved { .. }) => stale_refusal(store, run_id, wire, cause),
        Err(error) => Err(error.into()),
    }
}
