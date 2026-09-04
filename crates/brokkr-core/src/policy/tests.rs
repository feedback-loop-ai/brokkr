use super::*;
use serde_json::json;

fn table(rule: Value) -> Value {
    json!({
        "phases": ["work", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [rule],
    })
}

fn rule() -> Value {
    json!({
        "id": "WORK-DONE",
        "from": "work",
        "result": "complete",
        "next": "done",
        "reason": "complete",
    })
}

#[test]
fn loader_refuses_unreachable_phase_and_required_field_defects() {
    assert!(Machine::from_table(&Value::Null)
        .unwrap_err()
        .0
        .contains("table must be an object"));

    let mut value = table(rule());
    value["initial"] = json!(2);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("initial must be a string"));

    let mut value = table(rule());
    value["rules"] = json!({});
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("rules must be an array"));

    let mut value = table(rule());
    value["phases"] = json!(["work", 2]);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("entries must be strings"));

    let mut value = table(rule());
    value["rules"] = json!([2]);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("rule must be an object"));

    let mut value = table(rule());
    value["initial"] = json!("elsewhere");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("initial phase"));

    let mut value = table(rule());
    value["terminal"] = json!(["elsewhere"]);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("terminal phase"));

    let mut value = table(rule());
    value["rules"][0].as_object_mut().unwrap().remove("reason");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("missing 'reason'"));

    let mut value = table(rule());
    value["rules"][0]["next"] = json!("elsewhere");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("unknown phase"));

    let mut value = table(rule());
    value["rules"][0]["from"] = json!("elsewhere");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("unknown phase"));

    let mut value = table(rule());
    value["rules"][0]["severity"] = json!(2);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("severity must be a string"));

    let mut value = table(rule());
    value["rules"][0]["when"] = json!(2);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("'when' must be an object"));
}

/// Decision 0022's phase-visit predicate. The condition vocabulary stays
/// closed; it just closes over the table's OWN graph — `visits_<phase>`
/// must name a phase this table declares, and the only comparison is the
/// `_gte` every counter already speaks, so a bound reads "while the
/// count has not reached N".
#[test]
fn the_phase_visit_predicate_is_closed_over_the_tables_own_phases() {
    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"visits_nowhere_gte": 2});
    let error = Machine::from_table(&value).unwrap_err().0;
    assert!(
        error.contains("unknown counter 'visits_nowhere'"),
        "{error}"
    );
    assert!(error.contains("visits_<phase>"), "{error}");

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"visits_work_gte": "twice"});
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("needs a numeric threshold"));

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"visits_work_gte": 3});
    let machine = Machine::from_table(&value).unwrap();
    let visited = |count: Value| json!({"visits_work": count}).as_object().unwrap().clone();
    let ruled = |inputs: &Map<String, Value>| {
        matches!(
            machine.evaluate("work", "complete", inputs),
            Outcome::Ruling { .. }
        )
    };
    // Absent is never an advantage; below the bound and at it are the
    // two sides the reforging arithmetic turns on.
    assert!(!ruled(&Map::new()));
    assert!(!ruled(&visited(json!(2))));
    assert!(ruled(&visited(json!(3))));
    assert!(ruled(&visited(json!(4))));
    // A visit count is a number. Anything else parks rather than coerces.
    assert!(matches!(
        machine.evaluate("work", "complete", &visited(json!("3"))),
        Outcome::NoRule { problem: Some(_) }
    ));

    // The engine supplies exactly the visit facts a phase's rules ask
    // for — a counter that is not a visit count, a condition that is not
    // a counter, and the same phase named twice all read as one answer.
    let many = json!({
        "phases": ["work", "check", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [
            {"id": "A", "from": "work", "result": "a", "next": "done",
             "when": {"consecutive_failures_gte": 2}, "reason": "a counter, not a visit"},
            {"id": "B", "from": "work", "result": "b", "next": "done",
             "when": {"fixes_applied": true}, "reason": "not a counter at all"},
            {"id": "C", "from": "work", "result": "c", "next": "done",
             "when": {"visits_check_gte": 2}, "reason": "the predicate"},
            {"id": "D", "from": "work", "result": "d", "next": "done",
             "when": {"visits_check_gte": 3}, "reason": "the same phase, twice"},
        ],
    });
    let many = Machine::from_table(&many).unwrap();
    assert_eq!(many.visit_phases("work"), vec!["check".to_string()]);
    assert!(many.visit_phases("done").is_empty());
}

/// Decision 0041 ruling 6's enumerated condition: every class has a
/// matching arm, while absence, another class, malformed runtime data,
/// and malformed table vocabulary all fail closed.
#[test]
fn strategy_in_has_one_table_arm_for_every_triage_class() {
    for strategy in STRATEGIES {
        let mut value = table(rule());
        value["rules"][0]["when"] = json!({"strategy_in": [strategy]});
        let machine = Machine::from_table(&value).unwrap();

        let matching = json!({"strategy": strategy}).as_object().unwrap().clone();
        assert!(matches!(
            machine.evaluate("work", "complete", &matching),
            Outcome::Ruling { .. }
        ));
        let other = STRATEGIES
            .iter()
            .copied()
            .find(|candidate| *candidate != strategy)
            .unwrap();
        let nonmatching = json!({"strategy": other}).as_object().unwrap().clone();
        assert_eq!(
            machine.evaluate("work", "complete", &nonmatching),
            Outcome::NoRule { problem: None }
        );
        assert_eq!(
            machine.evaluate("work", "complete", &Map::new()),
            Outcome::NoRule { problem: None }
        );
    }

    for invalid in [json!([]), json!(["unknown"]), json!("feature")] {
        let mut value = table(rule());
        value["rules"][0]["when"] = json!({"strategy_in": invalid});
        assert!(Machine::from_table(&value).is_err(), "accepted {invalid}");
    }

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"strategy_in": ["feature"]});
    let machine = Machine::from_table(&value).unwrap();
    for malformed in [json!(7), json!("unknown")] {
        let inputs = json!({"strategy": malformed}).as_object().unwrap().clone();
        assert!(matches!(
            machine.evaluate("work", "complete", &inputs),
            Outcome::NoRule { problem: Some(_) }
        ));
    }
}

/// Decision 0022's rule-driven park. A park is not a stop, so a table
/// that contains one has to say which vocabulary it is written in.
#[test]
fn a_rule_may_rule_a_park_and_only_a_v2_table_may_hold_one() {
    let park = || {
        json!({
            "id": "WORK-PARK",
            "from": "work",
            "result": "complete",
            "park": true,
            "reason": "this one is the operator's",
        })
    };
    let v2 = |rule: Value| {
        let mut value = table(rule);
        value["schema"] = json!(TABLE_SCHEMA_V2);
        value
    };

    // The version string is load-bearing, not decoration.
    let error = Machine::from_table(&table(park())).unwrap_err().0;
    assert!(error.contains("no schema"), "{error}");
    let mut v1 = table(park());
    v1["schema"] = json!(TABLE_SCHEMA_V1);
    let error = Machine::from_table(&v1).unwrap_err().0;
    assert!(error.contains(TABLE_SCHEMA_V1), "{error}");
    assert!(error.contains(TABLE_SCHEMA_V2), "{error}");

    // Advance or park, never both; and a park is a ruling, not a switch.
    let mut both = park();
    both["next"] = json!("done");
    assert!(Machine::from_table(&v2(both))
        .unwrap_err()
        .0
        .contains("both parks and names a next phase"));
    let mut off = park();
    off["park"] = json!(false);
    assert!(Machine::from_table(&v2(off))
        .unwrap_err()
        .0
        .contains("'park' must be true when present"));

    // A park takes no transition, so it has neither a ruling severity
    // nor an artifact gate on one.
    for forbidden in ["severity", "requires_artifacts"] {
        let mut rule = park();
        rule[forbidden] = json!("hard");
        let error = Machine::from_table(&v2(rule)).unwrap_err().0;
        assert!(
            error.contains(&format!("parks and declares '{forbidden}'")),
            "{error}"
        );
    }

    let machine = Machine::from_table(&v2(park())).unwrap();
    assert_eq!(
        machine.evaluate("work", "complete", &Map::new()),
        Outcome::Park {
            rule_id: "WORK-PARK".into(),
            reason: "this one is the operator's".into(),
        }
    );
    assert!(machine.rules[0].next.is_none());
}

#[test]
fn every_runtime_condition_shape_is_strict() {
    let counter = Condition::CounterGte {
        name: "consecutive_failures".into(),
        threshold: 2.0,
    };
    let severity = Condition::SeverityAbove {
        name: "max_residual_severity".into(),
        threshold_rank: severity_rank("medium").unwrap(),
    };
    let flag = Condition::Flag {
        name: "fixes_applied".into(),
        expected: true,
    };

    assert_eq!(
        conditions_met(std::slice::from_ref(&counter), &Map::new()),
        Ok(false)
    );
    assert_eq!(
        conditions_met(
            std::slice::from_ref(&counter),
            &json!({"consecutive_failures": 1})
                .as_object()
                .unwrap()
                .clone()
        ),
        Ok(false)
    );
    assert!(conditions_met(
        std::slice::from_ref(&counter),
        &json!({"consecutive_failures": "two"})
            .as_object()
            .unwrap()
            .clone()
    )
    .is_err());

    assert_eq!(
        conditions_met(
            std::slice::from_ref(&severity),
            &json!({"max_residual_severity": "low"})
                .as_object()
                .unwrap()
                .clone()
        ),
        Ok(false)
    );
    assert!(conditions_met(
        std::slice::from_ref(&severity),
        &json!({"max_residual_severity": "unknown"})
            .as_object()
            .unwrap()
            .clone()
    )
    .is_err());
    assert!(conditions_met(
        std::slice::from_ref(&severity),
        &json!({"max_residual_severity": 3})
            .as_object()
            .unwrap()
            .clone()
    )
    .is_err());

    assert_eq!(
        conditions_met(std::slice::from_ref(&flag), &Map::new()),
        Ok(false)
    );
    assert_eq!(
        conditions_met(
            std::slice::from_ref(&flag),
            &json!({"fixes_applied": false}).as_object().unwrap().clone()
        ),
        Ok(false)
    );
    assert!(conditions_met(
        std::slice::from_ref(&flag),
        &json!({"fixes_applied": "yes"}).as_object().unwrap().clone()
    )
    .is_err());
}

fn shipped_machine(relative: &str) -> Machine {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let table: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    Machine::from_table(&table).unwrap()
}

fn ruling(machine: &Machine, phase: &str, result: &str, inputs: Value) -> (String, String) {
    match machine.evaluate(phase, result, inputs.as_object().unwrap()) {
        Outcome::Ruling {
            rule_id,
            next_phase,
            ..
        } => (rule_id, next_phase),
        other => panic!("expected ruling for ({phase}, {result}), got {other:?}"),
    }
}

/// Decision 0041 ruling 5, point-blank against shipped tables: every
/// return and every exhaustion arm is independently earned.
#[test]
fn every_finding_edge_and_bound_has_a_table_arm() {
    let machine = shipped_machine("../../bundles/self/policy.json");

    assert_eq!(
        ruling(&machine, "verify", "fail", json!({"visits_implement": 2})),
        ("VERIFY-FAIL".into(), "implement".into())
    );
    assert_eq!(
        ruling(&machine, "verify", "fail", json!({"visits_implement": 3})),
        ("VERIFY-FAIL-EXHAUSTED".into(), "stop".into())
    );
    for severity in ["medium", "high", "critical"] {
        assert_eq!(
            ruling(
                &machine,
                "review",
                "residual",
                json!({"visits_implement": 2, "max_residual_severity": severity})
            )
            .1,
            "implement",
            "{severity} must return before exhaustion"
        );
    }
    for severity in ["info", "low"] {
        assert_eq!(
            ruling(
                &machine,
                "review",
                "residual",
                json!({"visits_implement": 2, "max_residual_severity": severity,
                       "has_security_residual": true})
            )
            .1,
            "ship",
            "{severity} is named debt without a return"
        );
    }
    assert_eq!(
        ruling(
            &machine,
            "review",
            "residual",
            json!({"visits_implement": 3, "max_residual_severity": "high"})
        )
        .1,
        "stop"
    );
    assert!(matches!(
        machine.evaluate(
            "review",
            "residual",
            json!({"visits_implement": 3, "max_residual_severity": "medium"})
                .as_object()
                .unwrap()
        ),
        Outcome::Park { ref rule_id, .. } if rule_id == "REVIEW-REFORGE-EXHAUSTED-MEDIUM"
    ));
    assert_eq!(
        ruling(
            &machine,
            "implement",
            "complete",
            json!({"fixes_docs_only": true})
        ),
        ("IMPL-OK-DOCS-RETURN".into(), "review".into())
    );
    assert_eq!(
        ruling(&machine, "implement", "complete", json!({})),
        ("IMPL-OK".into(), "verify".into()),
        "a verify-fail return cannot take the docs shortcut without its input"
    );
    assert_eq!(ruling(&machine, "review", "clean", json!({})).1, "ship");
    assert_eq!(
        ruling(&machine, "review", "security-hold", json!({})).1,
        "stop"
    );

    let sdd = shipped_machine("../../recipes/sdd/policy.json");
    assert_eq!(
        ruling(
            &sdd,
            "review",
            "residual",
            json!({"spec_defect": true, "visits_design": 2,
                   "visits_implement": 3, "max_residual_severity": "critical"})
        )
        .1,
        "design"
    );
    assert!(matches!(
        sdd.evaluate(
            "review",
            "residual",
            json!({"spec_defect": true, "visits_design": 3})
                .as_object()
                .unwrap()
        ),
        Outcome::Park { ref rule_id, .. } if rule_id == "REVIEW-SPEC-DEFECT-EXHAUSTED"
    ));
    assert_eq!(
        ruling(
            &sdd,
            "review",
            "clean",
            json!({"spec_defect": true, "visits_design": 2})
        ),
        ("REVIEW-CLEAN-SPEC-DEFECT".into(), "design".into())
    );
    assert!(matches!(
        sdd.evaluate(
            "review",
            "clean",
            json!({"spec_defect": true, "visits_design": 3})
                .as_object()
                .unwrap()
        ),
        Outcome::Park { ref rule_id, .. }
            if rule_id == "REVIEW-CLEAN-SPEC-DEFECT-EXHAUSTED"
    ));
}

/// Every arm of the at-most predicate, point-blank against a synthetic
/// table so no earlier rule intercepts: the loader's two refusals
/// (unknown axis, unranked threshold), and the evaluator's four
/// verdicts (within, above, unranked token, non-string value).
#[test]
fn the_at_most_predicate_is_strict_in_every_arm() {
    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"something_at_most": "low"});
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("unknown severity axis 'something'"));

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"max_residual_severity_at_most": "sideways"});
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("not in"));

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"max_residual_severity_at_most": "low"});
    let machine = Machine::from_table(&value).unwrap();
    let with = |severity: Value| {
        json!({"max_residual_severity": severity})
            .as_object()
            .unwrap()
            .clone()
    };
    assert!(matches!(
        machine.evaluate("work", "complete", &with(json!("info"))),
        Outcome::Ruling { .. }
    ));
    assert!(matches!(
        machine.evaluate("work", "complete", &with(json!("high"))),
        Outcome::NoRule { problem: None }
    ));
    assert!(matches!(
        machine.evaluate("work", "complete", &with(json!("sideways"))),
        Outcome::NoRule { problem: Some(_) }
    ));
    assert!(matches!(
        machine.evaluate("work", "complete", &with(json!(7))),
        Outcome::NoRule { problem: Some(_) }
    ));
    // An explicit null is the same silence as an absent key.
    assert!(matches!(
        machine.evaluate("work", "complete", &with(Value::Null)),
        Outcome::NoRule { problem: None }
    ));

    // And the park flag is a ruling, not a switch: false is refused.
    let mut value = table(rule());
    value["schema"] = json!(TABLE_SCHEMA_V2);
    value["rules"][0]["park"] = json!(false);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("'park' must be true when present"));
}
