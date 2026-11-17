//! The `tree_sweeps` table and the store's record (decision 0104 ruling 8,
//! slice 5c1): every `CHECK` refuses its case by name, the unique index and
//! both triggers bite, a complete removal closes its seat tree, and a queue
//! this binary does not write here is left as found.

use std::path::Path;

use brokkr_core::envelope::EventType;
use rusqlite::types::Value;
use rusqlite::{params, Connection};
use serde_json::json;
use time::{Duration, OffsetDateTime};

use super::*;
use crate::{CooldownAppend, Cooling, SeatTreeRecord, Store, StoreError};

const RUN: &str = "run-a";
const EFFECT: &str = "effect-a";
const ATTEMPT: &str = "11111111-1111-4111-8111-111111111111";
const PATH: &str = "/run/scratch/11111111-1111-4111-8111-111111111111";

/// A store whose journal holds `RUN`.
fn store(dir: &Path) -> Store {
    crate::test_support::seat_tree_store(dir, RUN)
}

/// A store whose journal holds one seat tree and one dispatch holder, and
/// the seat tree's `seq` and the holder's generation.
fn seated(dir: &Path) -> (Store, i64, i64) {
    let mut store = store(dir);
    store
        .record_seat_tree(RUN, EFFECT, ATTEMPT, Path::new("/run/scratch"))
        .unwrap();
    let seq = store.seat_trees(RUN).unwrap()[0].seq;
    let generation = store.dispatch_hold(1).unwrap().0;
    (store, seq, generation)
}

/// One raw `tree_sweeps` row, past the append: the statement each `CHECK`,
/// the foreign key and the unique index refuse. The two integer columns
/// take a [`Value`], so a text planted in one is read by the table's own
/// check.
fn insert(
    conn: &Connection,
    kind: &str,
    tree_seq: Option<i64>,
    path: &str,
    bytes: Value,
    removed_at: Value,
    generation: i64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO tree_sweeps (kind, tree_seq, path, bytes, removed_at, generation)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![kind, tree_seq, path, bytes, removed_at, generation],
    )
}

fn integer(value: i64) -> Value {
    Value::Integer(value)
}

/// One valid scratch row for `seq`, whose bytes the caller chooses.
fn planted(store: &Store, seq: i64, generation: i64, bytes: Value) {
    insert(
        &store.conn,
        "scratch",
        Some(seq),
        "/run/scratch",
        bytes,
        integer(generation),
        generation,
    )
    .unwrap();
}

#[test]
fn every_tree_sweep_check_refuses_its_case_by_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let (store, seq, generation) = seated(dir.path());
    let nul = "/run/scratch\0/../..";
    let cases = [
        (
            "other",
            Some(seq),
            "/run/scratch",
            integer(0),
            integer(1),
            "sweep_kind_named",
        ),
        (
            "scratch",
            None,
            "/run/scratch",
            Value::Null,
            integer(1),
            "sweep_tree_matches_kind",
        ),
        (
            "hands",
            Some(seq),
            "/run/scratch",
            Value::Null,
            integer(1),
            "sweep_tree_matches_kind",
        ),
        (
            "hands",
            None,
            "relative/scratch",
            Value::Null,
            integer(1),
            "sweep_path_absolute",
        ),
        (
            "hands",
            None,
            nul,
            Value::Null,
            integer(1),
            "sweep_path_has_no_nul",
        ),
        (
            "hands",
            None,
            "/run/scratch",
            integer(-1),
            integer(1),
            "sweep_bytes_whole",
        ),
        (
            "hands",
            None,
            "/run/scratch",
            Value::Text("x".into()),
            integer(1),
            "sweep_bytes_whole",
        ),
        (
            "hands",
            None,
            "/run/scratch",
            Value::Null,
            Value::Text("x".into()),
            "sweep_removed_is_seconds",
        ),
    ];
    for (kind, tree_seq, path, bytes, removed_at, name) in cases {
        let error = insert(
            &store.conn,
            kind,
            tree_seq,
            path,
            bytes,
            removed_at,
            generation,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("CHECK constraint failed: {name}"),
            "{path} {kind}"
        );
    }
    assert_eq!(store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_generation_no_holder_names_is_refused_by_the_foreign_key() {
    let dir = tempfile::tempdir().unwrap();
    let (store, seq, generation) = seated(dir.path());
    let error = insert(
        &store.conn,
        "scratch",
        Some(seq),
        "/run/scratch",
        Value::Null,
        integer(generation),
        99,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "FOREIGN KEY constraint failed");
}

#[test]
fn both_triggers_refuse_an_update_and_a_delete() {
    let dir = tempfile::tempdir().unwrap();
    let (store, seq, generation) = seated(dir.path());
    planted(&store, seq, generation, integer(10));
    for statement in [
        "UPDATE tree_sweeps SET bytes = 20",
        "DELETE FROM tree_sweeps",
    ] {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(
            error.to_string(),
            "tree sweeps are append-only",
            "{statement}"
        );
    }
    assert_eq!(store.tree_sweeps().unwrap().len(), 1);
}

#[test]
fn the_unique_index_refuses_a_second_row_for_one_seat_tree() {
    let dir = tempfile::tempdir().unwrap();
    let (store, seq, generation) = seated(dir.path());
    planted(&store, seq, generation, Value::Null);
    let error = insert(
        &store.conn,
        "scratch",
        Some(seq),
        "/run/scratch",
        integer(1),
        integer(generation),
        generation,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "UNIQUE constraint failed: tree_sweeps.tree_seq"
    );
}

#[test]
fn a_recorded_sweep_closes_its_seat_tree() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, seq, generation) = seated(dir.path());
    assert_eq!(store.open_seat_trees().unwrap().len(), 1);
    let before = OffsetDateTime::now_utc().unix_timestamp();
    assert_eq!(
        store
            .record_tree_sweep(Swept::Scratch(seq), Path::new(PATH), Some(4096), generation)
            .unwrap(),
        TreeSweepRecord::Written
    );
    let after = OffsetDateTime::now_utc().unix_timestamp();
    assert_eq!(store.open_seat_trees().unwrap(), vec![]);
    let rows = store.tree_sweeps().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].kind,
            rows[0].path.as_path(),
            rows[0].bytes,
            rows[0].generation,
        ),
        (Swept::Scratch(seq), Path::new(PATH), Some(4096), generation,)
    );
    assert!(
        (before..=after).contains(&rows[0].removed_at),
        "removed_at {} is outside {}..={}",
        rows[0].removed_at,
        before,
        after
    );
    // A hands tree closes no seat tree, and two of them are distinct rows.
    for path in ["/tmp/brokkr-hands-serve-1-x", "/tmp/brokkr-hands-exec-2-y"] {
        assert_eq!(
            store
                .record_tree_sweep(Swept::Hands, Path::new(path), None, generation)
                .unwrap(),
            TreeSweepRecord::Written
        );
    }
    assert_eq!(store.tree_sweeps().unwrap().len(), 3);
}

#[test]
fn a_second_record_for_one_seat_tree_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, seq, generation) = seated(dir.path());
    let path = Path::new(PATH);
    store
        .record_tree_sweep(Swept::Scratch(seq), path, Some(1), generation)
        .unwrap();
    let refused = store
        .record_tree_sweep(Swept::Scratch(seq), path, Some(2), generation)
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        format!(
            "seat tree {seq} already has a complete removal recorded; this one was not recorded"
        )
    );
    assert!(matches!(
        refused,
        StoreError::SeatTree(SeatTreeRefusal::SweepClosed { tree_seq }) if tree_seq == seq
    ));
    assert_eq!(store.tree_sweeps().unwrap().len(), 1);
}

#[test]
fn a_queue_before_the_sweep_table_reads_its_seat_trees_as_open() {
    // A version 5 queue holds the trees but no sweep to close one; a version
    // 4 queue holds neither.
    for (version, holds_trees) in [("5", true), ("4", false)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("forge.db");
        let (store, _, _) = seated(dir.path());
        let mut surgery = format!(
            "UPDATE meta SET value = '{version}' WHERE key = 'queue_schema'; \
             DROP TABLE tree_sweeps;"
        );
        if !holds_trees {
            surgery.push_str(" DROP TABLE seat_trees;");
        }
        store.conn.execute_batch(&surgery).unwrap();
        drop(store);
        let reader = Store::open_read_only(&path).unwrap();
        let trees = reader.open_seat_trees().unwrap();
        assert_eq!(trees.len(), usize::from(holds_trees), "{version}");
        assert_eq!(reader.tree_sweeps().unwrap(), vec![], "{version}");
    }
}

#[test]
fn a_journal_with_no_seat_trees_reads_none() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(dir.path());
    assert_eq!(store.open_seat_trees().unwrap(), vec![]);
}

#[test]
fn an_unknown_queue_version_writes_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, seq, generation) = seated(dir.path());
    store
        .conn
        .execute("UPDATE meta SET value = '8' WHERE key = 'queue_schema'", [])
        .unwrap();
    assert_eq!(
        store
            .record_tree_sweep(
                Swept::Scratch(seq),
                Path::new("/run/scratch/x"),
                None,
                generation,
            )
            .unwrap(),
        TreeSweepRecord::Unrecorded
    );
    let count: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM tree_sweeps", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let refused = store.open_seat_trees().unwrap_err();
    assert_eq!(refused.to_string(), "queue storage 8 unsupported (want 7)");
}

#[test]
fn a_lost_unique_index_over_a_duplicate_refuses_by_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (store, seq, generation) = seated(dir.path());
    store
        .conn
        .execute_batch("DROP INDEX tree_sweeps_once")
        .unwrap();
    assert!(!crate::queue::queue_intact(&store.conn).unwrap());
    for bytes in [1, 2] {
        insert(
            &store.conn,
            "scratch",
            Some(seq),
            "/run/scratch/x",
            integer(bytes),
            integer(generation),
            generation,
        )
        .unwrap();
    }
    drop(store);
    let refused = match Store::open(&path).err().expect("refused") {
        StoreError::SeatTree(refused) => refused,
        other => panic!("not a seat-tree refusal: {other}"),
    };
    assert_eq!(refused, SeatTreeRefusal::SweepDuplicate { tree_seq: seq });
    assert_eq!(
        refused.to_string(),
        format!(
            "the journal's tree sweeps hold seat tree {seq} more than once; the lost unique \
             index is not rebuilt over them"
        )
    );
}

#[test]
fn a_kind_outside_the_closed_set_is_refused_when_read() {
    let dir = tempfile::tempdir().unwrap();
    let (store, seq, generation) = seated(dir.path());
    // The table's CHECK makes this unreachable from a journal this binary
    // wrote; the pragma plants it so the reader's own refusal is observed.
    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    insert(
        &store.conn,
        "other",
        Some(seq),
        "/run/scratch/x",
        Value::Null,
        integer(generation),
        generation,
    )
    .unwrap();
    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = OFF")
        .unwrap();
    let refused = store.tree_sweeps().unwrap_err();
    assert_eq!(
        refused.to_string(),
        "tree sweep 1 records kind 'other', which is neither 'scratch' nor 'hands'"
    );
}

#[cfg(unix)]
#[test]
fn a_swept_path_that_is_not_utf8_is_refused_and_writes_nothing() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let (mut store, seq, generation) = seated(dir.path());
    let path = Path::new(OsStr::from_bytes(b"/run/\xff/scratch"));
    let refused = store
        .record_tree_sweep(Swept::Scratch(seq), path, None, generation)
        .unwrap_err();
    assert!(matches!(
        &refused,
        StoreError::SeatTree(SeatTreeRefusal::SweepUnnameable(_))
    ));
    assert_eq!(
        refused.to_string(),
        "the swept path '/run/\u{fffd}/scratch' is not UTF-8, so the journal cannot name it"
    );
    assert_eq!(store.tree_sweeps().unwrap(), vec![]);
}

/// A terminal event's cool-down rows and a seat tree both write on a v6
/// journal: `cool_downs_writable` and `record_once` key to this binary's
/// version, and a v4- or v5-only check would silently drop both.
#[test]
fn a_v6_journal_writes_its_cool_downs_and_seat_trees() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(dir.path());
    store.dispatch_hold(1).unwrap();
    let until = OffsetDateTime::now_utc() + Duration::hours(1);
    let cooling = Cooling {
        site: "work".into(),
        provider: "claude".into(),
        route: "first".into(),
        until_at: until,
        reason: "rate_limit: slow down".into(),
        retry_after_seconds: Some(30),
        rounds: 1,
        held: None,
    };
    store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: RUN,
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({"effect_id": EFFECT, "error": "the provider's words"}),
                causation_id: None,
                attempt_id: None,
            },
            &[cooling],
        )
        .unwrap();
    assert_eq!(
        store
            .record_seat_tree(RUN, EFFECT, ATTEMPT, Path::new("/run/scratch"))
            .unwrap(),
        SeatTreeRecord::Written
    );
    let count = |table: &str| -> i64 {
        store
            .conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    assert_eq!((count("provider_cooldowns"), count("seat_trees")), (1, 1));
    assert_eq!(store.seat_trees(RUN).unwrap().len(), 1);
    assert_eq!(store.open_seat_trees().unwrap().len(), 1);
}
