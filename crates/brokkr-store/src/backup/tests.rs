//! The backup's own tests (decision 0100, the journal half of #431): a
//! snapshot taken while writers append, every refusal by name with the
//! destination untouched, and the inventory that makes a backup complete.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventType;
use serde_json::json;

use super::*;
use crate::test_support::{append, lags, plant_event};
use crate::Store;

/// The projection version these tests state; the store never reads the
/// view crate, so the taking build passes its own (here, a fixed word).
const PROJECTION: u32 = 27;

/// The instant every fixture backup declares.
const TAKEN_AT: &str = "2026-10-26T00:00:00Z";

/// Open (creating) the journal at `dir`.
fn journal(dir: &Path) -> Store {
    Store::open(&dir.join("forge.db")).unwrap()
}

/// Declare `run_id` and append `events` sealed phase entries to it.
fn seed(store: &mut Store, run_id: &str, events: usize) {
    store
        .create_run(run_id, "feature", "self", &json!({"files": {}}))
        .unwrap();
    for turn in 0..events {
        append(
            store,
            run_id,
            EventType::PhaseEntered,
            json!({"phase": format!("p{turn}")}),
        );
    }
}

/// `store`'s successful backup into `to`.
fn snapshot_of(store: &Store, to: &Path) -> Backup {
    backup(
        store,
        BackupOptions {
            to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap()
}

/// One backup of a journal of `runs`, each with `events` sealed phase
/// entries, ready for a refusal to be planted in it.
fn backed_up(dir: &Path, runs: &[&str], events: usize) -> (Store, Backup) {
    let mut store = journal(dir);
    for run_id in runs {
        seed(&mut store, run_id, events);
    }
    let backup = snapshot_of(&store, &dir.join("backup"));
    (store, backup)
}

/// The three-run, nine-event backup every refusal fixture starts from.
fn completed(dir: &Path) -> (Store, Backup) {
    backed_up(dir, &["alpha", "beta", "gamma"], 3)
}

/// The inventory a completed backup wrote, as its own type.
fn inventory_of(backup: &Backup) -> Inventory {
    let text = std::fs::read_to_string(backup.to.join(INVENTORY_FILE)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Write `inventory` back over a backup's own, as an edited backup.
fn rewrite_inventory(backup: &Backup, inventory: &Inventory) {
    let text = serde_json::to_string_pretty(inventory).unwrap();
    std::fs::write(backup.to.join(INVENTORY_FILE), text).unwrap();
}

/// The snapshot a backup wrote, open for a test's own surgery.
fn opened(backup: &Backup) -> (PathBuf, rusqlite::Connection) {
    let snapshot = backup.to.join(SNAPSHOT_FILE);
    let connection = rusqlite::Connection::open(&snapshot).unwrap();
    (snapshot, connection)
}

/// Bring a backup's inventory to the size and digest its snapshot holds
/// now, so an edit planted in the snapshot reaches the check under test.
fn resealed(backup: &Backup) -> Inventory {
    let snapshot = backup.to.join(SNAPSHOT_FILE);
    let mut inventory = inventory_of(backup);
    inventory.snapshot.bytes = std::fs::metadata(&snapshot).unwrap().len();
    inventory.snapshot.sha256 = sha256_file(&snapshot).unwrap();
    rewrite_inventory(backup, &inventory);
    inventory
}

/// Plant one event under `run_id` with no run, past the foreign-key fence,
/// and remove the projection rows its triggers write, so the `events`
/// table is the one trace of it a boundary must count.
fn plant_unowned(connection: &rusqlite::Connection) {
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    plant_event(connection, "ghost", &9i64, ZERO_HASH, "{").unwrap();
    connection
        .execute_batch(
            "DELETE FROM event_index WHERE run_id = 'ghost';
             DELETE FROM run_summary WHERE run_id = 'ghost';",
        )
        .unwrap();
}

/// A destination no restore in these tests may create on failure.
fn absent_destination(dir: &Path) -> PathBuf {
    dir.join("restored.db")
}

/// Assert that a refusal left `destination` absent.
fn assert_absent(destination: &Path) {
    assert!(
        !destination.exists(),
        "a refused restore left {} behind",
        destination.display()
    );
}

/// `to`'s SQLite sidecar named by `suffix`, exactly as SQLite names it.
fn sidecar_of(to: &Path, suffix: &str) -> PathBuf {
    let mut name = to.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// A restore into a destination whose sidecar `suffix` stands beside it
/// refuses by that sidecar's name, leaves it byte-equal and writes no main
/// file.
fn refused_beside_a_sidecar(suffix: &str) {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = backed_up(dir.path(), &["alpha"], 8);
    let destination = absent_destination(dir.path());
    let sidecar = sidecar_of(&destination, suffix);
    let bytes = b"a hot journal's bytes";
    std::fs::write(&sidecar, bytes).unwrap();
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::RestoreDestinationExists(found) = refused else {
        panic!("a destination occupied by {suffix} is refused by name: {refused:?}");
    };
    assert_eq!(found, sidecar);
    assert_eq!(std::fs::read(&sidecar).unwrap(), bytes);
    assert_absent(&destination);
}

/// Every regular file under `dir`, recursively.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found
}

#[test]
fn a_backup_during_appends_restores_a_coherent_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    let runs = ["alpha", "beta", "gamma"];
    for run_id in runs {
        seed(&mut store, run_id, 40);
    }
    // A second writer appends through its own store and folds nothing; the
    // runs stand behind their rows on purpose, and the next writable open
    // refolds them. The backup's own boundary is read whole, never from a
    // row.
    lags(&mut store, &runs);
    let running = Arc::new(AtomicBool::new(true));
    let (began, started) = mpsc::channel();
    let writer_runs = runs.map(str::to_string);
    let writer_flag = Arc::clone(&running);
    let writer_db = dir.path().join("forge.db");
    let writer = std::thread::spawn(move || {
        let mut writer = Store::open(&writer_db).unwrap();
        let mut turn = 0u64;
        began.send(()).unwrap();
        while writer_flag.load(Ordering::Relaxed) {
            for run_id in &writer_runs {
                append(
                    &mut writer,
                    run_id,
                    EventType::PhaseEntered,
                    json!({"phase": format!("writer-{turn}")}),
                );
            }
            turn += 1;
            std::thread::sleep(Duration::from_millis(1));
        }
        turn
    });
    started.recv().unwrap();
    // The verb opens the live journal READ-ONLY, so the snapshot is taken
    // through the same connection a backup uses.
    let reader = Store::open_read_only(&dir.path().join("forge.db")).unwrap();
    let backup = snapshot_of(&reader, &dir.path().join("backup"));
    running.store(false, Ordering::Relaxed);
    let turns = writer.join().unwrap();
    assert!(turns >= 1, "the writer appended nothing");
    let destination = absent_destination(dir.path());
    let restored = restore(&backup.to, &destination).unwrap();
    assert_eq!((restored.runs, restored.events), (3, backup.events));
    let inventory = inventory_of(&backup);
    assert_eq!(inventory.boundary.runs, 3);
    assert_eq!(inventory.boundary.events, backup.events);
    let source = Store::open_read_only(&dir.path().join("forge.db")).unwrap();
    let read = Store::open_read_only(&destination).unwrap();
    let mut total = 0u64;
    for run_id in runs {
        let chain = read.load(run_id).unwrap();
        total += chain.len() as u64;
        let head = chain.last().unwrap();
        let declared = inventory
            .boundary
            .heads
            .iter()
            .find(|head| head.run_id == run_id)
            .unwrap();
        assert_eq!(
            (head.seq, head.event_hash.as_str()),
            (declared.seq, declared.chain_head.as_str())
        );
        // The restored chain is a prefix of what the writer wrote: every
        // restored event is the source's at its own seq.
        let whole = source.load(run_id).unwrap();
        let prefix: Vec<_> = chain
            .iter()
            .map(|event| (event.seq, &event.event_hash))
            .collect();
        let source_prefix: Vec<_> = whole
            .iter()
            .take(chain.len())
            .map(|event| (event.seq, &event.event_hash))
            .collect();
        assert_eq!(prefix, source_prefix, "run {run_id} is not a source prefix");
    }
    assert_eq!(total, inventory.boundary.events);
}

#[test]
fn a_backup_records_the_versioned_inventory_this_build_reads() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let inventory = inventory_of(&backup);
    assert_eq!(inventory.format, JOURNAL_BACKUP_FORMAT);
    assert_eq!(inventory.engine, env!("CARGO_PKG_VERSION"));
    assert_eq!(inventory.boundary.taken_at, TAKEN_AT);
    assert_eq!(
        (
            inventory.schemas.database,
            inventory.schemas.queue,
            inventory.schemas.projection
        ),
        (crate::DATABASE_SCHEMA, 3, PROJECTION)
    );
    assert_eq!(inventory.boundary.runs, 3);
    assert_eq!(inventory.boundary.events, 9);
    assert_eq!(inventory.snapshot.sha256.len(), 64);
    assert_eq!(
        inventory.excluded, EXCLUDED,
        "a slice-1 backup names every exclusion"
    );
    assert_eq!(inventory.boundary.heads.len(), 3);
    assert!(inventory
        .boundary
        .heads
        .iter()
        .all(|head| head.seq == 3 && head.chain_head.len() == 64));
}

#[test]
fn an_interrupted_backup_is_incomplete_and_restore_refuses_a_missing_inventory() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 3);
    let to = dir.path().join("backup");
    let mut publishes = 0;
    let interrupted = backup_through(
        &store,
        BackupOptions {
            to: &to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
        &mut |temporary, final_path| {
            publishes += 1;
            if publishes == 2 {
                return Err(std::io::Error::other("the power went between publishes"));
            }
            publish_noclobber(temporary, final_path)
        },
    );
    assert!(
        interrupted.is_err(),
        "the interrupted backup reports its failure"
    );
    assert_eq!(publishes, 2, "the inventory was the second publish");
    assert!(
        to.join(SNAPSHOT_FILE).is_file(),
        "the snapshot is published before the inventory"
    );
    assert!(
        !to.join(INVENTORY_FILE).exists(),
        "a backup interrupted before its last publish has no inventory"
    );
    let interrupted = interrupted.unwrap_err();
    let BackupError::Io(error) = interrupted else {
        panic!("the interrupted backup reports its failure: {interrupted:?}");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert_eq!(error.to_string(), "the power went between publishes");
    let destination = absent_destination(dir.path());
    let refused = restore(&to, &destination).unwrap_err();
    let BackupError::MissingInventory(found) = refused else {
        panic!("a restore of an interrupted backup names its missing inventory: {refused:?}");
    };
    assert_eq!(found, to);
    assert_absent(&destination);
}

#[test]
fn a_truncated_snapshot_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let snapshot = backup.to.join(SNAPSHOT_FILE);
    let bytes = std::fs::read(&snapshot).unwrap();
    let truncated = bytes.len() / 2;
    std::fs::write(&snapshot, &bytes[..truncated]).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::SizeMismatch { found, inventory } = refused else {
        panic!("a truncated snapshot is refused for its size: {refused:?}");
    };
    assert_eq!(found, truncated as u64);
    assert_eq!(inventory, inventory_of(&backup).snapshot.bytes);
    assert_absent(&destination);
}

#[test]
fn an_altered_snapshot_byte_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let snapshot = backup.to.join(SNAPSHOT_FILE);
    let mut bytes = std::fs::read(&snapshot).unwrap();
    let at = bytes.len() / 2;
    bytes[at] ^= 0x20;
    std::fs::write(&snapshot, &bytes).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let found = sha256_file(&snapshot).unwrap();
    let BackupError::ChecksumMismatch {
        found: reported,
        inventory,
    } = refused
    else {
        panic!("an altered snapshot byte is refused for its digest: {refused:?}");
    };
    assert_eq!(reported, found);
    assert_eq!(inventory, inventory_of(&backup).snapshot.sha256);
    assert_absent(&destination);
}

#[test]
fn a_backup_into_a_file_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 1);
    let to = dir.path().join("a-file");
    std::fs::write(&to, "not a directory").unwrap();
    let refused = backup(
        &store,
        BackupOptions {
            to: &to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap_err();
    let BackupError::BackupDestinationNotDirectory(found) = refused else {
        panic!("a backup into a file is refused by name: {refused:?}");
    };
    assert_eq!(found, to);
    assert_eq!(std::fs::read_to_string(&to).unwrap(), "not a directory");
}

#[test]
fn a_missing_snapshot_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let snapshot = backup.to.join(SNAPSHOT_FILE);
    std::fs::remove_file(&snapshot).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::MissingSnapshot(found) = refused else {
        panic!("a backup without its snapshot is refused by name: {refused:?}");
    };
    assert_eq!(found, snapshot);
    assert_absent(&destination);
}

#[test]
fn an_unreadable_inventory_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let inventory = backup.to.join(INVENTORY_FILE);
    std::fs::remove_file(&inventory).unwrap();
    std::fs::create_dir(&inventory).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::UnreadableInventory { path, source } = refused else {
        panic!("an unreadable inventory is refused by name: {refused:?}");
    };
    assert_eq!(path, inventory);
    assert_eq!(source.kind(), std::io::ErrorKind::IsADirectory);
    assert_absent(&destination);
}

#[test]
fn an_inventory_that_is_not_json_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    std::fs::write(backup.to.join(INVENTORY_FILE), "{").unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::InvalidInventory { source } = refused else {
        panic!("an inventory that is not JSON is refused by name: {refused:?}");
    };
    assert_eq!(source.line(), 1);
    assert!(source.is_eof(), "{source}");
    assert_absent(&destination);
}

#[test]
fn a_boundary_naming_another_run_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let mut inventory = inventory_of(&backup);
    let found = inventory.boundary.heads[0].run_id.clone();
    inventory.boundary.heads[0].run_id = "zulu".into();
    rewrite_inventory(&backup, &inventory);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::BoundaryRun {
        found: named,
        inventory: declared,
    } = refused
    else {
        panic!("a boundary naming another run is refused by name: {refused:?}");
    };
    assert_eq!(named, found);
    assert_eq!(declared, "zulu");
    assert_absent(&destination);
}

#[test]
fn a_journal_that_records_no_queue_schema_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 1);
    store
        .conn
        .execute("DELETE FROM meta WHERE key = 'queue_schema'", [])
        .unwrap();
    let refused = backup(
        &store,
        BackupOptions {
            to: &dir.path().join("backup"),
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap_err();
    let BackupError::UnrecordedSchema { key } = refused else {
        panic!("a journal with no queue schema is refused by name: {refused:?}");
    };
    assert_eq!(key, "queue_schema");
}

#[cfg(unix)]
#[test]
fn a_path_sqlite_cannot_name_is_refused_by_name() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 1);
    let mut raw = dir.path().as_os_str().to_os_string().into_vec();
    raw.extend_from_slice(b"/\xff");
    let to = PathBuf::from(std::ffi::OsString::from_vec(raw));
    let refused = backup(
        &store,
        BackupOptions {
            to: &to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap_err();
    let BackupError::UnnameablePath(found) = refused else {
        panic!("a path SQLite cannot name is refused by name: {refused:?}");
    };
    assert_eq!(found, to);
}

#[test]
fn a_snapshot_that_fails_integrity_check_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let (snapshot, connection) = opened(&backup);
    let page_size: i64 = connection
        .pragma_query_value(None, "page_size", |row| row.get(0))
        .unwrap();
    let root: i64 = connection
        .query_row(
            "SELECT rootpage FROM sqlite_schema WHERE name = 'events'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);
    // The events table's first cell pointer, off the page it must point
    // into: a page `PRAGMA integrity_check` will not pass.
    let mut bytes = std::fs::read(&snapshot).unwrap();
    let at = ((root - 1) * page_size + 8) as usize;
    bytes[at] ^= 0xFF;
    std::fs::write(&snapshot, &bytes).unwrap();
    // The digest and size are brought to the edited bytes, so integrity is
    // the check that must refuse them.
    resealed(&backup);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::Integrity { reported } = refused else {
        panic!("a snapshot failing integrity is refused by name: {refused:?}");
    };
    assert_eq!(reported, "database disk image is malformed");
    assert_absent(&destination);
}

#[test]
fn a_broken_chain_is_refused_after_the_digest_matches() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let (_, connection) = opened(&backup);
    connection
        .execute_batch(
            "DROP TRIGGER events_append_only_update;
             UPDATE events SET envelope = replace(envelope, '\"p0\"', '\"pX\"')
             WHERE run_id = 'alpha' AND seq = 1;",
        )
        .unwrap();
    drop(connection);
    // The digest and size are brought to the edited bytes, so the chain is
    // the check that must refuse it.
    resealed(&backup);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::Chain { run_id, source } = refused else {
        panic!("a broken chain is refused by name: {refused:?}");
    };
    assert_eq!(run_id, "alpha");
    assert_eq!(
        source,
        brokkr_core::envelope::ChainError::BadHash { seq: 1 }
    );
    assert_absent(&destination);
}

#[test]
fn an_inventory_of_another_format_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let mut inventory = inventory_of(&backup);
    inventory.format = "brokkr.journal-backup.v2".into();
    rewrite_inventory(&backup, &inventory);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::UnknownFormat { found } = refused else {
        panic!("an inventory of another format is refused by name: {refused:?}");
    };
    assert_eq!(found, "brokkr.journal-backup.v2");
    assert_absent(&destination);
}

#[test]
fn a_head_that_disagrees_with_the_boundary_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let mut inventory = inventory_of(&backup);
    let declared = ZERO_HASH.to_string();
    inventory.boundary.heads[0].chain_head = declared.clone();
    let run_id = inventory.boundary.heads[0].run_id.clone();
    let seq = inventory.boundary.heads[0].seq;
    rewrite_inventory(&backup, &inventory);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let found = Store::open_read_only(&backup.to.join(SNAPSHOT_FILE))
        .unwrap()
        .load(&run_id)
        .unwrap();
    let found = found.last().unwrap();
    let BackupError::HeadMismatch {
        run_id: named,
        found_seq,
        found_head,
        inventory_seq,
        inventory_head,
    } = refused
    else {
        panic!("a head that disagrees with the boundary is refused by name: {refused:?}");
    };
    assert_eq!(named, run_id);
    assert_eq!(
        (found_seq, found_head.as_str()),
        (found.seq, found.event_hash.as_str())
    );
    assert_eq!(
        (inventory_seq, inventory_head.as_str()),
        (seq, declared.as_str())
    );
    assert_absent(&destination);
}

#[test]
fn a_count_that_disagrees_with_the_boundary_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let mut inventory = inventory_of(&backup);
    inventory.boundary.events += 1;
    rewrite_inventory(&backup, &inventory);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::CountsMismatch {
        found_runs,
        found_events,
        inventory_runs,
        inventory_events,
    } = refused
    else {
        panic!("a count that disagrees with the boundary is refused by name: {refused:?}");
    };
    assert_eq!(
        (found_runs, found_events, inventory_runs, inventory_events),
        (3, 9, 3, 10)
    );
    assert_absent(&destination);
}

#[test]
fn a_backup_into_a_place_that_holds_a_file_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 1);
    let to = dir.path().join("backup");
    std::fs::create_dir(&to).unwrap();
    std::fs::write(to.join("keep.txt"), "operator bytes").unwrap();
    let refused = backup(
        &store,
        BackupOptions {
            to: &to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap_err();
    let BackupError::BackupDestinationNotEmpty(found) = refused else {
        panic!("a backup into a non-empty place is refused by name: {refused:?}");
    };
    assert_eq!(found, to);
    assert_eq!(
        std::fs::read_to_string(to.join("keep.txt")).unwrap(),
        "operator bytes"
    );
}

#[test]
fn a_live_destination_is_refused_by_name_and_left_byte_equal() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("live.db");
    std::fs::write(&destination, "a live journal's bytes").unwrap();
    // The backup is deliberately unreadable: the destination refusal must
    // come first, before anything else is read.
    let refused = restore(&dir.path().join("no-such-backup"), &destination).unwrap_err();
    let BackupError::RestoreDestinationExists(found) = refused else {
        panic!("a live destination is refused by name: {refused:?}");
    };
    assert_eq!(found, destination);
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"a live journal's bytes"
    );
}

#[test]
fn restore_refuses_a_destination_with_a_hot_rollback_journal() {
    refused_beside_a_sidecar("-journal");
}

#[test]
fn restore_refuses_a_destination_with_a_write_ahead_log() {
    refused_beside_a_sidecar("-wal");
}

#[test]
fn restore_refuses_a_destination_with_a_shared_memory_index() {
    refused_beside_a_sidecar("-shm");
}

#[test]
fn restore_refuses_an_unowned_event_in_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = backed_up(dir.path(), &["alpha"], 8);
    let (_, connection) = opened(&backup);
    plant_unowned(&connection);
    drop(connection);
    // The size and digest are brought to the edited bytes, so ownership is
    // the check that must refuse them.
    resealed(&backup);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::UnownedEvents { count, run_id, seq } = refused else {
        panic!("a snapshot holding an unowned event is refused by name: {refused:?}");
    };
    assert_eq!((count, run_id.as_str(), seq), (1, "ghost", 9));
    assert_absent(&destination);
}

#[test]
fn a_backup_refuses_a_source_journal_holding_an_unowned_event() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 8);
    plant_unowned(&store.conn);
    let to = dir.path().join("backup");
    let refused = backup(
        &store,
        BackupOptions {
            to: &to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap_err();
    let BackupError::UnownedEvents { count, run_id, seq } = refused else {
        panic!("a source journal holding an unowned event is refused by name: {refused:?}");
    };
    assert_eq!((count, run_id.as_str(), seq), (1, "ghost", 9));
    assert!(
        files_under(&to).is_empty(),
        "a refused backup publishes neither name"
    );
}

#[test]
fn restore_refuses_a_snapshot_whose_projection_disagrees() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    let (_, connection) = opened(&backup);
    connection
        .execute(
            "UPDATE run_summary SET residual_count = 7 WHERE run_id = 'alpha'",
            [],
        )
        .unwrap();
    drop(connection);
    // The size and digest are brought to the edited bytes, so the
    // projection comparison is the check that must refuse them.
    resealed(&backup);
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    let BackupError::ProjectionDisagreement { first } = refused else {
        panic!("a snapshot whose projection disagrees is refused by name: {refused:?}");
    };
    assert_eq!(
        first,
        "run 'alpha': its summary row stands at its head, and its residual_count is not its \
         fold's"
    );
    assert_absent(&destination);
}

#[test]
fn no_secret_value_is_copied() {
    let dir = tempfile::tempdir().unwrap();
    const NAME: &str = "BROKKR_BACKUP_TEST_TOKEN";
    const VALUE: &str = "sk-backup-test-6b1f0c2e-do-not-copy";
    let mut store = journal(dir.path());
    store
        .create_run("alpha", "feature", "self", &json!({"files": {}}))
        .unwrap();
    append(
        &mut store,
        "alpha",
        EventType::PhaseEntered,
        json!({"phase": "work", "binding": NAME}),
    );
    // The value lives in the operator's store beside the journal; the
    // journal names its binding and holds no value.
    std::fs::write(dir.path().join("secrets.env"), format!("{NAME}={VALUE}\n")).unwrap();
    let backup = snapshot_of(&store, &dir.path().join("backup"));
    for file in files_under(&backup.to) {
        let bytes = std::fs::read(&file).unwrap();
        assert!(
            !bytes
                .windows(VALUE.len())
                .any(|window| window == VALUE.as_bytes()),
            "the secret's value appears in {}",
            file.display()
        );
    }
    let destination = absent_destination(dir.path());
    restore(&backup.to, &destination).unwrap();
    let restored = Store::open_read_only(&destination).unwrap();
    let chain = restored.load("alpha").unwrap();
    assert_eq!(chain[0].payload["binding"], NAME);
}

/// The parse error an inventory that is not JSON reports, which the two
/// pinning groups that name it share.
fn invalid_inventory() -> serde_json::Error {
    serde_json::from_str::<Inventory>("{").unwrap_err()
}

/// The destination refusals' words, each once.
fn destination_refusals() -> Vec<(BackupError, String)> {
    let path = Path::new("/a/journal");
    vec![
        (
            BackupError::BackupDestinationNotEmpty(path.to_path_buf()),
            "refused: '/a/journal' already holds a file, so it is not an empty place for a backup; \
             nothing was written"
                .to_string(),
        ),
        (
            BackupError::BackupDestinationNotDirectory(path.to_path_buf()),
            "refused: '/a/journal' exists and is not a directory; nothing was written".to_string(),
        ),
        (
            BackupError::RestoreDestinationExists(path.to_path_buf()),
            "refused: '/a/journal' exists; a restore writes into a new location and never over a \
             journal that may be live"
                .to_string(),
        ),
        (
            BackupError::UnnameablePath(path.to_path_buf()),
            "refused: '/a/journal' is not a path SQLite can name".to_string(),
        ),
    ]
}

/// The inventory refusals' words, each once.
fn inventory_refusals() -> Vec<(BackupError, String)> {
    let backup = Path::new("/a/backup");
    let invalid = invalid_inventory();
    let invalid_text = invalid.to_string();
    let denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    vec![
        (
            BackupError::MissingInventory(backup.to_path_buf()),
            "the backup at '/a/backup' has no inventory.json: an interrupted backup is \
             incomplete, because the inventory is written last"
                .to_string(),
        ),
        (
            BackupError::UnreadableInventory {
                path: backup.join(INVENTORY_FILE),
                source: denied,
            },
            "the backup's inventory.json at '/a/backup/inventory.json' cannot be read: denied"
                .to_string(),
        ),
        (
            BackupError::InvalidInventory { source: invalid },
            format!("the backup's inventory.json is not one this build reads: {invalid_text}"),
        ),
        (
            BackupError::UnknownFormat {
                found: "brokkr.journal-backup.v2".into(),
            },
            "the backup's format is 'brokkr.journal-backup.v2'; this build reads \
             'brokkr.journal-backup.v1'"
                .to_string(),
        ),
    ]
}

/// The snapshot refusals' words, each once.
fn snapshot_refusals() -> Vec<(BackupError, String)> {
    use brokkr_core::envelope::ChainError;

    let backup = Path::new("/a/backup");
    vec![
        (
            BackupError::MissingSnapshot(backup.join(SNAPSHOT_FILE)),
            "the backup has no journal.db at '/a/backup/journal.db': a backup without its \
             snapshot is incomplete"
                .to_string(),
        ),
        (
            BackupError::SizeMismatch {
                found: 1,
                inventory: 2,
            },
            "the backup's snapshot is 1 bytes; its inventory says 2".to_string(),
        ),
        (
            BackupError::ChecksumMismatch {
                found: "aa".into(),
                inventory: "bb".into(),
            },
            "the backup's snapshot sha256 is aa; its inventory says bb".to_string(),
        ),
        (
            BackupError::Integrity {
                reported: "boom".into(),
            },
            "the backup's snapshot fails PRAGMA integrity_check: boom".to_string(),
        ),
        (
            BackupError::Chain {
                run_id: "alpha".into(),
                source: ChainError::BadHash { seq: 1 },
            },
            "run 'alpha' does not verify: event 1: event_hash does not match canonical content"
                .to_string(),
        ),
        (
            BackupError::ProjectionDisagreement {
                first: "run 'alpha': its residual_count is not its fold's".into(),
            },
            "the snapshot's projections disagree: run 'alpha': its residual_count is not its \
             fold's"
                .to_string(),
        ),
    ]
}

/// The boundary refusals' words, each once.
fn boundary_refusals() -> Vec<(BackupError, String)> {
    vec![
        (
            BackupError::HeadMismatch {
                run_id: "alpha".into(),
                found_seq: 1,
                found_head: "aa".into(),
                inventory_seq: 2,
                inventory_head: "bb".into(),
            },
            "run 'alpha' head is seq 1 (aa); the inventory's boundary says seq 2 (bb)".to_string(),
        ),
        (
            BackupError::BoundaryRun {
                found: "alpha".into(),
                inventory: "zulu".into(),
            },
            "the snapshot holds run 'alpha', where the inventory's boundary names 'zulu'"
                .to_string(),
        ),
        (
            BackupError::CountsMismatch {
                found_runs: 3,
                found_events: 9,
                inventory_runs: 3,
                inventory_events: 10,
            },
            "the snapshot holds 3 runs and 9 events; the inventory's boundary says 3 runs and \
             10 events"
                .to_string(),
        ),
        (
            BackupError::UnownedEvents {
                count: 1,
                run_id: "ghost".into(),
                seq: 9,
            },
            "the snapshot holds 1 events no run owns; the first is run 'ghost' at seq 9"
                .to_string(),
        ),
        (
            BackupError::UnrecordedSchema {
                key: "queue_schema",
            },
            "the journal records no 'queue_schema' schema".to_string(),
        ),
    ]
}

/// The source errors a refusal wraps, each once.
fn wrapped_refusals() -> Vec<(BackupError, String)> {
    let invalid = invalid_inventory();
    let invalid_text = invalid.to_string();
    vec![
        (
            BackupError::Store(crate::StoreError::RunNotFound("alpha".into())),
            "run 'alpha' not found".to_string(),
        ),
        (
            BackupError::Sqlite(rusqlite::Error::InvalidQuery),
            "sqlite: Query is not read-only".to_string(),
        ),
        (
            BackupError::Io(std::io::Error::other("boom")),
            "io: boom".to_string(),
        ),
        (BackupError::Json(invalid), format!("json: {invalid_text}")),
    ]
}

/// Every refusal's words, pinned in one place: the tests that bind a
/// refusal match its variant and fields, never its text.
#[test]
fn each_refusal_names_itself_once() {
    for cases in [
        destination_refusals(),
        inventory_refusals(),
        snapshot_refusals(),
        boundary_refusals(),
        wrapped_refusals(),
    ] {
        for (refused, expected) in cases {
            assert_eq!(refused.to_string(), expected);
        }
    }
}
