//! An aggregate is named by one word, and read back from it; and a
//! panel's member findings are joined typed.

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

/// Fix round 0109a1r2, R6: the members' stamped findings are read typed
/// once and joined in declared order, each with its own member's site; a
/// member whose findings are not the engine's stamped records poisons the
/// aggregate with the member evidence, never joined as they stand.
#[test]
fn member_findings_join_typed_and_an_unstamped_one_poisons_the_aggregate() {
    let stamped = |site: char, id: &str| {
        json!({"site": site.to_string().repeat(64), "id": id, "file": "a.rs", "summary": "s",
               "severity": "low", "security": false})
    };
    let payloads = [
        json!({"result": "residual", "findings": [stamped('a', "x")]}),
        json!({"result": "clean"}),
        json!({"result": "clean", "findings": [stamped('c', "x")]}),
    ];
    let members = |payloads: &[Value]| {
        ["a", "b", "c"]
            .into_iter()
            .zip(payloads)
            .map(|(name, payload)| MemberVerdict::journaled(name, payload.clone()))
            .collect::<Vec<_>>()
    };
    let read = members(&payloads);
    let typed = Vec::<Finding>::deserialize(json!([stamped('a', "x")])).unwrap();
    assert_eq!(read[0].findings, MemberFindings::Read(typed));
    assert_eq!(read[0].payload, json!({"result": "residual"}), "taken out");
    let review = PanelJoin::open(Aggregate::ReviewPanel, &["a", "b", "c"]);
    let joined = review.aggregate_results(&read);
    assert_eq!(joined["result"], "residual");
    assert_eq!(
        joined["findings"],
        json!([stamped('a', "x"), stamped('c', "x")])
    );
    let mut unstamped = payloads.clone();
    unstamped[2]["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("site");
    let poisoned = review.aggregate_results(&members(&unstamped));
    assert_eq!(poisoned["result"], Aggregate::MEMBER_SCHEMA_INVALID);
    assert_eq!(poisoned["notes"]["members"]["c"], unstamped[2]);
    // Fix round 0109a1r6, W4: a's admitted finding stays the record's
    // keyed finding, once, under its own site, never hidden in the notes.
    let bare = json!({"result": "residual"});
    assert_eq!(poisoned["notes"]["members"]["a"], bare, "{poisoned}");
    assert_eq!(
        poisoned["findings"],
        json!([stamped('a', "x")]),
        "{poisoned}"
    );
}

/// Decision 0109 ruling 11: a review-panel member's word outside its
/// compiled law poisons the aggregate, naming the member and the reason;
/// a member the join holds no law for is refused, never read as lawful;
/// and a partial retry joins its carried members under their own laws.
#[test]
fn a_review_panel_members_word_answers_to_its_law_and_a_lawless_one_poisons() {
    let narrow = MemberLaw::declaring(&["clean", "residual"]);
    let members = MemberVerdict::all(&[
        ("a", json!({"result": "clean"})),
        ("b", json!({"result": "security-hold"})),
    ]);
    let join = PanelJoin::under(
        Aggregate::ReviewPanel,
        [("a", narrow.clone()), ("b", narrow)],
    );
    let poisoned = join.aggregate_results(&members);
    assert_eq!(poisoned["result"], Aggregate::MEMBER_SCHEMA_INVALID);
    assert_eq!(
        poisoned["notes"]["refused"],
        json!({"b": "member result 'security-hold' is not among its declared results [clean, residual]"})
    );
    let lawless = PanelJoin::open(Aggregate::ReviewPanel, &["a"]).aggregate_results(&members);
    assert_eq!(
        lawless["notes"]["refused"],
        json!({"b": "the member has no compiled result set"})
    );
    let carried = PanelJoin::open(Aggregate::ReviewPanel, &["a"])
        .carrying(&[("b".to_string(), MemberLaw::open())].into());
    assert_eq!(
        carried.aggregate_results(&members)["result"],
        "security-hold"
    );
    let unanimous = PanelJoin::open(Aggregate::UnanimousPass, &[]);
    let passing = MemberVerdict::all(&[("a", json!({"result": "pass"}))]);
    assert_eq!(
        unanimous.aggregate_results(&passing)["result"],
        "pass",
        "keeps its own check"
    );
}

/// The review panel's aggregate over `members`, each joined under an
/// open law, named by its position.
fn reviewed(members: Vec<Value>) -> Value {
    let names: Vec<String> = (0..members.len()).map(|i| format!("m{i}")).collect();
    let pairs: Vec<(&str, Value)> = names.iter().map(String::as_str).zip(members).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    PanelJoin::open(Aggregate::ReviewPanel, &refs).aggregate_results(&MemberVerdict::all(&pairs))
}

fn residual(inputs: Value) -> Value {
    json!({"result": "residual", "inputs": inputs})
}

/// Where a table that ships a residual only on explicitly safe facts
/// sends `review`: the rule that matched and its next phase.
fn admission(review: &Value) -> (String, String) {
    use brokkr_core::policy::{Machine, Outcome};
    let machine = Machine::from_table(&json!({
        "schema": "forge.phase-machine/v1", "phases": ["review", "ship", "stop"],
        "initial": "review", "terminal": ["ship", "stop"], "rules": [
            {"id": "HOLD", "from": "review", "result": "security-hold", "next": "stop",
             "severity": "hard", "reason": "hold"},
            {"id": "HIGH", "from": "review", "result": "residual", "next": "stop",
             "severity": "hard", "reason": "high",
             "when": {"max_residual_severity_above": "medium"}},
            {"id": "SAFE", "from": "review", "result": "residual", "next": "ship",
             "reason": "explicitly safe",
             "when": {"has_security_residual": false, "max_residual_severity_at_most": "medium"}},
            {"id": "UNSAFE", "from": "review", "result": "residual", "next": "stop",
             "severity": "hard", "reason": "unknown or unsafe"},
            {"id": "CLEAN", "from": "review", "result": "clean", "next": "ship", "reason": "clean"}
        ]
    }))
    .unwrap();
    let result = review["result"].as_str().unwrap();
    let inputs = review["inputs"].as_object().unwrap();
    match machine.evaluate("review", result, inputs) {
        Outcome::Ruling {
            rule_id,
            next_phase,
            ..
        } => (rule_id, next_phase),
        other => panic!("expected a ruling, got {other:?}"),
    }
}

fn ruled(rule: &str, next: &str) -> (String, String) {
    (rule.to_string(), next.to_string())
}

/// Decision 0120 ruling 1: a residual member whose safety facts are
/// missing, null or mistyped leaves the panel's fact unknown beside a
/// clean member, so the table's explicitly-safe route never matches.
#[test]
fn a_residual_members_missing_null_or_mistyped_safety_facts_stay_unknown() {
    let malformed = [
        json!({"result": "residual"}),
        residual(Value::Null),
        residual(json!([])),
        residual(json!({})),
        residual(json!({"max_residual_severity": "low"})),
        residual(json!({"has_security_residual": null, "max_residual_severity": "low"})),
        residual(json!({"has_security_residual": "false", "max_residual_severity": "low"})),
        residual(json!({"has_security_residual": 0, "max_residual_severity": "low"})),
        residual(json!({"has_security_residual": {}, "max_residual_severity": "low"})),
        residual(json!({"has_security_residual": false})),
        residual(json!({"has_security_residual": false, "max_residual_severity": null})),
        residual(json!({"has_security_residual": false, "max_residual_severity": "invented"})),
        residual(json!({"has_security_residual": false, "max_residual_severity": 0})),
        residual(json!({"has_security_residual": false, "max_residual_severity": []})),
    ];
    for member in malformed {
        let review = reviewed(vec![json!({"result": "clean"}), member]);
        assert_eq!(admission(&review), ruled("UNSAFE", "stop"), "{review}");
        let unknown = |name: &str| review["inputs"][name].is_null();
        assert!(
            unknown("has_security_residual") || unknown("max_residual_severity"),
            "{review}"
        );
    }
}

/// Every readable residual keeps its exact severity and security flag.
#[test]
fn a_readable_residual_keeps_every_severity_and_security_fact() {
    for severity in ["none", "info", "low", "medium", "high", "critical"] {
        for security in [false, true] {
            let review = reviewed(vec![residual(json!({
                "has_security_residual": security, "max_residual_severity": severity
            }))]);
            assert_eq!(review["inputs"]["has_security_residual"], security);
            assert_eq!(review["inputs"]["max_residual_severity"], severity);
            let expected = match (severity, security) {
                ("high" | "critical", _) => ruled("HIGH", "stop"),
                (_, true) => ruled("UNSAFE", "stop"),
                (_, false) => ruled("SAFE", "ship"),
            };
            assert_eq!(admission(&review), expected, "{review}");
        }
    }
}

/// A safe residual member cannot lend its facts to a malformed one, in
/// either order.
#[test]
fn a_safe_member_never_supplies_a_malformed_residuals_missing_facts() {
    let safe = residual(json!({"has_security_residual": false, "max_residual_severity": "medium"}));
    for bad in [
        residual(json!({})),
        residual(json!({"has_security_residual": "false", "max_residual_severity": "low"})),
    ] {
        for members in [vec![safe.clone(), bad.clone()], vec![bad, safe.clone()]] {
            let review = reviewed(members);
            assert_eq!(admission(&review), ruled("UNSAFE", "stop"), "{review}");
        }
    }
}

/// A witnessed severity above medium, and a witnessed security residual,
/// survive a sibling's unknown facts, so the table's stops still see them.
#[test]
fn witnessed_high_and_security_findings_survive_a_siblings_unknowns() {
    for severity in ["high", "critical"] {
        let high = residual(json!({"has_security_residual": false,
                                   "max_residual_severity": severity}));
        for members in [
            vec![high.clone(), residual(json!({}))],
            vec![residual(json!({})), high],
        ] {
            let review = reviewed(members);
            assert_eq!(review["inputs"]["max_residual_severity"], severity);
            assert_eq!(admission(&review), ruled("HIGH", "stop"));
        }
    }
    let security = residual(json!({"has_security_residual": true, "max_residual_severity": "low"}));
    for members in [
        vec![security.clone(), residual(json!({}))],
        vec![residual(json!({})), security],
    ] {
        let review = reviewed(members);
        assert_eq!(review["inputs"]["has_security_residual"], true);
        assert_eq!(review["inputs"]["max_residual_severity"], Value::Null);
        assert_eq!(admission(&review), ruled("UNSAFE", "stop"));
    }
}

/// A clean member's witnessed security residual still reaches a residual
/// panel's fact, whatever the residual member says.
#[test]
fn a_clean_members_witnessed_security_flag_still_stops_a_residual() {
    let clean = json!({"result": "clean", "inputs": {"has_security_residual": true}});
    for inputs in [
        json!({}),
        json!({"has_security_residual": false, "max_residual_severity": "low"}),
    ] {
        let member = residual(inputs);
        for members in [
            vec![clean.clone(), member.clone()],
            vec![member, clean.clone()],
        ] {
            let review = reviewed(members);
            assert_eq!(review["inputs"]["has_security_residual"], true);
            assert_eq!(admission(&review), ruled("UNSAFE", "stop"));
        }
    }
}

/// A hold dominates a sibling's unknowns, and an all-clean panel keeps
/// writing `none` and `false` (decision 0109 ruling 2): a clean member's
/// omitted facts are not a residual's.
#[test]
fn a_hold_dominates_unknowns_and_an_all_clean_panel_stays_none_and_false() {
    for members in [
        vec![
            json!({"result": "clean"}),
            residual(json!({})),
            json!({"result": "security-hold"}),
        ],
        vec![
            json!({"result": "security-hold"}),
            residual(json!({})),
            json!({"result": "clean"}),
        ],
    ] {
        let review = reviewed(members);
        assert_eq!(review["result"], "security-hold");
        assert_eq!(admission(&review), ruled("HOLD", "stop"));
    }
    let review = reviewed(vec![json!({"result": "clean"}), json!({"result": "clean"})]);
    assert_eq!(
        review["inputs"],
        json!({"fixes_applied": false, "spec_defect": false,
               "max_residual_severity": "none", "has_security_residual": false})
    );
    assert_eq!(admission(&review), ruled("CLEAN", "ship"));
}
