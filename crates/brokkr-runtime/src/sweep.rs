//! The sweep (decision 0104 rulings 7 to 9, as R3 to R6 and the operator's
//! fail-closed ruling amend them): one pass removes each run-local seat
//! scratch tree the journal proves its own driver let go, runs the hands
//! reaper, and records every complete removal in `tree_sweeps`. A tree is
//! kept, and reported by name, whenever its attempt ended indeterminate,
//! was closed by a conclude or a restarted engine, or the journal cannot
//! prove the driver ended it. A dry run reads the same verdicts and counts
//! what a pass would free, writing nothing. Nothing here prints: a
//! [`Swept`] names every tree it removed, kept or could not remove, and
//! the CLI paints it.

use std::path::{Path, PathBuf};

use brokkr_protocol::hands;
use brokkr_protocol::tree::{self, TreeError};
use brokkr_store::{SeatTree, Store, StoreError, Swept as Closed, TreeSweepRecord};
use thiserror::Error;

use crate::dispatch::Dispatcher;

mod eligibility;

use eligibility::Verdict;

/// The component every run-local scratch root ends in, as the engine
/// spells it.
const SCRATCH: &str = "scratch";

/// The words a queue that does not record sweeps is refused with.
const UNRECORDED: &str = "the journal's queue is not one this binary writes tree sweeps in";

/// Why a tree was kept, in the closed vocabulary the report prints. The
/// words never claim more than the journal shows: no reason says a process
/// may still hold the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeptReason {
    /// The effect closed only because the run is terminal, and an
    /// `effect/indeterminate` ended the attempt.
    Indeterminate,
    /// The terminal event is caused by the `operator/accepted` of a stop
    /// command, which only `close_riding_attempt` writes.
    Concluded,
    /// The terminal event is caused by one of the attempt's checkpoints,
    /// which a restarted engine's broker tallies append before it ends the
    /// attempt.
    Restarted,
    EffectOpen,
    NoEventNamesAttempt,
    NoTerminalEvent,
    AnotherRoot,
    RootNotAbsolute,
    AttemptNotEngineUuid,
    RootNotCanonical,
    RootUnreadable,
    TreeIsLink,
    TreeNotDirectory,
    TreeUnreadable,
    ForeignFilesystem,
    RunCannotBeRead,
    RunDoesNotFold,
    TmpdirRelative,
    TmpdirNotUtf8,
    /// The hands reaper kept the tree for its own reason, whose words are
    /// the reaper's (a lock it could not probe, a walk it could not do).
    Hands(String),
}

impl std::fmt::Display for KeptReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let words = match self {
            KeptReason::Indeterminate => "it ended indeterminate",
            KeptReason::Concluded => "its attempt was closed by a conclude",
            KeptReason::Restarted => "its attempt was closed by a restarted engine",
            KeptReason::EffectOpen => "its effect is still open",
            KeptReason::NoEventNamesAttempt => "no event names its attempt",
            KeptReason::NoTerminalEvent => "no terminal event names its attempt",
            KeptReason::AnotherRoot => "another row of its run names a different root",
            KeptReason::RootNotAbsolute => "its root is not an absolute path ending in 'scratch'",
            KeptReason::AttemptNotEngineUuid => "its attempt id is not the one the engine mints",
            KeptReason::RootNotCanonical => "its root is not its own canonical path",
            KeptReason::RootUnreadable => "its root cannot be read",
            KeptReason::TreeIsLink => "its tree is a link, not a directory",
            KeptReason::TreeNotDirectory => "its tree is not a directory",
            KeptReason::TreeUnreadable => "its tree cannot be read",
            KeptReason::ForeignFilesystem => "its tree is on another filesystem than its root",
            KeptReason::RunCannotBeRead => "its run cannot be read",
            KeptReason::RunDoesNotFold => "its run does not fold",
            KeptReason::TmpdirRelative => "its TMPDIR is not an absolute path",
            KeptReason::TmpdirNotUtf8 => "its TMPDIR is not UTF-8",
            KeptReason::Hands(words) => words,
        };
        f.write_str(words)
    }
}

/// Why one tree a pass could not remove stayed: the walk refused it, or
/// the removal completed and its sweep row did not. The record refusal
/// sits beside the removal it belongs to, so a report never hides a tree
/// whose bytes were freed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FailedCause {
    Walk(String),
    Record(String),
}

impl std::fmt::Display for FailedCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailedCause::Walk(cause) | FailedCause::Record(cause) => f.write_str(cause),
        }
    }
}

/// One tree a pass removed, or a dry run would remove. `bytes` is `None`
/// when the tree was gone before the pass; `lower_bound` says a dry run
/// could not read every directory, so `bytes` is the least it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Removed {
    pub(crate) path: PathBuf,
    pub(crate) bytes: Option<u64>,
    pub(crate) lower_bound: bool,
}

/// One tree kept, with the reason the journal gave. The reason names no
/// path: the line names the tree once, through the renderer's `safe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Kept {
    pub(crate) path: PathBuf,
    pub(crate) reason: KeptReason,
}

/// One tree a pass could not remove, with the walk's or the record's
/// cause. The cause names no path, for the same reason [`Kept`] does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Failed {
    pub(crate) path: PathBuf,
    pub(crate) cause: FailedCause,
}

/// What one pass did, or what a dry run would do.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Swept {
    removed: Vec<Removed>,
    kept: Vec<Kept>,
    failed: Vec<Failed>,
    /// Why the hands half removed nothing, where its `TMPDIR` is not an
    /// absolute, UTF-8 path.
    hands: Option<KeptReason>,
}

/// A sweep refused before it could report: the journal could not be read,
/// or a removed tree could not be recorded. A record refusal carries the
/// report of every removal the pass made, so the CLI paints them and
/// still exits non-zero.
#[derive(Debug, Error)]
pub enum SweepError {
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The dispatcher handed to [`pass`] took another journal, so nothing
    /// was read or written.
    #[error(transparent)]
    Dispatcher(#[from] crate::dispatch::DispatcherError),
    /// The removal completed but its sweep row could not be written.
    #[error("a tree was removed but its sweep could not be recorded: {cause}")]
    Record { report: Box<Swept>, cause: String },
}

impl Swept {
    /// The trees the pass removed, or the dry run would remove.
    #[cfg(test)]
    pub(crate) fn removed(&self) -> &[Removed] {
        &self.removed
    }

    /// The trees the journal kept, and why.
    #[cfg(test)]
    pub(crate) fn kept(&self) -> &[Kept] {
        &self.kept
    }

    /// The trees the walk could not remove, or whose sweep row it could
    /// not write.
    #[cfg(test)]
    pub(crate) fn failed(&self) -> &[Failed] {
        &self.failed
    }

    /// The bytes the pass freed, or the dry run would free.
    pub(crate) fn freed(&self) -> u64 {
        self.removed
            .iter()
            .filter_map(|removed| removed.bytes)
            .sum()
    }

    /// One line per tree, each path through `safe`, then the total. A dry
    /// run says what it would do and writes nothing. A total is a lower
    /// bound whenever any of its trees is.
    pub fn lines_with(&self, safe: fn(&str) -> String, dry_run: bool) -> String {
        let mut said = String::new();
        let (verb, gone) = match dry_run {
            true => ("would remove", "it is already gone"),
            false => ("removed", "it was already gone"),
        };
        for removed in &self.removed {
            let path = safe(&removed.path.display().to_string());
            let outcome = match (removed.bytes, removed.lower_bound) {
                (Some(bytes), true) => format!("at least {bytes} bytes"),
                (Some(bytes), false) => format!("{bytes} bytes"),
                (None, _) => gone.to_string(),
            };
            said.push_str(&format!("sweep: {verb} {path}: {outcome}\n"));
        }
        for kept in &self.kept {
            let path = safe(&kept.path.display().to_string());
            let reason = safe(&kept.reason.to_string());
            said.push_str(&format!("sweep: kept {path}: {reason}\n"));
        }
        for failed in &self.failed {
            let path = safe(&failed.path.display().to_string());
            let cause = safe(&failed.cause.to_string());
            let verb = match failed.cause {
                FailedCause::Walk(_) => "could not remove",
                FailedCause::Record(_) => "could not record",
            };
            said.push_str(&format!("sweep: {verb} {path}: {cause}\n"));
        }
        if let Some(ref reason) = self.hands {
            said.push_str(&format!("sweep: hands trees kept: {reason}\n"));
        }
        let verb = match dry_run {
            true => "would free",
            false => "freed",
        };
        let least = match self.removed.iter().any(|removed| removed.lower_bound) {
            true => "at least ",
            false => "",
        };
        said.push_str(&format!("sweep: {verb} {least}{} bytes\n", self.freed()));
        said
    }
}

/// Run one pass under `store`'s dispatch lock. `held` is the proof of the
/// lock: slice 6's dispatcher hands its own, and each removal records its
/// generation.
pub fn pass(store: &mut Store, held: &Dispatcher) -> Result<Swept, SweepError> {
    pass_with(store, held, same_device, tree::remove_tree)
}

/// [`pass`], with the walk and the device check a test drives: the seams
/// 5c1's removal walk has, so a foreign device and a partial removal are
/// provable on one host.
fn pass_with(
    store: &mut Store,
    held: &Dispatcher,
    device_of: impl Fn(&Path, u64) -> u64,
    remove: impl Fn(&Path) -> Result<Option<u64>, TreeError>,
) -> Result<Swept, SweepError> {
    held.took(store.journal()?)?;
    let verdicts = eligibility::decide(store, device_of)?;
    let mut swept = Swept::default();
    let mut refusal = None;
    for verdict in verdicts {
        let Some((row, path)) = keeps(&mut swept, verdict) else {
            continue;
        };
        let bytes = match remove(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                swept.failed.push(Failed {
                    path,
                    cause: FailedCause::Walk(cause(&error)),
                });
                continue;
            }
        };
        if let Err(cause) = record(
            store,
            Closed::Scratch(row.seq),
            &path,
            bytes,
            held.generation.0,
        ) {
            swept.removed.push(Removed {
                path: path.clone(),
                bytes,
                lower_bound: false,
            });
            swept.failed.push(Failed {
                path,
                cause: FailedCause::Record(cause.clone()),
            });
            refusal = Some(cause);
            break;
        }
        swept.removed.push(Removed {
            path,
            bytes,
            lower_bound: false,
        });
    }
    if refusal.is_none() {
        pass_hands(store, held, &mut swept, &mut refusal)?;
    }
    match refusal {
        Some(cause) => Err(SweepError::Record {
            report: Box::new(swept),
            cause,
        }),
        None => Ok(swept),
    }
}

/// Run the hands reaper and record each dead owner's free tree it removes.
/// A `TMPDIR` that is not absolute or not UTF-8 refuses the half before
/// the reaper reads it, so a relative tree is never removed of the host's.
fn pass_hands(
    store: &mut Store,
    held: &Dispatcher,
    swept: &mut Swept,
    refusal: &mut Option<String>,
) -> Result<(), SweepError> {
    if let Err(reason) = hands_tmp() {
        swept.hands = Some(reason);
        return Ok(());
    }
    let reaped = hands::reap_dead_sessions();
    for (path, bytes) in reaped.removed() {
        if let Err(cause) = record(store, Closed::Hands, path, Some(bytes), held.generation.0) {
            swept.removed.push(Removed {
                path: path.to_path_buf(),
                bytes: Some(bytes),
                lower_bound: false,
            });
            swept.failed.push(Failed {
                path: path.to_path_buf(),
                cause: FailedCause::Record(cause.clone()),
            });
            *refusal = Some(cause);
            return Ok(());
        }
        swept.removed.push(Removed {
            path: path.to_path_buf(),
            bytes: Some(bytes),
            lower_bound: false,
        });
    }
    for (path, reason) in reaped.kept() {
        swept.kept.push(Kept {
            path: path.to_path_buf(),
            reason: KeptReason::Hands(reason),
        });
    }
    Ok(())
}

/// What a pass would remove, with the bytes it would free, writing
/// nothing (R3): no row, no holder and no migration.
pub fn plan(store: &Store) -> Result<Swept, SweepError> {
    plan_with(store, hands_tmp(), same_device, tree::count_tree)
}

/// [`plan`], with the same seams the pass has.
fn plan_with(
    store: &Store,
    tmp: Result<PathBuf, KeptReason>,
    device_of: impl Fn(&Path, u64) -> u64,
    count: impl Fn(&Path) -> Result<Option<tree::Counted>, TreeError>,
) -> Result<Swept, SweepError> {
    let verdicts = eligibility::decide(store, device_of)?;
    let mut swept = Swept::default();
    for verdict in verdicts {
        let Some((_row, path)) = keeps(&mut swept, verdict) else {
            continue;
        };
        match count(&path) {
            Ok(counted) => swept.removed.push(removed(path, counted)),
            Err(error) => swept.failed.push(Failed {
                path,
                cause: FailedCause::Walk(cause(&error)),
            }),
        }
    }
    match tmp {
        Ok(tmp) => plan_hands(&tmp, &mut swept),
        Err(reason) => swept.hands = Some(reason),
    }
    Ok(swept)
}

/// One planned removal, saying whether its count is a lower bound.
fn removed(path: PathBuf, counted: Option<tree::Counted>) -> Removed {
    match counted {
        Some(counted) => Removed {
            path,
            bytes: Some(counted.bytes),
            lower_bound: counted.unreadable.is_some(),
        },
        None => Removed {
            path,
            bytes: None,
            lower_bound: false,
        },
    }
}

/// The trees the hands reaper would remove under `tmp`, with their sizes.
fn plan_hands(tmp: &Path, swept: &mut Swept) {
    plan_hands_with(hands::list_reapable_under(tmp).into_iter(), swept);
}

/// [`plan_hands`] over one listing, so a count that refuses is provable
/// without a hands tree whose read fails.
fn plan_hands_with(listed: impl Iterator<Item = hands::Listed>, swept: &mut Swept) {
    for listed in listed {
        match listed.count {
            Ok(counted) => swept.removed.push(removed(listed.tree, counted)),
            Err(error) => swept.failed.push(Failed {
                path: listed.tree,
                cause: FailedCause::Walk(cause(&error)),
            }),
        }
    }
}

/// The `TMPDIR` the hands reaper reads, refused when it is not absolute or
/// not UTF-8: a relative value would make the reaper remove a tree of the
/// process's own directory, and a non-UTF-8 one cannot be a seat tree the
/// journal names. Unset or empty is the platform's `/tmp`.
fn hands_tmp() -> Result<PathBuf, KeptReason> {
    let value = match std::env::var_os("TMPDIR").filter(|value| !value.is_empty()) {
        Some(value) => value,
        None => return Ok(PathBuf::from("/tmp")),
    };
    let path = PathBuf::from(&value);
    if !path.is_absolute() {
        return Err(KeptReason::TmpdirRelative);
    }
    if path.to_str().is_none() {
        return Err(KeptReason::TmpdirNotUtf8);
    }
    Ok(path)
}

/// Fold a kept verdict into the report, or hand back the row and path a
/// removal or a count may act on. One home for the kept line, so the pass
/// and the dry run cannot disagree about why a tree stayed.
fn keeps(swept: &mut Swept, verdict: Verdict) -> Option<(SeatTree, PathBuf)> {
    match verdict {
        Verdict::Kept { path, reason } => {
            swept.kept.push(Kept { path, reason });
            None
        }
        Verdict::Ready { row, path } => Some((row, path)),
    }
}

/// Record one complete removal, refusing a queue this binary does not
/// write its sweep rows in and a store that could not write. The cause is
/// text the report and the refusal both carry.
fn record(
    store: &mut Store,
    swept: Closed,
    path: &Path,
    bytes: Option<u64>,
    generation: i64,
) -> Result<(), String> {
    let bytes = bytes.map(|bytes| bytes as i64);
    match store.record_tree_sweep(swept, path, bytes, generation) {
        Ok(TreeSweepRecord::Written) => Ok(()),
        Ok(TreeSweepRecord::Unrecorded) => Err(UNRECORDED.to_string()),
        Err(error) => Err(error.to_string()),
    }
}

/// The real device a stat reports: the pass checks a tree's own device
/// against its root's.
fn same_device(_: &Path, device: u64) -> u64 {
    device
}

/// The cause a walk refusal gives, without the path its `Display` names.
/// 5c1's `TreeError::cause` is the purpose-built form, but it is `pub(crate)`
/// to brokkr-protocol, so this matches the variants it already exposes.
fn cause(error: &TreeError) -> String {
    match error {
        TreeError::Io { cause, .. } => cause.to_string(),
        TreeError::Foreign { device, tree, .. } => format!(
            "is on another filesystem (device {device}, not {tree}); a tree is removed on its own \
             filesystem"
        ),
    }
}

#[cfg(test)]
mod tests;
