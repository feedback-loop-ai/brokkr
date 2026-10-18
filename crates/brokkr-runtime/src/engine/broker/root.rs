//! The coordination root as admission holds it (decision 0065 slice two,
//! U8a2a; D6, R5): an already opened, owner-only directory, and the path
//! it was opened at. Production resolves it from the host's HOME through
//! the protected walk, which judges every ancestor ([`Root::of`]); a unit
//! test hands one of its own to admission's `_with` seam. Everything
//! admission reads or makes below it is reached from that handle, one
//! plain name at a time, never from HOME again.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustix::fs::{
    flock, fstat, fsync, mkdirat, openat, renameat, renameat_with, unlinkat, AtFlags, FileType,
    FlockOperation, Mode, OFlags, RenameFlags,
};
use rustix::io::Errno;
use serde::de::DeserializeOwned;

use super::admission::{coordination, Unknown, LOCK, STAGED};
use super::inventory::Budget;
use super::{directory, owned, protected, Synced, Unprovisioned, REPLY_BYTES_MAX};

/// The longest one wait for the lock lasts before the deadline is read
/// again.
const POLL: Duration = Duration::from_millis(10);

/// Where a seal stands when it hands its hook the directory and the staged
/// name ([`seal_with`]): its staged copy written and synced, before its
/// last deadline check ahead of the commit; or committed, before the
/// directory is synced and the deadline checked the last time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Staged,
    Committed,
}

/// What every record sealed below a root is handed at each [`Step`]; an
/// error it returns fails that step as the step's own write or sync
/// failing would. Nothing in production ([`unhooked`]); a unit test's own,
/// on a root it opened, to spend the deadline or fail the seal there (H1;
/// R1 of run four's review).
pub(super) type Hook = fn(Step, &OwnedFd, &str) -> std::io::Result<()>;

/// The coordination root, opened, and the hook its seals run.
#[derive(Debug)]
pub(in crate::engine) struct Root {
    dir: OwnedFd,
    path: PathBuf,
    hook: Hook,
}

/// Two handles are one root where they were opened at one path.
impl PartialEq for Root {
    fn eq(&self, other: &Root) -> bool {
        self.path == other.path
    }
}

impl Eq for Root {}

impl Root {
    /// The protected root under `home`, made where it is missing, HOME's
    /// every ancestor and every directory on the way judged as the broker
    /// judges them; none where any is not protected.
    pub(super) fn of(home: &Path) -> Option<Root> {
        let dir = protected(home, &[]).ok()?;
        let path = coordination(home);
        let hook = unhooked;
        Some(Root { dir, path, hook })
    }

    /// The root `dir` already opened at `path`, where it is this user's
    /// alone, its seals handed to `hook`: the handle a unit test stands in,
    /// on any TMPDIR.
    #[cfg(test)]
    pub(super) fn opened(dir: OwnedFd, path: PathBuf, hook: Hook) -> Option<Root> {
        let stat = fstat(&dir).ok()?;
        owned(&stat, user(), true).then_some(Root { dir, path, hook })
    }

    /// The opened root.
    pub(super) fn dir(&self) -> &OwnedFd {
        &self.dir
    }

    /// Where it was opened.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// The directory `names` below the root, each one plain name of a
    /// directory this user's alone, made where it is missing and synced
    /// in the parent that gained it (M9).
    pub(super) fn below(&self, names: &[&str]) -> Result<OwnedFd, Unknown> {
        let mut held = (self.dir.try_clone()).or(Err(Unknown::Root))?;
        for name in names {
            held = step(&held, OsStr::new(name), (true, true), &|parent| {
                fsync(parent)
            })
            .or(Err(Unknown::Root))?;
        }
        Ok(held)
    }

    /// `bytes` sealed as `name` in `dir`, a directory below this root,
    /// within `budget`'s deadline, its staged copy handed to this root's
    /// hook ([`seal_with`]).
    pub(super) fn seal(
        &self,
        dir: &OwnedFd,
        name: &str,
        bytes: &[u8],
        budget: &Budget,
    ) -> Result<(), Unknown> {
        seal_with(dir, name, bytes, budget, self.hook)
    }
}

/// Production's hook: nothing.
fn unhooked(_: Step, _: &OwnedFd, _: &str) -> std::io::Result<()> {
    Ok(())
}

/// The directory `name` in `parent`: one plain name, made owner-only where
/// `make` and missing, `parent` then handed to `synced`, entered without
/// following a link, and protecting what lies below it by [`owned`]'s rule,
/// this user's alone where `private`. A directory that stands already is
/// judged as it stands.
pub(super) fn step(
    parent: &OwnedFd,
    name: &OsStr,
    (make, private): (bool, bool),
    synced: Synced<'_>,
) -> Result<OwnedFd, Unprovisioned> {
    let plain = Path::new(name).file_name() == Some(name);
    plain.then_some(()).ok_or(Unprovisioned::Root)?;
    if make && mkdirat(parent, name, Mode::RWXU).is_ok() {
        synced(parent).or(Err(Unprovisioned::Root))?;
    }
    let held = openat(parent, name, directory(), Mode::empty()).or(Err(Unprovisioned::Root))?;
    let stat = fstat(&held).or(Err(Unprovisioned::Root))?;
    let guarded = owned(&stat, user(), private);
    guarded.then_some(held).ok_or(Unprovisioned::Root)
}

/// The admission lock in the protected `root`, made where it is missing,
/// held exclusively before `deadline`: this user's own regular file of one
/// link, or refused. Released as the handle is dropped.
pub(super) fn locked(root: &OwnedFd, deadline: Instant) -> Result<OwnedFd, Unknown> {
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let lock = openat(root, LOCK, flags, Mode::RUSR | Mode::WUSR).or(Err(Unknown::Lock))?;
    single(&lock).ok_or(Unknown::Lock)?;
    // A lock that cannot be taken now, for whatever reason, is waited for
    // until the deadline, and then refused.
    loop {
        if flock(&lock, FlockOperation::NonBlockingLockExclusive).is_ok() {
            return Ok(lock);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        (!left.is_zero()).then_some(()).ok_or(Unknown::Expired)?;
        std::thread::sleep(left.min(POLL));
    }
}

/// Whether the open `file` is this user's own regular file of one link.
fn single(file: &OwnedFd) -> Option<()> {
    let stat = fstat(file).ok()?;
    let regular = FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile;
    (regular & (stat.st_nlink == 1) & owned(&stat, user(), true)).then_some(())
}

/// The engine's own uid, which every directory it makes is owned by.
fn user() -> u32 {
    brokkr_protocol::hands::ids().0
}

/// The record `name` in the directory `dir` at `at`: this user's own
/// regular file of one link, within MB3's request bound, and one closed
/// record; or an unknown participant there.
pub(super) fn read<T: DeserializeOwned>(
    dir: &OwnedFd,
    name: &str,
    at: &Path,
) -> Result<T, Unknown> {
    let refused = || Unknown::Participant(at.join(name));
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = openat(dir, name, flags, Mode::empty()).or(Err(refused()))?;
    single(&file).ok_or_else(refused)?;
    let mut bytes = Vec::new();
    let file = std::fs::File::from(file);
    let read = file.take(REPLY_BYTES_MAX + 1).read_to_end(&mut bytes);
    let within = read.is_ok_and(|_| bytes.len() as u64 <= REPLY_BYTES_MAX);
    within.then_some(()).ok_or_else(refused)?;
    serde_json::from_slice(&bytes).or(Err(refused()))
}

/// `bytes` sealed as `name` in the directory `dir`, begun before `budget`'s
/// deadline: written as a new owner-only file beside it, `name` with
/// [`STAGED`] after it, synced and handed to `hook`; then, the deadline
/// checked once more, committed ([`commit`]), handed to `hook` again, the
/// directory synced and the deadline checked the last time. Nothing before
/// the commit touches `name`, and a seal that fails or runs past the
/// deadline after it is undone ([`undone`]): every failed or late seal
/// leaves the record that stood exactly as it was, its bytes and its
/// inode, or no record where none stood (H1; R1 of run four's review).
/// What one cut short staged goes first, and one that cannot go is refused
/// by the exclusive create; a staged name holds either the copy a seal is
/// committing or the record it replaced, and only `name` is ever read.
pub(super) fn seal_with(
    dir: &OwnedFd,
    name: &str,
    bytes: &[u8],
    budget: &Budget,
    hook: Hook,
) -> Result<(), Unknown> {
    budget.check()?;
    let staged = format!("{name}{STAGED}");
    let _ = unlinkat(dir, staged.as_str(), AtFlags::empty());
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(dir, staged.as_str(), flags, Mode::RUSR | Mode::WUSR);
    let mut file = std::fs::File::from(file.or(Err(Unknown::Unsealed))?);
    let written = (file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| hook(Step::Staged, dir, &staged));
    let committed = (written.or(Err(Unknown::Unsealed)))
        .and_then(|()| budget.check())
        .and_then(|()| commit(dir, &staged, name));
    let stood = match committed {
        Ok(stood) => stood,
        Err(refused) => {
            let _ = unlinkat(dir, staged.as_str(), AtFlags::empty());
            return Err(refused);
        }
    };
    let synced =
        hook(Step::Committed, dir, &staged).and_then(|()| fsync(dir).map_err(std::io::Error::from));
    let durable = (synced.or(Err(Unknown::Unsealed))).and_then(|()| budget.check());
    if let Err(refused) = durable {
        undone(dir, &staged, name, stood);
        return Err(refused);
    }
    // What stood was replaced for good; a crash before it goes leaves it
    // staged, for the next seal to remove.
    let _ = unlinkat(dir, staged.as_str(), AtFlags::empty());
    Ok(())
}

/// The staged copy committed as `name`: exchanged atomically with the
/// record that stands there, which is kept under the staged name until the
/// seal is final, so a seal undone puts that very file back; or, where
/// none stands, renamed in. Whether one stood. A directory that cannot
/// exchange two names refuses, never replacing a record it could not put
/// back.
fn commit(dir: &OwnedFd, staged: &str, name: &str) -> Result<bool, Unknown> {
    match renameat_with(dir, staged, dir, name, RenameFlags::EXCHANGE) {
        Ok(()) => Ok(true),
        Err(Errno::NOENT) => {
            (renameat(dir, staged, dir, name).map(|()| false)).or(Err(Unknown::Unsealed))
        }
        Err(_) => Err(Unknown::Unsealed),
    }
}

/// A seal that failed or ran late after its commit, undone: the record
/// that `stood` exchanged back in and the copy that replaced it removed,
/// or, where none stood, the record it made removed; the directory then
/// synced again.
fn undone(dir: &OwnedFd, staged: &str, name: &str, stood: bool) {
    if stood {
        let _ = renameat_with(dir, staged, dir, name, RenameFlags::EXCHANGE);
        let _ = unlinkat(dir, staged, AtFlags::empty());
    } else {
        let _ = unlinkat(dir, name, AtFlags::empty());
    }
    let _ = fsync(dir);
}
