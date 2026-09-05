//! Validation of the frozen seat-record contract — v1 (decision 0034)
//! and v2 (decision 0035), never one pretending to be the other.
//!
//! Both schemas are embedded beside this module so a packaged `brokkr`
//! remains an offline verifier. A test pins each embedded copy to the
//! published file in `contracts/`; they cannot drift inside this tree.
//!
//! **A record is validated against the version its run's engine wrote**
//! (decision 0035 ruling 7). There is no per-record version marker and
//! the journal is append-only, so the discriminator has to be a fact the
//! run already carries: the `engine` string in the `run/started`
//! manifest, which both manifest lineages keep. An engine older than the
//! one v2 landed in wrote v1 records and is read under v1 — which is
//! what keeps decision 0034 ruling 5's "old journals remain readable"
//! true, and what makes a v2-only field on such a record a refusal
//! rather than a quiet admission.

use std::sync::OnceLock;

use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;
use thiserror::Error;

const SCHEMA_V1: &str = include_str!("seat-record.v1.schema.json");
const SCHEMA_V2: &str = include_str!("seat-record.v2.schema.json");
const SCHEMA_V3: &str = include_str!("seat-record.v3.schema.json");

const CONTRACT_V1: &str = "contracts/seat-record.v1.schema.json";
const CONTRACT_V2: &str = "contracts/seat-record.v2.schema.json";
const CONTRACT_V3: &str = "contracts/seat-record.v3.schema.json";

/// The engine line in which seat-record v2 landed. A run whose
/// `run/started` manifest names an older engine is read under v1.
///
/// The manifest's `engine` is the crate version and carries no position
/// WITHIN a line, so the boundary is the line rather than a tag: v2
/// landed inside 0.8.0's development line, after the 0.8.0 tag, and a
/// constant naming a future release would misfile every record this
/// engine writes before that release ships. Reading a tagged-0.8.0
/// journal under v2 refuses nothing it wrote — v2 adds optional
/// properties and takes none away, so every valid v1 record is a valid
/// v2 record — while every engine before that line is dispatched to v1
/// exactly.
///
/// Seat-record v3 (decision 0034 ruling 7) landed in this same line, and
/// so shares this boundary. `engine` carries no position WITHIN a line,
/// so v2 and v3 cannot be told apart by it — both landed after the 0.8.0
/// tag. The newest contract in a line therefore wins, on exactly the
/// argument above: v3 adds an optional property and takes none away, so
/// every valid v2 record is a valid v3 record. The consequence is that
/// no engine string selects v2; it stays published, pinned and directly
/// nameable, but dispatch never chooses it.
const V2_ENGINE: (u64, u64, u64) = (0, 8, 0);

/// Which seat-record contract a record is judged against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatRecordVersion {
    V1,
    V2,
    V3,
}

impl SeatRecordVersion {
    /// The contract the named engine wrote. An engine string that does
    /// not parse as `major.minor.patch` is treated as older than the
    /// boundary: a version this binary cannot read is not a licence to
    /// admit fields its writer could not have produced.
    pub fn of_engine(engine: &str) -> SeatRecordVersion {
        // Two arms, not three, and the missing one is the point: v2 and
        // v3 share the 0.8.0 line (see `V2_ENGINE`), so no engine string
        // can select v2 — the newest contract in a line always wins. v2
        // stays a published, pinned contract and a version a caller can
        // name directly to judge a record against it; it is simply never
        // what dispatch chooses. `V2_ENGINE` still draws v1's boundary.
        match semver_triple(engine) {
            Some(version) if version >= V2_ENGINE => SeatRecordVersion::V3,
            _ => SeatRecordVersion::V1,
        }
    }

    /// The published file this version validates against, named in the
    /// refusal so a reader knows which contract was applied.
    pub fn contract(self) -> &'static str {
        match self {
            SeatRecordVersion::V1 => CONTRACT_V1,
            SeatRecordVersion::V2 => CONTRACT_V2,
            SeatRecordVersion::V3 => CONTRACT_V3,
        }
    }

    fn source(self) -> &'static str {
        match self {
            SeatRecordVersion::V1 => SCHEMA_V1,
            SeatRecordVersion::V2 => SCHEMA_V2,
            SeatRecordVersion::V3 => SCHEMA_V3,
        }
    }
}

/// `major.minor.patch`, ignoring any pre-release or build suffix.
fn semver_triple(version: &str) -> Option<(u64, u64, u64)> {
    let core = version
        .split(['-', '+'])
        .next()
        .filter(|core| !core.is_empty())?;
    let mut parts = core.split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let triple = (next()?, next()?, next()?);
    match parts.next() {
        Some(_) => None,
        None => Some(triple),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("seat record at journal seq {seq} violates {contract} at {path}")]
pub struct SeatRecordError {
    pub seq: u64,
    pub path: String,
    /// The published contract the record was judged against.
    pub contract: &'static str,
}

static VALIDATOR_V1: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V2: OnceLock<jsonschema::Validator> = OnceLock::new();
static VALIDATOR_V3: OnceLock<jsonschema::Validator> = OnceLock::new();

fn compile(version: SeatRecordVersion) -> jsonschema::Validator {
    let schema: Value =
        serde_json::from_str(version.source()).expect("the embedded seat-record schema is JSON");
    jsonschema::draft7::new(&schema).expect("the embedded seat-record schema is valid draft-07")
}

fn validator(version: SeatRecordVersion) -> &'static jsonschema::Validator {
    match version {
        SeatRecordVersion::V1 => VALIDATOR_V1.get_or_init(|| compile(SeatRecordVersion::V1)),
        SeatRecordVersion::V2 => VALIDATOR_V2.get_or_init(|| compile(SeatRecordVersion::V2)),
        SeatRecordVersion::V3 => VALIDATOR_V3.get_or_init(|| compile(SeatRecordVersion::V3)),
    }
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

/// The contract this run's engine wrote its records under, read from the
/// `run/started` manifest. A journal that names no engine at all is read
/// under v1: the older contract is the safe reading, because it admits
/// strictly less.
fn version_of(events: &[EventEnvelope]) -> SeatRecordVersion {
    events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted)
        .and_then(|event| event.payload.pointer("/manifest/engine"))
        .and_then(Value::as_str)
        .map(SeatRecordVersion::of_engine)
        .unwrap_or(SeatRecordVersion::V1)
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
    let version = version_of(events);
    for event in events {
        if let Some(record) = record_of(event.event_type, &event.payload) {
            validate_seat_record(record, event.seq, version)?;
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

    fn started(engine: &str) -> EventEnvelope {
        event(
            1,
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {"engine": engine}}),
        )
    }

    #[test]
    fn embedded_schemas_are_the_published_contracts() {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        for (relative, embedded) in [(CONTRACT_V1, SCHEMA_V1), (CONTRACT_V2, SCHEMA_V2)] {
            let published = std::fs::read(workspace.join(relative)).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&published).unwrap(),
                serde_json::from_str::<Value>(embedded).unwrap(),
                "{relative}"
            );
            assert_eq!(
                sha256_bytes(&published),
                sha256_bytes(embedded.as_bytes()),
                "{relative}"
            );
        }
    }

    #[test]
    fn fields_are_typed_positive_bounded_and_cache_reads_are_a_subset() {
        let valid = json!({
            "step":"seat-turn", "turn":1, "model":"claude-fable-5-1",
            "input_tokens":13, "output_tokens":2, "cache_read_tokens":3,
            "cache_write_tokens":4, "tool":"Read", "target":"src/lib.rs"
        });
        validate_seat_record(&valid, 7, SeatRecordVersion::V1).unwrap();

        for invalid in [
            json!({"step":"seat-turn", "turn":0}),
            json!({"step":"seat-turn", "turn":1, "input_tokens":0}),
            json!({"step":"seat-turn", "turn":1, "model":"configured guess"}),
            json!({"step":"seat-turn", "turn":1, "tool":"x".repeat(81)}),
            json!({"step":"seat-turn", "turn":1, "target":"src/lib.rs"}),
            json!({"step":"seat-turn", "turn":1, "content":"private prose"}),
        ] {
            assert!(
                validate_seat_record(&invalid, 8, SeatRecordVersion::V1).is_err(),
                "{invalid}"
            );
        }

        let subset = json!({
            "step":"turn-completed", "turn":1,
            "input_tokens":3, "cache_read_tokens":4
        });
        let error = validate_seat_record(&subset, 9, SeatRecordVersion::V1).unwrap_err();
        assert_eq!(error.seq, 9);
        assert_eq!(error.path, "/cache_read_tokens");
        assert_eq!(error.contract, CONTRACT_V1);
    }

    #[test]
    fn transcript_and_result_shapes_are_closed_without_rejecting_legacy_absence() {
        let transcript = json!({
            "step":"transcript", "model":"not reported",
            "transcript":{"kind":"codex-thread", "locator":"019c", "home":"/tmp/codex"}
        });
        validate_seat_record(&transcript, 1, SeatRecordVersion::V1).unwrap();
        validate_seat_record(
            &json!({"step":"seat-turn", "turn":3, "tool":"Bash"}),
            2,
            SeatRecordVersion::V1,
        )
        .unwrap();
        validate_seat_record(
            &json!({
                "result":"complete", "inputs":{"fixed":true}, "notes":"done",
                "model":"not applicable",
                "transcript":{"kind":"none", "locator":"", "home":""}
            }),
            3,
            SeatRecordVersion::V1,
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
            assert!(
                validate_seat_record(&invalid, 4, SeatRecordVersion::V1).is_err(),
                "{invalid}"
            );
        }
    }

    /// Decision 0035's two new facts live in v2 and ONLY in v2: v1 is a
    /// closed schema, so a record carrying either is refused under it.
    #[test]
    fn the_hires_effort_and_its_reasoning_belong_to_v2_alone() {
        let turn = json!({
            "step":"turn-completed", "turn":1, "harness":"codex",
            "model":"gpt-5.6-sol", "effort":"xhigh",
            "input_tokens":100, "output_tokens":40,
            "cache_read_tokens":60, "cache_write_tokens":7,
            "reasoning_output_tokens":31
        });
        validate_seat_record(&turn, 5, SeatRecordVersion::V2).unwrap();
        assert_eq!(
            validate_seat_record(&turn, 5, SeatRecordVersion::V1)
                .unwrap_err()
                .contract,
            CONTRACT_V1,
            "v1 is closed: a v2-only field is refused, never quietly admitted"
        );

        // Both sentinels are decision 0031's, reused rather than
        // reinvented, and both are legal efforts.
        for sentinel in ["not reported", "not applicable"] {
            validate_seat_record(
                &json!({"result":"complete", "effort":sentinel}),
                6,
                SeatRecordVersion::V2,
            )
            .unwrap();
        }
        for invalid in [
            // Configuration, not prose: an effort is one bounded word.
            json!({"step":"seat-turn", "turn":1, "effort":"thought about it hard"}),
            json!({"step":"seat-turn", "turn":1, "effort":""}),
            // Absent where unreported, NEVER zero.
            json!({"step":"seat-turn", "turn":1, "reasoning_output_tokens":0}),
        ] {
            assert!(
                validate_seat_record(&invalid, 7, SeatRecordVersion::V2).is_err(),
                "{invalid}"
            );
        }

        // A reported subset is never larger than the total it is drawn
        // from — the `cache_read_tokens` rule, one level down.
        let error = validate_seat_record(
            &json!({"step":"turn-completed", "turn":1,
                    "output_tokens":3, "reasoning_output_tokens":4}),
            8,
            SeatRecordVersion::V2,
        )
        .unwrap_err();
        assert_eq!(error.path, "/reasoning_output_tokens");
        assert_eq!(error.contract, CONTRACT_V2);

        // Every valid v1 record is a valid v2 record: v2 adds optional
        // properties and takes none away.
        validate_seat_record(
            &json!({"step":"seat-turn", "turn":1, "model":"not reported", "tool":"Read"}),
            9,
            SeatRecordVersion::V2,
        )
        .unwrap();
    }

    #[test]
    fn the_version_is_the_one_the_runs_engine_wrote() {
        // Ruling 7: v2 and v3 landed in the same 0.8.0 line and the
        // engine string cannot separate them, so within that line the
        // newest contract wins. This refuses nothing a v2 record could
        // have carried — v3 only adds an optional property — and it is
        // what lets a record THIS engine writes carry the `state` it is
        // already writing. `V2_ENGINE` still stands as the v1 boundary.
        assert_eq!(SeatRecordVersion::of_engine("0.8.0"), SeatRecordVersion::V3);
        assert_eq!(SeatRecordVersion::of_engine("0.9.1"), SeatRecordVersion::V3);
        assert_eq!(
            SeatRecordVersion::of_engine("1.0.0-rc.1"),
            SeatRecordVersion::V3
        );
        assert_eq!(SeatRecordVersion::of_engine("0.7.9"), SeatRecordVersion::V1);
        assert_eq!(SeatRecordVersion::of_engine("0.7"), SeatRecordVersion::V1);
        assert_eq!(
            SeatRecordVersion::of_engine("0.8.0.1"),
            SeatRecordVersion::V1
        );
        assert_eq!(SeatRecordVersion::of_engine(""), SeatRecordVersion::V1);
        assert_eq!(
            SeatRecordVersion::of_engine("not a version"),
            SeatRecordVersion::V1
        );
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

    /// Ruling 7: a dialect step's `state` rides the successful result
    /// under v3, and the same row is refused in a journal an engine
    /// before the v2/v3 line wrote — that engine had no dialect step to
    /// produce it. The field is admitted on the result alone; a
    /// checkpoint never carries one.
    #[test]
    fn a_dialect_steps_state_is_admitted_on_a_result_and_nowhere_else() {
        let result = event(
            2,
            EventType::EffectSucceeded,
            json!({"result":{
                "result":"pass", "notes":"validated", "state":"framework-state"
            }}),
        );
        validate_events(&[started("0.8.0"), result.clone()]).unwrap();
        assert_eq!(
            validate_events(&[started("0.7.9"), result])
                .unwrap_err()
                .seq,
            2
        );

        let checkpoint = event(
            2,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{"step":"seat-turn", "turn":1, "state":"framework-state"}}),
        );
        assert_eq!(
            validate_events(&[started("0.8.0"), checkpoint])
                .unwrap_err()
                .seq,
            2
        );
    }

    /// The dispatch, exercised both ways over the same record: a run
    /// this engine started carries its effort into the journal, and the
    /// identical row in a journal an older engine wrote is refused —
    /// that engine could not have written it.
    #[test]
    fn one_record_is_judged_by_the_engine_that_wrote_its_run() {
        let checkpoint = event(
            2,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{
                "step":"seat-turn", "turn":1, "model":"claude-opus-5", "effort":"high"
            }}),
        );
        validate_events(&[started("0.8.0"), checkpoint.clone()]).unwrap();
        let refused = validate_events(&[started("0.4.0"), checkpoint.clone()]).unwrap_err();
        assert_eq!(refused.seq, 2);
        assert_eq!(refused.contract, CONTRACT_V1);
        // A journal that names no engine reads under the older contract.
        assert_eq!(
            validate_events(&[
                event(1, EventType::RunStarted, json!({"feature": "f"})),
                checkpoint,
            ])
            .unwrap_err()
            .contract,
            CONTRACT_V1
        );
    }
}
