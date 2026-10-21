use super::*;
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;

/// An event of `event_type` carrying `payload`, at seq 7, its envelope
/// naming an attempt of its own that no reader keys an attempt by.
fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload)
        .seq(7)
        .attempt(Some("the envelope's"))
        .build()
}

/// Each event's row: the payload's attempt, the checkpoint's member or,
/// with no checkpoint, the result's, each only where it is text.
#[test]
fn an_event_s_row_reads_the_payload_s_attempt_and_its_record_s_member() {
    let checkpointed = EventType::EffectCheckpointed;
    let succeeded = EventType::EffectSucceeded;
    let cases = [
        (
            checkpointed,
            json!({"attempt_id": "a1", "checkpoint": {"member": "m1"}, "result": {"member": "r"}}),
            (Some("a1"), Some("m1")),
        ),
        (
            succeeded,
            json!({"attempt_id": "a1", "result": {"member": "m2"}}),
            (Some("a1"), Some("m2")),
        ),
        (
            checkpointed,
            json!({"attempt_id": "a1", "checkpoint": {}, "result": {"member": "r"}}),
            (Some("a1"), None),
        ),
        (
            checkpointed,
            json!({"attempt_id": "a1", "checkpoint": null, "result": {"member": "r"}}),
            (Some("a1"), None),
        ),
        (
            checkpointed,
            json!({"attempt_id": 1, "checkpoint": {"member": 2}}),
            (None, None),
        ),
        (
            succeeded,
            json!({"attempt_id": "a1", "result": "resolved"}),
            (Some("a1"), None),
        ),
        (EventType::RunStarted, json!(["attempt_id"]), (None, None)),
    ];
    for (event_type, payload, (attempt_id, member)) in cases {
        let event = event(event_type, payload.clone());
        assert_eq!(
            Indexed::of(&event),
            Indexed {
                seq: 7,
                event_type,
                attempt_id,
                member,
            },
            "{payload}"
        );
    }
}
