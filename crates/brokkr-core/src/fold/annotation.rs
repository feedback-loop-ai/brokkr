//! The `operator/commanded` words a fold or a readout reads as an
//! operator annotation, so a command that merely carries one cannot pass
//! for the annotation: `repair` (decision 0108 ruling 4), `classify`
//! (decision 0064 ruling 5), `supersede` (decision 0047) and `settle`
//! (decision 0108 ruling 3, unit 3b). The bridge's door refuses every one
//! before it journals it; the list has one home here.

use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};
use crate::fold::{FoldError, OperatorCommand, RunState};

/// The annotation words, one list the bridge reads.
pub(crate) const RESERVED_ANNOTATIONS: [&str; 4] = [
    super::REPAIR,
    crate::ending::CLASSIFY,
    crate::residual::SUPERSEDE,
    // 440d builds `operator settle`; a command word cannot stand in for it.
    "settle",
];

/// Is `word` reserved for an operator annotation? The bridge asks before
/// it journals a command, so a remote word can never reach the fold or a
/// readout as an annotation nobody wrote.
pub fn reserved_annotation(word: &str) -> bool {
    reserved_word_of(word).is_some()
}

/// The reserved annotation word `event` carries, if it is an
/// `operator/commanded` whose command word is one. What the bridge's
/// refusal and the fold's late-word rule read.
pub fn reserved_word(event: &EventEnvelope) -> Option<&'static str> {
    if event.event_type != EventType::OperatorCommanded {
        return None;
    }
    let word = event.payload.get("command").and_then(Value::as_str)?;
    reserved_word_of(word)
}

/// `word` as the one list's own constant, when it is reserved.
fn reserved_word_of(word: &str) -> Option<&'static str> {
    RESERVED_ANNOTATIONS
        .into_iter()
        .find(|reserved| *reserved == word)
}

/// The reserved annotation word `event` carries when its `args` are the
/// record its contract describes: `repair` with args other than `{}` (the
/// bridge's own shape, which never wrote a record that way), `classify`
/// with operator-ending args, and `supersede` with a citation and
/// findings. `settle` has no reader until 440d builds it. A reserved word
/// with no record is an ordinary command word, never an annotation
/// (decision 0108 ruling 4, #440 fix round 440c4).
pub(crate) fn record_word(event: &EventEnvelope) -> Option<&'static str> {
    let word = reserved_word(event)?;
    let record = match word {
        super::REPAIR => super::late::is_repair(event),
        crate::ending::CLASSIFY => event
            .payload
            .get("args")
            .and_then(crate::ending::operator_ending)
            .is_some(),
        crate::residual::SUPERSEDE => crate::residual::closing_of(event).is_some(),
        _ => false,
    };
    record.then_some(word)
}

/// Is `event` an annotation a terminal run admits? A recognized record,
/// or a command word the engine knows, whose journaled refusal evidence
/// must not break the run. Anything else — `dance`, or a reserved
/// annotation word carrying no record — is a late event after the
/// terminal one: kept and shown only when a repair names it.
pub(crate) fn terminal_annotation(event: &EventEnvelope) -> bool {
    if record_word(event).is_some() {
        return true;
    }
    event.event_type == EventType::OperatorCommanded
        && event
            .payload
            .get("command")
            .and_then(Value::as_str)
            .and_then(OperatorCommand::parse)
            .is_some()
}

/// The refusal a terminal run gives a command word that carries no
/// annotation record: it is a late event, not the annotation the
/// relation's terminal column admits, and `fold` refuses it until a
/// repair names it. Before the terminal event a word is the pending
/// command it is, and answers `Ok` here (decision 0108 ruling 4, #440 fix
/// round 440c4).
pub(crate) fn late_refusal(state: &RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    if state.terminal_seq.is_some()
        && event.event_type == EventType::OperatorCommanded
        && !terminal_annotation(event)
    {
        return Err(FoldError::AfterTerminal { seq: event.seq });
    }
    Ok(())
}
