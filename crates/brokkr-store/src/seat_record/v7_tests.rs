//! Seat-record v7 (decision 0109 ruling 1, slice 0109a1): a gate's
//! successful result carries `findings`, `follow_ups` and `covered`
//! beside the four report fields and the accounting fields, every new
//! record is closed, and a v6 run's record carrying any of the three is
//! refused.

use super::*;
use brokkr_core::EventType;
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
