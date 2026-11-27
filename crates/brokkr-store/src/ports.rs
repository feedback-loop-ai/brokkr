//! Synchronous storage ports and the backend-agnostic [`Journal`] facade
//! (decision 0126, rulings 1–4).
//!
//! `brokkr-store` separates three persistence concerns behind narrow
//! synchronous traits — [`EventLog`], [`ProjectionStore`], and
//! [`CoordinationStore`] — with no async runtime dependency. Each trait
//! marks a persistence seam whose operations must stay mutually consistent
//! across a storage backend boundary (decision 0071 ruling 2): a co-located
//! SQLite adapter over [`crate::Store`] today (Slice 1, via [`SqlitePorts`]),
//! local split files and trigger-free disposable indexes in Slice 2, and a
//! PostgreSQL service backend in Slice 3.
//!
//! Domain invariants (run-id and manifest admission, hash-chain continuity,
//! optimistic `expected_head` fencing, post-terminal refusal, seat-record
//! schema validation, and canonical NDJSON export/import) live once in
//! [`Journal`].

use std::collections::BTreeSet;
use std::ffi::{CString, OsString};
use std::fs::{File, Metadata};
use std::io;
use std::path::{Component, Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, ChainError, EventEnvelope, EventType};
use brokkr_core::fold::{lawful_after_terminal, terminal_event};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use rusqlite::{Connection, OpenFlags};

use crate::{
    ensure_wal, import, literal, patiently, seat_record, Cooldown, EntryId, ImportError, NewEntry,
    QueueEntry, SeatRecordVersion, StoreError, VerifyError, BUSY_TIMEOUT,
};

mod fs_log;
mod journal;
mod sqlite;
mod sqlite_coordination;
mod sqlite_projection;

pub use fs_log::FsEventLog;
pub use journal::{Journal, NewEvent};
pub use sqlite::SqlitePorts;
pub use sqlite_coordination::SqliteCoordinationStore;
pub use sqlite_projection::SqliteProjectionStore;

/// Why a run manifest was refused at the storage port boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ManifestRefusal {
    #[error("manifest must be a JSON object")]
    NotAnObject,
    #[error("manifest field 'engine' must be a JSON string")]
    EngineNotString,
    #[error("manifest field 'feature' must be a JSON string")]
    FeatureNotString,
    #[error("manifest field 'bundle_name' must be a JSON string")]
    BundleNameNotString,
}

pub(crate) fn format_canonical_ndjson(events: &[EventEnvelope]) -> Result<String, StoreError> {
    let mut out = String::new();
    for event in events {
        out.push_str(&serde_json::to_string(&serde_json::to_value(event)?)?);
        out.push('\n');
    }
    Ok(out)
}

/// Validate that `manifest` is a JSON object and, if `"engine"`, `"feature"`,
/// or `"bundle_name"` is present, that each is a JSON string (refusing
/// non-string `"engine"` rather than silently treating it as absent/V1, and
/// refusing non-string `"feature"` or `"bundle_name"` rather than projecting an
/// empty metadata value).
pub(crate) fn validate_manifest(run_id: &str, manifest: &Value) -> Result<(), StoreError> {
    let Some(object) = manifest.as_object() else {
        return Err(StoreError::InvalidManifest {
            run_id: run_id.to_string(),
            reason: ManifestRefusal::NotAnObject,
        });
    };
    for (field, reason) in [
        ("engine", ManifestRefusal::EngineNotString),
        ("feature", ManifestRefusal::FeatureNotString),
        ("bundle_name", ManifestRefusal::BundleNameNotString),
    ] {
        if object.get(field).is_some_and(|value| !value.is_string()) {
            return Err(StoreError::InvalidManifest {
                run_id: run_id.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// Validate an embedded `manifest` inside a `RunStarted` payload, if present.
pub(crate) fn validate_started_manifest(
    run_id: &str,
    event_type: EventType,
    payload: &Value,
) -> Result<(), StoreError> {
    if event_type == EventType::RunStarted {
        if let Some(manifest) = payload.get("manifest") {
            validate_manifest(run_id, manifest)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_started_manifests(
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        validate_started_manifest(run_id, envelope.event_type, &envelope.payload)?;
    }
    Ok(())
}

pub(crate) fn checked_seq(seq: u64) -> Result<i64, StoreError> {
    i64::try_from(seq).map_err(|_| {
        StoreError::Chain(ChainError::SeqOverflow {
            after: i64::MAX as u64,
        })
    })
}

pub(crate) fn verify_chain_checked(
    run_id: &str,
    expected_head: (u64, &str),
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        let _ = checked_seq(envelope.seq)?;
    }
    verify_chain_after(run_id, expected_head.0, expected_head.1, envelopes)?;
    Ok(())
}

pub(crate) fn validate_import_stream(
    run_id: &str,
    manifest: &Value,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    import::admit_run_id(run_id).map_err(|err| StoreError::Import(Box::new(err)))?;
    validate_manifest(run_id, manifest)?;
    verify_chain_checked(run_id, (0, ZERO_HASH), envelopes)?;
    let incoming: Vec<EventType> = envelopes.iter().map(|e| e.event_type).collect();
    refuse_after_terminal(run_id, &[], &incoming)?;
    validate_started_manifests(run_id, envelopes)?;
    validate_seat_records(manifest, &[], envelopes)?;
    brokkr_core::fold(envelopes)
        .map_err(|err| StoreError::Import(Box::new(ImportError::Verify(VerifyError::Fold(err)))))?;
    Ok(())
}

/// Lexically normalize an already-absolute path by dropping `.` and trailing
/// separators and resolving `..` segments without touching the filesystem.
pub fn lexically_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Validate that `path` is non-empty and free of NUL bytes, make it absolute
/// against the current directory, and lexically normalize it before any
/// symlink check, sidecar check, or filesystem mutation.
pub fn normalize_storage_path(path: &Path) -> Result<PathBuf, StoreError> {
    if path.as_os_str().is_empty() || CString::new(path.as_os_str().as_encoded_bytes()).is_err() {
        return Err(StoreError::InvalidPath(path.to_path_buf()));
    }
    let abs = std::path::absolute(path)?;
    Ok(lexically_normalize(&abs))
}

/// Inspect `path` without following symlinks, refusing any symlink with
/// [`StoreError::SymlinkRefused`] and returning `Ok(None)` when absent.
pub fn reject_symlink(path: &Path) -> Result<Option<Metadata>, StoreError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(StoreError::SymlinkRefused(path.to_path_buf()))
        }
        Ok(meta) => Ok(Some(meta)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(StoreError::Io(err)),
    }
}

/// Walk every existing prefix of a normalized `path` from root to leaf,
/// refusing any symlink ancestor or leaf with [`StoreError::SymlinkRefused`].
pub fn reject_symlink_path(path: &Path) -> Result<Option<Metadata>, StoreError> {
    let mut ancestors: Vec<&Path> = path.ancestors().collect();
    ancestors.reverse();
    let mut leaf = None;
    for ancestor in ancestors {
        if ancestor.as_os_str().is_empty() || ancestor == Path::new("/") {
            continue;
        }
        #[cfg(target_os = "macos")]
        if matches!(ancestor.to_str(), Some("/tmp" | "/var" | "/etc")) {
            continue;
        }
        match reject_symlink(ancestor)? {
            Some(meta) => {
                if ancestor == path {
                    leaf = Some(meta);
                }
            }
            None => break,
        }
    }
    Ok(leaf)
}

/// Append a SQLite sidecar suffix (`"-wal"`, `"-shm"`) to `path`.
pub fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

/// fsync a directory so published directory entries survive power loss,
/// ignoring macOS `EINVAL` on directory fsync (matching `backup::sync_dir`).
pub(crate) fn sync_dir_result(res: io::Result<()>) -> io::Result<()> {
    match res {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn sync_dir(dir: &Path) -> io::Result<()> {
    sync_dir_result(File::open(dir).and_then(|file| file.sync_all()))
}

fn open_standalone_sqlite_inner(
    path: &Path,
    migrate_op: &'static str,
    inspect_existing: &dyn Fn(&Connection, &Path) -> Result<(), StoreError>,
) -> Result<(Connection, PathBuf), StoreError> {
    let normalized = normalize_storage_path(path)?;
    let _ = reject_symlink_path(&normalized)?;
    let _ = reject_symlink(&sqlite_sidecar_path(&normalized, "-wal"))?;
    let _ = reject_symlink(&sqlite_sidecar_path(&normalized, "-shm"))?;
    let existed_non_empty = reject_symlink(&normalized)?.is_some_and(|meta| meta.len() > 0);
    if let Some(parent) = normalized.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let _ = reject_symlink_path(&normalized)?;
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(&normalized, flags)
        .map_err(|e| literal::as_given(e, path, &normalized))?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    if existed_non_empty {
        patiently(migrate_op, BUSY_TIMEOUT, &normalized, || {
            inspect_existing(&conn, &normalized)
        })?;
    }
    ensure_wal(&conn, &normalized)?;
    Ok((conn, normalized))
}

pub(crate) fn open_standalone_sqlite(
    path: &Path,
    migrate_op: &'static str,
    inspect_existing: impl Fn(&Connection, &Path) -> Result<(), StoreError>,
) -> Result<(Connection, PathBuf), StoreError> {
    let display_path = normalize_storage_path(path).unwrap_or_else(|_| path.to_path_buf());
    open_standalone_sqlite_inner(path, migrate_op, &inspect_existing)
        .map_err(|err| crate::damage::classified(&display_path, err))
}

pub(super) fn open_standalone_store(
    path: &Path,
    migrate_op: &'static str,
    inspect_existing: impl Fn(&Connection, &Path) -> Result<(), StoreError>,
    configure: impl FnOnce(&Connection) -> Result<(), StoreError>,
    ensure_schema: impl Fn(&Connection, &Path) -> Result<(), StoreError>,
) -> Result<(Connection, PathBuf), StoreError> {
    let (conn, normalized) = open_standalone_sqlite(path, migrate_op, inspect_existing)?;
    configure(&conn)?;
    patiently(migrate_op, BUSY_TIMEOUT, &normalized, || {
        ensure_schema(&conn, &normalized)
    })?;
    Ok((conn, normalized))
}

pub(super) fn validate_append_batch(
    run_id: &str,
    manifest: &Value,
    existing: &[EventEnvelope],
    expected_head: (u64, &str),
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let (head_seq, found_hash) = existing
        .last()
        .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
    if head_seq != expected_head.0 || found_hash != expected_head.1 {
        return Err(StoreError::HeadMoved {
            expected_seq: expected_head.0,
            found_seq: head_seq,
        });
    }
    verify_chain_checked(run_id, expected_head, envelopes)?;
    let incoming: Vec<EventType> = envelopes.iter().map(|e| e.event_type).collect();
    refuse_after_terminal(run_id, existing, &incoming)?;
    validate_started_manifests(run_id, envelopes)?;
    validate_seat_records(manifest, existing, envelopes)?;
    Ok(())
}

pub(crate) fn parse_clock(now: &str) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::parse(now, &Rfc3339).map_err(|_| StoreError::Clock {
        now: now.to_string(),
    })
}

pub(crate) fn divergent(run_id: &str) -> StoreError {
    StoreError::DivergentEventLog {
        run_id: run_id.to_string(),
    }
}

pub(crate) fn manifest_meta(manifest: &Value) -> (&str, &str) {
    let feature = manifest
        .get("feature")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let bundle_name = manifest
        .get("bundle_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    (feature, bundle_name)
}

/// Optional run-level listing metadata (`feature`, `created_at`, and optional
/// listing order) carried beside — never inside — a run's immutable
/// `manifest.json` so migrated and unstarted runs preserve `Store::list_runs()`
/// parity across projection rebuilds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunMetadata {
    pub feature: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_order: Option<u64>,
}

pub(crate) struct VerifiedRun {
    pub(crate) run_id: String,
    pub(crate) manifest: Value,
    pub(crate) metadata: Option<RunMetadata>,
    pub(crate) events: Vec<EventEnvelope>,
}

/// Read and verify every run from `log`, refusing duplicate `run_id`s with
/// [`StoreError::DivergentEventLog`], invalid manifests with
/// [`StoreError::InvalidManifest`], and broken chains with [`StoreError::Chain`].
pub(crate) fn read_verified_log(log: &dyn EventLog) -> Result<Vec<VerifiedRun>, StoreError> {
    let run_ids = log.list_run_ids()?;
    let mut seen = BTreeSet::new();
    let mut verified = Vec::with_capacity(run_ids.len());
    for run_id in run_ids {
        if !seen.insert(run_id.clone()) {
            return Err(divergent(&run_id));
        }
        let manifest = log.manifest(&run_id)?;
        validate_manifest(&run_id, &manifest)?;
        let metadata = log.run_metadata(&run_id)?;
        let events = log.read_stream(&run_id, 1)?;
        verify_chain_checked(&run_id, (0, ZERO_HASH), &events)?;
        validate_started_manifests(&run_id, &events)?;
        verified.push(VerifiedRun {
            run_id,
            manifest,
            metadata,
            events,
        });
    }
    Ok(verified)
}

pub(crate) fn non_empty_str(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|s| !s.is_empty())
}

pub(crate) fn started_feature(payload: &Value) -> Option<&str> {
    non_empty_str(payload.get("feature"))
        .or_else(|| non_empty_str(payload.pointer("/manifest/feature")))
}

pub(crate) fn started_feature_of(events: &[EventEnvelope]) -> Option<&str> {
    events
        .first()
        .filter(|event| event.seq == 1 && event.event_type == EventType::RunStarted)
        .and_then(|event| started_feature(&event.payload))
}

/// Validate every incoming envelope's seat record (`checkpoint` / `result`)
/// and marked `settled_members` against the stricter [`SeatRecordVersion`] of
/// the persisted `manifest` and `RunStarted`'s `/manifest/engine`.
pub(crate) fn validate_seat_records(
    manifest: &Value,
    existing: &[EventEnvelope],
    incoming: &[EventEnvelope],
) -> Result<(), StoreError> {
    let manifest_version =
        SeatRecordVersion::of_manifest(manifest.get("engine").and_then(Value::as_str));
    let started_version = seat_record::version_of_events(&[existing, incoming]);
    let version = manifest_version.stricter(started_version);
    seat_record::validate_incoming(existing, incoming, version)?;
    Ok(())
}

/// Refuse any non-annotation event in `incoming` once `existing` or an earlier
/// event in `incoming` has reached a terminal event.
pub(crate) fn refuse_after_terminal(
    run_id: &str,
    existing: &[EventEnvelope],
    incoming: &[EventType],
) -> Result<(), StoreError> {
    let mut terminal = existing
        .iter()
        .rev()
        .find_map(|event| terminal_event(event.event_type));
    for &event_type in incoming {
        if let Some(status) = terminal {
            if !lawful_after_terminal(event_type) {
                return Err(StoreError::AfterTerminal {
                    run_id: run_id.to_string(),
                    status,
                    event_type,
                });
            }
        } else {
            terminal = terminal_event(event_type);
        }
    }
    Ok(())
}

/// Persistence seam for durable per-run manifests and append-only event
/// streams (decision 0126 ruling 1; decision 0071 ruling 2).
///
/// Implementations persist pre-sealed envelopes atomically under an
/// optimistic `(seq, hash)` head fence and serve streams in sequence order;
/// domain validation and hashing remain in [`Journal`].
pub trait EventLog {
    /// Persist a new run record with its immutable manifest, refusing a
    /// duplicate `run_id` with [`StoreError::RunExists`] and an invalid
    /// manifest (see [`ManifestRefusal`]) with [`StoreError::InvalidManifest`].
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError>;

    /// Atomically append pre-sealed `envelopes` to `run_id` only while the
    /// stored head matches `expected_head`, refusing a moved head with
    /// [`StoreError::HeadMoved`], a duplicate or non-contiguous sequence number
    /// in a batch with [`brokkr_core::envelope::ChainError::SeqGap`], a foreign
    /// `run_id` with [`brokkr_core::envelope::ChainError::ForeignRun`],
    /// non-annotation events after a terminal event with
    /// [`StoreError::AfterTerminal`], an invalid embedded `RunStarted`
    /// manifest with [`StoreError::InvalidManifest`], and malformed seat
    /// records or settled members with [`StoreError::SeatRecord`].
    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// Atomically persist an imported run record and its verified pre-sealed
    /// `envelopes` in one transaction or unit of work, refusing an unadoptable
    /// `run_id` or an unfoldable non-empty stream with [`StoreError::Import`],
    /// non-annotation events after a terminal event with
    /// [`StoreError::AfterTerminal`], an invalid top-level or embedded
    /// `RunStarted` manifest with [`StoreError::InvalidManifest`], and
    /// malformed seat records or settled members with
    /// [`StoreError::SeatRecord`], leaving zero partial state on refusal, and
    /// leaving local origin unset so imported runs are never mistaken for runs
    /// started on this host (decision 0030 ruling 4).
    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// Return the current `(seq, event_hash)` head of `run_id`, or
    /// `(0, ZERO_HASH)` when the run holds no events yet.
    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError>;

    /// Read all stored envelopes of `run_id` with `seq >= from_seq`, in
    /// ascending sequence order. When `from_seq <= 1`, reads the run's full
    /// stored stream and refuses rows before sequence 1.
    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError>;

    /// Read the immutable manifest recorded when `run_id` was created.
    fn manifest(&self, run_id: &str) -> Result<Value, StoreError>;

    /// Read optional run-level listing metadata (`feature`, `created_at`,
    /// `list_order`) stored beside `manifest.json` when available.
    fn run_metadata(&self, _run_id: &str) -> Result<Option<RunMetadata>, StoreError> {
        Ok(None)
    }

    /// List every run id held by this event log.
    fn list_run_ids(&self) -> Result<Vec<String>, StoreError>;
}

/// Persistence seam for rebuildable read-model projections derived from an
/// [`EventLog`] (decision 0126 ruling 1; decision 0071 ruling 2).
///
/// Every row managed behind this trait is disposable and can be
/// reconstructed from an [`EventLog`] via [`ProjectionStore::rebuild_from`].
pub trait ProjectionStore {
    /// Project committed `envelopes` for `run_id` into read models.
    ///
    /// `manifest` supplies the run's persisted/attested manifest when known so
    /// decoupled projections have the manifest's metadata fallback (such as
    /// `"feature"`) even when `RunStarted` omits it. Co-located adapters such
    /// as [`SqlitePorts`] require `run_id` to exist in their local event log
    /// and verify that `manifest` (when supplied) and `envelopes` match the
    /// local verified stream; decoupled projection stores require a verified
    /// prefix starting at sequence 1. Implementations must remain monotonic and
    /// never regress a run whose projection is already at or past the batch's
    /// highest sequence number.
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// List known runs as `(run_id, feature, created_at)` triples.
    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError>;

    /// Rebuild projections from `log`, returning the total number of events
    /// projected across all runs.
    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError>;
}

/// Persistence seam for mutable operational coordination state — the
/// dispatcher queue and active provider cooldowns (decision 0126 rulings 1
/// and 3; decision 0071 ruling 2).
pub trait CoordinationStore {
    /// Enqueue a new work entry at the given RFC 3339 timestamp `now`.
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError>;

    /// List queue entries in dispatch order.
    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError>;

    /// Return all provider cooldowns still in force at the RFC 3339 instant
    /// `now`.
    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError>;
}

impl<T: EventLog + ?Sized> EventLog for &T {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        (**self).create_run(run_id, manifest)
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        (**self).append_envelopes(run_id, expected_head, envelopes)
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        (**self).import_run_stream(run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        (**self).head(run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        (**self).read_stream(run_id, from_seq)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        (**self).manifest(run_id)
    }

    fn run_metadata(&self, run_id: &str) -> Result<Option<RunMetadata>, StoreError> {
        (**self).run_metadata(run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        (**self).list_run_ids()
    }
}

impl<T: ProjectionStore + ?Sized> ProjectionStore for &T {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        (**self).apply_envelopes(run_id, manifest, envelopes)
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        (**self).list_runs()
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        (**self).rebuild_from(log)
    }
}

impl<T: CoordinationStore + ?Sized> CoordinationStore for &T {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        (**self).enqueue(entry, now)
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        (**self).list_queue()
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        (**self).active_cooldowns(now)
    }
}

#[cfg(test)]
mod tests;
