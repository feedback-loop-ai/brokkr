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
                recorded_at: "2026-01-01T00:00:00Z",
            },
            "{payload}"
        );
    }
}

/// A row's time is its envelope's stamp, and a stamp past the bound is
/// cut at the character boundary at or below it, as the summary cuts one.
#[test]
fn an_event_s_row_holds_its_stamp_cut_as_the_summary_cuts_it() {
    let long = format!("{}é{}", "2".repeat(STAMP_BYTES - 1), "z".repeat(9));
    let cases = [
        (
            "2026-10-21T08:00:00.123+02:00",
            "2026-10-21T08:00:00.123+02:00",
        ),
        (long.as_str(), &long[..STAMP_BYTES - 1]),
    ];
    for (stamp, cut) in cases {
        let event = EnvelopeBuilder::new(EventType::RunStarted, json!({}))
            .at(stamp)
            .build();
        assert_eq!(Indexed::of(&event).recorded_at, cut, "{stamp}");
    }
}
