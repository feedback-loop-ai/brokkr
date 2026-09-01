use brokkr_core::fold::{fold, Cursor, Status};
use brokkr_core::EventType;
use brokkr_runtime::{apply_fenced_operator_command, FencedCommandOutcome};
use brokkr_store::Store;
use serde_json::json;

fn started_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .create_run("run-1", "084-f", "fast", &json!({"files":{}}))
        .unwrap();
    store
        .append_next(
            "run-1",
            EventType::RunStarted,
            json!({"feature":"084-f", "manifest":{}}),
            None,
            None,
        )
        .unwrap();
    (dir, store)
}

#[test]
fn stale_command_is_durably_rejected_without_changing_control_state() {
    let (_dir, mut store) = started_store();
    let outcome = apply_fenced_operator_command(
        &mut store,
        "run-1",
        "command-1",
        "stop",
        "user:operator-1",
        "cancel",
        99,
        &"f".repeat(64),
    )
    .unwrap();
    assert!(
        matches!(outcome, FencedCommandOutcome::Rejected { reason, .. } if reason == "stale_cursor")
    );
    let events = store.load("run-1").unwrap();
    assert_eq!(events.len(), 3);
    let state = fold(&events).unwrap();
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.cursor, Cursor::Start);
}

#[test]
fn command_receipt_replay_is_idempotent_after_bridge_uncertainty() {
    let (_dir, mut store) = started_store();
    let first = apply_fenced_operator_command(
        &mut store,
        "run-1",
        "command-replay",
        "stop",
        "user:operator-1",
        "cancel",
        99,
        &"f".repeat(64),
    )
    .unwrap();
    let count = store.load("run-1").unwrap().len();
    let second = apply_fenced_operator_command(
        &mut store,
        "run-1",
        "command-replay",
        "stop",
        "user:operator-1",
        "cancel",
        99,
        &"f".repeat(64),
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(store.load("run-1").unwrap().len(), count);
}

#[test]
fn correctly_fenced_stop_is_accepted_only_from_a_safe_parked_boundary() {
    let (_dir, mut store) = started_store();
    for (event_type, payload, attempt_id) in [
        (EventType::PhaseEntered, json!({"phase":"implement"}), None),
        (
            EventType::EffectRequested,
            json!({"effect_id":"effect-1", "seat":"implementer"}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect-1", "attempt_id":"attempt-1"}),
            Some("attempt-1".into()),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id":"effect-1", "attempt_id":"attempt-1"}),
            Some("attempt-1".into()),
        ),
        (
            EventType::RunParked,
            json!({"reason":"executor_lost"}),
            None,
        ),
    ] {
        store
            .append_next("run-1", event_type, payload, None, attempt_id)
            .unwrap();
    }
    let (seq, hash) = store.head_hash("run-1").unwrap();
    let outcome = apply_fenced_operator_command(
        &mut store,
        "run-1",
        "command-2",
        "stop",
        "user:operator-1",
        "cancel after executor loss",
        seq,
        &hash,
    )
    .unwrap();
    assert!(matches!(outcome, FencedCommandOutcome::Accepted { .. }));
    let state = fold(&store.load("run-1").unwrap()).unwrap();
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.cursor, Cursor::Stop);
}
