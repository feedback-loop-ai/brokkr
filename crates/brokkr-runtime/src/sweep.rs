//! The sweep (decision 0104 rulings 7 to 9, as R3 to R6 amend them): one
//! pass removes each run-local seat scratch tree the journal proves free,
//! runs the hands reaper, and records every complete removal in
//! `tree_sweeps`. A dry run reads the same verdicts and counts what a pass
//! would free, writing nothing. Nothing here prints: a [`Swept`] names
//! every tree it removed, kept or could not remove, and the CLI paints it.

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

/// Which tree a removal closed: a run-local seat scratch tree, or a hands
/// session tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepKind {
    Scratch,
    Hands,
}

/// One tree a pass removed, or a dry run would remove. `bytes` is `None`
/// when the tree was gone before the pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    pub kind: SweepKind,
    pub path: PathBuf,
    pub bytes: Option<u64>,
}

/// One tree kept, with the reason the journal gave. The reason names no
/// path: the line names the tree once, through the renderer's `safe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub path: PathBuf,
    pub reason: String,
}

/// One tree a pass could not remove, with the walk's cause. The cause
/// names no path, for the same reason [`Kept::reason`] does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub path: PathBuf,
    pub cause: String,
}

/// What one pass did, or what a dry run would do.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Swept {
    removed: Vec<Removed>,
    kept: Vec<Kept>,
    failed: Vec<Failed>,
}

/// A sweep refused before it could report: the journal could not be read,
/// or a removed tree could not be recorded.
#[derive(Debug, Error)]
pub enum SweepError {
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The removal completed but the journal is a queue this binary does
    /// not write its sweep rows in, so nothing was recorded.
    #[error(
        "a tree was removed but not recorded: the journal's queue is not one this binary \
         writes tree sweeps in"
    )]
    Unrecorded,
}

impl Swept {
    /// The trees the pass removed, or the dry run would remove.
    pub fn removed(&self) -> &[Removed] {
        &self.removed
    }

    /// The trees the journal kept, and why.
    pub fn kept(&self) -> &[Kept] {
        &self.kept
    }

    /// The trees the walk could not remove.
    pub fn failed(&self) -> &[Failed] {
        &self.failed
    }

    /// The bytes the pass freed, or the dry run would free.
    pub fn freed(&self) -> u64 {
        self.removed
            .iter()
            .filter_map(|removed| removed.bytes)
            .sum()
    }

    /// One line per tree, each path through `safe`, then the total. A dry
    /// run says what it would do and writes nothing.
    pub fn lines_with(&self, safe: fn(&str) -> String, dry_run: bool) -> String {
        let mut said = String::new();
        for removed in &self.removed {
            let path = safe(&removed.path.display().to_string());
            match (dry_run, removed.bytes) {
                (true, Some(bytes)) => {
                    said.push_str(&format!("sweep: would remove {path}: {bytes} bytes\n"))
                }
                (true, None) => {
                    said.push_str(&format!("sweep: would remove {path}: it is already gone\n"))
                }
                (false, Some(bytes)) => {
                    said.push_str(&format!("sweep: removed {path}: {bytes} bytes\n"))
                }
                (false, None) => {
                    said.push_str(&format!("sweep: removed {path}: it was already gone\n"))
                }
            }
        }
        for kept in &self.kept {
            let path = safe(&kept.path.display().to_string());
            said.push_str(&format!("sweep: kept {path}: {}\n", kept.reason));
        }
        for failed in &self.failed {
            let path = safe(&failed.path.display().to_string());
            let cause = safe(&failed.cause);
            said.push_str(&format!("sweep: could not remove {path}: {cause}\n"));
        }
        let verb = match dry_run {
            true => "would free",
            false => "freed",
        };
        said.push_str(&format!("sweep: {verb} {} bytes\n", self.freed()));
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
    let verdicts = eligibility::decide(store, device_of)?;
    let mut swept = Swept::default();
    for verdict in verdicts {
        let Some((row, path)) = keeps(&mut swept, verdict) else {
            continue;
        };
        match remove(&path) {
            Ok(bytes) => {
                record(
                    store,
                    Closed::Scratch(row.seq),
                    &path,
                    bytes,
                    held.generation.0,
                )?;
                swept.removed.push(Removed {
                    kind: SweepKind::Scratch,
                    path,
                    bytes,
                });
            }
            Err(error) => swept.failed.push(Failed {
                path,
                cause: cause(&error),
            }),
        }
    }
    pass_hands(store, held, &mut swept)?;
    Ok(swept)
}

/// Run the hands reaper and record each dead owner's free tree it removes.
fn pass_hands(store: &mut Store, held: &Dispatcher, swept: &mut Swept) -> Result<(), SweepError> {
    let reaped = hands::reap_dead_sessions();
    for (path, bytes) in reaped.removed() {
        record(store, Closed::Hands, path, Some(bytes), held.generation.0)?;
        swept.removed.push(Removed {
            kind: SweepKind::Hands,
            path: path.to_path_buf(),
            bytes: Some(bytes),
        });
    }
    for (path, reason) in reaped.kept() {
        swept.kept.push(Kept {
            path: path.to_path_buf(),
            reason,
        });
    }
    Ok(())
}

/// What a pass would remove, with the bytes it would free, writing
/// nothing (R3): no row, no holder and no migration.
pub fn plan(store: &Store) -> Result<Swept, SweepError> {
    plan_with(store, &std::env::temp_dir(), same_device, tree::count_tree)
}

/// [`plan`], with the same seams the pass has.
fn plan_with(
    store: &Store,
    tmp: &Path,
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
            Ok(counted) => swept.removed.push(Removed {
                kind: SweepKind::Scratch,
                path,
                bytes: counted.map(|counted| counted.bytes),
            }),
            Err(error) => swept.failed.push(Failed {
                path,
                cause: cause(&error),
            }),
        }
    }
    plan_hands(tmp, &mut swept);
    Ok(swept)
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
            Ok(counted) => swept.removed.push(Removed {
                kind: SweepKind::Hands,
                path: listed.tree,
                bytes: counted.map(|counted| counted.bytes),
            }),
            Err(error) => swept.failed.push(Failed {
                path: listed.tree,
                cause: cause(&error),
            }),
        }
    }
}

/// Fold a kept verdict into the report, or hand back the row and path a
/// removal or a count may act on. One home for the kept line, so the pass
/// and the dry run cannot disagree about why a tree stayed.
fn keeps(swept: &mut Swept, verdict: Verdict) -> Option<(SeatTree, PathBuf)> {
    match verdict {
        Verdict::Kept { path, reason } => {
            swept.kept.push(Kept {
                path,
                reason: reason.to_string(),
            });
            None
        }
        Verdict::Ready { row, path } => Some((row, path)),
    }
}

/// Record one complete removal, refusing a queue this binary does not
/// write its sweep rows in and a store that could not write.
fn record(
    store: &mut Store,
    swept: Closed,
    path: &Path,
    bytes: Option<u64>,
    generation: i64,
) -> Result<(), SweepError> {
    let bytes = bytes.map(|bytes| bytes as i64);
    match store.record_tree_sweep(swept, path, bytes, generation)? {
        TreeSweepRecord::Written => Ok(()),
        TreeSweepRecord::Unrecorded => Err(SweepError::Unrecorded),
    }
}

/// The real device a stat reports: the pass checks a tree's own device
/// against its root's.
fn same_device(_: &Path, device: u64) -> u64 {
    device
}

/// The cause a walk refusal gives, without the path its `Display` names.
/// 5c1's `TreeError::cause` is the purpose-built form, but it is private
/// to brokkr-protocol, so this reads the cause half of the same display.
fn cause(error: &TreeError) -> String {
    match error {
        TreeError::Io { cause, .. } => cause.to_string(),
        TreeError::Foreign { path, .. } => {
            let text = error.to_string();
            let prefix = format!("{} ", path.display());
            text.strip_prefix(&prefix).unwrap_or(&text).to_string()
        }
    }
}

#[cfg(test)]
mod tests;
