use super::*;
use serde_json::{json, Map};

#[path = "../../../../tests/support/seat_journal.rs"]
mod seat_journal;
use seat_journal::{boxed_seat, event};

/// The report as `brokkr costs` prints it: these tests pin the bytes.
fn seat_costs(events: &[EventEnvelope]) -> (Map<String, Value>, f64) {
    let (report, total) = super::seat_costs(events);
    let report = serde_json::to_value(report).unwrap();
    (report.as_object().unwrap().clone(), total)
}

/// The divergence as `brokkr compare` prints it.
fn first_divergence(a: &[String], b: &[String]) -> Value {
    serde_json::to_value(super::first_divergence(a, b)).unwrap()
}

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

/// A seat whose ONLY accounting is what it spent thinking still reports
/// it. The reasoning count is a subset of an output this harness did not
/// report, so there is no total to carry it — and a record that dropped
/// it because nothing else came with it would lose a meter the harness
/// did report (decision 0035 ruling 4).
#[test]
fn a_seat_reporting_only_reasoning_still_reports_it() {
    let events = vec![
        event(
            EventType::EffectRequested,
            json!({"effect_id":"fx", "seat":"implement"}),
        ),
        event(EventType::EffectStarted, json!({"effect_id":"fx"})),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "checkpoint":{
                "step":"turn-completed", "turn":1,
                "reasoning_output_tokens":9}}),
        ),
    ];
    let (costs, _) = seat_costs(&events);
    assert_eq!(costs["implement"]["reasoning_output_tokens"], 9);
    assert!(costs["implement"].get("output_tokens").is_none());
    // No model and no effort were reported, so both say so with the
    // sentinel rather than borrowing an answer from the plan.
    assert_eq!(costs["implement"]["model"], "not reported");
    assert_eq!(costs["implement"]["effort"], "not reported");
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
        // The finishing result is where claude reports what it spent
        // thinking, and where a seat with no turn checkpoints names its
        // effort. Both are read here, and the reasoning is a subset of
        // the output it rides beside — never a second addend.
        event(
            EventType::EffectSucceeded,
            json!({"effect_id":"fx-2", "result":{
                "model":"claude-sonnet-5", "effort":"xhigh",
                "input_tokens":7, "output_tokens":3,
                "reasoning_output_tokens":2}}),
        ),
    ];
    let (costs, total) = seat_costs(&events);
    assert_eq!(
        costs["implement"],
        // One effect named an effort and the other did not, so the seat
        // reports the level it has rather than a sentinel (decision 0035
        // ruling 3, reusing 0031's sentinels rather than inventing a
        // second pair). The reasoning count is summed on its own key.
        json!({"attempts": 2, "turns": 2, "cost_usd": 0.125,
               "model": "claude-fable-5-1, claude-sonnet-5",
               // No record carries a boundary beside its model: a journal
               // written before decision 0046 reads an explicit absence.
               "boundary": "not recorded",
               "effort": "xhigh",
               "input_tokens":35, "output_tokens":8,
               "cache_read_tokens":13, "cache_write_tokens":4,
               "reasoning_output_tokens":2})
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

/// The seat-costs record carries `boundary` beside `model`, reduced the
/// same way: one word, a joined list where the seat's records disagree,
/// and `not recorded` — an explicit absence — where no record that names
/// a model carries one. A word on a record that names no model is not
/// read: the pair is read together.
#[test]
fn seat_costs_reduce_the_boundary_exactly_as_the_model() {
    let (costs, _) = seat_costs(&boxed_seat("verify", "harness"));
    assert_eq!(costs["verify"]["model"], "claude-fable-5-1");
    assert_eq!(costs["verify"]["boundary"], "harness");

    let mut events = boxed_seat("verify", "namespace");
    events[3].payload["result"]["boundary"] = json!("open");
    let (costs, _) = seat_costs(&events);
    assert_eq!(costs["verify"]["boundary"], "namespace, open");

    // The sentinel yields to a real word, as it does for the model.
    let mut events = boxed_seat("verify", "namespace");
    events[2].payload["checkpoint"]["boundary"] = json!("not applicable");
    let (costs, _) = seat_costs(&events);
    assert_eq!(costs["verify"]["boundary"], "namespace");

    // A pre-0046 seat: models named, no word beside them.
    let mut events = boxed_seat("verify", "namespace");
    events[2].payload["checkpoint"]["boundary"] = Value::Null;
    events[3].payload["result"]["boundary"] = Value::Null;
    let (costs, _) = seat_costs(&events);
    assert_eq!(costs["verify"]["model"], "claude-fable-5-1");
    assert_eq!(costs["verify"]["boundary"], "not recorded");

    // A word beside no model is no stamp.
    let mut events = boxed_seat("verify", "namespace");
    events[2].payload["checkpoint"]["model"] = Value::Null;
    events[3].payload["result"]["model"] = Value::Null;
    let (costs, _) = seat_costs(&events);
    assert_eq!(costs["verify"]["model"], "not reported");
    assert_eq!(costs["verify"]["boundary"], "not recorded");
}
