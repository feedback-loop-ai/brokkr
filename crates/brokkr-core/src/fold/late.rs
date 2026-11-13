//! The fold's reading of an operator's repair record (decision 0108
//! ruling 4, in the shape its 2026-11-13 erratum gives it after decision
//! 0047): `operator/commanded` with command `repair`, naming the late
//! tail of a run already broken by an event after its terminal one. The
//! tail is kept and shown, and it is never applied.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{apply, terminal_event, FoldError, RunState};
use crate::envelope::{EventEnvelope, EventType};

/// The command word a repair annotation carries: `operator/commanded`
/// with this word and `crate::payload::RepairArgs`, never a new event
/// type (decision 0108 ruling 4's erratum, following decision 0047).
pub const REPAIR: &str = "repair";

/// A run's late tail: the events after its terminal one that an
/// operator's repair record names, kept and shown, never applied.
#[derive(Debug, Clone, PartialEq)]
pub struct LateTail {
    /// The seq of the repair annotation that named this tail.
    pub repaired_at: u64,
    /// The run's terminal event, which the tail follows.
    pub terminal_seq: u64,
    /// The tail's first seq: the event after the terminal one.
    pub first_seq: u64,
    /// The tail's last seq: the head the repair was written at, less one.
    pub last_seq: u64,
    /// The tail's event types, in order.
    pub event_types: Vec<EventType>,
    /// The operator whose repair named it.
    pub operator: String,
}

impl LateTail {
    /// The tail's events, seq and type, in order: what `inspect` lists.
    pub fn events(&self) -> impl Iterator<Item = (u64, EventType)> + '_ {
        (self.first_seq..=self.last_seq).zip(self.event_types.iter().copied())
    }
}

/// `state` carried over `suffix`: repairs in `suffix` are read before any
/// event is applied, and the events they name fold as late.
///
/// A repair the state's prefix already holds is checked against the state:
/// the seqs before the carry are the run's own, so their types are not
/// read again. A repair wholly inside the suffix — the way `fold` reads a
/// whole journal, and the way any journal is read here for the first time
/// — is checked against every event it names.
pub(super) fn onto(mut state: RunState, suffix: &[EventEnvelope]) -> Result<RunState, FoldError> {
    let repaired = repair_of(&state, suffix)?;
    if let Some(tail) = repaired {
        state.late_tail = Some(tail);
    }
    for event in suffix {
        state.seq = event.seq;
        state.last_hash = event.event_hash.clone();
        if event.seq == 1 {
            continue;
        }
        record_tail_type(&mut state, event);
        if is_late(&state, event.seq) {
            continue;
        }
        apply(&mut state, event)?;
        record_terminal(&mut state, event);
    }
    Ok(state)
}

/// The last repair `suffix` carries that names exactly the late tail of
/// the run `state` describes. A repair that names another range is
/// refused by name. A `repair` command standing before the terminal
/// event is no repair record at all: it is the pending command the fold
/// has always read `operator/commanded` as, and the bridge's own door
/// keeps such a word out of the journal.
fn repair_of(state: &RunState, suffix: &[EventEnvelope]) -> Result<Option<LateTail>, FoldError> {
    let terminal = match state.terminal_seq {
        Some(seq) => Some(seq),
        None => suffix
            .iter()
            .find(|event| terminal_event(event.event_type).is_some())
            .map(|event| event.seq),
    };
    let Some(terminal) = terminal else {
        return Ok(None);
    };
    // The events before the carry are the state's own, so their types are
    // carried on it; the suffix's types are read here. A repair names the
    // whole tail, so both halves are checked, never only the suffix.
    let mut types = state.tail_types.clone();
    types.extend(
        suffix
            .iter()
            .filter(|event| event.seq > terminal)
            .map(|event| (event.seq, event.event_type)),
    );
    let mut honoured = None;
    for event in suffix
        .iter()
        .filter(|event| event.seq > terminal && is_repair(event))
    {
        honoured = Some(named(event, terminal, &types)?);
    }
    Ok(honoured)
}

/// Is `event` an `operator/commanded` carrying the repair word?
fn is_repair(event: &EventEnvelope) -> bool {
    (event.event_type == EventType::OperatorCommanded)
        & (event.payload.get("command").and_then(Value::as_str) == Some(REPAIR))
}

/// The late tail `event` names, checked against the run's terminal event
/// at `terminal` and the types the run holds after it in `types`: every
/// seq between the terminal and the repair, and no other.
fn named(
    event: &EventEnvelope,
    terminal: u64,
    types: &BTreeMap<u64, EventType>,
) -> Result<LateTail, FoldError> {
    let refused = |detail: String| FoldError::BadRepair {
        seq: event.seq,
        detail,
    };
    if event.seq == terminal + 1 {
        return Err(refused(format!(
            "it names no late tail: seq {} is the terminal event and seq {} is the repair",
            terminal, event.seq
        )));
    }
    let args = args_of(event)?;
    if args.terminal_seq != terminal {
        return Err(refused(format!(
            "it names the terminal event at seq {}, but the run's terminal event is at seq {terminal}",
            args.terminal_seq
        )));
    }
    let (first_seq, last_seq) = (terminal + 1, event.seq - 1);
    if (args.first_seq, args.last_seq) != (first_seq, last_seq) {
        return Err(refused(format!(
            "it names seqs {}..={}, but the late tail is seqs {first_seq}..={last_seq}",
            args.first_seq, args.last_seq
        )));
    }
    let expected = (last_seq - first_seq + 1) as usize;
    if args.event_types.len() != expected {
        return Err(refused(format!(
            "it names {} event type(s), but the late tail holds {expected}",
            args.event_types.len()
        )));
    }
    for (at, seq) in (first_seq..=last_seq).enumerate() {
        let Some(kind) = types.get(&seq) else {
            return Err(refused(format!(
                "it names seq {seq}, which the run does not hold"
            )));
        };
        let named = args.event_types[at];
        if named != *kind {
            return Err(refused(format!(
                "it names seq {seq} as {}, but the run holds {}",
                named.as_str(),
                kind.as_str()
            )));
        }
    }
    Ok(LateTail {
        repaired_at: event.seq,
        terminal_seq: terminal,
        first_seq,
        last_seq,
        event_types: args.event_types,
        operator: operator_of(event)?,
    })
}

/// The repair's `args`, typed by the published payload contract.
fn args_of(event: &EventEnvelope) -> Result<crate::payload::RepairArgs, FoldError> {
    let refused = |detail: String| FoldError::BadRepair {
        seq: event.seq,
        detail,
    };
    let args = event
        .payload
        .get("args")
        .ok_or_else(|| refused("it carries no args".to_string()))?;
    serde_json::from_value(args.clone())
        .map_err(|error| refused(format!("its args do not read: {error}")))
}

/// The operator the repair is attributed to.
fn operator_of(event: &EventEnvelope) -> Result<String, FoldError> {
    event
        .payload
        .get("operator")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| FoldError::BadRepair {
            seq: event.seq,
            detail: "it carries no operator".to_string(),
        })
}

/// Is `seq` inside the late tail the state already honours?
fn is_late(state: &RunState, seq: u64) -> bool {
    state
        .late_tail
        .as_ref()
        .is_some_and(|tail| (tail.first_seq..=tail.last_seq).contains(&seq))
}

/// Remember the seq of the event that concluded the run: the anchor every
/// later repair is checked against.
fn record_terminal(state: &mut RunState, event: &EventEnvelope) {
    if terminal_event(event.event_type).is_some() {
        state.terminal_seq = Some(event.seq);
    }
}

/// Carry the type of an event after the terminal one, so a repair read
/// from the rest of the journal is checked against the same types the
/// whole fold reads.
fn record_tail_type(state: &mut RunState, event: &EventEnvelope) {
    if state
        .terminal_seq
        .is_some_and(|terminal| event.seq > terminal)
    {
        state.tail_types.insert(event.seq, event.event_type);
    }
}

#[cfg(test)]
mod tests;
