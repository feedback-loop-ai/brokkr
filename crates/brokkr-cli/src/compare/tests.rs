use super::*;

#[path = "../../../../tests/support/seat_journal.rs"]
mod seat_journal;
use seat_journal::boxed_seat;

/// The view's divergence as `brokkr compare` prints it.
fn printed_divergence(a: &BTreeMap<String, Resolution>, b: &BTreeMap<String, Resolution>) -> Value {
    serde_json::to_value(resolution_divergence(a, b)).unwrap()
}

/// A run's resolution map as `brokkr compare` prints it.
fn printed(resolution: &BTreeMap<String, Resolution>) -> Value {
    serde_json::to_value(resolution).unwrap()
}

#[test]
fn run_facts_ignores_missing_display_only_phase_join() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .create_run("run", "feature", "bundle", &json!({"bundle_name":"bundle"}))
        .unwrap();
    for (event_type, payload) in [
        (EventType::RunStarted, json!({"feature":"feature"})),
        (EventType::PhaseEntered, json!({"phase":"work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id":"effect", "seat":"work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect", "attempt_id":"attempt"}),
        ),
    ] {
        store
            .append_next("run", event_type, payload, None, None)
            .unwrap();
    }
    let facts = run_facts(&store, "run").unwrap();
    assert_eq!(facts.attempts, 1);
    assert_eq!(facts.summary["phases_visited"], json!({}));
}

/// #376's two-attempt fixture: attempt one finished at $0.25 and was
/// retried, attempt two finished at $0.50. `brokkr costs --run latest`
/// resolves, and it and the view every other surface reads report the
/// same spend — every attempt's, summed.
#[test]
fn costs_resolve_latest_and_agree_with_the_view_on_a_retried_seat() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .create_run(
            "run-retried",
            "feature",
            "bundle",
            &json!({"bundle_name":"bundle"}),
        )
        .unwrap();
    let finished = |attempt: &str, cost: f64| {
        json!({"effect_id":"effect", "attempt_id":attempt, "checkpoint":{
            "step":"claude-session-finished", "total_cost_usd":cost}})
    };
    for (event_type, payload) in [
        (EventType::RunStarted, json!({"feature":"feature"})),
        (EventType::PhaseEntered, json!({"phase":"work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id":"effect", "seat":"work", "phase":"work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect", "attempt_id":"a1"}),
        ),
        (EventType::EffectCheckpointed, finished("a1", 0.25)),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect", "attempt_id":"a2"}),
        ),
        (EventType::EffectCheckpointed, finished("a2", 0.5)),
    ] {
        store
            .append_next("run-retried", event_type, payload, None, None)
            .unwrap();
    }

    assert_eq!(
        costs(&store, "latest").unwrap(),
        json!({"run_id": "run-retried",
               "seats": {"work": {"attempts": 2, "turns": 0, "cost_usd": 0.75,
                                  "model": "not reported", "boundary": "not recorded",
                                  "effort": "not reported"}},
               "total_cost_usd": 0.75})
    );
    assert_eq!(costs(&store, "run-re").unwrap()["run_id"], "run-retried");
    assert_eq!(
        costs(&store, "nope").unwrap_err().to_string(),
        "no run matching 'nope' in this workspace database"
    );

    let view = brokkr_view::run_view(&store.load("run-retried").unwrap(), None);
    let part = &view.participants[0];
    assert_eq!(part.cost, Some(0.75));
    assert_eq!(part.last_attempt_cost, Some(0.5));
    assert_eq!(part.cost_cell.text, "$0.7500 over 2 attempts");
}

/// `compare`'s resolution map carries the pair per participant as
/// siblings, and the structural divergence names a boundary difference
/// exactly as it names a model difference.
#[test]
fn the_resolution_map_carries_the_pair_and_diverges_on_the_boundary() {
    let harness = resolutions(&boxed_seat("verify", "harness"));
    let namespace = resolutions(&boxed_seat("verify", "namespace"));
    assert_eq!(
        printed(&harness),
        json!({"verify": {"model": "claude-fable-5-1", "boundary": "harness",
                          "selected": null}})
    );
    let divergence = printed_divergence(&harness, &namespace);
    assert_eq!(divergence["verify"]["a"]["boundary"], "harness");
    assert_eq!(divergence["verify"]["b"]["boundary"], "namespace");
    assert_eq!(divergence["verify"]["a"]["model"], "claude-fable-5-1");
    assert_eq!(printed_divergence(&harness, &harness), json!({}));

    // A pre-0046 seat reads the absence mark, which is itself a divergence.
    let mut old = boxed_seat("verify", "namespace");
    old[1].payload.as_object_mut().unwrap().remove("boundary");
    old[2].payload["checkpoint"]["boundary"] = Value::Null;
    old[3].payload["result"]["boundary"] = Value::Null;
    let old = resolutions(&old);
    assert_eq!(printed(&old)["verify"]["boundary"], brokkr_view::ABSENT);
    assert_eq!(
        printed_divergence(&namespace, &old)["verify"]["b"]["boundary"],
        brokkr_view::ABSENT
    );
}
