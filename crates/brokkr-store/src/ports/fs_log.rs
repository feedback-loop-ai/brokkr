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
    format_canonical_ndjson, normalize_storage_path, reject_symlink, reject_symlink_path, sync_dir,
    validate_append_batch, validate_import_stream, validate_manifest, verify_chain_checked,
    EventLog, RunMetadata,
};

const MANIFEST_FILE: &str = "manifest.json";
const EVENTS_FILE: &str = "events.ndjson";
const RUN_META_FILE: &str = "run_meta.json";
const LOCK_FILE: &str = ".lock";
const CREATE_LOCK_FILE: &str = ".create.lock";
const STAGING_PREFIX: &str = ".staging-";

/// Filesystem-backed per-run [`EventLog`] storing each run under
/// `<root>/<run-id>/manifest.json` and `<root>/<run-id>/events.ndjson`.
pub struct FsEventLog {
    root: PathBuf,
    root_dir: File,
}

impl FsEventLog {
    /// Open (creating if absent) a filesystem event log rooted at `root`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::open_at(root.as_ref())
    }

    fn open_at(root: &Path) -> Result<Self, StoreError> {
        let root = normalize_storage_path(root)?;
        let _ = reject_symlink_path(&root)?;
        fs::create_dir_all(&root)?;
        let pre = reject_symlink_path(&root)?;
        let root_dir = File::open(&root)?;
        let post = root_dir.metadata()?;
        let after = reject_symlink_path(&root)?;
        if !(post.is_dir() && same_dir(&pre, &post) && same_dir(&after, &post)) {
            return Err(StoreError::SymlinkRefused(root));
        }
        Ok(Self { root, root_dir })
    }

    /// Return the normalized root directory holding per-run subdirectories.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Persist a new run with its immutable `manifest` verbatim plus optional
    /// sidecar listing `metadata` in `run_meta.json`, never mutating
    /// `manifest.json`.
    pub fn create_run_with_metadata(
        &self,
        run_id: &str,
        manifest: &Value,
        metadata: &RunMetadata,
    ) -> Result<(), StoreError> {
        self.create_run_inner(run_id, manifest, Some(metadata))
    }

    fn create_run_inner(
        &self,
        run_id: &str,
        manifest: &Value,
        metadata: Option<&RunMetadata>,
    ) -> Result<(), StoreError> {
        self.verify_root()?;
        import::recordable(run_id)?;
        validate_run_id_segment(run_id)?;
        validate_manifest(run_id, manifest)?;
        create_run_dir_atomic(&self.root, run_id, manifest, metadata, &[])?;
        self.verify_root()
    }

    fn verify_root(&self) -> Result<(), StoreError> {
        let anchored = self.root_dir.metadata()?;
        let Some(current) = reject_symlink_path(&self.root)? else {
            return Err(StoreError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "event log root directory disappeared",
            )));
        };
        if !(current.is_dir() && current.dev() == anchored.dev() && current.ino() == anchored.ino())
        {
            return Err(StoreError::SymlinkRefused(self.root.clone()));
        }
        Ok(())
    }

    fn existing_run_dir(&self, run_id: &str) -> Result<PathBuf, StoreError> {
        self.verify_root()?;
        validate_run_id_segment(run_id)?;
        let dir = self.root.join(run_id);
        let found = reject_symlink(&dir)?;
        self.verify_root()?;
        match found {
            Some(meta) if meta.is_dir() => Ok(dir),
            _ => Err(StoreError::RunNotFound(run_id.to_string())),
        }
    }

    fn read_run_events(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, false)?;
        let _ = require_regular_manifest(&run_dir)?;
        let _ = reject_symlink(&run_dir.join(RUN_META_FILE))?;
        let events = read_verified_events(&run_dir, run_id)?;
        self.verify_root()?;
        Ok(events)
    }
}

fn same_dir(candidate: &Option<Metadata>, anchored: &Metadata) -> bool {
    candidate.as_ref().is_some_and(|meta| {
        meta.is_dir() && meta.dev() == anchored.dev() && meta.ino() == anchored.ino()
    })
}

fn malformed_json_error(path: &Path, err: &serde_json::Error) -> StoreError {
    StoreError::Damaged {
        journal: path.to_path_buf(),
        damage: Damage::MalformedJson {
            detail: err.to_string(),
        },
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

fn non_regular_error(path: &Path) -> StoreError {
    StoreError::Io(io::Error::other(format!(
        "refusing non-regular file at '{}'",
        path.display()
    )))
}

pub(super) fn open_no_symlink_with(
    path: &Path,
    open: impl FnOnce(&Path) -> io::Result<File>,
) -> Result<File, StoreError> {
    let pre = reject_symlink(path)?;
    if let Some(meta) = &pre {
        if !meta.is_file() {
            return Err(non_regular_error(path));
        }
    }
    let file = open(path)?;
    let post = file.metadata()?;
    if !post.is_file() {
        return Err(non_regular_error(path));
    }
    let after = reject_symlink(path)?;
    let same = |meta: &Metadata| meta.dev() == post.dev() && meta.ino() == post.ino();
    let intact = after.as_ref().is_some_and(same) && pre.as_ref().is_none_or(same);
    if !intact {
        return Err(StoreError::Io(io::Error::other("path changed during open")));
    }
    Ok(file)
}

pub(super) fn open_lock_exclusive_with(
    lock_path: &Path,
    mut open_rw: impl FnMut(&Path, bool) -> io::Result<File>,
) -> Result<File, StoreError> {
    if reject_symlink(lock_path)?.is_some() {
        return open_no_symlink_with(lock_path, |path| open_rw(path, false));
    }
    match open_no_symlink_with(lock_path, |path| open_rw(path, true)) {
        Ok(file) => Ok(file),
        Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::AlreadyExists => {
            open_no_symlink_with(lock_path, |path| open_rw(path, false))
        }
        Err(err) => Err(err),
    }
}

fn open_lock_exclusive(lock_path: &Path) -> Result<File, StoreError> {
    open_lock_exclusive_with(lock_path, |path, create_new| {
        let mut opts = OpenOptions::new();
        opts.read(true).write(true);
        if create_new {
            opts.create_new(true);
        }
        opts.open(path)
    })
}

fn open_lock_file(lock_path: &Path, exclusive: bool) -> Result<File, StoreError> {
    if exclusive {
        let file = open_lock_exclusive(lock_path)?;
        file.lock()?;
        Ok(file)
    } else {
        let file =
            open_no_symlink_with(lock_path, |path| OpenOptions::new().read(true).open(path))?;
        file.lock_shared()?;
        Ok(file)
    }
}

fn open_lock(run_dir: &Path, exclusive: bool) -> Result<File, StoreError> {
    match open_lock_file(&run_dir.join(LOCK_FILE), exclusive) {
        Ok(file) => Ok(file),
        Err(StoreError::Io(err)) if !exclusive && err.kind() == io::ErrorKind::NotFound => {
            let manifest_path = require_regular_manifest(run_dir)?;
            open_lock_file(&manifest_path, false)
        }
        Err(err) => Err(err),
    }
}

fn require_regular_manifest(run_dir: &Path) -> Result<PathBuf, StoreError> {
    let path = run_dir.join(MANIFEST_FILE);
    match reject_symlink(&path)? {
        Some(meta) if meta.is_file() => Ok(path),
        _ => Err(StoreError::Damaged {
            journal: path,
            damage: Damage::MissingManifest,
        }),
    }
}

fn write_new_synced_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    if !bytes.is_empty() {
        file.write_all(bytes)?;
    }
    file.sync_data()?;
    Ok(())
}

fn populate_run_dir(
    staging: &Path,
    manifest: &Value,
    metadata: Option<&RunMetadata>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let ndjson = if envelopes.is_empty() {
        String::new()
    } else {
        format_canonical_ndjson(envelopes)?
    };
    write_new_synced_file(&staging.join(EVENTS_FILE), ndjson.as_bytes())?;
    let manifest_bytes = serde_json::to_vec(manifest)?;
    write_new_synced_file(&staging.join(MANIFEST_FILE), &manifest_bytes)?;
    write_new_synced_file(&staging.join(LOCK_FILE), &[])?;
    if let Some(meta) = metadata {
        let meta_bytes = serde_json::to_vec(meta)?;
        write_new_synced_file(&staging.join(RUN_META_FILE), &meta_bytes)?;
    }
    sync_dir(staging)?;
    Ok(())
}

fn create_run_dir_atomic(
    root: &Path,
    run_id: &str,
    manifest: &Value,
    metadata: Option<&RunMetadata>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    create_run_dir_atomic_with(
        root,
        run_id,
        |staging| populate_run_dir(staging, manifest, metadata, envelopes),
        sync_dir,
    )
}

pub(super) fn create_run_dir_atomic_with(
    root: &Path,
    run_id: &str,
    populate: impl FnOnce(&Path) -> Result<(), StoreError>,
    sync_root: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), StoreError> {
    let _create_lock = open_lock_file(&root.join(CREATE_LOCK_FILE), true)?;
    let run_dir = root.join(run_id);
    if reject_symlink(&run_dir)?.is_some() {
        return Err(StoreError::RunExists(run_id.to_string()));
    }
    let staging = root.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    fs::create_dir(&staging)?;
    let published = populate(&staging).and_then(|()| {
        fs::rename(&staging, &run_dir)?;
        Ok(())
    });
    if let Err(err) = published {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }
    if let Err(err) = sync_root(root) {
        let _ = fs::remove_dir_all(&run_dir);
        return Err(StoreError::Io(err));
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
    let manifest: Value =
        serde_json::from_str(&raw).map_err(|err| malformed_json_error(&path, &err))?;
    validate_manifest(run_id, &manifest)?;
    Ok(manifest)
}

fn parse_ndjson_line(
    events_path: &Path,
    run_id: &str,
    line_no: i64,
    line: &str,
) -> Result<EventEnvelope, StoreError> {
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
            Err(malformed_json_error(events_path, &err))
        }
        Err(err) => Err(damage::classified(events_path, err)),
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
        return Err(StoreError::Damaged {
            journal: path,
            damage: Damage::Envelope {
                run_id: run_id.to_string(),
                seq: line_no,
            },
        });
    };
    let mut events = Vec::new();
    for (idx, line) in body.split('\n').enumerate() {
        events.push(parse_ndjson_line(&path, run_id, (idx + 1) as i64, line)?);
    }
    verify_chain_checked(run_id, (0, ZERO_HASH), &events)?;
    Ok(events)
}

impl EventLog for FsEventLog {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        self.create_run_inner(run_id, manifest, None)
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
                OpenOptions::new().append(true).open(path)
            })?;
            append_bytes_atomic_with(&mut file, |f| {
                f.write_all(ndjson.as_bytes())?;
                f.sync_data()
            })?;
        }
        self.verify_root()
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        self.verify_root()?;
        validate_import_stream(run_id, manifest, envelopes)?;
        validate_run_id_segment(run_id)?;
        create_run_dir_atomic(&self.root, run_id, manifest, None, envelopes)?;
        self.verify_root()
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
        let _ = reject_symlink(&run_dir.join(EVENTS_FILE))?;
        let _ = reject_symlink(&run_dir.join(RUN_META_FILE))?;
        let manifest = read_manifest_file(&run_dir, run_id)?;
        self.verify_root()?;
        Ok(manifest)
    }

    fn run_metadata(&self, run_id: &str) -> Result<Option<RunMetadata>, StoreError> {
        let run_dir = self.existing_run_dir(run_id)?;
        let _lock = open_lock(&run_dir, false)?;
        let _ = require_regular_manifest(&run_dir)?;
        let _ = reject_symlink(&run_dir.join(EVENTS_FILE))?;
        let meta_path = run_dir.join(RUN_META_FILE);
        if reject_symlink(&meta_path)?.is_none() {
            self.verify_root()?;
            return Ok(None);
        }
        let raw = read_file_no_symlink(&meta_path)?;
        let meta: RunMetadata =
            serde_json::from_str(&raw).map_err(|err| malformed_json_error(&meta_path, &err))?;
        self.verify_root()?;
        Ok(Some(meta))
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        self.verify_root()?;
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
                    damage: Damage::NonUtf8RunDir,
                });
            };
            if name.starts_with(STAGING_PREFIX) {
                continue;
            }
            let _ = require_regular_manifest(&path)?;
            let _ = reject_symlink(&path.join(EVENTS_FILE))?;
            let _ = reject_symlink(&path.join(RUN_META_FILE))?;
            ids.push(name);
        }
        self.verify_root()?;
        ids.sort();
        Ok(ids)
    }
}
