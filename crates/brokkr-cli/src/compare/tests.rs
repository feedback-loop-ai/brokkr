use super::*;

#[test]
fn divergence_handles_equal_prefix_and_mismatch_trails() {
    let a = vec!["one".to_string(), "two".to_string()];
    assert_eq!(first_divergence(&a, &a), Value::Null);
    assert_eq!(
        first_divergence(&a, &["one".to_string()]),
        json!({"index": 1, "a": "two", "b": "end"})
    );
    assert_eq!(
        first_divergence(&a, &["other".to_string(), "two".to_string()]),
        json!({"index": 0, "a": "one", "b": "other"})
    );
}

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EventEnvelope {
        run_id: "run".into(),
        seq: 1,
        event_id: "event".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: None,
        recorded_at: "2026-08-28T00:00:00Z".into(),
        previous_hash: String::new(),
        event_hash: String::new(),
    }
}

#[test]
fn seat_costs_ignore_unjoined_and_malformed_effect_evidence() {
    let events = vec![
        event(EventType::EffectRequested, json!({"effect_id":"only-id"})),
        event(EventType::EffectRequested, json!({"seat":"only-seat"})),
        event(EventType::EffectStarted, json!({"effect_id":"unknown"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"unknown", "checkpoint":{"num_turns":2}}),
        ),
        event(EventType::RunCompleted, json!({})),
    ];
    let (costs, total) = seat_costs(&events);
    assert!(costs.is_empty());
    assert_eq!(total, 0.0);
}

#[test]
fn seat_costs_sum_capture_carrying_lanetally_checkpoints_unchanged() {
    // The lanetally driver's session-finished checkpoint carries an
    // extra constant `capture` field; the aggregation keys off
    // num_turns/total_cost_usd only, so the cost flows through with
    // zero production changes here, in `brokkr costs`, or in the UI.
    let events = vec![
        event(
            EventType::EffectRequested,
            json!({"effect_id":"fx", "seat":"implement"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"fx"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "checkpoint":{
                "step":"claude-lanetally-session-finished",
                "capture":"lanetally",
                "model":"claude-fable-5-1",
                "num_turns":2, "total_cost_usd":0.125,
                "input_tokens":28, "output_tokens":5,
                "cache_read_tokens":13, "cache_write_tokens":4}}),
        ),
        event(
            EventType::EffectRequested,
            json!({"effect_id":"fx-2", "seat":"implement"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"fx-2"})),
        event(
            EventType::EffectSucceeded,
            json!({"effect_id":"fx-2", "result":{"model":"claude-sonnet-5"}}),
        ),
    ];
    let (costs, total) = seat_costs(&events);
    assert_eq!(
        costs["implement"],
        json!({"attempts": 2, "turns": 2, "cost_usd": 0.125,
               "model": "claude-fable-5-1, claude-sonnet-5",
               "input_tokens":28, "output_tokens":5,
               "cache_read_tokens":13, "cache_write_tokens":4})
    );
    assert_eq!(total, 0.125);
}

#[test]
fn seat_costs_prefer_turn_usage_over_the_repeated_finishing_totals() {
    let events = vec![
        event(
            EventType::EffectRequested,
            json!({"effect_id":"fx", "seat":"implement"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"fx"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "checkpoint":{
                "step":"seat-turn", "turn":1, "model":"claude-served",
                "input_tokens":13, "output_tokens":2,
                "cache_read_tokens":3, "cache_write_tokens":4}}),
        ),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "checkpoint":{
                "step":"seat-turn", "turn":2, "model":"claude-served",
                "input_tokens":15, "output_tokens":3, "cache_read_tokens":10}}),
        ),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "checkpoint":{
                "step":"claude-code-session-finished", "model":"claude-served",
                "num_turns":2, "input_tokens":28, "output_tokens":5,
                "cache_read_tokens":13, "cache_write_tokens":4}}),
        ),
        event(
            EventType::EffectSucceeded,
            json!({"effect_id":"fx", "result":{
                "result":"complete", "model":"claude-served",
                "input_tokens":28, "output_tokens":5,
                "cache_read_tokens":13, "cache_write_tokens":4}}),
        ),
    ];

    let (costs, _) = seat_costs(&events);
    assert_eq!(costs["implement"]["input_tokens"], 28);
    assert_eq!(costs["implement"]["output_tokens"], 5);
    assert_eq!(costs["implement"]["cache_read_tokens"], 13);
    assert_eq!(costs["implement"]["cache_write_tokens"], 4);
    assert!(costs["implement"].get("total_tokens").is_none());
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

#[test]
fn seat_costs_read_partial_usage_untracked_results_and_a_second_cost_report() {
    let events = vec![
        // Per-turn usage carrying only output: answering "is there any
        // usage here" has to look past the input count to find it.
        event(
            EventType::EffectRequested,
            json!({"effect_id":"out", "seat":"scribe"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"out"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"out", "checkpoint":{"step":"seat-turn", "output_tokens":9}}),
        ),
        // Only a cache read: past the input and output counts both.
        event(
            EventType::EffectRequested,
            json!({"effect_id":"cache", "seat":"reader"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"cache"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"cache", "checkpoint":{"step":"seat-turn", "cache_read_tokens":11}}),
        ),
        // A start naming no effect at all.
        event(EventType::EffectStarted, json!({})),
        // A result for an effect no request ever seated.
        event(
            EventType::EffectSucceeded,
            json!({"effect_id":"ghost", "result":{"total_cost_usd":9.5}}),
        ),
        // Cost already counted from the checkpoint: the result repeats
        // the same money and must not be charged a second time.
        event(
            EventType::EffectRequested,
            json!({"effect_id":"paid", "seat":"implement"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"paid"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"paid", "checkpoint":{
                "model":"claude-fable-5-1", "num_turns":1, "total_cost_usd":0.5}}),
        ),
        event(
            EventType::EffectSucceeded,
            json!({"effect_id":"paid", "result":{"num_turns":7, "total_cost_usd":99.0}}),
        ),
    ];

    let (costs, total) = seat_costs(&events);
    assert_eq!(costs["scribe"]["output_tokens"], 9);
    assert!(costs["scribe"].get("input_tokens").is_none());
    assert_eq!(costs["reader"]["cache_read_tokens"], 11);
    assert_eq!(costs["implement"]["cost_usd"], 0.5);
    assert_eq!(costs["implement"]["turns"], 1);
    assert_eq!(costs["implement"]["model"], "claude-fable-5-1");
    assert!(!costs.contains_key("ghost"));
    assert_eq!(total, 0.5);
}
