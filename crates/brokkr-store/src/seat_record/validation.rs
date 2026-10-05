//! Judging a seat record, and a run's journal of them, against the
//! contract its version names. Which version applies is the parent's
//! table and dispatch; this module only applies it — at the append
//! fence, and in the export and offline-verify sweeps.

use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use super::{SeatRecordError, SeatRecordVersion};

fn compile(version: SeatRecordVersion) -> jsonschema::Validator {
    let schema: Value =
        serde_json::from_str(version.source()).expect("the embedded seat-record schema is JSON");
    jsonschema::draft7::new(&schema).expect("the embedded seat-record schema is valid draft-07")
}

fn validator(version: SeatRecordVersion) -> &'static jsonschema::Validator {
    version.compiled().get_or_init(|| compile(version))
}

/// The subset relationships the schema itself cannot state: a reported
/// subset is never larger than the total it is drawn from. Both are the
/// same rule one level apart — a cache read IS an input token and a
/// reasoning token IS an output token — and both exist so a view can
/// show the subset without ever adding it to a total a second time.
const SUBSETS: [(&str, &str); 2] = [
    ("cache_read_tokens", "input_tokens"),
    ("reasoning_output_tokens", "output_tokens"),
];

/// Validate one checkpoint or successful result against the named
/// seat-record contract. Error text reports only the JSON pointer: an
/// invalid value is not echoed into diagnostics where prose could leak.
pub fn validate_seat_record(
    record: &Value,
    seq: u64,
    version: SeatRecordVersion,
) -> Result<(), SeatRecordError> {
    let refuse = |path: &str| SeatRecordError {
        seq,
        path: path.to_string(),
        contract: version.contract(),
    };
    if validator(version).validate(record).is_err() {
        // The contract's top level is a `oneOf`, so a violation is always
        // reported against the record as a whole: the pointer is the root
        // and the offending value is never echoed.
        return Err(refuse("/"));
    }
    for (subset, total) in SUBSETS {
        if let (Some(part), Some(whole)) = (
            record.get(subset).and_then(Value::as_u64),
            record.get(total).and_then(Value::as_u64),
        ) {
            if part > whole {
                return Err(refuse(&format!("/{subset}")));
            }
        }
    }
    Ok(())
}

/// The seat record an event carries, if its type carries one: a
/// checkpoint's `checkpoint`, a successful result's `result`. This is
/// the one place that knows which events are seat records. The append
/// fence and the export and verify sweeps all ask it, so they cannot
/// disagree about what is checked.
pub(crate) fn record_of(event_type: EventType, payload: &Value) -> Option<&Value> {
    match event_type {
        EventType::EffectCheckpointed => payload.get("checkpoint"),
        EventType::EffectSucceeded => payload.get("result"),
        _ => None,
    }
}

pub(crate) fn validate_events(events: &[EventEnvelope]) -> Result<(), SeatRecordError> {
    // The contract this run's engine wrote its records under.
    let engine = events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted)
        .and_then(|event| event.payload.pointer("/manifest/engine"));
    let version = SeatRecordVersion::of_manifest(engine.and_then(Value::as_str));
    for event in events {
        if let Some(record) = record_of(event.event_type, &event.payload) {
            validate_seat_record(record, event.seq, version)?;
        }
    }
    Ok(())
}
