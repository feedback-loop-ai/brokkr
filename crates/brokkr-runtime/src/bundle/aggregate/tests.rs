//! An aggregate is named by one word, and read back from it.

use super::*;

#[test]
fn every_aggregate_reads_back_from_its_word() {
    let words: Vec<&str> = Aggregate::ALL.map(Aggregate::word).to_vec();
    assert_eq!(words, ["unanimous-pass", "review-panel"]);
    for aggregate in Aggregate::ALL {
        assert_eq!(Aggregate::parse(aggregate.word()), Some(aggregate));
    }
    assert_eq!(Aggregate::parse("UnanimousPass"), None);
    assert_eq!(Aggregate::known(), "unanimous-pass, review-panel");
}

fn reviewed(members: Vec<Value>) -> Value {
    let members: Vec<_> = members
        .into_iter()
        .enumerate()
        .map(|(index, member)| (format!("member-{index}"), member))
        .collect();
    Aggregate::ReviewPanel.aggregate_results(&members)
}

fn residual(inputs: Value) -> Value {
    json!({"result":"residual", "inputs":inputs})
}

fn admission(review: &Value) -> (String, String) {
    use brokkr_core::policy::{Machine, Outcome};
    let machine = Machine::from_table(&json!({
        "schema":"forge.phase-machine/v1", "phases":["review","ship","stop"],
        "initial":"review", "terminal":["ship","stop"], "rules":[
            {"id":"HOLD", "from":"review", "result":"security-hold", "next":"stop", "severity":"hard", "reason":"hold"},
            {"id":"HIGH", "from":"review", "result":"residual", "next":"stop", "severity":"hard", "reason":"high", "when":{"max_residual_severity_above":"medium"}},
            {"id":"SAFE", "from":"review", "result":"residual", "next":"ship", "reason":"explicitly safe", "when":{"has_security_residual":false,"max_residual_severity_at_most":"medium"}},
            {"id":"UNSAFE", "from":"review", "result":"residual", "next":"stop", "severity":"hard", "reason":"unknown or unsafe"},
            {"id":"CLEAN", "from":"review", "result":"clean", "next":"ship", "reason":"clean"}
        ]
    })).unwrap();
    match machine.evaluate(
        "review",
        review["result"].as_str().unwrap(),
        review["inputs"].as_object().unwrap(),
    ) {
        Outcome::Ruling {
            rule_id,
            next_phase,
            ..
        } => (rule_id, next_phase),
        other => panic!("expected a named ruling, got {other:?}"),
    }
}

#[test]
fn residual_safety_facts_missing_null_or_wrong_typed_stay_unknown() {
    let malformed = [
        json!({"result":"residual"}),
        residual(Value::Null),
        residual(json!([])),
        residual(json!({})),
        residual(json!({"max_residual_severity":"low"})),
        residual(json!({"has_security_residual":null,"max_residual_severity":"low"})),
        residual(json!({"has_security_residual":"false","max_residual_severity":"low"})),
        residual(json!({"has_security_residual":0,"max_residual_severity":"low"})),
        residual(json!({"has_security_residual":[],"max_residual_severity":"low"})),
        residual(json!({"has_security_residual":{},"max_residual_severity":"low"})),
        residual(json!({"has_security_residual":false})),
        residual(json!({"has_security_residual":false,"max_residual_severity":null})),
        residual(json!({"has_security_residual":false,"max_residual_severity":"invented"})),
        residual(json!({"has_security_residual":false,"max_residual_severity":false})),
        residual(json!({"has_security_residual":false,"max_residual_severity":0})),
        residual(json!({"has_security_residual":false,"max_residual_severity":[]})),
        residual(json!({"has_security_residual":false,"max_residual_severity":{}})),
    ];
    for member in malformed {
        let review = reviewed(vec![json!({"result":"clean"}), member]);
        assert_eq!(
            admission(&review),
            ("UNSAFE".into(), "stop".into()),
            "{review}"
        );
        assert!(
            review["inputs"]["has_security_residual"].is_null()
                || review["inputs"]["max_residual_severity"].is_null(),
            "{review}"
        );
    }
}

#[test]
fn valid_residuals_keep_every_severity_and_security_fact() {
    for severity in ["none", "info", "low", "medium", "high", "critical"] {
        for security in [false, true] {
            let review = reviewed(vec![residual(json!({
                "has_security_residual":security, "max_residual_severity":severity
            }))]);
            assert_eq!(review["inputs"]["has_security_residual"], security);
            assert_eq!(review["inputs"]["max_residual_severity"], severity);
            let expected = if ["high", "critical"].contains(&severity) {
                ("HIGH", "stop")
            } else if security {
                ("UNSAFE", "stop")
            } else {
                ("SAFE", "ship")
            };
            assert_eq!(admission(&review), (expected.0.into(), expected.1.into()));
        }
    }
}

#[test]
fn a_safe_member_cannot_supply_a_malformed_residuals_missing_facts() {
    let safe = residual(json!({"has_security_residual":false,"max_residual_severity":"medium"}));
    for bad in [
        residual(json!({})),
        residual(json!({"has_security_residual":"false","max_residual_severity":"low"})),
    ] {
        for members in [vec![safe.clone(), bad.clone()], vec![bad, safe.clone()]] {
            let review = reviewed(members);
            assert_eq!(
                admission(&review),
                ("UNSAFE".into(), "stop".into()),
                "{review}"
            );
        }
    }
}

#[test]
fn known_high_and_security_findings_survive_unknown_member_facts() {
    for severity in ["high", "critical"] {
        let high =
            residual(json!({"has_security_residual":false,"max_residual_severity":severity}));
        for members in [
            vec![high.clone(), residual(json!({}))],
            vec![residual(json!({})), high],
        ] {
            let review = reviewed(members);
            assert_eq!(review["inputs"]["max_residual_severity"], severity);
            assert_eq!(admission(&review), ("HIGH".into(), "stop".into()));
        }
    }
    let security = residual(json!({"has_security_residual":true,"max_residual_severity":"low"}));
    for members in [
        vec![security.clone(), residual(json!({}))],
        vec![residual(json!({})), security],
    ] {
        let review = reviewed(members);
        assert_eq!(review["inputs"]["has_security_residual"], true);
        assert_eq!(review["inputs"]["max_residual_severity"], Value::Null);
        assert_eq!(admission(&review), ("UNSAFE".into(), "stop".into()));
    }
}

#[test]
fn security_observations_on_clean_members_still_stop_residuals() {
    let clean = json!({"result":"clean","inputs":{"has_security_residual":true}});
    for inputs in [
        json!({}),
        json!({"has_security_residual":false,"max_residual_severity":"low"}),
    ] {
        let member = residual(inputs);
        for members in [
            vec![clean.clone(), member.clone()],
            vec![member, clean.clone()],
        ] {
            let review = reviewed(members);
            assert_eq!(review["inputs"]["has_security_residual"], true);
            assert_eq!(admission(&review), ("UNSAFE".into(), "stop".into()));
        }
    }
}

#[test]
fn security_holds_dominate_unknowns_and_valid_clean_results_stay_clean() {
    for members in [
        vec![
            json!({"result":"clean"}),
            residual(json!({})),
            json!({"result":"security-hold"}),
        ],
        vec![
            json!({"result":"security-hold"}),
            residual(json!({})),
            json!({"result":"clean"}),
        ],
    ] {
        let review = reviewed(members);
        assert_eq!(review["result"], "security-hold");
        assert_eq!(admission(&review), ("HOLD".into(), "stop".into()));
    }
    let review = reviewed(vec![json!({"result":"clean"}), json!({"result":"clean"})]);
    assert_eq!(review["result"], "clean");
    assert_eq!(
        review["inputs"],
        json!({"fixes_applied":false,"spec_defect":false})
    );
    assert_eq!(admission(&review), ("CLEAN".into(), "ship".into()));
}

fn strict_scope_machine() -> brokkr_core::policy::Machine {
    brokkr_core::policy::Machine::from_table(&json!({
        "schema":"forge.phase-machine/v1",
        "phases":["review","verify","triage","ship","stop"],
        "initial":"review", "terminal":["ship","stop","verify","triage"], "rules":[
            {"id":"HOLD","from":"review","result":"security-hold","next":"stop","severity":"hard","reason":"hold"},
            {"id":"HIGH","from":"review","result":"residual","next":"stop","severity":"hard","reason":"high","when":{"max_residual_severity_above":"medium"}},
            {"id":"CLEAN-SCOPE","from":"review","result":"clean","next":"triage","reason":"scope","when":{"spec_defect":true,"fixes_applied":false}},
            {"id":"CLEAN-FIXES","from":"review","result":"clean","next":"verify","reason":"fixes","when":{"spec_defect":false,"fixes_applied":true}},
            {"id":"CLEAN-SAFE","from":"review","result":"clean","next":"ship","reason":"safe","when":{"spec_defect":false,"fixes_applied":false}},
            {"id":"CLEAN-UNSAFE","from":"review","result":"clean","next":"stop","severity":"hard","reason":"unknown"},
            {"id":"RESIDUAL-SCOPE","from":"review","result":"residual","next":"triage","reason":"scope","when":{"has_security_residual":false,"max_residual_severity_at_most":"medium","spec_defect":true,"fixes_applied":false}},
            {"id":"RESIDUAL-SAFE","from":"review","result":"residual","next":"ship","reason":"safe","when":{"has_security_residual":false,"max_residual_severity_at_most":"medium","spec_defect":false,"fixes_applied":false}},
            {"id":"RESIDUAL-UNSAFE","from":"review","result":"residual","next":"stop","severity":"hard","reason":"unknown"}
        ]
    })).unwrap()
}

fn strict_destination(machine: &brokkr_core::policy::Machine, review: &Value) -> String {
    use brokkr_core::policy::Outcome;
    if Aggregate::review_problem(review).is_some()
        || review["result"] == "__member-schema-invalid__"
    {
        return "refused".into();
    }
    match machine.evaluate(
        "review",
        review["result"].as_str().unwrap(),
        review["inputs"].as_object().unwrap(),
    ) {
        Outcome::Ruling { next_phase, .. } => next_phase,
        _ => "refused".into(),
    }
}

fn scope_member(result: &str, fixes: Option<Value>, scope: Option<Value>) -> Value {
    let mut inputs = json!({
        "has_security_residual":false,
        "max_residual_severity": if result == "residual" { "medium" } else { "none" }
    });
    if let Some(value) = fixes {
        inputs["fixes_applied"] = value;
    }
    if let Some(value) = scope {
        inputs["spec_defect"] = value;
    }
    json!({"result":result, "inputs":inputs})
}

/// The scope flags are witnessed or false (decision 0097 ruling 1): only a
/// member's readable `true` moves them, whatever shape another member
/// wrote, and the table's scope, fixes and safe routes follow.
#[test]
fn review_scope_and_fixes_are_witnessed_or_false_for_one_two_four_members() {
    let machine = strict_scope_machine();
    let values = [
        None,
        Some(Value::Null),
        Some(json!(false)),
        Some(json!(true)),
        Some(json!("false")),
        Some(json!(0)),
        Some(json!([])),
        Some(json!({})),
    ];
    for result in ["clean", "residual"] {
        for count in [1, 2, 4] {
            for position in 0..count {
                for (fixes, scope) in values
                    .iter()
                    .flat_map(|fixes| values.iter().map(move |scope| (fixes, scope)))
                {
                    let member = scope_member(result, fixes.clone(), scope.clone());
                    let mut members =
                        vec![scope_member("clean", Some(json!(false)), Some(json!(false))); count];
                    members[position] = member;
                    let review = reviewed(members);
                    let fixes = fixes.as_ref().and_then(Value::as_bool) == Some(true);
                    let scope = scope.as_ref().and_then(Value::as_bool) == Some(true);
                    assert_eq!(review["inputs"]["fixes_applied"], json!(fixes), "{review}");
                    assert_eq!(review["inputs"]["spec_defect"], json!(scope), "{review}");
                    let destination = strict_destination(&machine, &review);
                    let expected = match (result, fixes, scope) {
                        (_, false, false) => "ship",
                        (_, false, true) => "triage",
                        ("clean", true, false) => "verify",
                        _ => "stop",
                    };
                    assert_eq!(destination, expected, "count={count}: {review}");
                }
            }
        }
    }
}

#[test]
fn witnessed_true_flags_survive_unknown_members_in_every_order() {
    for flag in ["fixes_applied", "spec_defect"] {
        for result in ["clean", "residual"] {
            for count in [2, 4] {
                for position in 0..count {
                    let mut members = vec![scope_member("clean", None, None); count];
                    let mut witness = scope_member(result, None, None);
                    witness["inputs"][flag] = json!(true);
                    members[position] = witness;
                    let review = reviewed(members.clone());
                    assert_eq!(review["inputs"][flag], true, "{review}");
                    members.reverse();
                    assert_eq!(reviewed(members)["inputs"][flag], true);
                }
            }
        }
    }
}

/// A member with no readable inputs witnesses no fixes and no defect, so
/// a clean panel still takes the safe route; the same member's missing
/// safety facts keep a residual panel from it.
#[test]
fn absent_or_unreadable_inputs_witness_no_scope_but_leave_safety_unknown() {
    for result in ["clean", "residual"] {
        for payload in [
            json!({"result":result}),
            json!({"result":result,"inputs":null}),
            json!({"result":result,"inputs":[]}),
            json!({"result":result,"inputs":{}}),
        ] {
            for count in [1, 2, 4] {
                let mut members =
                    vec![scope_member("clean", Some(json!(false)), Some(json!(false))); count];
                members[0] = payload.clone();
                let review = reviewed(members);
                assert_eq!(review["inputs"]["fixes_applied"], false, "{review}");
                assert_eq!(review["inputs"]["spec_defect"], false, "{review}");
                let expected = if result == "clean" { "ship" } else { "stop" };
                assert_eq!(
                    strict_destination(&strict_scope_machine(), &review),
                    expected,
                    "{review}"
                );
            }
        }
    }
}

/// A clean verdict carries no residual (decision 0097 ruling 2): a
/// witnessed one, or a safety fact present but unreadable, is malformed,
/// alone and as any member of an all-clean panel. Absent facts are not.
#[test]
fn shared_clean_validation_rejects_witnessed_findings_without_requiring_optional_fields() {
    assert_eq!(Aggregate::review_problem(&json!({"result":"clean"})), None);
    assert_eq!(
        Aggregate::review_problem(&scope_member(
            "clean",
            Some(json!(false)),
            Some(json!(false))
        )),
        None
    );
    let security = "clean review reports a security residual";
    let above = "clean review reports a residual severity above none";
    let not_a_flag = "clean review's has_security_residual is not a boolean";
    let unranked = "clean review's max_residual_severity is outside the severity vocabulary";
    for (field, value, problem) in [
        ("has_security_residual", json!(true), security),
        ("has_security_residual", json!("true"), not_a_flag),
        ("has_security_residual", json!("false"), not_a_flag),
        ("has_security_residual", json!(1), not_a_flag),
        ("has_security_residual", json!(0), not_a_flag),
        ("has_security_residual", Value::Null, not_a_flag),
        ("max_residual_severity", json!("info"), above),
        ("max_residual_severity", json!("low"), above),
        ("max_residual_severity", json!("medium"), above),
        ("max_residual_severity", json!("high"), above),
        ("max_residual_severity", json!("critical"), above),
        ("max_residual_severity", json!("severe"), unranked),
        ("max_residual_severity", json!("NONE"), unranked),
        ("max_residual_severity", json!(0), unranked),
        ("max_residual_severity", json!(false), unranked),
        ("max_residual_severity", Value::Null, unranked),
    ] {
        let mut member = scope_member("clean", Some(json!(false)), Some(json!(false)));
        member["inputs"][field] = value;
        assert_eq!(
            Aggregate::review_problem(&member).as_deref(),
            Some(problem),
            "{member}"
        );
        for count in [1, 2, 4] {
            for position in 0..count {
                let mut members =
                    vec![scope_member("clean", Some(json!(false)), Some(json!(false))); count];
                members[position] = member.clone();
                let review = reviewed(members);
                assert_eq!(review["result"], "__member-schema-invalid__", "{review}");
                assert_eq!(review["notes"]["problem"], problem, "{review}");
                assert_eq!(review["notes"]["member"], format!("member-{position}"));
                assert_eq!(
                    review["notes"]["verdicts"][format!("member-{position}")],
                    "clean"
                );
            }
        }
    }
}

#[test]
fn holds_and_high_residuals_dominate_unknown_flags_and_clean_inconsistency() {
    let machine = strict_scope_machine();
    let clean_conflict = json!({"result":"clean","inputs":{"has_security_residual":true}});
    for dominant in [
        json!({"result":"security-hold"}),
        residual(json!({"max_residual_severity":"high","has_security_residual":false})),
        residual(json!({"max_residual_severity":"critical","has_security_residual":false})),
    ] {
        for members in [
            vec![dominant.clone(), clean_conflict.clone()],
            vec![clean_conflict.clone(), dominant.clone()],
        ] {
            let review = reviewed(members);
            assert_eq!(review["result"], dominant["result"]);
            assert_eq!(strict_destination(&machine, &review), "stop");
        }
    }
}
