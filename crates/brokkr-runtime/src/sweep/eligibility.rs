//! Which open `seat_trees` row may go, and why one may not (decision 0104
//! rulings 7 to 8, as R4 to R6 and the operator's succeeded-only ruling
//! amend them): the effect is closed, the attempt's terminal event is
//! `effect/succeeded`, and the path is the row's and the engine's shape.
//! The pass and the dry run read this one derivation, so they can never
//! disagree on what a pass would remove. The only typed reading kept
//! beside the rule words a conclude's keep; no other cause is keyed on.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{fold, Run};
use brokkr_store::{SeatTree, Store};
use uuid::Uuid;

use super::{KeptReason, ShapeReason, SCRATCH};

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

/// One terminal event's typed ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    Succeeded,
    Failed,
    Indeterminate,
}

/// The typed endings that named one attempt, and the one typed reading
/// kept beside the rule: a conclude's acceptance. A reused attempt id may
/// carry more than one ending; the tree goes only when all are
/// `effect/succeeded`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Terminal {
    /// Every ending a terminal event named the attempt with.
    endings: Vec<Ending>,
    /// A terminal's `causation_id` names the `operator/accepted` of a
    /// stop command, which only `close_riding_attempt` writes.
    concluded: bool,
}

impl Terminal {
    /// The attempt may go only when every terminal event that names it is
    /// `effect/succeeded`. A reused attempt id with any other ending keeps
    /// the tree. A `Terminal` is made only by [`Terminal::of`], so it always
    /// holds at least one ending.
    fn succeeded_only(&self) -> bool {
        self.endings
            .iter()
            .all(|ending| *ending == Ending::Succeeded)
    }

    /// The terminal that names the attempt is `effect/indeterminate`.
    fn indeterminate(&self) -> bool {
        self.endings.contains(&Ending::Indeterminate)
    }

    fn of(ending: Ending, concluded: bool) -> Terminal {
        Terminal {
            endings: vec![ending],
            concluded,
        }
    }
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
        let concluded = named(event.causation_id.as_deref(), &stops);
        let ending = match event.event_type {
            EventType::EffectSucceeded => Some(Ending::Succeeded),
            EventType::EffectFailed => Some(Ending::Failed),
            EventType::EffectIndeterminate => Some(Ending::Indeterminate),
            _ => None,
        };
        match ending {
            Some(ending) => {
                let terminal = Terminal::of(ending, concluded);
                match attempts.entry(attempt.to_string()) {
                    Entry::Occupied(mut entry) => match entry.get_mut() {
                        Attempt::Terminal(held) => {
                            held.endings.extend(terminal.endings);
                            held.concluded |= terminal.concluded;
                        }
                        Attempt::Unended => *entry.get_mut() = Attempt::Terminal(terminal),
                    },
                    Entry::Vacant(entry) => {
                        entry.insert(Attempt::Terminal(terminal));
                    }
                }
            }
            None => {
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
    let presence = match shaped(&row, same_root, device_of) {
        Ok(presence) => presence,
        Err(reason) => return Verdict::Kept { path, reason },
    };
    let folded = match &facts.folded {
        Ok(folded) => folded,
        Err(reason) => {
            return Verdict::Kept {
                path,
                reason: reason.clone(),
            }
        }
    };
    match held(folded, &row, presence) {
        Some(reason) => Verdict::Kept { path, reason },
        None => Verdict::Ready { row, path },
    }
}

/// Whether the row's tree stands under its root. An absent root or tree is
/// not a shape refusal: the row may close as gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Absent,
    Present,
}

/// Why the journal keeps the row, or `None` when it may go (ruling 7 as
/// amended: succeeded-only).
fn held(folded: &Folded, row: &SeatTree, presence: Presence) -> Option<KeptReason> {
    let attempt = folded.attempts.get(&row.attempt_id);
    // An absent tree whose attempt has a terminal event closes as gone
    // (ruling 8). An attempt with no terminal event may still be being
    // made, so its absent tree is kept.
    if presence == Presence::Absent && matches!(attempt, Some(Attempt::Terminal(_))) {
        return None;
    }
    if !folded.terminal && !folded.effects.contains(&row.effect_id) {
        return Some(KeptReason::EffectOpen);
    }
    match attempt {
        None => Some(KeptReason::NoEventNamesAttempt),
        Some(Attempt::Unended) => Some(KeptReason::NoTerminalEvent),
        Some(Attempt::Terminal(terminal)) if terminal.succeeded_only() => None,
        // The one typed reading kept: a conclude's acceptance is named, so
        // the keep's words can say the attempt was closed by a conclude.
        Some(Attempt::Terminal(terminal)) if terminal.concluded => Some(KeptReason::Concluded),
        Some(Attempt::Terminal(terminal)) if terminal.indeterminate() => {
            Some(KeptReason::Indeterminate)
        }
        // Every remaining ending is a failure: a reused attempt id with any
        // terminal that is not `effect/succeeded` keeps the tree.
        Some(Attempt::Terminal(_)) => Some(KeptReason::Failed),
    }
}

/// The path is the row's and the engine's shape (R5): an absolute root
/// ending in `scratch` that is its own canonical path, one root per run,
/// the engine's attempt UUID, and a real directory on the root's
/// filesystem. An absent root or tree is reported as [`Presence::Absent`],
/// so a row whose tree is gone can close.
fn shaped(
    row: &SeatTree,
    same_root: bool,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<Presence, KeptReason> {
    if !same_root {
        return Err(KeptReason::Shape(ShapeReason::AnotherRoot));
    }
    if !row.root.is_absolute() || row.root.file_name() != Some(OsStr::new(SCRATCH)) {
        return Err(KeptReason::Shape(ShapeReason::RootNotAbsolute));
    }
    if !engine_uuid(&row.attempt_id) {
        return Err(KeptReason::Shape(ShapeReason::AttemptNotEngineUuid));
    }
    // A root that is itself gone is reported absent, not refused: a row
    // whose attempt has a terminal event then closes as gone, as ruling 8
    // does for an absent tree.
    let Some(canonical) = canonical(row)? else {
        return Ok(Presence::Absent);
    };
    // Byte equality, not `Path` equality: the latter ignores `.`, repeated
    // separators and a trailing slash (R5).
    if canonical.as_os_str() != row.root.as_os_str() {
        return Err(KeptReason::Shape(ShapeReason::RootNotCanonical));
    }
    let root =
        std::fs::metadata(&row.root).map_err(|_| KeptReason::Shape(ShapeReason::RootUnreadable))?;
    let tree = row.root.join(&row.attempt_id);
    let meta = match std::fs::symlink_metadata(&tree) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Presence::Absent),
        Err(_) => return Err(KeptReason::Shape(ShapeReason::TreeUnreadable)),
    };
    if meta.file_type().is_symlink() {
        return Err(KeptReason::Shape(ShapeReason::TreeIsLink));
    }
    if !meta.file_type().is_dir() {
        return Err(KeptReason::Shape(ShapeReason::TreeNotDirectory));
    }
    if device_of(&tree, meta.dev()) != root.dev() {
        return Err(KeptReason::Shape(ShapeReason::ForeignFilesystem));
    }
    Ok(Presence::Present)
}

/// The root's canonical path, or `None` where the root itself is gone.
fn canonical(row: &SeatTree) -> Result<Option<PathBuf>, KeptReason> {
    match std::fs::canonicalize(&row.root) {
        Ok(canonical) => Ok(Some(canonical)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(KeptReason::Shape(ShapeReason::RootNotCanonical)),
    }
}

/// Is `text` exactly the lower-case hyphenated UUID the engine mints?
fn engine_uuid(text: &str) -> bool {
    Uuid::try_parse(text).is_ok_and(|id| id.hyphenated().to_string() == text)
}
