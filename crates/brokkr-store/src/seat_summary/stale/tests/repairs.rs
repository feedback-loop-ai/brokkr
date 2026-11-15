//! The stale-seat set's last three damage shapes (decision 0089 slice 4,
//! U5a2's LOWs): the redundant drop gone, a keyless `seat_head` remade,
//! a VIEW holding `seat_lag`'s name replaced, and a current journal's open
//! writing nothing.

use std::collections::BTreeMap;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventType;
use rusqlite::Connection;
use serde_json::json;

use super::*;
use crate::tests::envelope_builder::EnvelopeBuilder;

/// The `seat_head` rows of `run_id`.
fn heads(conn: &Connection, run_id: &str) -> i64 {
    conn.query_row(
        "SELECT count(*) FROM seat_head WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )
    .unwrap()
}

/// The journal's objects, by kind and name, as `sqlite_master` states
/// them.
fn objects(conn: &Connection) -> BTreeMap<(String, String), Option<String>> {
    conn.prepare("SELECT type, name, sql FROM sqlite_master")
        .unwrap()
        .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// `conn`'s schema version, which SQLite bumps on every DDL statement,
/// including a drop and remake of the same object.
fn schema_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA schema_version", [], |row| row.get(0))
        .unwrap()
}

/// What `Store::migrate` wrote on `path`: a schema version it moved, an
/// object it made, dropped or re-made, and the rows its statements changed.
/// A current journal's open leaves all empty, so the repairs wrote nothing.
/// The version catches a drop and remake of identical objects, which the
/// text comparison cannot see.
fn writes_on_open(path: &std::path::Path) -> Vec<String> {
    let mut conn = Connection::open(path).unwrap();
    let before = objects(&conn);
    let version = schema_version(&conn);
    let changes = conn.total_changes();
    Store::migrate(&mut conn).unwrap();
    let after = objects(&conn);
    let mut found: Vec<String> = after
        .iter()
        .filter(|(key, sql)| before.get(*key) != Some(*sql))
        .map(|((kind, name), _)| format!("{kind} {name}"))
        .collect();
    for (kind, name) in before.keys() {
        if !after.contains_key(&(kind.clone(), name.clone())) {
            found.push(format!("dropped {kind} {name}"));
        }
    }
    if schema_version(&conn) != version {
        found.push(format!(
            "schema version {} -> {}",
            version,
            schema_version(&conn)
        ));
    }
    let changed = conn.total_changes() - changes;
    if changed > 0 {
        found.push(format!("{changed} rows changed"));
    }
    found
}

/// The journal reopened after a damage shape: the open is `Ok`, the set is
/// remade, and the set equals the rule.
fn reopened(dir: &tempfile::TempDir) -> Store {
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert!(stale_intact(&store.conn).unwrap(), "the set is remade");
    assert_eq!(set(&store.conn), rule(&store.conn), "set == rule");
    store
}

/// A keyless `seat_head` is not this build's: the writer's
/// `INSERT OR REPLACE` appended where it should replace, so two folds of
/// one run stand. The seventh repair remakes the head and refolds the run,
/// leaving one head row and the set equal to the rule.
#[test]
fn a_keyless_seat_head_is_remade_and_the_run_refolded() {
    let (dir, store) = crate::tests::store();
    let event = EnvelopeBuilder::new(
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    )
    .run("run")
    .seq(1)
    .event_id("run-1")
    .at("2026-01-01T00:00:00Z")
    .previous(ZERO_HASH)
    .sealed();
    crate::test_support::plant_older_writer(&store, "run", &[event]);
    let store = crate::test_support::refolded(store);
    store
        .conn
        .execute_batch(
            "DROP TABLE seat_head;
             CREATE TABLE seat_head
                 (run_id TEXT, folded_seq INTEGER, folded_hash TEXT, shape TEXT);",
        )
        .unwrap();
    // Two folds of one run: with no key each write appends.
    crate::seat_summary::write_head(&store.conn, "run", (1, "h1")).unwrap();
    crate::seat_summary::write_head(&store.conn, "run", (2, "h2")).unwrap();
    assert_eq!(heads(&store.conn, "run"), 2, "the keyless writer appended");
    drop(store);

    let store = reopened(&dir);
    assert_eq!(heads(&store.conn, "run"), 1, "one head row for the run");
}

/// A VIEW named `seat_lag` is dropped as a view and the set remade there:
/// the writable open is `Ok`, the set equals the rule, and a second open
/// writes nothing.
#[test]
fn a_view_holding_the_sets_name_is_replaced() {
    let (dir, store) = current_store();
    store
        .conn
        .execute_batch(
            "DROP TABLE seat_lag;
             CREATE VIEW seat_lag AS SELECT 1 AS run_id;",
        )
        .unwrap();
    drop(store);

    let store = reopened(&dir);
    drop(store);
    let writes = writes_on_open(&dir.path().join("forge.db"));
    assert!(
        writes.is_empty(),
        "a second open writes nothing: {writes:?}"
    );
}

/// A current journal's open writes nothing after this unit's repairs; the
/// detector sees the eighth repair's own writes on a journal that lost the
/// set, so the empty answer is a read of a whole journal.
#[test]
fn a_current_journals_open_writes_nothing() {
    let (dir, store) = current_store();
    drop(store);
    let path = dir.path().join("forge.db");
    let writes = writes_on_open(&path);
    assert!(
        writes.is_empty(),
        "the steady state writes nothing: {writes:?}"
    );

    let (dir, store) = current_store();
    store.conn.execute_batch("DROP TABLE seat_lag;").unwrap();
    drop(store);
    let writes = writes_on_open(&dir.path().join("forge.db"));
    assert!(
        writes
            .iter()
            .any(|what| what == "table seat_lag" || what == "trigger seat_lag_of_a_summary"),
        "the detector sees the repair: {writes:?}"
    );
}
