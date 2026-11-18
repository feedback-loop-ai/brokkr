//! Which open `seat_trees` row may go, and why one may not (decision 0104
//! rulings 7 to 8, as R4 to R6 and the operator's succeeded-only and
//! gone-close rulings amend them): the effect is closed, the attempt's
//! terminal event is `effect/succeeded`, and the path is the row's and the
//! engine's shape. A row whose tree is gone closes as gone only once its
//! run has ended too; an absent tree of a live run keeps its row. The pass
//! and the dry run read this one derivation, so they can never disagree on
//! what a pass would remove. The only typed reading kept beside the rule
//! words a conclude's keep; no other cause is keyed on.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{fold, OperatorCommand, Run};
use brokkr_core::{accepted_command, operator_command, succeeded_effect};
use brokkr_store::{SeatTree, Store};
use uuid::Uuid;

use super::{HeldReason, KeptReason, ShapeReason, SCRATCH};

// The payloads the sweep reads are read by brokkr-core's own readers, the
// one home of their shapes (0071 rulings 3 and 5): a payload that does not
// read keeps the run's rows, never a panic and never a removal, and an
// array is refused as the fold refuses it.

/// What the row decided: remove it, close it as gone, or keep it for one
/// stated reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    Ready {
        row: SeatTree,
        path: PathBuf,
        /// Whether this row's root is its run's one root (R5), carried so
        /// the pass can re-run the shape check before walking.
        same_root: bool,
    },
    Gone {
        row: SeatTree,
        path: PathBuf,
    },
    Kept {
        path: PathBuf,
        reason: KeptReason,
    },
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
    /// The effects an applied `effect/succeeded` names, compared as stored.
    effects: HashSet<String>,
    /// Every attempt an event the fold applies names, and the terminal
    /// events that ended it.
    attempts: HashMap<String, Attempt>,
    /// Every attempt a late event names: the fold keeps and shows such an
    /// event and never applies it, and its process stood after the run
    /// ended, so a tree that stands at its path stays. A gone tree has
    /// nothing left to remove, so its row closes as gone like any other.
    late: HashSet<String>,
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
    /// stop command. `close_riding_attempt` writes one, and so does a
    /// resumed engine's `ended_restarted` terminal while that acceptance
    /// is the journal's tip.
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
    device_of: &impl Fn(&Path, u64) -> u64,
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
        verdicts.push(verdict(row, &roots, &facts[&run_id], device_of));
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

/// The facts a run's events state, or the reason the fold, or the sweep's
/// own decode of a payload it reads, refused them.
/// Only the events the fold applies are read: a repaired late tail is kept
/// and shown and never applied, so it neither ends an attempt nor closes
/// an effect, and an attempt it names keeps its tree.
fn folded(events: &[EventEnvelope]) -> Result<Folded, KeptReason> {
    let state = fold(events).map_err(|_| KeptReason::RunDoesNotFold)?;
    let stops = stop_acceptances(events, &state)?;
    let (effects, attempts) = applied(events, &state, &stops)?;
    Ok(Folded {
        terminal: matches!(state.run, Run::Completed | Run::Stopped),
        effects,
        attempts,
        late: late_attempts(events, &state),
    })
}

/// The effects an applied `effect/succeeded` closes and the attempts the
/// applied events name, with every terminal ending they carry.
fn applied(
    events: &[EventEnvelope],
    state: &brokkr_core::fold::RunState,
    stops: &HashSet<&str>,
) -> Result<(HashSet<String>, HashMap<String, Attempt>), KeptReason> {
    let mut effects = HashSet::new();
    let mut attempts = HashMap::new();
    for event in events {
        if !state.applies(event.seq) {
            continue;
        }
        if event.event_type == EventType::EffectSucceeded {
            // The fold read the effect id as a string before this loop
            // runs, so the refusal is a value, not a closure no test enters.
            let effect = succeeded_effect(event)
                .ok()
                .ok_or(KeptReason::RunDoesNotFold)?;
            effects.insert(effect.to_string());
        }
        let Some(attempt) = event.attempt_id.as_deref() else {
            continue;
        };
        let concluded = named(event.causation_id.as_deref(), stops);
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
    Ok((effects, attempts))
}

/// The attempts a late event names: the fold keeps and shows such an event
/// and never applies it, so its process stood after the run ended.
fn late_attempts(events: &[EventEnvelope], state: &brokkr_core::fold::RunState) -> HashSet<String> {
    events
        .iter()
        .filter(|event| !state.applies(event.seq))
        .filter_map(|event| event.attempt_id.clone())
        .collect()
}

/// The ids of the `operator/accepted` events that dispose of a stop
/// command: a terminal caused by one of them closed its attempt under an
/// accepted stop, whether a conclude or a resumed engine's restart wrote it.
/// Only the events the fold applies are read, as [`applied`] reads them: a
/// repaired late tail is never applied, so its payloads decide nothing.
/// Each applied operator payload is read once, here; one that does not
/// read keeps the run's rows.
fn stop_acceptances<'a>(
    events: &'a [EventEnvelope],
    state: &brokkr_core::fold::RunState,
) -> Result<HashSet<&'a str>, KeptReason> {
    let mut commands = HashSet::new();
    let mut accepted = Vec::new();
    for event in events.iter().filter(|event| state.applies(event.seq)) {
        if event.event_type == EventType::OperatorCommanded {
            let (command_id, command) = operator_command(event)
                .ok()
                .ok_or(KeptReason::RunDoesNotFold)?;
            if command == OperatorCommand::Stop.as_str() {
                commands.insert(command_id);
            }
        } else if event.event_type == EventType::OperatorAccepted {
            // The fold read an applied acceptance's command id as a string
            // before this loop runs, so the refusal is a value, not a
            // closure no test enters; the fold's refusal is pinned by
            // `an_operator_payload_that_does_not_decode_keeps_the_runs_tree`.
            let command_id = accepted_command(event)
                .ok()
                .ok_or(KeptReason::RunDoesNotFold)?;
            accepted.push((command_id, event.event_id.as_str()));
        }
    }
    Ok(accepted
        .into_iter()
        .filter(|(command, _)| commands.contains(command))
        .map(|(_, event_id)| event_id)
        .collect())
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
    match (held(folded, &row, presence), presence) {
        (Some(reason), _) => Verdict::Kept { path, reason },
        (None, Presence::Present) => Verdict::Ready {
            row,
            path,
            same_root,
        },
        (None, Presence::Absent) => Verdict::Gone { row, path },
    }
}

/// Whether the row's tree stands under its root. A present root with an
/// absent tree is reported absent: the row may close as gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Absent,
    Present,
}

/// Why the journal keeps the row, or `None` when it may go (ruling 7 as
/// amended: succeeded-only for a present tree, and a gone tree only once
/// its run has ended).
fn held(folded: &Folded, row: &SeatTree, presence: Presence) -> Option<KeptReason> {
    // The effect is closed before any close, as R7 has it.
    if !folded.terminal && !folded.effects.contains(&row.effect_id) {
        return Some(KeptReason::EffectOpen);
    }
    match presence {
        Presence::Absent => gone(folded, row),
        Presence::Present => present(folded, row),
    }
}

/// Why a gone row stays, or `None` when it closes as gone: every check in
/// ruling 7 as amended passed before this point.
fn gone(folded: &Folded, row: &SeatTree) -> Option<KeptReason> {
    match folded.attempts.get(&row.attempt_id) {
        None => Some(KeptReason::NoEventNamesAttempt),
        Some(Attempt::Unended) => Some(KeptReason::NoTerminalEvent),
        Some(Attempt::Terminal(_)) if !folded.terminal => {
            Some(KeptReason::Held(HeldReason::RunNotEnded))
        }
        Some(Attempt::Terminal(_)) => None,
    }
}

/// Why a present tree stays, or `None` when it may go: no late event names
/// its attempt, the attempt ended succeeded and R5 held, and the caller
/// re-checks the shape before a walk.
fn present(folded: &Folded, row: &SeatTree) -> Option<KeptReason> {
    // A late event shows the attempt's process stood after its run ended:
    // the fold never applies it, and no reading of it decides a removal.
    if folded.late.contains(&row.attempt_id) {
        return Some(KeptReason::Held(HeldReason::LateEvent));
    }
    match folded.attempts.get(&row.attempt_id) {
        None => Some(KeptReason::NoEventNamesAttempt),
        Some(Attempt::Unended) => Some(KeptReason::NoTerminalEvent),
        Some(Attempt::Terminal(terminal)) if terminal.succeeded_only() => None,
        // The one typed reading kept: a conclude's acceptance is named, so
        // the keep's words can say the attempt was closed under an accepted
        // stop.
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
/// filesystem. A present root with an absent tree is reported as
/// [`Presence::Absent`], so a row whose tree is gone can close; a root
/// that is itself gone is kept, because R5 checks a root that exists.
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
    let canonical = canonical(row)?;
    // Byte equality, not `Path` equality: the latter ignores `.`, repeated
    // separators and a trailing slash (R5).
    if canonical.as_os_str() != row.root.as_os_str() {
        return Err(KeptReason::Shape(ShapeReason::RootNotCanonical));
    }
    // Only a root that went between the two reads refuses here, so the
    // refusal is a value, not a closure no test can enter.
    let root = std::fs::metadata(&row.root)
        .ok()
        .ok_or(KeptReason::Shape(ShapeReason::RootUnreadable))?;
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

/// Re-run the row's R5 shape check immediately before a walk, so a tree
/// whose shape or presence changed since the verdict is kept, not removed.
pub(super) fn still_present(
    row: &SeatTree,
    same_root: bool,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<bool, KeptReason> {
    Ok(matches!(
        shaped(row, same_root, device_of)?,
        Presence::Present
    ))
}

/// The root's canonical path. A root that is itself gone is a shape
/// refusal, not an absent tree: R5 checks a root that exists.
fn canonical(row: &SeatTree) -> Result<PathBuf, KeptReason> {
    match std::fs::canonicalize(&row.root) {
        Ok(canonical) => Ok(canonical),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(KeptReason::Shape(ShapeReason::RootGone))
        }
        Err(_) => Err(KeptReason::Shape(ShapeReason::RootNotCanonical)),
    }
}

/// Is `text` exactly the lower-case hyphenated UUID the engine mints?
fn engine_uuid(text: &str) -> bool {
    Uuid::try_parse(text).is_ok_and(|id| id.hyphenated().to_string() == text)
}
