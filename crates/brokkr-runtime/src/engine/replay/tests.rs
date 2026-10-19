use super::*;
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, Run};
use brokkr_store::test_support::{every_run_reads_as_its_fold, lane};
use brokkr_store::StoreError;
use serde_json::json;

/// A run started and entered into `intake`: two events.
fn started(store: &mut Store, run_id: &str) {
    store
        .create_run(run_id, "feat", "self", &json!({}))
        .unwrap();
    let opened = json!({"feature": "feat", "manifest": {}});
    store
        .append_next(run_id, EventType::RunStarted, opened, None, None)
        .unwrap();
    let entered = json!({"phase": "intake"});
    store
        .append_next(run_id, EventType::PhaseEntered, entered, None, None)
        .unwrap();
}

fn requested(store: &mut Store, run_id: &str) {
    let payload = json!({"effect_id": "fx", "seat": "intake"});
    store
        .append_next(run_id, EventType::EffectRequested, payload, None, None)
        .unwrap();
}

/// A later turn reads, verifies and folds only what landed since the
/// last: a prefix spoiled after it was read goes unread, where a whole
/// load refuses it.
#[test]
fn a_turn_reads_only_what_landed_since_the_last() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    started(&mut store, "r1");
    let mut replay = Replay::default();
    assert_eq!(replay.caught_up(&mut store, "r1").unwrap().seq, 2);

    let raw = rusqlite::Connection::open(store.path()).unwrap();
    raw.execute_batch(
        "DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = replace(envelope, '\"feat\"', '\"spoiled\"') WHERE seq = 1;",
    )
    .unwrap();
    requested(&mut store, "r1");

    let state = replay.caught_up(&mut store, "r1").unwrap();
    assert_eq!(state.seq, 3);
    assert_eq!(
        state.run,
        Run::Running(Cursor::ExecuteEffect {
            effect_id: "fx".into(),
            seat: "intake".into(),
            failed_attempts: 0,
        })
    );
    assert_eq!(replay.events.len(), 3);
    let whole = store.load("r1").unwrap_err();
    assert!(
        matches!(
            whole,
            StoreError::Chain(brokkr_core::envelope::ChainError::BadHash { seq: 1 })
        ),
        "{whole}"
    );
}

/// A replay is one run's: asked for another, it reads that run whole
/// rather than carrying the first run's state onto the second's suffix.
#[test]
fn a_replay_asked_for_another_run_reads_it_whole() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    started(&mut store, "long");
    requested(&mut store, "long");
    started(&mut store, "short");
    let mut replay = Replay::default();
    assert_eq!(replay.caught_up(&mut store, "long").unwrap().seq, 3);

    let state = replay.caught_up(&mut store, "short").unwrap();
    assert_eq!((state.run_id.as_str(), state.seq), ("short", 2));
    assert_eq!(state.run, Run::Running(Cursor::RequestEffect));
    assert_eq!(replay.events.len(), 2);
}

/// A turn hands the store the account it caught up to, so the turn's
/// appends write the run's summary (decision 0089 ruling 3): an append
/// made with none leaves the row behind its head, one made after a turn
/// leaves it at its head. Its removal control is a replay that keeps its
/// account to itself, whose append leaves the row behind.
#[test]
fn a_turn_hands_the_store_the_account_it_caught_up_to() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    started(&mut Store::open(&path).unwrap(), "r1");
    let mut store = Store::open(&path).unwrap();
    requested(&mut store, "r1");
    assert_eq!(lane(&store, "r1").as_deref(), Some("behind"));

    let mut replay = Replay::default();
    assert_eq!(replay.caught_up(&mut store, "r1").unwrap().seq, 3);
    let opened = json!({"effect_id": "fx", "attempt_id": "a1"});
    store
        .append_next("r1", EventType::EffectStarted, opened, None, None)
        .unwrap();
    assert_eq!(lane(&store, "r1").as_deref(), Some("running"));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// An engine driven to its conclusion leaves its run's summary at the
/// head it concluded on, so every engine scenario's drop check compares
/// the row with the fold rather than passing a row behind.
#[test]
fn a_driven_run_leaves_its_summary_at_its_head() {
    let body = super::super::tests::single_body(vec!["driver".into()]);
    let (_dir, mut engine) = super::super::tests::engine(body);
    for (event_type, payload) in [
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (EventType::RunCompleted, json!({})),
    ] {
        engine.append(event_type, payload, None).unwrap();
    }
    let end = engine.drive().unwrap();
    assert_eq!(end.state.seq, 3);
    assert_eq!(
        lane(&engine.store, &engine.run_id).as_deref(),
        Some("finished")
    );
}
