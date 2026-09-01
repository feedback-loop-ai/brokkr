//! Differential tests: the Rust evaluator against the Python oracle.
//!
//! The corpus is exhaustive over behavior classes (see
//! contracts/README.md). Expected `problem` strings are diagnostic
//! evidence, not contract: parity compares rule id, next phase,
//! severity, and park-vs-rule including whether `problem` is set.

use std::fs;
use std::path::PathBuf;

use brokkr_core::policy::{Machine, Outcome, RULING_SEVERITIES};
use serde_json::{Map, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn production_machine() -> Machine {
    let table: Value = serde_json::from_str(
        &fs::read_to_string(repo_root().join("policy/phase-machine.json")).expect("table"),
    )
    .expect("table parses");
    Machine::from_table(&table).expect("production table loads under the strict loader")
}

#[test]
fn production_table_loads() {
    let machine = production_machine();
    assert_eq!(machine.initial, "intake");
    assert_eq!(machine.terminal, vec!["done", "stop"]);
}

#[test]
fn corpus_parity_with_python_oracle() {
    let machine = production_machine();
    let corpus =
        fs::read_to_string(repo_root().join("fixtures/evaluator/corpus.ndjson")).expect("corpus");
    let mut cases = 0usize;
    for line in corpus.lines().filter(|l| !l.trim().is_empty()) {
        let case: Value = serde_json::from_str(line).expect("corpus line parses");
        let phase = case["phase"].as_str().unwrap();
        let result = case["result"].as_str().unwrap();
        let inputs: Map<String, Value> = case["inputs"].as_object().unwrap().clone();
        let expect = &case["expect"];
        let outcome = machine.evaluate(phase, result, &inputs);
        match expect["kind"].as_str().unwrap() {
            "ruling" => match &outcome {
                Outcome::Ruling {
                    rule_id,
                    next_phase,
                    severity,
                    ..
                } => {
                    assert_eq!(rule_id, expect["rule_id"].as_str().unwrap(), "case: {case}");
                    assert_eq!(next_phase, expect["next"].as_str().unwrap(), "case: {case}");
                    assert_eq!(
                        severity,
                        expect["severity"].as_str().unwrap(),
                        "case: {case}"
                    );
                    assert!(RULING_SEVERITIES.contains(&severity.as_str()));
                }
                other => panic!("expected ruling, got {other:?} for case: {case}"),
            },
            "no_rule" => match &outcome {
                Outcome::NoRule { problem } => {
                    assert_eq!(
                        problem.is_some(),
                        !expect["problem"].is_null(),
                        "park-problem presence mismatch for case: {case} (got {problem:?})"
                    );
                }
                other => panic!("expected no_rule, got {other:?} for case: {case}"),
            },
            kind => panic!("unknown expectation kind {kind}"),
        }
        cases += 1;
    }
    assert!(cases >= 90, "corpus unexpectedly small: {cases} cases");
}

#[test]
fn table_wide_lint_matches_python_suite() {
    // Mirrors tests/test_machine.py: hard => stop, flagged => non-terminal,
    // security-hold => hard stop, review unavoidable on any path to ship.
    let table: Value = serde_json::from_str(
        &fs::read_to_string(repo_root().join("policy/phase-machine.json")).unwrap(),
    )
    .unwrap();
    let machine = production_machine();
    let rules = table["rules"].as_array().unwrap();
    for rule in rules {
        let severity = rule
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("normal");
        if severity == "hard" {
            assert_eq!(rule["next"], "stop", "rule {}", rule["id"]);
        }
        if severity == "flagged" {
            assert!(
                !machine
                    .terminal
                    .contains(&rule["next"].as_str().unwrap().to_string()),
                "rule {}",
                rule["id"]
            );
        }
        if rule["result"] == "security-hold" {
            assert_eq!(rule["next"], "stop", "rule {}", rule["id"]);
            assert_eq!(severity, "hard", "rule {}", rule["id"]);
        }
    }
    // Graph: with review removed, ship and done are unreachable.
    let mut reachable = vec![machine.initial.clone()];
    let mut frontier = vec![machine.initial.clone()];
    while let Some(node) = frontier.pop() {
        for rule in rules {
            if rule["from"] == node.as_str() && rule["from"] != "review" && rule["next"] != "review"
            {
                let next = rule["next"].as_str().unwrap().to_string();
                if !reachable.contains(&next) {
                    reachable.push(next.clone());
                    frontier.push(next);
                }
            }
        }
    }
    assert!(!reachable.contains(&"ship".to_string()));
    assert!(!reachable.contains(&"done".to_string()));
}
