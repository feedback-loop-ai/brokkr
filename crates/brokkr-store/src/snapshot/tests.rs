//! The snapshot's own binding: one state across a writer's commits for as
//! long as its guard is held, nested transactions that end inside it, and
//! a read-only open that refuses nothing.

use brokkr_core::envelope::EventType;
use rusqlite::Connection;
use serde_json::json;

use super::*;
use crate::test_support::{append, opened, plant_run};
use crate::tests::store;
use crate::{RunRead, SeatRows};

/// How many runs the journal holds as this connection reads it now.
fn runs(store: &Store) -> i64 {
    store
        .conn
        .query_row("SELECT count(*) FROM runs", [], |row| row.get(0))
        .unwrap()
}

/// A journal holding the run `one`, and a second connection to write it by.
fn journal_and_writer() -> (tempfile::TempDir, Store, Connection) {
    let (dir, store) = store();
    plant_run(&store.conn, "one", "{}").unwrap();
    let writer = Connection::open(store.path()).unwrap();
    (dir, store, writer)
}

/// Every read while the guard is held sees the journal as it stood when
/// the snapshot was taken, whatever a second connection commits between
/// them, and the read after the guard is dropped sees the commits.
#[test]
fn a_snapshot_reads_one_state_across_a_writers_commits_until_it_is_dropped() {
    let (_dir, store, writer) = journal_and_writer();
    let snapshot = store.snapshot().unwrap();
    let before = runs(&store);
    plant_run(&writer, "two", "{}").unwrap();
    let between = runs(&store);
    plant_run(&writer, "three", "{}").unwrap();
    assert_eq!((before, between, runs(&store)), (1, 1, 1));
    drop(snapshot);
    assert_eq!(runs(&store), 3);
    assert!(store.conn.is_autocommit(), "the pin is released with it");
}

/// A read transaction opened and ended while the guard is held, as every
/// store method that opens its own does, neither refuses nor releases it;
/// a snapshot taken while one is held joins it and reads its state, and
/// dropping it releases nothing.
#[test]
fn a_nested_transaction_and_a_nested_snapshot_join_it() {
    let (_dir, store, writer) = journal_and_writer();
    let snapshot = store.snapshot().unwrap();
    let own = store
        .as_one_read(|| Ok::<_, StoreError>(runs(&store)))
        .unwrap();
    plant_run(&writer, "two", "{}").unwrap();
    let after_own = runs(&store);
    let inner = store.snapshot().unwrap();
    let in_inner = runs(&store);
    drop(inner);
    plant_run(&writer, "three", "{}").unwrap();
    let after_inner = runs(&store);
    let inside_one_read = store
        .as_one_read(|| Ok::<_, StoreError>(runs(&store)))
        .unwrap();
    assert_eq!(
        [own, after_own, in_inner, after_inner, inside_one_read],
        [1, 1, 1, 1, 1]
    );
    drop(snapshot);
    assert_eq!(runs(&store), 3);
}

/// A read-only open is pinned as a writable one is.
#[test]
fn a_read_only_open_is_not_refused() {
    let (_dir, store, writer) = journal_and_writer();
    let reader = Store::open_read_only(store.path()).unwrap();
    let snapshot = reader.snapshot().unwrap();
    plant_run(&writer, "two", "{}").unwrap();
    assert_eq!(runs(&reader), 1);
    drop(snapshot);
    assert_eq!(runs(&reader), 2);
    assert!(reader.conn.is_autocommit());
}

/// The methods that open a read transaction of their own read inside the
/// snapshot, and read its state: a run read at the head its opening
/// verified is still [`RunRead::Current`] while a peer appends past it,
/// and is [`RunRead::Kept`] once the snapshot ends. The seat rows and the
/// whole opening are not refused either.
#[test]
fn a_store_method_with_its_own_transaction_reads_the_snapshots_state() {
    let (_dir, mut store) = crate::tests::store();
    opened(&mut store, "r", "work", &[]);
    let head = store.open_run("r").unwrap();
    let mut peer = Store::open(store.path()).unwrap();
    let snapshot = store.snapshot().unwrap();
    let first = store.run_read("r", &head, 0).unwrap();
    let seats = store.seat_rows("r").unwrap();
    let whole = store.open_kept("r").unwrap().1.len();
    append(
        &mut peer,
        "r",
        EventType::PhaseEntered,
        json!({"phase": "x"}),
    );
    peer.refold("r", &mut |_| {}).unwrap();
    let second = store.run_read("r", &head, 0).unwrap();
    drop(snapshot);
    assert!(matches!(first, RunRead::Current { .. }), "{first:?}");
    assert!(matches!(second, RunRead::Current { .. }), "{second:?}");
    assert!(matches!(seats, SeatRows::Current(_)), "{seats:?}");
    assert_eq!(whole, 2);
    let after = store.run_read("r", &head, 0).unwrap();
    assert!(matches!(after, RunRead::Kept), "{after:?}");
}
