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

/// The journal the fold used to refuse, folded end to end. This is the
/// verbatim export of a live run (105 events, read-only evidence): the
/// operator's `stop` was accepted at seq 93 while the verify seat's
/// effect was in flight, the attempt kept checkpointing to seq 104, and
/// the effect succeeded at seq 105. The engine accepted and recorded
/// all of it — the journal is the authority (decision 0001) — so the
/// whole export must verify, chain, and fold without a refusal.
#[test]
fn the_operator_stop_that_came_mid_flight_folds_end_to_end() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");
    let ndjson = std::fs::read_to_string(&fixture).unwrap();
    let state = verify_export(&ndjson).unwrap();

    assert_eq!(state.run_id, "tui-graph-the-selection-box-gets-80f98deb");
    assert_eq!(state.seq, 105, "the export's last event");
    // No terminal event was captured in this export: the run is still
    // running, in the phase whose seat was working when it was stopped.
    assert_eq!(state.status, forge_core::fold::Status::Running);
    assert_eq!(state.phase.as_deref(), Some("verify"));
    // The accepted stop rode the in-flight attempt to its boundary:
    // instead of the succeeded effect's normal `Decide`, the run
    // concludes per the operator's command.
    assert_eq!(state.cursor, forge_core::fold::Cursor::Stop);
    assert!(!state.riding_stop, "the riding command is spent");
    assert_eq!(state.pending_command, None, "the stop was disposed of");
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

#[test]
fn read_only_open_reads_the_journal_and_refuses_every_write() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
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
    drop(store);

    let mut reader = Store::open_read_only(&db).unwrap();
    assert_eq!(reader.list_runs().unwrap().len(), 1);
    assert_eq!(reader.load("r1").unwrap().len(), 1);
    // The refusal is SQLite's, not this module's discipline.
    assert!(matches!(
        reader.append_next("r1", EventType::PhaseEntered, json!({}), None, None),
        Err(StoreError::Sqlite(_))
    ));
    assert!(matches!(
        reader.create_run("r2", "feat", "self", &json!({})),
        Err(StoreError::Sqlite(_))
    ));
    drop(reader);

    // Nothing moved: the read-only session left the journal as it was.
    let after = Store::open(&db).unwrap();
    assert_eq!(after.load("r1").unwrap().len(), 1);
}

#[test]
fn read_only_open_refuses_a_missing_and_a_foreign_database() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        Store::open_read_only(&dir.path().join("absent.db")),
        Err(StoreError::Sqlite(_))
    ));

    // A SQLite file that is not a journal at all: it opens, and the
    // schema question is what refuses it. No migration runs to invent
    // the table it is missing.
    let foreign = dir.path().join("foreign.db");
    let conn = Connection::open(&foreign).unwrap();
    conn.execute_batch("CREATE TABLE something_else (key TEXT);")
        .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open_read_only(&foreign),
        Err(StoreError::Sqlite(_))
    ));

    let db = dir.path().join("forge.db");
    drop(Store::open(&db).unwrap());
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE meta SET value = 'not-a-schema' WHERE key = 'database_schema'",
        [],
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open_read_only(&db),
        Err(StoreError::SchemaMismatch { found: 0 })
    ));
}
