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

use super::fs_openat as posix;
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

    fn open_run_dir(&self, run_id: &str) -> Result<(PathBuf, File), StoreError> {
        self.verify_root()?;
        validate_run_id_segment(run_id)?;
        let dir = self.root.join(run_id);
        let run_dir_fd = open_dir_at(&self.root_dir, run_id, &dir)?
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        self.verify_root()?;
        Ok((dir, run_dir_fd))
    }

    fn verify_anchored_file(
        &self,
        run_id: &str,
        run_dir: &Path,
        run_dir_fd: &File,
        file_path: &Path,
        file: &File,
    ) -> Result<(), StoreError> {
        self.verify_root()?;
        let anchored_run = run_dir_fd.metadata()?;
        let cur_run = open_dir_at(&self.root_dir, run_id, run_dir)?
            .map(|dir| dir.metadata())
            .transpose()?;
        let cur_file = open_regular_at(run_dir_fd, EVENTS_FILE, file_path, posix::O_RDONLY)
            .ok()
            .and_then(|found| found.metadata().ok());
        let opened = file.metadata()?;
        let ok = same_dir(&cur_run, &anchored_run)
            && cur_file
                .as_ref()
                .is_some_and(|m| m.is_file() && m.dev() == opened.dev() && m.ino() == opened.ino());
        if !ok {
            return Err(StoreError::SymlinkRefused(file_path.to_path_buf()));
        }
        Ok(())
    }

    pub(super) fn append_envelopes_with(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
        before_write: impl FnOnce(),
    ) -> Result<(), StoreError> {
        let (run_dir, run_dir_fd) = self.open_run_dir(run_id)?;
        let _lock = open_lock_at(&run_dir_fd, &run_dir, true)?;
        let manifest = read_manifest_at(&run_dir_fd, &run_dir, run_id)?;
        let existing = read_verified_events_at(&run_dir_fd, &run_dir, run_id)?;
        validate_append_batch(run_id, &manifest, &existing, expected_head, envelopes)?;
        if !envelopes.is_empty() {
            let ndjson = format_canonical_ndjson(envelopes)?;
            let events_path = run_dir.join(EVENTS_FILE);
            self.verify_root()?;
            let mut file = open_regular_at(
                &run_dir_fd,
                EVENTS_FILE,
                &events_path,
                posix::O_WRONLY | posix::O_APPEND,
            )?;
            before_write();
            self.verify_anchored_file(run_id, &run_dir, &run_dir_fd, &events_path, &file)?;
            append_bytes_atomic_with(&mut file, |f| {
                f.write_all(ndjson.as_bytes())?;
                f.sync_data()
            })?;
        }
        self.verify_root()
    }

    pub(super) fn read_run_events_with(
        &self,
        run_id: &str,
        after_manifest_shared: impl FnOnce(),
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let (run_dir, run_dir_fd) = self.open_run_dir(run_id)?;
        let _lock = open_lock_at_with(&run_dir_fd, &run_dir, false, after_manifest_shared)?;
        let _ = open_manifest_at(&run_dir_fd, &run_dir)?;
        reject_symlink_child(&run_dir_fd, &run_dir, RUN_META_FILE)?;
        let events = read_verified_events_at(&run_dir_fd, &run_dir, run_id)?;
        self.verify_root()?;
        Ok(events)
    }

    fn read_run_events(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        self.read_run_events_with(run_id, || {})
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

fn open_dir_at(parent: &File, name: &str, display_path: &Path) -> Result<Option<File>, StoreError> {
    match posix::open_at(parent, name, posix::O_RDONLY | posix::O_DIRECTORY) {
        Ok(file) => Ok(Some(file)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err)
            if err.raw_os_error() == Some(posix::ELOOP)
                || err.kind() == io::ErrorKind::NotADirectory =>
        {
            // Linux returns ENOTDIR (macOS returns ELOOP) for O_DIRECTORY | O_NOFOLLOW on a
            // symlink; probe anchored to `parent` without O_DIRECTORY to classify symlink vs file.
            match posix::open_at(parent, name, posix::O_RDONLY | posix::O_NONBLOCK) {
                Err(probe) if probe.raw_os_error() == Some(posix::ELOOP) => {
                    Err(StoreError::SymlinkRefused(display_path.to_path_buf()))
                }
                _ => Ok(None),
            }
        }
        Err(err) => Err(StoreError::Io(err)),
    }
}

fn open_regular_at(
    dir: &File,
    name: &str,
    display_path: &Path,
    flags: std::os::raw::c_int,
) -> Result<File, StoreError> {
    let file = posix::open_at(dir, name, flags | posix::O_NONBLOCK)
        .map_err(|err| posix::map_err(err, display_path))?;
    if !file.metadata()?.is_file() {
        return Err(non_regular_error(display_path));
    }
    Ok(file)
}

fn open_manifest_at(run_dir_fd: &File, run_dir: &Path) -> Result<File, StoreError> {
    let path = run_dir.join(MANIFEST_FILE);
    match posix::open_at(
        run_dir_fd,
        MANIFEST_FILE,
        posix::O_RDONLY | posix::O_NONBLOCK,
    ) {
        Ok(file) if file.metadata()?.is_file() => Ok(file),
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(posix::map_err(err, &path)),
        Ok(_) | Err(_) => Err(StoreError::Damaged {
            journal: path,
            damage: Damage::MissingManifest,
        }),
    }
}

fn reject_symlink_child(run_dir_fd: &File, run_dir: &Path, name: &str) -> Result<(), StoreError> {
    let path = run_dir.join(name);
    match posix::open_at(run_dir_fd, name, posix::O_RDONLY | posix::O_NONBLOCK) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(posix::map_err(err, &path)),
    }
}

fn open_create_lock(lock_path: &Path) -> Result<File, StoreError> {
    let file = open_lock_exclusive(lock_path)?;
    file.lock()?;
    Ok(file)
}

struct RunLock {
    _primary: File,
    _fallback: Option<File>,
}

pub(super) fn open_or_create_lock_at(
    run_dir_fd: &File,
    lock_path: &Path,
) -> Result<File, StoreError> {
    match open_regular_at(
        run_dir_fd,
        LOCK_FILE,
        lock_path,
        posix::O_RDWR | posix::O_CREAT | posix::O_EXCL,
    ) {
        Ok(file) => Ok(file),
        Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::AlreadyExists => {
            open_regular_at(run_dir_fd, LOCK_FILE, lock_path, posix::O_RDWR)
        }
        Err(err) => Err(err),
    }
}

fn open_exclusive_run_lock(
    run_dir_fd: &File,
    run_dir: &Path,
    lock_path: &Path,
) -> Result<RunLock, StoreError> {
    match open_regular_at(run_dir_fd, LOCK_FILE, lock_path, posix::O_RDWR) {
        Ok(primary) => {
            primary.lock()?;
            Ok(RunLock {
                _primary: primary,
                _fallback: None,
            })
        }
        Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
            // Legacy run directory without `.lock`: serialize against readers
            // holding `lock_shared()` on `manifest.json` before creating `.lock`
            // and keep that lock held across creating `.lock` + appending.
            let fallback = open_manifest_at(run_dir_fd, run_dir)?;
            fallback.lock()?;
            let primary = open_or_create_lock_at(run_dir_fd, lock_path)?;
            primary.lock()?;
            Ok(RunLock {
                _primary: primary,
                _fallback: Some(fallback),
            })
        }
        Err(err) => Err(err),
    }
}

fn open_lock_at(run_dir_fd: &File, run_dir: &Path, exclusive: bool) -> Result<RunLock, StoreError> {
    open_lock_at_with(run_dir_fd, run_dir, exclusive, || {})
}

fn open_lock_at_with(
    run_dir_fd: &File,
    run_dir: &Path,
    exclusive: bool,
    after_manifest_shared: impl FnOnce(),
) -> Result<RunLock, StoreError> {
    let lock_path = run_dir.join(LOCK_FILE);
    if exclusive {
        return open_exclusive_run_lock(run_dir_fd, run_dir, &lock_path);
    }
    match open_regular_at(run_dir_fd, LOCK_FILE, &lock_path, posix::O_RDONLY) {
        Ok(primary) => {
            primary.lock_shared()?;
            Ok(RunLock {
                _primary: primary,
                _fallback: None,
            })
        }
        Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
            let manifest = open_manifest_at(run_dir_fd, run_dir)?;
            manifest.lock_shared()?;
            after_manifest_shared();
            // Re-check whether `.lock` appeared while acquiring `manifest.json`;
            // if so, drop `manifest.json` and switch to `lock_shared()` on `.lock`
            // so a subsequent writer locking only `.lock` cannot overlap this read.
            match open_regular_at(run_dir_fd, LOCK_FILE, &lock_path, posix::O_RDONLY) {
                Ok(lock_file) => {
                    drop(manifest);
                    lock_file.lock_shared()?;
                    Ok(RunLock {
                        _primary: lock_file,
                        _fallback: None,
                    })
                }
                Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::NotFound => Ok(RunLock {
                    _primary: manifest,
                    _fallback: None,
                }),
                Err(err) => Err(err),
            }
        }
        Err(err) => Err(err),
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
    let _create_lock = open_create_lock(&root.join(CREATE_LOCK_FILE))?;
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

fn read_file_bytes(mut file: File) -> Result<Vec<u8>, StoreError> {
    let mut out = Vec::new();
    file.read_to_end(&mut out)?;
    Ok(out)
}

fn read_manifest_at(run_dir_fd: &File, run_dir: &Path, run_id: &str) -> Result<Value, StoreError> {
    let path = run_dir.join(MANIFEST_FILE);
    let raw = read_file_bytes(open_manifest_at(run_dir_fd, run_dir)?)?;
    let manifest: Value =
        serde_json::from_slice(&raw).map_err(|err| malformed_json_error(&path, &err))?;
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

fn read_verified_events_at(
    run_dir_fd: &File,
    run_dir: &Path,
    run_id: &str,
) -> Result<Vec<EventEnvelope>, StoreError> {
    let path = run_dir.join(EVENTS_FILE);
    let raw = read_file_bytes(open_regular_at(
        run_dir_fd,
        EVENTS_FILE,
        &path,
        posix::O_RDONLY,
    )?)?;
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let content = String::from_utf8(raw).map_err(|err| StoreError::Damaged {
        journal: path.clone(),
        damage: Damage::MalformedJson {
            detail: err.to_string(),
        },
    })?;
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
        self.append_envelopes_with(run_id, expected_head, envelopes, || {})
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
        let (run_dir, run_dir_fd) = self.open_run_dir(run_id)?;
        let _lock = open_lock_at(&run_dir_fd, &run_dir, false)?;
        reject_symlink_child(&run_dir_fd, &run_dir, EVENTS_FILE)?;
        reject_symlink_child(&run_dir_fd, &run_dir, RUN_META_FILE)?;
        let manifest = read_manifest_at(&run_dir_fd, &run_dir, run_id)?;
        self.verify_root()?;
        Ok(manifest)
    }

    fn run_metadata(&self, run_id: &str) -> Result<Option<RunMetadata>, StoreError> {
        let (run_dir, run_dir_fd) = self.open_run_dir(run_id)?;
        let _lock = open_lock_at(&run_dir_fd, &run_dir, false)?;
        let _ = open_manifest_at(&run_dir_fd, &run_dir)?;
        reject_symlink_child(&run_dir_fd, &run_dir, EVENTS_FILE)?;
        let meta_path = run_dir.join(RUN_META_FILE);
        let file = match open_regular_at(&run_dir_fd, RUN_META_FILE, &meta_path, posix::O_RDONLY) {
            Ok(file) => file,
            Err(StoreError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                self.verify_root()?;
                return Ok(None);
            }
            Err(err) => return Err(err),
        };
        let raw = read_file_bytes(file)?;
        let meta: RunMetadata =
            serde_json::from_slice(&raw).map_err(|err| malformed_json_error(&meta_path, &err))?;
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
            let run_dir_fd =
                posix::open_at(&self.root_dir, &name, posix::O_RDONLY | posix::O_DIRECTORY)
                    .map_err(|err| posix::map_err(err, &path))?;
            let _ = open_manifest_at(&run_dir_fd, &path)?;
            reject_symlink_child(&run_dir_fd, &path, EVENTS_FILE)?;
            reject_symlink_child(&run_dir_fd, &path, RUN_META_FILE)?;
            ids.push(name);
        }
        self.verify_root()?;
        ids.sort();
        Ok(ids)
    }
}
