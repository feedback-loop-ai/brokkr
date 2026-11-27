//! Decision 0109 rulings 2 and 11 through the real engine: a seat's word
//! and every review-panel member's word are bound to their findings and
//! their declared residual pair, a member's lawful words are its compiled
//! law, and a refusal parks or fails with the evidence journaled, never a
//! rewritten word. The shipped panels are compiled from `recipes/` and run
//! with their compiled laws.

use serde_json::{json, Value};

use super::stop_tests::{clean, decided, shipped, steps_of};
use super::tests::{driven, driven_with, driver};
use crate::bundle::{
    Aggregate, Bundle, GateAssessment, Invocation, MemberLaw, PanelMember, SeatBody, SeatClass,
    SequenceStep, StepBody,
};
use crate::engine::tests::{own_engine, single_body, step_body};
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::findings::ResidualPair;
use brokkr_core::policy::Machine;

/// The compiled members of a panel step, each answering `answer(name)`
/// through a scripted driver in place of its model, its law unchanged.
fn answering(step: &SequenceStep, answer: impl Fn(&str) -> Value) -> (Vec<PanelMember>, Aggregate) {
    let StepBody::Invocation(Invocation::Panel { members, aggregate }) = &step.body else {
        panic!("{} is a panel step", step.name)
    };
    let members = (members.iter())
        .map(|member| PanelMember {
            role_path: "role.md".into(),
            command: driver(&answer(&member.name)),
            candidates: Vec::new(),
            ..member.clone()
        })
        .collect();
    (members, *aggregate)
}

/// A table whose review answers every word a review panel ranks.
fn review_table() -> Machine {
    Machine::from_table(&json!({
        "phases": ["work", "review", "ship", "done", "held"],
        "initial": "work",
        "terminal": ["done", "held"],
        "rules": [
            {"id": "WORK", "from": "work", "result": "complete", "next": "review", "reason": "work"},
            {"id": "REVIEW", "from": "review", "result": "clean", "next": "ship", "reason": "review"},
            {"id": "REVIEW-RESIDUAL", "from": "review", "result": "residual", "next": "held",
             "reason": "residual"},
            {"id": "REVIEW-HOLD", "from": "review", "result": "security-hold", "next": "held",
             "reason": "hold"},
            {"id": "SHIP", "from": "ship", "result": "shipped", "next": "done", "reason": "ship"}
        ]
    }))
    .unwrap()
}

/// The bundle reshaped so `review` is a gate of `body` compiling `inputs`,
/// answering every ranked word, before a ship that ships.
fn reviewing(compiled: &mut Bundle, body: SeatBody, inputs: &[&str]) {
    compiled.machine = review_table();
    let review = compiled.seats.get_mut("review").unwrap();
    review.body = body;
    review.gate = GateAssessment::AllGate;
    review.results = ["clean", "residual", "security-hold"]
        .map(String::from)
        .to_vec();
    review.inputs = inputs.iter().map(|input| input.to_string()).collect();
    let ship = compiled.seats.get_mut("ship").unwrap();
    ship.body = single_body(driver(&json!({"result": "shipped"})));
}

/// A run whose review is `body`, compiling `inputs`, driven to its stop.
fn reviewed(body: SeatBody, inputs: &[&str]) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let work = single_body(driver(&json!({"result": "complete"})));
    driven_with(work, |compiled| reviewing(compiled, body, inputs))
}

const PAIR: [&str; 2] = ["max_residual_severity", "has_security_residual"];

/// Ruling 11: the correctness and spec-compliance members compiled from
/// triage's engine case answer the charter-shaped clean, and so do the
/// security and adversarial members; the panel is admitted and ships.
#[test]
fn the_shipped_engine_panel_compiled_and_run_admits_every_members_clean() {
    own_engine!();
    let steps = steps_of(&shipped("triage"), "review", Some("engine"));
    let (members, aggregate) = answering(&steps[0], |_| clean());
    assert_eq!(members.len(), 4);
    let panel = SeatBody::Invocation(Invocation::Panel { members, aggregate });
    let (_dir, events) = reviewed(panel, &PAIR);
    let review = &decided(&events, "review")[0];
    assert_eq!(review["result"], "clean", "{review}");
    assert_eq!(review["problem"], Value::Null, "{review}");
    assert_eq!(decided(&events, "ship")[0]["result"], "shipped");
}

/// Ruling 11: the correctness member's agent lists `clean` and `residual`,
/// and a member whose agent declares no list compiles to the same narrow
/// pair; either one's `security-hold` poisons the panel, which parks with
/// the member's evidence and the reason it was refused.
#[test]
fn a_hold_outside_a_members_list_or_the_narrow_default_is_refused_and_parks() {
    own_engine!();
    let steps = steps_of(&shipped("triage"), "review", Some("engine"));
    let listless = Aggregate::ReviewPanel.member_results(None);
    for law in [None, Some(listless)] {
        let (mut members, aggregate) = answering(&steps[0], |_| clean());
        let held = (members.iter_mut())
            .find(|member| member.name == "correctness")
            .unwrap();
        held.command = driver(&json!({"result": "security-hold"}));
        if let Some(results) = law.clone() {
            held.law = MemberLaw {
                results,
                ..held.law.clone()
            };
        }
        let panel = SeatBody::Invocation(Invocation::Panel { members, aggregate });
        let (_dir, events) = reviewed(panel, &PAIR);
        let review = &decided(&events, "review")[0];
        assert_eq!(review["result"], "__schema-invalid__", "{review}");
        let raw = &review["inputs"]["raw_result"];
        assert_eq!(raw["result"], "__member-schema-invalid__");
        assert_eq!(
            raw["notes"]["refused"]["correctness"],
            "member result 'security-hold' is not among its declared results [clean, residual]"
        );
        let evidence = &raw["notes"]["members"]["correctness"];
        assert_eq!(evidence["result"], "security-hold");
        assert!(decided(&events, "ship").is_empty(), "never shipped");
    }
}

/// Ruling 11: triage's and gpt-flash's design `positions`, compiled with
/// their list-less agents under `unanimous-pass`, each return `pass` from
/// both members, and the sequence reaches its chief.
#[test]
fn the_shipped_design_positions_pass_and_reach_their_chief() {
    own_engine!();
    for recipe in ["triage", "gpt-flash"] {
        let steps = steps_of(&shipped(recipe), "design", None);
        let (members, aggregate) = answering(&steps[0], |_| json!({"result": "pass"}));
        assert_eq!(aggregate, Aggregate::UnanimousPass, "{recipe}");
        let positions = SequenceStep {
            body: StepBody::Invocation(Invocation::Panel { members, aggregate }),
            ..steps[0].clone()
        };
        let chief = uncompiled_sequence_step! {
            name: "chief".into(),
            class: SeatClass::Work,
            results: vec!["complete".into()],
            body: step_body(driver(&json!({"result": "complete"}))),
        };
        let work = SeatBody::Sequence {
            steps: vec![positions, chief],
        };
        let (_dir, events) = driven(work, false);
        let work = &decided(&events, "work")[0];
        assert_eq!(
            work["result"], "complete",
            "{recipe}: the chief answered: {work}"
        );
        let joined = (events.iter())
            .map(|event| &event.payload["checkpoint"])
            .find(|checkpoint| checkpoint["step_name"] == "positions")
            .unwrap_or_else(|| panic!("{recipe}: the positions step finished"));
        assert_eq!(joined["result"]["result"], "pass", "{recipe}: {joined}");
    }
}

/// A finding a member reports, `security` or not, at `severity`.
fn finding(id: &str, severity: &str, security: bool) -> Value {
    json!({"id": id, "file": "crates/a/src/lib.rs", "line": 3, "summary": "a finding",
           "severity": severity, "security": security})
}

/// Triage's engine review panel, compiled over its seat's `spec_defect`
/// and residual pair, each member answering `answer(name)`, as the
/// fixture's review seat compiling the same three inputs.
fn triage_panel(answer: impl Fn(&str) -> Value) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let steps = steps_of(&shipped("triage"), "review", Some("engine"));
    let (members, aggregate) = answering(&steps[0], answer);
    let panel = SeatBody::Invocation(Invocation::Panel { members, aggregate });
    reviewed(panel, &["spec_defect", PAIR[0], PAIR[1]])
}

/// The poisoned review's refusal of `member`, from the parked decision.
fn refusal_of(events: &[EventEnvelope], member: &str) -> Value {
    let review = &decided(events, "review")[0];
    assert_eq!(review["result"], "__schema-invalid__", "{review}");
    assert!(decided(events, "ship").is_empty(), "never shipped");
    review["inputs"]["raw_result"]["notes"]["refused"][member].clone()
}

/// Ruling 2: an all-clean panel's aggregate carries both residual inputs,
/// `none` and `false`, and a triage-shaped panel whose four members write
/// only the pair, over a seat that also compiles `spec_defect`, is
/// admitted: the member is held to the pair alone.
#[test]
fn a_triage_shaped_all_clean_panel_carries_both_inputs_and_is_admitted() {
    own_engine!();
    let (_dir, events) = triage_panel(|_| clean());
    let review = &decided(&events, "review")[0];
    assert_eq!(review["result"], "clean", "{review}");
    assert_eq!(review["problem"], Value::Null, "{review}");
    assert_eq!(review["inputs"]["max_residual_severity"], "none");
    assert_eq!(review["inputs"]["has_security_residual"], false);
    assert_eq!(review["inputs"]["spec_defect"], false);
}

/// Ruling 2: a member that omits a residual input its seat declares is
/// refused, and its word never reaches the join.
#[test]
fn a_member_omitting_a_residual_input_is_refused_before_the_join() {
    own_engine!();
    let (_dir, events) = triage_panel(|name| match name {
        "spec-compliance" => {
            json!({"result": "clean", "inputs": {"max_residual_severity": "none"}})
        }
        _ => clean(),
    });
    assert_eq!(
        refusal_of(&events, "spec-compliance"),
        "the passing word 'clean' omits 'has_security_residual', which the seat declares \
         (decision 0109 ruling 2)"
    );
}

/// The answer of a member reporting `word` over one high finding, flagged
/// `security` as the declared pair says.
fn high(word: &str, security: bool) -> Value {
    json!({"result": word, "findings": [finding("leak", "high", security)],
           "inputs": {"max_residual_severity": "high", "has_security_residual": security}})
}

/// Ruling 2: a correctness member's security finding has no lawful word,
/// because its agent lists no `security-hold`, so the panel parks.
#[test]
fn a_correctness_members_security_finding_is_refused_and_parks() {
    own_engine!();
    let (_dir, events) = triage_panel(|name| match name {
        "correctness" => high("residual", true),
        _ => clean(),
    });
    assert_eq!(
        refusal_of(&events, "correctness"),
        "the word 'residual' stands over a security finding, which requires 'security-hold' \
         (decision 0109 ruling 2)"
    );
}

/// Rulings 2 and 11: the security member's `security-hold` over a high
/// security finding is lawful, and the panel's word is the hold.
#[test]
fn the_security_members_hold_over_a_high_security_finding_is_admitted() {
    own_engine!();
    let (_dir, events) = triage_panel(|name| match name {
        "security" => high("security-hold", true),
        _ => clean(),
    });
    let review = &decided(&events, "review")[0];
    assert_eq!(review["result"], "security-hold", "{review}");
    assert_eq!(review["rule_id"], "REVIEW-HOLD", "{review}");
    assert_eq!(review["inputs"]["has_security_residual"], true);
}

/// Ruling 2: a review-panel member is bound whatever its class, so
/// gpt-flash's work-class member that writes `clean` over a declared high
/// residual is refused.
#[test]
fn a_work_class_members_clean_over_a_declared_high_residual_is_refused() {
    own_engine!();
    let steps = steps_of(&shipped("gpt-flash"), "review", Some("engine"));
    let (members, aggregate) = answering(&steps[0], |name| match name {
        "correctness" => json!({"result": "clean", "inputs": {
            "max_residual_severity": "high", "has_security_residual": false}}),
        _ => clean(),
    });
    assert!(members.iter().all(|member| member.class == SeatClass::Work));
    let panel = SeatBody::Invocation(Invocation::Panel { members, aggregate });
    let (_dir, events) = reviewed(panel, &PAIR);
    assert_eq!(
        refusal_of(&events, "correctness"),
        "the passing word 'clean' stands over a declared max_residual_severity of 'high' \
         (decision 0109 ruling 2)"
    );
}

/// A single gate seat answering `answer`.
fn gate(answer: &Value) -> SeatBody {
    SeatBody::Invocation(Invocation::Single {
        role_path: "role.md".into(),
        command: driver(answer),
        candidates: Vec::new(),
        written: Vec::new(),
        class: SeatClass::Gate,
    })
}

/// The review's decision when its single gate seat, compiling `inputs`,
/// answers `answer`.
fn decided_on(answer: Value, inputs: &[&str]) -> (Value, Vec<EventEnvelope>) {
    let (_dir, events) = reviewed(gate(&answer), inputs);
    (decided(&events, "review")[0].clone(), events)
}

/// The refusal decide journaled for the review's word, which parked.
fn refused_word(answer: Value, inputs: &[&str]) -> String {
    let (review, events) = decided_on(answer, inputs);
    assert_eq!(review["result"], "__schema-invalid__", "{review}");
    assert!(decided(&events, "ship").is_empty(), "never shipped");
    let parked = events.last().unwrap();
    assert_eq!(
        parked.event_type,
        EventType::RunParked,
        "{}",
        parked.payload
    );
    review["problem"].as_str().unwrap().to_string()
}

/// decide's prefix for a refused word.
fn word_refused(refusal: &str) -> String {
    format!(
        "result failed schema validation: the seat's word was refused: {refusal} (decision 0109 \
         ruling 2)"
    )
}

/// Ruling 2 at decide: a gate's passing word is refused over a declared
/// residual, a security residual, an omitted residual input, and a high
/// security finding, and decide journals the refusal and parks; the word
/// is never rewritten.
#[test]
fn decide_refuses_a_gates_clean_over_its_declared_residual_or_its_security_finding() {
    own_engine!();
    let pair = |severity: &str, security: bool| json!({"max_residual_severity": severity, "has_security_residual": security});
    let rows = [
        (
            json!({"result": "clean", "inputs": pair("high", false)}),
            "the passing word 'clean' stands over a declared max_residual_severity of 'high'",
        ),
        (
            json!({"result": "clean", "inputs": pair("none", true)}),
            "the passing word 'clean' stands over a declared has_security_residual of true",
        ),
        (
            json!({"result": "clean", "inputs": {"max_residual_severity": "none"}}),
            "the passing word 'clean' omits 'has_security_residual', which the seat declares",
        ),
        (
            json!({"result": "clean", "inputs": pair("none", false),
                   "findings": [finding("leak", "high", true)]}),
            "the word 'clean' stands over a security finding, which requires 'security-hold'",
        ),
    ];
    for (answer, refusal) in rows {
        assert_eq!(
            refused_word(answer.clone(), &PAIR),
            word_refused(refusal),
            "{answer}"
        );
    }
}

/// Ruling 2 at decide: a clean from a seat that declares no residual input
/// stands as today; the hold over a high security finding and a residual
/// over a medium finding are lawful words; and a work seat's word is not a
/// verdict, whatever it declares.
#[test]
fn decide_admits_every_lawful_word_and_leaves_a_work_seats_word_alone() {
    own_engine!();
    let (review, _) = decided_on(json!({"result": "clean"}), &[]);
    assert_eq!(
        (review["result"].as_str(), review["rule_id"].as_str()),
        (Some("clean"), Some("REVIEW"))
    );
    let held = json!({"result": "security-hold", "findings": [finding("leak", "high", true)],
                      "inputs": {"max_residual_severity": "high", "has_security_residual": true}});
    let (review, _) = decided_on(held, &PAIR);
    assert_eq!(review["rule_id"], "REVIEW-HOLD", "{review}");
    let residual = json!({"result": "residual", "findings": [finding("len", "medium", false)],
                          "inputs": {"max_residual_severity": "medium",
                                     "has_security_residual": false}});
    let (review, _) = decided_on(residual, &PAIR);
    assert_eq!(review["rule_id"], "REVIEW-RESIDUAL", "{review}");
    let work = single_body(driver(&json!({"result": "complete"})));
    let (_dir, events) = driven_with(work, |compiled| {
        reviewing(compiled, gate(&clean()), &PAIR);
        let work = compiled.seats.get_mut("work").unwrap();
        work.inputs = PAIR.map(String::from).to_vec();
    });
    assert_eq!(
        decided(&events, "work")[0]["rule_id"],
        "WORK",
        "a work seat's word stands"
    );
}

/// Ruling 2 at a non-final gate step: a refused word fails the attempt at
/// that step, as its vocabulary refusal does (decision 0006), and no
/// later step runs; the attempt cap then parks the run, and no word is
/// rewritten.
#[test]
fn a_non_final_gate_steps_refused_word_fails_the_attempt_and_no_later_step_runs() {
    own_engine!();
    // Every compiled step carries its seat's pair: triage's review steps
    // carry the review seat's.
    let pair = ResidualPair::declared_in(&PAIR);
    let compiled = steps_of(&shipped("triage"), "review", Some("engine"));
    assert!(compiled.iter().all(|step| step.residual == pair));
    let judge = SequenceStep {
        residual: pair,
        ..uncompiled_sequence_step! {
            name: "judge".into(),
            class: SeatClass::Gate,
            results: vec!["clean".into(), "residual".into()],
            body: step_body(driver(&json!({"result": "clean", "inputs": {
                "max_residual_severity": "high", "has_security_residual": false}}))),
        }
    };
    let chief = uncompiled_sequence_step! {
        name: "chief".into(),
        class: SeatClass::Gate,
        results: vec!["clean".into()],
        body: step_body(driver(&clean())),
    };
    let review = SeatBody::Sequence {
        steps: vec![judge, chief],
    };
    let (_dir, events) = reviewed(review, &PAIR);
    let failed: Vec<&Value> = (events.iter())
        .filter(|event| event.event_type == EventType::EffectFailed)
        .map(|event| &event.payload)
        .collect();
    assert!(!failed.is_empty(), "the attempt failed");
    for payload in &failed {
        assert_eq!(
            payload["error"],
            "sequence step 'judge': reported a word that was refused: the passing word 'clean' \
             stands over a declared max_residual_severity of 'high' (decision 0109 ruling 2); \
             driver stderr: empty",
            "{payload}"
        );
    }
    let chief_ran =
        (events.iter()).any(|event| event.payload["checkpoint"]["step_name"] == "chief");
    assert!(!chief_ran, "no later step ran");
    assert!(
        decided(&events, "review").is_empty(),
        "the word never reached a rule"
    );
    assert_eq!(events.last().unwrap().event_type, EventType::RunParked);
}
