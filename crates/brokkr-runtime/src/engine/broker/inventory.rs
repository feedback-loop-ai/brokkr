//! The protected inventory as admission reads it under its lock (decision
//! 0065 slice two, U8a2a; D6, MB3): every standing reservation and every
//! historical root in the coordination root, within D5's entry bound and
//! the attempt's startup deadline.
//!
//! The root holds the lock and one directory per worktree, named by its
//! key. A worktree's directory holds its historical roots' records and one
//! directory per run; a run's holds its reservation and the directory of
//! every attempt its reservation names. Each directory is this user's
//! alone and reached without following a link. A record written beside
//! the one it becomes, its rename never done, is passed over: a writer
//! starts only once its reservation is renamed in and synced, so such a
//! record admitted none. A reservation's prior name, which a seal not yet
//! final keeps the record it replaced under, is read as the reservation
//! and stands only beside it. Anything else, or anything that cannot be read,
//! is a participant nothing can vouch for, and refuses with MB3's identity
//! cause.

use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brokkr_protocol::hands;
use rustix::fs::{openat, statat, AtFlags, Dir, FileType, Mode};

use super::admission::{Reservation, Unknown, LOCK, RESERVATION, STAGED};
use super::historical::{named, Historical};
use super::root::{read, PRIOR};
use super::{directory, owned};

/// MB3's entry bound, which the inventory's traversal shares (D5; operator
/// ruling 2026-10-07: the count limits are its memory bound). Its depth is
/// fixed at three, it follows no link and reads no mount table, so D5's
/// other three bounds hold by construction.
const ENTRIES_MAX: u64 = 1_000_000;

/// What admission has left while it holds the lock: entries under D5's
/// bound, and time before the startup deadline it shares with
/// provisioning (M8).
#[derive(Debug)]
pub(super) struct Budget {
    pub(super) entries: u64,
    pub(super) deadline: Instant,
}

impl Budget {
    /// The whole bound, until `deadline`.
    pub(super) fn new(deadline: Instant) -> Budget {
        Budget {
            entries: ENTRIES_MAX,
            deadline,
        }
    }

    /// Whether the deadline is still ahead.
    pub(super) fn check(&self) -> Result<(), Unknown> {
        let ahead = Instant::now() < self.deadline;
        ahead.then_some(()).ok_or(Unknown::Expired)
    }

    /// One more entry read, or why none may be.
    pub(super) fn spend(&mut self) -> Result<(), Unknown> {
        self.check()?;
        self.entries = self.entries.checked_sub(1).ok_or(Unknown::Bounds)?;
        Ok(())
    }

    /// What is left before the deadline.
    pub(super) fn left(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
}

/// Everything the inventory holds: each run's reservation, and each
/// historical root.
#[derive(Debug, Default)]
pub(super) struct Found {
    pub(super) standing: Vec<Reservation>,
    pub(super) historical: Vec<PathBuf>,
}

/// Everything the protected `root` at `at` holds, read within `budget`:
/// the lock, and each worktree's directory ([`worktree`]).
pub(super) fn inventory(root: &OwnedFd, at: &Path, budget: &mut Budget) -> Result<Found, Unknown> {
    let mut found = Found::default();
    for (name, kind) in listed(root, at, budget)? {
        if (name == LOCK) & (kind == FileType::RegularFile) {
            continue;
        }
        let key = hex(&name).then_some(()).ok_or(unknown(at, &name));
        let repo = key.and_then(|()| entered(root, at, &name, kind))?;
        worktree(&repo, &at.join(&name), budget, &mut found)?;
    }
    Ok(found)
}

/// One worktree's directory `repo` at `at`: each historical root's record,
/// one staged beside it, and each run's directory ([`run`]).
fn worktree(
    repo: &OwnedFd,
    at: &Path,
    budget: &mut Budget,
    found: &mut Found,
) -> Result<(), Unknown> {
    for (name, kind) in listed(repo, at, budget)? {
        if kind == FileType::Directory {
            let dir = entered(repo, at, &name, kind)?;
            found.standing.extend(run(&dir, &at.join(&name), budget)?);
            continue;
        }
        let staged = name
            .strip_suffix(STAGED)
            .is_some_and(|name| named(name).is_some());
        match (named(&name), kind) {
            (Some(key), FileType::RegularFile) => {
                let record: Historical = read(repo, &name, at)?;
                let ours = named(&super::historical::name(&record.artifacts)) == Some(key);
                let absolute = record.artifacts.is_absolute();
                (ours & absolute).then_some(()).ok_or(unknown(at, &name))?;
                found.historical.push(record.artifacts);
            }
            (None, FileType::RegularFile) if staged => {}
            _ => return Err(unknown(at, &name)),
        }
    }
    Ok(())
}

/// The reservation standing in the run's directory `dir` at `at`, if one
/// does, which names that very worktree and run, and every attempt's
/// directory beside it: provisioning makes one only once admission sealed
/// its attempt in the reservation, so a directory it names neither
/// standing nor settled is unknown.
fn run(dir: &OwnedFd, at: &Path, budget: &mut Budget) -> Result<Option<Reservation>, Unknown> {
    let mut found: Option<Reservation> = None;
    let mut attempts = Vec::new();
    let (staged, prior) = (
        format!("{RESERVATION}{STAGED}"),
        format!("{RESERVATION}{PRIOR}"),
    );
    let mut kept = false;
    for (name, kind) in listed(dir, at, budget)? {
        match kind {
            FileType::RegularFile if name == RESERVATION => found = Some(read(dir, &name, at)?),
            FileType::RegularFile if name == staged => {}
            FileType::RegularFile if name == prior => kept = true,
            FileType::Directory => attempts.push(name),
            _ => return Err(unknown(at, &name)),
        }
    }
    // A prior name stands only beside the record a seal not yet final
    // replaced, which reads as that prior ([`read`]).
    if kept & found.is_none() {
        return Err(unknown(at, &prior));
    }
    let named = |name: &&String| {
        let names =
            |held: &Reservation| held.attempts.contains_key(*name) | held.settled.contains(*name);
        found.as_ref().is_some_and(names)
    };
    if let Some(stray) = attempts.iter().find(|name| !named(name)) {
        return Err(unknown(at, stray));
    }
    let names = |held: &Reservation| {
        let repo = at.parent().and_then(Path::file_name);
        (repo == Some(held.repo.as_ref())) & (at.file_name() == Some(held.run.as_ref()))
    };
    match found {
        Some(held) if !names(&held) => Err(Unknown::Participant(at.join(RESERVATION))),
        found => Ok(found),
    }
}

/// Every entry of the directory `dir` at `at` but `.` and `..`, by its
/// UTF-8 name and its type, each spent from `budget`.
fn listed(
    dir: &OwnedFd,
    at: &Path,
    budget: &mut Budget,
) -> Result<Vec<(String, FileType)>, Unknown> {
    let unread = || Unknown::Participant(at.to_path_buf());
    let mut entries = Vec::new();
    for entry in Dir::read_from(dir).or(Err(unread()))? {
        let entry = entry.or(Err(unread()))?;
        let name = entry.file_name().to_bytes();
        if (name == b".") | (name == b"..") {
            continue;
        }
        budget.spend()?;
        let name = std::str::from_utf8(name).or(Err(unread()))?.to_string();
        // Each entry's own type, never the listing's, which a filesystem
        // may leave unknown.
        let stat = statat(dir, name.as_str(), AtFlags::SYMLINK_NOFOLLOW);
        let stat = stat.or(Err(unknown(at, &name)))?;
        entries.push((name, FileType::from_raw_mode(stat.st_mode)));
    }
    Ok(entries)
}

/// The directory `name` of kind `kind` in `parent` at `at`, entered
/// without following a link: this user's alone, or an unknown participant.
fn entered(parent: &OwnedFd, at: &Path, name: &str, kind: FileType) -> Result<OwnedFd, Unknown> {
    let refused = || unknown(at, name);
    (kind == FileType::Directory)
        .then_some(())
        .ok_or_else(refused)?;
    let dir = openat(parent, name, directory(), Mode::empty()).or(Err(refused()))?;
    let stat = rustix::fs::fstat(&dir).or(Err(refused()))?;
    owned(&stat, hands::ids().0, true)
        .then_some(dir)
        .ok_or_else(refused)
}

/// The unknown participant `name` in the directory at `at`.
fn unknown(at: &Path, name: &str) -> Unknown {
    Unknown::Participant(at.join(name))
}

/// Whether `name` is a sha256's lowercase hex: a worktree's key, or an
/// artifact's name.
pub(super) fn hex(name: &str) -> bool {
    let digit = |byte: &u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte);
    (name.len() == 64) & name.as_bytes().iter().all(digit)
}
