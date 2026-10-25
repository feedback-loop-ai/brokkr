//! A run's start (decision 0095 ruling 4): its row and its `run/started`
//! land together or not at all, and a fence runs inside the transaction
//! before either write, its refusal writing nothing.

use serde_json::json;

use super::*;
use crate::tests::store;

/// What the journal holds of `run`: its row, as the manifest it pins, and
/// its events, by type and payload.
fn held(store: &Store, run: &str) -> Option<(Value, Vec<(EventType, Value)>)> {
    let manifest = store.manifest(run).ok()?;
    let events = store.load(run).unwrap();
    let events = events.into_iter().map(|e| (e.event_type, e.payload));
    Some((manifest, events.collect()))
}

/// The hash of each event `run` holds, in order.
fn hashes(store: &Store, run: &str) -> Vec<String> {
    let events = store.load(run).unwrap();
    events.into_iter().map(|event| event.event_hash).collect()
}

/// Start `run` with `fence`, pinning a manifest and carrying a payload that
/// name it.
fn start(
    store: &mut Store,
    run: &str,
    fence: Option<Fence<'_>>,
) -> Result<EventEnvelope, StoreError> {
    let manifest = json!({"schema": "run-manifest/v1", "run": run});
    store.start_run(run, "f", "self", &manifest, json!({"feature": run}), fence)
}

#[test]
fn a_start_writes_its_run_row_and_its_run_started_together() {
    let (_dir, mut store) = store();
    let sealed = start(&mut store, "r1", None).unwrap();
    assert_eq!(
        held(&store, "r1"),
        Some((
            json!({"schema": "run-manifest/v1", "run": "r1"}),
            vec![(EventType::RunStarted, json!({"feature": "r1"}))]
        ))
    );
    assert_eq!(hashes(&store, "r1"), vec![sealed.event_hash]);
    let refused = start(&mut store, "r1", None).unwrap_err();
    assert!(matches!(&refused, StoreError::RunExists(run) if run == "r1"));
    assert_eq!(refused.to_string(), "run 'r1' already exists");
    assert_eq!(store.load("r1").unwrap().len(), 1);
}

/// A fault between the two writes, after the run's row and before its
/// first event, leaves neither.
#[test]
fn a_fault_between_the_run_row_and_its_first_event_leaves_neither() {
    let (_dir, mut store) = store();
    store
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER fault_after_the_row BEFORE INSERT ON events
             BEGIN SELECT RAISE(ABORT, 'fault after the row'); END;",
        )
        .unwrap();
    let refused = start(&mut store, "r1", None).unwrap_err();
    assert!(matches!(&refused, StoreError::Sqlite(_)), "{refused:?}");
    assert_eq!(refused.to_string(), "sqlite: fault after the row");
    assert_eq!(held(&store, "r1"), None);
    assert_eq!(store.list_runs().unwrap(), vec![]);
}

/// The fence sees the journal before either write, and its refusal is the
/// start's, with nothing written; a fence that passes lets both land.
#[test]
fn a_fence_runs_before_either_write_and_its_refusal_writes_nothing() {
    let (_dir, mut store) = store();
    let mut seen = Vec::new();
    let mut refusing = |tx: &Transaction<'_>| {
        let runs: i64 = tx.query_row("SELECT COUNT(*) FROM runs", [], |row| row.get(0))?;
        seen.push(runs);
        Err(StoreError::RunNotFound("fenced".into()))
    };
    let refused = start(&mut store, "r1", Some(&mut refusing)).unwrap_err();
    assert!(matches!(&refused, StoreError::RunNotFound(why) if why == "fenced"));
    assert_eq!(seen, vec![0]);
    assert_eq!(store.list_runs().unwrap(), vec![]);

    let mut passing = |_: &Transaction<'_>| Ok(());
    start(&mut store, "r1", Some(&mut passing)).unwrap();
    assert_eq!(held(&store, "r1").unwrap().1.len(), 1);
}
