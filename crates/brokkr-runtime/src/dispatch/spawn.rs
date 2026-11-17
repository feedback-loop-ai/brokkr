//! The driver a dispatcher spawns, in safe code (decision 0110 rulings 3
//! and 4): the binary it runs, the log it writes and what it checks before
//! it resumes. The process itself is brokkr-cli's to start, since leading
//! a session of its own needs a call between fork and exec, which this
//! crate, forbidding unsafe code, does not make.
//!
//! The dispatcher spawns only the brokkr it was started from: it records
//! that file's path, device and inode at start, and before each claim and
//! each `spawn` row checks that the path still names the same file. A
//! replaced file ends the dispatcher, which claims nothing after (ruling
//! 4).
//!
//! A driver's stdout and stderr are appended to `<journal>.runs/<run>.log`,
//! beside the run's lock (decision 0108 ruling 1), opened as that lock is:
//! this user's own regular file of one link, made mode 0600 where it is
//! missing, never through a link, closed on exec, and never removed.
//!
//! A driver resumes only the run its entry is linked to under the claim it
//! was handed, once that entry's `spawn` row has landed, and refuses
//! otherwise having written nothing. It rebuilds the request from the
//! entry's payload exactly as the start did.

use std::ffi::OsString;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_store::{EntryId, Store, StoreError};
use rustix::fs::{openat, Mode, OFlags};
use thiserror::Error;

use super::pass::Ending;
use crate::argv::Executable;
use crate::launch::{LaunchError, LaunchRequest, QueuedLaunch};
use crate::{lock_file, run_lock};

/// What a driver's log is named, after the run's own id.
const LOG: &str = ".log";

/// Why a dispatcher could not name its binary or open a driver's log, or a
/// driver refused to resume.
#[derive(Debug, Error)]
pub enum DriverError {
    /// The platform could not say which file this process was started
    /// from, so no driver of it can be spawned.
    #[error(
        "dispatch: this process cannot name the brokkr it was started from, so it spawns no \
         driver (decision 0110)"
    )]
    Unnamed,
    /// The file this process was started from cannot be read now.
    #[error(
        "dispatch: the brokkr at {} cannot be read, so this dispatcher spawns no driver \
         (decision 0110)",
        .0.display()
    )]
    Unread(PathBuf),
    /// The log, or the directory it stands in, is not this user's own.
    #[error(
        "dispatch: the driver log {} is not this user's own single file in a directory of mode \
         0700, so the run's driver was not started (decision 0110)",
        .0.display()
    )]
    Log(PathBuf),
    /// The entry is not linked under the claim the driver was handed, or
    /// its `spawn` row has not landed: nothing was written.
    #[error(
        "dispatch drive: queue entry {entry} is not linked under claim {claim} with a `spawn` \
         row, so this driver resumes nothing and wrote nothing (decision 0110)"
    )]
    Unspawned { entry: EntryId, claim: i64 },
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Launch(#[from] LaunchError),
}

/// The brokkr a dispatcher was started from (decision 0110 ruling 4): its
/// path, and the device and inode of the file that path named at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary {
    path: PathBuf,
    file: (u64, u64),
}

impl Binary {
    /// Record the file `executable` names, as the dispatcher starts.
    pub fn started(executable: &Executable) -> Result<Binary, DriverError> {
        let path = executable.path().ok_or(DriverError::Unnamed)?.to_path_buf();
        match identity(&path) {
            Some(file) => Ok(Binary { path, file }),
            None => Err(DriverError::Unread(path)),
        }
    }

    /// The path the dispatcher was started from, the one each driver runs.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ruling 4's check, before each claim and each `spawn` row: the path
    /// still names the file recorded at start, or the dispatcher ends.
    pub fn stands(&self) -> Result<(), Ending> {
        match identity(&self.path) == Some(self.file) {
            true => Ok(()),
            false => Err(Ending::Replaced(self.path.clone())),
        }
    }
}

/// The device and inode of the file `path` names now, through any link.
fn identity(path: &Path) -> Option<(u64, u64)> {
    let read = std::fs::metadata(path).ok()?;
    Some((read.dev(), read.ino()))
}

/// Run `run`'s driver log, `<journal>.runs/<run>.log`, opened to append:
/// made mode 0600 where it is missing, never through a link, closed on
/// exec, and refused unless it is this user's own regular file of one link
/// in the run locks' own directory.
pub fn driver_log(journal: &Path, run: &str) -> Result<File, DriverError> {
    let name = format!("{run}{LOG}");
    let at = run_lock::directory_path(journal).join(&name);
    if !run_lock::safe(run) {
        return Err(DriverError::Log(at));
    }
    let Ok(dir) = run_lock::directory(journal) else {
        return Err(DriverError::Log(at));
    };
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::APPEND | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let Ok(log) = openat(&dir, name.as_str(), flags, Mode::RUSR | Mode::WUSR) else {
        return Err(DriverError::Log(at));
    };
    match lock_file::judged(&log, lock_file::user()) {
        Ok(()) => Ok(File::from(log)),
        Err(_) => Err(DriverError::Log(at)),
    }
}

/// The workspace `entry`'s payload names, where its driver is started.
pub fn workspace(journal: &Path, entry: EntryId) -> Result<PathBuf, DriverError> {
    let store = Store::open_read_only(journal)?;
    Ok(QueuedLaunch::decode(&store.queue_entry(entry)?.payload)?.workspace)
}

/// The request a driver of `entry`'s claim `seq` resumes, and its run
/// (decision 0110 ruling 3): refused, having written nothing, unless the
/// entry is linked under that claim and its `spawn` row has landed. The
/// request is rebuilt from the entry's payload as the start rebuilt it,
/// written to `journal` and looking for a boundary's tool on `host_path`.
pub fn driven(
    journal: &Path,
    entry: EntryId,
    seq: i64,
    host_path: OsString,
) -> Result<(LaunchRequest, String), DriverError> {
    let store = Store::open_read_only(journal)?;
    let unspawned = DriverError::Unspawned { entry, claim: seq };
    let run = store.queue_spawned(entry, seq)?.ok_or(unspawned)?;
    let queued = QueuedLaunch::decode(&store.queue_entry(entry)?.payload)?;
    let (request, _) = queued.rebuild(journal.to_path_buf(), host_path)?;
    Ok((request, run))
}

#[cfg(test)]
mod tests;
