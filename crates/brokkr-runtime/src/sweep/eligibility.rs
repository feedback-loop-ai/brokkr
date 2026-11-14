//! Which open `seat_trees` row may go, and why one may not (decision 0104
//! rulings 7 to 8, as R4 to R6 and the operator's fail-closed ruling amend
//! them): the effect is closed, the attempt's own driver ended it, and the
//! path is the row's and the engine's shape. The pass and the dry run read
//! this one derivation, so they can never disagree on what a pass would
//! remove.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{fold, Run};
use brokkr_store::{SeatTree, Store};
use uuid::Uuid;

use super::{KeptReason, SCRATCH};

/// What the row decided: remove it, or keep it for one stated reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    Ready { row: SeatTree, path: PathBuf },
    Kept { path: PathBuf, reason: KeptReason },
}

/// The journal facts one run contributes, read once per pass. A run the
/// journal cannot read or the fold refuses keeps every row it names: the
/// sweep never removes a tree whose effect it cannot place.
struct Facts {
    folded: Result<Folded, KeptReason>,
}

struct Folded {
    /// The fold reached `Completed` or `Stopped` (R6).
    terminal: bool,
    /// The effects an `effect/succeeded` names, compared as stored.
    effects: HashSet<String>,
    /// Every attempt an event names, and the terminal events that ended it.
    attempts: HashMap<String, Attempt>,
}

enum Attempt {
    /// An event names the attempt, and none of its terminal events does.
    Unended,
    /// Terminal events name it: which typed endings they carry, whichever
    /// came first.
    Terminal(Terminal),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Terminal {
    /// An `effect/indeterminate` ended the attempt.
    indeterminate: bool,
    /// The terminal's `causation_id` names the `operator/accepted` of a
    /// stop command, which only `close_riding_attempt` writes.
    concluded: bool,
    /// The terminal's `causation_id` names one of the attempt's
    /// checkpoints, which a restarted engine's broker tallies append
    /// before `ended_restarted` ends the attempt.
    restarted: bool,
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
                    folded: Err(KeptReason::RunCannotBeRead),
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
fn folded(events: &[EventEnvelope]) -> Result<Folded, KeptReason> {
    let state = fold(events).map_err(|_| KeptReason::RunDoesNotFold)?;
    let stops = stop_acceptances(events);
    let checkpoints: HashSet<(&str, &str)> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter_map(|event| Some((event.event_id.as_str(), event.attempt_id.as_deref()?)))
        .collect();
    let mut effects = HashSet::new();
    let mut attempts = HashMap::new();
    for event in events {
        if event.event_type == EventType::EffectSucceeded {
            // The fold read the effect id as a string before this loop runs,
            // so the read is an invariant, not a branch the pass can miss.
            let effect = event.payload["effect_id"]
                .as_str()
                .expect("a folded effect/succeeded names its effect");
            effects.insert(effect.to_string());
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
                let ending = Terminal {
                    indeterminate: event.event_type == EventType::EffectIndeterminate,
                    concluded: named(event.causation_id.as_deref(), &stops),
                    // Only a failed end can be `ended_restarted`'s: a
                    // succeeded event is never one a restart writes.
                    restarted: event.event_type == EventType::EffectFailed
                        && event
                            .causation_id
                            .as_deref()
                            .is_some_and(|cause| checkpoints.contains(&(cause, attempt))),
                };
                attempts
                    .entry(attempt.to_string())
                    .and_modify(|attempt| match attempt {
                        Attempt::Terminal(held) => {
                            held.indeterminate |= ending.indeterminate;
                            held.concluded |= ending.concluded;
                            held.restarted |= ending.restarted;
                        }
                        Attempt::Unended => *attempt = Attempt::Terminal(ending),
                    })
                    .or_insert(Attempt::Terminal(ending));
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

/// The ids of the `operator/accepted` events that dispose of a stop
/// command: a terminal caused by one of them is a conclude's.
fn stop_acceptances(events: &[EventEnvelope]) -> HashSet<&str> {
    let commands: HashSet<&str> = events
        .iter()
        .filter(|event| event.event_type == EventType::OperatorCommanded)
        .filter(|event| event.payload.get("command").and_then(|word| word.as_str()) == Some("stop"))
        .filter_map(command_id)
        .collect();
    events
        .iter()
        .filter(|event| event.event_type == EventType::OperatorAccepted)
        .filter(|event| command_id(event).is_some_and(|id| commands.contains(id)))
        .map(|event| event.event_id.as_str())
        .collect()
}

/// The `command_id` an operator event carries.
fn command_id(event: &EventEnvelope) -> Option<&str> {
    event.payload.get("command_id").and_then(|id| id.as_str())
}

/// Whether `cause` names one of `ids`.
fn named(cause: Option<&str>, ids: &HashSet<&str>) -> bool {
    cause.is_some_and(|cause| ids.contains(cause))
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
        Err(reason) => {
            return Verdict::Kept {
                path,
                reason: reason.clone(),
            }
        }
    };
    match held(folded, &row) {
        Some(reason) => Verdict::Kept { path, reason },
        None => Verdict::Ready { row, path },
    }
}

/// Why the journal keeps the row, or `None` when it may go (rulings 7 and
/// 8 as amended).
fn held(folded: &Folded, row: &SeatTree) -> Option<KeptReason> {
    if !folded.terminal && !folded.effects.contains(&row.effect_id) {
        return Some(KeptReason::EffectOpen);
    }
    match folded.attempts.get(&row.attempt_id) {
        None => Some(KeptReason::NoEventNamesAttempt),
        Some(Attempt::Unended) => Some(KeptReason::NoTerminalEvent),
        // The most specific typed ending first: a conclude's indeterminate
        // is reported as a conclude's, not as the indeterminate it is.
        Some(Attempt::Terminal(terminal)) if terminal.concluded => Some(KeptReason::Concluded),
        Some(Attempt::Terminal(terminal)) if terminal.restarted => Some(KeptReason::Restarted),
        Some(Attempt::Terminal(terminal)) if terminal.indeterminate => {
            Some(KeptReason::Indeterminate)
        }
        Some(Attempt::Terminal(_)) => None,
    }
}

/// The path is the row's and the engine's shape (R5): an absolute root
/// ending in `scratch` that is its own canonical path, one root per run,
/// the engine's attempt UUID, and a real directory on the root's
/// filesystem. A root that is itself absent closes the row as gone, so a
/// tree is never kept forever by a directory that no longer exists.
fn shaped(
    row: &SeatTree,
    same_root: bool,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<(), KeptReason> {
    if !same_root {
        return Err(KeptReason::AnotherRoot);
    }
    if !row.root.is_absolute() || row.root.file_name() != Some(OsStr::new(SCRATCH)) {
        return Err(KeptReason::RootNotAbsolute);
    }
    if !engine_uuid(&row.attempt_id) {
        return Err(KeptReason::AttemptNotEngineUuid);
    }
    // A root that is itself gone closes the row as gone, with no bytes, as
    // ruling 8 does for an absent tree.
    let Some(canonical) = canonical(row)? else {
        return Ok(());
    };
    // Byte equality, not `Path` equality: the latter ignores `.`, repeated
    // separators and a trailing slash (R5).
    if canonical.as_os_str() != row.root.as_os_str() {
        return Err(KeptReason::RootNotCanonical);
    }
    let root = std::fs::metadata(&row.root).map_err(|_| KeptReason::RootUnreadable)?;
    let tree = row.root.join(&row.attempt_id);
    let meta = match std::fs::symlink_metadata(&tree) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(KeptReason::TreeUnreadable),
    };
    if meta.file_type().is_symlink() {
        return Err(KeptReason::TreeIsLink);
    }
    if !meta.file_type().is_dir() {
        return Err(KeptReason::TreeNotDirectory);
    }
    if device_of(&tree, meta.dev()) != root.dev() {
        return Err(KeptReason::ForeignFilesystem);
    }
    Ok(())
}

/// The root's canonical path, or `None` where the root itself is gone.
fn canonical(row: &SeatTree) -> Result<Option<PathBuf>, KeptReason> {
    match std::fs::canonicalize(&row.root) {
        Ok(canonical) => Ok(Some(canonical)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(KeptReason::RootNotCanonical),
    }
}

/// Is `text` exactly the lower-case hyphenated UUID the engine mints?
fn engine_uuid(text: &str) -> bool {
    Uuid::try_parse(text).is_ok_and(|id| id.hyphenated().to_string() == text)
}
