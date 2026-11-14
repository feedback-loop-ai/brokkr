//! The run's lock (decision 0108 ruling 1): an exclusive, non-blocking
//! `flock` on `<journal>.runs/<run-id>.lock`, taken before a driver's
//! first append and held until it stops driving. Liveness is the lock,
//! which the kernel lets go when its process dies: there is no lease, no
//! heartbeat and no clock read for it.
//!
//! The directory is this user's own and mode 0700, and the file is this
//! user's own regular file of one link opened through the shared lock
//! file home (decision 0095 ruling 8), never through a link and closed on
//! exec. The holder's facts are recorded beside the lock for the
//! operator's eyes only; a second driver reads them to name the holder,
//! and the lock alone decides whether the run is held.
//!
//! The lock is held from before the driver's first append until its
//! drive returns, so a run view still painting after
//! [`crate::Engine::drive`] returns does not hold the run. Lock files
//! are never removed: one stays per run for the journal's life, and an
//! entry refused before its first append may leave an empty one. A
//! file's presence says nothing; the lock does, so a file removed under a
//! live driver, or a journal reached through another name, is the one
//! place a second driver can share a run, and the existing head fences
//! (decision 0029), not this lock, are what remain.
//!
//! Linux and macOS exclude alike (decision 0063). A filesystem where
//! `flock` does not exclude, such as some network mounts, is outside this
//! guarantee, as decision 0095 already notes for the dispatcher's lock.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::os::fd::{AsFd, OwnedFd};
use std::path::{Component, Path, PathBuf};

use brokkr_store::Store;
use rustix::fs::{fstat, ftruncate, mkdirat, openat, FileType, Mode, OFlags, CWD};
use rustix::io::{write, Errno};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::bundle::ENGINE_VERSION;
use crate::lock_file;

/// The directory the run locks stand in, named after the journal.
const DIRECTORY: &str = ".runs";

/// What a run lock is named, after the run's own id.
const SUFFIX: &str = ".lock";

/// Why a process could not take a run's lock. Nothing was written to the
/// run's journal.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RunLockError {
    /// The run id is not one plain file name, so its lock cannot be named
    /// inside the run lock directory.
    #[error("run '{0}' is not a safe file name, so it has no run lock")]
    UnsafeId(String),
    /// The run lock directory is a link, not a directory, another user's,
    /// or one others may enter.
    #[error(
        "run lock: the directory {} is not this user's own directory of mode 0700 (decision \
         0108)",
        .0.display()
    )]
    Directory(PathBuf),
    /// The lock file is a link, of more than one link, or not this user's
    /// alone.
    #[error(
        "run lock: the lock {} is not this user's own single file (decision 0108)",
        .0.display()
    )]
    File(PathBuf),
    /// Another driver holds the run. Its facts are what the lock file
    /// recorded, `None` when it has not recorded them yet.
    #[error("{}", named(run_id, holder))]
    Driven {
        run_id: String,
        holder: Option<Holder>,
    },
}

/// The facts a driver records in its run lock, for the operator's eyes.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    pub(crate) pid: u32,
    pub(crate) since: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) host: Option<String>,
    pub(crate) engine: String,
}

/// The words a held run is refused with: the run, the holder process, when
/// it took the lock and the engine that did, or that it recorded itself
/// not at all.
pub(crate) fn named(run_id: &str, holder: &Option<Holder>) -> String {
    match holder {
        Some(holder) => format!(
            "run '{run_id}' is driven by pid {} on this host since {} (engine {})",
            holder.pid, holder.since, holder.engine
        ),
        None => format!(
            "run '{run_id}' is driven by another process on this host, which has not recorded \
             its facts yet"
        ),
    }
}

/// A run's lock held by its descriptor through the shared lock-file
/// home, until this is dropped or the process ends.
#[derive(Debug)]
pub(crate) struct RunLock {
    _held: lock_file::Held,
}

/// Take the run's lock as this process's driver: at once or not at all,
/// recording the holder's facts where it is taken.
pub(crate) fn take(store: &Store, run_id: &str) -> Result<RunLock, RunLockError> {
    take_at(
        store.path(),
        run_id,
        &Facts {
            host: store.host_fingerprint(),
            since: now(),
            engine: ENGINE_VERSION.to_string(),
        },
    )
}

/// The run `run_id`'s lock taken by this process without recording a
/// holder: the broker's admission holds a quarantined run's lock across
/// the claim that releases its worktree, and `operator settle` holds it
/// while it writes, so no driver of that run starts beneath either. A
/// holder's facts stand as the last driver wrote them (decision 0108
/// ruling 3).
pub(crate) fn held_at(journal: &Path, run_id: &str) -> Result<RunLock, RunLockError> {
    let (held, _) = locked_at(journal, run_id)?;
    Ok(RunLock { _held: held })
}

/// Whether `run_id`'s lock file stood beside `journal` before anything
/// took it. A file's presence says nothing of the lock; a run journaled by
/// a binary that predates decision 0108 has none, and its lock is free.
pub(crate) fn present(journal: &Path, run_id: &str) -> bool {
    std::fs::symlink_metadata(beside(journal, run_id)).is_ok()
}

/// What the lock records of its holder.
struct Facts {
    host: Option<String>,
    since: String,
    engine: String,
}

/// [`take`], over the journal's path and the facts to record.
fn take_at(journal: &Path, run_id: &str, facts: &Facts) -> Result<RunLock, RunLockError> {
    let (held, _) = locked_at(journal, run_id)?;
    record(&held, facts);
    Ok(RunLock { _held: held })
}

/// The opened run lock `run_id` of `journal`, taken at once or not at all,
/// and whether its lock file stood before it was opened.
fn locked_at(journal: &Path, run_id: &str) -> Result<(lock_file::Held, bool), RunLockError> {
    if !safe(run_id) {
        return Err(RunLockError::UnsafeId(run_id.to_string()));
    }
    let present = present(journal, run_id);
    let dir = directory(journal)?;
    let name = format!("{run_id}{SUFFIX}");
    let Some(opened) = lock_file::opened(&dir, name.as_str()) else {
        return Err(RunLockError::File(beside(journal, run_id)));
    };
    // A lock that cannot be taken now, for whatever reason, is another's.
    let held = match lock_file::hold(opened) {
        Ok(held) => held,
        Err(opened) => {
            return Err(RunLockError::Driven {
                run_id: run_id.to_string(),
                holder: recorded(opened),
            })
        }
    };
    Ok((held, present))
}

/// The run lock directory: made owner-only where it is missing, opened
/// without following a link, and this user's own directory of mode 0700
/// as it stands.
fn directory(journal: &Path) -> Result<OwnedFd, RunLockError> {
    let at = directory_path(journal);
    match mkdirat(CWD, &at, Mode::RWXU) {
        Ok(()) | Err(Errno::EXIST) => {}
        Err(_) => return Err(RunLockError::Directory(at)),
    }
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let dir = match openat(CWD, &at, flags, Mode::empty()) {
        Ok(dir) => dir,
        Err(_) => return Err(RunLockError::Directory(at)),
    };
    let ours = fstat(&dir).is_ok_and(|stat| {
        (FileType::from_raw_mode(stat.st_mode) == FileType::Directory)
            & ((stat.st_mode & 0o777) == 0o700)
            & lock_file::alone(&stat, lock_file::user())
    });
    ours.then_some(dir).ok_or(RunLockError::Directory(at))
}

/// The run lock directory beside `journal`: `.runs` is a name added to
/// the journal's own, never a replacement of its extension, so
/// `<journal>.runs`, not `<journal without extension>.runs`.
pub(crate) fn directory_path(journal: &Path) -> PathBuf {
    let mut named = OsString::from(journal);
    named.push(DIRECTORY);
    PathBuf::from(named)
}

/// Whether `run_id` is one plain file name: no separator, no `.` or `..`,
/// no NUL, and the whole string.
fn safe(run_id: &str) -> bool {
    match Path::new(run_id)
        .components()
        .collect::<Vec<_>>()
        .as_slice()
    {
        [Component::Normal(name)] => *name == OsStr::new(run_id) && !run_id.contains('\0'),
        _ => false,
    }
}

/// The lock's path, for the operator's refusal.
fn beside(journal: &Path, run_id: &str) -> PathBuf {
    directory_path(journal).join(format!("{run_id}{SUFFIX}"))
}

/// The holder facts in a lock that could not be taken, or `None` when the
/// holder has not recorded them.
fn recorded(held: OwnedFd) -> Option<Holder> {
    serde_json::from_reader(File::from(held)).ok()
}

/// Record `facts` in the lock just taken. The lock is liveness; these are
/// for the operator, so a file that cannot be written leaves the lock
/// held rather than failing it.
fn record(held: &lock_file::Held, facts: &Facts) {
    let holder = Holder {
        pid: std::process::id(),
        since: facts.since.clone(),
        host: facts.host.clone(),
        engine: facts.engine.clone(),
    };
    let text = serde_json::to_vec(&holder).unwrap_or_default();
    let _ = ftruncate(held.as_fd(), 0);
    let _ = write(held.as_fd(), &text);
}

/// This process's start of the hold, in the spelling the dispatcher's
/// holder rows use.
fn now() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("a nanosecond is a valid component")
        .format(&Rfc3339)
        .expect("an RFC 3339 time formats")
}

#[cfg(test)]
mod tests;
