use std::path::Path;
use std::time::{Duration, Instant};

use crate::test_support::plant_run;
use crate::{Damage, Store, StoreError};

/// What a completer spends on a peer's lock.
const PATIENCE: Duration = Duration::from_millis(50);

/// Every name in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A journal at `db` holding `run-a`, its writer closed.
fn journal(db: &Path) {
    let store = Store::open(db).unwrap();
    plant_run(&store.conn, "run-a", "{}").unwrap();
}

/// The ids `store`'s page lists.
fn listed(store: &Store) -> Vec<String> {
    let page = store.recent_runs(50).unwrap();
    page.into_iter().map(|(run, _)| run).collect()
}

/// With no writer, the journal is read and nothing is left beside it:
/// the read-only open this replaces leaves a `-wal` and a `-shm`.
#[test]
fn a_journal_no_writer_holds_is_read_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    journal(&db);
    assert_eq!(names(dir.path()), ["forge.db"], "the writer closed clean");

    let store = Store::open_read_only_untouched(&db, PATIENCE).unwrap();
    assert_eq!(listed(&store), ["run-a"]);
    drop(store);
    assert_eq!(names(dir.path()), ["forge.db"], "no file was created");
}

/// Beside a live writer, its uncheckpointed rows are read through the
/// side files it holds, and no other file appears.
#[test]
fn beside_a_writer_its_wal_is_read_and_no_file_is_added() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let writer = Store::open(&db).unwrap();
    plant_run(&writer.conn, "run-live", "{}").unwrap();
    let before = names(dir.path());
    assert_eq!(before, ["forge.db", "forge.db-shm", "forge.db-wal"]);

    let store = Store::open_read_only_untouched(&db, PATIENCE).unwrap();
    assert_eq!(listed(&store), ["run-live"], "the WAL's row is read");
    drop(store);
    assert_eq!(names(dir.path()), before);
}

/// A missing journal is refused, as SQLite cannot open it, and nothing
/// is created in its place.
#[test]
fn a_missing_journal_is_refused_and_nothing_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let refused = Store::open_read_only_untouched(&dir.path().join("forge.db"), PATIENCE);
    let Err(StoreError::Sqlite(error)) = refused else {
        panic!("not SQLite's refusal: {:?}", refused.map(|_| ()));
    };
    assert_eq!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::CannotOpen)
    );
    assert_eq!(names(dir.path()), Vec::<String>::new());
}

/// A file that is no database is refused as damaged, and left as it was.
#[test]
fn a_file_that_is_no_journal_is_refused_as_damaged() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    std::fs::write(&db, "not a database, only text long enough to be read").unwrap();
    let refused = Store::open_read_only_untouched(&db, PATIENCE).map(|_| ());
    assert!(
        matches!(
            refused,
            Err(StoreError::Damaged {
                damage: Damage::NotADatabase,
                ..
            })
        ),
        "{refused:?}"
    );
    assert_eq!(names(dir.path()), ["forge.db"]);
}

/// A peer holding the journal exclusively refuses the read once the
/// patience is spent, never after the writer's thirty seconds.
#[test]
fn a_journal_a_peer_holds_exclusively_refuses_within_the_patience() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    journal(&db);
    let peer = rusqlite::Connection::open(&db).unwrap();
    peer.execute_batch("PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;")
        .unwrap();
    peer.execute("INSERT INTO meta VALUES ('peer', 'holds')", [])
        .unwrap();

    // The page-cache pragma meets the lock first, on the connection's
    // own busy handler, which the patience bounds too.
    let started = Instant::now();
    let refused = Store::open_read_only_untouched(&db, PATIENCE).map(|_| ());
    let Err(StoreError::Sqlite(error)) = &refused else {
        panic!("not SQLite's busy: {refused:?}");
    };
    assert_eq!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy)
    );
    assert!(started.elapsed() < Duration::from_secs(5), "it waited");
}

/// A directory whose name holds what a URI reads as a query, a fragment
/// or an escape opens the journal inside it, and nothing else.
#[test]
fn a_path_holding_uri_characters_opens_that_journal() {
    let dir = tempfile::tempdir().unwrap();
    let odd = dir.path().join("a?mode=rwc#b%41 c");
    std::fs::create_dir(&odd).unwrap();
    let db = odd.join("forge.db");
    journal(&db);

    let store = Store::open_read_only_untouched(&db, PATIENCE).unwrap();
    assert_eq!(listed(&store), ["run-a"]);
    assert_eq!(names(dir.path()), ["a?mode=rwc#b%41 c"]);
}
