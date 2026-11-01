use super::*;
use crate::bundle::entry::{admit, admitted, EntryRefusal};
use crate::places::places_tests::clean_git_config;

/// A synthetic gate road, with a work seat only on a return.
fn recipe() -> (Value, Value) {
    let phases = ["implement", "verify", "review", "ship", "done", "stop"];
    let mut seats = serde_json::Map::new();
    for (phase, results, class) in [
        ("implement", vec!["complete"], "work"),
        ("verify", vec!["pass", "fail"], "gate"),
        ("review", vec!["clean"], "gate"),
        ("ship", vec!["shipped", "blocked"], "gate"),
    ] {
        seats.insert(phase.into(), json!({"results":results,"class":class,
            "role":"roles/role.md","hands":"workspace","driver":{"command":["{brokkr}","driver","exec","--","true"]}}));
    }
    let rules: Vec<Value> = [
        ("IMPL-OK","implement","complete","verify"),
        ("VERIFY-PASS","verify","pass","review"),
        ("VERIFY-FAIL","verify","fail","implement"),
        ("REVIEW-CLEAN","review","clean","ship"),
        ("SHIP-COMPLETE","ship","shipped","done"),
        ("SHIP-BLOCKED","ship","blocked","stop"),
    ].into_iter().map(|(id,from,result,next)|
        json!({"id":id,"from":from,"result":result,"next":next,"reason":"synthetic"})).collect();
    (
        json!({"name":"entry-fixture","policy":"policy.json","seats":seats}),
        json!({"phases":phases,"initial":"implement","terminal":["done","stop"],"rules":rules}),
    )
}

fn compiled(config: &Value, policy: &Value) -> Bundle {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.root.join("bundle.json"),
        serde_json::to_vec(config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.root.join("policy.json"),
        serde_json::to_vec(policy).unwrap(),
    )
    .unwrap();
    let root = workspace_root();
    Bundle::compile_with(&fixture.root, &root.join("agents"), &root.join("adapters")).unwrap()
}

#[test]
fn entry_refusals_are_ranked_and_keep_review_unavoidable() {
    let (config, policy) = recipe();
    let bundle = compiled(&config, &policy);
    assert_eq!(admitted(&bundle), ["verify", "review"]);
    assert_eq!(admit(&bundle, "verify"), Ok(()));
    for (phase, refusal) in [
        (
            "implement",
            EntryRefusal::Work {
                site: WorkSite::seat(),
            },
        ),
        (
            "ship",
            EntryRefusal::AvoidsProtected {
                protected: "review".into(),
                terminal: "done".into(),
            },
        ),
        ("intake", EntryRefusal::Absent),
        ("done", EntryRefusal::Terminal),
    ] {
        let error = admit(&bundle, phase).unwrap_err();
        assert_eq!(error.phase, phase);
        assert_eq!(*error.refusal, refusal);
        assert_eq!(error.admitted, ["verify", "review"]);
    }
    assert_eq!(
        admit(&bundle, "implement").unwrap_err().to_string(),
        "cannot enter 'implement': work site '' (compiled: true); admitted entries: verify, review"
    );
    // The recipe and its identity remain unchanged by admission.
    assert_eq!(bundle.machine.initial, "implement");
}

#[test]
fn entry_names_are_checked_before_any_gate_walk() {
    for (name, expected) in [
        ("v".repeat(64), true),
        ("v".repeat(65), false),
        ("verify gate".into(), false),
    ] {
        let (config, policy) = recipe();
        let config = serde_json::from_str::<Value>(
            &serde_json::to_string(&config)
                .unwrap()
                .replace("verify", &name),
        )
        .unwrap();
        let policy = serde_json::from_str::<Value>(
            &serde_json::to_string(&policy)
                .unwrap()
                .replace("verify", &name),
        )
        .unwrap();
        let bundle = compiled(&config, &policy);
        assert_eq!(
            admit(&bundle, &name).map_err(|error| *error.refusal),
            if expected {
                Ok(())
            } else {
                Err(EntryRefusal::Unprintable)
            }
        );
    }
}

#[test]
fn forward_work_is_refused_by_the_ruling_that_reaches_it() {
    let (mut config, policy) = recipe();
    config["seats"]["review"]["class"] = json!("work");
    let bundle = compiled(&config, &policy);
    assert_eq!(
        *admit(&bundle, "verify").unwrap_err().refusal,
        EntryRefusal::ForwardWork {
            work: "review".into(),
            rule: "VERIFY-PASS".into()
        }
    );
    assert_eq!(
        *admit(&bundle, "review").unwrap_err().refusal,
        EntryRefusal::Work {
            site: WorkSite::seat()
        }
    );
}

#[test]
fn a_selector_needs_a_strategy_source_or_a_default() {
    let (mut config, policy) = recipe();
    let gate = selecting_review(&mut config);
    let bundle = compiled(&config, &policy);
    assert_eq!(
        *admit(&bundle, "verify").unwrap_err().refusal,
        EntryRefusal::NeedsStrategy {
            seat: "review".into(),
            reached_by: Some("VERIFY-PASS".into())
        }
    );
    config["seats"]["review"]["select"]["default"] = gate;
    let bundle = compiled(&config, &policy);
    assert_eq!(admitted(&bundle), ["verify", "review"]);
}

#[test]
fn strategy_dependent_rulings_need_a_source() {
    let (config, mut policy) = recipe();
    policy["rules"][3]["when"] =
        json!({"strategy_in":["chore","feature","design","engine","escalate"]});
    let bundle = compiled(&config, &policy);
    assert_eq!(
        *admit(&bundle, "verify").unwrap_err().refusal,
        EntryRefusal::NeedsStrategyRuling {
            group: ("review".into(), "clean".into()),
            reached_by: Some("VERIFY-PASS".into())
        }
    );
}

#[test]
fn strategy_entry_supplies_the_strategy_before_a_later_selector() {
    let (mut config, mut policy) = recipe();
    selecting_review(&mut config);
    config["seats"]["triage"] = config["seats"]["verify"].clone();
    config["seats"]["triage"]["results"] = json!(["engine"]);
    policy["phases"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!("triage"));
    policy["initial"] = json!("triage");
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"TRIAGE-ENGINE","from":"triage","result":"engine","next":"verify","reason":"synthetic"
    }));
    let bundle = compiled(&config, &policy);
    assert_eq!(admit(&bundle, "triage"), Ok(()));
    assert_eq!(
        *admit(&bundle, "verify").unwrap_err().refusal,
        EntryRefusal::NeedsStrategy {
            seat: "review".into(),
            reached_by: Some("VERIFY-PASS".into())
        }
    );
}

/// A dialect step body running `argv`.
fn dialect(argv: &[&str]) -> StepBody {
    StepBody::Dialect {
        execution: DialectExecution {
            argv: argv.iter().map(|arg| (*arg).into()).collect(),
            state: None,
        },
    }
}

/// A two-step gate sequence, `check` then `judge`, each a dialect step
/// running its argv; the tests differ only in which argv carries `{change}`.
fn check_judge(check: &[&str], judge: &[&str]) -> SeatBody {
    let steps = [("check", check), ("judge", judge)]
        .into_iter()
        .map(|(name, argv)| SequenceStep {
            name: name.into(),
            class: SeatClass::Gate,
            results: vec!["clean".into()],
            body: dialect(argv),
        })
        .collect();
    SeatBody::Sequence { steps }
}

#[test]
fn change_dependencies_name_the_step_and_the_ruling_that_reached_it() {
    let (config, policy) = recipe();
    let mut bundle = compiled(&config, &policy);
    let gate = bundle.seats["review"].clone();
    bundle.seats.get_mut("review").unwrap().body = check_judge(&["check", "{change}"], &["judge"]);
    assert_eq!(
        *admit(&bundle, "verify").unwrap_err().refusal,
        EntryRefusal::NeedsChange {
            step: "review:check".into(),
            sources: vec![],
            reached_by: Some("VERIFY-PASS".into())
        }
    );
    assert_eq!(
        *admit(&bundle, "review").unwrap_err().refusal,
        EntryRefusal::NeedsChange {
            step: "review:check".into(),
            sources: vec![],
            reached_by: None
        }
    );
    let mut source = gate;
    source.inputs = vec!["change".into()];
    bundle.seats.insert("design".into(), source);
    bundle.machine.phases.insert(1, "design".into());
    let mut rule = bundle.machine.rules[0].clone();
    rule.id = "DESIGN-OK".into();
    rule.from = "design".into();
    rule.next = Some("verify".into());
    bundle.machine.rules.push(rule);
    assert_eq!(admit(&bundle, "design"), Ok(()));
    assert_eq!(
        *admit(&bundle, "verify").unwrap_err().refusal,
        EntryRefusal::NeedsChange {
            step: "review:check".into(),
            sources: vec!["design".into()],
            reached_by: Some("VERIFY-PASS".into())
        }
    );
}

#[test]
fn a_declared_change_input_serves_every_step_after_the_first() {
    let (config, policy) = recipe();
    let mut bundle = compiled(&config, &policy);
    let review = bundle.seats.get_mut("review").unwrap();
    review.inputs = vec!["change".into()];
    review.body = check_judge(&["check"], &["judge", "{change}"]);
    assert_eq!(admitted(&bundle), ["verify", "review"]);
    bundle.seats.get_mut("review").unwrap().inputs = vec![];
    let needs = |step: &str| EntryRefusal::NeedsChange {
        step: format!("review:{step}"),
        sources: vec![],
        reached_by: None,
    };
    expect_refusal(&bundle, "review", needs("judge"));
    assert_eq!(admitted(&bundle), Vec::<String>::new());
    // The declared input serves no first step: that one has nothing before
    // it to carry the change.
    let review = bundle.seats.get_mut("review").unwrap();
    review.inputs = vec!["change".into()];
    let SeatBody::Sequence { steps } = &mut review.body else {
        panic!("sequence");
    };
    steps[0].body = dialect(&["check", "{change}"]);
    expect_refusal(&bundle, "review", needs("check"));
}

/// The compile writes every dialect step as a gate (step_class), so this arm
/// is reached only by hand. It stays a work site, never AllGate: were that
/// invariant to break, a work step would otherwise read as a gate.
#[test]
fn a_dialect_step_written_as_work_is_a_work_site_under_its_name() {
    let body = StepBody::Dialect {
        execution: DialectExecution {
            argv: vec!["check".into()],
            state: None,
        },
    };
    assert_eq!(
        crate::bundle::gate::step_assessment("check", SeatClass::Work, &body),
        GateAssessment::ContainsWork(WorkSite {
            path: vec!["check".into()],
            compiled: true
        })
    );
    assert_eq!(
        crate::bundle::gate::step_assessment("check", SeatClass::Gate, &body),
        GateAssessment::AllGate
    );
}

#[test]
fn the_first_uncompiled_dialect_work_site_is_the_witness() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/speckit.json"))
        .unwrap()
        .0;
    let (mut config, policy) = loop_config(json!([
        {"name":"check","dialect":"check","results":["clear"]},
        {"name":"judge","class":"gate","hands":"workspace","role":"roles/role.md",
         "driver":{"command":["{brokkr}","driver","exec","--","true"]}}
    ]));
    config["seats"]["review"] = config["seats"]["clarify"]["sequence"][1].clone();
    config["seats"]["review"]
        .as_object_mut()
        .unwrap()
        .remove("name");
    config["seats"]["review"]["results"] = json!(["clean"]);
    // This fixture's work phase is deliberately unrelated to the gate class.
    let bundle = compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).unwrap();
    assert_eq!(
        *admit(&bundle, "clarify").unwrap_err().refusal,
        EntryRefusal::Work {
            site: WorkSite {
                path: vec!["check".into()],
                compiled: false
            }
        }
    );
    config["seats"]["clarify"]["sequence"][0]["class"] = json!("gate");
    let bundle = compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).unwrap();
    assert_eq!(admitted(&bundle), ["clarify", "review"]);
}

fn selecting_review(config: &mut Value) -> Value {
    let mut gate = config["seats"]["review"].clone();
    gate.as_object_mut().unwrap().remove("results");
    config["seats"]["review"] = json!({"results":["clean"],"select":{
        "on":"strategy","cases":{"chore":gate,"feature":gate,"design":gate,"engine":gate}}});
    gate
}

#[test]
fn entry_refusal_text_names_each_witness() {
    for (refusal, text) in [
        (EntryRefusal::Absent, "phase is absent from the table"),
        (EntryRefusal::Terminal, "phase is terminal, with no seat"),
        (
            EntryRefusal::Unprintable,
            "phase is outside the bounded entry-name grammar",
        ),
        (
            EntryRefusal::Work {
                site: WorkSite {
                    path: vec!["case".into(), "check".into()],
                    compiled: false,
                },
            },
            "work site 'case:check' (compiled: false)",
        ),
        (
            EntryRefusal::AvoidsProtected {
                protected: "review".into(),
                terminal: "done".into(),
            },
            "terminal 'done' is reachable without protected phase 'review'",
        ),
        (
            EntryRefusal::ForwardWork {
                work: "review".into(),
                rule: "VERIFY-PASS".into(),
            },
            "forward rule 'VERIFY-PASS' reaches work phase 'review'",
        ),
        (
            EntryRefusal::NeedsStrategy {
                seat: "review".into(),
                reached_by: Some("VERIFY-PASS".into()),
            },
            "seat 'review' needs strategy, reached by VERIFY-PASS",
        ),
        (
            EntryRefusal::NeedsStrategyRuling {
                group: ("review".into(), "clean".into()),
                reached_by: None,
            },
            "ruling group (review, clean) needs strategy, reached by entry itself",
        ),
        (
            EntryRefusal::NeedsVisits {
                group: ("review".into(), "clean".into()),
                phase: "intake".into(),
                reached_by: Some("VERIFY-PASS".into()),
            },
            "ruling group (review, clean) needs the visits of 'intake', reached by VERIFY-PASS",
        ),
        (
            EntryRefusal::NeedsChange {
                step: "review:check".into(),
                sources: vec!["design".into(), "tasks".into()],
                reached_by: Some("VERIFY-PASS".into()),
            },
            "step 'review:check' needs change from [design, tasks], reached by VERIFY-PASS",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}

#[test]
fn shipped_refusals_name_the_written_work_site_and_first_missing_source() {
    clean_git_config!();
    let root = workspace_root();
    let compile = |name| {
        Bundle::compile_with(
            &root.join("recipes").join(name),
            &root.join("agents"),
            &root.join("adapters"),
        )
        .unwrap()
    };
    let flash = compile("gpt-flash");
    assert_eq!(
        *admit(&flash, "review").unwrap_err().refusal,
        EntryRefusal::Work {
            site: WorkSite {
                path: vec!["chore".into(), "positions".into(), "correctness".into()],
                compiled: true
            }
        }
    );
    assert_eq!(
        *admit(&flash, "verify").unwrap_err().refusal,
        EntryRefusal::ForwardWork {
            work: "review".into(),
            rule: "VERIFY-PASS".into()
        }
    );
    let triage = compile("triage");
    assert_eq!(
        *admit(&triage, "triage").unwrap_err().refusal,
        EntryRefusal::ForwardWork {
            work: "implement".into(),
            rule: "TRIAGE-CHORE".into()
        }
    );
    assert_eq!(
        *admit(&triage, "verify").unwrap_err().refusal,
        EntryRefusal::NeedsStrategy {
            seat: "implement".into(),
            reached_by: Some("VERIFY-FAIL".into())
        }
    );
    assert_eq!(
        *admit(&triage, "review").unwrap_err().refusal,
        EntryRefusal::NeedsStrategy {
            seat: "review".into(),
            reached_by: None
        }
    );
}

fn conditioned_road(id: &str, when: Value) -> (Value, Value) {
    let (config, mut policy) = recipe();
    policy["rules"].as_array_mut().unwrap().insert(
        3,
        json!({
            "id":id,"from":"review","result":"clean","severity":"hard",
            "when":when,"next":"stop","reason":"synthetic denial"
        }),
    );
    (config, policy)
}

fn expect_refusal(bundle: &Bundle, entry: &str, expected: EntryRefusal) {
    assert_eq!(*admit(bundle, entry).unwrap_err().refusal, expected);
}

fn strategy_refusal(phase: &str, result: &str, reached: Option<&str>) -> EntryRefusal {
    EntryRefusal::NeedsStrategyRuling {
        group: (phase.into(), result.into()),
        reached_by: reached.map(str::to_string),
    }
}

fn visits_refusal(from: &str, result: &str, counted: &str, reached: Option<&str>) -> EntryRefusal {
    EntryRefusal::NeedsVisits {
        group: (from.into(), result.into()),
        phase: counted.into(),
        reached_by: reached.map(str::to_string),
    }
}

fn verify_strategy_rule(policy: &mut Value) {
    policy["rules"].as_array_mut().unwrap().insert(
        2,
        json!({
            "id":"VERIFY-FAIL-ENGINE","from":"verify","result":"fail","next":"implement",
            "when":{"strategy_in":["engine"]},"reason":"synthetic"
        }),
    );
}

#[test]
fn a_strategy_denial_is_preserved_above_an_unconditional_allow() {
    let (mut config, mut policy) =
        conditioned_road("DENY-ENGINE", json!({"strategy_in":["engine"]}));
    let gate = selecting_review(&mut config);
    config["seats"]["review"]["select"]["default"] = gate;
    let bundle = compiled(&config, &policy);
    expect_refusal(
        &bundle,
        "verify",
        strategy_refusal("review", "clean", Some("VERIFY-PASS")),
    );
    expect_refusal(&bundle, "review", strategy_refusal("review", "clean", None));
    policy["rules"]
        .as_array_mut()
        .unwrap()
        .retain(|rule| rule["id"] != "DENY-ENGINE");
    assert_eq!(admit(&compiled(&config, &policy), "verify"), Ok(()));
}

#[test]
fn strategy_groups_use_result_key_order_and_selectors_rank_first() {
    let (mut config, mut policy) =
        conditioned_road("DENY-ENGINE", json!({"strategy_in":["engine"]}));
    config["seats"]["review"]["results"] = json!(["clean", "residual"]);
    let residual = vec![
        json!({"id":"DENY-DESIGN-RESIDUAL","from":"review","result":"residual","severity":"hard","next":"stop","when":{"strategy_in":["design"]},"reason":"synthetic"}),
        json!({"id":"RESIDUAL-REWORK","from":"review","result":"residual","next":"implement","reason":"synthetic"}),
    ];
    for front in [true, false] {
        let mut variant = policy.clone();
        let rules = variant["rules"].as_array_mut().unwrap();
        let index = if front { 3 } else { rules.len() };
        rules.splice(index..index, residual.clone());
        expect_refusal(
            &compiled(&config, &variant),
            "verify",
            strategy_refusal("review", "clean", Some("VERIFY-PASS")),
        );
    }
    verify_strategy_rule(&mut policy);
    let gate = selecting_review(&mut config);
    expect_refusal(
        &compiled(&config, &policy),
        "verify",
        EntryRefusal::NeedsStrategy {
            seat: "review".into(),
            reached_by: Some("VERIFY-PASS".into()),
        },
    );
    config["seats"]["review"]["select"]["default"] = gate;
    expect_refusal(
        &compiled(&config, &policy),
        "verify",
        strategy_refusal("verify", "fail", None),
    );
}

fn change_step(bundle: &mut Bundle, phase: &str) {
    bundle.seats.get_mut(phase).unwrap().body = SeatBody::Sequence {
        steps: vec![SequenceStep {
            name: "check".into(),
            class: SeatClass::Gate,
            results: vec!["clean".into()],
            body: StepBody::Dialect {
                execution: DialectExecution {
                    argv: vec!["check".into(), "{change}".into()],
                    state: None,
                },
            },
        }],
    };
}

#[test]
fn selector_and_ruling_facts_rank_before_a_missing_change() {
    let (mut config, mut policy) = recipe();
    policy["phases"] = json!([
        "triage",
        "intake",
        "implement",
        "verify",
        "analyze",
        "review",
        "ship",
        "done",
        "stop"
    ]);
    policy["initial"] = json!("triage");
    for phase in ["triage", "intake", "analyze"] {
        config["seats"][phase] = config["seats"]["verify"].clone();
        config["seats"][phase]["results"] = json!(["complete"]);
    }
    config["seats"]["intake"]["class"] = json!("work");
    config["seats"]["intake"]["inputs"] = json!(["change"]);
    let rules = policy["rules"].as_array_mut().unwrap();
    rules[1]["next"] = json!("analyze");
    for (id, from, next) in [
        ("TRIAGE-ENGINE", "triage", "intake"),
        ("INTAKE-OK", "intake", "implement"),
        ("ANALYZE-CONSISTENT", "analyze", "review"),
    ] {
        rules.push(
            json!({"id":id,"from":from,"result":"complete","next":next,"reason":"synthetic"}),
        );
    }
    let gate = selecting_review(&mut config);
    let make = |config: &Value, policy: &Value| {
        let mut bundle = compiled(config, policy);
        change_step(&mut bundle, "analyze");
        bundle
    };
    expect_refusal(
        &make(&config, &policy),
        "verify",
        EntryRefusal::NeedsStrategy {
            seat: "review".into(),
            reached_by: Some("ANALYZE-CONSISTENT".into()),
        },
    );
    config["seats"]["review"]["select"]["default"] = gate;
    expect_refusal(
        &make(&config, &policy),
        "verify",
        EntryRefusal::NeedsChange {
            step: "analyze:check".into(),
            sources: vec!["intake".into()],
            reached_by: Some("VERIFY-PASS".into()),
        },
    );
    verify_strategy_rule(&mut policy);
    expect_refusal(
        &make(&config, &policy),
        "verify",
        strategy_refusal("verify", "fail", None),
    );
}

fn counted_road() -> (Value, Value) {
    let (mut config, mut policy) =
        conditioned_road("STOP-AFTER-INTAKE", json!({"visits_intake_gte":1}));
    config["seats"]["intake"] = config["seats"]["implement"].clone();
    policy["phases"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!("intake"));
    policy["initial"] = json!("intake");
    policy["rules"].as_array_mut().unwrap().push(json!({"id":"INTAKE-OK","from":"intake","result":"complete","next":"implement","reason":"synthetic"}));
    (config, policy)
}

#[test]
fn skipped_counts_preserve_denials_and_require_an_unguarded_road() {
    let (config, policy) = counted_road();
    let bundle = compiled(&config, &policy);
    expect_refusal(
        &bundle,
        "verify",
        visits_refusal("review", "clean", "intake", Some("VERIFY-PASS")),
    );
    expect_refusal(
        &bundle,
        "review",
        visits_refusal("review", "clean", "intake", None),
    );
    let mut implement = policy.clone();
    implement["rules"][3]["when"] = json!({"visits_implement_gte":1});
    let bundle = compiled(&config, &implement);
    assert_eq!(admit(&bundle, "verify"), Ok(()));
    expect_refusal(
        &bundle,
        "review",
        visits_refusal("review", "clean", "implement", None),
    );
    for (when, from, result, reached) in [
        (json!({"visits_intake_gte":1}), "verify", "fail", None),
        (
            json!({"visits_implement_gte":1}),
            "review",
            "clean",
            Some("VERIFY-PASS"),
        ),
    ] {
        let mut variant = policy.clone();
        variant["rules"].as_array_mut().unwrap().insert(2, json!({"id":"REDO-INTAKE","from":"verify","result":"fail","next":"intake","when":when,"reason":"synthetic"}));
        expect_refusal(
            &compiled(&config, &variant),
            "verify",
            visits_refusal(from, result, "intake", reached),
        );
    }
    let mut config = config;
    config["seats"]["verify"]["inputs"] = json!(["spec_defect"]);
    let mut variant = policy;
    variant["rules"].as_array_mut().unwrap().insert(2, json!({"id":"REDO-INTAKE","from":"verify","result":"fail","next":"intake","when":{"spec_defect":true},"reason":"synthetic"}));
    let bundle = compiled(&config, &variant);
    assert_eq!(admitted(&bundle), ["verify"]);
    expect_refusal(
        &bundle,
        "review",
        visits_refusal("review", "clean", "intake", None),
    );
}

#[test]
fn count_guarded_phases_are_judged_with_the_first_meeting_witness() {
    let (config, mut policy) = counted_road();
    let rules = policy["rules"].as_array_mut().unwrap();
    rules.remove(1);
    rules.insert(2, json!({"id":"VERIFY-PASS-WORKED","from":"verify","result":"pass","next":"review","when":{"visits_implement_gte":1},"reason":"synthetic"}));
    rules.insert(3, json!({"id":"VERIFY-PASS-UNWORKED","from":"verify","result":"pass","next":"implement","reason":"synthetic"}));
    let bundle = compiled(&config, &policy);
    expect_refusal(
        &bundle,
        "verify",
        visits_refusal("review", "clean", "intake", Some("VERIFY-PASS-WORKED")),
    );
    expect_refusal(
        &bundle,
        "review",
        visits_refusal("review", "clean", "intake", None),
    );
    policy["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "STOP-AFTER-INTAKE")
        .unwrap()["when"] = json!({"visits_implement_gte":1});
    assert_eq!(admitted(&compiled(&config, &policy)), ["verify"]);
}

#[test]
fn counted_phase_order_and_change_rank_keep_their_witnesses() {
    let (mut config, mut policy) = counted_road();
    config["seats"]["design"] = config["seats"]["implement"].clone();
    policy["phases"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!("design"));
    let rules = policy["rules"].as_array_mut().unwrap();
    rules.iter_mut().find(|r| r["id"] == "INTAKE-OK").unwrap()["next"] = json!("design");
    rules.push(json!({"id":"DESIGN-OK","from":"design","result":"complete","next":"implement","reason":"synthetic"}));
    rules[3]["when"] = json!({"visits_intake_gte":1,"visits_design_gte":1});
    expect_refusal(
        &compiled(&config, &policy),
        "verify",
        visits_refusal("review", "clean", "design", Some("VERIFY-PASS")),
    );
    policy["rules"][3]["when"] = json!({"visits_intake_gte":1});
    policy["rules"].as_array_mut().unwrap().insert(4, json!({"id":"STOP-AFTER-DESIGN","from":"review","result":"clean","next":"stop","severity":"hard","when":{"visits_design_gte":1},"reason":"synthetic"}));
    let mut bundle = compiled(&config, &policy);
    expect_refusal(
        &bundle,
        "verify",
        visits_refusal("review", "clean", "intake", Some("VERIFY-PASS")),
    );
    change_step(&mut bundle, "review");
    expect_refusal(
        &bundle,
        "verify",
        EntryRefusal::NeedsChange {
            step: "review:check".into(),
            sources: vec![],
            reached_by: Some("VERIFY-PASS".into()),
        },
    );
}

#[test]
fn dialect_gate_checks_follow_only_forward_roads_to_work() {
    clean_git_config!();
    let root = workspace_root();
    let mut bundle = Bundle::compile_with(
        &root.join("recipes/triage"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap();
    for phase in ["clarify", "analyze"] {
        let seat = bundle.seats.get_mut(phase).unwrap();
        let SeatBody::Sequence { steps } = &mut seat.body else {
            panic!("sequence");
        };
        assert_eq!(steps[0].name, "check");
        steps[0].class = SeatClass::Gate;
        seat.gate = seat.body.assess();
        assert_eq!(seat.gate, GateAssessment::AllGate);
    }
    for (phase, work, rule) in [
        ("clarify", "design", "CLARIFY-CLEAR"),
        ("analyze", "implement", "ANALYZE-CONSISTENT"),
    ] {
        expect_refusal(
            &bundle,
            phase,
            EntryRefusal::ForwardWork {
                work: work.into(),
                rule: rule.into(),
            },
        );
    }
}
