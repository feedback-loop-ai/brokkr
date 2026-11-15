//! The coordination root as admission holds it (decision 0065 slice two,
//! U8a2a; D6, R5): an already opened, owner-only directory, and the path
//! it was opened at. Production resolves it from the host's HOME through
//! the protected walk, which judges every ancestor ([`Root::of`]); a unit
//! test hands one of its own to admission's `_with` seam. Everything
//! admission reads or makes below it is reached from that handle, one
//! plain name at a time, never from HOME again.

use std::ffi::OsStr;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustix::fs::{
    flock, fstat, fsync, mkdirat, openat, renameat_with, FileType, FlockOperation, Mode, OFlags,
    RenameFlags, Stat, CWD,
};
use rustix::io::Errno;
use serde::de::DeserializeOwned;

use super::admission::{Unknown, LOCK, STAGED};
use super::inventory::Budget;
use super::sources::{observed, Table};
use super::{directory, owned, protected, Synced, Unprovisioned, PROTECTED_ROOT, REPLY_BYTES_MAX};
use crate::lock_file::{self, user};

mod seal;
pub(in crate::engine::broker) use seal::seal_with;
use seal::{seal_made, Sheet};

/// The protected root's path under `home`.
pub(super) fn coordination(home: &Path) -> PathBuf {
    PROTECTED_ROOT
        .iter()
        .fold(home.to_path_buf(), |path, name| path.join(name))
}

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
    /// A reseal's record that stood linked under its prior name, before the
    /// atomic exchange that replaces it: the last point a hand can remove
    /// the name and have the exchange meet it gone (decision 0108 ruling 3,
    /// #440 fix round 440d6).
    Linked,
    /// A restore's record that stood linked under its prior name, before
    /// the exchange that puts it back: the last point a hand can remove the
    /// name and have the restore meet it gone (decision 0108 ruling 3,
    /// #440 fix round 440d6).
    Restored,
    /// An undo's record that stood exchanged back over the name it was
    /// replaced from, before the successor it swapped out is removed: a
    /// seal cut short here leaves the record that stood at its name and the
    /// successor under the staged name, never the other way round (decision
    /// 0108 ruling 3, #440 fix round 440d6).
    Exchanged,
    Committed,
    Retiring,
    Final,
    Undone,
}

/// A record's device and inode: the file a reseal proves it replaces is the
/// one it read, never a record made in its place (decision 0108 ruling 3,
/// #440 fix rounds 440d5 and 440d6).
pub(super) type Identity = (u64, u64);

/// A record this build read and keeps open until its proof: the device and
/// inode it read, and the descriptor that keeps that inode from being freed
/// and reused by a file planted at its name (decision 0108 ruling 3, #440
/// fix round 440d6). Numbers alone are not a proof on a filesystem that
/// reuses a freed inode at once, which ext4 does.
#[derive(Debug)]
pub(super) struct Record {
    /// The descriptor holds the inode open until the proof, so the
    /// filesystem cannot free and reuse its number under the name.
    pub(super) _file: OwnedFd,
    pub(super) id: Identity,
}

/// How a reseal replaces a record by one atomic exchange of two names, or
/// the error that refuses it: production's [`exchanged`], a test's seam
/// reporting a filesystem that has none (decision 0108 ruling 3, #440 fix
/// round 440d6).
pub(super) type Exchange = fn(&OwnedFd, &str, &OwnedFd, &str) -> Result<(), Errno>;

/// Production's exchange: `renameat2(RENAME_EXCHANGE)` on Linux,
/// `renameatx_np(RENAME_SWAP)` on macOS (rustix 1.1.6's `renameat_with`,
/// `RenameFlags::EXCHANGE`). A filesystem that has none reports `EINVAL`,
/// `ENOTSUP`/`EOPNOTSUPP` or `ENOSYS`, and the reseal fails closed.
fn exchanged(dir: &OwnedFd, one: &str, other: &OwnedFd, two: &str) -> Result<(), Errno> {
    renameat_with(dir, one, other, two, RenameFlags::EXCHANGE)
}

/// Whether a seal may make the record it writes where none stands: a first
/// seal does, and a reseal of a record that must still stand never does, so
/// a removal inside its window is refused rather than written back. A
/// reseal carries the record it read and holds open, so the pairing is the
/// type's (decision 0108 ruling 3, #440 fix round 440d5).
#[derive(Debug, Clone, Copy)]
enum Make<'a> {
    May,
    Never(&'a Record),
}

/// What a seal that committed reports (R1 of U8a2a's fourth run, fixed by
/// U8a2c): its directory synced as it was committed, or not proven synced
/// once its record that stood was retired, its successor then the record
/// every read finds all the same; the latter fails the run under the fence
/// rule where its caller asks it.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) enum Commit {
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

/// How the mount table is observed for a comparison of writers with an MCP
/// server's sources, the source first named where it proves nothing
/// ([`observed`] in production; U8a2b): a unit test's own on a root it
/// opened, to show the aliases a test cannot mount.
pub(super) type Mounts = fn(&Budget, &Path) -> Result<Table, Unknown>;

/// The coordination root, opened, the hook its seals run, and how the
/// mount table is observed beside it.
#[derive(Debug)]
pub(in crate::engine) struct Root {
    dir: OwnedFd,
    path: PathBuf,
    hook: Hook,
    mounts: Mounts,
    exchange: Exchange,
}

/// Why the coordination root a readout looks for is not one it may read:
/// it does not stand, a path on the way is a link, not one plain name or
/// not this user's alone, or it cannot be opened at all — which only its
/// absence is not (decision 0105, the controller's S2 ruling on #1020;
/// 1020b and 1020c).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Existing {
    Absent,
    NotAlone(PathBuf),
    Linked(PathBuf),
    NotPlain(PathBuf),
    Unreadable(PathBuf, std::io::ErrorKind),
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
        let (hook, mounts) = (unhooked, observed as Mounts);
        Some(Root {
            dir,
            path,
            hook,
            mounts,
            exchange: exchanged,
        })
    }

    /// The root under `home` where it already stands, reached by the walk
    /// [`Root::of`] takes — from `/`, one plain name at a time, never
    /// through a link, every ancestor judged by `owned` and the last one as
    /// this user's alone — and opened, nothing made: the read a verb that
    /// only lists or reads a retirement takes, so a readout writes no
    /// directory. Absent, a link or a path not this user's alone, and
    /// unreadable are told apart, so a readout can name the problem rather
    /// than as no history (decision 0105; 1020b).
    pub(super) fn existing(home: &Path) -> Result<Root, Existing> {
        let dir = walked(home)?;
        let path = coordination(home);
        let (hook, mounts) = (unhooked, observed as Mounts);
        Ok(Root {
            dir,
            path,
            hook,
            mounts,
            exchange: exchanged,
        })
    }

    /// The root `dir` already opened at `path`, where it is this user's
    /// alone, its seals handed to `hook`, the mount table observed by
    /// `mounts` and reseals exchanging by production's `exchanged`: the
    /// handle a unit test stands in, on any TMPDIR.
    #[cfg(test)]
    pub(super) fn opened(
        dir: OwnedFd,
        path: PathBuf,
        (hook, mounts): (Hook, Mounts),
    ) -> Option<Root> {
        Root::opened_exchanging(dir, path, (hook, mounts), exchanged)
    }

    /// [`Root::opened`], reseals exchanging by `exchange`: the house `_with`
    /// seam a test shows a filesystem with no atomic exchange, or one whose
    /// exchange fails, nothing in production (decision 0108 ruling 3, #440
    /// fix round 440d6).
    #[cfg(test)]
    pub(super) fn opened_exchanging(
        dir: OwnedFd,
        path: PathBuf,
        (hook, mounts): (Hook, Mounts),
        exchange: Exchange,
    ) -> Option<Root> {
        let stat = fstat(&dir).ok()?;
        let root = Root {
            dir,
            path,
            hook,
            mounts,
            exchange,
        };
        owned(&stat, user(), true).then_some(root)
    }

    /// How the mount table is observed beside an MCP server's sources.
    pub(super) fn mounts(&self) -> Mounts {
        self.mounts
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
        seal_with(dir, name, bytes, budget, self.hook, self.exchange)
    }

    /// `bytes` sealed as `name` in `dir` only over the very record `stood`
    /// named, which the caller read and holds open, by one atomic exchange:
    /// a record gone when the exchange runs, or a filesystem without an
    /// exchange, is never written back (decision 0108 ruling 3, #440 fix
    /// rounds 440d5 and 440d6). `read` is that record's own bytes, which the
    /// undo and the next seal's restore use to tell the record that stood
    /// from a successor left by a seal cut short.
    pub(super) fn reseal(
        &self,
        dir: &OwnedFd,
        name: &str,
        bytes: &[u8],
        budget: &Budget,
        stood: &Record,
        read: &[u8],
    ) -> Result<Commit, Unknown> {
        seal_made(Sheet {
            dir,
            name,
            bytes,
            budget,
            hook: self.hook,
            exchange: self.exchange,
            make: Make::Never(stood),
            read,
        })
    }
}

/// Production's hook: nothing.
fn unhooked(_: Step, _: &OwnedFd, _: &str) -> std::io::Result<()> {
    Ok(())
}

/// The coordination root under `home` where it already stands, reached from
/// `/` one plain name at a time without following a link and judged as
/// [`super::protected`] judges every ancestor: this user's or root's and
/// writable by no one else for each ancestor, and this user's alone for the
/// root itself. Nothing is made.
fn walked(home: &Path) -> Result<OwnedFd, Existing> {
    let names: Vec<&OsStr> = home
        .iter()
        .skip(1)
        .chain(PROTECTED_ROOT.iter().map(OsStr::new))
        .collect();
    let private = names.len() - 1;
    let mut at = PathBuf::from("/");
    // `/` and an `fstat` of an open descriptor cannot fail on a supported
    // host; their fallback is unreadable rather than a guess at a kind.
    let mut held = openat(CWD, "/", directory(), Mode::empty()).or(Err(Existing::Unreadable(
        at.clone(),
        std::io::ErrorKind::Other,
    )))?;
    for (index, name) in names.into_iter().enumerate() {
        at.push(name);
        held = step_existing(&held, name, &at, index == private)?;
    }
    Ok(held)
}

/// One plain name `name` on the way to the coordination root, at `at`,
/// opened in `parent` without following a link and judged [`owned`]'s way:
/// absent, a link, not one plain name, not this user's alone, or another
/// open error naming the path walked so far.
fn step_existing(
    parent: &OwnedFd,
    name: &OsStr,
    at: &Path,
    private: bool,
) -> Result<OwnedFd, Existing> {
    plain(name)
        .then_some(())
        .ok_or_else(|| Existing::NotPlain(at.to_path_buf()))?;
    let held = match openat(parent, name, directory(), Mode::empty()) {
        Ok(held) => held,
        Err(Errno::NOENT) => return Err(Existing::Absent),
        Err(Errno::NOTDIR | Errno::LOOP) => {
            let linked =
                std::fs::symlink_metadata(at).is_ok_and(|stat| stat.file_type().is_symlink());
            return Err(if linked {
                Existing::Linked(at.to_path_buf())
            } else {
                Existing::Unreadable(at.to_path_buf(), std::io::ErrorKind::NotADirectory)
            });
        }
        Err(error) => {
            return Err(Existing::Unreadable(
                at.to_path_buf(),
                std::io::Error::from(error).kind(),
            ))
        }
    };
    let stat = fstat(&held).or(Err(Existing::Unreadable(
        at.to_path_buf(),
        std::io::ErrorKind::Other,
    )))?;
    owned(&stat, user(), private)
        .then_some(held)
        .ok_or_else(|| Existing::NotAlone(at.to_path_buf()))
}

/// Whether `name` is one plain name: one component that names neither its
/// parent nor the current directory, so entering it walks out of no judged
/// directory and past no name the walk did not judge.
fn plain(name: &OsStr) -> bool {
    Path::new(name).file_name() == Some(name)
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
    plain(name).then_some(()).ok_or(Unprovisioned::Root)?;
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
/// link, as [`lock_file::opened`] judges it, or refused. Released as the
/// handle is dropped.
pub(super) fn locked(root: &OwnedFd, deadline: Instant) -> Result<OwnedFd, Unknown> {
    let lock = lock_file::opened(root, LOCK).ok_or(Unknown::Lock)?;
    taken(&lock, deadline)?;
    Ok(lock)
}

/// `lock` taken exclusively before `deadline`: a lock that cannot be taken
/// now, for whatever reason, is waited for until the deadline, and then
/// refused.
fn taken(lock: &OwnedFd, deadline: Instant) -> Result<(), Unknown> {
    loop {
        if flock(lock, FlockOperation::NonBlockingLockExclusive).is_ok() {
            return Ok(());
        }
        let left = deadline.saturating_duration_since(Instant::now());
        (!left.is_zero()).then_some(()).ok_or(Unknown::Expired)?;
        std::thread::sleep(left.min(POLL));
    }
}

/// The admission lock in the protected `root` where it already stands,
/// held exclusively before `deadline`: nothing made, so a readout creates
/// no lock. None where no lock file stands: nothing has ever been written
/// under it, so there is no record to read (decision 0105, L1). A lock
/// file that is not this user's own regular file of one link refuses as
/// [`locked`] refuses it, and so does one that cannot be opened for any
/// reason but its absence (decision 0105, the controller's S2 ruling on
/// #1020).
pub(super) fn locked_existing(
    root: &OwnedFd,
    deadline: Instant,
) -> Result<Option<OwnedFd>, Unknown> {
    let flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let lock = match openat(root, LOCK, flags, Mode::empty()) {
        Ok(lock) => lock,
        Err(Errno::NOENT) => return Ok(None),
        Err(_) => return Err(Unknown::Lock),
    };
    let single = fstat(&lock).is_ok_and(|stat| {
        (FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile)
            & (stat.st_nlink == 1)
            & lock_file::alone(&stat, user())
    });
    single.then_some(()).ok_or(Unknown::Lock)?;
    taken(&lock, deadline)?;
    Ok(Some(lock))
}

/// The record `name` in the directory `dir` at `at`, as it stands
/// ([`standing`]): within MB3's request bound, and one closed record; or
/// an unknown participant there.
pub(super) fn read<T: DeserializeOwned>(
    dir: &OwnedFd,
    name: &str,
    at: &Path,
) -> Result<T, Unknown> {
    identified(dir, name, at).map(|(record, _, _)| record)
}

/// [`read`], with the very file it read kept open and its own bytes: what a
/// reseal proves the record it replaces still is, why a freed inode number
/// cannot be reused under its name before the proof, and the record's bytes
/// as it stood, which its undo and the next seal's restore compare against
/// (decision 0108 ruling 3, #440 fix rounds 440d5 and 440d6). The bytes are
/// the file's own, not a re-serialisation, so a record another build wrote
/// in another byte form still restores (run four's review, S7).
pub(super) fn identified<T: DeserializeOwned>(
    dir: &OwnedFd,
    name: &str,
    at: &Path,
) -> Result<(T, Record, Vec<u8>), Unknown> {
    let refused = || Unknown::Participant(at.join(name));
    let (bytes, held) = bytes_identified(dir, name, at)?;
    let record = serde_json::from_slice(&bytes).or(Err(refused()))?;
    Ok((record, held, bytes))
}

/// Whether `name` itself stands in `dir`: a record's name, never its prior
/// name, which a seal not yet final leaves behind (decision 0108 ruling 3,
/// #440 fix round 440d6).
pub(super) fn present(dir: &OwnedFd, name: &str) -> bool {
    regular_at(dir, name).is_some()
}

/// The bytes of the record `name` in the directory `dir` at `at`, as it
/// stands ([`standing`]), within MB3's request bound; or an unknown
/// participant there.
pub(super) fn bytes(dir: &OwnedFd, name: &str, at: &Path) -> Result<Vec<u8>, Unknown> {
    bytes_identified(dir, name, at).map(|(bytes, _)| bytes)
}

/// [`bytes`], with the very file it read kept open.
fn bytes_identified(dir: &OwnedFd, name: &str, at: &Path) -> Result<(Vec<u8>, Record), Unknown> {
    let refused = || Unknown::Participant(at.join(name));
    let file = standing(dir, name).ok_or_else(refused)?;
    // Read the identity through std's metadata of the same descriptor,
    // whose device is a u64 on every platform: rustix's `st_dev` is an i32
    // on macOS (decision 0063; 0105 M2's home for that rule).
    let file = std::fs::File::from(file);
    let found = file.metadata().or(Err(refused()))?;
    let held = file.try_clone().or(Err(refused()))?;
    let mut bytes = Vec::new();
    let read = file.take(REPLY_BYTES_MAX + 1).read_to_end(&mut bytes);
    let within = read.is_ok_and(|_| bytes.len() as u64 <= REPLY_BYTES_MAX);
    within
        .then_some((
            bytes,
            Record {
                _file: held.into(),
                id: (found.dev(), found.ino()),
            },
        ))
        .ok_or_else(refused)
}

/// The file `name` in `dir` as it stands for every read: the one also
/// named with [`PRIOR`] where that stands — the record a seal not yet
/// final replaced, or the very file `name` is while that seal is between
/// its link and its exchange — or else the file `name`. Either is this
/// user's own regular file, opened without following a link or blocking,
/// of one link, or of two only where `name` is that same file, or where the
/// exchange has put it under the staged name until the old name is retired;
/// none otherwise (M2 of run four's second review; #440 fix round 440d6).
fn standing(dir: &OwnedFd, name: &str) -> Option<OwnedFd> {
    let prior = format!("{name}{PRIOR}");
    let staged = format!("{name}{STAGED}");
    let named = regular_at(dir, name);
    // `name` of two links whose prior was not yet there when looked for is
    // a seal that linked it since: its prior is that file.
    let linked = named
        .as_ref()
        .is_some_and(|(_, stat, _)| stat.st_nlink == 2);
    let kept = regular_at(dir, &prior).or_else(|| linked.then(|| regular_at(dir, &prior))?);
    match (kept, named) {
        (Some((file, stat, id)), named) => {
            let one = |(_, _, at): &(OwnedFd, Stat, Identity)| *at == id;
            let linked = (stat.st_nlink == 2) & named.as_ref().is_some_and(one);
            // A reseal's exchange leaves the record that stood under the
            // staged name as well as its prior name until the old name is
            // retired: either second link means `name` is not the record.
            let exchanging =
                (stat.st_nlink == 2) & regular_at(dir, &staged).as_ref().is_some_and(one);
            ((stat.st_nlink == 1) | linked | exchanging).then_some(file)
        }
        (None, named) => named
            .filter(|(_, stat, _)| stat.st_nlink == 1)
            .map(|(file, _, _)| file),
    }
}

/// The file `name` in `dir` and its status, opened without following a
/// link or blocking, where it is this user's own regular file, with the
/// device and inode std's metadata reports as a u64 on either supported
/// host.
fn regular_at(dir: &OwnedFd, name: &str) -> Option<(OwnedFd, Stat, Identity)> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = openat(dir, name, flags, Mode::empty()).ok()?;
    let stat = fstat(&file).ok()?;
    let regular = FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile;
    let found = std::fs::File::from(file.try_clone().ok()?)
        .metadata()
        .ok()?;
    let id = (found.dev(), found.ino());
    (regular & owned(&stat, user(), true)).then_some((file, stat, id))
}
