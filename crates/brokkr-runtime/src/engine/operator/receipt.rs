//! A bridge command's receipt: what the journal already records for its
//! command id. A redelivery answers with it, and so does a delivery of
//! the same id that lost the fence to it (decision 0029), so every
//! delivery of one command reads one disposition.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{terminal_event, ReceiptReason, Refusal};
use brokkr_core::EventEnvelope;
use brokkr_store::{Store, StoreError};
use serde_json::Value;

use super::{
    accepted_at, command_id_of, fenced_append, read_back, EngineError, FencedCommandOutcome, Head,
    Rejection, FENCE_ATTEMPTS,
};

/// The `operator/commanded` journaled for `command_id`, if any.
fn commanded_of<'e>(events: &'e [EventEnvelope], command_id: &str) -> Option<&'e EventEnvelope> {
    events.iter().find(|event| {
        event.event_type == EventType::OperatorCommanded && command_id_of(event) == Some(command_id)
    })
}

/// The disposition journaled for `command_id`: after its command, or
/// anywhere when it was refused before its command could be journaled.
fn recorded(
    events: &[EventEnvelope],
    commanded: Option<&EventEnvelope>,
    command_id: &str,
) -> Option<FencedCommandOutcome> {
    let disposition = events.iter().find(|event| {
        commanded.is_none_or(|commanded| event.seq > commanded.seq)
            && matches!(
                event.event_type,
                EventType::OperatorAccepted | EventType::OperatorRejected
            )
            && command_id_of(event) == Some(command_id)
    })?;
    Some(if disposition.event_type == EventType::OperatorAccepted {
        accepted_at(disposition)
    } else {
        let reason = disposition
            .payload
            .get("reason")
            .and_then(Value::as_str)
            .and_then(Refusal::parse)
            .map(ReceiptReason::Refusal)
            .unwrap_or(ReceiptReason::Refusal(Refusal::PreviouslyRejected));
        FencedCommandOutcome::Rejected {
            reason: reason.word().to_string(),
            head_seq: disposition.seq,
            head_hash: disposition.event_hash.clone(),
        }
    })
}

/// The disposition `command_id` already holds, read and never written:
/// a command journaled with no disposition yet holds none.
fn disposition(events: &[EventEnvelope], command_id: &str) -> Option<FencedCommandOutcome> {
    recorded(events, commanded_of(events, command_id), command_id)
}

/// The answer a word that is not journaled gets: a rejected receipt at
/// the head the door found. The one home for both the reserved word and
/// the terminal word the door writes nothing for (decision 0108 ruling 4,
/// #440 fix round 440c4).
pub(super) fn answered(reason: ReceiptReason, head: &Head) -> FencedCommandOutcome {
    FencedCommandOutcome::Rejected {
        reason: reason.word().to_string(),
        head_seq: head.seq,
        head_hash: head.hash.clone(),
    }
}

/// The head a receipt answers at: the journal's tip, or the empty run's
/// head before its first event.
fn tip(events: &[EventEnvelope]) -> Head {
    events.last().map_or_else(
        || Head {
            seq: 0,
            hash: ZERO_HASH.to_string(),
            event_id: String::new(),
        },
        Head::of,
    )
}

/// A reserved annotation word's answer at a journal's tip, with nothing
/// journaled. The one home for both the fresh word and the redelivery an
/// older binary left undisposed (decision 0108 ruling 4, #440 fix round
/// 440c4).
pub(super) fn reserved_receipt(events: &[EventEnvelope]) -> FencedCommandOutcome {
    answered(ReceiptReason::ReservedCommand, &tip(events))
}

/// A journaled late word's answer at a journal's tip, with nothing
/// journaled: after the terminal event the word is a late event, so no
/// door writes its disposition on a remote caller's behalf. The reason is
/// the fold error the word would mint (decision 0108 ruling 4, #440 fix
/// round 440c4).
fn late_receipt(events: &[EventEnvelope]) -> FencedCommandOutcome {
    answered(ReceiptReason::Refusal(Refusal::AfterTerminal), &tip(events))
}

/// Does the journal hold a terminal event before `seq`? A word after that
/// event is a late event; before it, the word is the pending command it
/// is.
fn late_after(events: &[EventEnvelope], seq: u64) -> bool {
    events
        .iter()
        .any(|event| event.seq < seq && terminal_event(event.event_type).is_some())
}

/// The receipt a command id already holds, if any: a journaled command
/// replays, and one refused stale before it was journaled answers with
/// that refusal, its only record, so a redelivery reads the same.
pub(super) fn journaled(
    store: &mut Store,
    run_id: &str,
    events: &[EventEnvelope],
    command_id: &str,
    operator: &str,
) -> Result<Option<FencedCommandOutcome>, EngineError> {
    match commanded_of(events, command_id) {
        Some(commanded) => replay(store, run_id, events, commanded, command_id, operator).map(Some),
        None => Ok(recorded(events, None, command_id)),
    }
}

/// A journaled command's recorded disposition, or — none after it — a
/// refusal of the incomplete replay. A reserved annotation word is the
/// exception: its journaled command has no disposition to replay, so it
/// is answered with the reserved refusal and nothing is written for it.
/// So is a word the fold reads as late after the terminal event, reserved
/// or not: no door writes a late event on a remote caller's behalf. An
/// older binary could journal the word and crash between its two appends;
/// this delivery must not add the rejection it never wrote (decision 0108
/// ruling 4, #440 fix round 440c4).
fn replay(
    store: &mut Store,
    run_id: &str,
    events: &[EventEnvelope],
    commanded: &EventEnvelope,
    command_id: &str,
    operator: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    if let Some(receipt) = recorded(events, Some(commanded), command_id) {
        return Ok(receipt);
    }
    if brokkr_core::fold::reserved_word(commanded).is_some() {
        return Ok(reserved_receipt(events));
    }
    if brokkr_core::fold::is_late_word(commanded) && late_after(events, commanded.seq) {
        return Ok(late_receipt(events));
    }
    let rejection = Rejection {
        command_id,
        operator,
        refusal: Refusal::IncompleteCommandReplay,
        cause: &commanded.event_id,
    };
    let refused = refuse_undisposed(store, run_id, rejection)?;
    read_back(store, run_id)?;
    Ok(refused)
}

/// Journal `rejection` on the head the lookup that found its command
/// undisposed read (decision 0029). A delivery of the same id that
/// disposes of it in between takes that head away, and the lookup is made
/// again: its disposition is this delivery's receipt too, so one command
/// reads one disposition. A head moved on every turn of
/// [`FENCE_ATTEMPTS`] refuses with [`EngineError::JournalMoved`] and
/// writes nothing.
pub(super) fn refuse_undisposed(
    store: &mut Store,
    run_id: &str,
    rejection: Rejection,
) -> Result<FencedCommandOutcome, EngineError> {
    let (payload, cause) = (rejection.payload(), rejection.cause);
    let mut lost = 0;
    loop {
        let events = store.load(run_id)?;
        if let Some(receipt) = disposition(&events, rejection.command_id) {
            return Ok(receipt);
        }
        let head = Head::tip(&events);
        #[cfg(test)]
        super::super::operator_tests::before_refusal(store);
        let kind = EventType::OperatorRejected;
        match fenced_append(store, run_id, &head, cause, kind, payload.clone(), None) {
            Err(StoreError::HeadMoved { .. }) if lost < FENCE_ATTEMPTS => lost += 1,
            Err(StoreError::HeadMoved {
                expected_seq,
                found_seq,
            }) => {
                let run_id = run_id.to_string();
                return Err(EngineError::JournalMoved {
                    run_id,
                    expected_seq,
                    found_seq,
                });
            }
            written => return Ok(rejection.landed(written?)),
        }
    }
}
