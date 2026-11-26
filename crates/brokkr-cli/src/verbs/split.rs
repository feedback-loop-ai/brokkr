//! Local split-backend journal verbs (decision 0126 ruling 4, Slice 2):
//! `journal reindex` rebuilds a disposable SQLite projection database
//! (`index.db`) from an [`FsEventLog`] runs directory, and `journal migrate`
//! migrates all runs from a monolithic `forge.db` into a split layout with
//! canonical NDJSON parity verification.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Result};
use brokkr_core::canonical::ZERO_HASH;
use brokkr_store::ports::{
    EventLog, FsEventLog, ProjectionStore, SqlitePorts, SqliteProjectionStore,
};
use brokkr_store::Store;
use serde::Serialize;

use crate::cli_args::JournalArgs;
use crate::{journal_of, open_journal, Access, Exit};

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
    ensure_existing_runs_dir(&runs_dir)?;
    refuse_symlink(&index_path)?;

    let log = FsEventLog::open(&runs_dir)?;
    let projections = SqliteProjectionStore::open(&index_path)?;
    let events = projections.rebuild_from(&log)?;
    let runs = projections.list_runs()?.len() as u64;

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

/// `brokkr journal migrate --to <dir>`.
pub(crate) fn migrate(
    workspace: &Path,
    MigrateArgs { to, journal }: MigrateArgs,
) -> Result<ExitCode> {
    let from = journal_of(workspace, journal.realms, journal.db)?;
    // Open read-only so migration can never mutate or corrupt the source DB.
    let store = open_journal(&from, Access::Read)?;
    let parent = prepare_migration_target(&to)?;
    let staging = tempfile::Builder::new()
        .prefix(".brokkr-migrate-")
        .tempdir_in(&parent)?;
    let (runs, events) = stage_migration(&store, staging.path())?;
    fs::rename(staging.path(), &to)?;
    let _ = staging.keep();

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

fn ensure_existing_runs_dir(path: &Path) -> Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            bail!("runs directory '{}' does not exist", path.display());
        }
        Err(err) => return Err(err.into()),
    };
    if meta.file_type().is_symlink() {
        bail!("refusing symlink runs directory '{}'", path.display());
    }
    if !meta.is_dir() {
        bail!("runs path '{}' is not a directory", path.display());
    }
    Ok(())
}

fn refuse_symlink(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            bail!("refusing symlink at '{}'", path.display());
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn prepare_migration_target(to: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(to) {
        Ok(meta) if meta.file_type().is_symlink() => {
            bail!("refusing symlink destination '{}'", to.display());
        }
        Ok(meta) if !meta.is_dir() => {
            bail!(
                "migration destination '{}' is not a directory",
                to.display()
            );
        }
        Ok(_) => {
            if fs::read_dir(to)?.next().is_some() {
                bail!("migration destination '{}' is not empty", to.display());
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    let parent = to
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    fs::create_dir_all(&parent)?;
    Ok(parent)
}

fn stage_migration(store: &Store, staging: &Path) -> Result<(u64, u64)> {
    let source = SqlitePorts::new(store);
    let runs_dir = staging.join("runs");
    let fs_log = FsEventLog::open(&runs_dir)?;
    let run_ids = source.list_run_ids()?;
    let mut total_events = 0u64;

    for run_id in &run_ids {
        let manifest = source.manifest(run_id)?;
        let events = source.read_stream(run_id, 1)?;
        let source_ndjson = store.export_ndjson(run_id)?;
        fs_log.create_run(run_id, &manifest)?;
        if !events.is_empty() {
            fs_log.append_envelopes(run_id, (0, ZERO_HASH), &events)?;
        }
        verify_run_parity(
            &fs_log,
            &runs_dir,
            run_id,
            &manifest,
            &events,
            &source_ndjson,
        )?;
        total_events += events.len() as u64;
    }

    let index = SqliteProjectionStore::open(staging.join("index.db"))?;
    let indexed_events = index.rebuild_from(&fs_log)?;
    let indexed_runs = index.list_runs()?.len();
    if indexed_events != total_events || indexed_runs != run_ids.len() {
        bail!(
            "migration projection mismatch: expected {} runs / {} events, indexed {} / {}",
            run_ids.len(),
            total_events,
            indexed_runs,
            indexed_events
        );
    }
    // Drop SQLite connection so WAL checkpoints cleanly before atomic rename.
    drop(index);
    Ok((run_ids.len() as u64, total_events))
}

fn verify_run_parity(
    fs_log: &FsEventLog,
    runs_dir: &Path,
    run_id: &str,
    manifest: &serde_json::Value,
    events: &[brokkr_core::EventEnvelope],
    source_ndjson: &str,
) -> Result<()> {
    if fs_log.manifest(run_id)? != *manifest {
        bail!("manifest parity mismatch for run '{run_id}'");
    }
    let migrated_events = fs_log.read_stream(run_id, 1)?;
    if serde_json::to_vec(&migrated_events)? != serde_json::to_vec(events)? {
        bail!("event stream parity mismatch for run '{run_id}'");
    }
    let migrated_ndjson = fs::read_to_string(runs_dir.join(run_id).join("events.ndjson"))?;
    if migrated_ndjson != source_ndjson {
        bail!("canonical NDJSON parity mismatch for run '{run_id}'");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
