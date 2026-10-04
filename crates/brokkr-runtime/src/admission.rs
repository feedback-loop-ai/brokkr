//! The dispatcher's admission pass (decision 0068 ruling 3; #430's second
//! slice): for each entry still waiting, in the queue's order, whether it
//! may start now, and every reason it may not. The pass reads the journal
//! and the realms maps and decides; it starts nothing and writes nothing,
//! so the same journal and the same maps give the same verdicts, and
//! `brokkr queue list` shows them.
//!
//! A reason either WAITS, and clears by itself (an awaited entry whose run
//! has not ended), or HOLDS, and only the operator clears it: the
//! operator's own hold; a wait that can never be met, because the awaited
//! entry was dropped or its run stopped where `completed` was asked; a
//! realm whose governing facts changed since the entry was queued; and a
//! map that cannot be read to tell. Admission never resolves a hold. It
//! never starts an entry on a grant the map no longer gives, and never
//! takes one the map gives now that the entry was not queued with: the
//! operator re-pins the entry to the map on disk, or re-queues or drops it.
//!
//! Provider and route concurrency and cool-downs, the scratch-space floor
//! and the boxed-build ceiling are not judged here yet: decision 0068
//! declares them in the realm's host configuration, which the realms map
//! does not carry yet.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use brokkr_core::fold::{fold, FoldError, Status};
use brokkr_core::realms::{Boundary, CapabilityGrant};
use brokkr_store::{EntryId, EntryState, QueueEntry, Store, StoreError, Wait, WaitOn};
use serde_json::Value;
use thiserror::Error;

use crate::launch::{HeldAndNow, LaunchError, QueuedLaunch};
use crate::realms::{World, WorldError};

/// Why an entry may not start now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The operator holds the entry.
    OperatorHold,
    /// An entry it waits for has not started a run.
    NotStarted(Wait),
    /// An entry it waits for started a run that has not ended.
    Unended { wait: Wait, run: String },
    /// An entry it waits for was dropped, so no run will meet the wait.
    Dropped(Wait),
    /// It waits for a run to complete, and that run stopped.
    Stopped { wait: Wait, run: String },
    /// The governing facts of its realm in the map it was queued with
    /// differ from the map on disk.
    RealmChanged {
        realm: String,
        differences: Vec<Difference>,
    },
    /// The map on disk cannot be read, so the realm cannot be compared.
    MapUnreadable(String),
}

impl Reason {
    /// Does only the operator clear it?
    pub(crate) fn holds(&self) -> bool {
        match self {
            Reason::NotStarted(_) | Reason::Unended { .. } => false,
            Reason::OperatorHold
            | Reason::Dropped(_)
            | Reason::Stopped { .. }
            | Reason::RealmChanged { .. }
            | Reason::MapUnreadable(_) => true,
        }
    }

    /// The reason's kind, as scripts read it.
    pub fn kind(&self) -> &'static str {
        match self {
            Reason::OperatorHold => "operator_hold",
            Reason::NotStarted(_) => "not_started",
            Reason::Unended { .. } => "unended",
            Reason::Dropped(_) => "dropped",
            Reason::Stopped { .. } => "stopped",
            Reason::RealmChanged { .. } => "realm_changed",
            Reason::MapUnreadable(_) => "map_unreadable",
        }
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let never = "which can never hold";
        match self {
            Reason::OperatorHold => write!(f, "held by the operator"),
            Reason::NotStarted(wait) => {
                write!(
                    f,
                    "waits on {wait}: entry {} has not started a run",
                    wait.entry
                )
            }
            Reason::Unended { wait, run } => {
                write!(f, "waits on {wait}: its run '{run}' has not ended")
            }
            Reason::Dropped(wait) => {
                write!(
                    f,
                    "waits on {wait}, {never}: entry {} was dropped",
                    wait.entry
                )
            }
            Reason::Stopped { wait, run } => {
                write!(f, "waits on {wait}, {never}: its run '{run}' stopped")
            }
            Reason::RealmChanged { realm, differences } => {
                let each: Vec<String> = differences.iter().map(ToString::to_string).collect();
                write!(f, "realm {realm} changed since queued: {}", each.join("; "))
            }
            Reason::MapUnreadable(detail) => write!(
                f,
                "the realms map it was queued under cannot be read now: {detail}"
            ),
        }
    }
}

/// One governing fact of a realm that differs between the map an entry
/// was queued with and the map on disk. Each is compared whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Difference {
    /// The map names the operated repository as another realm, or none.
    Realm {
        was: Option<String>,
        now: Option<String>,
    },
    GrantAdded(String),
    GrantRemoved(String),
    /// A grant under the same name that is not the same grant, by any
    /// key it holds.
    GrantChanged(String),
    Boundary {
        was: Boundary,
        now: Boundary,
    },
    /// The house rules' text.
    House,
    /// The dialect, or the instructions it renders.
    Dialect,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let named = |realm: &Option<String>| realm.clone().unwrap_or_else(|| "none".into());
        match self {
            Difference::Realm { was, now } => {
                write!(f, "the repository's realm {} → {}", named(was), named(now))
            }
            Difference::GrantAdded(name) => write!(f, "grant {name} added"),
            Difference::GrantRemoved(name) => write!(f, "grant {name} removed"),
            Difference::GrantChanged(name) => write!(f, "grant {name} changed"),
            Difference::Boundary { was, now } => write!(f, "boundary {was} → {now}"),
            Difference::House => write!(f, "house rules changed"),
            Difference::Dialect => write!(f, "dialect changed"),
        }
    }
}

/// Where an entry stands at admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Nothing stops it starting now.
    Admissible,
    /// Every reason clears by itself.
    Waiting,
    /// A reason only the operator clears.
    Held,
}

impl Standing {
    pub fn word(self) -> &'static str {
        match self {
            Standing::Admissible => "admissible",
            Standing::Waiting => "waiting",
            Standing::Held => "held",
        }
    }
}

/// Admission's word on one waiting entry: every reason it may not start
/// now, the operator's hold first, then its waits in order, then its
/// realm. None, and it may.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub reasons: Vec<Reason>,
}

impl Verdict {
    pub fn standing(&self) -> Standing {
        match (
            self.reasons.is_empty(),
            self.reasons.iter().any(Reason::holds),
        ) {
            (true, _) => Standing::Admissible,
            (false, false) => Standing::Waiting,
            (false, true) => Standing::Held,
        }
    }
}

/// One entry of the queue as the pass read it: the entry, its launch,
/// and admission's verdict while it waits; `None` once it started a run.
#[derive(Debug)]
pub struct Judged {
    pub entry: QueueEntry,
    pub launch: QueuedLaunch,
    pub verdict: Option<Verdict>,
}

/// Why the pass could not judge the queue at all. Nothing is admitted.
#[derive(Debug, Error)]
pub enum AdmissionError {
    #[error(transparent)]
    Store(#[from] StoreError),
    /// An entry whose launch cannot be read, or whose held world does not
    /// answer for itself.
    #[error("queue entry {entry}")]
    Entry {
        entry: EntryId,
        #[source]
        source: LaunchError,
    },
    /// An awaited run whose journal does not fold.
    #[error("folding run '{run}'")]
    Fold {
        run: String,
        #[source]
        source: FoldError,
    },
}

/// Judge the queue: every entry `brokkr queue list` lists, in its order,
/// with a verdict for each one that still waits.
pub fn pass(store: &Store) -> Result<Vec<Judged>, AdmissionError> {
    store
        .queue_list()?
        .into_iter()
        .map(|entry| {
            let launch =
                QueuedLaunch::decode(&entry.payload).map_err(|source| AdmissionError::Entry {
                    entry: entry.id,
                    source,
                })?;
            let verdict = match entry.state {
                EntryState::Queued | EntryState::Held => Some(judge(store, &entry, &launch)?),
                EntryState::Claimed { .. } | EntryState::Dropped => None,
            };
            Ok(Judged {
                entry,
                launch,
                verdict,
            })
        })
        .collect()
}

fn judge(
    store: &Store,
    entry: &QueueEntry,
    launch: &QueuedLaunch,
) -> Result<Verdict, AdmissionError> {
    let mut reasons = Vec::new();
    if entry.state == EntryState::Held {
        reasons.push(Reason::OperatorHold);
    }
    for wait in &entry.waits {
        reasons.extend(awaiting(*wait, observe(store, wait.entry)?));
    }
    let worlds = launch
        .held_and_now()
        .map_err(|source| AdmissionError::Entry {
            entry: entry.id,
            source,
        })?;
    reasons.extend(worlds.and_then(|worlds| realm_drift(&worlds)));
    Ok(Verdict { reasons })
}

/// What an awaited entry has come to.
#[derive(Debug)]
enum Awaited {
    /// Queued or held: no run yet.
    Unstarted,
    Dropped,
    /// It started this run, which stands so.
    Run {
        run: String,
        status: Status,
    },
}

/// Read what `entry` has come to: its standing, and its run's status.
fn observe(store: &Store, entry: EntryId) -> Result<Awaited, AdmissionError> {
    Ok(match store.queue_entry(entry)?.state {
        EntryState::Queued | EntryState::Held => Awaited::Unstarted,
        EntryState::Dropped => Awaited::Dropped,
        EntryState::Claimed { run } => {
            let events = store.load(&run)?;
            let status = match fold(&events) {
                Ok(state) => state.status,
                Err(source) => return Err(AdmissionError::Fold { run, source }),
            };
            Awaited::Run { run, status }
        }
    })
}

/// Why `wait` does not hold of what its entry came to, or `None` when it
/// holds.
fn awaiting(wait: Wait, awaited: Awaited) -> Option<Reason> {
    use WaitOn::{Completed, Ended};
    match (awaited, wait.on) {
        (Awaited::Unstarted, Completed | Ended) => Some(Reason::NotStarted(wait)),
        (Awaited::Dropped, Completed | Ended) => Some(Reason::Dropped(wait)),
        (
            Awaited::Run {
                status: Status::Completed,
                ..
            },
            Completed | Ended,
        )
        | (
            Awaited::Run {
                status: Status::Stopped,
                ..
            },
            Ended,
        ) => None,
        (
            Awaited::Run {
                run,
                status: Status::Stopped,
            },
            Completed,
        ) => Some(Reason::Stopped { wait, run }),
        (
            Awaited::Run {
                run,
                status: Status::Running | Status::AwaitingOperator,
            },
            Completed | Ended,
        ) => Some(Reason::Unended { wait, run }),
    }
}

/// The facts of the operated repository's realm that govern a run in it
/// (the operator's realm-drift ruling, 2026-10-04): which realm it is, its
/// boundary, its grants, its house rules and its dialect. Each is held
/// WHOLE, so a fact a later map version adds to any of them, a grant key
/// above all, is compared without this code naming it. A repository the
/// map does not name is governed as a run with no realm is: `namespace`,
/// no grant, no house, no dialect.
#[derive(Debug)]
struct Governing {
    realm: Option<String>,
    boundary: Boundary,
    grants: BTreeMap<String, CapabilityGrant>,
    house: Value,
    dialect: Value,
}

fn governing(world: &World, repo: &Path) -> Result<Governing, WorldError> {
    let pin = world.pin(Some(repo))?;
    let realm = world.realm_for(repo);
    Ok(Governing {
        realm: realm.map(|realm| realm.name.clone()),
        boundary: world.boundary_for(repo),
        grants: realm.map(|realm| realm.grants.clone()).unwrap_or_default(),
        house: unplaced(&pin, "house"),
        dialect: unplaced(&pin, "dialect"),
    })
}

/// A text a world pins, whole but for the path it was read from: the
/// same text read from the same map is the same fact wherever the map is
/// opened from. `null` when the realm names none.
fn unplaced(pin: &Value, key: &str) -> Value {
    let mut text = pin[key].clone();
    if let Value::Object(fields) = &mut text {
        fields.remove("source");
    }
    text
}

/// The hold a realm that changed since the entry was queued puts on it,
/// or a map on disk that cannot be read to tell.
fn realm_drift(worlds: &HeldAndNow) -> Option<Reason> {
    let facts = |world: &World| governing(world, &worlds.repo).map_err(|error| error.to_string());
    let compared = facts(&worlds.held).and_then(|held| {
        let now = worlds.now.as_ref().map_err(ToString::to_string);
        let now = now.and_then(facts)?;
        Ok((
            held.realm.clone().or(now.realm.clone()),
            differences(&held, &now),
        ))
    });
    match compared {
        Ok((_, differences)) if differences.is_empty() => None,
        Ok((realm, differences)) => Some(Reason::RealmChanged {
            realm: realm.unwrap_or_default(),
            differences,
        }),
        Err(detail) => Some(Reason::MapUnreadable(detail)),
    }
}

/// Every governing fact that is not the same, in the order the ruling
/// names them. Grants compare as whole grants.
fn differences(held: &Governing, now: &Governing) -> Vec<Difference> {
    let mut found = Vec::new();
    if held.realm != now.realm {
        found.push(Difference::Realm {
            was: held.realm.clone(),
            now: now.realm.clone(),
        });
    }
    let names: BTreeSet<&String> = held.grants.keys().chain(now.grants.keys()).collect();
    for name in names {
        match (held.grants.get(name), now.grants.get(name)) {
            (Some(_), None) => found.push(Difference::GrantRemoved(name.clone())),
            (None, Some(_)) => found.push(Difference::GrantAdded(name.clone())),
            (Some(was), Some(is)) if was != is => {
                found.push(Difference::GrantChanged(name.clone()))
            }
            (Some(_), Some(_)) | (None, None) => {}
        }
    }
    if held.boundary != now.boundary {
        found.push(Difference::Boundary {
            was: held.boundary,
            now: now.boundary,
        });
    }
    if held.house != now.house {
        found.push(Difference::House);
    }
    if held.dialect != now.dialect {
        found.push(Difference::Dialect);
    }
    found
}

#[cfg(test)]
mod tests;
