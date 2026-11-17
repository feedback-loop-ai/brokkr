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

use rustix::fs::{flock, fstat, openat, FileType, FlockOperation, Mode, OFlags, RawMode, Stat};
use rustix::io::Errno;
use rustix::path::Arg;
use thiserror::Error;

/// Why a lock file was not opened as this user's own single file (#1082):
/// the errno its open met, or the one clause of [`judged`] it failed. Its
/// words follow whatever sentence its caller refuses by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Unopened {
    #[error("its open failed with {}", named(*.0))]
    Open(Errno),
    #[error("its status could not be read: {}", named(*.0))]
    Status(Errno),
    #[error("it is not a regular file")]
    NotRegular,
    #[error("it has {0} links")]
    Links(u64),
    #[error("it is another user's, uid {0}")]
    Owner(u32),
    #[error("its mode {0:04o} lets its group or others in")]
    Mode(RawMode),
}

/// The name of `errno` where it is one an open or a status read meets, and
/// its number otherwise.
fn named(errno: Errno) -> String {
    let name = match errno {
        Errno::MFILE => "EMFILE",
        Errno::NFILE => "ENFILE",
        Errno::INTR => "EINTR",
        Errno::NOMEM => "ENOMEM",
        Errno::ACCESS => "EACCES",
        Errno::PERM => "EPERM",
        Errno::LOOP => "ELOOP",
        Errno::ISDIR => "EISDIR",
        Errno::NOTDIR => "ENOTDIR",
        Errno::NOSPC => "ENOSPC",
        Errno::ROFS => "EROFS",
        Errno::IO => "EIO",
        other => return format!("errno {}", other.raw_os_error()),
    };
    name.to_owned()
}

/// The lock file `name` in `dir`, opened to be locked: made owner-only
/// where it is missing, opened without following a link and closed on
/// exec, so no process spawned while it is held inherits the lock; none
/// unless it is this user's own regular file of one link.
pub(crate) fn opened(dir: impl AsFd, name: impl Arg) -> Option<OwnedFd> {
    open(dir, name).ok()
}

/// [`opened`], and why not where it is not (#1082).
pub(crate) fn open(dir: impl AsFd, name: impl Arg) -> Result<OwnedFd, Unopened> {
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let lock = openat(dir, name, flags, Mode::RUSR | Mode::WUSR).map_err(Unopened::Open)?;
    judged(&lock, user())?;
    Ok(lock)
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

/// Whether the open `file` is `user`'s own regular file of one link, and
/// the first clause it fails where it is not: the one home of that
/// judgment, for an open that made the file and one that found it.
pub(crate) fn judged(file: impl AsFd, user: u32) -> Result<(), Unopened> {
    let stat = fstat(file).map_err(Unopened::Status)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(Unopened::NotRegular);
    }
    if stat.st_nlink != 1 {
        return Err(Unopened::Links(widened(stat.st_nlink)));
    }
    exposed(&stat, user).map_or(Ok(()), Err)
}

/// Whether what `stat` describes is `user`'s, and no one else's to read,
/// write or enter.
pub(crate) fn alone(stat: &Stat, user: u32) -> bool {
    exposed(stat, user).is_none()
}

/// The clause by which what `stat` describes is not `user`'s alone.
fn exposed(stat: &Stat, user: u32) -> Option<Unopened> {
    if stat.st_uid != user {
        return Some(Unopened::Owner(stat.st_uid));
    }
    (stat.st_mode & 0o077 != 0).then_some(Unopened::Mode(stat.st_mode & 0o777))
}

/// A link count as the one width its words carry: `st_nlink` is a `u64`
/// here, a `u32` or a `u16` elsewhere, and a conversion written against
/// one of them is useless or missing on the others.
fn widened(count: impl Into<u64>) -> u64 {
    count.into()
}

/// The engine's own uid, which every lock and directory it makes is owned
/// by.
pub(crate) fn user() -> u32 {
    brokkr_protocol::hands::ids().0
}

#[cfg(test)]
pub(crate) mod tests;
