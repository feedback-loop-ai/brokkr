//! A run whose participant records land before their effect's request
//! reads whole in every reader (#753; review 753s4s3, M1; decision 0089
//! ruling 8). Split from `tests.rs` to keep each test file within its
//! ceiling.

use super::*;

/// A run whose start and checkpoint land before their effect's request
/// keeps no seat head, so every reader reads it whole: the view whole and
/// the frame both print the run view's bytes, and its live block keeps
/// both journal-time labels (review 753s4s3, M1; decision 0089 ruling 8).
#[test]
fn a_record_before_its_request_reads_whole_in_every_reader() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "before request";
    let at = |seq: u64| format!("2026-01-01T00:00:0{seq}Z");
    let event = |seq: u64, event_type: brokkr_core::EventType, payload: Value| {
        envelope_builder::EnvelopeBuilder::new(event_type, payload)
            .run(run)
            .seq(seq)
            .event_id(format!("{run}-{seq}"))
            .at(at(seq))
            .sealed()
    };
    let checkpoint = |seq, turn, tool| {
        event(
            seq,
            brokkr_core::EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "checkpoint": {"member": "m", "step": "seat-turn", "turn": turn,
                                  "tool": tool, "model": "opus", "input_tokens": 10}}),
        )
    };
    let events = vec![
        event(
            1,
            brokkr_core::EventType::RunStarted,
            json!({"manifest": {}}),
        ),
        event(
            2,
            brokkr_core::EventType::PhaseEntered,
            json!({"phase": "work"}),
        ),
        event(
            3,
            brokkr_core::EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}]}),
        ),
        checkpoint(4, 1, "Read"),
        event(
            5,
            brokkr_core::EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
        ),
        checkpoint(6, 2, "Write"),
    ];
    planted(&db, vec![(run.to_string(), chained(run, events))]);
    let store = Store::open_read_only(&db).unwrap();
    let events = store.load(run).unwrap();
    assert!(
        !brokkr_core::summary::Account::whole(run, &events).seats_kept(),
        "a record before its request keeps no seat head"
    );
    let folded = fold(&events);
    let whole = brokkr_view::run_view(&events, folded.as_ref().ok());
    let labels: Vec<&str> = whole.live.iter().map(|line| line.label.as_str()).collect();
    assert_eq!(labels, ["?:m", "worker:m"]);
    let expected = printed(whole, &folded);
    for shows in [Shows::Everything, Shows::Frame] {
        let read = viewed(&store, run, shows).unwrap();
        assert_eq!(printed(read.view, &read.folded), expected, "{shows:?}");
    }
}
