//! The stale-seat set's tests (decision 0089 slice 4, U3d): the set the
//! triggers keep equals the rule's seq-and-hash half over every lag shape,
//! the shape arm supplies the rest, and the repair fills the set from the
//! rule a journal migrated mid-life already states.

use brokkr_core::canonical::ZERO_HASH;
use rusqlite::{params, Connection};

use super::*;
use crate::seat_summary::lags;
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

/// The runs [`Store::window_fallbacks`] names, store-wide and in its order.
fn fallback(store: &Store) -> Vec<String> {
    let mut named = Vec::new();
    store
        .window_fallbacks(0, 0, None, |run| named.push(run.run_id.to_string()))
        .unwrap();
    named
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

/// The rule's seq-and-hash half: the runs [`lags`] names less the ones
/// only the shape arm supplies.
fn seq_and_hash_half(conn: &Connection) -> Vec<String> {
    let shape = shape_only(conn);
    rule(conn)
        .into_iter()
        .filter(|run_id| !shape.contains(run_id))
        .collect()
}

/// Write `run_id`'s seat head as another build did, at the summary's own
/// seq and hash: a shape lag only the read-time arm names.
fn shape_lag(conn: &Connection, run_id: &str) {
    conn.execute(
        "UPDATE seat_head SET shape = 'another-build' WHERE run_id = ?1",
        [run_id],
    )
    .unwrap();
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

/// A store and its directory carrying one current run, `run`.
fn current_store() -> (tempfile::TempDir, Store) {
    let (dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    current(&store.conn, "run");
    (dir, store)
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

/// A head written by `UPDATE` is kept as a head written by `INSERT` is:
/// moved off the summary's head it joins the set, and moved back it
/// leaves. No production writer updates a head, but the triggers keep the
/// set exact whatever writes the journal's own table.
#[test]
fn an_updated_seat_head_keeps_the_set() {
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    current(&store.conn, "run");
    assert_eq!(set(&store.conn), Vec::<String>::new(), "the run is current");

    store
        .conn
        .execute(
            "UPDATE seat_head SET folded_seq = 7 WHERE run_id = 'run'",
            [],
        )
        .unwrap();
    assert_eq!(
        set(&store.conn),
        ["run".to_string()],
        "a head moved in place joins"
    );
    assert_eq!(rule(&store.conn), ["run".to_string()]);

    store
        .conn
        .execute(
            "UPDATE seat_head SET folded_seq = 0 WHERE run_id = 'run'",
            [],
        )
        .unwrap();
    assert_eq!(
        set(&store.conn),
        Vec::<String>::new(),
        "a head moved back leaves"
    );
    assert_eq!(rule(&store.conn), Vec::<String>::new());
}

/// A summary repair does not name a current run: the seed names the head
/// columns on every row, and the trigger decides by the seat head it finds
/// rather than joining on the write alone. A head that moves and comes
/// back leaves the set as the rule says.
#[test]
fn a_summary_repair_leaves_the_set_equal_to_the_rule() {
    let (dir, store) = current_store();
    store
        .conn
        .execute("DROP INDEX run_summary_page", [])
        .unwrap();
    assert!(!crate::summary::summary_intact(&store.conn).unwrap());
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(crate::summary::summary_intact(&conn).unwrap());
    assert_eq!(
        set(&conn),
        Vec::<String>::new(),
        "a current run stays out of the set"
    );
    assert_eq!(set(&conn), rule(&conn));

    conn.execute(
        "UPDATE run_summary SET head_hash = 'h2' WHERE run_id = 'run'",
        [],
    )
    .unwrap();
    assert_eq!(set(&conn), ["run".to_string()], "a moved head joins");
    conn.execute(
        "UPDATE run_summary SET head_hash = ?1 WHERE run_id = 'run'",
        [ZERO_HASH],
    )
    .unwrap();
    assert_eq!(
        set(&conn),
        Vec::<String>::new(),
        "a head that comes back leaves"
    );
    assert_eq!(set(&conn), rule(&conn));
}

/// A trigger body another build wrote is not this build's: the repair
/// replaces it, so a journal carrying the old unconditional join takes the
/// fixed body rather than being trusted by its name.
#[test]
fn a_trigger_of_another_body_is_replaced() {
    let (dir, store) = current_store();
    store
        .conn
        .execute_batch(&format!(
            "DROP TRIGGER {SUMMARY_MOVED};
             CREATE TRIGGER {SUMMARY_MOVED}
                 AFTER UPDATE OF head_seq, head_hash ON run_summary BEGIN
                 INSERT OR IGNORE INTO {STALE} (run_id) VALUES (NEW.run_id);
             END;"
        ))
        .unwrap();
    assert!(
        !stale_intact(&store.conn).unwrap(),
        "another body is not this build's"
    );
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(stale_intact(&conn).unwrap(), "the repair replaced the body");
    assert_eq!(
        set(&conn),
        Vec::<String>::new(),
        "the current run stays out"
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
    shape_lag(&store.conn, "shape-lag");

    let named = rule(&store.conn);
    let expected = seq_and_hash_half(&store.conn);
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
         DROP TRIGGER IF EXISTS {SEAT_HEAD_UPDATED};
         DROP TRIGGER IF EXISTS {SEAT_HEAD_DROPPED};
         DROP INDEX IF EXISTS {SHAPE_INDEX};
         DROP TABLE IF EXISTS {STALE};"
    ))
    .unwrap();
}

/// A journal from before the set is repaired by the open's repair path,
/// and the fill names the rule's seq-and-hash half, not the shape lag the
/// triggers never keep: with the shape arm, the fallback still names
/// exactly what the rule names.
#[test]
fn the_repair_fills_the_set_from_the_rule() {
    let (dir, store) = crate::tests::store();
    for run_id in ["current", "headless", "shape-lag"] {
        plant_run(&store.conn, run_id, "{}").unwrap();
    }
    current(&store.conn, "current");
    headless(&store.conn, "headless");
    current(&store.conn, "shape-lag");
    shape_lag(&store.conn, "shape-lag");
    let named = rule(&store.conn);
    let expected = seq_and_hash_half(&store.conn);

    drop_set(&store.conn);
    assert!(!stale_intact(&store.conn).unwrap(), "the set is gone");
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(stale_intact(&conn).unwrap(), "the repair restored the set");
    assert_eq!(set(&conn), expected, "the fill names the seq-and-hash half");
    assert_eq!(stale(&conn), named, "with the shape arm, the rule");
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

/// A reader that cannot write and finds no set reads the rule's own
/// statement, not the set's absent table: a journal from before U3d is
/// still answered exactly, as the projection fallbacks answer (decision
/// 0014).
#[test]
fn a_read_only_reader_without_the_set_reads_the_rule() {
    let (dir, store) = crate::tests::store();
    plant_run(&store.conn, "current", "{}").unwrap();
    current(&store.conn, "current");
    plant_run(&store.conn, "headless", "{}").unwrap();
    headless(&store.conn, "headless");
    let named = rule(&store.conn);
    assert_eq!(named, ["headless".to_string()]);
    drop_set(&store.conn);
    drop(store);

    let reader = Store::open_read_only(&dir.path().join("forge.db")).unwrap();
    assert!(
        !stale_intact(&reader.conn).unwrap(),
        "the reader finds no set"
    );
    assert_eq!(
        fallback(&reader),
        named,
        "the fallback reads the rule, not the absent table"
    );
}

/// A reader that cannot write and finds a damaged set reads the rule's own
/// statement too: a lost trigger can leave a lagging run out of the set,
/// and the reader must still name it.
#[test]
fn a_read_only_reader_with_a_damaged_set_reads_the_rule() {
    let (dir, store) = current_store();
    store
        .conn
        .execute(&format!("DROP TRIGGER {SEAT_HEAD_DROPPED}"), [])
        .unwrap();
    store
        .conn
        .execute("DELETE FROM seat_head WHERE run_id = 'run'", [])
        .unwrap();
    assert_eq!(
        set(&store.conn),
        Vec::<String>::new(),
        "the lost trigger kept no entry"
    );
    assert_eq!(
        rule(&store.conn),
        ["run".to_string()],
        "the rule names the run the set lost"
    );
    assert!(!stale_intact(&store.conn).unwrap(), "the loss is noticed");
    drop(store);

    let reader = Store::open_read_only(&dir.path().join("forge.db")).unwrap();
    assert_eq!(
        fallback(&reader),
        ["run".to_string()],
        "the fallback reads the rule, not the damaged set"
    );
}

/// A summary repair whose seed writes `run_summary` while `seat_head` or
/// `seat_lag` is gone: SQLite compiles the set's summary triggers on that
/// write, so the open drops them first and the eighth repair remakes them.
/// The open is `Ok` and the set afterwards equals the rule.
#[test]
fn a_summary_repair_without_a_seat_table_reseeds() {
    for lost in ["seat_head", STALE] {
        let (dir, store) = current_store();
        store
            .conn
            .execute_batch(&format!(
                "DROP INDEX run_summary_page;
                 DROP TABLE {lost};"
            ))
            .unwrap();
        drop(store);

        let store = Store::open(&dir.path().join("forge.db"))
            .unwrap_or_else(|error| panic!("open with {lost} gone: {error}"));
        assert!(stale_intact(&store.conn).unwrap(), "{lost} is repaired");
        assert_eq!(set(&store.conn), rule(&store.conn), "{lost}");
    }
}

/// The statement that fires a trigger may declare any conflict policy,
/// and an upsert its own: the body's join never meets a duplicate, so no
/// write fails on `seat_lag` and the run stays named.
#[test]
fn a_firing_conflict_policy_does_not_replace_the_body() {
    // `UPDATE OR ABORT run_summary` overrides the body's old `OR IGNORE`.
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "a", "{}").unwrap();
    store
        .conn
        .execute(
            "UPDATE OR ABORT run_summary SET head_seq = 5 WHERE run_id = 'a'",
            [],
        )
        .unwrap();
    assert_eq!(set(&store.conn), ["a".to_string()], "the update succeeded");
    assert_eq!(set(&store.conn), rule(&store.conn));

    // An upsert that moves the summary head.
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "a", "{}").unwrap();
    store
        .conn
        .execute(
            "INSERT INTO run_summary (run_id, head_seq, head_hash)
             VALUES ('a', 6, 'h')
             ON CONFLICT(run_id) DO UPDATE
                 SET head_seq = excluded.head_seq, head_hash = excluded.head_hash",
            [],
        )
        .unwrap();
    assert_eq!(set(&store.conn), ["a".to_string()], "the upsert succeeded");
    assert_eq!(set(&store.conn), rule(&store.conn));

    // An upsert on an existing seat head that does not match the summary.
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "a", "{}").unwrap();
    store
        .conn
        .execute(
            "INSERT OR IGNORE INTO seat_head (run_id, folded_seq, folded_hash, shape)
             VALUES ('a', 5, 'h', ?1)",
            [SHAPE.as_str()],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO seat_head (run_id, folded_seq, folded_hash, shape)
             VALUES ('a', 7, 'h2', ?1)
             ON CONFLICT(run_id) DO UPDATE
                 SET folded_seq = excluded.folded_seq, folded_hash = excluded.folded_hash",
            [SHAPE.as_str()],
        )
        .unwrap();
    assert_eq!(set(&store.conn), ["a".to_string()], "the upsert succeeded");
    assert_eq!(set(&store.conn), rule(&store.conn));

    // A plain insert into the seat head of a run already in the set.
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "a", "{}").unwrap();
    store
        .conn
        .execute(
            "INSERT OR ABORT INTO seat_head (run_id, folded_seq, folded_hash, shape)
             VALUES ('a', 5, 'h', ?1)",
            [SHAPE.as_str()],
        )
        .unwrap();
    assert_eq!(set(&store.conn), ["a".to_string()], "the insert succeeded");
    assert_eq!(set(&store.conn), rule(&store.conn));
}

/// The fill uses the rule's seq-and-hash half alone: a run that lags by
/// shape is not put in the set, so a later shape update cannot leave it
/// named. The shape arm reads it at read time.
#[test]
fn a_shape_only_lag_is_not_filled_into_the_set() {
    let (dir, store) = crate::tests::store();
    plant_run(&store.conn, "run", "{}").unwrap();
    current(&store.conn, "run");
    shape_lag(&store.conn, "run");
    drop_set(&store.conn);
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert_eq!(
        set(&conn),
        Vec::<String>::new(),
        "the seq and hash match, so the fill leaves it out"
    );
    conn.execute(
        "UPDATE seat_head SET shape = ?1 WHERE run_id = 'run'",
        [SHAPE.as_str()],
    )
    .unwrap();
    drop(conn);

    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert_eq!(
        fallback(&store),
        Vec::<String>::new(),
        "once the shape matches, nothing names the run"
    );
}

/// A seat head's `run_id` moved in place: the run it moved from has no
/// head and joins the set, and the run it moved to is judged as an insert
/// is, so a matching head still leaves it.
#[test]
fn a_moved_run_id_joins_the_old_run_and_judges_the_new() {
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "from", "{}").unwrap();
    plant_run(&store.conn, "to", "{}").unwrap();
    current(&store.conn, "from");
    headless(&store.conn, "to");
    assert_eq!(set(&store.conn), ["to".to_string()], "to has no head");

    store
        .conn
        .execute(
            "UPDATE seat_head SET run_id = 'to' WHERE run_id = 'from'",
            [],
        )
        .unwrap();
    assert_eq!(
        set(&store.conn),
        ["from".to_string()],
        "from lost its head and to gained a matching one"
    );
    assert_eq!(set(&store.conn), rule(&store.conn));
}

/// A run whose seat head is deleted joins the set by its own trigger,
/// whatever removed the head.
#[test]
fn a_dropped_seat_head_joins_the_set() {
    let (_dir, store) = current_store();
    assert_eq!(set(&store.conn), Vec::<String>::new(), "the run is current");

    store
        .conn
        .execute("DELETE FROM seat_head WHERE run_id = 'run'", [])
        .unwrap();
    assert_eq!(
        set(&store.conn),
        ["run".to_string()],
        "a dropped head joins"
    );
    assert_eq!(set(&store.conn), rule(&store.conn));
}

/// A `seat_lag` of another shape is not this build's, and the open remakes
/// it: a table with another column, one with `run_id` but no key, and one
/// whose `run_id` is not the key all fail `stale_intact` and are repaired.
#[test]
fn a_seat_lag_of_another_shape_is_not_intact_and_is_repaired() {
    for remake in [
        "CREATE TABLE seat_lag (other TEXT PRIMARY KEY);",
        "CREATE TABLE seat_lag (run_id TEXT);",
        "CREATE TABLE seat_lag (run_id TEXT, other TEXT PRIMARY KEY);",
    ] {
        let (dir, store) = current_store();
        store
            .conn
            .execute_batch(&format!("DROP TABLE {STALE}; {remake}"))
            .unwrap();
        assert!(!stale_intact(&store.conn).unwrap(), "{remake}");
        drop(store);

        let store = Store::open(&dir.path().join("forge.db")).unwrap();
        assert!(stale_intact(&store.conn).unwrap(), "{remake} is repaired");
        assert_eq!(set(&store.conn), rule(&store.conn), "{remake}");
    }
}
/// The chief's R1: a `seat_lag` or `seat_head` of another shape beside a
/// summary repair. The drop that lets the fifth repair seed decides by
/// shape, not presence, and a `seat_head` the set's bodies cannot read is
/// remade by the seventh repair. The open is `Ok` and the set afterwards
/// equals the rule.
#[test]
fn a_summary_repair_with_a_seat_table_of_another_shape_reseeds() {
    for remake in [
        "DROP TABLE seat_lag; CREATE TABLE seat_lag (other TEXT PRIMARY KEY);",
        "DROP TABLE seat_head; CREATE TABLE seat_head (run_id TEXT PRIMARY KEY, shape TEXT);",
    ] {
        let (dir, store) = current_store();
        store
            .conn
            .execute_batch(&format!("{remake} DROP INDEX run_summary_page;"))
            .unwrap();
        drop(store);

        let store = Store::open(&dir.path().join("forge.db"))
            .unwrap_or_else(|error| panic!("open after {remake}: {error}"));
        assert!(stale_intact(&store.conn).unwrap(), "{remake} is repaired");
        assert_eq!(set(&store.conn), rule(&store.conn), "{remake}");
    }
}

/// The chief's R2: a summary row moved to another `run_id` is judged as an
/// insert is, so the run it moved to joins the set by the rule. A row moved
/// from a lagging run leaves the set too: no summary row names the run it
/// moved from any more.
#[test]
fn a_moved_summary_run_id_judges_the_new_run() {
    let (_dir, store) = crate::tests::store();
    for run_id in ["current", "headless", "seq-lag", "shape-lag"] {
        plant_run(&store.conn, run_id, "{}").unwrap();
    }
    current(&store.conn, "current");
    headless(&store.conn, "headless");
    current(&store.conn, "seq-lag");
    head(&store.conn, "seq-lag", -1, "h", SHAPE.as_str());
    current(&store.conn, "shape-lag");
    shape_lag(&store.conn, "shape-lag");

    store
        .conn
        .execute(
            "UPDATE run_summary SET run_id = 'renamed' WHERE run_id = 'current'",
            [],
        )
        .unwrap();
    assert_eq!(
        set(&store.conn),
        [
            "headless".to_string(),
            "renamed".to_string(),
            "seq-lag".to_string()
        ],
        "the new run has no head, so it joins"
    );
    assert_eq!(stale(&store.conn), rule(&store.conn), "with the shape arm");

    store
        .conn
        .execute(
            "UPDATE run_summary SET run_id = 'moved' WHERE run_id = 'headless'",
            [],
        )
        .unwrap();
    assert_eq!(
        set(&store.conn),
        [
            "moved".to_string(),
            "renamed".to_string(),
            "seq-lag".to_string()
        ],
        "the run it moved from leaves the set"
    );
    assert_eq!(stale(&store.conn), rule(&store.conn));
}

/// The chief's R3: the set's probe does not ask the summary head trigger's
/// body. A journal whose only damage is there still reads the set, because
/// the set is intact and the by-set statement answers exactly; the summary
/// repair keeps that body, not the set.
#[test]
fn the_sets_probe_does_not_read_the_summarys_head_trigger() {
    let (_dir, store) = current_store();
    plant_run(&store.conn, "headless", "{}").unwrap();
    headless(&store.conn, "headless");
    store
        .conn
        .execute_batch(
            "DROP TRIGGER run_summary_of_an_event;
             CREATE TRIGGER run_summary_of_an_event AFTER INSERT ON events
             BEGIN SELECT 1; END;",
        )
        .unwrap();
    assert!(
        !crate::summary::summary_intact(&store.conn).unwrap(),
        "the summary's own head trigger is another body"
    );
    assert!(
        stale_intact(&store.conn).unwrap(),
        "the set is still this build's"
    );
    assert_eq!(fallback(&store), rule(&store.conn));
}

/// A summary trigger of an older body is not this build's: the open
/// replaces it, so a journal whose trigger did not watch `run_id` gains the
/// body that does.
#[test]
fn an_older_summary_trigger_is_replaced_on_open() {
    let (dir, store) = current_store();
    store
        .conn
        .execute_batch(&format!(
            "DROP TRIGGER {SUMMARY_MOVED};
             CREATE TRIGGER {SUMMARY_MOVED}
                 AFTER UPDATE OF head_seq, head_hash ON run_summary
                 WHEN OLD.head_seq IS NOT NEW.head_seq
                   OR OLD.head_hash IS NOT NEW.head_hash
             BEGIN SELECT 1; END;"
        ))
        .unwrap();
    assert!(
        !stale_intact(&store.conn).unwrap(),
        "an older body is not this build's"
    );
    drop(store);

    let conn = repaired(&dir.path().join("forge.db"));
    assert!(stale_intact(&conn).unwrap(), "the repair replaced the body");
    conn.execute(
        "UPDATE run_summary SET run_id = 'renamed' WHERE run_id = 'run'",
        [],
    )
    .unwrap();
    assert_eq!(
        set(&conn),
        ["renamed".to_string()],
        "the new body watches it"
    );
}
