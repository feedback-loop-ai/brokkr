//! The stale-seat set's tests (decision 0089 slice 4, U3d): the set the
//! triggers keep equals the rule's seq-and-hash half over every lag shape,
//! the shape arm supplies the rest, and the repair fills the set from the
//! rule a journal migrated mid-life already states.

use brokkr_core::canonical::ZERO_HASH;
use rusqlite::{params, Connection};

use super::*;
use crate::test_support::plant_run;
use crate::Store;

/// The envelope an older writer's event carries: only its `recorded_at`
/// matters to the summary's head trigger.
const ENVELOPE: &str = r#"{"recorded_at":"2026-01-01T00:00:00Z"}"#;

/// The run ids `sql` names with `params`, in the statement's order.
fn rows(conn: &Connection, sql: &str, params: impl rusqlite::Params) -> Vec<String> {
    let mut statement = conn.prepare(sql).unwrap();
    statement
        .query_map(params, |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// The runs `[STALE]` holds, in run id order.
fn set(conn: &Connection) -> Vec<String> {
    rows(
        conn,
        &format!("SELECT run_id FROM {STALE} ORDER BY run_id"),
        rusqlite::params![],
    )
}

/// The runs [`lags`] names on this journal, in run id order: the rule's
/// own statement, the test's oracle.
fn rule(conn: &Connection) -> Vec<String> {
    let sql = format!(
        "SELECT r.run_id FROM run_summary AS r
           LEFT JOIN seat_head AS h ON h.run_id = r.run_id
          WHERE {} ORDER BY r.run_id",
        lags("r", "h", "?1")
    );
    rows(conn, &sql, rusqlite::params![SHAPE.as_str()])
}

/// The runs [`stale_runs`] names, in run id order: the set union the heads
/// of another shape.
fn stale(conn: &Connection) -> Vec<String> {
    let sql = format!("SELECT run_id FROM ({}) ORDER BY run_id", stale_runs("?1"));
    rows(conn, &sql, rusqlite::params![SHAPE.as_str()])
}

/// The runs that lag by shape alone: a seat head at the summary's own seq
/// and hash, written by another build.
fn shape_only(conn: &Connection) -> Vec<String> {
    rows(
        conn,
        "SELECT h.run_id FROM seat_head AS h
           JOIN run_summary AS r ON r.run_id = h.run_id
          WHERE h.folded_seq = r.head_seq AND h.folded_hash = r.head_hash
            AND h.shape IS NOT ?1 ORDER BY h.run_id",
        rusqlite::params![SHAPE.as_str()],
    )
}

/// `run_id`'s seat head, written as the writer writes it.
fn head(conn: &Connection, run_id: &str, seq: i64, hash: &str, shape: &str) {
    conn.execute(
        "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash, shape)
         VALUES (?1, ?2, ?3, ?4)",
        params![run_id, seq, hash, shape],
    )
    .unwrap();
}

/// An event planted straight into the tables, as an older writer leaves
/// one: the summary's head trigger moves the row's head.
fn appended(conn: &Connection, run_id: &str, seq: i64, hash: &str) {
    crate::test_support::plant_event(conn, run_id, &seq, hash, ENVELOPE).unwrap();
}

/// The folded columns at the head the row names, so the row is not
/// `behind` and the rule alone decides it.
fn settled(conn: &Connection, run_id: &str) {
    conn.execute(
        "UPDATE run_summary
            SET folded_seq = 0, folded_hash = ?2, aggregated_seq = 0,
                stop_folded_seq = 0, ending_folded_seq = 0
          WHERE run_id = ?1",
        params![run_id, ZERO_HASH],
    )
    .unwrap();
}

/// A current run: no fact of its row lags, and its seat head names this
/// build's shape.
fn current(conn: &Connection, run_id: &str) {
    settled(conn, run_id);
    head(conn, run_id, 0, ZERO_HASH, SHAPE.as_str());
}

/// A run the journal already carries at `head`, by an older or a current
/// writer, with no seat head: the shape `seats_kept` false leaves.
fn headless(conn: &Connection, run_id: &str) {
    settled(conn, run_id);
    conn.execute("DELETE FROM seat_head WHERE run_id = ?1", [run_id])
        .unwrap();
}

/// The repair's shared tail: reopen `path` and run the open's repair path,
/// without the catch-up whose refold would clear the lags the test states.
fn repaired(path: &std::path::Path) -> Connection {
    let mut conn = Connection::open(path).unwrap();
    Store::migrate(&mut conn).unwrap();
    conn
}

/// A headless run joins, a matching head leaves, a head of another shape
/// is named through the index, and a head written at another seq joins
/// again.
#[test]
fn every_transition_keeps_the_set() {
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    assert_eq!(set(&store.conn), ["run".to_string()], "a new run lags");

    current(&store.conn, "run");
    assert_eq!(set(&store.conn), Vec::<String>::new(), "a match leaves");

    appended(&store.conn, "run", 1, "h1");
    assert_eq!(set(&store.conn), ["run".to_string()], "a moved head joins");

    head(&store.conn, "run", 1, "h1", SHAPE.as_str());
    assert_eq!(
        set(&store.conn),
        Vec::<String>::new(),
        "the new head leaves"
    );

    head(&store.conn, "run", 1, "h1", "another-build");
    assert_eq!(
        set(&store.conn),
        Vec::<String>::new(),
        "the seq and hash match, so the set does not hold it"
    );
    assert_eq!(
        stale(&store.conn),
        ["run".to_string()],
        "the shape arm names it"
    );

    head(&store.conn, "run", 0, "old", SHAPE.as_str());
    assert_eq!(
        set(&store.conn),
        ["run".to_string()],
        "a head at another seq joins again"
    );
}

/// The set is the rule's seq-and-hash half, over every lag shape: no head,
/// a seq lag, a hash lag, a shape lag, and current. The shape arm supplies
/// the shape lag, so the set union the shape arm is the rule itself.
#[test]
fn the_set_matches_the_rule_over_every_lag_shape() {
    let (_dir, store) = crate::tests::store();
    for run_id in ["current", "headless", "seq-lag", "hash-lag", "shape-lag"] {
        plant_run(&store.conn, run_id, "{}").unwrap();
    }
    current(&store.conn, "current");
    headless(&store.conn, "headless");
    // The seat head is behind the summary head, which the row names, so
    // the lane does not hide the lag.
    current(&store.conn, "seq-lag");
    head(&store.conn, "seq-lag", -1, "h", SHAPE.as_str());
    current(&store.conn, "hash-lag");
    store
        .conn
        .execute(
            "UPDATE run_summary SET head_hash = 'h2' WHERE run_id = 'hash-lag'",
            [],
        )
        .unwrap();
    current(&store.conn, "shape-lag");
    store
        .conn
        .execute(
            "UPDATE seat_head SET shape = 'another-build' WHERE run_id = 'shape-lag'",
            [],
        )
        .unwrap();

    let named = rule(&store.conn);
    let shape = shape_only(&store.conn);
    let expected: Vec<String> = named
        .iter()
        .filter(|run_id| !shape.contains(run_id))
        .cloned()
        .collect();
    assert_eq!(set(&store.conn), expected, "the triggers keep one half");
    assert_eq!(stale(&store.conn), named, "with the shape arm, the rule");
    assert_eq!(
        named,
        [
            "hash-lag".to_string(),
            "headless".to_string(),
            "seq-lag".to_string(),
            "shape-lag".to_string()
        ],
        "every lag shape but the current run"
    );
}

/// The drops a journal from before the set has taken: the table, the
/// shape index and every trigger that keeps them.
fn drop_set(conn: &Connection) {
    conn.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {SUMMARY_INSERT};
         DROP TRIGGER IF EXISTS {SUMMARY_MOVED};
         DROP TRIGGER IF EXISTS {SEAT_HEAD};
         DROP TRIGGER IF EXISTS {SEAT_HEAD_DROPPED};
         DROP INDEX IF EXISTS {SHAPE_INDEX};
         DROP TABLE IF EXISTS {STALE};"
    ))
    .unwrap();
}

/// A journal from before the set is repaired by the open's repair path,
/// and the set then names exactly what the rule names now, shape lag
/// included: the fill reads the rule, not the triggers it just made.
#[test]
fn the_repair_fills_the_set_from_the_rule() {
    let (dir, store) = crate::tests::store();
    for run_id in ["current", "headless", "shape-lag"] {
        plant_run(&store.conn, run_id, "{}").unwrap();
    }
    current(&store.conn, "current");
    headless(&store.conn, "headless");
    current(&store.conn, "shape-lag");
    store
        .conn
        .execute(
            "UPDATE seat_head SET shape = 'another-build' WHERE run_id = 'shape-lag'",
            [],
        )
        .unwrap();
    let named = rule(&store.conn);

    drop_set(&store.conn);
    assert!(!stale_intact(&store.conn).unwrap(), "the set is gone");
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(stale_intact(&conn).unwrap(), "the repair restored the set");
    assert_eq!(set(&conn), named, "the fill names every run that lags now");
    assert_eq!(stale(&conn), named, "and the fallback's rule agrees");
}

/// A lost trigger is repaired, and the fill removes an entry the lost
/// trigger would have removed: a current run is never left named.
#[test]
fn a_missing_trigger_is_repaired_and_the_set_refilled() {
    let (dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    assert_eq!(set(&store.conn), ["run".to_string()], "the run lags");
    store
        .conn
        .execute(&format!("DROP TRIGGER {SEAT_HEAD}"), [])
        .unwrap();
    current(&store.conn, "run");
    assert_eq!(
        set(&store.conn),
        ["run".to_string()],
        "the lost trigger left the entry behind"
    );
    assert!(!stale_intact(&store.conn).unwrap(), "the loss is noticed");
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(stale_intact(&conn).unwrap(), "the trigger is back");
    assert_eq!(
        set(&conn),
        Vec::<String>::new(),
        "the fill removed the current run's stale entry"
    );
}

/// The repair sits on the open's path: a journal that lost the set, a
/// trigger and the shape index opens, and the next read of it is held to a
/// whole set. The open's own catch-up refolds the runs, so what this binds
/// is that the objects are back, not the fill's mid-migration contents.
#[test]
fn a_journal_that_lost_the_set_is_repaired_on_open() {
    let (dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    current(&store.conn, "run");
    drop_set(&store.conn);
    drop(store);

    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert!(stale_intact(&store.conn).unwrap(), "the open repaired it");
    assert_eq!(set(&store.conn), Vec::<String>::new(), "the run is current");
}

/// An index another build's shape made is not this build's: the repair
/// replaces it, so a changed `SHAPE` never trusts the old predicate.
#[test]
fn an_index_of_another_shape_is_replaced() {
    let (dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    current(&store.conn, "run");
    store
        .conn
        .execute_batch(&format!(
            "DROP INDEX {SHAPE_INDEX};
             CREATE INDEX {SHAPE_INDEX} ON seat_head (run_id)
                 WHERE shape IS NOT 'another-build';"
        ))
        .unwrap();
    assert!(
        !stale_intact(&store.conn).unwrap(),
        "another shape's index is not this build's"
    );
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(
        stale_intact(&conn).unwrap(),
        "the repair made this build's index"
    );
}

/// The repair leaves an older writer's path writing the set: events
/// inserted straight into the tables move the summary head by the
/// journal's trigger, and the set joins the run without the writer knowing
/// the set exists. A matching seat head written after them leaves it.
#[test]
fn an_older_writers_append_joins_the_set() {
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    current(&store.conn, "run");
    appended(&store.conn, "run", 1, "h1");
    appended(&store.conn, "run", 2, "h2");
    assert_eq!(
        set(&store.conn),
        ["run".to_string()],
        "the head moved twice, the seat head did not"
    );
    head(&store.conn, "run", 2, "h2", SHAPE.as_str());
    assert_eq!(
        set(&store.conn),
        Vec::<String>::new(),
        "the matching head leaves"
    );
}
