use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::test_support::plant_run;
use crate::{Damage, Store, StoreError};

/// What a completer spends on a peer's lock.
const PATIENCE: Duration = Duration::from_millis(50);

/// The most a refused read may take: its patience, with room for a loaded
/// machine, and far below a second, so a patience of a second fails.
const WITHIN: Duration = Duration::from_millis(500);

/// Every name in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let found: std::collections::BTreeSet<String> = (std::fs::read_dir(dir).unwrap())
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    found.into_iter().collect()
}

/// A journal at `db` holding `run-a`, its writer closed.
fn journal(db: &Path) {
    let store = Store::open(db).unwrap();
    plant_run(&store.conn, "run-a", "{}").unwrap();
}

/// [`journal`], then turned to a rollback-mode journal, its writer closed.
fn rollback_journal(db: &Path) {
    journal(db);
    let conn = rusqlite::Connection::open(db).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "delete");
}

/// The ids the page lists, read untouched.
fn ids(store: &Store) -> Result<Vec<String>, StoreError> {
    let page = store.recent_runs(50)?;
    Ok(page.into_iter().map(|(run, _)| run).collect())
}

/// The ids the journal at `db` lists, read untouched.
fn listed(db: &Path) -> Result<Vec<String>, StoreError> {
    Store::read_untouched(db, PATIENCE, ids)
}

/// Is `refused` SQLite's busy, the refusal of a journal a writer holds?
fn busy<T: std::fmt::Debug>(refused: &Result<T, StoreError>) -> bool {
    match refused {
        Err(StoreError::Sqlite(error)) => {
            error.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy)
        }
        _ => false,
    }
}

/// With no writer, the journal is read and nothing is left beside it:
/// the read-only open this replaces leaves a `-wal` and a `-shm`.
#[test]
fn a_journal_no_writer_holds_is_read_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    journal(&db);
    assert_eq!(names(dir.path()), ["forge.db"], "the writer closed clean");

    assert_eq!(listed(&db).unwrap(), ["run-a"]);
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

    assert_eq!(listed(&db).unwrap(), ["run-live"], "the WAL's row is read");
    assert_eq!(names(dir.path()), before);
}

/// A `--db` that is a link is read as SQLite opens it: the live writer's
/// side files stand beside the link's target, and its rows are read
/// there, never missed by looking beside the link.
#[cfg(unix)]
#[test]
fn a_linked_journal_reads_the_wal_beside_its_target() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let db = real.join("forge.db");
    let writer = Store::open(&db).unwrap();
    plant_run(&writer.conn, "run-live", "{}").unwrap();
    let link = dir.path().join("link.db");
    std::os::unix::fs::symlink(&db, &link).unwrap();
    let (beside_link, beside_target) = (names(dir.path()), names(&real));

    assert_eq!(listed(&link).unwrap(), ["run-live"], "the target's WAL");
    assert_eq!(names(dir.path()), beside_link, "nothing beside the link");
    assert_eq!(names(&real), beside_target, "nothing beside the target");
}

/// A rollback-mode journal no writer holds is read under SQLite's own
/// locks, which leave no file in that mode.
#[test]
fn a_rollback_journal_no_writer_holds_is_read_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    rollback_journal(&db);
    assert_eq!(names(dir.path()), ["forge.db"]);

    assert_eq!(listed(&db).unwrap(), ["run-a"]);
    assert_eq!(names(dir.path()), ["forge.db"], "no file was created");
}

/// A rollback-mode journal a peer holds exclusively, with nothing yet
/// written beside it, is never read lock-free: the read meets the lock
/// and refuses once the patience is spent (decision 0115 ruling 7).
#[test]
fn a_rollback_journal_a_peer_holds_completes_nothing_within_the_patience() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    rollback_journal(&db);
    let peer = rusqlite::Connection::open(&db).unwrap();
    peer.execute_batch("PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;")
        .unwrap();
    assert_eq!(names(dir.path()), ["forge.db"], "only the lock is held");

    let started = Instant::now();
    let refused = listed(&db);
    assert!(
        matches!(refused, Err(StoreError::Contended { .. })) || busy(&refused),
        "{refused:?}"
    );
    assert!(started.elapsed() < WITHIN, "it waited past the patience");
    assert_eq!(names(dir.path()), ["forge.db"]);
}

/// A rollback journal beside the journal means a writer holds it or left
/// it hot for the next writer to roll back: nothing is read, and the
/// journal and its rollback journal are left byte for byte.
#[test]
fn a_hot_rollback_journal_completes_nothing_and_is_left_as_it_stands() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    rollback_journal(&db);
    let hot = dir.path().join("forge.db-journal");
    std::fs::write(&hot, b"a writer's pages, never rolled back by a reader").unwrap();
    let (journal_bytes, hot_bytes) = (std::fs::read(&db).unwrap(), std::fs::read(&hot).unwrap());

    let refused = listed(&db);
    assert!(busy(&refused), "{refused:?}");
    let error = refused.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("a rollback journal stands beside it"),
        "{error}"
    );
    assert_eq!(names(dir.path()), ["forge.db", "forge.db-journal"]);
    assert_eq!(std::fs::read(&db).unwrap(), journal_bytes);
    assert_eq!(std::fs::read(&hot).unwrap(), hot_bytes);
}

/// A scratch directory holding [`journal`]'s journal, and its path.
fn planted() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    journal(&db);
    (dir, db)
}

/// A lock-free read of [`planted`]'s journal during which a writer opens
/// it and plants a run: the read's answer, and the writer where `kept`
/// keeps it open past the read.
fn read_beside_a_writer(
    kept: bool,
) -> (
    tempfile::TempDir,
    Result<Vec<String>, StoreError>,
    Option<Store>,
) {
    let (dir, db) = planted();
    let mut held = None;
    let read = Store::read_untouched(&db, PATIENCE, |store| {
        let page = ids(store)?;
        let writer = Store::open(&db)?;
        plant_run(&writer.conn, "run-b", "{}")?;
        held = kept.then_some(writer);
        Ok::<_, StoreError>(page)
    });
    (dir, read, held)
}

/// A writer that opens the journal during a lock-free read, and is still
/// writing when it ends, voids the read: its answer is refused.
#[test]
fn a_writer_starting_during_a_lock_free_read_voids_its_answer() {
    let (_dir, refused, held) = read_beside_a_writer(true);
    assert!(held.is_some(), "the writer outlived the read");
    assert!(busy(&refused), "{refused:?}");
    let error = refused.unwrap_err();
    assert!(
        error.to_string().contains("changed while it was read"),
        "{error}"
    );
}

/// A writer that opens, writes and closes within a lock-free read leaves
/// no side file behind it, but it moved the main file: the read is voided.
#[test]
fn a_writer_come_and_gone_during_a_lock_free_read_voids_its_answer() {
    let (dir, refused, held) = read_beside_a_writer(false);
    assert!(held.is_none());
    assert_eq!(names(dir.path()), ["forge.db"], "the writer closed clean");
    assert!(busy(&refused), "{refused:?}");
}

/// The read's own refusal passes through as the caller's, and nothing is
/// left beside the journal.
#[test]
fn a_refusal_inside_the_read_is_the_callers() {
    let (dir, db) = planted();
    let refused = Store::read_untouched(&db, PATIENCE, |store| {
        store.run_seats("run-a", 1)?;
        Err::<(), _>(StoreError::RunNotFound("run-z".into()))
    });
    assert!(matches!(refused, Err(StoreError::RunNotFound(ref run)) if run == "run-z"));
    assert_eq!(names(dir.path()), ["forge.db"]);
}

/// A missing journal is refused, as SQLite cannot open it, and nothing
/// is created in its place.
#[test]
fn a_missing_journal_is_refused_and_nothing_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let refused = listed(&dir.path().join("forge.db"));
    let Err(StoreError::Sqlite(error)) = refused else {
        panic!("not SQLite's refusal: {refused:?}");
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
    let refused = listed(&db);
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
    let refused = listed(&db);
    assert!(busy(&refused), "not SQLite's busy: {refused:?}");
    assert!(started.elapsed() < WITHIN, "it waited past the patience");
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

    assert_eq!(listed(&db).unwrap(), ["run-a"]);
    assert_eq!(names(dir.path()), ["a?mode=rwc#b%41 c"]);
}
