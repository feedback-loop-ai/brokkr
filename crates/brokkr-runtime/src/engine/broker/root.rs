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
    flock, fstat, fsync, linkat, mkdirat, openat, renameat, unlinkat, AtFlags, FileType,
    FlockOperation, Mode, OFlags, Stat,
};
use rustix::io::Errno;
use serde::de::DeserializeOwned;

use super::admission::{coordination, Unknown, LOCK, STAGED};
use super::inventory::Budget;
use super::{directory, owned, protected, Synced, Unprovisioned, REPLY_BYTES_MAX};

/// What a record's predecessor is also named while a seal of it is not
/// final: `name` with this after it, a second link to the very file that
/// stood ([`seal_with`]).
pub(super) const PRIOR: &str = ".old";

/// The longest one wait for the lock lasts before the deadline is read
/// again.
const POLL: Duration = Duration::from_millis(10);

/// Where a seal stands when it hands its hook the directory and a name
/// ([`seal_with`]): its staged copy written and synced, before its
/// deadline check ahead of the rename; renamed in, before the directory is
/// synced and the deadline checked; retiring the record that stood, before
/// the deadline is checked the last time and its prior name removed, the
/// commit; final, before the directory is synced once more; or undone, the
/// record that stood put back, before the directory is synced again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Staged,
    Committed,
    Retiring,
    Final,
    Undone,
}

/// What a seal that committed reports (R1 of U8a2a's fourth run, fixed by
/// U8a2c): its directory synced as it was committed, or not proven synced
/// once its record that stood was retired, its successor then the record
/// every read finds all the same; the latter fails the run under the fence
/// rule where its caller asks it.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Commit {
    Synced,
    Unsynced,
}

impl Commit {
    /// Synced only where both are.
    pub(super) fn and(self, other: Commit) -> Commit {
        match (self, other) {
            (Commit::Synced, Commit::Synced) => Commit::Synced,
            (Commit::Unsynced, _) | (_, Commit::Unsynced) => Commit::Unsynced,
        }
    }
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
    ) -> Result<Commit, Unknown> {
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

/// The record `name` in the directory `dir` at `at`, as it stands
/// ([`standing`]): within MB3's request bound, and one closed record; or
/// an unknown participant there.
pub(super) fn read<T: DeserializeOwned>(
    dir: &OwnedFd,
    name: &str,
    at: &Path,
) -> Result<T, Unknown> {
    let refused = || Unknown::Participant(at.join(name));
    let file = standing(dir, name).ok_or_else(refused)?;
    let mut bytes = Vec::new();
    let file = std::fs::File::from(file);
    let read = file.take(REPLY_BYTES_MAX + 1).read_to_end(&mut bytes);
    let within = read.is_ok_and(|_| bytes.len() as u64 <= REPLY_BYTES_MAX);
    within.then_some(()).ok_or_else(refused)?;
    serde_json::from_slice(&bytes).or(Err(refused()))
}

/// The file `name` in `dir` as it stands for every read: the one also
/// named with [`PRIOR`] where that stands — the record a seal not yet
/// final replaced, or the very file `name` is while that seal is between
/// its link and its rename — or else the file `name`. Either is this
/// user's own regular file, opened without following a link or blocking,
/// of one link, or of two only where `name` is that same file; none
/// otherwise (M2 of run four's second review).
fn standing(dir: &OwnedFd, name: &str) -> Option<OwnedFd> {
    let prior = format!("{name}{PRIOR}");
    let named = regular_at(dir, name);
    // `name` of two links whose prior was not yet there when looked for is
    // a seal that linked it since: its prior is that file.
    let linked = named.as_ref().is_some_and(|(_, stat)| stat.st_nlink == 2);
    let kept = regular_at(dir, &prior).or_else(|| linked.then(|| regular_at(dir, &prior))?);
    match (kept, named) {
        (Some((file, stat)), named) => {
            let one = |(_, at): (OwnedFd, Stat)| same(&at, &stat);
            let linked = (stat.st_nlink == 2) & named.is_some_and(one);
            ((stat.st_nlink == 1) | linked).then_some(file)
        }
        (None, named) => named
            .filter(|(_, stat)| stat.st_nlink == 1)
            .map(|(file, _)| file),
    }
}

/// The file `name` in `dir` and its status, opened without following a
/// link or blocking, where it is this user's own regular file.
fn regular_at(dir: &OwnedFd, name: &str) -> Option<(OwnedFd, Stat)> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = openat(dir, name, flags, Mode::empty()).ok()?;
    let stat = fstat(&file).ok()?;
    let regular = FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile;
    (regular & owned(&stat, user(), true)).then_some((file, stat))
}

/// Whether two statuses are of one file.
fn same(one: &Stat, other: &Stat) -> bool {
    (one.st_dev == other.st_dev) & (one.st_ino == other.st_ino)
}

/// `bytes` sealed as `name` in the directory `dir`, begun before `budget`'s
/// deadline. A seal of `name` cut short before it was final is undone
/// first ([`restored`]). The bytes are written as a new owner-only file
/// beside it ([`staged_as`]); then, the deadline checked once more,
/// renamed in ([`commit`]): the record that stood linked as `name` with
/// [`PRIOR`] after it, and the staged copy renamed over `name`. That is
/// handed to `hook` again, the directory synced and the deadline checked.
/// A first seal is then committed. A reseal is committed only once the
/// record that stood is retired ([`retired`]): the deadline checked the
/// last time, its prior name removed, and the directory synced again.
/// Until then every read finds the record that stood ([`standing`]). A
/// seal that fails or runs late before its commit is undone ([`undone`]):
/// the record that stood renamed back, the same file, so its bytes and its
/// inode, or the record made removed where none stood (H1; R1 of run
/// four's review). One whose undoing fails refuses as unsealed, whatever
/// cut it short, the record that stood still under its prior name for
/// every read and for the next seal to put back (M2 of run four's second
/// review). Once committed, no seal reports failure or lateness: one whose
/// directory is not proven synced after reports itself committed but
/// [`Commit::Unsynced`] (R1 of the fourth run, U8a2c). Only links, renames
/// and removals within one directory, which every filesystem a protected
/// root can stand on provides: no atomic exchange (M3).
pub(super) fn seal_with(
    dir: &OwnedFd,
    name: &str,
    bytes: &[u8],
    budget: &Budget,
    hook: Hook,
) -> Result<Commit, Unknown> {
    budget.check()?;
    restored(dir, name)?;
    let staged = format!("{name}{STAGED}");
    let committed = (staged_as(dir, &staged, bytes, hook))
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
        let undone = undone(dir, name, stood, hook);
        return Err(undone.map_or(Unknown::Unsealed, |()| refused));
    }
    if !stood {
        return Ok(Commit::Synced);
    }
    retired(dir, name, budget, hook)
}

/// The reseal of `name` in `dir`, renamed in and synced, committed by
/// retiring the record that stood: handed to `hook`, the deadline of
/// `budget` checked the last time, and its prior name removed; whether the
/// directory is then synced. One late or whose prior name cannot be
/// removed is undone ([`undone`]) and refused, the record that stood still
/// what every read finds.
fn retired(dir: &OwnedFd, name: &str, budget: &Budget, hook: Hook) -> Result<Commit, Unknown> {
    let prior = format!("{name}{PRIOR}");
    let retiring = (hook(Step::Retiring, dir, name).or(Err(Unknown::Unsealed)))
        .and_then(|()| budget.check())
        .and_then(|()| unlinkat(dir, prior.as_str(), AtFlags::empty()).or(Err(Unknown::Unsealed)));
    if let Err(refused) = retiring {
        let undone = undone(dir, name, true, hook);
        return Err(undone.map_or(Unknown::Unsealed, |()| refused));
    }
    // Committed: its successor alone is the record, as every read finds it.
    let synced = hook(Step::Final, dir, name).and_then(|()| Ok(fsync(dir)?));
    Ok(synced.map_or(Commit::Unsynced, |()| Commit::Synced))
}

/// `bytes` written as the new owner-only file `staged` in `dir`, synced
/// and handed to `hook`: what a seal cut short staged goes first, and one
/// that cannot go is refused by the exclusive create.
fn staged_as(dir: &OwnedFd, staged: &str, bytes: &[u8], hook: Hook) -> Result<(), Unknown> {
    let _ = unlinkat(dir, staged, AtFlags::empty());
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(dir, staged, flags, Mode::RUSR | Mode::WUSR);
    let mut file = std::fs::File::from(file.or(Err(Unknown::Unsealed))?);
    let written = (file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| hook(Step::Staged, dir, staged));
    written.or(Err(Unknown::Unsealed))
}

/// The staged copy committed as `name`: the record that stands there
/// first linked under its prior name, so a seal undone puts that very file
/// back, then the copy renamed over it; or, where none stands, renamed in.
/// Whether one stood. Where the rename fails, the record still is `name`,
/// and its second link goes.
fn commit(dir: &OwnedFd, staged: &str, name: &str) -> Result<bool, Unknown> {
    let prior = format!("{name}{PRIOR}");
    let stood = match linkat(dir, name, dir, prior.as_str(), AtFlags::empty()) {
        Ok(()) => true,
        Err(Errno::NOENT) => false,
        Err(_) => return Err(Unknown::Unsealed),
    };
    let renamed = renameat(dir, staged, dir, name);
    if renamed.is_err() & stood {
        let _ = unlinkat(dir, prior.as_str(), AtFlags::empty());
    }
    renamed.map(|()| stood).or(Err(Unknown::Unsealed))
}

/// A seal that failed or ran late after its commit, undone: the record
/// that `stood` renamed back from its prior name over the copy that
/// replaced it, or, where none stood, the record it made removed; then
/// handed to `hook` and the directory synced again. Why not, where any of
/// that fails: the record that stood is then still under its prior name.
fn undone(dir: &OwnedFd, name: &str, stood: bool, hook: Hook) -> std::io::Result<()> {
    let prior = format!("{name}{PRIOR}");
    let put = if stood {
        renameat(dir, prior.as_str(), dir, name)
    } else {
        unlinkat(dir, name, AtFlags::empty())
    };
    put.map_err(std::io::Error::from)?;
    hook(Step::Undone, dir, name)?;
    Ok(fsync(dir)?)
}

/// A seal of `name` in `dir` cut short before it was final, undone: the
/// record that stood, still under its prior name, renamed back over
/// `name`, or, where both names are already that one file (a seal cut
/// short between its link and its rename), the second name removed; and
/// the directory synced. Nothing where no prior name stands; refused where
/// it cannot be undone.
fn restored(dir: &OwnedFd, name: &str) -> Result<(), Unknown> {
    let prior = format!("{name}{PRIOR}");
    let Some((_, kept)) = regular_at(dir, &prior) else {
        return Ok(());
    };
    let one = regular_at(dir, name).is_some_and(|(_, at)| same(&at, &kept));
    let put = if one {
        unlinkat(dir, prior.as_str(), AtFlags::empty())
    } else {
        renameat(dir, prior.as_str(), dir, name)
    };
    (put.and_then(|()| fsync(dir))).or(Err(Unknown::Unsealed))
}
