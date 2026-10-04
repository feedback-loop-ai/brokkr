//! The dispatcher's admission pass (decision 0068 ruling 3; #430's second
//! slice): for each entry still waiting, in the queue's order, whether it
//! may start now, and every reason it may not. [`pass`] reads the journal
//! and the realms maps and decides; it starts nothing and writes nothing,
//! so `brokkr queue list` shows its verdicts from a journal opened only to
//! read. [`judge`] is the same pass, and it latches what it finds.
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
//! A realm that changed is a fact about the past, so its hold LATCHES (the
//! operator's ruling of 2026-10-04): [`judge`] records what it found in the
//! queue's own append-only storage, journaled, and from then on the entry
//! is held by that record, whatever the map on disk comes to, a map put
//! back as it was queued included. Only [`release`] (`brokkr queue repin`)
//! clears it, and only under the map the latch found: a map edited since
//! is refused, and judged again before it is taken. Dropping the entry,
//! and queuing its launch afresh as a new one, ends it too. An entry queued
//! under no map is
//! compared like any other: a map that names its repository now is a
//! change. A map that cannot be read is no finding, so its hold does not
//! latch; it holds for as long as it cannot be read.
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
use brokkr_store::{Attribution, EntryId, EntryState, QueueEntry, Store, StoreError, Wait, WaitOn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    /// differ from the map on disk, and no latch records it yet.
    RealmChanged {
        realm: String,
        differences: Vec<Difference>,
    },
    /// A realm-drift hold latched on it: the differences the latch found,
    /// and whether the map on disk has changed since it was found.
    RealmLatched {
        realm: String,
        differences: Vec<Difference>,
        moved: bool,
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
            | Reason::RealmLatched { .. }
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
            Reason::RealmLatched { .. } => "realm_latched",
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
                write!(
                    f,
                    "realm {realm} changed since queued: {}",
                    each(differences)
                )
            }
            Reason::RealmLatched {
                realm,
                differences,
                moved,
            } => {
                let found = match differences.is_empty() {
                    true => "no difference now".to_string(),
                    false => each(differences),
                };
                write!(
                    f,
                    "realm {realm} changed since queued, latched until the operator re-pins, \
                     re-queues or drops it: {found}"
                )?;
                match moved {
                    true => write!(
                        f,
                        "; the map on disk has changed since, and `brokkr queue judge` latches \
                         what it finds now"
                    ),
                    false => Ok(()),
                }
            }
            Reason::MapUnreadable(detail) => write!(
                f,
                "the realms map it was queued under cannot be read now: {detail}"
            ),
        }
    }
}

/// Each difference, as an operator reads them in a line.
fn each(differences: &[Difference]) -> String {
    let each: Vec<String> = differences.iter().map(ToString::to_string).collect();
    each.join("; ")
}

/// One governing fact of a realm that differs between the map an entry
/// was queued with and the map on disk. Each is compared whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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

/// What a latch records (#430): the realm, the differences admission
/// found against the map the entry was queued with, and the digest of the
/// pin a re-pin to the map it read would write, `None` where no map was
/// there. Encoded with its version first, as a queued launch is, and a
/// latch in any other is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Finding {
    encoding: FindingEncoding,
    realm: String,
    differences: Vec<Difference>,
    on_disk: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum FindingEncoding {
    #[serde(rename = "realm-drift/v1")]
    V1,
}

impl Finding {
    /// The hold this latched finding puts on its entry.
    fn latched(self, moved: bool) -> Reason {
        Reason::RealmLatched {
            realm: self.realm,
            differences: self.differences,
            moved,
        }
    }

    /// The finding `entry`'s latch records.
    fn read(entry: EntryId, latch: &str) -> Result<Finding, AdmissionError> {
        serde_json::from_str(latch).map_err(|source| AdmissionError::Latch { entry, source })
    }
}

/// The differences an operator's re-pin accepted, as it says them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Released(pub Vec<Difference>);

impl fmt::Display for Released {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.is_empty() {
            true => write!(f, "no difference: the realm is as it was queued"),
            false => write!(f, "{}", each(&self.0)),
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
    /// A latch this brokkr cannot read.
    #[error("queue entry {entry}'s latched hold cannot be read")]
    Latch {
        entry: EntryId,
        #[source]
        source: serde_json::Error,
    },
    /// A re-pin of an entry no realm-drift hold is latched on.
    #[error(
        "queue entry {0} holds no latched realm drift to release; `brokkr queue judge` latches \
         what it finds"
    )]
    NothingLatched(EntryId),
    /// A re-pin under a map that is not the one the latch found.
    #[error(
        "the realms map on disk is not the one queue entry {0}'s latched hold found; \
         `brokkr queue judge` shows and latches what differs now"
    )]
    MapMoved(EntryId),
}

/// Judge the queue: every entry `brokkr queue list` lists, in its order,
/// with a verdict for each one that still waits. Nothing is written.
pub fn pass(store: &Store) -> Result<Vec<Judged>, AdmissionError> {
    Ok(read(store)?.into_iter().map(|(judged, _)| judged).collect())
}

/// Judge the queue as [`pass`] does, and first latch on each waiting
/// entry the realm drift it finds that no latch records yet, or that the
/// map on disk has moved past since its latch, journaling each `latch` as
/// `by` asks it. The verdicts are the queue's after the latches.
pub fn judge(store: &mut Store, by: Attribution<'_>) -> Result<Vec<Judged>, AdmissionError> {
    for (judged, finding) in read(store)? {
        if let Some(finding) = finding {
            let finding = json!(finding).to_string();
            store.queue_latch(judged.entry.id, &finding, by)?;
        }
    }
    pass(store)
}

/// Release a waiting entry from its latched realm-drift hold: re-pin it
/// to the map that would govern it now, journaling the `repin` as `by`
/// asks it, and say the differences accepted. Refused with no latch on
/// the entry, and when the map that would be pinned is not the one the
/// latch found, so a map edited since it was judged is never taken unseen.
pub fn release(
    store: &mut Store,
    entry: EntryId,
    by: Attribution<'_>,
) -> Result<Released, AdmissionError> {
    let queued = store.queue_entry(entry)?;
    let latch = queued.latch.ok_or(AdmissionError::NothingLatched(entry))?;
    let finding = Finding::read(entry, &latch.finding)?;
    let (payload, digest) = QueuedLaunch::decode(&queued.payload)
        .and_then(QueuedLaunch::repinned)
        .and_then(|launch| {
            launch
                .encode()
                .map(|payload| (payload, launch.map.digest()))
        })
        .map_err(|source| AdmissionError::Entry { entry, source })?;
    if digest != finding.on_disk {
        return Err(AdmissionError::MapMoved(entry));
    }
    store.queue_repin(entry, &payload, by)?;
    Ok(Released(finding.differences))
}

/// Every entry the queue lists, judged, each waiting one with the
/// finding a writing pass would latch on it.
fn read(store: &Store) -> Result<Vec<(Judged, Option<Finding>)>, AdmissionError> {
    store
        .queue_list()?
        .into_iter()
        .map(|entry| {
            let launch =
                QueuedLaunch::decode(&entry.payload).map_err(|source| AdmissionError::Entry {
                    entry: entry.id,
                    source,
                })?;
            let (verdict, finding) = match entry.state {
                EntryState::Queued | EntryState::Held => {
                    let (verdict, finding) = weigh(store, &entry, &launch)?;
                    (Some(verdict), finding)
                }
                EntryState::Claimed { .. } | EntryState::Dropped => (None, None),
            };
            let judged = Judged {
                entry,
                launch,
                verdict,
            };
            Ok((judged, finding))
        })
        .collect()
}

fn weigh(
    store: &Store,
    entry: &QueueEntry,
    launch: &QueuedLaunch,
) -> Result<(Verdict, Option<Finding>), AdmissionError> {
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
    let latched = entry
        .latch
        .as_ref()
        .map(|latch| Finding::read(entry.id, &latch.finding))
        .transpose()?;
    let (realm, finding) = realm(latched, sight(&worlds));
    reasons.extend(realm);
    Ok((Verdict { reasons }, finding))
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
/// map does not name, or one under no map at all, is governed as a run
/// with no realm is: `namespace`, no grant, no house, no dialect.
#[derive(Debug)]
struct Governing {
    realm: Option<String>,
    boundary: Boundary,
    grants: BTreeMap<String, CapabilityGrant>,
    house: Value,
    dialect: Value,
}

/// The facts that govern a run in `repo` under `world`, or under no map.
fn governing(world: Option<&World>, repo: &Path) -> Result<Governing, WorldError> {
    let Some(world) = world else {
        return Ok(Governing {
            realm: None,
            boundary: Boundary::Namespace,
            grants: BTreeMap::new(),
            house: Value::Null,
            dialect: Value::Null,
        });
    };
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

/// What admission sees of an entry's realm now.
enum Sight {
    /// What differs from the world it was queued with.
    Seen(Finding),
    /// Why the map now cannot be read to tell, as the operator reads it.
    Unreadable(String),
}

fn sight(worlds: &HeldAndNow) -> Sight {
    let held = governing(worlds.held.as_ref(), &worlds.repo);
    let now = worlds.now.as_ref().map(|now| {
        let facts = governing(now.world.as_ref(), &worlds.repo);
        (facts, &now.digest)
    });
    match (held, now) {
        (Ok(held), Ok((Ok(now), on_disk))) => Sight::Seen(Finding {
            encoding: FindingEncoding::V1,
            realm: held.realm.clone().or(now.realm.clone()).unwrap_or_default(),
            differences: differences(&held, &now),
            on_disk: on_disk.clone(),
        }),
        (Err(error), _) | (Ok(_), Ok((Err(error), _))) => Sight::Unreadable(error.to_string()),
        (Ok(_), Err(error)) => Sight::Unreadable(error.to_string()),
    }
}

/// The holds an entry's realm puts on it, from the latch that stands on
/// it and what admission sees now, and the finding a writing pass
/// latches: a drift no latch records yet, or the map on disk as it is now
/// when it has moved since the latch was found. A latch holds whatever
/// is seen, and nothing seen clears it.
fn realm(latched: Option<Finding>, sight: Sight) -> (Vec<Reason>, Option<Finding>) {
    match (latched, sight) {
        (None, Sight::Unreadable(detail)) => (vec![Reason::MapUnreadable(detail)], None),
        (Some(latched), Sight::Unreadable(detail)) => (
            vec![latched.latched(false), Reason::MapUnreadable(detail)],
            None,
        ),
        (None, Sight::Seen(seen)) if seen.differences.is_empty() => (vec![], None),
        (None, Sight::Seen(seen)) => {
            let changed = Reason::RealmChanged {
                realm: seen.realm.clone(),
                differences: seen.differences.clone(),
            };
            (vec![changed], Some(seen))
        }
        (Some(latched), Sight::Seen(seen)) if latched.on_disk == seen.on_disk => {
            (vec![latched.latched(false)], None)
        }
        (Some(latched), Sight::Seen(seen)) => (vec![latched.latched(true)], Some(seen)),
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
