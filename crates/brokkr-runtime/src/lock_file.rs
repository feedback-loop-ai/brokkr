//! The one home of a lock file brokkr holds by `flock` (decision 0095
//! rulings 2 and 8, decision 0108 ruling 1): the broker's admission lock
//! in its protected root (decision 0065 slice two), the dispatcher's lock
//! beside its journal, and a run's lock under `<journal>.runs` are each
//! opened, held and judged here, never by a copy (decision 0071 ruling
//! 5). It sits at the crate's root because the callers meet nowhere
//! lower: the broker is private to the engine, and the dispatcher and the
//! run lock are the engine's siblings. How long each waits for the lock
//! is its caller's.

use std::os::fd::{AsFd, OwnedFd};

use rustix::fs::{flock, fstat, openat, FileType, FlockOperation, Mode, OFlags, Stat};
use rustix::path::Arg;

/// The lock file `name` in `dir`, opened to be locked: made owner-only
/// where it is missing, opened without following a link and closed on
/// exec, so no process spawned while it is held inherits the lock; none
/// unless it is this user's own regular file of one link.
pub(crate) fn opened(dir: impl AsFd, name: impl Arg) -> Option<OwnedFd> {
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let lock = openat(dir, name, flags, Mode::RUSR | Mode::WUSR).ok()?;
    single(&lock, user()).then_some(lock)
}

/// A lock this process holds by its descriptor, let go as this is
/// dropped rather than as its descriptor closes: a process this one
/// forked holds a copy of the descriptor, and the lock with it, from its
/// fork until its exec.
#[derive(Debug)]
pub(crate) struct Held(OwnedFd);

impl AsFd for Held {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Held {
    /// A copy of the held descriptor, as a process forked while the lock
    /// was held keeps one until it execs.
    #[cfg(test)]
    pub(crate) fn try_clone(&self) -> std::io::Result<OwnedFd> {
        self.0.try_clone()
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // Unlocking a lock this descriptor holds has no failure to act on.
        let _ = flock(&self.0, FlockOperation::Unlock);
    }
}

/// Hold the opened lock `lock` exclusively and at once: the hold, or the
/// descriptor back where another process holds it, whose recorded facts
/// the caller may read.
pub(crate) fn hold(lock: OwnedFd) -> Result<Held, OwnedFd> {
    match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(Held(lock)),
        Err(_) => Err(lock),
    }
}

/// Whether the open `file` is `user`'s own regular file of one link.
fn single(file: &OwnedFd, user: u32) -> bool {
    fstat(file).is_ok_and(|stat| {
        let regular = FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile;
        regular & (stat.st_nlink == 1) & alone(&stat, user)
    })
}

/// Whether what `stat` describes is `user`'s, and no one else's to read,
/// write or enter.
pub(crate) fn alone(stat: &Stat, user: u32) -> bool {
    (stat.st_uid == user) & (stat.st_mode & 0o077 == 0)
}

/// The engine's own uid, which every lock and directory it makes is owned
/// by.
pub(crate) fn user() -> u32 {
    brokkr_protocol::hands::ids().0
}

#[cfg(test)]
mod tests;
