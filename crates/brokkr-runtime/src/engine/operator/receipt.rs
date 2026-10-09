//! A bridge command's receipt: what the journal already records for its
//! command id. A redelivery answers with it, and so does a delivery of
//! the same id that lost the fence to it (decision 0029), so every
//! delivery of one command reads one disposition.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Refusal;
use brokkr_core::EventEnvelope;
use brokkr_store::Store;
use serde_json::{json, Value};

use super::{command_id_of, read_back, EngineError, FencedCommandOutcome};

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
        FencedCommandOutcome::Accepted {
            head_seq: disposition.seq,
            head_hash: disposition.event_hash.clone(),
        }
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
pub(super) fn disposition(
    events: &[EventEnvelope],
    command_id: &str,
) -> Option<FencedCommandOutcome> {
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
    let incomplete = Refusal::IncompleteCommandReplay.word();
    let rejected = store.append_next(
        run_id,
        EventType::OperatorRejected,
        json!({
            "command_id": command_id,
            "operator": operator,
            "reason": incomplete,
        }),
        Some(commanded.event_id.clone()),
        None,
    )?;
    read_back(store, run_id)?;
    Ok(FencedCommandOutcome::Rejected {
        reason: incomplete.into(),
        head_seq: rejected.seq,
        head_hash: rejected.event_hash,
    })
}
