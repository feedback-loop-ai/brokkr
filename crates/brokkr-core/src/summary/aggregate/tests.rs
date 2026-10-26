use super::*;
use crate::decision::Decision;
use crate::fold::Run;
use crate::run_state::run_state;
use crate::summary::tests::journal;
use crate::summary::{Account, Listing, Summary};
use serde_json::json;

/// The time of the event at `seq` in a [`journal`].
fn second(seq: i64) -> i64 {
    1_767_225_600_000 + seq * 1000
}

/// A start whose manifest pins realm `alpha` of a map and names recipe
/// `self`.
fn pinned() -> (EventType, Value) {
    let manifest = json!({
        "bundle_name": "self",
        "realms": {"source": "/work/realms.json", "realm": "alpha",
                   "map": {"realms": [{"name": "alpha", "path": "repos/alpha"}]}},
    });
    (EventType::RunStarted, json!({"manifest": manifest}))
}

fn entered(phase: &str) -> (EventType, Value) {
    (EventType::PhaseEntered, json!({ "phase": phase }))
}

fn ruled(from: &str, next: Option<&str>) -> (EventType, Value) {
    let mut ruling = json!({"from": from, "result": "pass", "rule_id": "R"});
    if let Some(next) = next {
        ruling["next"] = json!(next);
    }
    (EventType::TransitionDecided, ruling)
}

/// A run's fold standing in `run` at `phase`, its last ruling naming
/// `rule_id` as `ruling` gives it, and `visits` implement visits.
fn folded(run: Run, phase: &str, ruling: Value, visits: u64) -> RunState {
    let mut state = run_state("run", 9, run);
    state.phase = Some(phase.to_string());
    state.last_decision = Some(serde_json::from_value::<Decision>(ruling).unwrap());
    state.visits.insert("implement".to_string(), visits);
    state.visits.insert("review".to_string(), 7);
    state
}

#[test]
fn a_start_names_its_realm_and_recipe_and_the_first_event_its_start() {
    let read = aggregate(&journal(&[pinned(), entered("design")]), None);
    assert_eq!(read.realm.as_deref(), Some("alpha"));
    assert_eq!(read.recipe.as_deref(), Some("self"));
    assert_eq!(read.started_ms, Some(second(1)));
    // A pin that does not type leaves both unread, as the console's
    // working tree reads it.
    let mut unpinned = pinned();
    unpinned.1["manifest"]["realms"]["map"]["realms"][0]["path"] = json!(7);
    let read = aggregate(&journal(&[unpinned]), None);
    assert_eq!((read.realm, read.recipe), (None, None));
    let mut sourceless = pinned();
    sourceless.1["manifest"]["realms"]["source"] = json!(null);
    assert_eq!(aggregate(&journal(&[sourceless]), None).recipe, None);
    // A first event that is no start pins nothing, and a later start
    // is never read for it.
    let read = aggregate(&journal(&[entered("design"), pinned()]), None);
    assert_eq!((read.realm, read.recipe), (None, None));
    assert_eq!(read.started_ms, Some(second(1)));
    let mut unstamped = journal(&[pinned()]);
    unstamped[0].recorded_at = "yesterday".to_string();
    let read = aggregate(&unstamped, None);
    assert_eq!(
        (read.realm.as_deref(), read.started_ms),
        (Some("alpha"), None)
    );
    assert_eq!(aggregate(&[], None), Reading::default().of(None));
}

/// Moved with the reading from brokkr-view's `clarify` (decision 0089
/// slice 4).
#[test]
fn the_recipe_is_the_bundle_name_the_start_recorded() {
    let recipe = |events: &[EventEnvelope]| aggregate(events, None).recipe;
    let start = |manifest: Value| (EventType::RunStarted, json!({ "manifest": manifest }));
    let named = journal(&[start(json!({"bundle_name": "triage"}))]);
    assert_eq!(recipe(&named).as_deref(), Some("triage"));
    let unnamed = journal(&[start(json!({}))]);
    assert_eq!(recipe(&unnamed), None, "a start that names no recipe");
    assert_eq!(recipe(&[]), None, "no run/started");
    let second = journal(&[
        (
            EventType::PhaseEntered,
            json!({"manifest": {"bundle_name": "x"}}),
        ),
        start(json!({"bundle_name": "y"})),
    ]);
    assert_eq!(
        recipe(&second),
        None,
        "only a first event that starts the run"
    );
    let malformed = journal(&[start(json!("nope"))]);
    assert_eq!(recipe(&malformed), None, "a manifest that does not read");
}

#[test]
fn a_run_ends_as_its_fold_concludes_at_its_last_terminal_event() {
    let ended = journal(&[
        pinned(),
        (EventType::RunStopped, json!({})),
        entered("x"),
        (EventType::RunCompleted, json!({})),
        entered("y"),
    ]);
    let rule = json!({"from": "verify", "result": "fail", "rule_id": "VERIFY-EXHAUSTED"});
    let stopped = folded(Run::Stopped, "stop", rule.clone(), 3);
    let read = aggregate(&ended, Some(&stopped));
    let by_rule = Ending::ByRule(Some("VERIFY-EXHAUSTED".to_string()));
    assert_eq!(read.finished, Some((second(4), by_rule)));
    assert_eq!(read.implement_visits, 3);
    let null_rule = json!({"from": "verify", "result": "fail", "rule_id": null});
    let unruled = folded(Run::Stopped, "stop", null_rule, 0);
    let read = aggregate(&ended, Some(&unruled));
    assert_eq!(read.finished, Some((second(4), Ending::ByRule(None))));
    let by_operator = folded(Run::Stopped, "review", rule.clone(), 0);
    let read = aggregate(&ended, Some(&by_operator));
    assert_eq!(read.finished, Some((second(4), Ending::ByOperator)));
    let shipped = folded(Run::Completed, "done", rule.clone(), 1);
    let read = aggregate(&ended, Some(&shipped));
    assert_eq!(read.finished, Some((second(4), Ending::Shipped)));
    assert_eq!(read.implement_visits, 1);
    // An open run has not ended, whatever its journal holds; one whose
    // terminal stamp does not parse, or with no terminal, has no end.
    for run in [Run::Running(crate::fold::Cursor::RequestEffect), awaiting()] {
        let open = folded(run, "stop", rule.clone(), 2);
        let read = aggregate(&ended, Some(&open));
        assert_eq!((read.finished, read.implement_visits), (None, 2));
    }
    let mut unstamped = ended.clone();
    unstamped[3].recorded_at = "later".to_string();
    assert_eq!(aggregate(&unstamped, Some(&shipped)).finished, None);
    assert_eq!(aggregate(&ended[..1], Some(&shipped)).finished, None);
    let unread = aggregate(&ended, None);
    assert_eq!((unread.finished, unread.implement_visits), (None, 0));
    let mut unvisited = shipped.clone();
    unvisited.visits.clear();
    assert_eq!(aggregate(&ended, Some(&unvisited)).implement_visits, 0);
}

fn awaiting() -> Run {
    crate::run_state::standing(Status::AwaitingOperator, "held")
}

#[test]
fn a_review_ruling_sends_the_run_back_on_or_holds_it() {
    let routes = |steps: &[(EventType, Value)]| {
        let read = aggregate(&journal(steps), None);
        (read.review_first, read.review_second, read.review_back)
    };
    let back_then_on = [
        entered("implement"),
        entered("review"),
        ruled("review", Some("implement")),
        ruled("verify", Some("verify")),
        ruled("review", Some("ship")),
    ];
    assert_eq!(routes(&back_then_on), (true, true, true));
    let on_then_back = [
        entered("implement"),
        ruled("review", Some("ship")),
        ruled("review", Some("implement")),
    ];
    assert_eq!(routes(&on_then_back), (false, false, true));
    let held = [
        entered("implement"),
        ruled("review", None),
        ruled("review", Some("stop")),
        ruled("review", Some("implement")),
    ];
    assert_eq!(routes(&held), (false, false, true));
    let back_twice = [
        entered("implement"),
        ruled("review", Some("implement")),
        ruled("review", Some("implement")),
    ];
    assert_eq!(routes(&back_twice), (true, false, true));
    // A phase entered only after the ruling was not one it went back to.
    let on = [ruled("review", Some("implement")), entered("implement")];
    assert_eq!(routes(&on), (false, false, false));
    assert_eq!(routes(&[entered("review")]), (false, false, false));
}

/// The writer's account carries the reading over each append as the
/// readout reads the run whole: the summary's aggregates are the run's.
#[test]
fn an_account_carries_the_aggregates_the_run_reads_whole() {
    let events = journal(&[
        pinned(),
        entered("implement"),
        entered("review"),
        ruled("review", Some("implement")),
        entered("implement"),
    ]);
    let listing = Listing {
        feature: "f",
        created_at: "2026-01-01T00:00:00Z",
    };
    let whole = Account::whole("run", &events);
    let expected = aggregate(&events, whole.state().ok());
    assert_eq!(Summary::of(&whole, listing).aggregate, expected);
    let carried = Account::whole("run", &events[..3]).onto(&events[3..]);
    assert_eq!(Summary::of(&carried, listing).aggregate, expected);
    assert_eq!(
        (expected.realm.as_deref(), expected.started_ms),
        (Some("alpha"), Some(second(1)))
    );
    assert!(expected.review_first && expected.review_back);
}
