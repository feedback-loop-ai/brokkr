//! The `operator/commanded` words a fold or a readout reads as an
//! operator annotation, so a command that merely carries one cannot pass
//! for the annotation: `repair` (decision 0108 ruling 4), `classify`
//! (decision 0064 ruling 5), `supersede` (decision 0047) and `settle`
//! (decision 0108 ruling 3, unit 3b). The bridge's door refuses every one
//! before it journals it; the list has one home here, and each word
//! answers for its own record through one typed match.

use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};
use crate::fold::{FoldError, OperatorCommand, RunState};

/// The command word an operator's settle record carries: `operator/commanded`
/// with this word and the args of contracts/operator-settle.v1.schema.json
/// (decision 0108 ruling 3, in ruling 4's erratum shape after decision 0047).
pub const SETTLE: &str = "settle";

/// The annotation words, the closed set a fold or readout reads as an
/// annotation. One list, matched exhaustively: a word added to it must
/// answer for its own record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationWord {
    Repair,
    Classify,
    Supersede,
    Settle,
}

impl AnnotationWord {
    /// Every word, in the order the bridge's list has always named them.
    const ALL: [AnnotationWord; 4] = [
        AnnotationWord::Repair,
        AnnotationWord::Classify,
        AnnotationWord::Supersede,
        AnnotationWord::Settle,
    ];

    /// The word `operator/commanded` carries.
    const fn as_str(self) -> &'static str {
        match self {
            AnnotationWord::Repair => super::REPAIR,
            AnnotationWord::Classify => crate::ending::CLASSIFY,
            AnnotationWord::Supersede => crate::residual::SUPERSEDE,
            AnnotationWord::Settle => SETTLE,
        }
    }

    /// The annotation `word` names, when it is one.
    fn parse(word: &str) -> Option<AnnotationWord> {
        Self::ALL
            .into_iter()
            .find(|annotation| annotation.as_str() == word)
    }

    /// Does `event` carry THIS word's record: the args its published
    /// contract describes? `repair` with args other than exactly `{}` is
    /// a record; `classify` and `supersede` with the args a fold or
    /// readout reads are. No fold or readout reads a `settle` record: the
    /// broker's release reads it by type, never through the fold.
    fn is_record(self, event: &EventEnvelope) -> bool {
        match self {
            AnnotationWord::Repair => super::late::is_repair(event),
            AnnotationWord::Classify => event
                .payload
                .get("args")
                .and_then(crate::ending::operator_ending)
                .is_some(),
            AnnotationWord::Supersede => crate::residual::closing_of(event).is_some(),
            AnnotationWord::Settle => false,
        }
    }
}

/// Is `word` reserved for an operator annotation? The bridge asks before
/// it journals a command, so a remote word can never reach the fold or a
/// readout as an annotation nobody wrote.
pub fn reserved_annotation(word: &str) -> bool {
    AnnotationWord::parse(word).is_some()
}

/// The reserved annotation word `event` carries, if it is an
/// `operator/commanded` whose command word is one. What the bridge's
/// refusal and the fold's late-word rule read.
pub fn reserved_word(event: &EventEnvelope) -> Option<&'static str> {
    if event.event_type != EventType::OperatorCommanded {
        return None;
    }
    let word = event.payload.get("command").and_then(Value::as_str)?;
    let annotation = AnnotationWord::parse(word)?;
    Some(annotation.as_str())
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
    let annotation = AnnotationWord::parse(word)?;
    annotation.is_record(event).then_some(word)
}

/// Is the `operator/commanded` `event` an annotation a terminal run
/// admits? A recognized record, or a command word the engine knows, whose
/// journaled refusal evidence must not break the run. Anything else is
/// decided by [`is_late_word`], its one caller, which asks the event's
/// type first.
pub(crate) fn terminal_annotation(event: &EventEnvelope) -> bool {
    if record_word(event).is_some() {
        return true;
    }
    event
        .payload
        .get("command")
        .and_then(Value::as_str)
        .and_then(OperatorCommand::parse)
        .is_some()
}

/// Is `event` one the fold reads as a late event when it stands after the
/// terminal one? The one late-word rule: `operator/commanded` carrying a
/// command word and exactly the bridge's empty `args` object, and no
/// annotation record and no command the engine knows.
///
/// A word with non-empty args this build does not read is NOT late: it is
/// admitted and not read, as an annotation always was (decision 0064
/// ruling 5), so a newer binary's word does not break this build's fold.
/// A repair word with malformed non-empty args is a record the fold
/// refuses by name, not a late word (decision 0108 ruling 4, #440 fix
/// round 440c4).
pub fn is_late_word(event: &EventEnvelope) -> bool {
    event.event_type == EventType::OperatorCommanded
        && !terminal_annotation(event)
        && bridge_empty_args(event)
}

/// Does `event` carry the bridge's own command shape: a command word and
/// exactly the empty `args` object it writes for a word it does not
/// execute? No door ever wrote a record that way.
fn bridge_empty_args(event: &EventEnvelope) -> bool {
    event
        .payload
        .get("command")
        .and_then(Value::as_str)
        .is_some()
        && matches!(event.payload.get("args"), Some(Value::Object(args)) if args.is_empty())
}

/// The refusal a terminal run gives a late word: it is a late event, not
/// the annotation the relation's terminal column admits, and `fold`
/// refuses it until a repair names it. Before the terminal event a word is
/// the pending command it is, and answers `Ok` here (decision 0108 ruling
/// 4, #440 fix round 440c4).
pub(crate) fn late_refusal(state: &RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    if state.terminal_seq.is_some() && is_late_word(event) {
        return Err(FoldError::AfterTerminal { seq: event.seq });
    }
    Ok(())
}
