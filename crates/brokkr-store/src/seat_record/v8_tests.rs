//! Seat-record v8 (decision 0075 ruling 2, with its addendum of
//! 2026-11-21): v7 with `transcript.kind` and `root_session.kind` opened
//! from closed lists to one grammar, `none` refused as a root session
//! kind, and nothing else changed.

use super::*;
use serde_json::{json, Value};

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

fn judged(record: &Value, version: SeatRecordVersion) -> Result<(), SeatRecordError> {
    validate_seat_record(record, 4, version)
}

fn transcript(kind: &str, locator: &str, home: &str) -> Value {
    json!({"step": "transcript", "model": "not reported",
           "transcript": {"kind": kind, "locator": locator, "home": home}})
}

fn rooted(kind: &str) -> Value {
    json!({"step": "seat-turn", "model": "m", "root_session": {
        "kind": kind, "id": "019c4b7e", "harness_version": "1.2.3", "persistent": true}})
}

/// Records v7 judges, admitted and refused alike: every transcript and
/// root session kind it lists, `none`'s empty locator, the gate report,
/// and refusals in both kind members and beside them.
fn v7_corpus() -> Vec<Value> {
    let finding = json!({"site": "5a1e".repeat(16), "id": "unchecked-len",
                         "file": "crates/a/src/lib.rs", "line": 12, "summary": "s",
                         "severity": "medium", "security": false});
    let gate = json!({"result": "residual", "inputs": {}, "notes": "n", "findings": [finding],
                      "covered": {"kind": "head", "head": HEAD, "dirty": true}});
    let mut corpus = vec![
        transcript("claude-session", "8f2c", "/h/.claude/projects"),
        transcript("codex-thread", "019c", "/tmp/codex"),
        transcript("dsh-session", "s-1", "/h/.dsh"),
        transcript("none", "", ""),
        transcript("none", "019c", ""),
        transcript("codex-thread", &"x".repeat(81), "/tmp/codex"),
        rooted("claude-session"),
        rooted("codex-thread"),
        rooted("dsh-session"),
        rooted("none"),
        json!({"step": "s", "root_session": {"kind": "claude-session", "id": "abc"}}),
        gate.clone(),
        json!({"result": "clean", "inputs": {}, "notes": "n", "covered": {"kind": "head", "dirty": true}}),
    ];
    let mut both = gate;
    both["covered"]["head_digest"] = json!("d16e".repeat(16));
    corpus.push(both);
    corpus
}

/// Every record v7 judges keeps its verdict, refusal included, under v8.
#[test]
fn every_v7_record_keeps_its_verdict_under_v8() {
    let mut admitted = 0;
    for record in v7_corpus() {
        let v7 = judged(&record, SeatRecordVersion::V7).map_err(|refusal| refusal.path);
        let v8 = judged(&record, SeatRecordVersion::V8).map_err(|refusal| refusal.path);
        assert_eq!(v7, v8, "{record}");
        admitted += usize::from(v7.is_ok());
    }
    assert_eq!(admitted, 8, "the corpus holds admitted and refused records");
}

/// v8 admits a kind in the grammar that no list names, in both members,
/// and v7's frozen closed lists still refuse it.
#[test]
fn v8_admits_a_declared_grammar_kind_and_v7_still_refuses_it() {
    let refused_v7 = Err(SeatRecordError {
        seq: 4,
        path: "/".into(),
        contract: CONTRACT_V7,
        settled_member: None,
    });
    let at_bound = format!("k{}9", "-9".repeat(19));
    assert_eq!(at_bound.len(), 40);
    for record in [
        transcript("fixture-trail", "t-1", "/h/.fixture"),
        transcript(&at_bound, "t-1", "/h"),
        rooted("fixture-session"),
        rooted("a1-b2"),
    ] {
        assert_eq!(judged(&record, SeatRecordVersion::V8), Ok(()), "{record}");
        assert_eq!(
            judged(&record, SeatRecordVersion::V7),
            refused_v7,
            "{record}"
        );
    }
}

/// The first violation of `def`'s definition alone, from v8's own bytes:
/// the member's path and the keyword of the clause it broke.
fn violation(def: &str, member: &Value) -> Option<(String, String)> {
    let mut schema: Value = serde_json::from_str(SCHEMA_V8).unwrap();
    schema.as_object_mut().unwrap().remove("oneOf");
    schema["$ref"] = json!(format!("#/$defs/{def}"));
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let error = validator.iter_errors(member).next()?;
    let clause = error.schema_path().to_string();
    let keyword = clause.rsplit('/').next().unwrap_or_default().to_string();
    Some((error.instance_path().to_string(), keyword))
}

/// A kind outside the grammar, an over-long kind, and `none` as a root
/// session kind are each refused at the member's `kind`, by the clause
/// that rules it, and the whole record is refused under v8.
#[test]
fn v8_refuses_an_ungrammatical_an_over_long_and_a_none_root_kind_by_path() {
    let over_long = "k".repeat(41);
    let root = |kind: &str| json!({"kind": kind, "id": "019c", "harness_version": "1", "persistent": true});
    let trail = |kind: &str| json!({"kind": kind, "locator": "", "home": ""});
    let cases = [
        ("transcript", trail("Fixture"), "pattern"),
        ("transcript", trail("fixture--trail"), "pattern"),
        ("transcript", trail("fixture-"), "pattern"),
        ("transcript", trail("9lives"), "pattern"),
        ("transcript", trail(&over_long), "maxLength"),
        ("root_session", root("fixture_session"), "pattern"),
        ("root_session", root(&over_long), "maxLength"),
        ("root_session", root("none"), "not"),
    ];
    for (def, member, keyword) in cases {
        assert_eq!(
            violation(def, &member),
            Some(("/kind".to_string(), keyword.to_string())),
            "{member}"
        );
        let mut record = json!({"step": "s", "model": "m"});
        record[def] = member;
        assert_eq!(
            judged(&record, SeatRecordVersion::V8).map_err(|refusal| refusal.contract),
            Err(CONTRACT_V8),
            "{record}"
        );
    }
    assert_eq!(violation("transcript", &trail("dsh-session")), None);
    assert_eq!(violation("root_session", &root("dsh-session")), None);
}

/// v8 stands at v7's own line: every engine v7 was dispatched for reads
/// v8, the 0.13 line keeps v6, and no engine string selects v7.
#[test]
fn the_zero_fourteen_line_writes_v8_and_dispatch_never_chooses_v7() {
    for (engine, want) in [
        ("0.13.99", SeatRecordVersion::V6),
        ("0.14.0-rc.1", SeatRecordVersion::V8),
        ("0.14.0", SeatRecordVersion::V8),
        ("0.15.2", SeatRecordVersion::V8),
        ("1.0.0", SeatRecordVersion::V8),
    ] {
        assert_eq!(SeatRecordVersion::of_engine(engine), want, "{engine}");
    }
    for minor in 0..=40 {
        let engine = format!("0.{minor}.0");
        assert_ne!(
            SeatRecordVersion::of_engine(&engine),
            SeatRecordVersion::V7,
            "{engine}"
        );
    }
    let (_dir, mut store) = crate::tests::store();
    super::tests::started_run(&mut store, "open", "0.15.2");
    let row = json!({"effect_id": "fx", "attempt_id": "attempt-1",
                     "checkpoint": transcript("fixture-trail", "t-1", "/h/.fixture")});
    let checkpointed = brokkr_core::EventType::EffectCheckpointed;
    (store.append_next("open", checkpointed, row, None, None)).unwrap();
    let exported = store.export_ndjson("open").unwrap();
    assert_eq!(crate::verify_export(&exported).unwrap().seq, 5);
}
