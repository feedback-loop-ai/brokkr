use super::*;
use crate::tests::envelope_builder;

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

/// A printed document, read back as a value.
fn value(document: impl Serialize) -> Value {
    serde_json::to_value(document).unwrap()
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
    assert_eq!(value(&facts.summary)["phases_visited"], json!({}));
}

/// #376's two-attempt fixture: attempt one finished at $0.25 and was
/// retried, attempt two finished at $0.50. `brokkr costs --run latest`
/// resolves, and it and the view every other surface reads report the
/// same spend — every attempt's, summed.
#[test]
fn costs_resolve_latest_and_agree_with_the_view_on_a_retried_seat() {
    let (_dir, store) = retried_store();

    assert_eq!(
        value(costs(&store, "latest").unwrap()),
        json!({"run_id": "run-retried",
               "seats": {"work": {"attempts": 2, "turns": 0, "cost_usd": 0.75,
                                  "model": "not reported", "boundary": "not recorded",
                                  "effort": "not reported"}},
               "total_cost_usd": 0.75})
    );
    assert_eq!(
        value(costs(&store, "run-re").unwrap())["run_id"],
        "run-retried"
    );
    assert_eq!(
        costs(&store, "nope").map(value).unwrap_err().to_string(),
        "no run matching 'nope' in this workspace database"
    );

    let view = brokkr_view::run_view(&store.load("run-retried").unwrap(), None);
    let part = &view.participants[0];
    assert_eq!(part.cost, Some(0.75));
    assert_eq!(part.last_attempt_cost, Some(0.5));
    assert_eq!(part.cost_cell.text, "$0.7500 over 2 attempts");
}

/// A journal holding #376's two-attempt fixture, `run-retried`.
fn retried_store() -> (tempfile::TempDir, Store) {
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
    (dir, store)
}

/// `text` with each run's first and last `recorded_at` written in for
/// its placeholders: the store stamps them at append, so the pins below
/// spell them `<run>.first` and `<run>.last`.
fn stamped(store: &Store, text: &str, runs: &[&str]) -> String {
    let mut text = text.to_string();
    for run in runs {
        let events = store.load(run).unwrap();
        for (mark, event) in [("first", events.first()), ("last", events.last())] {
            text = text.replace(&format!("{run}.{mark}"), &event.unwrap().recorded_at);
        }
    }
    text
}

/// The exact bytes `brokkr costs` prints (#346), LaneTally's join
/// surface: a value comparison ignores key order, so it is pinned as a
/// string.
#[test]
fn costs_print_these_bytes() {
    let (_dir, store) = retried_store();
    assert_eq!(
        serde_json::to_string_pretty(&costs(&store, "latest").unwrap()).unwrap(),
        r#"{
  "run_id": "run-retried",
  "seats": {
    "work": {
      "attempts": 2,
      "boundary": "not recorded",
      "cost_usd": 0.75,
      "effort": "not reported",
      "model": "not reported",
      "turns": 0
    }
  },
  "total_cost_usd": 0.75
}"#
    );
}

/// A run of one `work` seat that cost `cost` and whose result `rule`
/// decided, ended by `ending`.
fn one_seat_run(store: &mut Store, run: &str, cost: f64, rule: Value, ending: (EventType, Value)) {
    let next = if rule.is_null() {
        json!(null)
    } else {
        json!("done")
    };
    store
        .create_run(run, run, "bundle", &json!({"bundle_name": "bundle"}))
        .unwrap();
    for (event_type, payload) in [
        (EventType::RunStarted, json!({"feature": run})),
        (EventType::PhaseEntered, json!({"phase":"work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id":"e", "seat":"work", "phase":"work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"e", "attempt_id":"a"}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id":"e", "attempt_id":"a", "checkpoint":{
                "step":"claude-session-finished", "total_cost_usd":cost}}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id":"e", "attempt_id":"a", "result":{"result":"done"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from":"work", "result":"done", "rule_id":rule, "next":next,
                   "severity":null, "inputs":{}, "problem":null}),
        ),
        ending,
    ] {
        store
            .append_next(run, event_type, payload, None, None)
            .unwrap();
    }
}

/// The exact bytes `brokkr compare` prints (#346), both runs' summaries
/// keyed by run id and the comparison: one run parked by a decision with
/// no rule, the other completed by a rule.
#[test]
fn compare_prints_these_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let parked = (EventType::RunParked, json!({"reason":"no rule matched"}));
    one_seat_run(&mut store, "run-parked", 0.5, json!(null), parked);
    let completed = (EventType::RunCompleted, json!({}));
    one_seat_run(&mut store, "run-shipped", 0.25, json!("WORK-OK"), completed);
    let a = run_facts(&store, "run-parked").unwrap();
    let b = run_facts(&store, "run-shipped").unwrap();
    assert_eq!(
        serde_json::to_string_pretty(&document("run-parked", a, "run-shipped", b)).unwrap(),
        stamped(&store, COMPARED, &["run-parked", "run-shipped"])
    );
}

/// `compare_prints_these_bytes`'s document, its timestamps placeholders.
const COMPARED: &str = r#"{
  "comparison": {
    "attempts_delta": 0,
    "cost_delta_usd": -0.25,
    "first_divergence": {
      "a": "park",
      "b": "WORK-OK",
      "index": 0
    },
    "resolution_divergence": {},
    "same_feature": false,
    "same_recipe": true,
    "status_pair": [
      "awaiting_operator",
      "completed"
    ]
  },
  "runs": {
    "run-parked": {
      "bundle_name": "bundle",
      "decision_trail": [
        "park"
      ],
      "events": 8,
      "feature": "run-parked",
      "first_recorded_at": "run-parked.first",
      "last_recorded_at": "run-parked.last",
      "manifest": {
        "sha256": "2da169fab6fd75d1ea8b36a382f23040181d24a31df91ffce5224caf6897a5ed"
      },
      "park_reason": "no rule matched",
      "phase": "work",
      "phases_visited": {
        "work": 1
      },
      "resolution": {
        "work": {
          "boundary": "—",
          "model": "—",
          "selected": null
        }
      },
      "seats": {
        "work": {
          "attempts": 1,
          "boundary": "not recorded",
          "cost_usd": 0.5,
          "effort": "not reported",
          "model": "not reported",
          "turns": 0
        }
      },
      "status": "awaiting_operator",
      "total_cost_usd": 0.5
    },
    "run-shipped": {
      "bundle_name": "bundle",
      "decision_trail": [
        "WORK-OK"
      ],
      "events": 8,
      "feature": "run-shipped",
      "first_recorded_at": "run-shipped.first",
      "last_recorded_at": "run-shipped.last",
      "manifest": {
        "sha256": "2da169fab6fd75d1ea8b36a382f23040181d24a31df91ffce5224caf6897a5ed"
      },
      "park_reason": null,
      "phase": "work",
      "phases_visited": {
        "work": 1
      },
      "resolution": {
        "work": {
          "boundary": "—",
          "model": "—",
          "selected": null
        }
      },
      "seats": {
        "work": {
          "attempts": 1,
          "boundary": "not recorded",
          "cost_usd": 0.25,
          "effort": "not reported",
          "model": "not reported",
          "turns": 0
        }
      },
      "status": "completed",
      "total_cost_usd": 0.25
    }
  }
}"#;

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
