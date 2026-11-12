//! The host's historical evidence roots and their retirement (decision
//! 0105 ruling 1(a)): what `brokkr evidence list` prints, and the one
//! operator act that retires a historical root once its evidence is
//! archived or discarded.
//!
//! Everything here is read or written under the admission lock, from the
//! coordination root the engine keeps under HOME. A retirement is a
//! sealed record beside the historical one, which stays; admission reads
//! it to decide whether a historical root still guards evidence
//! (decision 0105 rulings 2 and 3(a)). The archive inventory
//! `brokkr.evidence-archive.v1` is written last, and a digest that does
//! not match refuses the retirement with nothing written.

use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::admission::{beside, Conflict, Unadmitted, Unknown};
use super::exclusion::{covers, Guarded};
use super::historical::retired;
pub use super::historical::retired::{Cause, Retirement};
pub use super::historical::superseded::Superseded;
use super::historical::{self};
use super::inventory::{inventory, Budget, Found, HistoricalRoot, Reaching, Standing, Unread};
use super::root::{coordination, locked, locked_existing, Existing, Root};
use crate::engine::artifacts::{hex, DIGESTS};

use destination::Destination;

mod destination;
mod history;

#[cfg(test)]
pub(in crate::engine) use history::retirement_in;
pub use history::{retirements, RetirementHistory};

/// The one inventory format this build writes and reads (decision 0105,
/// minted under decision 0082). A change a previous build could not read
/// takes a new string, as `brokkr.journal-backup.v1` does.
pub const EVIDENCE_ARCHIVE_FORMAT: &str = "brokkr.evidence-archive.v1";

/// One historical root as `brokkr evidence list` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRoot {
    /// The artifact root the historical record names.
    pub artifacts: PathBuf,
    /// Whether anything stands at that path.
    pub exists: bool,
    /// How many digests its `sha256/` directory holds.
    pub digests: u64,
    /// Their total bytes.
    pub bytes: u64,
    /// Its retirement, where the operator retired it.
    pub retirement: Option<Retirement>,
    /// The retirements a later evidence-protecting admission superseded,
    /// oldest first (decision 0105, H1).
    pub history: Vec<Superseded>,
    /// Why this root's own counts could not be read, where they could not:
    /// the row is still printed, and says what is wrong with that root
    /// (decision 0105, M4).
    pub problem: Option<String>,
}

/// What the `brokkr.evidence-archive.v1` inventory holds: the root, every
/// digest and its bytes, the total, the engine's version and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceArchive {
    pub format: String,
    pub root: PathBuf,
    pub engine: String,
    pub archived_at: String,
    pub artifacts: Vec<ArchiveArtifact>,
    pub total_bytes: u64,
}

/// One artifact in an archive: its digest and its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveArtifact {
    pub sha256: String,
    pub bytes: u64,
}

/// What `brokkr evidence retire` is asked to do.
#[derive(Debug, Clone, Copy)]
pub struct RetireOptions<'a> {
    /// The artifact root the historical record names.
    pub artifacts: &'a Path,
    /// Archive the evidence, or discard it.
    pub action: RetireAction<'a>,
    /// The operator whose verb retires it, read by the CLI from USER.
    pub operator: &'a str,
    /// Why, in the operator's words.
    pub reason: &'a str,
    /// When, RFC 3339, passed in so a test can pin it.
    pub now: &'a str,
}

/// What a retirement does with the evidence at the root.
#[derive(Debug, Clone, Copy)]
pub enum RetireAction<'a> {
    /// Copy every artifact into `to`, an absent or empty directory, write
    /// the inventory last, and keep the root.
    Archived { to: &'a Path },
    /// Remove the root by a walk that follows no link and crosses no
    /// filesystem.
    Discarded,
}

/// Why a historical root could not be listed or retired, in the operator's
/// words (decision 0071 ruling 8). Each refusal leaves the host as it was,
/// and a retirement is written only once the archive or the discard is
/// complete.
#[derive(Debug, Error)]
pub enum EvidenceError {
    #[error("the capability coordination root is not protected")]
    Coordination,
    #[error("the admission lock is not this user's own single file")]
    Lock,
    #[error("evidence did not complete before its deadline")]
    Expired,
    #[error("the protected inventory exceeds its entry bound")]
    Bounds,
    #[error("the protected inventory holds an unknown participant at '{}'", .0.display())]
    Participant(PathBuf),
    /// A refusal whose typed cause is kept as its source, never flattened
    /// into words at the conversion (decision 0071 rulings 2 and 8; 0105,
    /// L2).
    #[error("{0}")]
    Refused(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "the coordination root is not protected: '{}' is not this user's alone",
        .0.display()
    )]
    Unprotected(PathBuf),
    /// A component on the way to the coordination root is a link, which a
    /// readout never follows; the path that stands there is named.
    #[error(
        "the coordination root's path '{}' is a link, which is never followed",
        .0.display()
    )]
    Linked(PathBuf),
    /// The root cannot be opened at all: only its absence is read as no
    /// root (decision 0105, the controller's S2 ruling on #1020).
    #[error("the coordination root '{}' cannot be read: {}", .0.display(), .1)]
    Unreadable(PathBuf, std::io::ErrorKind),
    #[error("the retirement of '{}' cannot be read", .0.display())]
    Retirement(PathBuf),
    #[error("no historical evidence root is recorded at '{}'", .0.display())]
    NotRecorded(PathBuf),
    /// `--archived-to` for a root that does not stand: `--discarded` is the
    /// record for an absent root (decision 0105 ruling 2; 1020b).
    #[error(
        "refused: the evidence root '{}' does not stand, so it cannot be archived; --discarded is \
         the record for an absent root",
        .0.display()
    )]
    Absent(PathBuf),
    /// The operator's reason is empty or whitespace: the record takes their
    /// words (decision 0105 ruling 1; 1020b).
    #[error(
        "refused: a retirement needs the operator's reason, and the one given is empty; nothing \
         was written"
    )]
    Reason,
    #[error("the evidence root '{}' is already retired", .0.display())]
    AlreadyRetired(PathBuf),
    #[error(
        "evidence root '{root}' is protected by run '{run}'s standing attempt",
        root = .root.display()
    )]
    Protected { root: PathBuf, run: String },
    #[error(
        "refused: the archive destination '{}' lies inside the coordination root; nothing was \
         written",
        .0.display()
    )]
    ArchiveInsideCoordination(PathBuf),
    #[error(
        "refused: the archive destination '{}' lies inside the evidence root being retired; \
         nothing was written",
        .0.display()
    )]
    ArchiveInsideRoot(PathBuf),
    #[error(
        "refused: '{}' already holds a file, so it is not an empty place for an archive; \
         nothing was written",
        .0.display()
    )]
    DestinationNotEmpty(PathBuf),
    #[error("refused: '{}' exists and is not a directory; nothing was written", .0.display())]
    DestinationNotDirectory(PathBuf),
    #[error(
        "refused: the archive destination '{}' names a component below a directory that does not \
         exist; nothing was written",
        .0.display()
    )]
    Unresolved(PathBuf),
    #[error(
        "refused: the archive destination '{}' does not name a UTF-8 path, so a retirement could \
         not record it; nothing was written",
        .0.display()
    )]
    Unnamed(PathBuf),
    #[error(
        "refused: '{}' is not the directory that was resolved; nothing was written",
        .0.display()
    )]
    Replaced(PathBuf),
    #[error(
        "the artifact '{}' is not one file of one link holding the bytes its name digests; \
         nothing was written",
        .0.display()
    )]
    Corrupt(PathBuf),
    #[error("the artifact '{}' is not one regular file named by a digest; nothing was written", .0.display())]
    Unfinished(PathBuf),
    /// `sha256/` is a link or is not a directory, which a readout never
    /// follows (decision 0105, M4; 1020b).
    #[error(
        "the digest directory '{}' is a link or is not a directory, which a readout never follows",
        .0.display()
    )]
    Digests(PathBuf),
    #[error("the evidence root '{}' is a link, which a discard never follows", .0.display())]
    Link(PathBuf),
    #[error("the evidence root '{}' is not a directory", .0.display())]
    NotDirectory(PathBuf),
    #[error(
        "the evidence root '{}' lies on another filesystem, which a discard never crosses",
        .0.display()
    )]
    OtherFilesystem(PathBuf),
    #[error("the device of '{}' cannot be read", .0.display())]
    Device(PathBuf),
    #[error("the evidence archive inventory is format '{0}', not '{EVIDENCE_ARCHIVE_FORMAT}'")]
    Format(String),
    #[error("the evidence archive inventory cannot be read: {0}")]
    Inventory(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<Unknown> for EvidenceError {
    fn from(unknown: Unknown) -> EvidenceError {
        match unknown {
            Unknown::Root => EvidenceError::Coordination,
            Unknown::Lock => EvidenceError::Lock,
            Unknown::Expired => EvidenceError::Expired,
            Unknown::Bounds => EvidenceError::Bounds,
            Unknown::Participant(at) => EvidenceError::Participant(at),
            other @ (Unknown::Worktree
            | Unknown::Overflow
            | Unknown::Formatted(..)
            | Unknown::Unsealed
            | Unknown::Unsynced
            | Unknown::Lost
            | Unknown::Unproven
            | Unknown::Orphaned { .. }
            | Unknown::Serving { .. }
            | Unknown::Unconfined { .. }
            | Unknown::Unmounted(..)
            | Unknown::Homeless
            | Unknown::Unrouted(..)) => EvidenceError::Refused(Box::new(other)),
        }
    }
}

impl From<Unread> for EvidenceError {
    fn from(unread: Unread) -> EvidenceError {
        match unread {
            Unread::Unknown(unknown) => unknown.into(),
            Unread::Retirement(at) | Unread::Superseded(at) => EvidenceError::Retirement(at),
        }
    }
}

/// A refusal `verified` makes as the archive reads the root: a corrupt
/// artifact keeps its own cause, and every other refusal is kept as its
/// typed source (decision 0071 rulings 2 and 8; 0105, L2).
fn unadmitted(refusal: Unadmitted) -> EvidenceError {
    match refusal {
        Unadmitted::Evidence(Conflict::Corrupt(at)) => EvidenceError::Corrupt(at),
        Unadmitted::Evidence(conflict) => EvidenceError::Refused(Box::new(conflict)),
        Unadmitted::Identity(unknown) => EvidenceError::Refused(Box::new(unknown)),
        Unadmitted::Unrooted(unrooted) => EvidenceError::Refused(Box::new(unrooted)),
    }
}

/// The coordination root under `home` where it stands, nothing written:
/// none where it is absent, and a named problem where it cannot be opened
/// or is not this user's alone (decision 0105, I2 and the controller's S2
/// ruling on #1020).
fn existing(home: &Path) -> Result<Option<Root>, EvidenceError> {
    match Root::existing(home) {
        Ok(root) => Ok(Some(root)),
        Err(Existing::Absent) => Ok(None),
        Err(Existing::NotAlone(at)) => Err(EvidenceError::Unprotected(at)),
        Err(Existing::Linked(at)) => Err(EvidenceError::Linked(at)),
        Err(Existing::Unreadable(kind)) => Err(EvidenceError::Unreadable(coordination(home), kind)),
    }
}

/// Every historical root the host holds, each with whether its artifact
/// root stands, its digest count and bytes, and its retirement; empty
/// where the coordination root does not stand. Read under the admission
/// lock, nothing written.
pub fn list(home: &Path, deadline: Instant) -> Result<Vec<EvidenceRoot>, EvidenceError> {
    match existing(home)? {
        Some(root) => list_in(&root, deadline),
        None => Ok(Vec::new()),
    }
}

/// [`list`] in an already opened coordination root.
pub(in crate::engine) fn list_in(
    root: &Root,
    deadline: Instant,
) -> Result<Vec<EvidenceRoot>, EvidenceError> {
    let Some(_lock) = locked_existing(root.dir(), deadline)? else {
        return Ok(Vec::new());
    };
    let mut budget = Budget::new(deadline);
    let found = inventory(root.dir(), root.path(), ["", ""], &mut budget)?;
    Ok(found.historical.iter().map(described).collect())
}

/// The evidence root type `held` describes, read without following a link;
/// a root whose own counts cannot be read still describes, carrying its
/// problem, so one bad root hides no other (decision 0105, M4).
fn described(held: &HistoricalRoot) -> EvidenceRoot {
    let history: Vec<Superseded> = held
        .history
        .iter()
        .map(|(_, record)| record.clone())
        .collect();
    let row = |exists, digests, bytes, problem| EvidenceRoot {
        artifacts: held.artifacts.clone(),
        exists,
        digests,
        bytes,
        retirement: held.retirement.clone(),
        history: history.clone(),
        problem,
    };
    match counts(&held.artifacts) {
        Ok((exists, digests, bytes)) => row(exists, digests, bytes, None),
        Err(refused) => row(
            std::fs::symlink_metadata(&held.artifacts).is_ok(),
            0,
            0,
            Some(refused.to_string()),
        ),
    }
}

/// Whether `root` stands, and its `sha256/` directory's count and total
/// bytes; a root on no filesystem is absent. Anything at a digest name
/// that is not one regular file refuses.
fn counts(root: &Path) -> Result<(bool, u64, u64), EvidenceError> {
    match std::fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((false, 0, 0)),
        Err(error) => return Err(error.into()),
        Ok(stat) if !stat.file_type().is_dir() => return Ok((true, 0, 0)),
        Ok(_) => {}
    }
    let digests = digests(root)?;
    let bytes = digests.iter().map(|(_, bytes)| *bytes).sum();
    Ok((true, digests.len() as u64, bytes))
}

/// Every digest in `root`'s `sha256/` directory and its bytes, sorted by
/// name; empty where the directory does not stand. `sha256/` is read
/// without following a link: a link, or anything that is not a directory,
/// is a problem on that root's row. A name that is not a digest, or an
/// entry that is not one regular file, refuses.
fn digests(root: &Path) -> Result<Vec<(String, u64)>, EvidenceError> {
    let at = root.join(DIGESTS);
    match std::fs::symlink_metadata(&at) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
        Ok(stat) if !stat.file_type().is_dir() => return Err(EvidenceError::Digests(at)),
        Ok(_) => {}
    }
    let entries = std::fs::read_dir(&at)?;
    let mut found = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| hex(name))
            .ok_or_else(|| EvidenceError::Unfinished(path.clone()))?;
        let stat = std::fs::symlink_metadata(&path)?;
        if !stat.file_type().is_file() {
            return Err(EvidenceError::Unfinished(path));
        }
        found.push((name.to_string(), stat.len()));
    }
    found.sort();
    Ok(found)
}

/// Retire the historical root `options.artifacts`, after archiving or
/// discarding its evidence, under the admission lock. Nothing is written
/// until every check passed, and the retirement is sealed last.
pub fn retire(
    home: &Path,
    options: RetireOptions<'_>,
    deadline: Instant,
) -> Result<Retirement, EvidenceError> {
    // The reason is the record's own words: check it before the protected
    // walk makes a directory, so the refusal wrote nothing (0105, 1020b).
    required_reason(options.reason)?;
    let root = Root::of(home).ok_or(EvidenceError::Coordination)?;
    retire_in(&root, options, deadline)
}

/// [`retire`] in an already opened coordination root.
pub(in crate::engine) fn retire_in(
    root: &Root,
    options: RetireOptions<'_>,
    deadline: Instant,
) -> Result<Retirement, EvidenceError> {
    retire_in_with(root, options, deadline, &|| {})
}

/// [`retire_in`], `between` run after the root's artifacts are verified and
/// its `sha256/` is made, before they are copied: the house `_with` seam a
/// test changes a source under, so the copy's own digest check is the one
/// that fails, or plants a name under the destination, so nothing this verb
/// did not write is removed; nothing in production.
pub(in crate::engine) fn retire_in_with(
    root: &Root,
    options: RetireOptions<'_>,
    deadline: Instant,
    between: &dyn Fn(),
) -> Result<Retirement, EvidenceError> {
    required_reason(options.reason)?;
    let _lock = locked(root.dir(), deadline)?;
    let mut budget = Budget::new(deadline);
    let found = inventory(root.dir(), root.path(), ["", ""], &mut budget)?;
    let held = find(&found, options.artifacts)
        .ok_or_else(|| EvidenceError::NotRecorded(options.artifacts.to_path_buf()))?;
    if held.retirement.is_some() {
        return Err(EvidenceError::AlreadyRetired(held.artifacts.clone()));
    }
    if matches!(options.action, RetireAction::Archived { .. }) {
        archived_root(&held.artifacts)?;
    }
    protected(&found.standing, &held.artifacts)?;
    let destination = destination(root.path(), &held.artifacts, options.action)?;
    let retirement = match &destination {
        Some(to) => archived(&held.artifacts, to, &mut budget, between, options)?,
        None => {
            discard_with(&held.artifacts, &device_of)?;
            retirement_of(&held.artifacts, options, Cause::Discarded, None, None)
        }
    };
    committed(root, &held.repo, retirement, &budget)
}

/// The operator's reason, which a retirement records in their words: an
/// empty or whitespace-only one refuses before anything is made or written
/// (decision 0105 ruling 1; 1020b).
fn required_reason(reason: &str) -> Result<(), EvidenceError> {
    match reason.trim().is_empty() {
        true => Err(EvidenceError::Reason),
        false => Ok(()),
    }
}

/// Whether the artifact root `artifacts` stands, by a stat that follows a
/// link: a root that does not stand cannot be archived, and a dangling
/// link does not stand, where `--discarded` is the record for an absent
/// root (decision 0105 ruling 2; 1020b).
fn archived_root(artifacts: &Path) -> Result<(), EvidenceError> {
    match std::fs::metadata(artifacts) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(EvidenceError::Absent(artifacts.to_path_buf()))
        }
        Err(error) => Err(error.into()),
        Ok(_) => Ok(()),
    }
}

/// Where an archived retirement writes, in the one place its resolution
/// allows: the resolved parent alone, compared with both roots before a
/// byte is written (decision 0105, the controller's S1 ruling on #1020).
fn destination(
    coordination: &Path,
    artifacts: &Path,
    action: RetireAction<'_>,
) -> Result<Option<Destination>, EvidenceError> {
    match action {
        RetireAction::Archived { to } => Ok(Some(Destination::open(to, coordination, artifacts)?)),
        RetireAction::Discarded => Ok(None),
    }
}

/// `retirement` sealed beside the historical record of the worktree `repo`:
/// the record every later read finds (decision 0105 ruling 3(a)). A
/// directory sync the seal could not prove makes the record unsynced, which
/// `list` shows as the retirement it reads; the verb reports the record it
/// wrote.
fn committed(
    root: &Root,
    repo: &str,
    retirement: Retirement,
    budget: &Budget,
) -> Result<Retirement, EvidenceError> {
    let dir = root.below(&[repo])?;
    let bytes = serde_json::to_vec(&retirement)?;
    let name = retired::name(&historical::key(&retirement.artifacts));
    let _commit = root.seal(&dir, &name, &bytes, budget)?;
    Ok(retirement)
}

/// The historical root `artifacts` in the inventory, matched as admission
/// records and inspect finds it: by the path as recorded, or through
/// [`beside`] so a root named by a symlinked worktree is the same root
/// (decision 0105, the controller's R2 ruling on #1020; 1020b).
fn find<'f>(found: &'f Found, artifacts: &Path) -> Option<&'f HistoricalRoot> {
    let by = |at: &Path| found.historical.iter().find(|held| held.artifacts == at);
    by(artifacts).or_else(|| by(&beside(artifacts)))
}

/// Whether a standing attempt protects `artifacts`, by its reservation's
/// artifact root or by a writer that reaches it, as admission compares a
/// writer with a guarded root: refused naming the run.
fn protected(standing: &[Standing], artifacts: &Path) -> Result<(), EvidenceError> {
    let blocked = |run: &Standing| EvidenceError::Protected {
        root: artifacts.to_path_buf(),
        run: run.run.clone(),
    };
    let root = Guarded::of(artifacts.to_path_buf());
    let reaching = |held: &Standing| {
        held.writers().any(|writer| match &writer.reach {
            Reaching::Unbounded => true,
            Reaching::Boxed { writable, .. } => {
                writable.iter().any(|writable| covers(writable, &root))
            }
        })
    };
    for held in standing {
        let holds = !held.attempts.is_empty() && held.artifacts == artifacts;
        if holds | reaching(held) {
            return Err(blocked(held));
        }
    }
    Ok(())
}

/// The retirement for `root` with the operator's words.
fn retirement_of(
    root: &Path,
    options: RetireOptions<'_>,
    cause: Cause,
    archive: Option<PathBuf>,
    inventory_sha256: Option<String>,
) -> Retirement {
    Retirement {
        artifacts: root.to_path_buf(),
        cause,
        archive,
        inventory_sha256,
        operator: options.operator.to_string(),
        reason: options.reason.to_string(),
        retired_at: options.now.to_string(),
        engine: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Retire `root` as archived into `to`: every artifact verified, copied,
/// verified there, and the inventory written last; answers the retirement
/// naming the resolved destination. A failure removes everything the copy
/// wrote and names the artifact (decision 0105 ruling 2, the controller's
/// S1 ruling on #1020).
fn archived(
    root: &Path,
    to: &Destination,
    budget: &mut Budget,
    between: &dyn Fn(),
    options: RetireOptions<'_>,
) -> Result<Retirement, EvidenceError> {
    let sha256 = to.archive(root, budget, between, options.now)?;
    Ok(retirement_of(
        root,
        options,
        Cause::Archived,
        Some(to.at.clone()),
        Some(sha256),
    ))
}

/// `dir` synced, so a name published in it survives a power loss. This
/// mirrors `brokkr-store`'s backup `sync_dir`, which is private to that
/// crate: macOS answers `EINVAL` for a directory fsync, where the name
/// still landed and there is no stronger call to make, so that one answer
/// is not a refusal (decision 0105, M2; decision 0100).
fn sync_dir(dir: &OwnedFd) -> Result<(), EvidenceError> {
    match rustix::fs::fsync(dir) {
        Ok(()) | Err(rustix::io::Errno::INVAL) => Ok(()),
        Err(error) => Err(std::io::Error::from(error).into()),
    }
}

/// [`discard`] with the device of `path` read by `device`: the house
/// `_with` seam a test shows another filesystem with, nothing in
/// production.
pub(in crate::engine) fn discard_with(
    root: &Path,
    device: &dyn Fn(&Path) -> Option<u64>,
) -> Result<(), EvidenceError> {
    let stat = match std::fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(stat) => stat,
    };
    let refused = || EvidenceError::OtherFilesystem(root.to_path_buf());
    if stat.file_type().is_symlink() {
        return Err(EvidenceError::Link(root.to_path_buf()));
    }
    if !stat.file_type().is_dir() {
        return Err(EvidenceError::NotDirectory(root.to_path_buf()));
    }
    let own = device(root).ok_or_else(|| EvidenceError::Device(root.to_path_buf()))?;
    let parent = root.parent().and_then(device);
    parent.filter(|parent| *parent == own).ok_or_else(refused)?;
    validated(root, own, device)?;
    removed_tree(root)
}

/// Whether every entry below `dir` lies on `own`'s filesystem, none of
/// them followed: a link is removed, never entered.
fn validated(
    dir: &Path,
    own: u64,
    device: &dyn Fn(&Path) -> Option<u64>,
) -> Result<(), EvidenceError> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let stat = std::fs::symlink_metadata(&path)?;
        if stat.file_type().is_symlink() {
            continue;
        }
        let at = device(&path).ok_or_else(|| EvidenceError::Device(path.clone()))?;
        if at != own {
            return Err(EvidenceError::OtherFilesystem(path));
        }
        if stat.file_type().is_dir() {
            validated(&path, own, device)?;
        }
    }
    Ok(())
}

/// `dir` removed: every entry below it bottom-up, a link removed as a
/// link, then the directory itself.
fn removed_tree(dir: &Path) -> Result<(), EvidenceError> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let stat = std::fs::symlink_metadata(&path)?;
        if stat.file_type().is_dir() {
            removed_tree(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    std::fs::remove_dir(dir)?;
    Ok(())
}

/// The device `path` lies on, by the link itself where it is one.
fn device_of(path: &Path) -> Option<u64> {
    std::fs::symlink_metadata(path).ok().map(|stat| stat.dev())
}

/// The inventory `bytes` read as this build's one archive format: a
/// present inventory that is another format refuses by name, never read as
/// absent.
pub fn read_archive(bytes: &[u8]) -> Result<EvidenceArchive, EvidenceError> {
    let inventory: EvidenceArchive = serde_json::from_slice(bytes)?;
    if inventory.format != EVIDENCE_ARCHIVE_FORMAT {
        return Err(EvidenceError::Format(inventory.format));
    }
    Ok(inventory)
}

#[cfg(test)]
mod tests;
