//! Per-run filesystem [`FsEventLog`] implementation of [`EventLog`] (decision
//! 0126 ruling 4, Slice 2).
//!
//! Each run lives in its own directory `<root>/<run-id>/` holding:
//! - `manifest.json` — immutable run manifest written via temp-file +
//!   `sync_data()` + atomic `rename`.
//! - `events.ndjson` — append-only canonical NDJSON event stream written with
//!   `O_APPEND` and `sync_data()`.
//! - `.lock` — per-run advisory lock file coordinated via Rust 1.89's
//!   [`std::fs::File::lock`] and [`std::fs::File::lock_shared`].
//!
//! Independent runs share zero locks, eliminating cross-run write contention.

use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::Value;

use crate::{damage, import, StoreError};

use super::{
    format_canonical_ndjson, refuse_after_terminal, validate_import_stream, validate_manifest,
    validate_seat_records, validate_started_manifests, verify_chain_checked, EventLog,
};

const MANIFEST_FILE: &str = "manifest.json";
const EVENTS_FILE: &str = "events.ndjson";
const LOCK_FILE: &str = ".lock";

/// Filesystem-backed per-run [`EventLog`] storing each run under
/// `<root>/<run-id>/manifest.json` and `<root>/<run-id>/events.ndjson`.
pub struct FsEventLog {
    root: PathBuf,
}

impl FsEventLog {
    /// Open (creating if absent) a filesystem event log rooted at `root`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::open_at(root.as_ref())
    }

    fn open_at(root: &Path) -> Result<Self, StoreError> {
        let root = root.to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Return the root directory holding per-run subdirectories.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn existing_run_dir(&self, run_id: &str) -> Result<PathBuf, StoreError> {
        validate_run_id_segment(run_id)?;
        let dir = self.root.join(run_id);
        match reject_symlink(&dir)? {
            Some(meta) if meta.is_dir() => Ok(dir),
            _ => Err(StoreError::RunNotFound(run_id.to_string())),
        }
    }
}

fn validate_run_id_segment(run_id: &str) -> Result<(), StoreError> {
    if run_id.is_empty() || run_id == "." || run_id == ".." || run_id.contains(['/', '\\', '\0']) {
        return Err(StoreError::UnsafeRunId(import::escaped(run_id)));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<Option<Metadata>, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(StoreError::SymlinkRefused(path.to_path_buf()))
        }
        Ok(meta) => Ok(Some(meta)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(StoreError::Io(err)),
    }
}

fn open_lock(run_dir: &Path, exclusive: bool) -> Result<File, StoreError> {
    let lock_path = run_dir.join(LOCK_FILE);
    let _ = reject_symlink(&lock_path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    if exclusive {
        file.lock()?;
    } else {
        file.lock_shared()?;
    }
    Ok(file)
}

fn write_atomic(dir: &Path, final_name: &str, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp_path = dir.join(format!(".{final_name}.{}.tmp", uuid::Uuid::new_v4()));
    let mut tmp = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    tmp.write_all(bytes)?;
    tmp.sync_data()?;
    drop(tmp);
    fs::rename(&tmp_path, dir.join(final_name))?;
    Ok(())
}

fn populate_run_dir(
    run_dir: &Path,
    manifest: &Value,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let _lock = open_lock(run_dir, true)?;
    let mut events_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(run_dir.join(EVENTS_FILE))?;
    if !envelopes.is_empty() {
        let ndjson = format_canonical_ndjson(envelopes)?;
        events_file.write_all(ndjson.as_bytes())?;
    }
    events_file.sync_data()?;
    let manifest_bytes = serde_json::to_vec(manifest)?;
    write_atomic(run_dir, MANIFEST_FILE, &manifest_bytes)?;
    File::open(run_dir)?.sync_all()?;
    Ok(())
}

fn create_run_dir_atomic(
    root: &Path,
    run_id: &str,
    manifest: &Value,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    create_run_dir_atomic_with(root, run_id, |run_dir| {
        populate_run_dir(run_dir, manifest, envelopes)
    })
}

pub(super) fn create_run_dir_atomic_with(
    root: &Path,
    run_id: &str,
    populate: impl FnOnce(&Path) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    let run_dir = root.join(run_id);
    let _ = reject_symlink(&run_dir)?;
    match fs::create_dir(&run_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            return Err(StoreError::RunExists(run_id.to_string()));
        }
        Err(err) => return Err(StoreError::Io(err)),
    }
    if let Err(err) = populate(&run_dir) {
        let _ = fs::remove_dir_all(&run_dir);
        return Err(err);
    }
    Ok(())
}

fn read_manifest_file(run_dir: &Path, run_id: &str) -> Result<Value, StoreError> {
    let path = run_dir.join(MANIFEST_FILE);
    let _ = reject_symlink(&path)?;
    let raw = fs::read_to_string(&path)?;
    let manifest: Value = serde_json::from_str(&raw)?;
    validate_manifest(run_id, &manifest)?;
    Ok(manifest)
}

fn parse_ndjson_line(run_id: &str, line_no: i64, line: &str) -> Result<EventEnvelope, StoreError> {
    match damage::envelope(run_id, line_no, line) {
        Ok(envelope) if envelope.seq < 1 => Err(StoreError::RowBeforeChain {
            run_id: run_id.to_string(),
            seq: 0,
        }),
        Ok(envelope) => Ok(envelope),
        Err(StoreError::Json(err)) => {
            if let Some(seq) = serde_json::from_str::<Value>(line)
                .ok()
                .and_then(|value| value.get("seq").and_then(Value::as_i64))
                .filter(|seq| *seq < 1)
            {
                return Err(StoreError::RowBeforeChain {
                    run_id: run_id.to_string(),
                    seq,
                });
            }
            Err(StoreError::Json(err))
        }
        Err(err) => Err(err),
    }
}

fn read_verified_events(run_dir: &Path, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
    let path = run_dir.join(EVENTS_FILE);
    let _ = reject_symlink(&path)?;
    let content = fs::read_to_string(&path)?;
    if content.is_empty() {
        return Ok(Vec::new());
    }
    let Some(body) = content.strip_suffix('\n') else {
        let line_no = content.lines().count().max(1) as i64;
        return Err(StoreError::Unparsed {
            run_id: run_id.to_string(),
            seq: line_no,
        });
    };
    let mut events = Vec::new();
    for (idx, line) in body.split('\n').enumerate() {
        events.push(parse_ndjson_line(run_id, (idx + 1) as i64, line)?);
    }
    verify_chain_checked(run_id, (0, ZERO_HASH), &events)?;
    Ok(events)
}

fn is_regular_manifest(run_dir: &Path) -> bool {
    fs::symlink_metadata(run_dir.join(MANIFEST_FILE))
        .is_ok_and(|meta| !meta.file_type().is_symlink() && meta.is_file())
}

impl EventLog for FsEventLog {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        import::recordable(run_id)?;
        validate_run_id_segment(run_id)?;
        validate_manifest(run_id, manifest)?;
        create_run_dir_atomic(&self.root, run_id, manifest, &[])
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, true)?;
        let manifest = read_manifest_file(&run_dir, run_id)?;
        let existing = read_verified_events(&run_dir, run_id)?;
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
        refuse_after_terminal(run_id, &existing, &incoming)?;
        validate_started_manifests(run_id, envelopes)?;
        validate_seat_records(&manifest, &existing, envelopes)?;
        if !envelopes.is_empty() {
            let ndjson = format_canonical_ndjson(envelopes)?;
            let events_path = run_dir.join(EVENTS_FILE);
            let _ = reject_symlink(&events_path)?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&events_path)?;
            file.write_all(ndjson.as_bytes())?;
            file.sync_data()?;
        }
        Ok(())
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        validate_import_stream(run_id, manifest, envelopes)?;
        create_run_dir_atomic(&self.root, run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, false)?;
        let events = read_verified_events(&run_dir, run_id)?;
        Ok(events.last().map_or((0, ZERO_HASH.to_string()), |last| {
            (last.seq, last.event_hash.clone())
        }))
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, false)?;
        let events = read_verified_events(&run_dir, run_id)?;
        Ok(events
            .into_iter()
            .filter(|event| event.seq >= from_seq)
            .collect())
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, false)?;
        read_manifest_file(&run_dir, run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            if ft.is_symlink() || !ft.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') || validate_run_id_segment(&name).is_err() {
                continue;
            }
            if is_regular_manifest(&entry.path()) {
                ids.push(name);
            }
        }
        ids.sort();
        Ok(ids)
    }
}
