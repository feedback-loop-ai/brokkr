//! A journal's backup: one consistent snapshot and the versioned
//! inventory that makes it a complete recovery unit (decision 0100, the
//! journal half of #431).
//!
//! A byte copy of a WAL journal is not a snapshot: SQLite may hold recent
//! commits in the `-wal` beside it and may write pages while the copy
//! runs. The snapshot here is SQLite's own consistent copy, `VACUUM
//! INTO`, which reads one transaction of the live journal while writers
//! may run, so the file it writes is a database at one instant. The
//! boundary in the inventory — the run and event counts, and every run's
//! head seq and chain head — is read FROM that snapshot, so the declared
//! boundary and the bytes it declares are the same database by
//! construction. (The commission's recommendation put `VACUUM INTO` and
//! the boundary read inside one read transaction; `VACUUM` cannot run
//! inside a transaction, and reading the boundary from its output is the
//! stronger form of the same promise.)
//!
//! Completeness is by construction. The snapshot and the inventory are
//! written under temporary names and fsynced, and the inventory is
//! published LAST, so a directory that holds a snapshot and no inventory
//! is an interrupted backup and never a complete one. Neither name is
//! ever written over: a backup refuses a `--to` that holds any file.
//!
//! [`restore`] is the other half, and it accepts nothing it has not
//! checked: the inventory's `format`, the snapshot's size and SHA-256,
//! SQLite's `integrity_check`, every run's chain and its boundary head,
//! the whole-table counts, and the projections. It writes to a temporary
//! name beside its destination and publishes only after every check has
//! passed, so a refusal leaves the destination absent. It starts nothing
//! — no driver, no queue claim, no resume — and a run in flight at the
//! snapshot stays as it was recorded, under the existing
//! indeterminate-recovery rule.
//!
//! What this slice deliberately does not carry is named in the
//! inventory's `excluded` list: Git objects and refs/forge anchors,
//! artifacts outside the journal, harness transcripts, and credentials.
//! The journal holds no credential by design (decision 0065's bindings
//! name secrets; the values live in the operator's store), so a backup of
//! the journal copies none.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::queue::queue_schema;
use crate::schema::schema_version;
use crate::{patiently, Store, StoreError};

/// The one inventory format this build writes and reads. A change to the
/// field set that a previous build could not read takes a new string;
/// `format` is the whole version rule (decision 0100 ruling 2).
pub const JOURNAL_BACKUP_FORMAT: &str = "brokkr.journal-backup.v1";

/// The snapshot's one name inside a backup directory.
pub(crate) const SNAPSHOT_FILE: &str = "journal.db";

/// The inventory's one name inside a backup directory: the last file a
/// backup publishes, and the mark that makes a backup complete.
pub(crate) const INVENTORY_FILE: &str = "inventory.json";

/// What a backup is asked for.
#[derive(Debug, Clone, Copy)]
pub struct BackupOptions<'a> {
    /// The directory the backup is written into. It must not exist or be
    /// empty; nothing in it is overwritten.
    pub to: &'a Path,
    /// When the snapshot was taken, RFC 3339, passed in rather than read
    /// so a test can pin it.
    pub now: &'a str,
    /// The projection version the taking build reads
    /// (`brokkr_view::VIEW_VERSION` at the CLI). The journal records no
    /// projection version of its own, so the build states the one it is
    /// writing for.
    pub projection: u32,
}

/// What a completed backup wrote, for the verb's readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    pub to: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub runs: u64,
    pub events: u64,
}

/// What a completed restore accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub to: PathBuf,
    pub runs: u64,
    pub events: u64,
}

/// The journal's schema versions, as the store records them, plus the
/// projection version the taking build reads.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Schemas {
    database: u32,
    queue: u32,
    projection: u32,
}

/// The snapshot file's size and digest.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    bytes: u64,
    sha256: String,
}

/// One run's place in the declared boundary: its last event's seq and the
/// chain head that event sealed, or `0` and the zero hash for a run with
/// no event.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Head {
    run_id: String,
    seq: u64,
    chain_head: String,
}

/// The instant a snapshot stands at, as the snapshot itself records it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Boundary {
    taken_at: String,
    runs: u64,
    events: u64,
    heads: Vec<Head>,
}

/// What a slice-1 backup does not carry, each named so a reader never
/// mistakes the journal for a complete recovery unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Excluded {
    GitObjectsAndAnchors,
    ArtifactsOutsideTheJournal,
    HarnessTranscripts,
    Credentials,
}

const EXCLUDED: [Excluded; 4] = [
    Excluded::GitObjectsAndAnchors,
    Excluded::ArtifactsOutsideTheJournal,
    Excluded::HarnessTranscripts,
    Excluded::Credentials,
];

/// `inventory.json`, exactly as this format states it. `deny_unknown_fields`
/// keeps a field this build does not read from passing as one it does.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    format: String,
    engine: String,
    schemas: Schemas,
    snapshot: Snapshot,
    boundary: Boundary,
    excluded: Vec<Excluded>,
}

/// Which stored fact of an event row a snapshot's envelope disagrees with
/// (decision 0100 ruling 4): the row's key, or the hash the store appends
/// under.
pub use consistency::RowField;

/// Why a backup or a restore refused. Each variant names what failed; a
/// restore's variants all leave its destination absent.
#[derive(Debug, Error)]
pub enum BackupError {
    #[error(
        "refused: '{0}' already holds a file, so it is not an empty place for a backup; \
         nothing was written"
    )]
    BackupDestinationNotEmpty(PathBuf),
    #[error("refused: '{0}' exists and is not a directory; nothing was written")]
    BackupDestinationNotDirectory(PathBuf),
    #[error(
        "refused: '{0}' exists; a restore writes into a new location and never over a \
         journal that may be live"
    )]
    RestoreDestinationExists(PathBuf),
    #[error(
        "the backup at '{0}' has no {INVENTORY_FILE}: an interrupted backup is incomplete, \
         because the inventory is written last"
    )]
    MissingInventory(PathBuf),
    #[error("the backup's {INVENTORY_FILE} at '{path}' cannot be read: {source}")]
    UnreadableInventory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("the backup's {INVENTORY_FILE} is not one this build reads: {source}")]
    InvalidInventory { source: serde_json::Error },
    #[error("the backup's format is '{found}'; this build reads '{JOURNAL_BACKUP_FORMAT}'")]
    UnknownFormat { found: String },
    #[error(
        "the backup has no {SNAPSHOT_FILE} at '{0}': a backup without its snapshot is incomplete"
    )]
    MissingSnapshot(PathBuf),
    #[error("the backup's snapshot is {found} bytes; its inventory says {inventory}")]
    SizeMismatch { found: u64, inventory: u64 },
    #[error("the backup's snapshot sha256 is {found}; its inventory says {inventory}")]
    ChecksumMismatch { found: String, inventory: String },
    #[error("the backup's snapshot fails PRAGMA integrity_check: {reported}")]
    Integrity { reported: String },
    #[error("run '{run_id}' does not verify: {source}")]
    Chain {
        run_id: String,
        #[source]
        source: brokkr_core::envelope::ChainError,
    },
    #[error(
        "run '{run_id}' head is seq {found_seq} ({found_head}); the inventory's boundary \
         says seq {inventory_seq} ({inventory_head})"
    )]
    HeadMismatch {
        run_id: String,
        found_seq: u64,
        found_head: String,
        inventory_seq: u64,
        inventory_head: String,
    },
    #[error(
        "the snapshot holds run '{found}', where the inventory's boundary names '{inventory}'"
    )]
    BoundaryRun { found: String, inventory: String },
    #[error(
        "the snapshot holds {found_runs} runs and {found_events} events; the inventory's \
         boundary says {inventory_runs} runs and {inventory_events} events"
    )]
    CountsMismatch {
        found_runs: u64,
        found_events: u64,
        inventory_runs: u64,
        inventory_events: u64,
    },
    #[error(
        "the snapshot holds {count} events no run owns; the first is run '{run_id}' at seq {seq}"
    )]
    UnownedEvents {
        count: u64,
        run_id: String,
        seq: i64,
    },
    #[error(
        "the snapshot's event row for run '{run_id}' at SQL seq {seq} disagrees with its \
         envelope: its {field} is '{found}', and the envelope's is '{expected}'"
    )]
    RowMismatch {
        run_id: String,
        seq: i64,
        field: RowField,
        found: String,
        expected: String,
    },
    #[error("the snapshot's projections disagree: {first}")]
    ProjectionDisagreement { first: String },
    #[error("the journal records no '{key}' schema")]
    UnrecordedSchema { key: &'static str },
    #[error("refused: '{0}' is not a path SQLite can name")]
    UnnameablePath(PathBuf),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Take one consistent snapshot of `store`'s journal into `options.to`,
/// and write the inventory that completes it.
pub fn backup(store: &Store, options: BackupOptions<'_>) -> Result<Backup, BackupError> {
    backup_through(store, options, &mut publish_noclobber)
}

/// [`backup`] with its one publish step handed in, so the test that proves
/// the inventory is written LAST can interrupt the second publish (ruling
/// 9). Production passes [`publish_noclobber`].
fn backup_through(
    store: &Store,
    options: BackupOptions<'_>,
    publish: &mut dyn FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<Backup, BackupError> {
    let to = &literal::destination(options.to)?;
    let scratch = Scratch::in_directory(to);
    let temporary = vacuum_into(store, &scratch)?;
    sync_file(&temporary)?;
    let snapshot_store = Store::open_read_only(&temporary)?;
    let boundary = boundary_of(&snapshot_store, options.now)?;
    let schemas = schemas_of(&snapshot_store, options.projection)?;
    drop(snapshot_store);
    let snapshot = Snapshot {
        bytes: std::fs::metadata(&temporary)?.len(),
        sha256: sha256_file(&temporary)?,
    };
    // The snapshot is published first and the inventory last: an
    // interrupted backup therefore leaves a snapshot with no inventory,
    // which [`restore`] refuses.
    publish(&temporary, &to.join(SNAPSHOT_FILE))
        .map_err(|error| backup_publish_error(error, to))?;
    sync_dir(to)?;
    let inventory = Inventory {
        format: JOURNAL_BACKUP_FORMAT.into(),
        engine: env!("CARGO_PKG_VERSION").into(),
        schemas,
        snapshot,
        boundary,
        excluded: EXCLUDED.to_vec(),
    };
    let written = scratch.path(&format!("{INVENTORY_FILE}.tmp"));
    write_file(&written, &serde_json::to_vec_pretty(&inventory)?)?;
    publish(&written, &to.join(INVENTORY_FILE)).map_err(|error| backup_publish_error(error, to))?;
    sync_dir(to)?;
    let boundary = inventory.boundary;
    Ok(Backup {
        to: to.to_path_buf(),
        bytes: inventory.snapshot.bytes,
        sha256: inventory.snapshot.sha256,
        runs: boundary.runs,
        events: boundary.events,
    })
}

/// Restore the backup at `from` into the journal path `to`, which no file
/// or SQLite sidecar may occupy, after every check passed.
pub fn restore(from: &Path, to: &Path) -> Result<Restored, BackupError> {
    restore_through(from, to, &mut publish_noclobber)
}

/// [`restore`] with its one publish step handed in, so the test that proves
/// a destination appearing in the race is refused by its name can make it
/// appear (decision 0100 ruling 4). Production passes [`publish_noclobber`].
fn restore_through(
    from: &Path,
    to: &Path,
    publish: &mut dyn FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<Restored, BackupError> {
    let (from, to) = (&literal::literal(from)?, &literal::literal(to)?);
    refuse_existing(to)?;
    let inventory = read_inventory(from)?;
    let source = from.join(SNAPSHOT_FILE);
    let metadata = match std::fs::metadata(&source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(BackupError::MissingSnapshot(source));
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.len() != inventory.snapshot.bytes {
        return Err(BackupError::SizeMismatch {
            found: metadata.len(),
            inventory: inventory.snapshot.bytes,
        });
    }
    let temporary = Temp::beside(to)?;
    // The metadata above is the path's; the bytes are the stream's. A source
    // whose stream differs from its metadata (a FIFO reports 0) is refused
    // on what the copy read, before the digest.
    let copied = copy_file(&source, temporary.path())?;
    if copied != inventory.snapshot.bytes {
        return Err(BackupError::SizeMismatch {
            found: copied,
            inventory: inventory.snapshot.bytes,
        });
    }
    verify_snapshot(temporary.path(), &inventory)?;
    publish(temporary.path(), to).map_err(|error| destination_error(error, to))?;
    temporary.disarm();
    if let Err(error) = sync_dir(parent_of(to)) {
        // A failure after the publish must not leave the destination: the
        // restore reports a refusal, and a refusal writes nothing.
        let _ = std::fs::remove_file(to);
        return Err(error);
    }
    Ok(Restored {
        to: to.to_path_buf(),
        runs: inventory.boundary.runs,
        events: inventory.boundary.events,
    })
}

/// The names SQLite gives a journal's sidecars beside it: the hot rollback
/// journal, the write-ahead log and the WAL's shared-memory index. A
/// destination any of them occupies is refused, so a later writable open
/// cannot roll one into the restored bytes.
const SQLITE_SIDECARS: [&str; 3] = ["-journal", "-wal", "-shm"];

/// Refuse a destination that already exists, of any kind — including a
/// symlink, which is why this asks the link itself and not its target —
/// or that any SQLite sidecar beside it occupies.
fn refuse_existing(to: &Path) -> Result<(), BackupError> {
    let sidecars = SQLITE_SIDECARS.iter().map(|suffix| {
        let mut name = to.as_os_str().to_os_string();
        name.push(suffix);
        PathBuf::from(name)
    });
    for occupied in std::iter::once(to.to_path_buf()).chain(sidecars) {
        match std::fs::symlink_metadata(&occupied) {
            Ok(_) => return Err(BackupError::RestoreDestinationExists(occupied)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// The inventory a backup directory states, parsed and of this build's
/// format. A missing one is the interrupted backup's own diagnostic.
fn read_inventory(from: &Path) -> Result<Inventory, BackupError> {
    let path = from.join(INVENTORY_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(BackupError::MissingInventory(from.to_path_buf()));
        }
        Err(source) => return Err(BackupError::UnreadableInventory { path, source }),
    };
    let inventory: Inventory =
        serde_json::from_str(&text).map_err(|source| BackupError::InvalidInventory { source })?;
    if inventory.format != JOURNAL_BACKUP_FORMAT {
        return Err(BackupError::UnknownFormat {
            found: inventory.format,
        });
    }
    Ok(inventory)
}

/// The checks the copied bytes must pass before they may be published:
/// the digest (their size was checked against the inventory twice, on the
/// source's metadata before the copy and on the bytes the copy read; a
/// source that changed under the copy changes the digest), SQLite's own
/// integrity, every chain with its boundary head, and the counts.
fn verify_snapshot(path: &Path, inventory: &Inventory) -> Result<(), BackupError> {
    let found = sha256_file(path)?;
    if found != inventory.snapshot.sha256 {
        return Err(BackupError::ChecksumMismatch {
            found,
            inventory: inventory.snapshot.sha256.clone(),
        });
    }
    let store = Store::open_read_only(path)?;
    integrity_of(&store)?;
    let boundary = boundary_of(&store, &inventory.boundary.taken_at)?;
    compared_boundary(&boundary, &inventory.boundary)?;
    match store.summary_disagreements()?.into_iter().next() {
        Some(first) => Err(BackupError::ProjectionDisagreement {
            first: first.to_string(),
        }),
        None => Ok(()),
    }
}

/// SQLite's `PRAGMA integrity_check`, which must answer exactly `ok`. A
/// failure prints one row per problem; every row is reported, and no row
/// is a refusal too (the operator's 2026-09-26 ruling).
fn integrity_of(store: &Store) -> Result<(), BackupError> {
    let mut statement = store.conn.prepare("PRAGMA integrity_check")?;
    // A pragma that cannot step is not the answer `ok` either; it is the
    // same refusal, with the database's own words.
    let rows = match statement
        .query_map([], |row| row.get::<_, String>(0))
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
    {
        Ok(rows) => rows,
        Err(reported) => {
            return Err(BackupError::Integrity {
                reported: reported.to_string(),
            });
        }
    };
    if rows.as_slice() == ["ok"] {
        return Ok(());
    }
    Err(BackupError::Integrity {
        reported: rows.join("; "),
    })
}

/// The boundary a store's own pages record: every run, by id, with its
/// chain verified and its head taken from the chain's last event. The
/// counts are the whole tables', never the sum of what a reader loaded, so
/// an event no run owns is counted and then refused by name.
fn boundary_of(store: &Store, taken_at: &str) -> Result<Boundary, BackupError> {
    let (runs, events): (u64, u64) = store.conn.query_row(
        "SELECT (SELECT count(*) FROM runs), (SELECT count(*) FROM events)",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    consistency::refuse_unowned(store)?;
    let mut statement = store
        .conn
        .prepare("SELECT run_id FROM runs ORDER BY run_id")?;
    let run_ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut heads = Vec::with_capacity(runs as usize);
    for run_id in run_ids {
        let chain = consistency::chain(store, &run_id)?;
        let (seq, chain_head) = chain
            .last()
            .map(|event| (event.seq, event.event_hash.clone()))
            .unwrap_or((0, ZERO_HASH.to_string()));
        heads.push(Head {
            run_id,
            seq,
            chain_head,
        });
    }
    Ok(Boundary {
        taken_at: taken_at.to_string(),
        runs,
        events,
        heads,
    })
}

/// Compare the boundary the snapshot records with the inventory's. Runs
/// are matched by id, and the first difference in the inventory's order
/// is the one named.
fn compared_boundary(found: &Boundary, inventory: &Boundary) -> Result<(), BackupError> {
    if found.runs != inventory.runs || found.events != inventory.events {
        return Err(BackupError::CountsMismatch {
            found_runs: found.runs,
            found_events: found.events,
            inventory_runs: inventory.runs,
            inventory_events: inventory.events,
        });
    }
    let mut found = found.heads.iter();
    let mut inventory = inventory.heads.iter();
    loop {
        match (found.next(), inventory.next()) {
            (None, None) => return Ok(()),
            (Some(found), Some(inventory)) if found.run_id != inventory.run_id => {
                return Err(BackupError::BoundaryRun {
                    found: found.run_id.clone(),
                    inventory: inventory.run_id.clone(),
                });
            }
            (Some(found), Some(inventory))
                if found.seq != inventory.seq || found.chain_head != inventory.chain_head =>
            {
                return Err(BackupError::HeadMismatch {
                    run_id: inventory.run_id.clone(),
                    found_seq: found.seq,
                    found_head: found.chain_head.clone(),
                    inventory_seq: inventory.seq,
                    inventory_head: inventory.chain_head.clone(),
                });
            }
            (Some(_), Some(_)) => {}
            (Some(found), None) => {
                return Err(BackupError::BoundaryRun {
                    found: found.run_id.clone(),
                    inventory: "<none>".into(),
                });
            }
            (None, Some(inventory)) => {
                return Err(BackupError::BoundaryRun {
                    found: "<none>".into(),
                    inventory: inventory.run_id.clone(),
                });
            }
        }
    }
}

/// The journal's recorded schema versions, read through the store's own
/// readers, and the projection version the taking build reads.
fn schemas_of(store: &Store, projection: u32) -> Result<Schemas, BackupError> {
    let database = schema_version(&store.conn)?.ok_or(BackupError::UnrecordedSchema {
        key: "database_schema",
    })?;
    let queue = queue_schema(&store.conn)?.ok_or(BackupError::UnrecordedSchema {
        key: "queue_schema",
    })?;
    Ok(Schemas {
        database,
        queue,
        projection,
    })
}

/// `VACUUM INTO` one temporary file under `scratch`, retried through the
/// store's one busy handler. A retry that meets the file its own refused
/// attempt left behind fails as a refusal, never as a second snapshot.
fn vacuum_into(store: &Store, scratch: &Scratch) -> Result<PathBuf, BackupError> {
    let temporary = scratch.path("snapshot.db");
    // `literal::nameable` refused any other destination first; bound by
    // a_path_sqlite_cannot_name_is_refused_by_name.
    let named = temporary
        .to_str()
        .expect("a destination SQLite can name, joined with an ASCII file name");
    patiently("backup", store.patience, || {
        Ok(store.conn.execute("VACUUM INTO ?1", params![named])?)
    })?;
    Ok(temporary)
}

/// The directory a path lives in, with an empty parent read as the
/// current directory (a bare file name has one).
fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// A backup destination that is absent or an empty directory; anything
/// else is refused before a byte is written.
fn prepare_directory(to: &Path) -> Result<(), BackupError> {
    match std::fs::symlink_metadata(to) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            let mut entries = std::fs::read_dir(to)?;
            match entries.next() {
                Some(_) => Err(BackupError::BackupDestinationNotEmpty(to.to_path_buf())),
                None => Ok(()),
            }
        }
        Ok(_) => Err(BackupError::BackupDestinationNotDirectory(to.to_path_buf())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(to)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Publish `temporary` as `final_path` without ever writing over a file
/// that is already there. A hard link refuses an existing name atomically;
/// the temporary is then removed, leaving one name for the inode.
fn publish_noclobber(temporary: &Path, final_path: &Path) -> std::io::Result<()> {
    std::fs::hard_link(temporary, final_path)?;
    let _ = std::fs::remove_file(temporary);
    Ok(())
}

/// Map a publish failure on backup to the empty-place refusal when the
/// name appeared in the race, and to the io error otherwise.
fn backup_publish_error(error: std::io::Error, to: &Path) -> BackupError {
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        BackupError::BackupDestinationNotEmpty(to.to_path_buf())
    } else {
        BackupError::Io(error)
    }
}

/// Map a publish failure on restore to the destination refusal when the
/// name appeared in the race, and to the io error otherwise.
fn destination_error(error: std::io::Error, to: &Path) -> BackupError {
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        BackupError::RestoreDestinationExists(to.to_path_buf())
    } else {
        BackupError::Io(error)
    }
}

/// Write `bytes` to a new file under a temporary name and fsync it.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), BackupError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Copy `from` to a new file at `to`, fsync it, and answer how many bytes
/// the copy read.
fn copy_file(from: &Path, to: &Path) -> Result<u64, BackupError> {
    let mut source = File::open(from)?;
    let mut target = OpenOptions::new().write(true).create_new(true).open(to)?;
    let copied = std::io::copy(&mut source, &mut target)?;
    target.sync_all()?;
    Ok(copied)
}

/// fsync one file already written.
fn sync_file(path: &Path) -> Result<(), BackupError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

/// fsync a directory, so a published name survives a power loss. macOS
/// answers `EINVAL` for a directory fsync; the publish still landed and
/// there is no stronger call to make there, so that one answer is not a
/// refusal.
fn sync_dir(dir: &Path) -> Result<(), BackupError> {
    match File::open(dir).and_then(|file| file.sync_all()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// The SHA-256 of a file's bytes, in lowercase hex.
fn sha256_file(path: &Path) -> Result<String, BackupError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Temporary names one backup owns, all under one random prefix, so a
/// failed or interrupted attempt never leaves a file the next backup
/// could mistake for its own — and never leaves one of its own behind.
struct Scratch {
    dir: PathBuf,
    prefix: String,
}

impl Scratch {
    fn in_directory(dir: &Path) -> Scratch {
        Scratch {
            dir: dir.to_path_buf(),
            prefix: format!(".brokkr-backup-{}-", uuid::Uuid::new_v4()),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}{name}", self.prefix))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A directory that cannot be read holds nothing this can remove.
        for entry in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with(&self.prefix)
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// One restore's temporary name beside its destination, removed on drop
/// unless it was published: a refusal leaves neither the destination nor
/// a temporary behind.
struct Temp {
    path: PathBuf,
    live: bool,
}

impl Temp {
    fn beside(to: &Path) -> Result<Temp, BackupError> {
        let parent = parent_of(to);
        std::fs::create_dir_all(parent)?;
        Ok(Temp {
            path: parent.join(format!(".brokkr-restore-{}.tmp", uuid::Uuid::new_v4())),
            live: true,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(mut self) {
        self.live = false;
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        if self.live {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

mod consistency;
mod literal;

#[cfg(test)]
mod tests;
