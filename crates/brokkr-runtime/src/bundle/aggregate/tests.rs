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
