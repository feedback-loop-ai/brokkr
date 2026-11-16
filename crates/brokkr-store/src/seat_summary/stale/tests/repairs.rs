//! The stale-seat set's damage shapes (decision 0089 slice 4, U5b1's
//! carried INFO items): the redundant drop gone, a keyless `seat_head` or
//! `seat_summary` remade, a VIEW or an INDEX holding a name replaced, and a
//! current journal's real open writing nothing.

use std::collections::BTreeMap;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use rusqlite::Connection;
use serde_json::json;

use super::*;
use crate::tests::envelope_builder::EnvelopeBuilder;

/// A run's start as an older writer leaves it, the one event both catch-up
/// cases plant: the summary has not folded it, so the open walks it.
fn started(run: &str, event_id: &str) -> EventEnvelope {
    EnvelopeBuilder::new(
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    )
    .run(run)
    .seq(1)
    .event_id(event_id)
    .at("2026-01-01T00:00:00Z")
    .previous(ZERO_HASH)
    .sealed()
}

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

/// `table`'s primary-key columns in key order, the order the repair's
/// `PRIMARY KEY` declares.
fn primary_key(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare("SELECT name FROM pragma_table_info(?1) WHERE pk > 0 ORDER BY pk")
        .unwrap()
        .query_map([table], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// What a real [`Store::open`] wrote on `path`: a schema version it moved,
/// an object it made, dropped or re-made, and the rows its own connection
/// changed. The open runs `ensure_wal` and `catch_up` as a production open
/// does, so their writes are inside the detector, not beside a raw
/// `Store::migrate`. A current journal's open leaves all empty, so the
/// repairs wrote nothing. The version catches a drop and remake of
/// identical objects, which the text comparison cannot see.
fn writes_on_open(path: &std::path::Path) -> Vec<String> {
    let (before, version) = {
        let conn = Connection::open(path).unwrap();
        (objects(&conn), schema_version(&conn))
    };
    let store = Store::open(path).unwrap();
    let after = objects(&store.conn);
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
    if schema_version(&store.conn) != version {
        found.push(format!(
            "schema version {} -> {}",
            version,
            schema_version(&store.conn)
        ));
    }
    // The open's connection starts at zero, so this counts the rows the
    // open itself wrote: `ensure_wal`'s and `catch_up`'s included.
    let changed = store.conn.total_changes();
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
    let event = started("run", "run-1");
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

/// A keyless `seat_summary` is not this build's any more than a keyless
/// head is: the writer's key is `(run_id, opened)`, and without it two
/// folds of one run can both stand. The seventh repair asks the key, so
/// the table is remade with it.
#[test]
fn a_keyless_seat_summary_is_remade() {
    let (dir, store) = current_store();
    store
        .conn
        .execute_batch(&format!(
            "DROP TABLE seat_summary;
             CREATE TABLE seat_summary (
                 run_id TEXT NOT NULL, opened INTEGER NOT NULL, {}, digest TEXT NOT NULL
             );
             CREATE INDEX seat_summary_started ON seat_summary (started_ms)
                 WHERE site IS NOT NULL;
             CREATE INDEX seat_summary_site
                 ON seat_summary (run_id, attempt_id, member, started_seq)
                 WHERE site IS NOT NULL;",
            crate::seat_summary::declared()
        ))
        .unwrap();
    assert!(
        !crate::seat_summary::seats_intact(&store.conn).unwrap(),
        "a keyless seat_summary is not this build's"
    );
    drop(store);

    let store = reopened(&dir);
    assert!(
        crate::seat_summary::seats_intact(&store.conn).unwrap(),
        "the seventh repair remade the table"
    );
    assert_eq!(
        primary_key(&store.conn, "seat_summary"),
        ["run_id", "opened"],
        "the key is asked and remade"
    );
}

/// A VIEW named `seat_head` is dropped as a view, not refused by `DROP
/// TABLE`, and the seventh repair remakes the head. The writable open is
/// `Ok`, the set equals the rule, and a second open writes nothing.
#[test]
fn a_view_holding_the_heads_name_is_replaced() {
    let (dir, store) = current_store();
    store
        .conn
        .execute_batch(
            "DROP TABLE seat_head;
             CREATE VIEW seat_head AS
                 SELECT '' AS run_id, 0 AS folded_seq, '' AS folded_hash, '' AS shape;",
        )
        .unwrap();
    assert!(
        !crate::seat_summary::seats_intact(&store.conn).unwrap(),
        "a view is not this build's head"
    );
    drop(store);

    let store = reopened(&dir);
    assert_eq!(
        primary_key(&store.conn, "seat_head"),
        ["run_id"],
        "the head is a keyed table again"
    );
    drop(store);
    let writes = writes_on_open(&dir.path().join("forge.db"));
    assert!(
        writes.is_empty(),
        "a second open writes nothing: {writes:?}"
    );
}

/// An INDEX named `seat_lag` is dropped as an index, not refused by `DROP
/// TABLE`, and the eighth repair remakes the set. The writable open is
/// `Ok`, the set equals the rule, and a second open writes nothing.
#[test]
fn an_index_holding_the_sets_name_is_replaced() {
    let (dir, store) = current_store();
    store
        .conn
        .execute_batch(
            "DROP TABLE seat_lag;
             CREATE INDEX seat_lag ON run_summary (run_id);",
        )
        .unwrap();
    assert!(
        !stale_intact(&store.conn).unwrap(),
        "an index is not the set"
    );
    drop(store);

    let store = reopened(&dir);
    assert_eq!(
        primary_key(&store.conn, "seat_lag"),
        ["run_id"],
        "the set is a keyed table again"
    );
    drop(store);
    let writes = writes_on_open(&dir.path().join("forge.db"));
    assert!(
        writes.is_empty(),
        "a second open writes nothing: {writes:?}"
    );
}

/// A current journal's real open writes nothing after this unit's repairs:
/// `ensure_wal` and `catch_up` run inside the detector, and the eighth
/// repair's own writes on a journal that lost the set are seen, so the
/// empty answer is a read of a whole journal.
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

/// The real-open detector sees the open's catch-up: a journal carrying an
/// event its summary has not folded yet writes rows on `Store::open`, and
/// `Store::migrate` alone writes none. A detector built on the raw migrate
/// answer would call this journal steady, so this case binds the detector
/// to the whole open.
#[test]
fn the_open_detector_sees_the_catch_up() {
    let (dir, store) = crate::tests::store();
    let event = started("unfolded", "unfolded-1");
    crate::test_support::plant_older_writer(&store, "unfolded", &[event]);
    drop(store);

    let writes = writes_on_open(&dir.path().join("forge.db"));
    assert!(
        writes.iter().any(|what| what.ends_with("rows changed")),
        "the open's catch-up writes rows: {writes:?}"
    );
}
