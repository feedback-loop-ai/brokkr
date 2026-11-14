//! Which open `seat_trees` row may go, and why one may not (decision 0104
//! rulings 7 to 8, as R4 to R6 amend them): the effect is closed, no
//! process holds the attempt, and the path is the row's and the engine's
//! shape. The pass and the dry run read this one derivation, so they can
//! never disagree on what a pass would remove.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{fold, Run};
use brokkr_store::{SeatTree, Store};
use uuid::Uuid;

use super::SCRATCH;

/// What the row decided: remove it, or keep it for one stated reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    Ready { row: SeatTree, path: PathBuf },
    Kept { path: PathBuf, reason: &'static str },
}

/// The journal facts one run contributes, read once per pass. A run the
/// journal cannot read or the fold refuses keeps every row it names: the
/// sweep never removes a tree whose effect it cannot place.
struct Facts {
    folded: Result<Folded, &'static str>,
}

struct Folded {
    /// The fold reached `Completed` or `Stopped` (R6).
    terminal: bool,
    /// The effects an `effect/succeeded` names, compared as stored.
    effects: HashSet<String>,
    /// Every attempt an event names, and whether its terminal event is
    /// proven over.
    attempts: HashMap<String, Attempt>,
}

enum Attempt {
    /// An event names the attempt, and none of its terminal events does.
    Unended,
    /// A terminal event names it. `unproven` is true when any of them
    /// carries cleanup evidence.
    Terminal { unproven: bool },
}

/// One verdict per open row, in `seq` order, with each run's events loaded
/// once. `device_of` is the seam a test uses to plant a tree on another
/// filesystem, as 5c1's walk has.
pub(super) fn decide(
    store: &Store,
    device_of: impl Fn(&Path, u64) -> u64,
) -> Result<Vec<Verdict>, brokkr_store::StoreError> {
    let rows = store.open_seat_trees()?;
    let mut roots: HashMap<String, Option<PathBuf>> = HashMap::new();
    let mut facts: HashMap<String, Facts> = HashMap::new();
    for row in &rows {
        if roots.contains_key(&row.run_id) {
            continue;
        }
        let all = store.seat_trees(&row.run_id)?;
        roots.insert(row.run_id.clone(), one_root(&all));
        facts.insert(row.run_id.clone(), Facts::of(store, &row.run_id));
    }
    let mut verdicts = Vec::with_capacity(rows.len());
    for row in rows {
        let run_id = row.run_id.clone();
        verdicts.push(verdict(row, &roots, &facts[&run_id], &device_of));
    }
    Ok(verdicts)
}

/// The one root every row of a run names, or `None` where they disagree
/// (R5). The rows include those a sweep already closed: the journal's
/// history of a run is what says which root was its own.
fn one_root(rows: &[SeatTree]) -> Option<PathBuf> {
    let roots: HashSet<&Path> = rows.iter().map(|row| row.root.as_path()).collect();
    match roots.len() {
        1 => roots.into_iter().next().map(Path::to_path_buf),
        _ => None,
    }
}

impl Facts {
    /// A run's facts, or the reason its rows stay: a journal that cannot be
    /// read and a fold that refuses are kept trees, never removed ones.
    fn of(store: &Store, run_id: &str) -> Facts {
        let events = match store.load(run_id) {
            Ok(events) => events,
            Err(_) => {
                return Facts {
                    folded: Err("its run cannot be read"),
                }
            }
        };
        match folded(&events) {
            Ok(folded) => Facts { folded: Ok(folded) },
            Err(reason) => Facts {
                folded: Err(reason),
            },
        }
    }
}

/// The facts a run's events state, or the reason the fold refused them.
fn folded(events: &[EventEnvelope]) -> Result<Folded, &'static str> {
    let state = fold(events).map_err(|_| "its run does not fold")?;
    let mut effects = HashSet::new();
    let mut attempts = HashMap::new();
    for event in events {
        if event.event_type == EventType::EffectSucceeded {
            if let Some(effect) = event.payload.get("effect_id").and_then(|id| id.as_str()) {
                effects.insert(effect.to_string());
            }
        }
        let Some(attempt) = event.attempt_id.as_deref() else {
            continue;
        };
        let terminal = matches!(
            event.event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        );
        match terminal {
            true => {
                let unproven = carries_evidence(event);
                attempts
                    .entry(attempt.to_string())
                    .and_modify(|attempt| match attempt {
                        Attempt::Terminal { unproven: held } => *held |= unproven,
                        Attempt::Unended => *attempt = Attempt::Terminal { unproven },
                    })
                    .or_insert(Attempt::Terminal { unproven });
            }
            false => {
                attempts
                    .entry(attempt.to_string())
                    .or_insert(Attempt::Unended);
            }
        }
    }
    Ok(Folded {
        terminal: matches!(state.run, Run::Completed | Run::Stopped),
        effects,
        attempts,
    })
}

/// Does the terminal event carry the cleanup evidence R4 keeps the tree
/// for: a seat's `received` and `cleanup`, or a panel's
/// `unresolved_members`.
fn carries_evidence(event: &EventEnvelope) -> bool {
    ["received", "cleanup", "unresolved_members"]
        .iter()
        .any(|key| event.payload.get(*key).is_some())
}

fn verdict(
    row: SeatTree,
    roots: &HashMap<String, Option<PathBuf>>,
    facts: &Facts,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Verdict {
    let path = row.root.join(&row.attempt_id);
    let same_root =
        roots.get(&row.run_id).and_then(|root| root.as_deref()) == Some(row.root.as_path());
    if let Err(reason) = shaped(&row, same_root, device_of) {
        return Verdict::Kept { path, reason };
    }
    let folded = match &facts.folded {
        Ok(folded) => folded,
        Err(reason) => return Verdict::Kept { path, reason },
    };
    match held(folded, &row) {
        Some(reason) => Verdict::Kept { path, reason },
        None => Verdict::Ready { row, path },
    }
}

/// Why the journal keeps the row, or `None` when it may go (rulings 7 and
/// 8 as amended).
fn held(folded: &Folded, row: &SeatTree) -> Option<&'static str> {
    if !folded.terminal && !folded.effects.contains(&row.effect_id) {
        return Some("its effect is still open");
    }
    match folded.attempts.get(&row.attempt_id) {
        None => Some("no event names its attempt"),
        Some(Attempt::Unended) => Some("no terminal event names its attempt"),
        Some(Attempt::Terminal { unproven: true }) => {
            Some("its attempt carries unproven cleanup evidence")
        }
        Some(Attempt::Terminal { unproven: false }) => None,
    }
}

/// The path is the row's and the engine's shape (R5): an absolute root
/// ending in `scratch` that is its own canonical path, one root per run,
/// the engine's attempt UUID, and a real directory on the root's
/// filesystem.
fn shaped(
    row: &SeatTree,
    same_root: bool,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<(), &'static str> {
    if !same_root {
        return Err("another row of its run names a different root");
    }
    if !row.root.is_absolute() || row.root.file_name() != Some(OsStr::new(SCRATCH)) {
        return Err("its root is not an absolute path ending in 'scratch'");
    }
    if !engine_uuid(&row.attempt_id) {
        return Err("its attempt id is not the one the engine mints");
    }
    let canonical =
        std::fs::canonicalize(&row.root).map_err(|_| "its root is not its own canonical path")?;
    // Byte equality, not `Path` equality: the latter ignores `.`, repeated
    // separators and a trailing slash (R5).
    if canonical.as_os_str() != row.root.as_os_str() {
        return Err("its root is not its own canonical path");
    }
    let root = std::fs::metadata(&row.root).map_err(|_| "its root cannot be read")?;
    let tree = row.root.join(&row.attempt_id);
    let meta = match std::fs::symlink_metadata(&tree) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("its tree cannot be read"),
    };
    if meta.file_type().is_symlink() {
        return Err("its tree is a link, not a directory");
    }
    if !meta.file_type().is_dir() {
        return Err("its tree is not a directory");
    }
    if device_of(&tree, meta.dev()) != root.dev() {
        return Err("its tree is on another filesystem than its root");
    }
    Ok(())
}

/// Is `text` exactly the lower-case hyphenated UUID the engine mints?
fn engine_uuid(text: &str) -> bool {
    Uuid::try_parse(text).is_ok_and(|id| id.hyphenated().to_string() == text)
}
