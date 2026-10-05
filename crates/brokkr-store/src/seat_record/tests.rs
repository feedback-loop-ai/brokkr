//! The seat-record contracts, judged: each version's embedded bytes, its
//! own fields and refusals, and the engine-line dispatch that picks it.

use super::*;
use brokkr_core::canonical::sha256_bytes;
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::EventType;
use serde_json::{json, Value};

fn event(seq: u64, event_type: EventType, payload: Value) -> EventEnvelope {
    crate::tests::envelope_builder::EnvelopeBuilder::new(event_type, payload)
        .seq(seq)
        .event_id(format!("event-{seq}"))
        .at("2026-09-03T00:00:00Z")
        .hash("1".repeat(64))
        .build()
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
    for (relative, embedded) in [
        (CONTRACT_V1, SCHEMA_V1),
        (CONTRACT_V2, SCHEMA_V2),
        (CONTRACT_V3, SCHEMA_V3),
        (CONTRACT_V4, SCHEMA_V4),
        (CONTRACT_V5, SCHEMA_V5),
        (CONTRACT_V6, SCHEMA_V6),
    ] {
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
    assert_eq!(SeatRecordVersion::of_engine("0.8.9"), SeatRecordVersion::V3);
    // Decision 0046 ruling 3 (v4, per the commission's erratum): the
    // 0.9 line and everything after it reads v4.
    assert_eq!(SeatRecordVersion::of_engine("0.9.0"), SeatRecordVersion::V4);
    assert_eq!(SeatRecordVersion::of_engine("0.9.1"), SeatRecordVersion::V4);
    // Proposed decision 0056 ruling 7 moved the top of this ladder to
    // v5 at the 0.10 line, and decision 0065 ruling 8 to v6 at the 0.12
    // line, on the same superset argument, so a release-candidate of
    // 1.0.0 reads v6.
    assert_eq!(
        SeatRecordVersion::of_engine("1.0.0-rc.1"),
        SeatRecordVersion::V6
    );
    assert_eq!(
        SeatRecordVersion::of_engine("0.10.0"),
        SeatRecordVersion::V5
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

/// Decision 0046 ruling 3: the boundary rides beside the model on a
/// checkpoint and on a result under v4, the five words and the
/// sentinel and nothing else; the same row is refused in a journal
/// the 0.8 line wrote, whose engine had no boundary to stamp; and a
/// tagged-0.9 journal that carries no `boundary` still validates,
/// because v4 adds an optional property and takes none away.
#[test]
fn the_boundary_is_admitted_under_v4_and_nowhere_before_it() {
    for word in [
        "namespace",
        "seatbelt",
        "container",
        "harness",
        "open",
        "not applicable",
    ] {
        let checkpoint = event(
            2,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{
                "step":"exec-session-finished", "model":"not applicable", "boundary": word
            }}),
        );
        let result = event(
            3,
            EventType::EffectSucceeded,
            json!({"result":{"result":"pass", "model":"claude-opus-5", "boundary": word}}),
        );
        validate_events(&[started("0.9.0"), checkpoint.clone(), result.clone()]).unwrap();
        let refused = validate_events(&[started("0.8.0"), checkpoint]).unwrap_err();
        assert_eq!((refused.seq, refused.contract), (2, CONTRACT_V3));
        let refused = validate_events(&[started("0.8.0"), result]).unwrap_err();
        assert_eq!((refused.seq, refused.contract), (3, CONTRACT_V3));
    }
    let wrong = event(
        2,
        EventType::EffectSucceeded,
        json!({"result":{"result":"pass", "model":"claude-opus-5", "boundary":"chroot"}}),
    );
    let refused = validate_events(&[started("0.9.1"), wrong]).unwrap_err();
    assert_eq!((refused.seq, refused.path.as_str()), (2, "/"));
    assert_eq!(refused.contract, CONTRACT_V4);
    // A record without the word — every record the tagged 0.9.0 and
    // 0.9.1 engines wrote — is what it always was.
    validate_events(&[
        started("0.9.1"),
        event(
            2,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{"step":"seat-turn", "turn":1, "model":"claude-opus-5"}}),
        ),
    ])
    .unwrap();
}

/// A confirmed root, as proposed decision 0056 ruling 3 records it:
/// the complete provider id, the version OBSERVED when it opened,
/// and whether that root persists.
fn root(id: &str) -> Value {
    json!({
        "kind":"claude-session", "id": id,
        "harness_version":"2.1.266", "persistent": true
    })
}

const SITE: &str = "5c1e0000000000000000000000000000000000000000000000000000000051fe";
const INSTANCE: &str = "1a2b000000000000000000000000000000000000000000000000000000003c4d";

/// v5's three added fields and five added refusal tokens are v5's
/// and only v5's: v4 is a closed schema, so each is refused under it
/// rather than quietly admitted. This is the same test the effort
/// and boundary fields each got when they landed.
#[test]
fn the_root_the_stamps_and_the_new_refusals_belong_to_v5_alone() {
    let launched = json!({
        "step":"harness-started", "harness":"claude", "launch":"resumed",
        "site_ref": SITE, "instance_ref": INSTANCE,
        "root_session": root("019c4b7e-0000-7000-8000-000000000001")
    });
    validate_seat_record(&launched, 5, SeatRecordVersion::V5).unwrap();
    assert_eq!(
        validate_seat_record(&launched, 5, SeatRecordVersion::V4)
            .unwrap_err()
            .contract,
        CONTRACT_V4,
        "v4 is closed: a v5-only field is refused, never quietly admitted"
    );

    for token in [
        "unsupported-resume",
        "unverified-harness",
        "restrictions-unavailable",
        "instance-changed",
        "nonpersistent-session",
    ] {
        let row = json!({
            "step":"harness-started", "launch":"cold", "resume_refusal": token
        });
        validate_seat_record(&row, 6, SeatRecordVersion::V5).unwrap();
        assert!(
            validate_seat_record(&row, 6, SeatRecordVersion::V4).is_err(),
            "{token} is v5's own token"
        );
    }
    // The five v4 tokens keep their meaning under v5.
    for token in [
        "invalid-session-id",
        "sandbox-unavailable",
        "unsupported-sandbox",
        "incompatible-argv",
        "harness-refused",
    ] {
        validate_seat_record(
            &json!({"step":"harness-started", "launch":"cold", "resume_refusal": token}),
            6,
            SeatRecordVersion::V5,
        )
        .unwrap();
    }
}

/// The bounds on a root, each one a hazard rather than a formality:
/// an over-long or flag-shaped id would be truncated or read as a
/// selector, and a permission mode offered as a `sandbox` is a
/// vocabulary from another provider (design D4 declines it).
#[test]
fn a_root_is_bounded_closed_and_never_another_providers_vocabulary() {
    for invalid in [
        json!({"step":"s", "root_session":{
            "kind":"claude-session", "id":"a".repeat(81),
            "harness_version":"2.1.266", "persistent": true}}),
        json!({"step":"s", "root_session":{
            "kind":"claude-session", "id":"--resume",
            "harness_version":"2.1.266", "persistent": true}}),
        json!({"step":"s", "root_session":{
            "kind":"claude-session", "id":"",
            "harness_version":"2.1.266", "persistent": true}}),
        // A kind the mapping does not name, and the transcript's
        // `none` is not a root kind.
        json!({"step":"s", "root_session":{
            "kind":"none", "id":"abc",
            "harness_version":"2.1.266", "persistent": true}}),
        // Every required part is required: absence is not "unknown".
        json!({"step":"s", "root_session":{"kind":"claude-session", "id":"abc"}}),
        json!({"step":"s", "root_session":{
            "kind":"claude-session", "id":"abc",
            "harness_version":"2.1.266", "persistent":"yes"}}),
        // Closed: no field behind the four this contract admits.
        json!({"step":"s", "root_session":{
            "kind":"claude-session", "id":"abc", "harness_version":"2.1.266",
            "persistent": true, "home":"/home/someone/.claude"}}),
        // The stamps are digests, not display tags.
        json!({"step":"s", "site_ref":"implement:alpha"}),
        json!({"step":"s", "instance_ref":"ABCD"}),
        // Codex's three classes, and nothing from another provider.
        json!({"step":"s", "sandbox":"acceptEdits"}),
    ] {
        assert!(
            validate_seat_record(&invalid, 7, SeatRecordVersion::V5).is_err(),
            "{invalid}"
        );
    }
}

/// The compatibility rule v5 exists to keep (design D4's superset
/// rule, task repairs F1 and F5): the 0.10 line dispatches to v5, so
/// v5 also judges rows the shipped 0.10.0 engine ALREADY wrote. An
/// unconditional resumed-requires-root rule would have refused them
/// at append, export, import verification and offline verification.
/// Both new conditions are therefore scoped on `site_ref`, the one
/// within-row fact only an engine enacting 0056 writes.
///
/// The refusal-bearing rows are synthetic contract counterexamples,
/// not observed provider telemetry: one validator behind
/// `lib.rs`'s fence judges third-party driver checkpoints too, and
/// v4 admits `launch` and `resume_refusal` independently over a free
/// `step` string, so both shapes are valid v4 rows a driver outside
/// this tree could have written.
#[test]
fn valid_unstamped_history_survives_v5_and_a_stamped_row_does_not() {
    // Exactly what shipped `codex_started` writes on a rejoin.
    let historical = json!({
        "step":"harness-started", "harness":"codex",
        "launch":"resumed", "sandbox":"workspace-write"
    });
    // The two shapes a third-party driver could have written under
    // v4, which an unconditional rule would newly refuse.
    let bare_refusal = json!({"step":"resume-declined", "resume_refusal":"incompatible-argv"});
    let resumed_with_refusal = json!({
        "step":"harness-started", "launch":"resumed",
        "resume_refusal":"incompatible-argv"
    });

    for unstamped in [&historical, &bare_refusal, &resumed_with_refusal] {
        validate_seat_record(unstamped, 2, SeatRecordVersion::V4).unwrap();
        let valid = validate_seat_record(unstamped, 2, SeatRecordVersion::V5);
        assert!(
            valid.is_ok(),
            "{unstamped} must stay valid under v5: {valid:?}"
        );

        // The same row, once this engine stamps it, IS refused.
        let mut stamped = unstamped.clone();
        stamped["site_ref"] = json!(SITE);
        let refused = validate_seat_record(&stamped, 3, SeatRecordVersion::V5).unwrap_err();
        assert_eq!((refused.seq, refused.path.as_str()), (3, "/"));
        assert_eq!(refused.contract, CONTRACT_V5);
    }

    // A stamped row that obeys both conditions is admitted.
    validate_seat_record(
        &json!({
            "step":"harness-started", "harness":"codex", "launch":"resumed",
            "site_ref": SITE, "instance_ref": INSTANCE,
            "root_session":{"kind":"codex-thread", "id":"019c4b7e",
                            "harness_version":"0.153.4", "persistent": true}
        }),
        4,
        SeatRecordVersion::V5,
    )
    .unwrap();
    validate_seat_record(
        &json!({
            "step":"harness-started", "launch":"cold",
            "resume_refusal":"unverified-harness", "site_ref": SITE
        }),
        5,
        SeatRecordVersion::V5,
    )
    .unwrap();
}

/// The whole dispatch matrix at all four version boundaries, and the
/// one direction that matters for the amended `boundary-record`
/// requirement: a v5-only field under a 0.9-line manifest is refused
/// under v4 rather than selecting v5 from its presence. There is no
/// per-record version marker; the run's engine decides.
#[test]
fn the_zero_ten_line_reads_v5_and_the_nine_line_still_reads_v4() {
    for (engine, want) in [
        ("0.7.9", SeatRecordVersion::V1),
        ("0.8.0", SeatRecordVersion::V3),
        ("0.8.99", SeatRecordVersion::V3),
        ("0.9.0", SeatRecordVersion::V4),
        ("0.9.1", SeatRecordVersion::V4),
        ("0.9.99", SeatRecordVersion::V4),
        ("0.10.0", SeatRecordVersion::V5),
        ("0.10.1", SeatRecordVersion::V5),
        ("0.11.99", SeatRecordVersion::V5),
        ("not a version", SeatRecordVersion::V1),
    ] {
        assert_eq!(SeatRecordVersion::of_engine(engine), want, "{engine}");
    }

    let stamped = event(
        2,
        EventType::EffectCheckpointed,
        json!({"checkpoint":{"step":"harness-started", "site_ref": SITE}}),
    );
    validate_events(&[started("0.10.0"), stamped.clone()]).unwrap();
    let refused = validate_events(&[started("0.9.1"), stamped]).unwrap_err();
    assert_eq!((refused.seq, refused.contract), (2, CONTRACT_V4));

    // The tagged 0.9.0/0.9.1 no-boundary example stays exactly what
    // it was: v5's arrival moves nothing in the 0.9 line.
    validate_events(&[
        started("0.9.1"),
        event(
            2,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{"step":"seat-turn", "turn":1, "model":"claude-opus-5"}}),
        ),
    ])
    .unwrap();

    // v5 keeps the boundary's authority and its refusal behavior.
    validate_events(&[
        started("0.10.0"),
        event(
            2,
            EventType::EffectCheckpointed,
            json!({"checkpoint":{
                "step":"harness-started", "model":"claude-opus-5",
                "boundary":"namespace", "site_ref": SITE
            }}),
        ),
    ])
    .unwrap();
    let refused = validate_events(&[
        started("0.10.0"),
        event(
            2,
            EventType::EffectSucceeded,
            json!({"result":{"result":"pass", "model":"claude-opus-5", "boundary":"chroot"}}),
        ),
    ])
    .unwrap_err();
    assert_eq!((refused.seq, refused.contract), (2, CONTRACT_V5));
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

/// A complete SC4 attribution group, as the engine stamps one: the
/// selected capability and dialect, the concrete tool and the call. No
/// turn: only a settled broker state may stand without one.
fn attributed(state: &str) -> Value {
    json!({
        "step":"capability-call", "tool":"WebSearch",
        "capability":"web-search", "dialect":"claude-native-search",
        "call_id":"attempt-1:toolu_01", "call_state": state
    })
}

fn with(mut record: Value, field: &str, value: Value) -> Value {
    record[field] = value;
    record
}

fn without(mut record: Value, field: &str) -> Value {
    record.as_object_mut().unwrap().remove(field);
    record
}

/// A native call the harness reported, on the turn that reported it.
fn observed() -> Value {
    with(attributed("observed"), "turn", json!(3))
}

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SETTLED: [&str; 4] = ["succeeded", "failed", "refused", "interrupted"];

fn refused_by(seq: u64, contract: &'static str) -> Result<(), SeatRecordError> {
    let path = "/".to_string();
    Err(SeatRecordError {
        seq,
        path,
        contract,
    })
}

/// The v6 boundary is the 0.12 line, the development line on main when
/// v6 landed, read through the same `major.minor.patch` convention as
/// every earlier boundary: a pre-release or build of the line is the
/// line, and a malformed or missing engine falls back to v1. The 0.11
/// line keeps v5, so the identical complete group is admitted in a 0.12
/// run and refused, as v5's violation, in a 0.11 one.
#[test]
fn the_zero_twelve_line_reads_v6_and_the_eleven_line_still_reads_v5() {
    for (engine, want) in [
        ("0.11.0", SeatRecordVersion::V5),
        ("0.11.99", SeatRecordVersion::V5),
        ("0.11.99+build.7", SeatRecordVersion::V5),
        ("0.12.0-rc.1", SeatRecordVersion::V6),
        ("0.12.0+build.7", SeatRecordVersion::V6),
        ("0.12.0", SeatRecordVersion::V6),
        ("0.12.3", SeatRecordVersion::V6),
        ("1.0.0", SeatRecordVersion::V6),
        ("0.12", SeatRecordVersion::V1),
        ("0.12.0.1", SeatRecordVersion::V1),
        ("", SeatRecordVersion::V1),
    ] {
        assert_eq!(SeatRecordVersion::of_engine(engine), want, "{engine}");
    }
    let group = event(
        2,
        EventType::EffectCheckpointed,
        json!({"checkpoint": observed()}),
    );
    validate_events(&[started("0.12.0"), group.clone()]).unwrap();
    assert_eq!(
        validate_events(&[started("0.11.99"), group.clone()]),
        refused_by(2, CONTRACT_V5)
    );
    let unnamed = event(1, EventType::RunStarted, json!({"feature": "f"}));
    assert_eq!(
        validate_events(&[unnamed, group]),
        refused_by(2, CONTRACT_V1)
    );
}

/// SC4's admitted shapes: one native observed group on its real turn,
/// and each of the four settled broker states with or without a turn —
/// the broker's call identity owns it, and a correlated turn when one was
/// measured stays the real one. A digest rides succeeded and failed. The
/// bounds are inclusive and the tool is never truncated to fit.
#[test]
fn v6_admits_one_native_observed_or_broker_settled_group() {
    let admitted = |record: &Value| validate_seat_record(record, 3, SeatRecordVersion::V6);
    assert_eq!(admitted(&observed()), Ok(()));
    for state in SETTLED {
        assert_eq!(admitted(&attributed(state)), Ok(()), "{state}");
        let turned = with(attributed(state), "turn", json!(2));
        assert_eq!(admitted(&turned), Ok(()), "{state} on its turn");
    }
    for state in ["succeeded", "failed"] {
        let retained = with(attributed(state), "response_sha256", json!(DIGEST));
        assert_eq!(admitted(&retained), Ok(()), "{state} with its digest");
    }
    let mut widest = observed();
    for (field, bytes) in [("capability", 128), ("dialect", 128), ("call_id", 128)] {
        widest[field] = json!("a".repeat(bytes));
    }
    widest["tool"] = json!(format!("mcp:{}", "t".repeat(252)));
    assert_eq!(admitted(&widest), Ok(()));
}

/// SC4's refusals, each the exact v6 violation at the record's seq: a
/// partial group, a public `started` or unknown state, a digest beside a
/// state with no retained response or in the wrong form, every identity
/// one byte over or outside its vocabulary, a dependency met by no turn,
/// and a private observation or an unknown key.
#[test]
fn v6_refuses_partial_groups_bad_states_digests_and_identities() {
    let broker = attributed("succeeded");
    let legacy = json!({"step":"seat-turn", "turn":1, "tool":"WebSearch"});
    let mut refused: Vec<Value> = ["capability", "dialect", "tool", "call_id", "call_state"]
        .iter()
        .map(|field| without(observed(), field))
        .collect();
    refused.extend([
        // Each group field alone on an old-shaped row, so only its own
        // dependency entry can refuse it.
        with(legacy.clone(), "capability", json!("web-search")),
        with(legacy.clone(), "dialect", json!("claude-native-search")),
        with(legacy.clone(), "call_id", json!("attempt-1:toolu_01")),
        with(legacy.clone(), "call_state", json!("observed")),
        with(legacy.clone(), "response_sha256", json!(DIGEST)),
        // On a real turn, so the state alone decides.
        with(observed(), "call_state", json!("started")),
        with(observed(), "call_state", json!("pending")),
        with(observed(), "response_sha256", json!(DIGEST)),
        with(attributed("refused"), "response_sha256", json!(DIGEST)),
        with(attributed("interrupted"), "response_sha256", json!(DIGEST)),
        with(
            broker.clone(),
            "response_sha256",
            json!(DIGEST.to_uppercase()),
        ),
        with(broker.clone(), "response_sha256", json!(&DIGEST[1..])),
        with(broker.clone(), "capability", json!("a".repeat(129))),
        with(broker.clone(), "capability", json!("Web-Search")),
        with(broker.clone(), "dialect", json!("d".repeat(129))),
        with(broker.clone(), "dialect", json!("Claude-Native-Search")),
        with(broker.clone(), "call_id", json!("c".repeat(129))),
        with(broker.clone(), "call_id", json!("attempt 1")),
        with(broker.clone(), "call_id", json!("attempt-é")),
        with(broker.clone(), "tool", json!("t".repeat(257))),
        with(broker.clone(), "tool", json!("Web Search")),
        with(legacy.clone(), "tool", json!("t".repeat(81))),
        without(observed(), "turn"),
        without(legacy.clone(), "turn"),
        json!({"step":"seat-turn", "tool":"Read", "target":"src/lib.rs"}),
        with(
            observed(),
            "observation",
            json!({"call":"toolu_01", "name":"WebSearch"}),
        ),
        with(broker, "server", json!("cap-web-search")),
    ]);
    for record in refused {
        assert_eq!(
            validate_seat_record(&record, 4, SeatRecordVersion::V6),
            refused_by(4, CONTRACT_V6),
            "{record}"
        );
    }
}

/// CC3's old absence is honest: every valid v5 shape — a legacy tool
/// row on its turn, an 80-byte tool, an unstamped historical resume, a
/// stamped root launch, a refusal token, a dialect step's result — is a
/// valid v6 record with no group invented, and v5's two stamped-row
/// conditions still refuse under v6.
#[test]
fn every_valid_v5_shape_is_a_valid_v6_record_and_v5s_conditions_stand() {
    for record in [
        json!({"step":"seat-turn", "turn":1, "tool":"Read", "target":"src/lib.rs"}),
        json!({"step":"seat-turn", "turn":1, "tool":"t".repeat(80)}),
        json!({"step":"harness-started", "harness":"codex", "launch":"resumed"}),
        json!({"step":"harness-started", "launch":"resumed", "site_ref": SITE,
               "root_session": root("019c4b7e")}),
        json!({"step":"harness-started", "launch":"cold", "resume_refusal":"instance-changed"}),
        json!({"result":"pass", "state":"framework-state", "boundary":"namespace"}),
    ] {
        validate_seat_record(&record, 5, SeatRecordVersion::V5).unwrap();
        assert_eq!(
            validate_seat_record(&record, 5, SeatRecordVersion::V6),
            Ok(()),
            "{record}"
        );
    }
    let stamped = json!({"step":"harness-started", "launch":"resumed", "site_ref": SITE});
    assert_eq!(
        validate_seat_record(&stamped, 6, SeatRecordVersion::V6),
        refused_by(6, CONTRACT_V6)
    );
}

/// Append, export and offline verification read one contract for a
/// 0.12 run: a partial group is refused at the seq it would take with
/// the journal standing still; complete native and broker groups land
/// and export and verify; a malformed group planted past the fence is
/// refused by both read sweeps with the same violation. A 0.11 run's
/// fence refuses the complete group under v5.
#[test]
fn append_export_and_verify_judge_one_v6_contract() {
    let (_dir, mut store) = crate::tests::store();
    for (run, engine) in [("twelve", "0.12.0"), ("eleven", "0.11.0")] {
        let manifest = json!({"engine": engine});
        store.create_run(run, "feat", "self", &manifest).unwrap();
        for (event_type, payload) in [
            (
                EventType::RunStarted,
                json!({"feature":"feat", "manifest": manifest}),
            ),
            (EventType::PhaseEntered, json!({"phase":"implement"})),
            (
                EventType::EffectRequested,
                json!({"effect_id":"fx", "seat":"implement"}),
            ),
            (
                EventType::EffectStarted,
                json!({"effect_id":"fx", "attempt_id":"attempt-1"}),
            ),
        ] {
            store
                .append_next(run, event_type, payload, None, None)
                .unwrap();
        }
    }
    let append = |store: &mut crate::Store, run: &str, record: Value| {
        let payload = json!({"effect_id":"fx", "checkpoint": record});
        store.append_next(run, EventType::EffectCheckpointed, payload, None, None)
    };
    let head = store.head_hash("twelve").unwrap();
    let crate::StoreError::SeatRecord(partial) =
        append(&mut store, "twelve", without(observed(), "call_id")).unwrap_err()
    else {
        panic!("a partial group is a seat-record refusal");
    };
    assert_eq!(
        partial.to_string(),
        "seat record at journal seq 5 violates contracts/seat-record.v6.schema.json at /"
    );
    assert_eq!(Err(partial), refused_by(5, CONTRACT_V6));
    assert_eq!(store.head_hash("twelve").unwrap(), head);
    for record in [observed(), attributed("refused")] {
        append(&mut store, "twelve", record).unwrap();
    }
    let exported = store.export_ndjson("twelve").unwrap();
    assert_eq!(crate::verify_export(&exported).unwrap().seq, 6);
    let crate::StoreError::SeatRecord(old) = append(&mut store, "eleven", observed()).unwrap_err()
    else {
        panic!("v5 has no attribution group");
    };
    assert_eq!(Err(old), refused_by(5, CONTRACT_V5));

    let malformed = with(attributed("refused"), "response_sha256", json!(DIGEST));
    let payload = json!({"effect_id":"fx", "checkpoint": malformed});
    crate::tests::plant_unfenced(&mut store, "twelve", EventType::EffectCheckpointed, payload);
    let crate::StoreError::SeatRecord(exported) = store.export_ndjson("twelve").unwrap_err() else {
        panic!("export refuses a malformed group as a seat record");
    };
    assert_eq!(Err(exported), refused_by(7, CONTRACT_V6));
    // The sealed rows themselves, as another writer could hand them over.
    let ndjson: String = store
        .load("twelve")
        .unwrap()
        .iter()
        .map(|event| serde_json::to_string(event).unwrap() + "\n")
        .collect();
    let Err(crate::VerifyError::SeatRecord(verified)) = crate::verify_export(&ndjson) else {
        panic!("offline verify refuses a malformed group as a seat record");
    };
    assert_eq!(Err(verified), refused_by(7, CONTRACT_V6));
}
