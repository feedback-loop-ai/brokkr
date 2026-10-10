//! The capacity half of decision 0068 ruling 3 (#430's third slice): an
//! entry nothing else stops starts only when the machine has room for it,
//! as its host configuration ([`super::host`]) declares that room.
//!
//! What a run seats is read from its manifest, one derivation for both
//! sides: the entry's, compiled as its start would compile it, and each
//! running run's, as the journal pinned it. A seat is each candidate the
//! manifest's capability records name, fallbacks included, since any of
//! them may be seated: its provider, and the route of each concrete model
//! id its compile pinned for it (`run-manifest/v13`; the prefix before the
//! first `/`, decision 0036 ruling 2). Nothing is resolved again through
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
//! own; the scratch filesystem's free bytes are at or above its floor;
//! and, for an entry that builds in a box, the boxed builds running are
//! below their ceiling. Nothing here latches: each pass measures afresh.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brokkr_core::fold::{fold, Status};
use brokkr_core::realms::Boundary;
use brokkr_store::Store;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use super::host::{self, HostConfig, Hosting, Provider, Route, RouteClass};
use super::{AdmissionError, Judged};
use crate::capabilities::manifest::{route, ModelPins};
use crate::launch::{LaunchError, QueuedLaunch};

/// The machine an admission pass measures, its effects injected: where
/// its host configuration is, and how the free bytes under a path are
/// measured.
pub struct Host<'a> {
    pub file: &'a Path,
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
    /// The scratch filesystem cannot be measured.
    ScratchUnmeasured {
        path: PathBuf,
        kind: ErrorKind,
    },
    ScratchLow {
        path: PathBuf,
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
    /// A site's model pin could not be read as one concrete id.
    #[error(
        "site '{site}' pins a model that cannot be read as one concrete id on {}",
        .flags.join(", ")
    )]
    Pin { site: String, flags: Vec<String> },
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
            Capacity::ScratchUnmeasured { .. } => "scratch_unmeasured",
            Capacity::ScratchLow { .. } => "scratch_low",
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
            Capacity::ScratchUnmeasured { path, kind } => write!(
                f,
                "the scratch filesystem at {} cannot be measured: {kind}",
                path.display()
            ),
            Capacity::ScratchLow { path, free, floor } => write!(
                f,
                "the scratch filesystem at {} has {free} bytes free, below its floor of {floor}",
                path.display()
            ),
            Capacity::BoxedFull { running, ceiling } => {
                write!(f, "boxed builds are full: {}", against(running, ceiling))
            }
        }
    }
}

/// Judge the machine's room for each entry in `queue` that nothing else
/// stops, recording the first unmet check as its reason. The host
/// configuration is read, and the running runs counted, once, and only
/// when some entry is judged.
pub(super) fn sized(
    store: &Store,
    host: &Host<'_>,
    mut queue: Vec<Judged>,
) -> Result<Vec<Judged>, AdmissionError> {
    let unstopped = |judged: &&mut Judged| {
        judged
            .verdict
            .as_ref()
            .is_some_and(|verdict| verdict.reasons.is_empty())
    };
    if !queue.iter_mut().any(|judged| unstopped(&judged)) {
        return Ok(queue);
    }
    let measure = Measure::take(store, host)?;
    for judged in queue.iter_mut().filter(unstopped) {
        if let (Some(verdict), Err(unmet)) = (&mut judged.verdict, measure.room(&judged.launch)) {
            verdict.reasons.push(super::Reason::Capacity(unmet));
        }
    }
    Ok(queue)
}

/// What one pass measured of the machine.
struct Measure<'h> {
    host: &'h Host<'h>,
    hosting: Hosting,
    live: Vec<Live>,
}

impl<'h> Measure<'h> {
    /// Read the host configuration, and when it declares capacity, the
    /// runs running now.
    fn take(store: &Store, host: &'h Host<'h>) -> Result<Measure<'h>, AdmissionError> {
        let hosting = host::read(host.file)?;
        let live = match hosting {
            Hosting::Declared(_) => census(store)?,
            Hosting::Absent | Hosting::Unreadable(_) => Vec::new(),
        };
        Ok(Measure {
            host,
            hosting,
            live,
        })
    }

    /// Each check in its order; the first unmet one.
    fn room(&self, launch: &QueuedLaunch) -> Result<(), Capacity> {
        let config = self.declared()?;
        let entry = Entry::measure(launch).map_err(Capacity::Unmeasured)?;
        let running = Running::count(&self.live, config)?;
        let bound = bind(config, &entry.lanes)?;
        bound
            .iter()
            .filter(|bound| bound.class() == RouteClass::Cloud)
            .try_for_each(|bound| bound.below(&running, true))?;
        bound
            .iter()
            .filter(|bound| bound.class() == RouteClass::SharedLocal)
            .try_for_each(|bound| bound.below(&running, false))?;
        scratch(config, self.host, &launch.scratch())?;
        match (entry.boxed, config.boxed_builds.ceiling.get()) {
            (true, ceiling) if running.boxed >= ceiling as usize => Err(Capacity::BoxedFull {
                running: running.boxed,
                ceiling,
            }),
            _ => Ok(()),
        }
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

/// The free bytes on the scratch filesystem at or above its floor: the
/// filesystem of the declared path, else of the entry's own seat scratch,
/// `default`.
fn scratch(config: &HostConfig, host: &Host<'_>, default: &Path) -> Result<(), Capacity> {
    let path = (config.scratch.path.as_ref()).map_or(default, |path| path.0.as_path());
    let floor = config.scratch.floor_bytes.get();
    match (host.free)(path) {
        Err(error) => Err(Capacity::ScratchUnmeasured {
            path: path.to_path_buf(),
            kind: error.kind(),
        }),
        Ok(free) if free < floor => Err(Capacity::ScratchLow {
            path: path.to_path_buf(),
            free,
            floor,
        }),
        Ok(_) => Ok(()),
    }
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
        let pinned: Pinned = serde_json::from_value(manifest)
            .map_err(|error| Unmeasurable::Manifest(Shared(Arc::new(error))))?;
        let mut lanes = BTreeSet::new();
        for (site, Site { candidates }) in pinned.capabilities.sites {
            for Seated {
                provider,
                model_pins,
            } in candidates
            {
                let ids = match model_pins {
                    ModelPins::Read(ids) => ids,
                    ModelPins::Unreadable(flags) => return Err(Unmeasurable::Pin { site, flags }),
                };
                let routes: Vec<Option<String>> = match ids.is_empty() {
                    true => vec![None],
                    false => ids.iter().map(|id| route(id).map(str::to_string)).collect(),
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
struct Live {
    run: String,
    seats: Result<Seats, Unmeasurable>,
}

/// The runs the journal holds running now.
fn census(store: &Store) -> Result<Vec<Live>, AdmissionError> {
    let mut live = Vec::new();
    for (run, ..) in store.list_runs()? {
        let status = fold(&store.load(&run)?)
            .map_err(|source| AdmissionError::Fold {
                run: run.clone(),
                source,
            })?
            .status;
        if status == Status::Running {
            let seats = Seats::of(store.manifest(&run)?);
            live.push(Live { run, seats });
        }
    }
    Ok(live)
}

/// The entry as admission measures it: the lanes its compiled bundle
/// seats, and whether it builds in a box.
struct Entry {
    lanes: BTreeSet<Lane>,
    boxed: bool,
}

impl Entry {
    fn measure(launch: &QueuedLaunch) -> Result<Entry, Unmeasurable> {
        let bundle =
            (launch.compiled()).map_err(|error| Unmeasurable::Compile(Shared(Arc::new(error))))?;
        let Seats { lanes, boxed } = Seats::of(bundle.manifest)?;
        Ok(Entry { lanes, boxed })
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
    fn count(live: &[Live], config: &HostConfig) -> Result<Running, Capacity> {
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
