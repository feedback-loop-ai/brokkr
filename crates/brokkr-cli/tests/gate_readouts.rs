//! Decision 0109 slice 0109a1's read-back: `brokkr inspect` prints what a
//! gate verdict recorded beside its word — its covered head and each
//! finding — and a run an older engine wrote prints none and invents
//! none. Every journal is the test's own, in a temporary directory.

use std::path::Path;

use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::boundary_readouts::brokkr;

const SITE: &str = "5a1e00000000beef000000000000000000000000000000000000000000000000";
const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

/// A run through `review`, whose one result is `result`, written by the
/// engine line `engine`.
fn plant(db: &Path, run_id: &str, engine: &str, result: Value) {
    let manifest = json!({"engine": engine});
    let attempt = json!({"effect_id": "e1", "attempt_id": "a1"});
    let mut started = attempt.clone();
    started["driver"] = json!("claude");
    let mut succeeded = attempt;
    succeeded["result"] = result;
    let journal = [
        (
            EventType::RunStarted,
            json!({"feature": "gate it", "manifest": &manifest}),
        ),
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "review", "phase": "review"}),
        ),
        (EventType::EffectStarted, started),
        (EventType::EffectSucceeded, succeeded),
    ];
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, "gate it", "test", &manifest)
        .unwrap();
    for (kind, payload) in journal {
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    }
}

#[test]
fn inspect_prints_the_findings_and_covered_of_a_v7_gate() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let result = json!({
        "result": "residual", "inputs": {}, "notes": "two",
        "findings": [
            {"site": SITE, "id": "unchecked-len", "file": "crates/a/src/lib.rs", "line": 12,
             "summary": "the length is not checked", "severity": "medium", "security": false},
            {"site": SITE, "id": "token-in-log", "file": "crates/b/src/log.rs",
             "summary": "a token reaches the log", "severity": "high", "security": true}
        ],
        "covered": {"kind": "head", "head": HEAD, "dirty": true}
    });
    plant(&db, "gated", "0.14.0", result);
    let db = db.to_str().unwrap();
    let (inspect, err, ok) = brokkr(dir.path(), &["inspect", "--run", "gated", "--db", db]);
    assert!(ok, "{inspect}{err}");
    let block = format!(
        "\nverdicts\n  review · covered head {HEAD} · dirty\n  review · finding unchecked-len · \
         site 5a1e00000000 · medium · crates/a/src/lib.rs:12 · the length is not checked\n  \
         review · finding token-in-log · site 5a1e00000000 · high · security · \
         crates/b/src/log.rs · a token reaches the log\n"
    );
    assert!(inspect.ends_with(&block), "{inspect}");
}

/// Decision 0109 ruling 6: a v6 run's gate result records no key, and
/// `inspect` prints no verdict block for it rather than reading one out
/// of its notes or its residual inputs.
#[test]
fn inspect_prints_no_verdict_for_a_v6_gate_and_invents_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let result = json!({"result": "residual",
                        "inputs": {"max_residual_severity": "high",
                                   "has_security_residual": true},
                        "notes": "unchecked-len at crates/a/src/lib.rs:12"});
    plant(&db, "older", "0.13.0", result);
    let db = db.to_str().unwrap();
    let (inspect, err, ok) = brokkr(dir.path(), &["inspect", "--run", "older", "--db", db]);
    assert!(ok, "{inspect}{err}");
    assert!(!inspect.contains("verdicts"), "{inspect}");
    assert!(!inspect.contains("· finding "), "{inspect}");
    assert!(!inspect.contains("· covered "), "{inspect}");
}
