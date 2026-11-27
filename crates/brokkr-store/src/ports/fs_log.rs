//! Per-run filesystem [`FsEventLog`] implementation of [`EventLog`] (decision
//! 0126 ruling 4, Slice 2).
//!
//! Each run lives in its own directory `<root>/<run-id>/` holding:
//! - `manifest.json` — immutable run manifest published atomically with the run
//!   directory via a hidden staging directory, `sync_data()`, and `rename`.
//! - `events.ndjson` — append-only canonical NDJSON event stream written with
//!   `O_APPEND`, `sync_data()`, and rollback truncation on write/sync failure.
//! - `.lock` — per-run advisory lock file coordinated via Rust 1.89's
//!   [`std::fs::File::lock`] and [`std::fs::File::lock_shared`].
//!
//! Independent runs share zero locks on appends and reads, eliminating
//! cross-run write contention.

use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use serde_json::Value;

use crate::{damage, import, Damage, StoreError};

use super::{
    format_canonical_ndjson, reject_symlink, reject_symlink_path, validate_append_batch,
    validate_import_stream, validate_manifest, verify_chain_checked, EventLog,
};

const MANIFEST_FILE: &str = "manifest.json";
const EVENTS_FILE: &str = "events.ndjson";
const LOCK_FILE: &str = ".lock";
const CREATE_LOCK_FILE: &str = ".create.lock";
const STAGING_PREFIX: &str = ".staging-";

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
        if root.as_os_str().is_empty() {
            return Err(StoreError::InvalidPath(root.to_path_buf()));
        }
        let root = root.to_path_buf();
        let _ = reject_symlink_path(&root)?;
        fs::create_dir_all(&root)?;
        let _ = reject_symlink(&root)?;
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

    fn read_run_events(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, false)?;
        let _ = require_regular_manifest(&run_dir)?;
        read_verified_events(&run_dir, run_id)
    }
}

fn validate_run_id_segment(run_id: &str) -> Result<(), StoreError> {
    if run_id.is_empty()
        || run_id == "."
        || run_id == ".."
        || run_id == CREATE_LOCK_FILE
        || run_id.starts_with(STAGING_PREFIX)
        || run_id.contains(['/', '\\', '\0'])
    {
        return Err(StoreError::UnsafeRunId(import::escaped(run_id)));
    }
    Ok(())
}

pub(super) fn open_no_symlink_with(
    path: &Path,
    open: impl FnOnce(&Path) -> io::Result<File>,
) -> Result<File, StoreError> {
    let pre = reject_symlink(path)?;
    let file = open(path)?;
    let post = file.metadata()?;
    let after = reject_symlink(path)?;
    let same = |meta: &Metadata| meta.dev() == post.dev() && meta.ino() == post.ino();
    let intact = after.as_ref().is_some_and(same) && pre.as_ref().is_none_or(same);
    if !intact {
        return Err(StoreError::Io(io::Error::other("path changed during open")));
    }
    Ok(file)
}

fn open_lock_file(lock_path: &Path, exclusive: bool) -> Result<File, StoreError> {
    let file = open_no_symlink_with(lock_path, |path| {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
    })?;
    if exclusive {
        file.lock()?;
    } else {
        file.lock_shared()?;
    }
    Ok(file)
}

fn open_lock(run_dir: &Path, exclusive: bool) -> Result<File, StoreError> {
    open_lock_file(&run_dir.join(LOCK_FILE), exclusive)
}

fn require_regular_manifest(run_dir: &Path) -> Result<PathBuf, StoreError> {
    let path = run_dir.join(MANIFEST_FILE);
    match reject_symlink(&path)? {
        Some(meta) if meta.is_file() => Ok(path),
        _ => Err(StoreError::Damaged {
            journal: path,
            damage: Damage::Malformed,
        }),
    }
}

fn populate_run_dir(
    staging: &Path,
    manifest: &Value,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut events_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staging.join(EVENTS_FILE))?;
    if !envelopes.is_empty() {
        let ndjson = format_canonical_ndjson(envelopes)?;
        events_file.write_all(ndjson.as_bytes())?;
    }
    events_file.sync_data()?;
    let manifest_bytes = serde_json::to_vec(manifest)?;
    let mut manifest_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staging.join(MANIFEST_FILE))?;
    manifest_file.write_all(&manifest_bytes)?;
    manifest_file.sync_data()?;
    File::open(staging)?.sync_all()?;
    Ok(())
}

fn create_run_dir_atomic(
    root: &Path,
    run_id: &str,
    manifest: &Value,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    create_run_dir_atomic_with(root, run_id, |staging| {
        populate_run_dir(staging, manifest, envelopes)
    })
}

pub(super) fn create_run_dir_atomic_with(
    root: &Path,
    run_id: &str,
    populate: impl FnOnce(&Path) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    let _create_lock = open_lock_file(&root.join(CREATE_LOCK_FILE), true)?;
    let run_dir = root.join(run_id);
    if reject_symlink(&run_dir)?.is_some() {
        return Err(StoreError::RunExists(run_id.to_string()));
    }
    let staging = root.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    fs::create_dir(&staging)?;
    let outcome = populate(&staging).and_then(|()| {
        fs::rename(&staging, &run_dir)?;
        File::open(root)?.sync_all()?;
        Ok(())
    });
    if let Err(err) = outcome {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }
    Ok(())
}

pub(super) fn append_bytes_atomic_with(
    file: &mut File,
    write_and_sync: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), StoreError> {
    let original_len = file.metadata()?.len();
    if let Err(err) = write_and_sync(file) {
        let _ = file.set_len(original_len);
        let _ = file.sync_data();
        return Err(StoreError::Io(err));
    }
    Ok(())
}

fn read_file_no_symlink(path: &Path) -> Result<String, StoreError> {
    let mut file = open_no_symlink_with(path, |p| File::open(p))?;
    let mut out = String::new();
    file.read_to_string(&mut out)?;
    Ok(out)
}

fn read_manifest_file(run_dir: &Path, run_id: &str) -> Result<Value, StoreError> {
    let path = require_regular_manifest(run_dir)?;
    let raw = read_file_no_symlink(&path)?;
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
    let content = read_file_no_symlink(&path)?;
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
        validate_append_batch(run_id, &manifest, &existing, expected_head, envelopes)?;
        if !envelopes.is_empty() {
            let ndjson = format_canonical_ndjson(envelopes)?;
            let events_path = run_dir.join(EVENTS_FILE);
            let mut file = open_no_symlink_with(&events_path, |path| {
                OpenOptions::new().create(true).append(true).open(path)
            })?;
            append_bytes_atomic_with(&mut file, |f| {
                f.write_all(ndjson.as_bytes())?;
                f.sync_data()
            })?;
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
        validate_run_id_segment(run_id)?;
        create_run_dir_atomic(&self.root, run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        let events = self.read_run_events(run_id)?;
        Ok(events.last().map_or((0, ZERO_HASH.to_string()), |last| {
            (last.seq, last.event_hash.clone())
        }))
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        Ok(self
            .read_run_events(run_id)?
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
            let path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_symlink() {
                return Err(StoreError::SymlinkRefused(path));
            }
            if !ft.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                return Err(StoreError::Damaged {
                    journal: path,
                    damage: Damage::Malformed,
                });
            };
            if name.starts_with(STAGING_PREFIX) {
                continue;
            }
            let _ = require_regular_manifest(&path)?;
            ids.push(name);
        }
        ids.sort();
        Ok(ids)
    }
}
