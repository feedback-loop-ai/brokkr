//! A clarify-exhausted park's questions and the run's working tree
//! (#568, design D7): present where the journal says so, absent
//! everywhere else, never guessed.

use super::*;
use crate::fleet::tests::ruled;
use crate::tests::{ev, state};
use serde_json::{json, Value};

const T: &str = "2026-10-13T10:00:00Z";

/// A journal whose last succeeded effect is at seq 4.
fn judged() -> Vec<EventEnvelope> {
    vec![
        ev(1, EventType::RunStarted, json!({"feature": "f"}), T),
        ev(2, EventType::PhaseEntered, json!({"phase": "clarify"}), T),
        ev(3, EventType::EffectSucceeded, json!({"effect_id": "e0"}), T),
        ev(4, EventType::EffectSucceeded, json!({"effect_id": "e1"}), T),
        ev(5, EventType::TransitionDecided, json!({}), T),
        ev(6, EventType::RunParked, json!({}), T),
    ]
}

/// A run parked by `decision`, its last result carrying `notes`.
fn parked(phase: &str, decision: Option<Value>, notes: Value) -> RunState {
    let mut parked = state(Some(phase), Status::AwaitingOperator, decision);
    parked.last_result = Some(json!({"result": "ambiguous", "notes": notes, "model": "m"}));
    parked
}

fn exhausted() -> Option<Value> {
    ruled("CLARIFY-AMBIGUOUS-EXHAUSTED", "clarify", None)
}

#[test]
fn a_clarify_exhausted_park_carries_its_questions_in_recorded_order() {
    let text = parked(
        "clarify",
        exhausted(),
        json!("1. Which target counts?\n\n  2. Does search include reindexing?  \n"),
    );
    assert_eq!(
        clarify(&judged(), &text),
        Some(Clarify {
            seq: 4,
            questions: vec![
                "1. Which target counts?".to_string(),
                "2. Does search include reindexing?".to_string(),
            ],
        })
    );
    let listed = parked("clarify", exhausted(), json!(["Second?", " ", "First?"]));
    assert_eq!(
        clarify(&judged(), &listed).map(|found| found.questions),
        Some(vec!["Second?".to_string(), "First?".to_string()])
    );
    let unreadable = parked("clarify", exhausted(), json!({"q": 1}));
    assert_eq!(
        clarify(&judged(), &unreadable),
        Some(Clarify {
            seq: 4,
            questions: Vec::new()
        }),
        "notes in neither shape are no questions, never a guess"
    );
}

#[test]
fn every_other_park_and_a_running_run_carry_no_questions() {
    let at_review = parked(
        "review",
        ruled("REVIEW-HOLD", "review", None),
        json!("Is this real?"),
    );
    assert_eq!(clarify(&judged(), &at_review), None);
    let routed_on = parked(
        "clarify",
        ruled("CLARIFY-AMBIGUOUS", "clarify", Some("specify")),
        json!("Which?"),
    );
    assert_eq!(
        clarify(&judged(), &routed_on),
        None,
        "a park after a ruling that routed on is an attempt limit, not clarify's"
    );
    let mut running = parked("clarify", exhausted(), json!("Which?"));
    running.status = Status::Running;
    assert_eq!(clarify(&judged(), &running), None);
}

/// A journal whose `run/started` pins `pin` as its world.
fn started(pin: Value) -> Vec<EventEnvelope> {
    vec![ev(
        1,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {"realms": pin}}),
        T,
    )]
}

fn pin(source: &str, realm: Option<&str>, path: &str) -> Value {
    let mut pin = json!({
        "source": source,
        "sha256": "0",
        "map": {"realms": [{"name": "other", "path": "elsewhere"}, {"name": "alpha", "path": path}]},
    });
    if let Some(realm) = realm {
        pin["realm"] = json!(realm);
    }
    pin
}

#[test]
fn the_worktree_is_the_pinned_realms_path_resolved_against_the_map() {
    let relative = started(pin("/work/realms.json", Some("alpha"), "repos/alpha"));
    assert_eq!(worktree(&relative).as_deref(), Some("/work/repos/alpha"));
    let absolute = started(pin("/work/realms.json", Some("alpha"), "/srv/alpha"));
    assert_eq!(worktree(&absolute).as_deref(), Some("/srv/alpha"));
    assert_eq!(pinned_realm(&relative).as_deref(), Some("alpha"));
}

#[test]
fn a_worktree_the_journal_does_not_name_is_absent() {
    let unmapped = vec![ev(1, EventType::RunStarted, json!({"manifest": {}}), T)];
    assert_eq!(worktree(&unmapped), None, "no world pinned");
    assert_eq!(worktree(&[]), None, "no run/started");
    let realmless = started(pin("/work/realms.json", None, "repos/alpha"));
    assert_eq!(worktree(&realmless), None, "no realm pinned");
    let unknown = started(pin("/work/realms.json", Some("gamma"), "repos/alpha"));
    assert_eq!(worktree(&unknown), None, "a realm the map lacks");
    let bare = started(pin("realms.json", Some("alpha"), "repos/alpha"));
    assert_eq!(worktree(&bare), None, "a relative path is never guessed");
    let nested = started(pin("work/realms.json", Some("alpha"), "repos/alpha"));
    assert_eq!(worktree(&nested), None, "nor one under a relative map");
    let root = started(pin("/realms.json", Some("alpha"), "repos/alpha"));
    assert_eq!(worktree(&root).as_deref(), Some("/repos/alpha"));
    let half = started(json!({"source": "/work/realms.json", "realm": "alpha"}));
    assert_eq!(worktree(&half), None, "a pin with no map");
    let second = vec![
        ev(1, EventType::PhaseEntered, json!({}), T),
        started(pin("/work/realms.json", Some("alpha"), "a")).remove(0),
    ];
    assert_eq!(worktree(&second), None, "only the first event starts a run");
}

#[test]
fn the_recipe_is_the_bundle_name_the_start_recorded() {
    let named = vec![ev(
        1,
        EventType::RunStarted,
        json!({"manifest": {"bundle_name": "triage"}}),
        T,
    )];
    assert_eq!(recipe(&named).as_deref(), Some("triage"));
    let unnamed = vec![ev(1, EventType::RunStarted, json!({"manifest": {}}), T)];
    assert_eq!(recipe(&unnamed), None, "a start that names no recipe");
    assert_eq!(recipe(&[]), None, "no run/started");
    let second = vec![
        ev(
            1,
            EventType::PhaseEntered,
            json!({"manifest": {"bundle_name": "x"}}),
            T,
        ),
        ev(
            2,
            EventType::RunStarted,
            json!({"manifest": {"bundle_name": "y"}}),
            T,
        ),
    ];
    assert_eq!(
        recipe(&second),
        None,
        "only a first event that starts the run"
    );
    let malformed = vec![ev(1, EventType::RunStarted, json!({"manifest": "nope"}), T)];
    assert_eq!(recipe(&malformed), None, "a manifest that does not read");
}
