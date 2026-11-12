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
//! cause. This run's own reservation is read whole; another run's, through
//! the view every build writes ([`standing`]; decision 0090).

use std::ffi::OsStr;
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brokkr_protocol::hands;
use rustix::fs::{openat, statat, AtFlags, Dir, FileType, Mode};

use super::admission::{Conflict, Unadmitted, Unknown, LOCK, RESERVATION, STAGED};
use super::historical::retired::{self, Retirement};
use super::historical::superseded::{self, Superseded};
use super::historical::{named, Historical};
use super::root::{bytes, read, PRIOR};
use super::{directory, owned};
use crate::engine::artifacts::hex;
use standing::standing;
pub(super) use standing::{Reaching, Standing, Writing};

mod standing;

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

/// One historical root the inventory holds: the worktree's directory it
/// was recorded in, the artifact root it names, and its retirement where
/// the operator retired it (decision 0105 ruling 3(a)).
#[derive(Debug, Clone)]
pub(super) struct HistoricalRoot {
    pub(super) repo: String,
    pub(super) artifacts: PathBuf,
    pub(super) retirement: Option<Retirement>,
    /// The retirements a later evidence-protecting admission superseded,
    /// by sequence, oldest first (decision 0105, H1).
    pub(super) history: Vec<(u64, Superseded)>,
}

/// Why the inventory could not be read: an unknown participant or a spent
/// bound (MB3), or a retirement that does not read, which refuses with
/// CR2's cause because admitting it either way would be a guess
/// (decision 0105 ruling 3(a)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Unread {
    Unknown(Unknown),
    Retirement(PathBuf),
    /// A superseded retirement that does not read refuses as a retirement
    /// does: never read as absent history (decision 0105, H1).
    Superseded(PathBuf),
}

impl From<Unknown> for Unread {
    fn from(unknown: Unknown) -> Unread {
        Unread::Unknown(unknown)
    }
}

impl From<Unread> for Unadmitted {
    fn from(unread: Unread) -> Unadmitted {
        match unread {
            Unread::Unknown(unknown) => Unadmitted::Identity(unknown),
            Unread::Retirement(at) | Unread::Superseded(at) => {
                Unadmitted::Evidence(Conflict::Retirement(at))
            }
        }
    }
}

/// Everything the inventory holds: each run's reservation, and each
/// historical root.
#[derive(Debug, Default)]
pub(super) struct Found {
    pub(super) standing: Vec<Standing>,
    pub(super) historical: Vec<HistoricalRoot>,
}

impl Found {
    /// The historical roots that still guard evidence: every one the
    /// operator has not retired (decision 0105 ruling 2).
    pub(super) fn records(&self, artifacts: &Path) -> bool {
        self.historical
            .iter()
            .any(|root| root.artifacts == artifacts)
    }

    pub(super) fn guarded(&self) -> Vec<PathBuf> {
        self.historical
            .iter()
            .filter(|root| root.retirement.is_none())
            .map(|root| root.artifacts.clone())
            .collect()
    }
}

/// Everything the protected `root` at `at` holds, read within `budget`:
/// the lock, and each worktree's directory ([`worktree`]), the run `own`
/// names by its worktree's key and its id read whole.
pub(super) fn inventory(
    root: &OwnedFd,
    at: &Path,
    own: [&str; 2],
    budget: &mut Budget,
) -> Result<Found, Unread> {
    let mut found = Found::default();
    for (name, kind) in listed(root, at, budget)? {
        if (name == LOCK) & (kind == FileType::RegularFile) {
            continue;
        }
        let key = hex(&name).then_some(()).ok_or(unknown(at, &name));
        let repo = key.and_then(|()| entered(root, at, &name, kind))?;
        worktree(&repo, &at.join(&name), own, budget, &mut found)?;
    }
    Ok(found)
}

/// One worktree's directory `repo` at `at`: each historical root's record,
/// its retirement and superseded retirements where they were sealed, one
/// staged beside any, and each run's directory ([`run`]), the one `own`
/// names its run's own.
fn worktree(
    repo: &OwnedFd,
    at: &Path,
    [key, id]: [&str; 2],
    budget: &mut Budget,
    found: &mut Found,
) -> Result<(), Unread> {
    let repo_name = at
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let held = collected(repo, at, [key, id], budget, &repo_name, found)?;
    found
        .historical
        .extend(attached(held)?.into_iter().map(|(_, held)| held));
    Ok(())
}

/// The records, retirements and superseded retirements one worktree's
/// directory holds, as its entries name them.
#[derive(Debug, Default)]
struct Collected {
    records: Vec<(String, HistoricalRoot)>,
    retirements: Vec<(String, PathBuf, Retirement)>,
    superseded: Vec<(String, u64, PathBuf, Superseded)>,
}

/// [`worktree`]'s one pass over the entries of `repo` at `at`: every run's
/// reservation into `found`, and every record here into the `Collected`
/// returned.
fn collected(
    repo: &OwnedFd,
    at: &Path,
    [key, id]: [&str; 2],
    budget: &mut Budget,
    repo_name: &str,
    found: &mut Found,
) -> Result<Collected, Unread> {
    let mut held = Collected::default();
    for (name, kind) in listed(repo, at, budget)? {
        if kind == FileType::Directory {
            let dir = entered(repo, at, &name, kind)?;
            let own = (at.file_name() == Some(OsStr::new(key))) & (name == id);
            found
                .standing
                .extend(run(&dir, &at.join(&name), own, budget)?);
            continue;
        }
        classified(repo, at, &name, kind, repo_name, &mut held)?;
    }
    Ok(held)
}

/// Whether `name` is one record a seal staged and did not rename in.
fn staged(name: &str) -> bool {
    name.strip_suffix(STAGED).is_some_and(|name| {
        named(name).is_some() | retired::named(name).is_some() | superseded::named(name).is_some()
    })
}

/// The one entry `name` of kind `kind` in `repo` at `at`, put where its
/// shape names it: a historical record, a retirement, a superseded one, or
/// a staged copy passed over. Anything else is an unknown participant.
fn classified(
    repo: &OwnedFd,
    at: &Path,
    name: &str,
    kind: FileType,
    repo_name: &str,
    held: &mut Collected,
) -> Result<(), Unread> {
    match (
        named(name),
        retired::named(name),
        superseded::named(name),
        kind,
    ) {
        (Some(key), None, None, FileType::RegularFile) => {
            record(repo, at, name, key, repo_name, held)
        }
        (None, Some(key), None, FileType::RegularFile) => retired_at(repo, at, name, key, held),
        (None, None, Some((key, n)), FileType::RegularFile) => {
            superseded_at(repo, at, name, (key, n), held)
        }
        (None, None, None, FileType::RegularFile) if staged(name) => Ok(()),
        _ => Err(Unread::Unknown(unknown(at, name))),
    }
}

/// The historical record `name` in `repo` at `at`, for the key it carries
/// and the worktree `repo_name`, into `held`; an unknown participant where
/// its artifact root is not the one its name digests, or is not absolute.
fn record(
    repo: &OwnedFd,
    at: &Path,
    name: &str,
    key: &str,
    repo_name: &str,
    held: &mut Collected,
) -> Result<(), Unread> {
    let record: Historical = read(repo, name, at)?;
    let ours = named(&super::historical::name(&record.artifacts)) == Some(key);
    let absolute = record.artifacts.is_absolute();
    (ours & absolute).then_some(()).ok_or(unknown(at, name))?;
    held.records.push((
        key.to_string(),
        HistoricalRoot {
            repo: repo_name.to_string(),
            artifacts: record.artifacts,
            retirement: None,
            history: Vec::new(),
        },
    ));
    Ok(())
}

/// The retirement `name` in `repo` at `at`, for the key it carries, into
/// `held`.
fn retired_at(
    repo: &OwnedFd,
    at: &Path,
    name: &str,
    key: &str,
    held: &mut Collected,
) -> Result<(), Unread> {
    let refused = || Unread::Retirement(at.join(name));
    let read = bytes(repo, name, at).map_err(|_| refused())?;
    let retirement = serde_json::from_slice(&read).map_err(|_| refused())?;
    held.retirements
        .push((key.to_string(), at.join(name), retirement));
    Ok(())
}

/// The superseded retirement `name` in `repo` at `at`, for the key and
/// sequence it carries, into `held`.
fn superseded_at(
    repo: &OwnedFd,
    at: &Path,
    name: &str,
    (key, n): (&str, u64),
    held: &mut Collected,
) -> Result<(), Unread> {
    let refused = || Unread::Superseded(at.join(name));
    let read = bytes(repo, name, at).map_err(|_| refused())?;
    let record = serde_json::from_slice(&read).map_err(|_| refused())?;
    held.superseded
        .push((key.to_string(), n, at.join(name), record));
    Ok(())
}

/// `held`'s records with every retirement and superseded retirement
/// attached to the historical record it names, its history by sequence;
/// a retirement that names no record, or another root, refuses.
fn attached(held: Collected) -> Result<Vec<(String, HistoricalRoot)>, Unread> {
    let Collected {
        mut records,
        retirements,
        superseded,
    } = held;
    for (key, at, retirement) in retirements {
        let index = records
            .iter()
            .position(|(held, _)| *held == key)
            .ok_or_else(|| Unread::Retirement(at.clone()))?;
        let same = records[index].1.artifacts == retirement.artifacts;
        same.then_some(()).ok_or(Unread::Retirement(at))?;
        records[index].1.retirement = Some(retirement);
    }
    for (key, n, at, record) in superseded {
        let index = records
            .iter()
            .position(|(held, _)| *held == key)
            .ok_or_else(|| Unread::Superseded(at.clone()))?;
        let same = records[index].1.artifacts == record.retirement.artifacts;
        same.then_some(()).ok_or(Unread::Superseded(at))?;
        records[index].1.history.push((n, record));
    }
    for (_, held) in records.iter_mut() {
        held.history.sort_by_key(|(n, _)| *n);
    }
    Ok(records)
}

/// The reservation standing in the run's directory `dir` at `at`, if one
/// does, which names that very worktree and run, and every attempt's
/// directory beside it: provisioning makes one only once admission sealed
/// its attempt in the reservation, so a directory it names neither
/// standing nor settled is unknown. The reservation is read whole where it
/// is the run's `own`.
fn run(
    dir: &OwnedFd,
    at: &Path,
    own: bool,
    budget: &mut Budget,
) -> Result<Option<Standing>, Unknown> {
    let mut found: Option<Standing> = None;
    let mut attempts = Vec::new();
    let (staged, prior) = (
        format!("{RESERVATION}{STAGED}"),
        format!("{RESERVATION}{PRIOR}"),
    );
    let mut kept = false;
    for (name, kind) in listed(dir, at, budget)? {
        match kind {
            FileType::RegularFile if name == RESERVATION => found = Some(standing(dir, at, own)?),
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
            |held: &Standing| held.attempts.contains_key(*name) | held.settled.contains(*name);
        found.as_ref().is_some_and(names)
    };
    if let Some(stray) = attempts.iter().find(|name| !named(name)) {
        return Err(unknown(at, stray));
    }
    let names = |held: &Standing| {
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
