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
use crate::test_support::{append, lags};
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

/// One backup of a small journal of `runs`, ready for a refusal to be
/// planted in it.
fn completed(dir: &Path) -> (Store, Backup) {
    let mut store = journal(dir);
    for run_id in ["alpha", "beta", "gamma"] {
        seed(&mut store, run_id, 3);
    }
    let backup = snapshot_of(&store, &dir.join("backup"));
    (store, backup)
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

/// A destination no restore in these tests may create on failure.
fn absent_destination(dir: &Path) -> PathBuf {
    dir.join("restored.db")
}

/// Assert that `refused`'s text is exactly `expected`, and that the
/// destination is still absent.
fn refused_by(refused: BackupError, expected: &str, destination: &Path) {
    assert_eq!(refused.to_string(), expected);
    assert!(
        !destination.exists(),
        "a refused restore left {} behind",
        destination.display()
    );
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
    let destination = absent_destination(dir.path());
    let refused = restore(&to, &destination).unwrap_err();
    refused_by(
        refused,
        &format!(
            "the backup at '{}' has no {INVENTORY_FILE}: an interrupted backup is incomplete, \
             because the inventory is written last",
            to.display()
        ),
        &destination,
    );
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
    refused_by(
        refused,
        &format!(
            "the backup's snapshot is {truncated} bytes; its inventory says {}",
            inventory_of(&backup).snapshot.bytes
        ),
        &destination,
    );
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
    refused_by(
        refused,
        &format!(
            "the backup's snapshot sha256 is {found}; its inventory says {}",
            inventory_of(&backup).snapshot.sha256
        ),
        &destination,
    );
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
    assert_eq!(
        refused.to_string(),
        format!(
            "refused: '{}' exists and is not a directory; nothing was written",
            to.display()
        )
    );
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
    refused_by(
        refused,
        &format!(
            "the backup has no {SNAPSHOT_FILE} at '{}': a backup without its snapshot is incomplete",
            snapshot.display()
        ),
        &destination,
    );
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
    let text = refused.to_string();
    assert!(
        text.starts_with(&format!(
            "the backup's {INVENTORY_FILE} at '{}' cannot be read: ",
            inventory.display()
        )),
        "{text}"
    );
    assert!(!destination.exists());
}

#[test]
fn an_inventory_that_is_not_json_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, backup) = completed(dir.path());
    std::fs::write(backup.to.join(INVENTORY_FILE), "{").unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&backup.to, &destination).unwrap_err();
    assert!(
        refused
            .to_string()
            .starts_with("the backup's inventory.json is not one this build reads: "),
        "{refused}"
    );
    assert!(!destination.exists());
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
    refused_by(
        refused,
        &format!("the snapshot holds run '{found}', where the inventory's boundary names 'zulu'"),
        &destination,
    );
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
    assert_eq!(
        refused.to_string(),
        "the journal records no 'queue_schema' schema"
    );
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
    assert_eq!(
        refused.to_string(),
        format!("refused: '{}' is not a path SQLite can name", to.display())
    );
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
    assert!(
        refused
            .to_string()
            .starts_with("the backup's snapshot fails PRAGMA integrity_check: "),
        "{refused}"
    );
    assert!(!destination.exists());
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
    refused_by(
        refused,
        "run 'alpha' does not verify: event 1: event_hash does not match canonical content",
        &destination,
    );
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
    refused_by(
        refused,
        "the backup's format is 'brokkr.journal-backup.v2'; this build reads \
         'brokkr.journal-backup.v1'",
        &destination,
    );
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
    refused_by(
        refused,
        &format!(
            "run '{run_id}' head is seq {} ({}); the inventory's boundary says seq {seq} ({declared})",
            found.seq, found.event_hash
        ),
        &destination,
    );
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
    refused_by(
        refused,
        "the snapshot holds 3 runs and 9 events; the inventory's boundary says 3 runs and 10 events",
        &destination,
    );
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
    assert_eq!(
        refused.to_string(),
        format!(
            "refused: '{}' already holds a file, so it is not an empty place for a backup; \
             nothing was written",
            to.display()
        )
    );
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
    assert_eq!(
        refused.to_string(),
        format!(
            "refused: '{}' exists; a restore writes into a new location and never over a \
             journal that may be live",
            destination.display()
        )
    );
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"a live journal's bytes"
    );
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
