use super::*;
use serde_json::json;

impl PolicyError {
    /// The parse refusal: every table these tests refuse is malformed,
    /// not refused for a decision 0050 finding.
    fn malformed(self) -> Malformed {
        match self {
            PolicyError::Malformed(fault) => fault,
            PolicyError::Refused(refusal) => panic!("refused, not malformed: {refusal}"),
        }
    }
}

/// The table `table` refuses to load as, malformed.
fn fault(table: &Value) -> Malformed {
    Machine::from_table(table).unwrap_err().malformed()
}

/// The refusal `machine` rules `(phase, complete)` with under `inputs`.
fn refused(machine: &Machine, phase: &str, inputs: &Map<String, Value>) -> Unreadable {
    match machine.evaluate(phase, "complete", inputs) {
        Outcome::Refused { problem } => problem,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

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

/// The typed severity is the table's vocabulary: each variant at its
/// rank, named and serialized by the one word the table writes.
#[test]
fn a_severity_is_named_ranked_and_serialized_as_the_table_names_it() {
    for (rank, severity) in Severity::ALL.into_iter().enumerate() {
        assert_eq!(severity.name(), SEVERITY_ORDER[rank]);
        assert_eq!(Severity::named(SEVERITY_ORDER[rank]), Some(severity));
        assert_eq!(
            serde_json::to_value(severity).unwrap(),
            json!(SEVERITY_ORDER[rank])
        );
    }
    assert!(Severity::Low < Severity::Medium && Severity::High < Severity::Critical);
    assert_eq!(Severity::named("severe"), None);
    assert_eq!(Severity::named("High"), None);
}

#[test]
fn change_identifiers_are_typed_data_and_never_condition_keys() {
    for accepted in ["a", "0", "change-42", "a.b_c-9"] {
        assert!(is_identifier(accepted), "{accepted}");
    }
    for refused in ["", "-a", "A", "a/b", "a b", "é"] {
        assert!(!is_identifier(refused), "{refused}");
    }
    for (condition, key) in [
        (json!({"change": "x"}), "change"),
        (json!({"change_in": ["x"]}), "change_in"),
    ] {
        let mut value = table(rule());
        value["rules"][0]["when"] = condition;
        assert_eq!(
            fault(&value),
            Malformed::IdentifierCondition {
                rule: "WORK-DONE".into(),
                key: key.into(),
            }
        );
    }
}

#[test]
fn loader_refuses_unreachable_phase_and_required_field_defects() {
    assert_eq!(fault(&Value::Null), Malformed::NotAnObject);

    let mut value = table(rule());
    value["initial"] = json!(2);
    assert_eq!(fault(&value), Malformed::InitialNotAString);

    let mut value = table(rule());
    value["rules"] = json!({});
    assert_eq!(
        fault(&value),
        Malformed::NotAnArray(Place::Table(TableKey::Rules))
    );

    let mut value = table(rule());
    value["phases"] = json!(["work", 2]);
    assert_eq!(
        fault(&value),
        Malformed::NotStrings(Place::Table(TableKey::Phases))
    );

    let mut value = table(rule());
    value["rules"] = json!([2]);
    assert_eq!(fault(&value), Malformed::RuleNotAnObject);

    let mut value = table(rule());
    value["initial"] = json!("elsewhere");
    assert_eq!(fault(&value), Malformed::InitialUnknown);

    let mut value = table(rule());
    value["terminal"] = json!(["elsewhere"]);
    assert_eq!(
        fault(&value),
        Malformed::TerminalUnknown("elsewhere".into())
    );

    let mut value = table(rule());
    value["rules"][0].as_object_mut().unwrap().remove("reason");
    assert_eq!(
        fault(&value),
        Malformed::MissingField {
            rule: Some("WORK-DONE".into()),
            key: RuleKey::Reason,
        }
    );

    for end in ["next", "from"] {
        let mut value = table(rule());
        value["rules"][0][end] = json!("elsewhere");
        assert_eq!(fault(&value), Malformed::UnknownPhase("WORK-DONE".into()));
    }

    let mut value = table(rule());
    value["rules"][0]["severity"] = json!(2);
    assert_eq!(
        fault(&value),
        Malformed::SeverityNotAString("WORK-DONE".into())
    );

    let mut value = table(rule());
    value["rules"][0]["when"] = json!(2);
    assert_eq!(
        fault(&value),
        Malformed::WhenNotAnObject("WORK-DONE".into())
    );
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
    assert_eq!(
        fault(&value),
        Malformed::UnknownCounter {
            rule: "WORK-DONE".into(),
            name: "visits_nowhere".into(),
            key: "visits_nowhere_gte".into(),
            phases: vec!["work".into(), "done".into()],
        }
    );

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"visits_work_gte": "twice"});
    assert_eq!(
        fault(&value),
        Malformed::NotNumeric {
            rule: "WORK-DONE".into(),
            key: "visits_work_gte".into(),
            written: r#""twice""#.into(),
        }
    );

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
    assert_eq!(
        refused(&machine, "work", &visited(json!("3"))),
        Unreadable::NotANumber {
            name: "visits_work".into(),
            written: r#""3""#.into(),
        }
    );

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
    // `check` is never entered: this table is read for its visits alone.
    let many = Machine::parse(&many).unwrap();
    assert_eq!(many.visit_phases("work"), vec!["check".to_string()]);
    assert!(many.visit_phases("done").is_empty());
}

#[test]
fn counter_introspection_is_scoped_to_the_rules_phase() {
    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"consecutive_failures_gte": 2});
    let machine = Machine::from_table(&value).unwrap();
    assert!(machine.reads_counter("work", "consecutive_failures"));
    assert!(!machine.reads_counter("review", "consecutive_failures"));
    assert!(!machine.reads_counter("work", "another_counter"));
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
            Outcome::Unmatched
        );
        assert_eq!(
            machine.evaluate("work", "complete", &Map::new()),
            Outcome::Unmatched
        );
    }

    enum_condition_refusals("strategy_in", "feature", "unknown", &STRATEGIES);
    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"strategy_in": ["feature"]});
    let machine = Machine::from_table(&value).unwrap();
    enum_input_refusals(&machine, "strategy", "unknown", &STRATEGIES);
}

/// The three ways a table writes an enumeration condition it may not:
/// no value, a value outside `vocabulary`, and a bare word for a list.
fn enum_condition_refusals(key: &str, word: &str, outside: &str, vocabulary: &'static [&str]) {
    let (id, key) = ("WORK-DONE".to_string(), key.to_string());
    for (written, expected) in [
        (
            json!([]),
            Malformed::NoValues {
                rule: id.clone(),
                key: key.clone(),
            },
        ),
        (
            json!([outside]),
            Malformed::OutsideVocabulary {
                rule: id.clone(),
                key: key.clone(),
                value: outside.into(),
                vocabulary,
            },
        ),
        (
            json!(word),
            Malformed::NotAnArray(Place::Condition {
                rule: id.clone(),
                key: key.clone(),
            }),
        ),
    ] {
        let mut value = table(rule());
        value["rules"][0]["when"] = json!({ key.clone(): written });
        assert_eq!(fault(&value), expected, "accepted {written}");
    }
}

/// The two ways an enumeration input the table reads cannot be read: a
/// value that is not a word, and a word outside `vocabulary`.
fn enum_input_refusals(machine: &Machine, name: &str, outside: &str, vocabulary: &'static [&str]) {
    for (given, expected) in [
        (
            json!(7),
            Unreadable::NotAString {
                name: name.into(),
                written: "7".into(),
            },
        ),
        (
            json!(outside),
            Unreadable::OutsideVocabulary {
                name: name.into(),
                word: outside.into(),
                vocabulary,
            },
        ),
    ] {
        let inputs = json!({ name: given }).as_object().unwrap().clone();
        assert_eq!(refused(machine, "work", &inputs), expected);
    }
}

/// Decision 0042's analysis return is data, not a match arm in the engine:
/// each artifact phase is selected by the same closed enum condition.
#[test]
fn drift_in_has_one_table_arm_for_every_artifact_phase() {
    for phase in DRIFT_PHASES {
        let mut value = table(rule());
        value["rules"][0]["when"] = json!({"drift_in": [phase]});
        let machine = Machine::from_table(&value).unwrap();
        let matching = json!({"drift_in": phase}).as_object().unwrap().clone();
        assert!(matches!(
            machine.evaluate("work", "complete", &matching),
            Outcome::Ruling { .. }
        ));
        let other = DRIFT_PHASES
            .iter()
            .copied()
            .find(|candidate| *candidate != phase)
            .unwrap();
        let nonmatching = json!({"drift_in": other}).as_object().unwrap().clone();
        assert_eq!(
            machine.evaluate("work", "complete", &nonmatching),
            Outcome::Unmatched
        );
    }

    enum_condition_refusals("drift_in", "design", "implement", &DRIFT_PHASES);
    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"drift_in": ["design"]});
    let machine = Machine::from_table(&value).unwrap();
    enum_input_refusals(&machine, "drift_in", "implement", &DRIFT_PHASES);
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
    let outside = |schema: Option<&str>| Malformed::ParkOutsideV2 {
        rule: "WORK-PARK".into(),
        schema: schema.map(str::to_string),
    };
    assert_eq!(fault(&table(park())), outside(None));
    let mut v1 = table(park());
    v1["schema"] = json!(TABLE_SCHEMA_V1);
    assert_eq!(fault(&v1), outside(Some(TABLE_SCHEMA_V1)));

    // Advance or park, never both; and a park is a ruling, not a switch.
    let mut both = park();
    both["next"] = json!("done");
    assert_eq!(fault(&v2(both)), Malformed::ParkAndNext("WORK-PARK".into()));
    let mut off = park();
    off["park"] = json!(false);
    assert_eq!(
        fault(&v2(off)),
        Malformed::ParkNotTrue {
            rule: "WORK-PARK".into(),
            written: "false".into(),
        }
    );

    // A park takes no transition, so it has neither a ruling severity
    // nor an artifact gate on one.
    for (written, key) in [
        ("severity", RuleKey::Severity),
        ("requires_artifacts", RuleKey::RequiresArtifacts),
    ] {
        let mut rule = park();
        rule[written] = json!("hard");
        assert_eq!(
            fault(&v2(rule)),
            Malformed::ParkDeclares {
                rule: "WORK-PARK".into(),
                key,
            }
        );
    }

    // Another result reaches `done`, so the table is live (decision 0050).
    let mut parking = v2(park());
    let mut done = rule();
    done["result"] = json!("shipped");
    parking["rules"].as_array_mut().unwrap().push(done);
    let machine = Machine::from_table(&parking).unwrap();
    assert_eq!(
        machine.evaluate("work", "complete", &Map::new()),
        Outcome::Park {
            rule_id: "WORK-PARK".into(),
            reason: "this one is the operator's".into(),
        }
    );
    assert!(machine.rules[0].next.is_none());
}

/// Issue #369: a v2 rule key outside the contract's nine refuses to
/// load, so a misspelt artifact gate, condition or severity cannot vanish.
/// v1 stays open for the frozen table's annotation keys.
#[test]
fn a_v2_rule_refuses_a_key_outside_its_vocabulary_and_v1_stays_open() {
    for misspelt in ["requires_artifact", "whenn", "severty", "source"] {
        let mut misspelt_rule = rule();
        misspelt_rule[misspelt] = json!(["review"]);
        let mut v2 = table(misspelt_rule.clone());
        v2["schema"] = json!(TABLE_SCHEMA_V2);
        assert_eq!(
            fault(&v2),
            Malformed::UnknownRuleKey {
                rule: "WORK-DONE".into(),
                key: misspelt.into(),
            }
        );

        let mut v1 = table(misspelt_rule.clone());
        v1["schema"] = json!(TABLE_SCHEMA_V1);
        for open in [v1, table(misspelt_rule)] {
            let machine = Machine::from_table(&open).unwrap();
            assert!(machine.rules[0].requires_artifacts.is_empty());
        }
    }

    let mut full = rule();
    full["severity"] = json!("flagged");
    full["requires_artifacts"] = json!(["review"]);
    full["when"] = json!({"fixes_applied": true});
    let mut v2 = table(full);
    v2["schema"] = json!(TABLE_SCHEMA_V2);
    let machine = Machine::from_table(&v2).unwrap();
    assert_eq!(machine.rules[0].requires_artifacts, vec!["review"]);
    assert_eq!(machine.rules[0].severity, "flagged");
    assert_eq!(machine.rules[0].when.len(), 1);
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
    assert_eq!(
        conditions_met(
            std::slice::from_ref(&counter),
            &json!({"consecutive_failures": "two"})
                .as_object()
                .unwrap()
                .clone()
        ),
        Err(Unreadable::NotANumber {
            name: "consecutive_failures".into(),
            written: r#""two""#.into(),
        })
    );

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
    for (given, refusal) in [
        (
            json!("unknown"),
            Unreadable::UnrankedSeverity {
                name: "max_residual_severity".into(),
                word: "unknown".into(),
            },
        ),
        (
            json!(3),
            Unreadable::SeverityNotAWord {
                name: "max_residual_severity".into(),
                written: "3".into(),
            },
        ),
    ] {
        let inputs = json!({"max_residual_severity": given});
        assert_eq!(
            conditions_met(std::slice::from_ref(&severity), inputs.as_object().unwrap()),
            Err(refusal)
        );
    }

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
    assert_eq!(
        conditions_met(
            std::slice::from_ref(&flag),
            &json!({"fixes_applied": "yes"}).as_object().unwrap().clone()
        ),
        Err(Unreadable::NotABoolean {
            name: "fixes_applied".into(),
            written: r#""yes""#.into(),
        })
    );
}

/// Each closed enum refuses a value outside it by naming its own
/// vocabulary, never the other one's (#419).
#[test]
fn an_enum_refusal_names_its_own_vocabulary() {
    for (name, actual, expected) in [
        (
            "strategy",
            "bogus",
            r#"strategy 'bogus' not in ["chore", "feature", "design", "engine", "escalate"]"#,
        ),
        (
            "drift_in",
            "implement",
            r#"drift_in 'implement' not in ["specify", "design", "tasks"]"#,
        ),
    ] {
        let condition = Condition::EnumIn {
            name: name.into(),
            allowed: Vec::new(),
        };
        let inputs = json!({ name: actual }).as_object().unwrap().clone();
        assert_eq!(
            conditions_met(std::slice::from_ref(&condition), &inputs).map_err(|e| e.to_string()),
            Err(expected.to_string())
        );
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "the test reads a shipped policy table"
)]
pub(super) fn shipped_table(relative: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn shipped_machine(relative: &str) -> Machine {
    Machine::from_table(&shipped_table(relative)).unwrap()
}

/// A recipe's own table, which is an overlay its composed recipe
/// completes: its arms are read before the composer makes it whole.
fn overlay_machine(relative: &str) -> Machine {
    Machine::parse(&shipped_table(relative)).unwrap()
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn every_finding_edge_and_bound_has_a_table_arm() {
    let machine = shipped_machine("../../recipes/fast/policy.json");

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

    let sdd = overlay_machine("../../recipes/triage/policy.json");
    assert_eq!(
        ruling(
            &sdd,
            "review",
            "residual",
            json!({"strategy": "design", "spec_defect": true, "visits_specify": 2,
                   "visits_implement": 3, "max_residual_severity": "critical"})
        )
        .1,
        "specify"
    );
    assert!(matches!(
        sdd.evaluate(
            "review",
            "residual",
            json!({"strategy": "design", "spec_defect": true, "visits_specify": 3})
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
            json!({"strategy": "design", "spec_defect": true, "visits_specify": 2})
        ),
        ("REVIEW-CLEAN-SPEC-DEFECT".into(), "specify".into())
    );
    assert!(matches!(
        sdd.evaluate(
            "review",
            "clean",
            json!({"strategy": "design", "spec_defect": true, "visits_specify": 3})
                .as_object()
                .unwrap()
        ),
        Outcome::Park { ref rule_id, .. }
            if rule_id == "REVIEW-CLEAN-SPEC-DEFECT-EXHAUSTED"
    ));
}

/// Decision 0042's shipped table is evaluated arm by arm. These are not
/// shape assertions: each pair proves first-match ordering on either side of
/// its literal bound, and each `drift_in` value drives the real table.
#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn the_shipped_sdd_table_rules_every_artifact_and_loop_arm() {
    let machine = overlay_machine("../../recipes/triage/policy.json");
    let park = |phase: &str, result: &str, inputs: Value| match machine.evaluate(
        phase,
        result,
        inputs.as_object().unwrap(),
    ) {
        Outcome::Park { rule_id, .. } => rule_id,
        other => panic!("expected park for ({phase}, {result}), got {other:?}"),
    };

    for phase in ["specify", "design", "tasks"] {
        let failures_after_other_returns = |count| {
            json!({
                "consecutive_failures": count,
                format!("visits_{phase}"): 9
            })
        };
        assert_eq!(
            ruling(&machine, phase, "fail", failures_after_other_returns(1)),
            (
                format!("{}-RETRY", phase.to_ascii_uppercase()),
                phase.into()
            )
        );
        assert_eq!(
            ruling(&machine, phase, "fail", failures_after_other_returns(2)),
            (
                format!("{}-FAIL-TWICE", phase.to_ascii_uppercase()),
                "stop".into()
            )
        );
    }

    assert_eq!(park("specify", "upstream", json!({})), "SPECIFY-UPSTREAM");
    assert_eq!(
        ruling(&machine, "specify", "drafted", json!({})),
        ("SPECIFY-DRAFTED".into(), "clarify".into())
    );
    assert_eq!(
        ruling(&machine, "design", "upstream", json!({"visits_specify": 2})),
        ("DESIGN-UPSTREAM".into(), "specify".into())
    );
    assert_eq!(
        park("design", "upstream", json!({"visits_specify": 3})),
        "DESIGN-UPSTREAM-EXHAUSTED"
    );
    assert_eq!(
        ruling(&machine, "design", "drafted", json!({})),
        ("DESIGN-DRAFTED".into(), "tasks".into())
    );
    assert_eq!(
        ruling(&machine, "tasks", "upstream", json!({"visits_design": 2})),
        ("TASKS-UPSTREAM".into(), "design".into())
    );
    assert_eq!(
        park("tasks", "upstream", json!({"visits_design": 3})),
        "TASKS-UPSTREAM-EXHAUSTED"
    );
    assert_eq!(
        ruling(&machine, "tasks", "drafted", json!({})),
        ("TASKS-DRAFTED".into(), "analyze".into())
    );

    assert_eq!(
        ruling(
            &machine,
            "clarify",
            "ambiguous",
            json!({"visits_clarify": 4})
        ),
        ("CLARIFY-AMBIGUOUS".into(), "specify".into())
    );
    assert_eq!(
        park("clarify", "ambiguous", json!({"visits_clarify": 5})),
        "CLARIFY-AMBIGUOUS-EXHAUSTED"
    );
    assert_eq!(
        ruling(&machine, "clarify", "clear", json!({})),
        ("CLARIFY-CLEAR".into(), "design".into())
    );

    for drift_in in ["specify", "design", "tasks"] {
        assert_eq!(
            ruling(
                &machine,
                "analyze",
                "drift",
                json!({"visits_analyze": 4, "drift_in": drift_in})
            ),
            (
                format!("ANALYZE-DRIFT-{}", drift_in.to_ascii_uppercase()),
                drift_in.into()
            )
        );
        assert_eq!(
            park(
                "analyze",
                "drift",
                json!({"visits_analyze": 5, "drift_in": drift_in})
            ),
            "ANALYZE-DRIFT-EXHAUSTED"
        );
    }
    assert_eq!(
        ruling(&machine, "analyze", "consistent", json!({})),
        ("ANALYZE-CONSISTENT".into(), "implement".into())
    );
}

/// Every arm of the at-most predicate, point-blank against a synthetic
/// table so no earlier rule intercepts: the loader's two refusals
/// (unknown axis, unranked threshold), and the evaluator's four
/// verdicts (within, above, unranked token, non-string value).
#[test]
fn the_at_most_predicate_is_strict_in_every_arm() {
    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"something_at_most": "low"});
    assert_eq!(
        fault(&value),
        Malformed::UnknownAxis {
            rule: "WORK-DONE".into(),
            name: "something".into(),
            key: "something_at_most".into(),
        }
    );

    let mut value = table(rule());
    value["rules"][0]["when"] = json!({"max_residual_severity_at_most": "sideways"});
    assert_eq!(
        fault(&value),
        Malformed::Unranked {
            rule: "WORK-DONE".into(),
            key: "max_residual_severity_at_most".into(),
            written: r#""sideways""#.into(),
        }
    );

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
    assert_eq!(
        machine.evaluate("work", "complete", &with(json!("high"))),
        Outcome::Unmatched
    );
    assert_eq!(
        refused(&machine, "work", &with(json!("sideways"))),
        Unreadable::UnrankedSeverity {
            name: "max_residual_severity".into(),
            word: "sideways".into(),
        }
    );
    assert_eq!(
        refused(&machine, "work", &with(json!(7))),
        Unreadable::SeverityNotAWord {
            name: "max_residual_severity".into(),
            written: "7".into(),
        }
    );
    // An explicit null is the same silence as an absent key.
    assert_eq!(
        machine.evaluate("work", "complete", &with(Value::Null)),
        Outcome::Unmatched
    );

    // And the park flag is a ruling, not a switch: false is refused.
    let mut value = table(rule());
    value["schema"] = json!(TABLE_SCHEMA_V2);
    value["rules"][0]["park"] = json!(false);
    assert_eq!(
        fault(&value),
        Malformed::ParkNotTrue {
            rule: "WORK-DONE".into(),
            written: "false".into(),
        }
    );
}

/// The one-rule table with its top-level `key` written as `written`, or
/// left out when `written` is `None`.
fn with_top(key: &str, written: Option<Value>) -> Value {
    let mut value = table(rule());
    match written {
        Some(written) => value[key] = written,
        None => drop(value.as_object_mut().unwrap().remove(key)),
    }
    value
}

/// The one-rule table with its rule's `key` written as `written`, or left
/// out when `written` is `None`.
fn with_rule(key: &str, written: Option<Value>) -> Value {
    let mut value = table(rule());
    let rule = value["rules"][0].as_object_mut().unwrap();
    match written {
        Some(written) => drop(rule.insert(key.into(), written)),
        None => drop(rule.remove(key)),
    }
    value
}

/// `value` under `schema`.
fn labelled(mut value: Value, schema: &str) -> Value {
    value["schema"] = json!(schema);
    value
}

/// The one-rule table whose rule parks instead of advancing, with its
/// rule's `key` written as `written`.
fn parked(key: &str, written: Value) -> Value {
    let mut value = with_rule("next", None);
    value["rules"][0]["park"] = json!(true);
    value["rules"][0][key] = written;
    value
}

/// One malformed table, the variant it earns, and that variant's bytes.
type Faulty = (Value, Malformed, &'static str);

fn work_done() -> String {
    "WORK-DONE".to_string()
}

/// The table's own shape: its header keys and its list of rules.
fn table_faults() -> Vec<Faulty> {
    let mut twice = table(rule());
    twice["rules"] = json!([rule(), rule()]);
    let mut shadowed = table(rule());
    shadowed["rules"] = json!([
        rule(),
        with_rule("id", Some(json!("WORK-AGAIN")))["rules"][0]
    ]);
    vec![
        (
            Value::Null,
            Malformed::NotAnObject,
            "table must be an object",
        ),
        (
            with_top("rules", None),
            Malformed::MissingKey(TableKey::Rules),
            "table missing 'rules'",
        ),
        (
            with_top("phases", Some(json!(2))),
            Malformed::NotAnArray(Place::Table(TableKey::Phases)),
            "phases must be an array",
        ),
        (
            with_top("terminal", Some(json!(["done", 2]))),
            Malformed::NotStrings(Place::Table(TableKey::Terminal)),
            "terminal entries must be strings",
        ),
        (
            with_top("initial", Some(json!(2))),
            Malformed::InitialNotAString,
            "initial must be a string",
        ),
        (
            with_top("initial", Some(json!("elsewhere"))),
            Malformed::InitialUnknown,
            "initial phase not in phases",
        ),
        (
            with_top("terminal", Some(json!(["elsewhere"]))),
            Malformed::TerminalUnknown("elsewhere".into()),
            "terminal phase 'elsewhere' not in phases",
        ),
        (
            with_top("shippable_from", Some(json!({}))),
            Malformed::NotAnArray(Place::Table(TableKey::ShippableFrom)),
            "shippable_from must be an array",
        ),
        (
            with_top("rules", Some(json!([2]))),
            Malformed::RuleNotAnObject,
            "rule must be an object",
        ),
        (
            twice,
            Malformed::DuplicateId(work_done()),
            "duplicate rule id WORK-DONE",
        ),
        (
            shadowed,
            Malformed::Unreachable {
                rule: "WORK-AGAIN".into(),
                from: "work".into(),
                result: "complete".into(),
            },
            "rule WORK-AGAIN is unreachable: an unconditional rule for (work, complete) \
             precedes it and first match wins",
        ),
    ]
}

/// A rule's required fields, its v2 vocabulary and its park.
fn rule_faults() -> Vec<Faulty> {
    vec![
        (
            with_rule("reason", None),
            Malformed::MissingField {
                rule: Some(work_done()),
                key: RuleKey::Reason,
            },
            "rule WORK-DONE missing 'reason'",
        ),
        (
            with_rule("id", None),
            Malformed::MissingField {
                rule: None,
                key: RuleKey::Id,
            },
            "rule ? missing 'id'",
        ),
        (
            labelled(with_rule("whenn", Some(json!(1))), TABLE_SCHEMA_V2),
            Malformed::UnknownRuleKey {
                rule: work_done(),
                key: "whenn".into(),
            },
            "rule WORK-DONE declares 'whenn', which is not forge.phase-machine/v2 rule \
             vocabulary",
        ),
        (
            labelled(with_rule("park", Some(json!(0))), TABLE_SCHEMA_V2),
            Malformed::ParkNotTrue {
                rule: work_done(),
                written: "0".into(),
            },
            "rule WORK-DONE: 'park' must be true when present, got 0; a park is a ruling, not \
             a switch to leave off",
        ),
        (
            labelled(with_rule("park", Some(json!(true))), TABLE_SCHEMA_V2),
            Malformed::ParkAndNext(work_done()),
            "rule WORK-DONE both parks and names a next phase; a parked run takes no transition",
        ),
        (
            labelled(parked("reason", json!("parked")), TABLE_SCHEMA_V1),
            Malformed::ParkOutsideV2 {
                rule: work_done(),
                schema: Some(TABLE_SCHEMA_V1.into()),
            },
            "rule WORK-DONE parks, which is forge.phase-machine/v2 vocabulary, but the table \
             declares forge.phase-machine/v1",
        ),
        (
            parked("reason", json!("parked")),
            Malformed::ParkOutsideV2 {
                rule: work_done(),
                schema: None,
            },
            "rule WORK-DONE parks, which is forge.phase-machine/v2 vocabulary, but the table \
             declares no schema",
        ),
        (
            labelled(parked("requires_artifacts", json!([])), TABLE_SCHEMA_V2),
            Malformed::ParkDeclares {
                rule: work_done(),
                key: RuleKey::RequiresArtifacts,
            },
            "rule WORK-DONE parks and declares 'requires_artifacts'; a park takes no \
             transition, so it has neither a ruling severity nor an artifact gate",
        ),
    ]
}

/// A rule's phases, severity, artifact gate and conditions map.
fn field_faults() -> Vec<Faulty> {
    vec![
        (
            with_rule("next", Some(json!("elsewhere"))),
            Malformed::UnknownPhase(work_done()),
            "rule WORK-DONE references unknown phase",
        ),
        (
            with_rule("from", Some(json!("done"))),
            Malformed::LeavesTerminal {
                rule: work_done(),
                phase: "done".into(),
            },
            "rule WORK-DONE leaves terminal phase 'done'",
        ),
        (
            with_rule("severity", Some(json!(2))),
            Malformed::SeverityNotAString(work_done()),
            "rule WORK-DONE severity must be a string",
        ),
        (
            with_rule("severity", Some(json!("loud"))),
            Malformed::UnknownSeverity {
                rule: work_done(),
                severity: "loud".into(),
            },
            r#"rule WORK-DONE severity 'loud' not in ["normal", "flagged", "hard"]"#,
        ),
        (
            with_rule("requires_artifacts", Some(json!("x"))),
            Malformed::NotAnArray(Place::Artifacts { rule: work_done() }),
            "rule WORK-DONE requires_artifacts must be an array",
        ),
        (
            with_rule("requires_artifacts", Some(json!([1]))),
            Malformed::NotStrings(Place::Artifacts { rule: work_done() }),
            "rule WORK-DONE requires_artifacts entries must be strings",
        ),
        (
            with_rule("when", Some(json!(2))),
            Malformed::WhenNotAnObject(work_done()),
            "rule WORK-DONE 'when' must be an object",
        ),
    ]
}

fn when(condition: Value) -> Value {
    with_rule("when", Some(condition))
}

/// The identifier and enumeration conditions.
fn enum_faults() -> Vec<Faulty> {
    vec![
        (
            when(json!({"change": "x"})),
            Malformed::IdentifierCondition {
                rule: work_done(),
                key: "change".into(),
            },
            "rule WORK-DONE: identifier input 'change' may be declared by a seat but never \
             used as a condition key",
        ),
        (
            when(json!({"strategy_in": ["feature", 2]})),
            Malformed::NotStrings(Place::Condition {
                rule: work_done(),
                key: "strategy_in".into(),
            }),
            "rule WORK-DONE condition 'strategy_in' entries must be strings",
        ),
        (
            when(json!({"drift_in": []})),
            Malformed::NoValues {
                rule: work_done(),
                key: "drift_in".into(),
            },
            "rule WORK-DONE: condition 'drift_in' needs at least one value",
        ),
        (
            when(json!({"strategy_in": ["unknown"]})),
            Malformed::OutsideVocabulary {
                rule: work_done(),
                key: "strategy_in".into(),
                value: "unknown".into(),
                vocabulary: &STRATEGIES,
            },
            r#"rule WORK-DONE: condition 'strategy_in' value 'unknown' not in ["chore", "feature", "design", "engine", "escalate"]"#,
        ),
    ]
}

/// The counter, severity and flag conditions, and a key in none of them.
fn threshold_faults() -> Vec<Faulty> {
    vec![
        (
            when(json!({"visits_nowhere_gte": 2})),
            Malformed::UnknownCounter {
                rule: work_done(),
                name: "visits_nowhere".into(),
                key: "visits_nowhere_gte".into(),
                phases: vec!["work".into(), "done".into()],
            },
            r#"rule WORK-DONE: unknown counter 'visits_nowhere' in condition 'visits_nowhere_gte'; known: ["consecutive_failures"] plus 'visits_<phase>' over this table's phases ["work", "done"]"#,
        ),
        (
            when(json!({"consecutive_failures_gte": "twice"})),
            Malformed::NotNumeric {
                rule: work_done(),
                key: "consecutive_failures_gte".into(),
                written: r#""twice""#.into(),
            },
            r#"rule WORK-DONE: condition 'consecutive_failures_gte' needs a numeric threshold, got "twice""#,
        ),
        (
            when(json!({"something_above": "low"})),
            Malformed::UnknownAxis {
                rule: work_done(),
                name: "something".into(),
                key: "something_above".into(),
            },
            r#"rule WORK-DONE: unknown severity axis 'something' in condition 'something_above'; known: ["max_residual_severity"]"#,
        ),
        (
            when(json!({"max_residual_severity_above": "sideways"})),
            Malformed::Unranked {
                rule: work_done(),
                key: "max_residual_severity_above".into(),
                written: r#""sideways""#.into(),
            },
            r#"rule WORK-DONE: condition 'max_residual_severity_above' threshold "sideways" not in ["none", "info", "low", "medium", "high", "critical"]"#,
        ),
        (
            when(json!({"skip_verify": "yes"})),
            Malformed::NotBoolean {
                rule: work_done(),
                key: "skip_verify".into(),
                written: r#""yes""#.into(),
            },
            r#"rule WORK-DONE: condition 'skip_verify' expects true/false, got "yes""#,
        ),
        (
            when(json!({"bogus": true})),
            Malformed::UnknownCondition {
                rule: work_done(),
                key: "bogus".into(),
            },
            r#"rule WORK-DONE: unknown condition key 'bogus'; known: ["skip_verify", "fixes_applied", "spec_defect", "has_security_residual", "high_risk_uncovered", "drift_detected", "dirty_worktrees", "fixes_docs_only"] plus strategy_in over ["chore", "feature", "design", "engine", "escalate"], *_gte over ["consecutive_failures"] and *_above/*_at_most over ["max_residual_severity"]"#,
        ),
    ]
}

/// Every way a table is malformed: the table that earns it, the variant
/// it earns, and the one place that variant's bytes are pinned — the text
/// the operator has always read (#353).
#[test]
fn every_malformed_table_reads_as_it_always_has() {
    let faults = [
        table_faults(),
        rule_faults(),
        field_faults(),
        enum_faults(),
        threshold_faults(),
    ];
    for (value, expected, text) in faults.into_iter().flatten() {
        let error = Machine::from_table(&value).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("malformed phase machine table: {text}")
        );
        assert_eq!(chain(&error), error.to_string());
        assert_eq!(error.malformed(), expected);
    }
}

/// An error and its sources as the CLI's `{:#}` failure line joins them:
/// the stderr bytes a refusal prints.
fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    std::iter::successors(Some(error), |link| link.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

/// Every refusal to rule: the input that earns it, the variant it earns,
/// and the one place that variant's bytes — the `problem` a decision
/// journals — are pinned (#353).
#[test]
fn every_refusal_to_rule_reads_as_it_always_has() {
    let machine = Machine::from_table(&with_rule(
        "when",
        Some(json!({"consecutive_failures_gte": 2, "fixes_applied": true,
                    "max_residual_severity_above": "low", "strategy_in": ["feature"]})),
    ))
    .unwrap();
    let given = |name: &str, written: Value| {
        let mut inputs = json!({"consecutive_failures": 3, "fixes_applied": true,
                                "max_residual_severity": "high", "strategy": "feature"});
        inputs[name] = written;
        inputs.as_object().unwrap().clone()
    };
    let order = r#"["none", "info", "low", "medium", "high", "critical"]"#;
    for (phase, inputs, expected, text) in [
        (
            "nowhere",
            given("strategy", json!("feature")),
            Unreadable::UnknownPhase("nowhere".into()),
            "unknown phase 'nowhere'".to_string(),
        ),
        (
            "work",
            given("consecutive_failures", json!("two")),
            Unreadable::NotANumber {
                name: "consecutive_failures".into(),
                written: r#""two""#.into(),
            },
            r#"consecutive_failures must be a number, got "two""#.to_string(),
        ),
        (
            "work",
            given("fixes_applied", json!("yes")),
            Unreadable::NotABoolean {
                name: "fixes_applied".into(),
                written: r#""yes""#.into(),
            },
            r#"fixes_applied must be a boolean, got "yes""#.to_string(),
        ),
        (
            "work",
            given("max_residual_severity", json!("unknown")),
            Unreadable::UnrankedSeverity {
                name: "max_residual_severity".into(),
                word: "unknown".into(),
            },
            format!("max_residual_severity severity 'unknown' not in {order}"),
        ),
        (
            "work",
            given("max_residual_severity", json!(3)),
            Unreadable::SeverityNotAWord {
                name: "max_residual_severity".into(),
                written: "3".into(),
            },
            format!("max_residual_severity severity 3 not in {order}"),
        ),
        (
            "work",
            given("strategy", json!(7)),
            Unreadable::NotAString {
                name: "strategy".into(),
                written: "7".into(),
            },
            "strategy must be a string, got 7".to_string(),
        ),
        (
            "work",
            given("strategy", json!("bogus")),
            Unreadable::OutsideVocabulary {
                name: "strategy".into(),
                word: "bogus".into(),
                vocabulary: &STRATEGIES,
            },
            r#"strategy 'bogus' not in ["chore", "feature", "design", "engine", "escalate"]"#
                .to_string(),
        ),
    ] {
        let problem = refused(&machine, phase, &inputs);
        assert_eq!(problem.to_string(), text);
        assert_eq!(problem, expected);
    }
    assert!(matches!(
        machine.evaluate("work", "complete", &given("strategy", json!("feature"))),
        Outcome::Ruling { .. }
    ));
}

/// The split #353 makes: a pair no rule matches is `Unmatched`, and a
/// present input the vocabulary cannot read is `Refused` with its problem
/// — never the same outcome, so the journal can tell them apart.
#[test]
fn an_unmatched_pair_and_an_unreadable_input_are_two_outcomes() {
    let machine =
        Machine::from_table(&with_rule("when", Some(json!({"fixes_applied": true})))).unwrap();
    let inputs = |written: Value| {
        json!({"fixes_applied": written})
            .as_object()
            .unwrap()
            .clone()
    };
    assert_eq!(
        machine.evaluate("work", "complete", &inputs(json!(false))),
        Outcome::Unmatched
    );
    assert_eq!(
        machine.evaluate("work", "complete", &inputs(json!("no"))),
        Outcome::Refused {
            problem: Unreadable::NotABoolean {
                name: "fixes_applied".into(),
                written: r#""no""#.into(),
            },
        }
    );
    assert_eq!(
        machine.evaluate("work", "shipped", &inputs(json!(true))),
        Outcome::Unmatched
    );
}
