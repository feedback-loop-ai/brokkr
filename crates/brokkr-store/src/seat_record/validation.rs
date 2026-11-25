//! Judging a seat record, and a run's journal of them, against the
//! contract its version names. Which version applies is the parent's
//! table and dispatch; this module only applies it — at the append
//! fence, and in the export and offline-verify sweeps.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use brokkr_core::findings::{EngineMarker, Finding, AUTHORED};
use brokkr_core::{EventEnvelope, EventType};
use serde::Deserialize;
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

/// Each contract's result branch alone, compiled once, by version.
static RESULT_BRANCH: [OnceLock<jsonschema::Validator>; 8] = [const { OnceLock::new() }; 8];

/// `version`'s contract with its top-level `oneOf` narrowed to the result
/// branch (fix round 0109a1r7, X3): every published seat-record contract
/// is a `oneOf` of a checkpoint and a result, so a record that must be a
/// result is judged by the result's own closed definition, and a
/// checkpoint-shaped record is refused.
fn result_branch(version: SeatRecordVersion) -> &'static jsonschema::Validator {
    RESULT_BRANCH[version as usize].get_or_init(|| {
        let mut schema: Value = serde_json::from_str(version.source())
            .expect("the embedded seat-record schema is JSON");
        let result = (schema["oneOf"].as_array().into_iter().flatten())
            .find(|arm| {
                arm["$ref"]
                    .as_str()
                    .is_some_and(|at| at.ends_with("/result"))
            })
            .cloned()
            .expect("every seat-record contract has a result branch");
        schema["oneOf"] = Value::Array(vec![result]);
        jsonschema::draft7::new(&schema).expect("a seat-record result branch is valid draft-07")
    })
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
    validated_by(validator(version), record, seq, version)
}

/// [`validate_seat_record`] with `schema`, the whole contract or one of
/// its branches, in place of the contract's own validator.
fn validated_by(
    schema: &jsonschema::Validator,
    record: &Value,
    seq: u64,
    version: SeatRecordVersion,
) -> Result<(), SeatRecordError> {
    let refuse = |path: &str| SeatRecordError {
        seq,
        path: path.to_string(),
        contract: version.contract(),
        settled_member: None,
    };
    if schema.validate(record).is_err() || !gate_report_reads(record, version) {
        // The contract's top level is a `oneOf`, so a violation is always
        // reported against the record as a whole: the pointer is the root
        // and the offending value is never echoed.
        return Err(refuse("/"));
    }
    for (at, result) in results_of(record, version) {
        for (subset, total) in SUBSETS {
            if let (Some(part), Some(whole)) = (
                result.get(subset).and_then(Value::as_u64),
                result.get(total).and_then(Value::as_u64),
            ) {
                if part > whole {
                    return Err(refuse(&format!("{at}/{subset}")));
                }
            }
        }
    }
    Ok(())
}

/// Every result `record` holds, with its JSON pointer: the record itself,
/// and under v7 the nested `result` of a checkpoint the engine authored,
/// so one validation applies to a result wherever it sits (fix round
/// 0109a1r5, V3). The nested read is scoped on `authored`, as v7's closed
/// nested result is (fix round 0109a1r4, K5), so every row a released
/// engine wrote is judged exactly as before.
fn results_of(record: &Value, version: SeatRecordVersion) -> Vec<(&'static str, &Value)> {
    let nested = (version.carries_gate_report() && record.get(AUTHORED).is_some())
        .then(|| record.get("result"))
        .flatten();
    let nested = nested.map(|result| ("/result", result));
    [("", record)].into_iter().chain(nested).collect()
}

/// Whether a v7 record's gate report reads through the engine's own typed
/// reader, `brokkr_core::findings::Finding`, so the fence refuses exactly
/// what the reader refuses, by construction (fix round 0109a1r3, M1): a
/// number the schema's `integer` admits that is not a `u64`, such as
/// `1.0` or `2^64`, is refused here as the reader refuses it. The report
/// is the result's own, or the nested `result` of a checkpoint the engine
/// authored, scoped on `authored` as v7's closed nested result is (fix
/// round 0109a1r4, K5), so no v6-era row is read here. Earlier versions
/// carry no gate report, so there is nothing to read.
fn gate_report_reads(record: &Value, version: SeatRecordVersion) -> bool {
    if !version.carries_gate_report() {
        return true;
    }
    let report = match record.get(AUTHORED) {
        Some(_) => record.get("result").unwrap_or(record),
        None if record.get("result").is_some_and(Value::is_object) => return true,
        None => record,
    };
    ["findings", "follow_ups"].iter().all(|field| {
        report
            .get(field)
            .is_none_or(|findings| Vec::<Finding>::deserialize(findings).is_ok())
    })
}

/// Each settled member of a panel's failed or indeterminate terminal
/// `payload`, by its tag, with its journaled result: the payloads a
/// failure's evidence carries beside the members that succeeded.
pub(crate) fn settled_of(
    event_type: EventType,
    payload: &Value,
) -> impl Iterator<Item = (&String, &Value)> {
    let terminal = matches!(
        event_type,
        EventType::EffectFailed | EventType::EffectIndeterminate
    );
    (terminal.then(|| payload.get("settled_members").and_then(Value::as_object)))
        .flatten()
        .into_iter()
        .flatten()
}

/// Validate each settled member's payload in `payload` that the engine
/// marked, `marked` naming the members its own markers settled in the
/// attempt, as a standalone result is validated (fix round 0109a1r6, W3):
/// the marked payload reads as a verdict, so it gets every fence one does,
/// and a refusal names `/settled_members/<tag>` before the result's own
/// pointer. The marked payload is a result, so it is judged by the
/// contract's closed result branch alone, never the checkpoint's (fix
/// round 0109a1r7, X3). An unmarked payload, a released engine's raw copy,
/// keeps its admission.
///
/// The refusal names its member as data, the exact tag, in
/// [`SeatRecordError::settled_member`] (fix round 0109a1r8, M4). The
/// pointer escapes the tag (RFC 6901), so a tag holding `/` points at
/// itself, but no reader parses it back.
pub(crate) fn validate_settled(
    (event_type, payload): (EventType, &Value),
    marked: &BTreeSet<String>,
    seq: u64,
    version: SeatRecordVersion,
) -> Result<(), SeatRecordError> {
    let marked = settled_of(event_type, payload).filter(|(tag, _)| marked.contains(*tag));
    for (tag, result) in marked {
        validated_by(result_branch(version), result, seq, version).map_err(|mut refusal| {
            let inner = refusal.path.strip_prefix('/').unwrap_or_default();
            let escaped = tag.replace('~', "~0").replace('/', "~1");
            refusal.path = match inner {
                "" => format!("/settled_members/{escaped}"),
                inner => format!("/settled_members/{escaped}/{inner}"),
            };
            refusal.settled_member = Some(Box::new(tag.clone()));
            refusal
        })?;
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
    let mut marked: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let none = BTreeSet::new();
    for event in events {
        if let Some(record) = record_of(event.event_type, &event.payload) {
            validate_seat_record(record, event.seq, version)?;
        }
        let attempt = event.payload["attempt_id"].as_str().unwrap_or_default();
        if let Some(member) = marked_member(event.event_type, &event.payload) {
            marked.entry(attempt).or_default().insert(member.to_owned());
        }
        let settled = marked.get(attempt).unwrap_or(&none);
        validate_settled(
            (event.event_type, &event.payload),
            settled,
            event.seq,
            version,
        )?;
    }
    Ok(())
}

/// The member a checkpoint event's engine marker settles, by the one
/// check inspect and the carry read the mark by.
pub(crate) fn marked_member(event_type: EventType, payload: &Value) -> Option<&str> {
    (event_type == EventType::EffectCheckpointed).then_some(())?;
    EngineMarker::settled_member(payload.get("checkpoint")?)
}
