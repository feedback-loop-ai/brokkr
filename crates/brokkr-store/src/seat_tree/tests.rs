//! The `seat_trees` table and the store's append (decision 0104 ruling 6,
//! slice 5b): every `CHECK` refuses its case by name, the unique index and
//! both triggers bite, the append is once per attempt, and a lost index
//! over duplicate rows is refused by its key.

use std::path::Path;

use rusqlite::{params, Connection};

use super::*;
use crate::test_support::plant_run;
use crate::{Store, StoreError};

const RUN: &str = "run-a";
const EFFECT: &str = "effect-a";
const ATTEMPT: &str = "11111111-1111-4111-8111-111111111111";
const OTHER_ATTEMPT: &str = "22222222-2222-4222-8222-222222222222";
/// Two ids whose attempt order is the reverse of their `seq` order, so a
/// read without `ORDER BY seq` fails the order test.
const HIGH: &str = "ffffffff-ffff-4fff-8fff-ffffffffffff";
const LOW: &str = "00000000-0000-4000-8000-000000000000";
const ROOT: &str = "/run/scratch";

/// A store whose journal holds `RUN`, and one row planted by a raw insert.
fn store(dir: &Path) -> Store {
    let store = Store::open(&dir.join("forge.db")).unwrap();
    plant_run(&store.conn, RUN, "{}").unwrap();
    store
}

/// One raw `seat_trees` row, past the append: the statement each `CHECK`
/// and the unique index refuse.
fn insert(
    conn: &Connection,
    run: &str,
    effect: &str,
    attempt: &str,
    root: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO seat_trees (run_id, effect_id, attempt_id, root, recorded_at)
         VALUES (?1, ?2, ?3, ?4, 1)",
        params![run, effect, attempt, root],
    )
}

#[test]
fn every_seat_tree_check_refuses_its_case_by_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    // A NUL stops `GLOB` and `length()`, so only the byte length and the
    // root's byte scan refuse these: the tail after it is not read by them.
    let nul_attempt = format!("{OTHER_ATTEMPT}\0/../..");
    let nul_root = "/a/scratch\0/../..";
    let cases = [
        (RUN, "", ATTEMPT, ROOT, "effect_named"),
        (RUN, EFFECT, "..", ROOT, "attempt_is_uuid"),
        (RUN, EFFECT, "a/b", ROOT, "attempt_is_uuid"),
        (RUN, EFFECT, "", ROOT, "attempt_is_uuid"),
        (RUN, EFFECT, "not-a-uuid", ROOT, "attempt_is_uuid"),
        (
            RUN,
            EFFECT,
            nul_attempt.as_str(),
            "/x/scratch",
            "attempt_is_uuid",
        ),
        (RUN, EFFECT, ATTEMPT, "relative/scratch", "root_absolute"),
        (RUN, EFFECT, ATTEMPT, "/run/not-scratch", "root_is_scratch"),
        (RUN, EFFECT, ATTEMPT, "/run/scratch/", "root_is_scratch"),
        (RUN, EFFECT, ATTEMPT, nul_root, "root_has_no_nul"),
    ];
    for (run, effect, attempt, root, name) in cases {
        let error = insert(&store.conn, run, effect, attempt, root).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("CHECK constraint failed: {name}"),
            "{root}"
        );
    }
    assert_eq!(read_trees(&store.conn, RUN).unwrap(), vec![]);
}

#[test]
fn the_unique_index_refuses_a_second_row_for_one_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    insert(&store.conn, RUN, EFFECT, ATTEMPT, ROOT).unwrap();
    let error = insert(&store.conn, RUN, "other-effect", ATTEMPT, ROOT).unwrap_err();
    assert_eq!(
        error.to_string(),
        "UNIQUE constraint failed: seat_trees.run_id, seat_trees.attempt_id"
    );
    assert_eq!(read_trees(&store.conn, RUN).unwrap().len(), 1);
}

#[test]
fn both_triggers_refuse_an_update_and_a_delete() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    insert(&store.conn, RUN, EFFECT, ATTEMPT, ROOT).unwrap();
    for statement in [
        "UPDATE seat_trees SET root = '/other/scratch'",
        "DELETE FROM seat_trees",
    ] {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(
            error.to_string(),
            "seat trees are append-only",
            "{statement}"
        );
    }
    assert_eq!(read_trees(&store.conn, RUN).unwrap().len(), 1);
}

#[test]
fn a_row_for_a_run_that_does_not_exist_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    let error = insert(&store.conn, "never-started", EFFECT, ATTEMPT, ROOT).unwrap_err();
    assert_eq!(error.to_string(), "FOREIGN KEY constraint failed");
}

#[test]
fn the_same_append_twice_writes_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let root = Path::new(ROOT);
    let before = time::OffsetDateTime::now_utc().unix_timestamp();
    assert_eq!(
        store.record_seat_tree(RUN, EFFECT, ATTEMPT, root).unwrap(),
        SeatTreeRecord::Written
    );
    assert_eq!(
        store.record_seat_tree(RUN, EFFECT, ATTEMPT, root).unwrap(),
        SeatTreeRecord::Present
    );
    let after = time::OffsetDateTime::now_utc().unix_timestamp();
    let rows = store.seat_trees(RUN).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].effect_id.as_str(),
            rows[0].attempt_id.as_str(),
            rows[0].root.as_path()
        ),
        (EFFECT, ATTEMPT, root)
    );
    // The append reads the clock inside the window the test holds, so a
    // constant time fails.
    assert!(
        (before..=after).contains(&rows[0].recorded_at),
        "recorded_at {} is outside {}..={}",
        rows[0].recorded_at,
        before,
        after
    );
}

#[test]
fn an_append_that_differs_in_effect_or_root_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    store
        .record_seat_tree(RUN, EFFECT, ATTEMPT, Path::new(ROOT))
        .unwrap();
    let before = store.seat_trees(RUN).unwrap();
    for (effect, root) in [("other-effect", ROOT), (EFFECT, "/other/scratch")] {
        let refused = store
            .record_seat_tree(RUN, effect, ATTEMPT, Path::new(root))
            .unwrap_err();
        assert_eq!(
            refused.to_string(),
            "run 'run-a' already records a seat tree for attempt \
             '11111111-1111-4111-8111-111111111111' with another effect or root; this one was \
             not written"
        );
        assert!(matches!(
            refused,
            StoreError::SeatTree(SeatTreeRefusal::Mismatch { .. })
        ));
        assert_eq!(store.seat_trees(RUN).unwrap(), before);
    }
}

#[test]
fn an_append_the_table_refuses_is_the_table_s_error_not_a_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let nul = format!("{OTHER_ATTEMPT}\0/../..");
    for attempt in ["..".to_string(), nul] {
        let refused = store
            .record_seat_tree(RUN, EFFECT, &attempt, Path::new(ROOT))
            .unwrap_err();
        assert_eq!(
            refused.to_string(),
            "sqlite: CHECK constraint failed: attempt_is_uuid"
        );
        assert_eq!(store.seat_trees(RUN).unwrap(), vec![]);
    }
}

#[test]
fn an_unknown_queue_version_writes_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    store
        .conn
        .execute("UPDATE meta SET value = '6' WHERE key = 'queue_schema'", [])
        .unwrap();
    assert_eq!(
        store
            .record_seat_tree(RUN, EFFECT, ATTEMPT, Path::new(ROOT))
            .unwrap(),
        SeatTreeRecord::Unrecorded
    );
    let count: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM seat_trees", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let refused = store.seat_trees(RUN).unwrap_err();
    assert_eq!(refused.to_string(), "queue storage 6 unsupported (want 5)");
}

#[test]
fn an_older_queue_reads_no_seat_trees_and_records_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = store(dir.path());
    store
        .conn
        .execute_batch(
            "UPDATE meta SET value = '4' WHERE key = 'queue_schema';
             DROP TABLE seat_trees;",
        )
        .unwrap();
    drop(store);
    let mut reader = Store::open_read_only(&path).unwrap();
    assert_eq!(reader.seat_trees(RUN).unwrap(), vec![]);
    assert_eq!(
        reader
            .record_seat_tree(RUN, EFFECT, ATTEMPT, Path::new(ROOT))
            .unwrap(),
        SeatTreeRecord::Unrecorded
    );
    assert_eq!(reader.seat_trees(RUN).unwrap(), vec![]);
}

#[test]
fn a_journal_with_no_queue_reads_no_seat_trees() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = store(dir.path());
    store
        .conn
        .execute("DELETE FROM meta WHERE key = 'queue_schema'", [])
        .unwrap();
    drop(store);
    let reader = Store::open_read_only(&path).unwrap();
    assert_eq!(reader.seat_trees(RUN).unwrap(), vec![]);
}

#[test]
fn the_read_returns_a_run_s_rows_in_seq_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    // The higher id goes first so the unique index's attempt order is the
    // reverse of `seq`: only `ORDER BY seq` makes this read right.
    for attempt in [HIGH, LOW] {
        store
            .record_seat_tree(RUN, EFFECT, attempt, Path::new(ROOT))
            .unwrap();
    }
    plant_run(&store.conn, "run-b", "{}").unwrap();
    store
        .record_seat_tree("run-b", EFFECT, ATTEMPT, Path::new("/other/scratch"))
        .unwrap();
    let rows = store.seat_trees(RUN).unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (row.seq, row.attempt_id.as_str()))
            .collect::<Vec<_>>(),
        [(1, HIGH), (2, LOW)]
    );
    assert_eq!(store.seat_trees("run-b").unwrap().len(), 1);
}

#[test]
fn a_lost_unique_index_over_a_duplicate_refuses_by_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = store(dir.path());
    store
        .record_seat_tree(RUN, EFFECT, ATTEMPT, Path::new(ROOT))
        .unwrap();
    store
        .conn
        .execute_batch("DROP INDEX seat_trees_once")
        .unwrap();
    assert!(!crate::queue::queue_intact(&store.conn).unwrap());
    insert(&store.conn, RUN, "other-effect", ATTEMPT, "/other/scratch").unwrap();
    drop(store);

    for _ in 0..2 {
        let refused = match Store::open(&path).err().expect("refused") {
            StoreError::SeatTree(refused) => refused,
            other => panic!("not a seat-tree refusal: {other}"),
        };
        assert_eq!(
            refused,
            SeatTreeRefusal::Duplicate {
                run_id: RUN.into(),
                attempt_id: ATTEMPT.into(),
            }
        );
        assert_eq!(
            refused.to_string(),
            "the journal's seat trees hold run 'run-a' attempt \
             '11111111-1111-4111-8111-111111111111' more than once; the lost unique index is \
             not rebuilt over them"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_root_that_is_not_utf8_is_refused_and_writes_nothing() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    let root = Path::new(OsStr::from_bytes(b"/run/\xff/scratch"));
    let refused = store
        .record_seat_tree(RUN, EFFECT, ATTEMPT, root)
        .unwrap_err();
    assert!(matches!(
        &refused,
        StoreError::SeatTree(SeatTreeRefusal::RootUnnameable(_))
    ));
    assert_eq!(
        refused.to_string(),
        "the seat tree root '/run/\u{fffd}/scratch' is not UTF-8, so the journal cannot name it"
    );
    assert_eq!(store.seat_trees(RUN).unwrap(), vec![]);
}
