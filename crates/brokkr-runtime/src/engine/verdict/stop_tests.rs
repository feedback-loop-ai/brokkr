//! Decision 0050 rulings 5 and 7 through the real engine (#1221, #1212):
//! a protected sequence's work step before every gate step ends the run
//! on a compiled hard word, and decide takes that stop on no head. The
//! shipped `recipes/gpt-flash` is compiled and run with only its drivers
//! scripted.

use std::path::PathBuf;

use serde_json::{json, Value};

use super::tests::{driven_with, driver};
use crate::bundle::{Bundle, Invocation, Limits, SeatBody, SeatClass, SequenceStep, StepBody};
use crate::engine::tests::single_body;
use brokkr_core::envelope::{EventEnvelope, EventType};

/// The shipped recipe `name`, compiled against the shipped library.
pub(super) fn shipped(name: &str) -> Bundle {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("recipes").join(name);
    Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters")).unwrap()
}

/// The compiled steps of `recipe`'s `seat`, under `case` where it selects.
pub(super) fn steps_of(recipe: &Bundle, seat: &str, case: Option<&str>) -> Vec<SequenceStep> {
    let mut body = &recipe.seats[seat].body;
    if let (SeatBody::Select { cases, .. }, Some(case)) = (body, case) {
        body = &cases[case];
    }
    let SeatBody::Sequence { steps } = body else {
        panic!("{seat} is a sequence")
    };
    steps.clone()
}

/// `phase`'s journaled decisions, in order.
pub(super) fn decided(events: &[EventEnvelope], phase: &str) -> Vec<Value> {
    (events.iter())
        .filter(|event| event.event_type == EventType::TransitionDecided)
        .filter(|event| event.payload["from"] == phase)
        .map(|event| event.payload.clone())
        .collect()
}

/// The clean every member charter orders.
fn clean() -> Value {
    json!({"result": "clean", "inputs": {"max_residual_severity": "none",
                                         "has_security_residual": false}})
}

/// `step`'s drivers scripted in place of its models, its compiled law,
/// class and endings unchanged: each panel member answers `member(name)`
/// and a single step the charter-shaped clean.
fn scripted(step: &mut SequenceStep, member: &impl Fn(&str) -> Value) {
    match &mut step.body {
        StepBody::Invocation(Invocation::Panel { members, .. }) => {
            for each in members {
                each.command = driver(&member(&each.name));
                each.role_path = "role.md".into();
                each.candidates = Vec::new();
            }
        }
        StepBody::Invocation(Invocation::Single {
            command,
            role_path,
            candidates,
            ..
        }) => {
            *command = driver(&clean());
            *role_path = "role.md".into();
            candidates.clear();
        }
        StepBody::Dialect { .. } => panic!("{} runs a driver", step.name),
    }
}

/// A run of gpt-flash's compiled review seat under its compiled table and
/// protected phase, every driver scripted: each `positions` member answers
/// `member(name)` and the chief the charter-shaped clean. Triage answers
/// `chore`, which the table routes straight to review here. The work seat
/// keeps the default timeout, so a loaded host never times its scripted
/// driver out before review.
fn gpt_flash_reviewed(member: impl Fn(&str) -> Value) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let recipe = shipped("gpt-flash");
    let mut review = recipe.seats["review"].clone();
    let SeatBody::Select { cases, .. } = &mut review.body else {
        panic!("gpt-flash's review selects on the strategy")
    };
    for case in cases.values_mut() {
        let SeatBody::Sequence { steps } = case else {
            panic!("every gpt-flash review case is a sequence")
        };
        steps.iter_mut().for_each(|step| scripted(step, &member));
    }
    let triage = single_body(driver(&json!({"result": "chore"})));
    driven_with(triage, |compiled| {
        let mut triage = compiled.seats.remove("work").unwrap();
        triage.results = vec!["chore".into()];
        triage.limits.timeout_seconds = Limits::default().timeout_seconds;
        compiled.seats.insert("triage".into(), triage);
        compiled.seats.insert("review".into(), review);
        compiled.machine = recipe.machine.clone();
        let chore = (compiled.machine.rules.iter_mut())
            .find(|rule| rule.id == "TRIAGE-CHORE")
            .unwrap();
        chore.next = Some("review".into());
        compiled.protected_phase = recipe.protected_phase.clone();
    })
}

/// Whether `events` journal a checkpoint of the step `name`.
fn ran(events: &[EventEnvelope], name: &str) -> bool {
    (events.iter()).any(|event| event.payload["checkpoint"]["step_name"] == name)
}

/// gpt-flash protects review, whose `positions` is a work-class panel
/// before its chief, the gate. A member's `security-hold` is the panel's
/// word and a compiled hard ending of `positions`, so the run rules
/// REVIEW-SECURITY-HOLD and stops: it never parks, never runs the chief,
/// records no reviewed head and never requests ship.
#[test]
fn the_gpt_flash_positions_security_hold_stops_the_run_before_its_chief() {
    let positions = &steps_of(&shipped("gpt-flash"), "review", Some("chore"))[0];
    assert_eq!(positions.class, SeatClass::Work);
    assert!(positions.ends_on.contains(&"security-hold".to_string()));
    assert!(positions
        .hard_results
        .contains(&"security-hold".to_string()));
    // A work-class member reports no findings (ruling 1): its hold stands
    // over the high security residual its declared pair names.
    let held = json!({"result": "security-hold", "inputs": {
        "max_residual_severity": "high", "has_security_residual": true}});
    let (_dir, events) = gpt_flash_reviewed(|name| match name {
        "security" => held.clone(),
        _ => clean(),
    });
    let review = &decided(&events, "review")[0];
    assert_eq!(review["result"], "security-hold", "{review}");
    assert_eq!(review["rule_id"], "REVIEW-SECURITY-HOLD", "{review}");
    assert_eq!(review["next"], "stop", "{review}");
    assert_eq!(review["problem"], Value::Null, "{review}");
    assert_eq!(review["inputs"].get("reviewed_heads"), None, "{review}");
    assert!(!ran(&events, "chief"), "the chief never ran");
    let requested_ship = (events.iter()).any(|event| {
        event.event_type == EventType::EffectRequested && event.payload["phase"] == "ship"
    });
    assert!(!requested_ship, "ship is never requested");
    assert!(!(events.iter()).any(|event| event.event_type == EventType::RunParked));
    let last = events.last().unwrap();
    assert_eq!(last.event_type, EventType::RunStopped, "{}", last.payload);
    assert_eq!(
        last.payload["reason"],
        "hard stop ruled by REVIEW-SECURITY-HOLD"
    );
}

/// The same gpt-flash body with every member clean reaches its chief and
/// decides on the chief's gate record, as before.
#[test]
fn clean_gpt_flash_positions_reach_the_chief_and_decide_on_its_record() {
    let (_dir, events) = gpt_flash_reviewed(|_| clean());
    let review = &decided(&events, "review")[0];
    assert_eq!(review["result"], "clean", "{review}");
    assert_eq!(review["rule_id"], "REVIEW-CLEAN", "{review}");
    assert_eq!(review["problem"], Value::Null, "{review}");
    let chief = (events.iter())
        .map(|event| &event.payload["checkpoint"])
        .find(|checkpoint| checkpoint["step_name"] == "chief")
        .expect("the chief ran");
    let head = chief["result"]["checked_head"].as_str().expect("a head");
    assert_eq!(review["inputs"]["reviewed_heads"], json!({"repo": head}));
}
