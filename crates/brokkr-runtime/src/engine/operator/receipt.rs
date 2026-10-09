//! A bridge command's receipt: what the journal already records for its
//! command id. A redelivery answers with it, and so does a delivery of
//! the same id that lost the fence to it (decision 0029), so every
//! delivery of one command reads one disposition.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Refusal;
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
        FencedCommandOutcome::Rejected {
            reason: disposition
                .payload
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or(Refusal::PreviouslyRejected.word())
                .to_string(),
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
/// refusal of the incomplete replay.
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
