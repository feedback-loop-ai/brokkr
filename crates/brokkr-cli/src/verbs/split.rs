//! Local split-backend journal verbs (decision 0126 ruling 4, Slice 2):
//! `journal reindex` rebuilds a disposable SQLite projection database
//! (`index.db`) from an [`FsEventLog`] runs directory, and `journal migrate`
//! migrates all runs from a monolithic `forge.db` into a split layout with
//! canonical NDJSON parity verification.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::canonical::ZERO_HASH;
use brokkr_store::ports::{
    normalize_storage_path, reject_symlink, reject_symlink_path, sqlite_sidecar_path, sync_dir,
    EventLog, FsEventLog, ProjectionStore, RunMetadata, SqlitePorts, SqliteProjectionStore,
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
    RunMetadata,
    Listing,
}

impl ParityKind {
    fn as_str(self) -> &'static str {
        match self {
            ParityKind::Manifest => "manifest",
            ParityKind::EventStream => "event stream",
            ParityKind::CanonicalNdjson => "canonical NDJSON",
            ParityKind::RunMetadata => "run metadata",
            ParityKind::Listing => "run listing",
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
    SidecarRemovalFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    PublishRollbackFailed {
        source: std::io::Error,
        rollback: std::io::Error,
    },
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
            SplitError::SidecarRemovalFailed { path, source } => {
                write!(
                    f,
                    "failed to remove SQLite sidecar '{}': {source}",
                    path.display()
                )
            }
            SplitError::PublishRollbackFailed { source, rollback } => {
                write!(
                    f,
                    "migration publish failed ({source}) and rollback failed ({rollback})"
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
            SplitError::SidecarRemovalFailed { source, .. }
            | SplitError::PublishRollbackFailed { source, .. } => Some(source),
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

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ReindexReport {
    runs_dir: String,
    index: String,
    runs: u64,
    events: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
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
            let _ = fs::remove_file(&self.path);
            let _ = remove_sidecars_checked(&self.path);
        }
    }
}

struct PreparedTarget {
    normalized: PathBuf,
    parent: PathBuf,
    in_place: bool,
}

struct PreparedIndex {
    normalized: PathBuf,
    parent: PathBuf,
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
    let report = reindex_report(&runs_dir, &index_path)?;

    println!("{}", serde_json::to_string_pretty(&report)?);
    eprintln!(
        "reindexed {} runs ({} events) from {} into {}",
        report.runs, report.events, report.runs_dir, report.index
    );
    Ok(Exit::Completed.into())
}

fn reindex_report(runs_dir: &Path, index_path: &Path) -> Result<ReindexReport, SplitError> {
    let normalized_runs = ensure_existing_runs_dir(runs_dir)?;
    let target = prepare_index_target(index_path)?;
    let log = FsEventLog::open(&normalized_runs)?;
    let mut staged = StagedDb::new(&target.parent)?;
    let projections = SqliteProjectionStore::open(staged.path())?;
    let events = projections.rebuild_from(&log)?;
    let runs = projections.list_runs()?.len() as u64;
    drop(projections);
    remove_sidecars_checked(staged.path())?;
    publish_staged_index_with(&mut staged, &target, |src, dst| fs::rename(src, dst))?;
    Ok(ReindexReport {
        runs_dir: normalized_runs.display().to_string(),
        index: target.normalized.display().to_string(),
        runs,
        events,
    })
}

fn publish_staged_index_with(
    staged: &mut StagedDb,
    target: &PreparedIndex,
    rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), SplitError> {
    rename(staged.path(), &target.normalized)?;
    staged.disarm();
    remove_sidecars_checked(&target.normalized)?;
    fsync_dir(&target.parent)?;
    Ok(())
}

#[cfg(test)]
fn reindex_paths(runs_dir: &Path, index_path: &Path) -> Result<(u64, u64), SplitError> {
    let report = reindex_report(runs_dir, index_path)?;
    Ok((report.runs, report.events))
}

/// `brokkr journal migrate --to <dir>`.
pub(crate) fn migrate(
    workspace: &Path,
    MigrateArgs { to, journal }: MigrateArgs,
) -> Result<ExitCode> {
    let from = journal_of(workspace, journal.realms, journal.db)?;
    // Open read-only so migration can never mutate or corrupt the source DB.
    let store = open_journal(&from, Access::Read)?;
    let report = migrate_report(&store, &from, &to)?;

    println!("{}", serde_json::to_string_pretty(&report)?);
    eprintln!(
        "migrated {} runs ({} events) from {} to {}",
        report.runs, report.events, report.from, report.to
    );
    Ok(Exit::Completed.into())
}

fn migrate_report(store: &Store, from: &Path, to: &Path) -> Result<MigrateReport, SplitError> {
    let (normalized_to, runs, events) = migrate_store(store, to)?;
    let runs_dir = normalized_to.join("runs");
    let index_path = normalized_to.join("index.db");
    Ok(MigrateReport {
        from: from.display().to_string(),
        to: normalized_to.display().to_string(),
        runs_dir: runs_dir.display().to_string(),
        index: index_path.display().to_string(),
        runs,
        events,
    })
}

fn migrate_store(store: &Store, to: &Path) -> Result<(PathBuf, u64, u64), SplitError> {
    let target = prepare_migration_target(to)?;
    let staging = tempfile::Builder::new()
        .prefix(".brokkr-migrate-")
        .tempdir_in(&target.parent)?;
    let (runs, events) = stage_migration(store, staging.path())?;
    publish_staged_migration(staging, &target)?;
    Ok((target.normalized, runs, events))
}

fn publish_staged_migration(
    staging: tempfile::TempDir,
    target: &PreparedTarget,
) -> Result<(), SplitError> {
    publish_staged_migration_with(staging, target, |src, dst| fs::rename(src, dst))
}

fn publish_staged_migration_with(
    staging: tempfile::TempDir,
    target: &PreparedTarget,
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), SplitError> {
    if !target.in_place {
        rename(staging.path(), &target.normalized)?;
        let _ = staging.keep();
        fsync_dir(&target.parent)?;
        return Ok(());
    }
    let src_runs = staging.path().join("runs");
    let dst_runs = target.normalized.join("runs");
    rename(&src_runs, &dst_runs)?;
    let src_idx = staging.path().join("index.db");
    let dst_idx = target.normalized.join("index.db");
    if let Err(err) = rename(&src_idx, &dst_idx) {
        if let Err(rollback) = rename(&dst_runs, &src_runs) {
            return Err(SplitError::PublishRollbackFailed {
                source: err,
                rollback,
            });
        }
        return Err(err.into());
    }
    fsync_dir(&target.normalized)?;
    fsync_dir(&target.parent)?;
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

fn map_path_err(err: StoreError) -> SplitError {
    match err {
        StoreError::SymlinkRefused(path) => SplitError::SymlinkRefused(path),
        StoreError::InvalidPath(path) => SplitError::InvalidPath(path),
        StoreError::Io(err) => SplitError::Io(err),
        other => SplitError::Store(other),
    }
}

fn ensure_existing_runs_dir(path: &Path) -> Result<PathBuf, SplitError> {
    let normalized = normalize_storage_path(path).map_err(map_path_err)?;
    let Some(meta) = reject_symlink_path(&normalized).map_err(map_path_err)? else {
        return Err(SplitError::RunsDirMissing(normalized));
    };
    if !meta.is_dir() {
        return Err(SplitError::RunsPathNotDirectory(normalized));
    }
    Ok(normalized)
}

fn is_disposable_corrupt(err: &StoreError) -> bool {
    matches!(err, StoreError::Damaged { .. })
}

fn check_sidecar_removable(db_path: &Path, suffix: &str) -> Result<(), SplitError> {
    let sidecar = sqlite_sidecar_path(db_path, suffix);
    if let Some(meta) = reject_symlink(&sidecar).map_err(map_path_err)? {
        if meta.is_dir() {
            return Err(SplitError::SidecarRemovalFailed {
                path: sidecar,
                source: std::io::Error::new(
                    ErrorKind::IsADirectory,
                    "SQLite sidecar path is a directory",
                ),
            });
        }
    }
    Ok(())
}

fn prepare_index_target(index_path: &Path) -> Result<PreparedIndex, SplitError> {
    let normalized = normalize_storage_path(index_path).map_err(map_path_err)?;
    let meta = reject_symlink_path(&normalized).map_err(map_path_err)?;
    check_sidecar_removable(&normalized, "-wal")?;
    check_sidecar_removable(&normalized, "-shm")?;
    if let Some(meta) = meta {
        if meta.is_dir() {
            return Err(SplitError::IndexPathIsDirectory(normalized));
        }
        if let Err(err) = SqliteProjectionStore::inspect_read_only(&normalized) {
            if !is_disposable_corrupt(&err) {
                return Err(err.into());
            }
        }
    }
    let parent = normalized
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| SplitError::InvalidPath(index_path.to_path_buf()))?
        .to_path_buf();
    fs::create_dir_all(&parent)?;
    Ok(PreparedIndex { normalized, parent })
}

fn prepare_migration_target(to: &Path) -> Result<PreparedTarget, SplitError> {
    let normalized = normalize_storage_path(to).map_err(map_path_err)?;
    let meta = reject_symlink_path(&normalized).map_err(map_path_err)?;
    let parent = normalized
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| SplitError::TargetHasNoParent(to.to_path_buf()))?
        .to_path_buf();
    let existed_empty = match meta {
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
    let in_place = existed_empty
        && std::env::current_dir()
            .ok()
            .is_some_and(|cwd| cwd == normalized);
    fs::create_dir_all(&parent)?;
    Ok(PreparedTarget {
        normalized,
        parent,
        in_place,
    })
}

fn migrate_one_run(
    store: &Store,
    source: &SqlitePorts<'_>,
    fs_log: &FsEventLog,
    runs_dir: &Path,
    list_order: u64,
    (run_id, feature, created_at): &(String, String, String),
) -> Result<u64, SplitError> {
    // Keep manifest.json strictly immutable and byte-identical to source.manifest(run_id).
    let manifest = source.manifest(run_id)?;
    let metadata = RunMetadata {
        feature: feature.clone(),
        created_at: created_at.clone(),
        list_order: Some(list_order),
    };
    let events = source.read_stream(run_id, 1)?;
    let source_ndjson = store.export_ndjson(run_id)?;
    fs_log.create_run_with_metadata(run_id, &manifest, &metadata)?;
    if !events.is_empty() {
        fs_log.append_envelopes(run_id, (0, ZERO_HASH), &events)?;
    }
    let expected = (
        &manifest,
        &metadata,
        events.as_slice(),
        source_ndjson.as_str(),
    );
    verify_run_parity(fs_log, runs_dir, run_id, expected)?;
    Ok(events.len() as u64)
}

fn stage_migration(store: &Store, staging: &Path) -> Result<(u64, u64), SplitError> {
    stage_migration_with(store, staging, || {})
}

fn stage_migration_with(
    store: &Store,
    staging: &Path,
    mut after_list: impl FnMut(),
) -> Result<(u64, u64), SplitError> {
    let source = SqlitePorts::new(store);
    let runs_dir = staging.join("runs");
    let fs_log = FsEventLog::open(&runs_dir)?;
    let (source_runs, total_events) = store.read_snapshot(|| {
        let source_runs = store.list_runs()?;
        after_list();
        let mut total_events = 0u64;
        for (order, entry) in source_runs.iter().enumerate() {
            total_events +=
                migrate_one_run(store, &source, &fs_log, &runs_dir, order as u64, entry)?;
        }
        Ok::<_, SplitError>((source_runs, total_events))
    })?;

    let index_path = staging.join("index.db");
    let index = SqliteProjectionStore::open(&index_path)?;
    let indexed_events = index.rebuild_from(&fs_log)?;
    let indexed_list = index.list_runs()?;
    let counts = (
        source_runs.len(),
        total_events,
        indexed_list.len(),
        indexed_events,
    );
    verify_projection_counts(counts)?;
    verify_listing_parity(&source_runs, &indexed_list)?;
    drop(index);
    remove_sidecars_checked(&index_path)?;
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

fn verify_listing_parity(
    expected: &[(String, String, String)],
    actual: &[(String, String, String)],
) -> Result<(), SplitError> {
    for (exp, act) in expected.iter().zip(actual.iter()) {
        if exp != act {
            return Err(SplitError::ParityMismatch {
                kind: ParityKind::Listing,
                run_id: exp.0.clone(),
            });
        }
    }
    Ok(())
}

fn verify_run_parity(
    fs_log: &FsEventLog,
    runs_dir: &Path,
    run_id: &str,
    (manifest, metadata, events, source_ndjson): (
        &Value,
        &RunMetadata,
        &[brokkr_core::EventEnvelope],
        &str,
    ),
) -> Result<(), SplitError> {
    let manifest_bytes = fs::read(runs_dir.join(run_id).join("manifest.json"))?;
    if manifest_bytes != serde_json::to_vec(manifest)? || fs_log.manifest(run_id)? != *manifest {
        return Err(SplitError::ParityMismatch {
            kind: ParityKind::Manifest,
            run_id: run_id.to_string(),
        });
    }
    if fs_log.run_metadata(run_id)?.as_ref() != Some(metadata) {
        return Err(SplitError::ParityMismatch {
            kind: ParityKind::RunMetadata,
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

fn remove_sidecar_if_exists(path: &Path) -> Result<(), SplitError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(SplitError::SidecarRemovalFailed {
            path: path.to_path_buf(),
            source: err,
        }),
    }
}

fn remove_sidecars_checked(db_path: &Path) -> Result<(), SplitError> {
    remove_sidecar_if_exists(&sqlite_sidecar_path(db_path, "-wal"))?;
    remove_sidecar_if_exists(&sqlite_sidecar_path(db_path, "-shm"))?;
    Ok(())
}

fn fsync_dir(dir: &Path) -> Result<(), SplitError> {
    sync_dir(dir)?;
    Ok(())
}

#[cfg(test)]
mod tests;
