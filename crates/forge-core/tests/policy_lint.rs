//! Loader-rejection suite: the Rust port of the retired oracle's
//! `test_machine.py` lint cases (decision 0009). A malformed table
//! refuses to LOAD; it never degrades into rules that silently stop
//! matching (the forge-control.py typo incident, twice removed).

use forge_core::policy::Machine;
use serde_json::{json, Value};

fn minimal_table(rules: Value) -> Value {
    json!({
        "phases": ["a", "b", "stop"],
        "initial": "a",
        "terminal": ["stop"],
        "rules": rules,
    })
}

fn rule(overrides: Value) -> Value {
    let mut base = json!({"id": "R", "from": "a", "result": "ok", "next": "b", "reason": "r"});
    for (k, v) in overrides.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}

#[test]
fn loader_rejects_malformed_conditions() {
    for when in [
        json!({"has_security_residualz": true}),  // the recorded typo incident
        json!({"skip_verify": "yes"}),            // non-bool threshold
        json!({"consecutive_failures_gte": "2"}), // counter not numeric
        json!({"retries_gte": 2}),                // undeclared counter
        json!({"max_residual_severity_above": "banana"}), // unknown severity
        json!({"severity_above": "medium"}),      // undeclared severity axis
    ] {
        let table = minimal_table(json!([rule(json!({"when": when}))]));
        assert!(Machine::from_table(&table).is_err(), "accepted: {when}");
    }
}

#[test]
fn loader_rejects_structural_defects() {
    // Unknown ruling severity.
    let table = minimal_table(json!([rule(json!({"severity": "critical"}))]));
    assert!(Machine::from_table(&table).is_err());
    // Rule shadowed by a preceding unconditional rule (dead policy).
    let table = minimal_table(json!([
        rule(json!({"id": "R1"})),
        rule(json!({"id": "R2", "when": {"skip_verify": true}})),
    ]));
    assert!(Machine::from_table(&table).is_err());
    // Rules leaving a terminal phase.
    let table = minimal_table(json!([rule(json!({"from": "stop"}))]));
    assert!(Machine::from_table(&table).is_err());
    // Duplicate rule ids.
    let table = minimal_table(json!([rule(json!({})), rule(json!({}))]));
    assert!(Machine::from_table(&table).is_err());
    // requires_artifacts as a bare string.
    let table = minimal_table(json!([rule(json!({"requires_artifacts": "spec.md"}))]));
    assert!(Machine::from_table(&table).is_err());
}
