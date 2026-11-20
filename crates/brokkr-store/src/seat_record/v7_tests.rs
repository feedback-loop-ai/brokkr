//! Seat-record v7 (decision 0109 ruling 1, slice 0109a1): a gate's
//! successful result carries `findings`, `follow_ups` and `covered`
//! beside the four report fields and the accounting fields, every new
//! record is closed, and a v6 run's record carrying any of the three is
//! refused.

use super::*;
use brokkr_core::EventType;
use serde::Deserialize;
use serde_json::{json, Value};

const SITE: &str = "5a1e000000000000000000000000000000000000000000000000000000000000";
const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
const DIGEST: &str = "d16e000000000000000000000000000000000000000000000000000000000000";

fn finding() -> Value {
    json!({"site": SITE, "id": "unchecked-len", "file": "crates/a/src/lib.rs", "line": 12,
           "summary": "the length is not checked", "severity": "medium", "security": false})
}

fn gate(field: &str, value: Value) -> Value {
    let mut record = json!({"result": "residual", "inputs": {}, "notes": "n", "model": "m"});
    record[field] = value;
    record
}

fn with(mut record: Value, pointer: &str, field: &str, value: Value) -> Value {
    record.pointer_mut(pointer).unwrap()[field] = value;
    record
}

fn without(mut record: Value, pointer: &str, field: &str) -> Value {
    record
        .pointer_mut(pointer)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove(field);
    record
}

fn judged(record: &Value, version: SeatRecordVersion) -> Result<(), SeatRecordError> {
    validate_seat_record(record, 9, version)
}

fn refused(contract: &'static str) -> Result<(), SeatRecordError> {
    Err(SeatRecordError {
        seq: 9,
        path: "/".into(),
        contract,
    })
}

/// Every shape v7 admits on a successful result: findings, follow-ups,
/// both `head` field sets and the `artifacts` arm, beside `state` and
/// the accounting fields, with `line` optional and `[]` an empty set.
#[test]
fn v7_admits_the_gate_report_beside_the_four_report_fields() {
    let head = json!({"kind": "head", "head": HEAD, "dirty": true});
    let sha256 = json!({"kind": "head", "head_digest": DIGEST, "dirty": true});
    let long = json!({"kind": "head", "head": format!("{HEAD}{}", "a".repeat(24)), "dirty": false});
    let artifacts = json!({"kind": "artifacts",
                           "files": [{"path": "specs/a/spec.md", "sha256": DIGEST}]});
    for covered in [head, sha256, long, artifacts] {
        assert_eq!(
            judged(&gate("covered", covered.clone()), SeatRecordVersion::V7),
            Ok(()),
            "{covered}"
        );
    }
    let unlined = without(finding(), "", "line");
    for findings in [json!([]), json!([finding()]), json!([finding(), unlined])] {
        assert_eq!(
            judged(&gate("findings", findings.clone()), SeatRecordVersion::V7),
            Ok(())
        );
        assert_eq!(
            judged(&gate("follow_ups", findings), SeatRecordVersion::V7),
            Ok(())
        );
    }
    let whole = json!({"result": "clean", "inputs": {"max_residual_severity": "none"},
                       "notes": "n", "state": {"phase": "x"}, "model": "m", "num_turns": 3,
                       "input_tokens": 10, "output_tokens": 2, "total_cost_usd": 0.5,
                       "findings": [finding()], "follow_ups": [],
                       "covered": {"kind": "head", "head": HEAD, "dirty": true}});
    assert_eq!(judged(&whole, SeatRecordVersion::V7), Ok(()));
}

/// The finding, the covered record and the result are each closed, the
/// id keeps ruling 3's pattern and 64-byte bound, the site is 64 hex, and
/// the head arm takes exactly one of `head` and `head_digest` and no
/// `base`. Each is refused with the contract named and no value echoed.
#[test]
fn v7_closes_the_finding_the_covered_record_and_the_result() {
    let found = |finding: Value| gate("findings", json!([finding]));
    let head = json!({"kind": "head", "head": HEAD, "dirty": true});
    let refusals = [
        found(with(finding(), "", "extra", json!(1))),
        found(without(finding(), "", "site")),
        found(without(finding(), "", "id")),
        found(without(finding(), "", "file")),
        found(without(finding(), "", "summary")),
        found(without(finding(), "", "severity")),
        found(without(finding(), "", "security")),
        found(with(finding(), "", "id", json!("Upper"))),
        found(with(finding(), "", "id", json!("-lead"))),
        found(with(finding(), "", "id", json!("a".repeat(65)))),
        found(with(finding(), "", "site", json!("A".repeat(64)))),
        found(with(finding(), "", "site", json!("a".repeat(63)))),
        found(with(finding(), "", "file", json!("/etc/passwd"))),
        found(with(finding(), "", "file", json!(""))),
        found(with(finding(), "", "line", json!(0))),
        found(with(finding(), "", "line", json!(null))),
        found(with(finding(), "", "summary", json!(""))),
        found(with(finding(), "", "severity", json!("severe"))),
        found(with(finding(), "", "security", json!("no"))),
        gate("findings", json!({"id": "x"})),
        gate(
            "covered",
            with(head.clone(), "", "head_digest", json!(DIGEST)),
        ),
        gate("covered", without(head.clone(), "", "head")),
        gate("covered", with(head.clone(), "", "base", json!(HEAD))),
        gate("covered", without(head.clone(), "", "dirty")),
        gate("covered", with(head.clone(), "", "kind", json!("tree"))),
        gate("covered", with(head.clone(), "", "head", json!("HEAD"))),
        gate("covered", json!({"kind": "artifacts", "files": []})),
        gate(
            "covered",
            json!({"kind": "artifacts", "files": [{"path": "a.md"}]}),
        ),
        gate(
            "covered",
            json!({"kind": "artifacts", "files": [], "head": HEAD}),
        ),
        gate("verdict", json!("clean")),
        gate("checked_head", json!("HEAD")),
        gate("checked_head", json!(HEAD.to_uppercase())),
        gate("checked_head", json!(null)),
    ];
    assert_eq!(judged(&found(finding()), SeatRecordVersion::V7), Ok(()));
    assert_eq!(
        judged(&gate("checked_head", json!(HEAD)), SeatRecordVersion::V7),
        Ok(())
    );
    assert_eq!(
        judged(&gate("covered", head), SeatRecordVersion::V7),
        Ok(())
    );
    for record in refusals {
        let refusal = judged(&record, SeatRecordVersion::V7);
        assert_eq!(refusal, refused(CONTRACT_V7), "{record}");
        assert_eq!(
            refusal.unwrap_err().to_string(),
            "seat record at journal seq 9 violates contracts/seat-record.v7.schema.json at /"
        );
    }
}

/// A finding's `file` is closed alike by v7 and by the engine's typed
/// reader, `brokkr_core::findings::RepoFile`: every case is admitted by
/// both or refused by both, so the journal fence never takes a finding
/// `inspect` would drop.
#[test]
fn v7_refuses_a_findings_file_exactly_where_the_reader_does() {
    let (longest, longer) = ("a".repeat(4096), "a".repeat(4097));
    let wide = "é".repeat(4096);
    let cases = [
        ("crates/a/src/lib.rs", true),
        ("a", true),
        ("a..b/c", true),
        ("..a/b", true),
        ("a/b..", true),
        (".../x", true),
        ("./a", true),
        ("a//b", true),
        (longest.as_str(), true),
        (wide.as_str(), true),
        ("", false),
        ("/etc/passwd", false),
        ("..", false),
        ("../a", false),
        ("a/..", false),
        ("a/../../etc", false),
        ("a/./../b", false),
        (longer.as_str(), false),
    ];
    for (file, admitted) in cases {
        let record = gate(
            "findings",
            json!([with(finding(), "", "file", json!(file))]),
        );
        let schema = judged(&record, SeatRecordVersion::V7).is_ok();
        let reader = brokkr_core::findings::RepoFile::try_from(file.to_string()).is_ok();
        assert_eq!((schema, reader), (admitted, admitted), "{file:.40}");
    }
}

/// A finding's `line` is bounded alike by the fence and by the engine's
/// typed reader, `brokkr_core::findings::Finding` (fix round 0109a1r3,
/// M1): each case, parsed from journal text as the store reads it, is
/// admitted by both or refused by both, standalone and in a step
/// checkpoint's result, so the fence never takes a finding `inspect`
/// cannot read. `1.0` is an integer to draft-07 and `2^64` is past `u64`.
#[test]
fn v7_refuses_a_findings_line_exactly_where_the_reader_does() {
    let cases = [
        ("", true),
        (r#", "line": 1"#, true),
        (r#", "line": 18446744073709551615"#, true),
        (r#", "line": null"#, false),
        (r#", "line": 0"#, false),
        (r#", "line": 1.0"#, false),
        (r#", "line": 18446744073709551616"#, false),
    ];
    for (line, admitted) in cases {
        let text = format!(
            r#"{{"site": "{SITE}", "id": "a", "file": "a.rs", "summary": "s",
                "severity": "low", "security": false{line}}}"#
        );
        let finding: Value = serde_json::from_str(&text).unwrap();
        let reader = brokkr_core::findings::Finding::deserialize(&finding).is_ok();
        fenced_as_read(gate("findings", json!([finding])), reader, admitted);
    }
}

/// `standalone`, a result, judged under v7 alone and as the nested result
/// of the engine's own step checkpoint: the fence and the reader, which
/// said `reader`, each say `admitted`.
fn fenced_as_read(standalone: Value, reader: bool, admitted: bool) {
    let stepped = json!({"step": "sequence-step-finished", "authored": "engine",
                         "step_name": "judge", "model": "m", "result": standalone});
    for record in [&standalone, &stepped] {
        let fence = judged(record, SeatRecordVersion::V7).is_ok();
        assert_eq!((fence, reader), (admitted, admitted), "{record}");
    }
}

/// The fence and the reader agree on `covered` by construction (fix round
/// 0109a1r4, K4): in each `head` field set, the other set's field absent
/// is admitted by both, and present as `null` or as a value is refused
/// by both, as the set's own field `null` is, standalone and in the
/// engine's step checkpoint alike.
#[test]
fn v7_refuses_a_covered_null_exactly_where_the_reader_does() {
    let forms = [
        ("head", HEAD, "head_digest", DIGEST),
        ("head_digest", DIGEST, "head", HEAD),
    ];
    for (own, value, other, other_value) in forms {
        let cases = [
            (String::new(), true),
            (format!(r#", "{other}": null"#), false),
            (format!(r#", "{other}": "{other_value}""#), false),
        ];
        let own_null = (
            format!(r#""kind": "head", "{own}": null, "dirty": true"#),
            false,
        );
        let cases = cases.into_iter().map(|(extra, admitted)| {
            let fields = format!(r#""kind": "head", "{own}": "{value}", "dirty": true{extra}"#);
            (fields, admitted)
        });
        for (fields, admitted) in cases.chain([own_null]) {
            let covered: Value = serde_json::from_str(&format!("{{{fields}}}")).unwrap();
            let reader = brokkr_core::findings::Covered::deserialize(&covered).is_ok();
            fenced_as_read(gate("covered", covered), reader, admitted);
        }
    }
}

/// Ruling 1: only v7 admits the three gate fields. A v6 record carrying
/// any of them is refused under v6, and the v6 record without them is
/// still admitted, so v6's bytes govern exactly what they always did.
#[test]
fn a_v6_record_carrying_a_v7_field_is_refused() {
    let fields = [
        ("findings", json!([finding()])),
        ("follow_ups", json!([])),
        (
            "covered",
            json!({"kind": "head", "head": HEAD, "dirty": true}),
        ),
        ("checked_head", json!(HEAD)),
    ];
    for (field, value) in fields {
        let record = gate(field, value);
        assert_eq!(
            judged(&record, SeatRecordVersion::V6),
            refused(CONTRACT_V6),
            "{field}"
        );
        assert_eq!(judged(&record, SeatRecordVersion::V7), Ok(()), "{field}");
    }
    let plain = json!({"result": "clean", "inputs": {}, "notes": "n", "state": {}});
    assert_eq!(judged(&plain, SeatRecordVersion::V6), Ok(()));
    assert_eq!(judged(&plain, SeatRecordVersion::V7), Ok(()));
}

/// The v7 boundary is the 0.14 line: the 0.13 line keeps v6, so one
/// result with findings is admitted in a 0.14 journal and refused, as
/// v6's violation, in a 0.13 one, at the append fence, and the export
/// and offline-verify sweeps judge the same row by the same contract.
#[test]
fn the_zero_fourteen_line_writes_v7_and_the_thirteen_line_still_reads_v6() {
    for (engine, want) in [
        ("0.13.0", SeatRecordVersion::V6),
        ("0.13.99+build.7", SeatRecordVersion::V6),
        ("0.14.0-rc.1", SeatRecordVersion::V7),
        ("0.14.0", SeatRecordVersion::V7),
        ("0.14.2", SeatRecordVersion::V7),
        ("1.0.0", SeatRecordVersion::V7),
        ("0.14", SeatRecordVersion::V1),
    ] {
        assert_eq!(SeatRecordVersion::of_engine(engine), want, "{engine}");
    }
    let (_dir, mut store) = crate::tests::store();
    for (run, engine) in [("fourteen", "0.14.0"), ("thirteen", "0.13.0")] {
        super::tests::started_run(&mut store, run, engine);
    }
    let result = gate("findings", json!([finding()]));
    let payload = json!({"effect_id": "fx", "attempt_id": "attempt-1", "result": result});
    let crate::StoreError::SeatRecord(old) = store
        .append_next(
            "thirteen",
            EventType::EffectSucceeded,
            payload.clone(),
            None,
            None,
        )
        .unwrap_err()
    else {
        panic!("v6 has no findings");
    };
    assert_eq!(
        old.to_string(),
        "seat record at journal seq 5 violates contracts/seat-record.v6.schema.json at /"
    );
    store
        .append_next("fourteen", EventType::EffectSucceeded, payload, None, None)
        .unwrap();
    let exported = store.export_ndjson("fourteen").unwrap();
    assert_eq!(crate::verify_export(&exported).unwrap().seq, 5);

    let foreign = gate(
        "covered",
        json!({"kind": "head", "head": HEAD, "head_digest": DIGEST, "dirty": true}),
    );
    let planted = json!({"effect_id": "fx", "attempt_id": "attempt-1", "result": foreign});
    crate::tests::plant_unfenced(&mut store, "fourteen", EventType::EffectSucceeded, planted);
    let crate::StoreError::SeatRecord(swept) = store.export_ndjson("fourteen").unwrap_err() else {
        panic!("export refuses a covered record with both head field sets");
    };
    assert_eq!(swept.contract, CONTRACT_V7);
    assert_eq!(swept.seq, 6);
}

/// Fix round 0109a1r3, M2: a non-final step checkpoint's result is the
/// closed result a standalone one is. One whose finding has `line: null`,
/// and one whose `covered` names no head, are each refused at the append
/// fence, by export and by offline verify, all under v7; the sound one is
/// admitted by all three.
#[test]
fn a_step_checkpoints_result_is_closed_at_append_export_and_verify() {
    let unlined = with(finding(), "", "line", json!(null));
    let refused_cases = [
        gate("findings", json!([unlined])),
        gate("covered", json!({"kind": "head", "dirty": true})),
    ];
    let (_dir, mut store) = crate::tests::store();
    for (index, result) in refused_cases.into_iter().enumerate() {
        let run = format!("refused-{index}");
        refused_at_append_export_and_verify(&mut store, &run, stepped(result), "/");
    }
    super::tests::started_run(&mut store, "sound", "0.14.0");
    let sound = stepped(gate("findings", json!([finding()])));
    (store.append_next("sound", EventType::EffectCheckpointed, sound, None, None)).unwrap();
    let exported = store.export_ndjson("sound").unwrap();
    assert!(crate::verify_export(&exported).is_ok());
}

/// The engine's step checkpoint payload around `result`.
fn stepped(result: Value) -> Value {
    let checkpoint = json!({"step": "sequence-step-finished", "authored": "engine",
                            "step_name": "judge", "model": "m", "result": result});
    json!({"effect_id": "fx", "attempt_id": "attempt-1", "checkpoint": checkpoint})
}

/// `payload`, a checkpoint in a new v7 run `run`, is refused at the append
/// fence, by export and by offline verify alike, at `path`.
fn refused_at_append_export_and_verify(
    store: &mut crate::Store,
    run: &str,
    payload: Value,
    path: &str,
) {
    let rows = [(EventType::EffectCheckpointed, payload)];
    refused_after(store, run, &rows, path);
}

/// The last of `rows`, in a new v7 run `run` whose earlier rows are
/// appended as they stand, refused at the append fence, by export and by
/// offline verify alike, at `path`.
fn refused_after(store: &mut crate::Store, run: &str, rows: &[(EventType, Value)], path: &str) {
    super::tests::started_run(store, run, "0.14.0");
    let ((kind, payload), earlier) = rows.split_last().unwrap();
    for (kind, payload) in earlier {
        (store.append_next(run, *kind, payload.clone(), None, None)).unwrap();
    }
    let (kind, payload) = (*kind, payload.clone());
    let crate::StoreError::SeatRecord(appended) = store
        .append_next(run, kind, payload.clone(), None, None)
        .unwrap_err()
    else {
        panic!("the fence refuses the step's result");
    };
    assert_eq!(
        (appended.contract, appended.path.as_str()),
        (CONTRACT_V7, path)
    );
    crate::tests::plant_unfenced(store, run, kind, payload);
    let crate::StoreError::SeatRecord(exported) = store.export_ndjson(run).unwrap_err() else {
        panic!("export refuses the step's result");
    };
    assert_eq!(
        (exported.contract, exported.path.as_str()),
        (CONTRACT_V7, path)
    );
    let ndjson: String = (store.load(run).unwrap().iter())
        .map(|event| serde_json::to_string(event).unwrap() + "\n")
        .collect();
    let Err(crate::VerifyError::SeatRecord(verified)) = crate::verify_export(&ndjson) else {
        panic!("offline verify refuses the step's result");
    };
    assert_eq!(verified, exported);
}

/// Fix round 0109a1r5, V3: a result's subsets are checked wherever it
/// sits. A valid word with `input_tokens: 1` and `cache_read_tokens: 2` is
/// refused standalone at `/cache_read_tokens`, and in the engine's step
/// checkpoint at `/result/cache_read_tokens`, at append, export and verify.
#[test]
fn a_nested_results_subsets_are_checked_as_a_standalone_results_are() {
    let over = json!({"result": "clean", "input_tokens": 1, "cache_read_tokens": 2});
    assert_eq!(
        judged(&over, SeatRecordVersion::V7),
        Err(SeatRecordError {
            seq: 9,
            path: "/cache_read_tokens".into(),
            contract: CONTRACT_V7,
        })
    );
    let (_dir, mut store) = crate::tests::store();
    let path = "/result/cache_read_tokens";
    refused_at_append_export_and_verify(&mut store, "nested", stepped(over), path);
}

/// Fix round 0109a1r6, W3: a marked settled member's payload reads as a
/// verdict, so it gets every fence a standalone result gets. A current
/// panel member settled with `input_tokens: 1` and `cache_read_tokens: 2`
/// beside its failed sibling, its engine marker journaled, is refused at
/// `/settled_members/a/cache_read_tokens` at append, export and verify. The
/// same row beside a released engine's unauthored marker keeps its
/// admission.
///
/// Fix round 0109a1r7, X3: the marked payload is a result, so a
/// checkpoint-shaped one with no result word, `{"step": "seat-turn"}`, is
/// refused at `/settled_members/a` at all three fences, by the contract's
/// closed result branch; unmarked, it keeps its admission.
#[test]
fn a_marked_settled_members_payload_is_validated_as_a_standalone_result() {
    let over = json!({"result": "clean", "input_tokens": 1, "cache_read_tokens": 2});
    let checkpoint_shaped = json!({"step": "seat-turn"});
    for (run, payload, path) in [
        ("over", over, "/settled_members/a/cache_read_tokens"),
        ("wordless", checkpoint_shaped, "/settled_members/a"),
    ] {
        settled_refused_only_beside_the_mark(run, payload, path);
    }
}

/// Fix round 0109a1r7, X4: a settled member's refusal names that member,
/// read back from the pointer the fence writes, and no other refusal
/// names one.
#[test]
fn a_settled_members_refusal_names_its_member() {
    let named = |path: &str| {
        let refusal = SeatRecordError {
            seq: 1,
            path: path.into(),
            contract: CONTRACT_V7,
        };
        refusal.settled_member().map(str::to_string)
    };
    assert_eq!(
        named("/settled_members/a/cache_read_tokens").as_deref(),
        Some("a")
    );
    assert_eq!(
        named("/settled_members/positions:b").as_deref(),
        Some("positions:b")
    );
    assert_eq!(named("/result/cache_read_tokens"), None);
    assert_eq!(named("/"), None);
}

/// A failed panel row settling member `a` with `payload`: beside the
/// engine's authored marker it is refused at `path` at append, export and
/// verify; beside a released engine's unauthored marker it is admitted.
fn settled_refused_only_beside_the_mark(run: &str, payload: Value, path: &str) {
    let failed = json!({"effect_id": "fx", "attempt_id": "attempt-1", "error": "b failed",
                        "tree": {}, "settled_members": {"a": payload}});
    let marker = |authored: bool| {
        let mut marker = json!({"step": "panel-member-finished", "member": "a",
                                "outcome": "succeeded", "model": "m"});
        if authored {
            marker["authored"] = json!("engine");
        }
        json!({"effect_id": "fx", "attempt_id": "attempt-1", "checkpoint": marker})
    };
    let rows = |authored| {
        [
            (EventType::EffectCheckpointed, marker(authored)),
            (EventType::EffectFailed, failed.clone()),
        ]
    };
    let (_dir, mut store) = crate::tests::store();
    refused_after(&mut store, run, &rows(true), path);
    super::tests::started_run(&mut store, "released", "0.14.0");
    for (kind, payload) in rows(false) {
        (store.append_next("released", kind, payload, None, None)).unwrap();
    }
    let exported = store.export_ndjson("released").unwrap();
    assert!(crate::verify_export(&exported).is_ok(), "{run}");
}

/// Fix round 0109a1r4, K5: a released engine's records stay admissible.
/// The tagged 0.14.0 and 0.15.0 engines checked only a non-final step's
/// word and journaled its whole result, so a v6-valid step checkpoint
/// whose nested result carries an extra property is appended, exported
/// and verified offline in a journal of either, exactly as under v6. The
/// same checkpoint the engine authored is v7's closed result, and is
/// refused, as M2 holds.
#[test]
fn a_released_engines_step_checkpoint_stays_admissible_under_v7() {
    let result = json!({"result": "clean", "diagnostic": {"took_ms": 12}, "findings": "none"});
    let released = json!({"step": "sequence-step-finished", "step_name": "judge",
                          "model": "m", "result": result});
    assert_eq!(judged(&released, SeatRecordVersion::V6), Ok(()));
    let (_dir, mut store) = crate::tests::store();
    for engine in ["0.14.0", "0.15.0"] {
        assert_eq!(SeatRecordVersion::of_engine(engine), SeatRecordVersion::V7);
        super::tests::started_run(&mut store, engine, engine);
        let payload = json!({"effect_id": "fx", "attempt_id": "attempt-1",
                             "checkpoint": released});
        (store.append_next(engine, EventType::EffectCheckpointed, payload, None, None)).unwrap();
        let exported = store.export_ndjson(engine).unwrap();
        assert!(crate::verify_export(&exported).is_ok(), "{engine}");
    }
    let mut authored = released;
    authored["authored"] = json!("engine");
    assert_eq!(
        judged(&authored, SeatRecordVersion::V7),
        refused(CONTRACT_V7)
    );
}
