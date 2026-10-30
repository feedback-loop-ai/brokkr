//! The verbs that move a whole journal across this machine's edge:
//! `backup` takes one consistent snapshot with its versioned inventory,
//! and `restore` accepts one into a new location after every check
//! (decision 0100, the journal half of #431).

use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use brokkr_store::backup::{
    backup as take, restore as place, BackupOptions, JOURNAL_BACKUP_FORMAT,
};
use serde::Serialize;

use crate::cli_args::JournalArgs;
use crate::{journal_of, now_rfc3339, open_journal, Access, Exit};

/// `brokkr backup`: one consistent snapshot of the journal and the
/// inventory that makes it a complete recovery unit.
#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct BackupArgs {
    /// The directory the snapshot and its inventory are written into; it
    /// must be absent or empty, and nothing in it is overwritten.
    #[arg(long)]
    pub(crate) to: std::path::PathBuf,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}

/// `brokkr restore`: accept a backup into a journal path that must not
/// exist yet.
#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct RestoreArgs {
    /// The backup directory holding the snapshot and its inventory.
    #[arg(long)]
    pub(crate) from: std::path::PathBuf,
    /// The new journal path. It must not exist; a live journal is never a
    /// restore's destination.
    #[arg(long)]
    pub(crate) to: std::path::PathBuf,
}

/// `brokkr backup --to <dir>`.
pub(crate) fn backup(workspace: &Path, BackupArgs { to, journal }: BackupArgs) -> Result<ExitCode> {
    let db = journal_of(workspace, journal.realms, journal.db)?;
    // The journal is read, never written: a backup must not migrate,
    // repair or otherwise touch the live file it copies.
    let store = open_journal(&db, Access::Read)?;
    let report = take(
        &store,
        BackupOptions {
            to: &to,
            now: &now_rfc3339(),
            projection: brokkr_view::VIEW_VERSION,
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&backed_up(&report))?);
    eprintln!(
        "backed up {} to {} ({} runs, {} events)",
        db.display(),
        report.to.display(),
        report.runs,
        report.events
    );
    Ok(Exit::Completed.into())
}

/// `brokkr restore --from <dir> --to <journal path>`.
pub(crate) fn restore(RestoreArgs { from, to }: RestoreArgs) -> Result<ExitCode> {
    let report = place(&from, &to)?;
    println!("{}", serde_json::to_string_pretty(&restored(&report))?);
    eprintln!(
        "restored {} into {} ({} runs, {} events); nothing was started",
        from.display(),
        report.to.display(),
        report.runs,
        report.events
    );
    Ok(Exit::Completed.into())
}

/// `brokkr backup`'s document: the format written, where, and the
/// boundary the inventory declares.
#[derive(Serialize)]
struct BackedUp {
    format: &'static str,
    to: String,
    snapshot: SnapshotOut,
    boundary: BoundaryOut,
}

#[derive(Serialize)]
struct SnapshotOut {
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
struct BoundaryOut {
    runs: u64,
    events: u64,
}

fn backed_up(backup: &brokkr_store::backup::Backup) -> BackedUp {
    BackedUp {
        format: JOURNAL_BACKUP_FORMAT,
        to: backup.to.display().to_string(),
        snapshot: SnapshotOut {
            bytes: backup.bytes,
            sha256: backup.sha256.clone(),
        },
        boundary: BoundaryOut {
            runs: backup.runs,
            events: backup.events,
        },
    }
}

/// `brokkr restore`'s document: the journal written and what it holds.
#[derive(Serialize)]
struct Restored {
    restored: String,
    runs: u64,
    events: u64,
}

fn restored(restored: &brokkr_store::backup::Restored) -> Restored {
    Restored {
        restored: restored.to.display().to_string(),
        runs: restored.runs,
        events: restored.events,
    }
}

#[cfg(test)]
mod tests;
