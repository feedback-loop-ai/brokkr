//! The snapshot's own binding: one state across a writer's commits, nested
//! transactions that end inside it, and a read-only open that refuses
//! nothing.

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

/// Every read inside a snapshot sees the journal as it stood when the
/// snapshot opened, whatever a second connection commits between them, and
/// the read after it sees the commit.
#[test]
fn a_snapshot_reads_one_state_across_a_writers_commit() {
    let (_dir, store, writer) = journal_and_writer();
    let seen = store.read_snapshot(|| {
        let before = runs(&store);
        plant_run(&writer, "two", "{}").unwrap();
        Ok::<_, StoreError>((before, runs(&store)))
    });
    assert_eq!(seen.unwrap(), (1, 1));
    assert_eq!(runs(&store), 2);
    assert!(store.conn.is_autocommit(), "the pin is released with it");
}

/// A read transaction opened and ended inside the snapshot, as every
/// store method that opens its own does, neither refuses nor releases it;
/// a snapshot inside one joins it and reads its state.
#[test]
fn a_nested_transaction_and_a_nested_snapshot_join_it() {
    let (_dir, store, writer) = journal_and_writer();
    let seen = store.read_snapshot(|| {
        let own = store.as_one_read(|| Ok::<_, StoreError>(runs(&store)))?;
        plant_run(&writer, "two", "{}").unwrap();
        let after_own = runs(&store);
        let inner = store.read_snapshot(|| Ok::<_, StoreError>(runs(&store)))?;
        plant_run(&writer, "three", "{}").unwrap();
        let after_inner = runs(&store);
        let inside_one_read = store.as_one_read(|| Ok::<_, StoreError>(runs(&store)))?;
        Ok::<_, StoreError>([own, after_own, inner, after_inner, inside_one_read])
    });
    assert_eq!(seen.unwrap(), [1, 1, 1, 1, 1]);
    assert_eq!(runs(&store), 3);
}

/// A read-only open reads under a snapshot as a writable one does, and the
/// read's own refusal is the snapshot's result.
#[test]
fn a_read_only_open_is_not_refused_and_a_refusal_is_the_reads() {
    let (_dir, store, _writer) = journal_and_writer();
    let reader = Store::open_read_only(store.path()).unwrap();
    let counted = reader.read_snapshot(|| Ok::<_, StoreError>(runs(&reader)));
    assert_eq!(counted.unwrap(), 1);
    let refused = reader.read_snapshot(|| Err::<(), _>(StoreError::RunNotFound("gone".into())));
    assert!(matches!(refused, Err(StoreError::RunNotFound(ref run)) if run == "gone"));
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
    let inside = store.read_snapshot(|| {
        let first = store.run_read("r", &head, 0)?;
        let seats = store.seat_rows("r")?;
        let whole = store.open_kept("r")?.1.len();
        append(
            &mut peer,
            "r",
            EventType::PhaseEntered,
            json!({"phase": "x"}),
        );
        peer.refold("r", &mut |_| {}).unwrap();
        let second = store.run_read("r", &head, 0)?;
        Ok::<_, StoreError>((first, seats, whole, second))
    });
    let (first, seats, whole, second) = inside.unwrap();
    assert!(matches!(first, RunRead::Current { .. }), "{first:?}");
    assert!(matches!(second, RunRead::Current { .. }), "{second:?}");
    assert!(matches!(seats, SeatRows::Current(_)), "{seats:?}");
    assert_eq!(whole, 2);
    let after = store.run_read("r", &head, 0).unwrap();
    assert!(matches!(after, RunRead::Kept), "{after:?}");
}
