//! The one home of a lock file brokkr holds by `flock` (decision 0095
//! rulings 2 and 8): the broker's admission lock in its protected root
//! (decision 0065 slice two) and the dispatcher's lock beside its journal
//! are each opened and judged here, never by a copy (decision 0071 ruling
//! 5). It sits at the crate's root because the two callers meet nowhere
//! lower: the broker is private to the engine, and the dispatcher is the
//! engine's sibling. How long each waits for the lock is its caller's.

use std::os::fd::{AsFd, OwnedFd};

use rustix::fs::{fstat, openat, FileType, Mode, OFlags, Stat};
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
