//! The capacity half of decision 0068 ruling 3 (#430's third slice): an
//! entry nothing else stops starts only when the machine has room for it,
//! as its host configuration ([`super::host`]) declares that room.
//!
//! What a run seats is read from its manifest, one derivation for both
//! sides: the entry's, compiled as its start would compile it, and each
//! running run's, as the journal pinned it. A seat is each candidate the
//! manifest's capability records name, fallbacks included, since any of
//! them may be seated: its provider, and the route its compile pinned for
//! each concrete model id it may be served (`run-manifest/v13`): the one
//! its driver serves the id on, the id's own prefix before the first `/`
//! (decision 0036 ruling 2) or the driver's default route, as dsh serves a
//! bare id on `deepseek-official`. Nothing is resolved again through
//! anyone's adapters, so a run is counted by what its own compile seated
//! whichever workspace admits beside it; a manifest that does not pin what
//! it seats, or pins it unreadably, cannot be measured. A run counts once
//! against each provider and route it seats; a seat on a `shared-local`
//! route counts against that route alone, in place of its provider (ruling
//! 5). A run builds in a box when any of its hands sites stands behind a
//! boxed boundary.
//!
//! The checks run in a fixed order, and the first unmet one is the reason
//! recorded: the host configuration is declared and readable; the entry's
//! seats can be measured, and every running run says what it seats; every
//! provider and route the entry seats is declared; each provider and
//! cloud route is below its ceiling; each shared-local route is below its
//! own; each free-space floor, in the file's order, finds the free bytes
//! of the filesystem it names at or above it, each judged by its own
//! number though another on the same filesystem holds; and, for an entry
//! that builds in a box, the boxed builds running are below their
//! ceiling. Nothing here latches: each pass measures afresh.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, Status};
use brokkr_core::realms::Boundary;
use brokkr_store::{Claim, EntryState, Generation, QueueEntry, Store, Wait};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use super::host::{self, At, HostConfig, Hosting, Place, Provider, Route, RouteClass};
use super::{awaiting, came_to, AdmissionError, Judged, Reason};
use crate::capabilities::manifest::ModelPins;
use crate::launch::{LaunchError, QueuedLaunch};
use census::{census, Census};
use journal::Journal;

mod census;
mod journal;

/// The machine an admission pass measures, its effects injected: where
/// its host configuration is, the temp directory a `temp` floor measures,
/// and how the free bytes under a path are measured.
pub struct Host<'a> {
    pub file: &'a Path,
    pub temp: &'a Path,
    pub free: &'a dyn Fn(&Path) -> io::Result<u64>,
}

/// Why the machine has no room for an entry now: the limit, and what was
/// measured against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capacity {
    /// Nothing at the host configuration's path declares capacity.
    Undeclared(PathBuf),
    /// The host configuration's path holds something that cannot be read.
    Unreadable {
        path: PathBuf,
        kind: ErrorKind,
    },
    /// What the entry seats cannot be measured.
    Unmeasured(Unmeasurable),
    /// A running run whose manifest does not say what it seats.
    RunUnmeasured {
        run: String,
        why: Unmeasurable,
    },
    ProviderUndeclared(String),
    RouteUndeclared {
        provider: String,
        route: String,
    },
    ProviderFull {
        provider: String,
        running: usize,
        ceiling: u32,
    },
    RouteFull {
        provider: String,
        route: String,
        class: RouteClass,
        running: usize,
        ceiling: u32,
    },
    /// The filesystem a free-space floor names cannot be measured.
    FloorUnmeasured {
        at: Measured,
        kind: ErrorKind,
    },
    FloorLow {
        at: Measured,
        free: u64,
        floor: u64,
    },
    BoxedFull {
        running: usize,
        ceiling: u32,
    },
}

/// Why what a run seats cannot be measured.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unmeasurable {
    /// The entry's bundle does not compile as its start would compile it.
    #[error("{0}")]
    Compile(Shared<LaunchError>),
    /// A manifest's capability records do not pin what it seats.
    #[error("{0}")]
    Manifest(Shared<serde_json::Error>),
    /// A site's model pin could not be read as one concrete id, on the
    /// flags named; an agent candidate's names none.
    #[error(
        "site '{site}' pins a model that cannot be read as one concrete id{}",
        on_flags(.flags)
    )]
    Pin { site: String, flags: Vec<String> },
}

/// The flags a pin could not be read on, where it names any.
fn on_flags(flags: &[String]) -> String {
    match flags.is_empty() {
        true => String::new(),
        false => format!(" on {}", flags.join(", ")),
    }
}

/// An error a reason carries whole, shared by its clones: equal only to
/// itself, because an error has no equality of its own and its text is
/// not one (decision 0071 ruling 8).
#[derive(Debug)]
pub struct Shared<E>(Arc<E>);

impl<E> Clone for Shared<E> {
    fn clone(&self) -> Shared<E> {
        Shared(Arc::clone(&self.0))
    }
}

impl<E> PartialEq for Shared<E> {
    fn eq(&self, other: &Shared<E>) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<E> Eq for Shared<E> {}

impl<E: fmt::Display> fmt::Display for Shared<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Capacity {
    /// Does only the operator clear it? A host configuration that cannot
    /// be read, and an entry whose seats cannot be measured, wait on a
    /// mend nothing but the operator makes.
    pub(crate) fn holds(&self) -> bool {
        matches!(self, Capacity::Unreadable { .. } | Capacity::Unmeasured(_))
    }

    /// The reason's kind, as scripts read it.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Capacity::Undeclared(_) => "host_undeclared",
            Capacity::Unreadable { .. } => "host_unreadable",
            Capacity::Unmeasured(_) => "unmeasured",
            Capacity::RunUnmeasured { .. } => "run_unmeasured",
            Capacity::ProviderUndeclared(_) => "provider_undeclared",
            Capacity::RouteUndeclared { .. } => "route_undeclared",
            Capacity::ProviderFull { .. } => "provider_full",
            Capacity::RouteFull { .. } => "route_full",
            Capacity::FloorUnmeasured { .. } => "floor_unmeasured",
            Capacity::FloorLow { .. } => "floor_low",
            Capacity::BoxedFull { .. } => "boxed_full",
        }
    }
}

impl fmt::Display for Capacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let against = |running: &usize, ceiling: &u32| {
            format!("{running} running against a ceiling of {ceiling}")
        };
        match self {
            Capacity::Undeclared(path) => write!(
                f,
                "no host configuration at {} declares capacity",
                path.display()
            ),
            Capacity::Unreadable { path, kind } => write!(
                f,
                "the host configuration at {} cannot be read: {kind}",
                path.display()
            ),
            Capacity::Unmeasured(why) => write!(f, "what it seats cannot be measured: {why}"),
            Capacity::RunUnmeasured { run, why } => {
                write!(f, "running run '{run}' does not say what it seats: {why}")
            }
            Capacity::ProviderUndeclared(provider) => write!(
                f,
                "the host configuration declares no ceiling for provider {provider}"
            ),
            Capacity::RouteUndeclared { provider, route } => write!(
                f,
                "the host configuration declares no ceiling for route {route} of provider \
                 {provider}"
            ),
            Capacity::ProviderFull {
                provider,
                running,
                ceiling,
            } => write!(
                f,
                "provider {provider} is full: {}",
                against(running, ceiling)
            ),
            Capacity::RouteFull {
                provider,
                route,
                class,
                running,
                ceiling,
            } => write!(
                f,
                "{class} route {route} of provider {provider} is full: {}",
                against(running, ceiling)
            ),
            Capacity::FloorUnmeasured { at, kind } => {
                write!(f, "{at} cannot be measured: {kind}")
            }
            Capacity::FloorLow { at, free, floor } => {
                write!(f, "{at} has {free} bytes free, below its floor of {floor}")
            }
            Capacity::BoxedFull { running, ceiling } => {
                write!(f, "boxed builds are full: {}", against(running, ceiling))
            }
        }
    }
}

/// Judge the machine's room for each entry in `queue` that nothing else
/// stops, recording the first unmet check as its reason. The host
/// configuration is read, and the running runs counted, once, and only
/// when some entry is judged; the unstarted runs are named on every pass,
/// by the one reading of the journal's runs it makes.
pub(super) fn sized(
    store: &Store,
    host: &Host<'_>,
    mut queue: Vec<Judged>,
) -> Result<(Vec<Judged>, Vec<Reason>), AdmissionError> {
    let unstopped = |judged: &&mut Judged| {
        judged
            .verdict
            .as_ref()
            .is_some_and(|verdict| verdict.reasons.is_empty())
    };
    if !queue.iter_mut().any(|judged| unstopped(&judged)) {
        return Ok((queue, census(store, false)?.unstarted));
    }
    let measure = Measure::take(store, host)?;
    for judged in queue.iter_mut().filter(unstopped) {
        match (
            &mut judged.verdict,
            measure.room(&judged.entry, &judged.launch),
        ) {
            (Some(verdict), Err(unmet)) => verdict.reasons.push(super::Reason::Capacity(unmet)),
            (_, room) => judged.room = room.ok(),
        }
    }
    Ok((queue, measure.unstarted))
}

/// What one pass measured of the machine.
struct Measure<'h> {
    host: &'h Host<'h>,
    hosting: Hosting,
    /// The journal measured, read before what it holds.
    journal: Option<Journal>,
    /// The events high-water, read before the census.
    high_water: i64,
    live: Arc<Vec<Live>>,
    unstarted: Vec<Reason>,
}

impl<'h> Measure<'h> {
    /// Read the host configuration, and the journal's runs: the unstarted
    /// ones, and when it declares capacity, the ones running now.
    fn take(store: &Store, host: &'h Host<'h>) -> Result<Measure<'h>, AdmissionError> {
        let hosting = host::read(host.file)?;
        let journal = Journal::of(store)?;
        let high_water = store.high_water()?;
        let counted = matches!(hosting, Hosting::Declared(_));
        let Census { live, unstarted } = census(store, counted)?;
        Ok(Measure {
            host,
            hosting,
            journal,
            high_water,
            live: Arc::new(live),
            unstarted,
        })
    }

    /// Each check in its order; the first unmet one, or the room found for
    /// `read`, as the pass read it.
    fn room(&self, read: &QueueEntry, launch: &QueuedLaunch) -> Result<Room, Capacity> {
        let config = self.declared()?;
        let entry = Entry::measure(launch).map_err(Capacity::Unmeasured)?;
        let running = ceilings(config, self.live.iter(), &entry.lanes)?;
        floors(config, self.host, entry.scratch)?;
        boxed(config, &running, entry.boxed)?;
        Ok(Room {
            journal: self.journal,
            entry: read.clone(),
            high_water: self.high_water,
            digest: entry.digest,
            lanes: entry.lanes,
            boxed: entry.boxed,
            config: config.clone(),
            census: Arc::clone(&self.live),
        })
    }

    /// The host configuration, or why it does not declare capacity.
    fn declared(&self) -> Result<&HostConfig, Capacity> {
        let path = self.host.file.to_path_buf();
        match &self.hosting {
            Hosting::Absent => Err(Capacity::Undeclared(path)),
            Hosting::Unreadable(kind) => Err(Capacity::Unreadable { path, kind: *kind }),
            Hosting::Declared(config) => Ok(config),
        }
    }
}

/// The entry's lanes bound to `config`, each below its ceiling with the
/// runs of `live` counted: each provider and cloud route, then each
/// shared-local route; and what runs, as counted.
fn ceilings<'a>(
    config: &HostConfig,
    live: impl IntoIterator<Item = &'a Live>,
    lanes: &BTreeSet<Lane>,
) -> Result<Running, Capacity> {
    let running = Running::count(live, config)?;
    let bound = bind(config, lanes)?;
    bound
        .iter()
        .filter(|bound| bound.class() == RouteClass::Cloud)
        .try_for_each(|bound| bound.below(&running, true))?;
    bound
        .iter()
        .filter(|bound| bound.class() == RouteClass::SharedLocal)
        .try_for_each(|bound| bound.below(&running, false))?;
    Ok(running)
}

/// For an entry that builds in a box, the boxed builds running below
/// their ceiling.
fn boxed(config: &HostConfig, running: &Running, boxed: bool) -> Result<(), Capacity> {
    match (boxed, config.boxed_builds.ceiling.get()) {
        (true, ceiling) if running.boxed >= ceiling as usize => Err(Capacity::BoxedFull {
            running: running.boxed,
            ceiling,
        }),
        _ => Ok(()),
    }
}

/// The room a pass found for the one entry it measured it for, as that
/// entry's claim re-checks it (decision 0095 ruling 7): the journal the
/// pass measured; the entry as the pass read it, its id and the pin and
/// latch it was seen under; the events high-water the pass read before
/// its census, the census, the manifest digest of the bundle it compiled
/// and what that bundle seats, under the host configuration it read.
#[derive(Debug)]
pub struct Room {
    journal: Option<Journal>,
    entry: QueueEntry,
    high_water: i64,
    digest: String,
    lanes: BTreeSet<Lane>,
    boxed: bool,
    config: HostConfig,
    census: Arc<Vec<Live>>,
}

/// Why a claim's re-check refused it.
enum Recheck {
    /// A wait no longer holds, or the entry has no room now.
    Refused(Reason),
    /// A run read again does not fold.
    Failed(Box<AdmissionError>),
}

/// An awaited entry, as the claim's transaction reads it.
type Awaiting = (Wait, EntryState, Vec<EventEnvelope>);

impl Room {
    /// Claim the entry this room was measured for, as the pass read it,
    /// for the dispatcher of `generation`, journaling `reason`, and record
    /// this room's digest on the claim, in the journal the pass measured
    /// alone, by its id ([`Room::judge`]). Inside the claim's transaction the
    /// entry must still be queued under the pin and latch the pass read,
    /// each of its waits must hold of what its awaited entry has come to,
    /// and each run with an event past the pass's high-water is folded
    /// again and counted in place of what the census held of it, and the
    /// entry's lanes and the boxed builds are judged again by its
    /// ceilings: an unmet wait or one at a ceiling now is the inner
    /// refusal, and nothing is claimed. Free space is the pass's to judge,
    /// and is not measured again.
    pub fn claim(
        &self,
        store: &mut Store,
        generation: Generation,
        reason: &str,
    ) -> Result<Result<Claim, Reason>, AdmissionError> {
        let claimed = store.queue_claim_with(
            &self.entry,
            self.high_water,
            &self.digest,
            generation,
            reason,
            self.judge(store.path()),
        );
        match claimed? {
            Ok(claim) => Ok(Ok(claim)),
            Err(Recheck::Refused(reason)) => Ok(Err(reason)),
            Err(Recheck::Failed(error)) => Err(*error),
        }
    }

    /// The entry's waits judged again, then the census corrected by the
    /// runs that `moved`, and the entry's room judged again against it.
    fn recheck(
        &self,
        waits: &[Awaiting],
        moved: &[(String, Vec<EventEnvelope>, String)],
    ) -> Result<(), Recheck> {
        let failed = |error| Recheck::Failed(Box::new(error));
        for (wait, state, events) in waits {
            let awaited = came_to(state.clone(), events).map_err(failed)?;
            if let Some(unmet) = awaiting(*wait, awaited) {
                return Err(Recheck::Refused(unmet));
            }
        }
        let full = |capacity| Recheck::Refused(Reason::Capacity(capacity));
        let mut now = Vec::new();
        for (run, events, manifest) in moved {
            let state = fold(events).map_err(|source| {
                let run = run.clone();
                failed(AdmissionError::Fold { run, source })
            })?;
            if state.run.status() == Status::Running {
                let seats = Seats::read(manifest);
                now.push(Live {
                    run: run.clone(),
                    seats,
                });
            }
        }
        let stood =
            (self.census.iter()).filter(|live| moved.iter().all(|(run, ..)| *run != live.run));
        let running = ceilings(&self.config, stood.chain(&now), &self.lanes).map_err(full)?;
        boxed(&self.config, &running, self.boxed).map_err(full)
    }
}

/// Where a free-space floor was measured: the place the host
/// configuration names, at the path it resolved to, or the path it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Measured {
    Place { place: Place, path: PathBuf },
    Path(PathBuf),
}

impl Measured {
    /// Where `at` is measured for an entry whose seat scratch is
    /// `scratch`, on a host whose temp directory is `temp`.
    fn of(at: &At, scratch: &Path, temp: &Path) -> Measured {
        let place = |place: Place, path: &Path| Measured::Place {
            place,
            path: path.to_path_buf(),
        };
        match at {
            At::Place(Place::SeatScratch) => place(Place::SeatScratch, scratch),
            At::Place(Place::Temp) => place(Place::Temp, temp),
            At::Path(path) => Measured::Path(path.0.clone()),
        }
    }

    fn path(&self) -> &Path {
        match self {
            Measured::Place { path, .. } | Measured::Path(path) => path,
        }
    }
}

impl fmt::Display for Measured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Measured::Place { place, path } => {
                write!(f, "the {place} filesystem at {}", path.display())
            }
            Measured::Path(path) => write!(f, "the filesystem at {}", path.display()),
        }
    }
}

/// Each free-space floor, in the file's order, held: the free bytes on
/// the filesystem it names at or above it. Every floor is measured and
/// judged by its own number, whichever other floor shares its filesystem.
fn floors(config: &HostConfig, host: &Host<'_>, scratch: PathBuf) -> Result<(), Capacity> {
    config.floors.0.iter().try_for_each(|declared| {
        let at = Measured::of(&declared.at, &scratch, host.temp);
        let floor = declared.floor_bytes.get();
        match (host.free)(at.path()) {
            Err(error) => Err(Capacity::FloorUnmeasured {
                at,
                kind: error.kind(),
            }),
            Ok(free) if free < floor => Err(Capacity::FloorLow { at, free, floor }),
            Ok(_) => Ok(()),
        }
    })
}

/// What a run's manifest says it seats: the capability record of every
/// site, and the boundary each hands site stands behind. A projection of
/// a closed contract, so it reads only the keys it needs.
#[derive(Deserialize)]
struct Pinned {
    capabilities: Sites,
    #[serde(default)]
    boundary: BTreeMap<String, Boundary>,
}

#[derive(Deserialize)]
struct Sites {
    sites: BTreeMap<String, Site>,
}

#[derive(Deserialize)]
struct Site {
    candidates: Vec<Seated>,
}

/// One candidate a site may seat: its provider, and the model ids its
/// compile pinned for it.
#[derive(Deserialize)]
struct Seated {
    provider: String,
    model_pins: ModelPins,
}

/// Each lane one run seats, once, and whether it builds in a box.
#[derive(Debug)]
struct Seats {
    lanes: BTreeSet<Lane>,
    boxed: bool,
}

impl Seats {
    /// What `manifest` pins: a lane for each concrete id of each candidate,
    /// and one with no route for a candidate that takes no model.
    fn of(manifest: Value) -> Result<Seats, Unmeasurable> {
        Seats::pinned(serde_json::from_value(manifest))
    }

    /// [`Seats::of`] a manifest as the journal holds it.
    fn read(manifest: &str) -> Result<Seats, Unmeasurable> {
        Seats::pinned(serde_json::from_str(manifest))
    }

    fn pinned(pinned: Result<Pinned, serde_json::Error>) -> Result<Seats, Unmeasurable> {
        let pinned = pinned.map_err(|error| Unmeasurable::Manifest(Shared(Arc::new(error))))?;
        let mut lanes = BTreeSet::new();
        for (site, Site { candidates }) in pinned.capabilities.sites {
            for Seated {
                provider,
                model_pins,
            } in candidates
            {
                let served = match model_pins {
                    ModelPins::Read(served) => served,
                    ModelPins::Unreadable(flags) => return Err(Unmeasurable::Pin { site, flags }),
                };
                let routes: Vec<Option<String>> = match served.is_empty() {
                    true => vec![None],
                    false => served.into_iter().map(|served| served.route).collect(),
                };
                lanes.extend(routes.into_iter().map(|route| Lane {
                    provider: provider.clone(),
                    route,
                }));
            }
        }
        Ok(Seats {
            lanes,
            boxed: pinned.boundary.values().any(|boundary| boundary.is_boxed()),
        })
    }
}

/// A provider, and the route of it, if any, a seat takes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Lane {
    provider: String,
    route: Option<String>,
}

/// A running run, with what it seats or why its manifest does not say.
#[derive(Debug)]
struct Live {
    run: String,
    seats: Result<Seats, Unmeasurable>,
}

/// The entry as admission measures it: the lanes its compiled bundle
/// seats, whether it builds in a box, where its seats make scratch, and
/// the digest of the manifest measured.
struct Entry {
    lanes: BTreeSet<Lane>,
    boxed: bool,
    scratch: PathBuf,
    digest: String,
}

impl Entry {
    fn measure(launch: &QueuedLaunch) -> Result<Entry, Unmeasurable> {
        let unmeasured = |error| Unmeasurable::Compile(Shared(Arc::new(error)));
        let bundle = launch.compiled().map_err(unmeasured)?;
        let scratch = launch.scratch().map_err(unmeasured)?;
        let digest = bundle.manifest_digest();
        let Seats { lanes, boxed } = Seats::of(bundle.manifest)?;
        Ok(Entry {
            lanes,
            boxed,
            scratch,
            digest,
        })
    }
}

/// What runs now: the runs counted against each provider and each route,
/// and the boxed builds.
#[derive(Debug, Default)]
struct Running {
    providers: BTreeMap<String, usize>,
    routes: BTreeMap<(String, String), usize>,
    boxed: usize,
}

impl Running {
    /// Count `live` against `config`, or name the first running run that
    /// does not say what it seats.
    fn count<'a>(
        live: impl IntoIterator<Item = &'a Live>,
        config: &HostConfig,
    ) -> Result<Running, Capacity> {
        let mut running = Running::default();
        for Live { run, seats } in live {
            let seats = seats.as_ref().map_err(|why| Capacity::RunUnmeasured {
                run: run.clone(),
                why: why.clone(),
            })?;
            let providers: BTreeSet<&String> = (seats.lanes.iter())
                .filter(|lane| class(config, lane) == RouteClass::Cloud)
                .map(|lane| &lane.provider)
                .collect();
            for provider in providers {
                *running.providers.entry(provider.clone()).or_default() += 1;
            }
            for lane in &seats.lanes {
                if let Some(route) = &lane.route {
                    let key = (lane.provider.clone(), route.clone());
                    *running.routes.entry(key).or_default() += 1;
                }
            }
            running.boxed += usize::from(seats.boxed);
        }
        Ok(running)
    }
}

/// The class a lane's route is declared with: a lane with no route, or
/// one the configuration does not declare, is counted as cloud is.
fn class(config: &HostConfig, lane: &Lane) -> RouteClass {
    (lane.route.as_ref())
        .and_then(|route| config.providers.get(&lane.provider)?.routes.get(route))
        .map_or(RouteClass::Cloud, |route| route.class)
}

/// An entry's lane bound to the ceilings the configuration declares for
/// it.
struct Bound<'a> {
    lane: &'a Lane,
    provider: &'a Provider,
    route: Option<(&'a String, &'a Route)>,
}

/// Each of the entry's lanes bound to its declarations, or the first a
/// provider or route of which the configuration does not declare.
fn bind<'a>(config: &'a HostConfig, lanes: &'a BTreeSet<Lane>) -> Result<Vec<Bound<'a>>, Capacity> {
    lanes
        .iter()
        .map(|lane| {
            let provider = (config.providers.get(&lane.provider))
                .ok_or_else(|| Capacity::ProviderUndeclared(lane.provider.clone()))?;
            let route = (lane.route.as_ref())
                .map(|route| {
                    let declared = provider.routes.get_key_value(route);
                    declared.ok_or_else(|| Capacity::RouteUndeclared {
                        provider: lane.provider.clone(),
                        route: route.clone(),
                    })
                })
                .transpose()?;
            Ok(Bound {
                lane,
                provider,
                route,
            })
        })
        .collect()
}

impl Bound<'_> {
    fn class(&self) -> RouteClass {
        self.route
            .map_or(RouteClass::Cloud, |(_, route)| route.class)
    }

    /// The lane's provider, where `provider` asks it, and its route, each
    /// below its ceiling.
    fn below(&self, running: &Running, provider: bool) -> Result<(), Capacity> {
        let name = &self.lane.provider;
        let count = running.providers.get(name).copied().unwrap_or_default();
        let ceiling = self.provider.ceiling.get();
        if provider && count >= ceiling as usize {
            return Err(Capacity::ProviderFull {
                provider: name.clone(),
                running: count,
                ceiling,
            });
        }
        let Some((route, declared)) = self.route else {
            return Ok(());
        };
        let key = (name.clone(), route.clone());
        let count = running.routes.get(&key).copied().unwrap_or_default();
        let ceiling = declared.ceiling.get();
        match count >= ceiling as usize {
            true => Err(Capacity::RouteFull {
                provider: name.clone(),
                route: route.clone(),
                class: declared.class,
                running: count,
                ceiling,
            }),
            false => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
