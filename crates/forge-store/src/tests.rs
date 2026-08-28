use super::*;
use serde_json::json;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    (dir, store)
}

#[test]
fn append_load_roundtrip_and_chain() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    let events = store.load("r1").unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].previous_hash, events[0].event_hash);
    let state = forge_core::fold(&events).unwrap();
    assert_eq!(state.phase.as_deref(), Some("intake"));
}

#[test]
fn events_are_append_only() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    let update = store.conn.execute("UPDATE events SET envelope = 'x'", []);
    assert!(update.is_err(), "update must be rejected by trigger");
    let delete = store.conn.execute("DELETE FROM events", []);
    assert!(delete.is_err(), "delete must be rejected by trigger");
}

#[test]
fn duplicate_run_and_missing_run_fail() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    assert!(matches!(
        store.create_run("r1", "feat", "self", &json!({"files": {}})),
        Err(StoreError::RunExists(_))
    ));
    assert!(matches!(
        store.load("nope"),
        Err(StoreError::RunNotFound(_))
    ));
}

#[test]
fn export_verifies_offline() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    let ndjson = store.export_ndjson("r1").unwrap();
    let state = verify_export(&ndjson).unwrap();
    assert_eq!(state.seq, 2);

    // A tampered export fails closed.
    let tampered = ndjson.replace("intake", "ship");
    assert!(matches!(
        verify_export(&tampered),
        Err(VerifyError::Chain(_))
    ));
}

#[test]
fn schema_head_listing_and_append_conflict_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();

    assert_eq!(store.list_runs().unwrap().len(), 1);
    assert_eq!(store.head_hash("r1").unwrap(), (0, ZERO_HASH.to_string()));

    store
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER refuse_test_append BEFORE INSERT ON events
             BEGIN SELECT RAISE(IGNORE); END;",
        )
        .unwrap();
    assert!(matches!(
        store.append_next("r1", EventType::RunStarted, json!({}), None, None),
        Err(StoreError::AppendConflict { seq: 1 })
    ));

    store
        .conn
        .execute(
            "UPDATE meta SET value = 'not-a-schema' WHERE key = 'database_schema'",
            [],
        )
        .unwrap();
    drop(store);
    assert!(matches!(
        Store::open(&db),
        Err(StoreError::SchemaMismatch { found: 0 })
    ));
}

#[test]
fn exported_garbage_is_a_parse_refusal() {
    assert!(matches!(
        verify_export("not-json\n"),
        Err(VerifyError::Parse(_))
    ));
}

#[test]
fn open_and_create_surface_sqlite_failures_at_exact_boundaries() {
    // SQLite treats an empty filename as a private temporary database; this
    // also exercises the path-without-parent initialization branch.
    Store::open(Path::new("")).unwrap();

    let insert_dir = tempfile::tempdir().unwrap();
    let insert_db = insert_dir.path().join("insert.db");
    let conn = Connection::open(&insert_db).unwrap();
    conn.execute_batch(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TRIGGER refuse_meta_insert BEFORE INSERT ON meta
         BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open(&insert_db),
        Err(StoreError::Sqlite(_))
    ));

    let query_dir = tempfile::tempdir().unwrap();
    let query_db = query_dir.path().join("query.db");
    let conn = Connection::open(&query_db).unwrap();
    conn.execute_batch(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO meta (key, value) VALUES ('database_schema', x'ff');",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(Store::open(&query_db), Err(StoreError::Sqlite(_))));

    let (_dir, mut store) = store();
    store
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER refuse_run_insert BEFORE INSERT ON runs
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .unwrap();
    assert!(matches!(
        store.create_run("refused", "feature", "bundle", &json!({})),
        Err(StoreError::Sqlite(_))
    ));
}
