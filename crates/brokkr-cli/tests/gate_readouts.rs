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
    plant_phases(db, run_id, engine, &[("review", &[], result)]);
}

/// One phase a planted run passes: its name, its panel members' `(name,
/// site)` stamps, and its result.
type Planted<'a> = (&'a str, &'a [(&'a str, &'a str)], Value);

/// A run through each `(phase, members, result)` in turn, one effect per
/// phase, whose seat is the phase and whose panel `members` each have the
/// engine's own finish marker carrying its `(name, site)` stamp.
fn plant_phases(db: &Path, run_id: &str, engine: &str, phases: &[Planted<'_>]) {
    let manifest = json!({"engine": engine});
    let mut journal = vec![(
        EventType::RunStarted,
        json!({"feature": "gate it", "manifest": &manifest}),
    )];
    for (index, (phase, members, result)) in phases.iter().enumerate() {
        if let Some(index) = index.checked_sub(1) {
            let (from, _, word) = &phases[index];
            let decided = json!({"rule_id": "ON", "severity": "normal", "from": from,
                                 "next": phase, "result": word["result"]});
            journal.push((EventType::TransitionDecided, decided));
        }
        let attempt = json!({"effect_id": format!("e{index}"), "attempt_id": format!("a{index}")});
        let mut started = attempt.clone();
        started["driver"] = json!("claude");
        let mut requested = attempt.clone();
        requested["seat"] = json!(phase);
        requested["phase"] = json!(phase);
        journal.push((EventType::PhaseEntered, json!({"phase": phase})));
        journal.push((EventType::EffectRequested, requested));
        journal.push((EventType::EffectStarted, started));
        for (member, site) in members.iter() {
            let mut checkpoint = attempt.clone();
            checkpoint["checkpoint"] = json!({"step": "panel-member-finished",
                                              "authored": "engine", "member": member,
                                              "site_ref": site, "outcome": "succeeded",
                                              "model": "m"});
            journal.push((EventType::EffectCheckpointed, checkpoint));
        }
        let mut succeeded = attempt;
        succeeded["result"] = result.clone();
        journal.push((EventType::EffectSucceeded, succeeded));
    }
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

/// A finding at `site` with `id`, as the engine journals it.
fn stamped(site: &str, id: &str) -> Value {
    json!({"site": site, "id": id, "file": "a.rs", "summary": "s", "severity": "low",
           "security": false})
}

/// R5: a scoped `inspect` prints only its own scope's verdicts. `--phase
/// verify` keeps verify's record and none of review's findings; `--seat`
/// on one review position keeps that position's findings and the head
/// the panel covered, and none of the other position's or of verify.
#[test]
fn a_scoped_inspect_prints_only_its_own_scopes_verdicts() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let other = "07e4000000000000000000000000000000000000000000000000000000000000";
    let verify = json!({"result": "pass", "inputs": {}, "notes": "",
                        "findings": [stamped(SITE, "verify-note")],
                        "covered": {"kind": "head", "head": HEAD, "dirty": true}});
    let review = json!({"result": "residual", "inputs": {}, "notes": "",
                        "findings": [stamped(SITE, "adversarial-one"), stamped(other, "correct-one")]});
    let members: &[(&str, &str)] = &[("adversarial", SITE), ("correctness", other)];
    plant_phases(
        &db,
        "scoped",
        "0.14.0",
        &[("verify", &[], verify), ("review", members, review)],
    );
    let db = db.to_str().unwrap();
    let read = |scope: &[&str]| {
        let args = [&["inspect", "--run", "scoped", "--db", db][..], scope].concat();
        let (inspect, err, ok) = brokkr(dir.path(), &args);
        assert!(ok, "{inspect}{err}");
        inspect
            .split("\nverdicts\n")
            .nth(1)
            .unwrap_or("")
            .to_string()
    };
    let finding = |phase: &str, id: &str, by: &str, site: &str| {
        format!(
            "  {phase} · finding {id} · by {by} · site {} · low · a.rs · s\n",
            &site[..12]
        )
    };
    let covered = format!("  verify · covered head {HEAD} · dirty\n");
    let adversarial = finding("review", "adversarial-one", "review:adversarial", SITE);
    let correct = finding("review", "correct-one", "review:correctness", other);
    assert_eq!(
        read(&["--phase", "verify"]),
        format!(
            "{covered}{}",
            finding("verify", "verify-note", "verify", SITE)
        )
    );
    assert_eq!(read(&["--seat", "review:adversarial"]), adversarial);
    assert_eq!(read(&["--seat", "review:correctness"]), correct);
    assert_eq!(
        read(&[]),
        format!(
            "{covered}{}{adversarial}{correct}",
            finding("verify", "verify-note", "verify", SITE),
        )
    );
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
         by review · site 5a1e00000000 · medium · crates/a/src/lib.rs:12 · the length is not \
         checked\n  review · finding token-in-log · by review · site 5a1e00000000 · high · \
         security · crates/b/src/log.rs · a token reaches the log\n"
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
