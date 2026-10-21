use super::*;
use crate::bundle::entry::{admit, admitted, EntryRefusal};

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

#[test]
fn change_dependencies_name_the_step_and_the_ruling_that_reached_it() {
    let (config, policy) = recipe();
    let mut bundle = compiled(&config, &policy);
    let gate = bundle.seats["review"].clone();
    let body = |argv: Vec<String>| StepBody::Dialect {
        execution: DialectExecution { argv, state: None },
    };
    bundle.seats.get_mut("review").unwrap().body = SeatBody::Sequence {
        steps: vec![
            SequenceStep {
                name: "check".into(),
                class: SeatClass::Gate,
                results: vec!["clean".into()],
                body: body(vec!["check".into(), "{change}".into()]),
            },
            SequenceStep {
                name: "judge".into(),
                class: SeatClass::Gate,
                results: vec!["clean".into()],
                body: body(vec!["judge".into()]),
            },
        ],
    };
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
