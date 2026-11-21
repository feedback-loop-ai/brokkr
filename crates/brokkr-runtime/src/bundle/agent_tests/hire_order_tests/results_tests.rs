//! Decision 0109 ruling 11 at compile: a review-panel member's lawful
//! words are its hired agent's typed `results` list, or the narrow
//! `clean` and `residual` where it declares none; a `unanimous-pass`
//! member keeps that aggregate's own two words; a member's own `results`
//! key is refused; and a single seat's declared results lie within its
//! hired agent's list, a ceiling and never a floor.

use super::*;

const THREE: [&str; 3] = ["clean", "residual", "security-hold"];

/// A table where `review` may answer every word a review panel ranks.
fn review_policy() -> Value {
    let rule = |id: &str, result: &str| json!({"id": id, "from": "review", "result": result, "next": "done", "reason": id});
    let mut table = policy();
    let rules = table["rules"].as_array_mut().unwrap();
    rules.push(rule("REVIEW-RESIDUAL", "residual"));
    rules.push(rule("REVIEW-HOLD", "security-hold"));
    table
}

/// The fixture with `listed`, an office declaring `results`, and `plain`,
/// one declaring none.
fn offices() -> AgentFixture {
    let fixture = AgentFixture::new();
    for (name, results) in [("listed", Some(THREE.to_vec())), ("plain", None)] {
        let mut office = json!({"description": name, "charter": "charters/work.md",
                                "models": ["opus"], "efforts": {"opus": "high"},
                                "tools": {"allow": ["cargo"]}});
        if let Some(results) = results {
            office["results"] = json!(results);
        }
        fixture.write(&format!("agents/{name}.json"), office);
    }
    fixture
}

/// A `review` seat that is a panel of `members` under `aggregate`.
fn panelled(fixture: &AgentFixture, aggregate: &str, members: Value) -> Value {
    let mut config = fixture.config();
    let results = if aggregate == "review-panel" {
        json!(THREE)
    } else {
        json!(["pass", "fail"])
    };
    config["seats"]["review"] =
        json!({"results": results, "aggregate": aggregate, "panel": members});
    config
}

/// Each member's compiled result set, by name.
fn laws(bundle: &Bundle) -> Vec<(String, Vec<String>)> {
    let SeatBody::Invocation(Invocation::Panel { members, .. }) = &bundle.seats["review"].body
    else {
        unreachable!("a panel seat")
    };
    (members.iter())
        .map(|member| (member.name.clone(), member.law.results.clone()))
        .collect()
}

/// The two offices as panel members.
fn two_members() -> Value {
    json!({"listed": {"agent": "listed"}, "plain": {"agent": "plain"}})
}

/// A table where `review` may answer `pass` and `fail`.
fn pass_fail_policy() -> Value {
    let mut table = policy();
    table["rules"].as_array_mut().unwrap().extend([
        json!({"id": "PASS", "from": "review", "result": "pass", "next": "done", "reason": "p"}),
        json!({"id": "FAIL", "from": "review", "result": "fail", "next": "done", "reason": "f"}),
    ]);
    table
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

#[test]
fn a_review_panel_member_is_held_to_its_agents_list_or_the_narrow_default() {
    let fixture = offices();
    let inline = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    let members =
        json!({"listed": {"agent": "listed"}, "plain": {"agent": "plain"}, "inline": inline});
    let config = panelled(&fixture, "review-panel", members);
    let bundle = fixture
        .compile_with_policy(config, &review_policy())
        .unwrap();
    let narrow = words(&["clean", "residual"]);
    assert_eq!(
        laws(&bundle),
        [
            ("inline".to_string(), narrow.clone()),
            ("listed".to_string(), words(&THREE)),
            ("plain".to_string(), narrow),
        ]
    );
}

#[test]
fn a_unanimous_pass_member_keeps_pass_and_fail_whatever_its_agent_declares() {
    let fixture = offices();
    let members = two_members();
    let table = pass_fail_policy();
    let bundle = fixture.compile_with_policy(panelled(&fixture, "unanimous-pass", members), &table);
    let pass_fail = words(&["pass", "fail"]);
    assert_eq!(
        laws(&bundle.unwrap()),
        [
            ("listed".to_string(), pass_fail.clone()),
            ("plain".to_string(), pass_fail)
        ]
    );
}

#[test]
fn a_members_own_results_key_is_refused_and_only_its_agent_declares_them() {
    let fixture = offices();
    let members =
        json!({"listed": {"agent": "listed", "results": ["clean"]}, "plain": {"agent": "plain"}});
    let config = panelled(&fixture, "review-panel", members);
    let refused = error(fixture.compile_with_policy(config, &review_policy()));
    assert!(
        refused.contains("seat 'review:listed' has unknown key 'results'"),
        "{refused}"
    );
}

#[test]
fn a_single_seats_results_lie_within_its_agents_list_and_a_list_less_agent_leaves_it() {
    let fixture = offices();
    let mut config = fixture.config();
    config["seats"]["review"] = json!({"results": ["clean"], "agent": "listed"});
    assert!(
        fixture.compile(config.clone()).is_ok(),
        "within the ceiling"
    );
    config["seats"]["work"] = json!({"results": ["complete"], "agent": "listed"});
    assert_eq!(
        error(fixture.compile(config.clone())),
        "bundle: seat 'work' declares result 'complete', which its hired agent's results \
         [clean, residual, security-hold] do not admit; an agent's results are the ceiling of \
         every seat that hires it (decision 0109 ruling 11)"
    );
    config["seats"]["work"] = json!({"results": ["complete"], "agent": "plain"});
    assert!(fixture.compile(config).is_ok(), "no list, no ceiling");
}

#[test]
fn a_selected_cases_results_lie_within_its_agents_list() {
    let fixture = offices();
    let mut config = fixture.config();
    config["seats"]["work"] = json!({"results": ["complete"], "select": {"on": "strategy",
        "cases": {"chore": {"agent": "listed"}}, "default": {"agent": "plain"}}});
    let refused = error(fixture.compile(config));
    assert!(
        refused.contains("seat 'work:chore' declares result 'complete', which its hired agent's"),
        "{refused}"
    );
}

/// The shipped panels compile every member to the list its agent declares:
/// the correctness and spec-compliance members to `clean` and `residual`,
/// the security and adversarial members, their gpt-flash overlays among
/// them, to the hold beside both, and the design `positions` members of
/// triage and gpt-flash, whose agents declare none, to `pass` and `fail`.
#[test]
fn the_shipped_members_compile_to_their_agents_lists() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = |recipe: &str| {
        let dir = root.join("recipes").join(recipe);
        Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters")).unwrap()
    };
    let narrow = words(&["clean", "residual"]);
    let expected = |member: &str| match member {
        "security" | "adversarial" => words(&THREE),
        "correctness" | "spec-compliance" => narrow.clone(),
        "simplicity" | "robustness" => words(&["pass", "fail"]),
        other => panic!("an unexpected shipped member '{other}'"),
    };
    let mut seen = 0;
    for recipe in ["triage", "gpt-flash", "panel-review"] {
        let bundle = compiled(recipe);
        let mut members = Vec::new();
        for seat in bundle.seats.values() {
            panels(&seat.body, &mut members);
        }
        for member in members {
            assert_eq!(
                member.law.results,
                expected(&member.name),
                "{recipe} {}",
                member.name
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 26, "every shipped panel member");
}

/// #1221 A: a `unanimous-pass` panel writes no residual pair, so under a
/// seat whose input set declares one it could never pass ruling 2's
/// aggregate arm. It is refused at compile in each position a panel may
/// take, naming the panel's site; a review-panel under the same seat
/// compiles.
#[test]
fn a_unanimous_pass_panel_under_a_residual_seat_is_refused_where_it_stands() {
    let fixture = offices();
    let members = two_members();
    let panel = json!({"aggregate": "unanimous-pass", "panel": members.clone()});
    let table = pass_fail_policy();
    let single = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    let mut chief = single.clone();
    chief["name"] = json!("chief");
    let mut step = panel.clone();
    step["name"] = json!("positions");
    let select = json!({"on": "strategy", "cases": {"chore": panel.clone()}, "default": single});
    let bodies = [
        (panel, "review"),
        (json!({"sequence": [step, chief]}), "review:positions"),
        (json!({"select": select}), "review:chore"),
    ];
    for (body, site) in bodies {
        for input in ["max_residual_severity", "has_security_residual"] {
            let mut seat = body.clone();
            seat["results"] = json!(["pass", "fail"]);
            seat["inputs"] = json!([input]);
            let mut config = fixture.config();
            config["seats"]["review"] = seat;
            match fixture.compile_with_policy(config, &table) {
                Err(CompileError::Body(refusal)) => assert_eq!(
                    refusal,
                    crate::bundle::BodyRefusal::UnanimousPassUnderResidual { panel: site.into() },
                    "{site} under {input}"
                ),
                other => panic!("{site} under {input}: the refusal, not {:?}", other.err()),
            }
        }
    }
    assert_eq!(
        crate::bundle::BodyRefusal::UnanimousPassUnderResidual {
            panel: "review".into()
        }
        .to_string(),
        "panel 'review' joins by 'unanimous-pass', which writes no residual pair, under a seat \
         whose input set declares 'max_residual_severity' or 'has_security_residual'; its pass \
         could never stand (decision 0109 ruling 2): join it by 'review-panel' or drop the pair \
         from the seat's inputs"
    );

    let mut config = panelled(&fixture, "review-panel", members);
    config["seats"]["review"]["inputs"] = json!(["max_residual_severity", "has_security_residual"]);
    let bundle = (fixture.compile_with_policy(config, &review_policy()))
        .expect("a review-panel binds the pair");
    assert_eq!(
        bundle.seats["review"].inputs,
        ["max_residual_severity", "has_security_residual"]
    );
}

/// Every panel member `body` can run, whatever contains it.
fn panels<'a>(body: &'a SeatBody, out: &mut Vec<&'a PanelMember>) {
    match body {
        SeatBody::Invocation(Invocation::Panel { members, .. }) => out.extend(members),
        SeatBody::Invocation(Invocation::Single { .. }) => {}
        SeatBody::Sequence { steps } => {
            for step in steps {
                if let StepBody::Invocation(Invocation::Panel { members, .. }) = &step.body {
                    out.extend(members);
                }
            }
        }
        SeatBody::Select { cases, default } => {
            for case in cases.values().chain(default.as_deref()) {
                panels(case, out);
            }
        }
    }
}
