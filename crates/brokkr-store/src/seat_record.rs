//! Validation of decision 0034's frozen seat-record contract.
//!
//! The schema is embedded beside this module so a packaged `brokkr`
//! remains an offline verifier. A test pins that embedded copy to the
//! published file in `contracts/`; they cannot drift inside this tree.

use std::sync::OnceLock;

use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;
use thiserror::Error;

const SCHEMA: &str = include_str!("seat-record.v1.schema.json");

static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();

#[derive(Debug, Error, PartialEq, Eq)]
#[error("seat record at journal seq {seq} violates contracts/seat-record.v1.schema.json at {path}")]
pub struct SeatRecordError {
    pub seq: u64,
    pub path: String,
}

fn validator() -> &'static jsonschema::Validator {
    VALIDATOR.get_or_init(|| {
        let schema: Value =
            serde_json::from_str(SCHEMA).expect("the embedded seat-record schema is JSON");
        jsonschema::draft7::new(&schema).expect("the embedded seat-record schema is valid draft-07")
    })
}

/// Validate one checkpoint or successful result against the frozen v1
/// seat-record contract. Error text reports only the JSON pointer: an
/// invalid value is not echoed into diagnostics where prose could leak.
pub fn validate_seat_record(record: &Value, seq: u64) -> Result<(), SeatRecordError> {
    if validator().validate(record).is_err() {
        // The contract's top level is a `oneOf`, so a violation is always
        // reported against the record as a whole: the pointer is the root
        // and the offending value is never echoed.
        return Err(SeatRecordError {
            seq,
            path: "/".to_string(),
        });
    }
    if let (Some(input), Some(cache_read)) = (
        record.get("input_tokens").and_then(Value::as_u64),
        record.get("cache_read_tokens").and_then(Value::as_u64),
    ) {
        if cache_read > input {
            return Err(SeatRecordError {
                seq,
                path: "/cache_read_tokens".to_string(),
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_events(events: &[EventEnvelope]) -> Result<(), SeatRecordError> {
    for event in events {
        let record = match event.event_type {
            EventType::EffectCheckpointed => event.payload.get("checkpoint"),
            EventType::EffectSucceeded => event.payload.get("result"),
            _ => None,
        };
        if let Some(record) = record {
            validate_seat_record(record, event.seq)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use brokkr_core::canonical::sha256_bytes;
    use brokkr_core::envelope::EventEnvelope;
    use serde_json::json;

    fn event(seq: u64, event_type: EventType, payload: Value) -> EventEnvelope {
        EventEnvelope {
            event_schema_version: 1,
            event_id: format!("event-{seq}"),
            run_id: "run".to_string(),
            seq,
            recorded_at: "2026-09-03T00:00:00Z".to_string(),
            event_type,
            correlation_id: "run".to_string(),
            causation_id: None,
            attempt_id: None,
            payload,
            previous_hash: "0".repeat(64),
            event_hash: "1".repeat(64),
        }
    }

    #[test]
    fn embedded_schema_is_the_published_contract() {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let published =
            std::fs::read(workspace.join("contracts/seat-record.v1.schema.json")).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&published).unwrap(),
            serde_json::from_str::<Value>(SCHEMA).unwrap()
        );
        assert_eq!(sha256_bytes(&published), sha256_bytes(SCHEMA.as_bytes()));
    }

    #[test]
    fn fields_are_typed_positive_bounded_and_cache_reads_are_a_subset() {
        let valid = json!({
            "step":"seat-turn", "turn":1, "model":"claude-fable-5-1",
            "input_tokens":13, "output_tokens":2, "cache_read_tokens":3,
            "cache_write_tokens":4, "tool":"Read", "target":"src/lib.rs"
        });
        validate_seat_record(&valid, 7).unwrap();

        for invalid in [
            json!({"step":"seat-turn", "turn":0}),
            json!({"step":"seat-turn", "turn":1, "input_tokens":0}),
            json!({"step":"seat-turn", "turn":1, "model":"configured guess"}),
            json!({"step":"seat-turn", "turn":1, "tool":"x".repeat(81)}),
            json!({"step":"seat-turn", "turn":1, "target":"src/lib.rs"}),
            json!({"step":"seat-turn", "turn":1, "content":"private prose"}),
        ] {
            assert!(validate_seat_record(&invalid, 8).is_err(), "{invalid}");
        }

        let subset = json!({
            "step":"turn-completed", "turn":1,
            "input_tokens":3, "cache_read_tokens":4
        });
        let error = validate_seat_record(&subset, 9).unwrap_err();
        assert_eq!(error.seq, 9);
        assert_eq!(error.path, "/cache_read_tokens");
    }

    #[test]
    fn transcript_and_result_shapes_are_closed_without_rejecting_legacy_absence() {
        let transcript = json!({
            "step":"transcript", "model":"not reported",
            "transcript":{"kind":"codex-thread", "locator":"019c", "home":"/tmp/codex"}
        });
        validate_seat_record(&transcript, 1).unwrap();
        validate_seat_record(&json!({"step":"seat-turn", "turn":3, "tool":"Bash"}), 2).unwrap();
        validate_seat_record(
            &json!({
                "result":"complete", "inputs":{"fixed":true}, "notes":"done",
                "model":"not applicable",
                "transcript":{"kind":"none", "locator":"", "home":""}
            }),
            3,
        )
        .unwrap();

        for invalid in [
            json!({"step":"transcript", "transcript":{
                "kind":"invented", "locator":"id", "home":"/tmp"
            }}),
            json!({"step":"transcript", "transcript":{
                "kind":"none", "locator":"secret", "home":""
            }}),
            json!({"result":"complete", "unexpected":"prose"}),
        ] {
            assert!(validate_seat_record(&invalid, 4).is_err(), "{invalid}");
        }
    }

    #[test]
    fn event_validation_checks_only_checkpoints_and_successful_results() {
        let events = vec![
            event(1, EventType::RunStarted, json!({"anything":"outside"})),
            event(
                2,
                EventType::EffectCheckpointed,
                json!({"checkpoint":{"step":"working"}}),
            ),
            event(
                3,
                EventType::EffectSucceeded,
                json!({"result":{"result":"complete"}}),
            ),
        ];
        validate_events(&events).unwrap();

        let invalid = [event(
            4,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{"step":"seat-turn", "turn":"one"}}),
        )];
        assert_eq!(validate_events(&invalid).unwrap_err().seq, 4);
    }
}
