//! Local split-backend journal verbs (decision 0126 ruling 4, Slice 2):
//! `journal reindex` rebuilds a disposable SQLite projection database
//! (`index.db`) from an [`FsEventLog`] runs directory, and `journal migrate`
//! migrates all runs from a monolithic `forge.db` into a split layout with
//! canonical NDJSON parity verification.

use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::canonical::ZERO_HASH;
use brokkr_store::ports::{
    EventLog, FsEventLog, ProjectionStore, SqlitePorts, SqliteProjectionStore,
};
use brokkr_store::{Store, StoreError};
use serde::Serialize;
use serde_json::Value;

use crate::cli_args::JournalArgs;
use crate::{journal_of, open_journal, Access, Exit};

/// Split-backend journal subcommands dispatched through `GroupedCmd::Split`.
pub(crate) enum SplitCmd {
    Reindex(ReindexArgs),
    Migrate(MigrateArgs),
}

/// Typed failures for `brokkr journal reindex` and `brokkr journal migrate`
/// (decision 0071 ruling 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParityKind {
    Manifest,
    EventStream,
    CanonicalNdjson,
}

impl ParityKind {
    fn as_str(self) -> &'static str {
        match self {
            ParityKind::Manifest => "manifest",
            ParityKind::EventStream => "event stream",
            ParityKind::CanonicalNdjson => "canonical NDJSON",
        }
    }
}

#[derive(Debug)]
pub(crate) enum SplitError {
    RunsDirMissing(PathBuf),
    SymlinkRefused(PathBuf),
    RunsPathNotDirectory(PathBuf),
    IndexPathIsDirectory(PathBuf),
    InvalidPath(PathBuf),
    TargetNotDirectory(PathBuf),
    TargetNotEmpty(PathBuf),
    TargetHasNoParent(PathBuf),
    ParityMismatch {
        kind: ParityKind,
        run_id: String,
    },
    ProjectionMismatch {
        expected_runs: usize,
        expected_events: u64,
        indexed_runs: usize,
        indexed_events: u64,
    },
    Store(StoreError),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitError::RunsDirMissing(p) => {
                write!(f, "runs directory '{}' does not exist", p.display())
            }
            SplitError::SymlinkRefused(p) => write!(f, "refusing symlink at '{}'", p.display()),
            SplitError::RunsPathNotDirectory(p) => {
                write!(f, "runs path '{}' is not a directory", p.display())
            }
            SplitError::IndexPathIsDirectory(p) => {
                write!(f, "index path '{}' is a directory", p.display())
            }
            SplitError::InvalidPath(p) => write!(f, "invalid path '{}'", p.display()),
            SplitError::TargetNotDirectory(p) => {
                write!(f, "migration destination '{}' is not a directory", p.display())
            }
            SplitError::TargetNotEmpty(p) => {
                write!(f, "migration destination '{}' is not empty", p.display())
            }
            SplitError::TargetHasNoParent(p) => {
                write!(
                    f,
                    "migration destination '{}' has no parent directory",
                    p.display()
                )
            }
            SplitError::ParityMismatch { kind, run_id } => {
                write!(f, "{} parity mismatch for run '{run_id}'", kind.as_str())
            }
            SplitError::ProjectionMismatch {
                expected_runs,
                expected_events,
                indexed_runs,
                indexed_events,
            } => write!(
                f,
                "migration projection mismatch: expected {expected_runs} runs / {expected_events} events, indexed {indexed_runs} / {indexed_events}"
            ),
            SplitError::Store(err) => err.fmt(f),
            SplitError::Io(err) => err.fmt(f),
            SplitError::Json(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for SplitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SplitError::Store(err) => Some(err),
            SplitError::Io(err) => Some(err),
            SplitError::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<StoreError> for SplitError {
    fn from(err: StoreError) -> Self {
        SplitError::Store(err)
    }
}

impl From<std::io::Error> for SplitError {
    fn from(err: std::io::Error) -> Self {
        SplitError::Io(err)
    }
}

impl From<serde_json::Error> for SplitError {
    fn from(err: serde_json::Error) -> Self {
        SplitError::Json(err)
    }
}

/// `brokkr journal reindex`: rebuild a disposable SQLite projection index
/// (`index.db`) from a filesystem event-log runs directory.
#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct ReindexArgs {
    /// The filesystem event-log runs directory to read from (default:
    /// `runs` beside the workspace journal).
    #[arg(long)]
    pub(crate) runs: Option<PathBuf>,
    /// The disposable SQLite projection index database to rebuild (default:
    /// `index.db` beside the workspace journal).
    #[arg(long)]
    pub(crate) index: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}

/// `brokkr journal migrate`: migrate all runs from a monolithic SQLite journal
/// (`forge.db`) into a split layout (`<to>/runs` + rebuilt `<to>/index.db`),
/// verifying canonical NDJSON parity and failing atomically without touching
/// the source database.
#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct MigrateArgs {
    /// The target directory for the split layout (`<to>/runs` and
    /// `<to>/index.db`); it must be absent or empty, and nothing in it is
    /// overwritten.
    #[arg(long)]
    pub(crate) to: PathBuf,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}

#[derive(Serialize)]
struct ReindexReport {
    runs_dir: String,
    index: String,
    runs: u64,
    events: u64,
}

#[derive(Serialize)]
struct MigrateReport {
    from: String,
    to: String,
    runs_dir: String,
    index: String,
    runs: u64,
    events: u64,
}

struct StagedDb {
    path: PathBuf,
    keep: bool,
}

impl StagedDb {
    fn new(parent: &Path) -> Result<Self, SplitError> {
        let file = tempfile::Builder::new()
            .prefix(".brokkr-index-rebuild-")
            .suffix(".db")
            .tempfile_in(parent)?;
        let path = file.into_temp_path().keep().map_err(|e| e.error)?;
        Ok(Self { path, keep: false })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.keep = true;
    }
}

impl Drop for StagedDb {
    fn drop(&mut self) {
        if !self.keep {
            remove_if_exists(&self.path);
            remove_sidecars(&self.path);
        }
    }
}

struct PreparedTarget {
    normalized: PathBuf,
    parent: PathBuf,
    existed_empty: bool,
}

/// Dispatch `brokkr journal reindex` or `brokkr journal migrate`.
pub(crate) fn run(workspace: &Path, command: SplitCmd) -> Result<ExitCode> {
    match command {
        SplitCmd::Reindex(args) => reindex(workspace, args),
        SplitCmd::Migrate(args) => migrate(workspace, args),
    }
}

/// `brokkr journal reindex [--runs <dir>] [--index <path>]`.
pub(crate) fn reindex(
    workspace: &Path,
    ReindexArgs {
        runs,
        index,
        journal,
    }: ReindexArgs,
) -> Result<ExitCode> {
    let (runs_dir, index_path) = resolve_split_paths(workspace, runs, index, journal)?;
    let (runs, events) = reindex_paths(&runs_dir, &index_path)?;

    let report = ReindexReport {
        runs_dir: runs_dir.display().to_string(),
        index: index_path.display().to_string(),
        runs,
        events,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    eprintln!(
        "reindexed {} runs ({} events) from {} into {}",
        runs,
        events,
        runs_dir.display(),
        index_path.display()
    );
    Ok(Exit::Completed.into())
}

fn reindex_paths(runs_dir: &Path, index_path: &Path) -> Result<(u64, u64), SplitError> {
    ensure_existing_runs_dir(runs_dir)?;
    let parent = prepare_index_target(index_path)?;
    let log = FsEventLog::open(runs_dir)?;
    let mut staged = StagedDb::new(&parent)?;
    let projections = SqliteProjectionStore::open(staged.path())?;
    let events = projections.rebuild_from(&log)?;
    let runs = projections.list_runs()?.len() as u64;
    drop(projections);
    remove_sidecars(staged.path());
    remove_sidecars(index_path);
    fs::rename(staged.path(), index_path)?;
    fsync_dir(&parent)?;
    staged.disarm();
    Ok((runs, events))
}

/// `brokkr journal migrate --to <dir>`.
pub(crate) fn migrate(
    workspace: &Path,
    MigrateArgs { to, journal }: MigrateArgs,
) -> Result<ExitCode> {
    let from = journal_of(workspace, journal.realms, journal.db)?;
    // Open read-only so migration can never mutate or corrupt the source DB.
    let store = open_journal(&from, Access::Read)?;
    let (runs, events) = migrate_store(&store, &to)?;

    let runs_dir = to.join("runs");
    let index_path = to.join("index.db");
    let report = MigrateReport {
        from: from.display().to_string(),
        to: to.display().to_string(),
        runs_dir: runs_dir.display().to_string(),
        index: index_path.display().to_string(),
        runs,
        events,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    eprintln!(
        "migrated {} runs ({} events) from {} to {}",
        runs,
        events,
        from.display(),
        to.display()
    );
    Ok(Exit::Completed.into())
}

fn migrate_store(store: &Store, to: &Path) -> Result<(u64, u64), SplitError> {
    let target = prepare_migration_target(to)?;
    let staging = tempfile::Builder::new()
        .prefix(".brokkr-migrate-")
        .tempdir_in(&target.parent)?;
    let (runs, events) = stage_migration(store, staging.path())?;
    publish_staged_migration(staging, &target)?;
    Ok((runs, events))
}

fn publish_staged_migration(
    staging: tempfile::TempDir,
    target: &PreparedTarget,
) -> Result<(), SplitError> {
    if target.existed_empty {
        let src_runs = staging.path().join("runs");
        let dst_runs = target.normalized.join("runs");
        fs::rename(src_runs, dst_runs)?;
        let src_idx = staging.path().join("index.db");
        let dst_idx = target.normalized.join("index.db");
        fs::rename(src_idx, dst_idx)?;
        fsync_dir(&target.normalized)?;
        fsync_dir(&target.parent)?;
    } else {
        fs::rename(staging.path(), &target.normalized)?;
        let _ = staging.keep();
        fsync_dir(&target.parent)?;
    }
    Ok(())
}

fn resolve_split_paths(
    workspace: &Path,
    runs: Option<PathBuf>,
    index: Option<PathBuf>,
    journal: JournalArgs,
) -> Result<(PathBuf, PathBuf)> {
    match (runs, index) {
        (Some(runs), Some(index)) => Ok((runs, index)),
        (runs, index) => {
            let db = journal_of(workspace, journal.realms, journal.db)?;
            let state_dir = db.parent().unwrap_or(workspace);
            Ok((
                runs.unwrap_or_else(|| state_dir.join("runs")),
                index.unwrap_or_else(|| state_dir.join("index.db")),
            ))
        }
    }
}

fn ensure_existing_runs_dir(path: &Path) -> Result<(), SplitError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(SplitError::RunsDirMissing(path.to_path_buf()));
        }
        Err(err) => return Err(err.into()),
    };
    if meta.file_type().is_symlink() {
        return Err(SplitError::SymlinkRefused(path.to_path_buf()));
    }
    if !meta.is_dir() {
        return Err(SplitError::RunsPathNotDirectory(path.to_path_buf()));
    }
    Ok(())
}

fn refuse_symlink(path: &Path) -> Result<Option<fs::Metadata>, SplitError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(SplitError::SymlinkRefused(path.to_path_buf()))
        }
        Ok(meta) => Ok(Some(meta)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn refuse_symlink_ancestors(path: &Path) -> Result<(), SplitError> {
    let mut probe = PathBuf::new();
    for comp in path.components() {
        probe.push(comp.as_os_str());
        if matches!(comp, Component::RootDir | Component::Prefix(_)) {
            continue;
        }
        #[cfg(target_os = "macos")]
        if matches!(probe.to_str(), Some("/tmp" | "/var" | "/etc")) {
            continue;
        }
        if refuse_symlink(&probe)?.is_none() {
            break;
        }
    }
    Ok(())
}

fn is_disposable_corrupt(err: &StoreError) -> bool {
    matches!(err, StoreError::Damaged { .. })
}

fn prepare_index_target(index_path: &Path) -> Result<PathBuf, SplitError> {
    if index_path.as_os_str().is_empty() {
        return Err(SplitError::InvalidPath(index_path.to_path_buf()));
    }
    let abs = std::path::absolute(index_path)?;
    refuse_symlink_ancestors(&abs)?;
    let _ = refuse_symlink(&sidecar_path(&abs, "-wal"))?;
    let _ = refuse_symlink(&sidecar_path(&abs, "-shm"))?;
    if let Some(meta) = refuse_symlink(&abs)? {
        if meta.is_dir() {
            return Err(SplitError::IndexPathIsDirectory(index_path.to_path_buf()));
        }
        if let Err(err) = SqliteProjectionStore::open(&abs) {
            if !is_disposable_corrupt(&err) {
                return Err(err.into());
            }
        }
    }
    let parent = abs
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| SplitError::InvalidPath(index_path.to_path_buf()))?
        .to_path_buf();
    fs::create_dir_all(&parent)?;
    Ok(parent)
}

fn lexically_normalize(abs: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in abs.components() {
        if comp == Component::ParentDir {
            out.pop();
        } else {
            out.push(comp.as_os_str());
        }
    }
    out
}

fn prepare_migration_target(to: &Path) -> Result<PreparedTarget, SplitError> {
    if to.as_os_str().is_empty() {
        return Err(SplitError::InvalidPath(to.to_path_buf()));
    }
    let abs = std::path::absolute(to)?;
    refuse_symlink_ancestors(&abs)?;
    let normalized = lexically_normalize(&abs);
    refuse_symlink_ancestors(&normalized)?;
    let parent = normalized
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| SplitError::TargetHasNoParent(to.to_path_buf()))?
        .to_path_buf();
    let existed_empty = match refuse_symlink(&normalized)? {
        Some(meta) if !meta.is_dir() => {
            return Err(SplitError::TargetNotDirectory(to.to_path_buf()));
        }
        Some(_) => {
            if fs::read_dir(&normalized)?.next().is_some() {
                return Err(SplitError::TargetNotEmpty(to.to_path_buf()));
            }
            true
        }
        None => false,
    };
    fs::create_dir_all(&parent)?;
    Ok(PreparedTarget {
        normalized,
        parent,
        existed_empty,
    })
}

fn preserve_run_metadata(manifest: &Value, feature: &str, created_at: &str) -> Value {
    let mut out = manifest.clone();
    if let Some(obj) = out.as_object_mut() {
        for (key, value) in [("feature", feature), ("created_at", created_at)] {
            let missing = obj
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty);
            if !value.is_empty() && missing {
                obj.insert(key.to_string(), Value::String(value.to_string()));
            }
        }
    }
    out
}

fn migrate_one_run(
    store: &Store,
    source: &SqlitePorts<'_>,
    fs_log: &FsEventLog,
    runs_dir: &Path,
    (run_id, feature, created_at): &(String, String, String),
) -> Result<u64, SplitError> {
    let raw_manifest = source.manifest(run_id)?;
    let manifest = preserve_run_metadata(&raw_manifest, feature, created_at);
    let events = source.read_stream(run_id, 1)?;
    let source_ndjson = store.export_ndjson(run_id)?;
    fs_log.create_run(run_id, &manifest)?;
    if !events.is_empty() {
        fs_log.append_envelopes(run_id, (0, ZERO_HASH), &events)?;
    }
    let expected = (&manifest, events.as_slice(), source_ndjson.as_str());
    verify_run_parity(fs_log, runs_dir, run_id, expected)?;
    Ok(events.len() as u64)
}

fn stage_migration(store: &Store, staging: &Path) -> Result<(u64, u64), SplitError> {
    let source = SqlitePorts::new(store);
    let runs_dir = staging.join("runs");
    let fs_log = FsEventLog::open(&runs_dir)?;
    let source_runs = store.list_runs()?;
    let mut total_events = 0u64;
    for entry in &source_runs {
        total_events += migrate_one_run(store, &source, &fs_log, &runs_dir, entry)?;
    }

    let index_path = staging.join("index.db");
    let index = SqliteProjectionStore::open(&index_path)?;
    let indexed_events = index.rebuild_from(&fs_log)?;
    let indexed_runs = index.list_runs()?.len();
    let counts = (
        source_runs.len(),
        total_events,
        indexed_runs,
        indexed_events,
    );
    verify_projection_counts(counts)?;
    drop(index);
    remove_sidecars(&index_path);
    Ok((source_runs.len() as u64, total_events))
}

fn verify_projection_counts(
    (expected_runs, expected_events, indexed_runs, indexed_events): (usize, u64, usize, u64),
) -> Result<(), SplitError> {
    if indexed_events != expected_events || indexed_runs != expected_runs {
        return Err(SplitError::ProjectionMismatch {
            expected_runs,
            expected_events,
            indexed_runs,
            indexed_events,
        });
    }
    Ok(())
}

fn verify_run_parity(
    fs_log: &FsEventLog,
    runs_dir: &Path,
    run_id: &str,
    (manifest, events, source_ndjson): (&Value, &[brokkr_core::EventEnvelope], &str),
) -> Result<(), SplitError> {
    if fs_log.manifest(run_id)? != *manifest {
        return Err(SplitError::ParityMismatch {
            kind: ParityKind::Manifest,
            run_id: run_id.to_string(),
        });
    }
    let migrated_events = fs_log.read_stream(run_id, 1)?;
    if serde_json::to_vec(&migrated_events)? != serde_json::to_vec(events)? {
        return Err(SplitError::ParityMismatch {
            kind: ParityKind::EventStream,
            run_id: run_id.to_string(),
        });
    }
    let migrated_ndjson = fs::read_to_string(runs_dir.join(run_id).join("events.ndjson"))?;
    if migrated_ndjson != source_ndjson {
        return Err(SplitError::ParityMismatch {
            kind: ParityKind::CanonicalNdjson,
            run_id: run_id.to_string(),
        });
    }
    Ok(())
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

fn remove_if_exists(path: &Path) {
    let _ = fs::remove_file(path);
}

fn remove_sidecars(db_path: &Path) {
    remove_if_exists(&sidecar_path(db_path, "-wal"));
    remove_if_exists(&sidecar_path(db_path, "-shm"));
}

fn fsync_dir(dir: &Path) -> Result<(), SplitError> {
    File::open(dir)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests;
