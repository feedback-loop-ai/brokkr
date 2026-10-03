//! Loader-rejection suite: the Rust port of the retired oracle's
//! `test_machine.py` lint cases (decision 0009). A malformed table
//! refuses to LOAD; it never degrades into rules that silently stop
//! matching (the heritage control-script typo incident, twice removed).

use brokkr_core::policy::audit::{Finding, Refusal};
use brokkr_core::policy::{Machine, PolicyError, TABLE_SCHEMA_V1, TABLE_SCHEMA_V2};
use serde_json::{json, Value};

/// A live table (decision 0050, ruling 3) around `rules`, so each case
/// below is refused for the defect it plants and not for a dead end.
fn minimal_table(rules: Value) -> Value {
    let mut rules = rules.as_array().unwrap().clone();
    rules.push(json!({"id": "B", "from": "b", "result": "ok", "next": "stop", "reason": "r"}));
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
        json!({"has_security_residualz": true}), // the recorded typo incident
        json!({"skip_verify": "yes"}),           // non-bool threshold
        json!({"consecutive_failures_gte": "2"}), // counter not numeric
        json!({"retries_gte": 2}),               // undeclared counter
        json!({"max_residual_severity_above": "banana"}), // unknown severity
        json!({"severity_above": "medium"}),     // undeclared severity axis
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

// Decision 0050's first enactment (#429): order (ruling 2), liveness
// (ruling 3) and v2 presence (ruling 1) refuse at load. Each case plants
// the defect beside the valid table it breaks.

/// `work → review → done`, where review's `security-hold` stops hard,
/// holding `review` after those rules.
fn delivery(schema: &str, review: Value) -> Value {
    let mut rules = vec![
        json!({"id": "WORK", "from": "work", "result": "complete", "next": "review",
               "reason": "r"}),
        json!({"id": "CLEAN", "from": "review", "result": "clean", "next": "done",
               "reason": "r"}),
        json!({"id": "HALT", "from": "review", "result": "security-hold", "next": "stop",
               "severity": "hard", "reason": "r"}),
    ];
    rules.extend(review.as_array().unwrap().iter().cloned());
    json!({"schema": schema, "phases": ["work", "review", "done", "stop"],
           "initial": "work", "terminal": ["done", "stop"], "rules": rules})
}

fn residual(id: &str, when: Value, next: &str, severity: &str) -> Value {
    json!({"id": id, "from": "review", "result": "residual", "when": when, "next": next,
           "severity": severity, "reason": "r"})
}

fn refusal(table: &Value) -> PolicyError {
    Machine::from_table(table).unwrap_err()
}

/// Ruling 2's refusal of `rule`, dead behind `by`.
fn dead_behind(rule: &str, by: &[&str]) -> PolicyError {
    let rule = rule.to_string();
    PolicyError::Refused(Refusal::Order(match by {
        [behind] => Finding::Shadowed {
            rule,
            behind: (*behind).to_string(),
        },
        _ => Finding::Covered {
            rule,
            by: by.iter().map(ToString::to_string).collect(),
        },
    }))
}

/// The arms that partition `skip_verify`: together they rule every
/// present valuation of the flag.
fn partition() -> [Value; 2] {
    [
        residual("VERIFIED", json!({"skip_verify": false}), "done", "normal"),
        residual("UNVERIFIED", json!({"skip_verify": true}), "done", "normal"),
    ]
}

#[test]
fn a_vacuous_ceiling_ahead_of_a_hard_floor_is_refused() {
    let permit = residual(
        "PERMIT",
        json!({"max_residual_severity_at_most": "critical"}),
        "done",
        "normal",
    );
    let deny = residual(
        "DENY",
        json!({"max_residual_severity_above": "medium"}),
        "stop",
        "hard",
    );
    assert_eq!(
        refusal(&delivery(TABLE_SCHEMA_V2, json!([permit, deny]))),
        dead_behind("DENY", &["PERMIT"])
    );
    Machine::from_table(&delivery(TABLE_SCHEMA_V2, json!([deny, permit]))).unwrap();
}

#[test]
fn two_flag_arms_that_partition_an_axis_ahead_of_a_hard_rule_are_refused() {
    let [verified, unverified] = partition();
    let exhausted = residual(
        "EXHAUSTED",
        json!({"consecutive_failures_gte": 2}),
        "stop",
        "hard",
    );
    assert_eq!(
        refusal(&delivery(
            TABLE_SCHEMA_V2,
            json!([verified, unverified, exhausted])
        )),
        dead_behind("EXHAUSTED", &["VERIFIED", "UNVERIFIED"])
    );
    Machine::from_table(&delivery(
        TABLE_SCHEMA_V2,
        json!([exhausted, verified, unverified]),
    ))
    .unwrap();
}

/// An absent flag is outside the domain (the addendum's reading), so the
/// park it would reach is refused: presence, not a fallback, answers for
/// a seat that omits the flag.
#[test]
fn a_fallback_behind_complementary_flag_arms_fires_on_no_present_valuation() {
    let [verified, unverified] = partition();
    let fallback = json!({"id": "FALLBACK", "from": "review", "result": "residual",
                          "park": true, "reason": "r"});
    assert_eq!(
        refusal(&delivery(
            TABLE_SCHEMA_V2,
            json!([verified, unverified, fallback])
        )),
        dead_behind("FALLBACK", &["VERIFIED", "UNVERIFIED"])
    );
    Machine::from_table(&delivery(TABLE_SCHEMA_V2, json!([verified, fallback]))).unwrap();
}

/// `delivery` with a `loop` phase that leaves for `done`, holding `extra`.
fn looped(extra: Value) -> Value {
    let mut rules = vec![
        json!({"id": "OUT", "from": "loop", "result": "done", "next": "done",
                                "reason": "r"}),
    ];
    rules.extend(extra.as_array().unwrap().iter().cloned());
    let mut table = delivery(TABLE_SCHEMA_V2, Value::Array(rules));
    table["phases"] = json!(["work", "review", "loop", "done", "stop"]);
    table
}

fn into_loop() -> Value {
    json!({"id": "INTO", "from": "review", "result": "odd", "next": "loop", "reason": "r"})
}

#[test]
fn an_unreachable_phase_is_refused() {
    assert_eq!(
        refusal(&looped(json!([]))),
        PolicyError::Refused(Refusal::Liveness(Finding::Unreachable {
            phase: "loop".into()
        }))
    );
    Machine::from_table(&looped(json!([into_loop()]))).unwrap();
}

#[test]
fn a_dead_end_is_refused() {
    let mut spin = looped(json!([into_loop()]));
    spin["rules"][3] = json!({"id": "SPIN", "from": "loop", "result": "again",
                              "next": "loop", "reason": "r"});
    assert_eq!(
        refusal(&spin),
        PolicyError::Refused(Refusal::Liveness(Finding::DeadEnd {
            phase: "loop".into()
        }))
    );
    spin["rules"].as_array_mut().unwrap().push(
        json!({"id": "STUCK", "from": "loop", "result": "stuck", "park": true, "reason": "r"}),
    );
    Machine::from_table(&spin).unwrap();
}

/// Presence refuses a v2 table only: the v1 tables (the frozen heritage
/// table, `bundles/verify`, `recipes/preflight`) load as they did.
#[test]
fn a_v2_table_with_an_unread_hard_input_is_refused_and_the_same_v1_table_loads() {
    let arms = || {
        json!([
            residual(
                "HOLD",
                json!({"has_security_residual": true}),
                "stop",
                "hard"
            ),
            residual("SHIP", json!({}), "done", "normal"),
        ])
    };
    assert_eq!(
        refusal(&delivery(TABLE_SCHEMA_V2, arms())),
        PolicyError::Refused(Refusal::Presence(Finding::Unread {
            rule: "SHIP".into(),
            inputs: vec!["has_security_residual".into()],
        }))
    );
    Machine::from_table(&delivery(TABLE_SCHEMA_V1, arms())).unwrap();
    // An engine-owned input is always supplied, so presence exempts it.
    for owned in [
        json!({"consecutive_failures_gte": 2}),
        json!({"visits_review_gte": 3}),
    ] {
        let arms = json!([
            residual("HOLD", owned, "stop", "hard"),
            residual("SHIP", json!({}), "done", "normal"),
        ]);
        Machine::from_table(&delivery(TABLE_SCHEMA_V2, arms)).unwrap();
    }
}
