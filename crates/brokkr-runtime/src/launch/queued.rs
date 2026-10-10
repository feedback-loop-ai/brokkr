//! A launch as a queue entry holds it (decision 0068 ruling 1): the plain
//! data of a [`LaunchRequest`] and its [`NewRun`], versioned, so the
//! dispatcher can rebuild the request without the CLI that queued it.
//!
//! A relative path a launch names is relative to its `workspace`, the
//! directory the launch is made from, as it is for `brokkr run` in that
//! directory. An entry is admitted from wherever the dispatcher stands, so
//! it names its workspace absolutely and anchors every path to it: the
//! request's when the entry is made, and the request's again, the map
//! file a world was read from, which the world's pin holds as named, and
//! the repository, the workspace when none is named, when it is rebuilt.
//! So the realm and bundle an entry is admitted under are the ones it was
//! queued for, whatever directory admits it. Two fields of the request
//! are not stored, because neither is the entry's. The journal is the one the entry lives in. The search path a
//! boundary's tool is looked for on is the admitting host's, read when
//! the entry is admitted, as `now` would be.

use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;
use brokkr_core::realms::{Realm, DEFAULT_MAP_FILE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    refuse_realms_with_dispatch, BundleSource, LaunchError, LaunchRequest, NewRun, RunMap,
};
use crate::realms::{World, WorldError};
use crate::Bundle;

/// The encoding's version, the first field of every payload: a payload
/// in any other is refused, never read as this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encoding {
    #[serde(rename = "queued-launch/v1")]
    V1,
}

/// A queued run's world as the entry holds it: the pin a run manifest
/// carries ([`World::pin`]), taken when the entry is queued. That is the
/// file it was read from, its canonical content and the digest over it,
/// and the house and dialect of the realm the launch operates. It is read
/// back through [`World::from_manifest`] and never off the disk, so the
/// entry starts under the map it was queued with, whatever the file holds
/// by then; the digest is re-derived there, and a pin that does not
/// answer for itself is refused. The crossings the world draws are read
/// when the entry is rebuilt, fenced as the disk then stands as a resumed
/// run's are, and pinned into the run it starts as a direct start's are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HeldWorld(Value);

impl HeldWorld {
    /// The digest of the pin ([`MapSource::digest`]).
    fn digest(&self) -> String {
        canonical::sha256_hex(&self.0)
    }

    /// The world the entry was queued with, its map file anchored to the
    /// entry's `workspace`.
    fn world(&self, workspace: &Path) -> Result<World, WorldError> {
        // Always a world: the pin is read as a manifest's `realms`.
        let unpinned = Err(WorldError::Unpinned("it holds no map".into()));
        let mut world =
            World::from_manifest(&json!({ "realms": &self.0 }))?.map_or(unpinned, Ok)?;
        world.source = workspace.join(&world.source);
        Ok(world)
    }

    /// The world the entry was queued with and the realm its pin selected
    /// then, read off the pin and never resolved through the disk again:
    /// refused when the selection cannot be read, or names a realm its map
    /// does not hold.
    fn held(&self, workspace: &Path) -> Result<Held, WorldError> {
        let world = self.world(workspace)?;
        let unselected = |problem| WorldError::Unpinned(format!("its selected realm {problem}"));
        let Selection { realm } = Selection::deserialize(&self.0)
            .map_err(|error| unselected(format!("cannot be read: {error}")))?;
        let selected = realm
            .map(|name| {
                let at = world.map.realms.iter().position(|realm| realm.name == name);
                at.ok_or_else(|| unselected(format!("{name} is not a realm its map holds")))
            })
            .transpose()?;
        Ok(Held { world, selected })
    }

    /// The world the entry was queued with, stood on the disk as it is at
    /// admission: what the run it starts pins, crossings and all.
    fn stood(&self, workspace: &Path) -> Result<World, WorldError> {
        self.world(workspace)?.standing_on(workspace)
    }

    /// The map file the entry's world was read from, as it is on disk now.
    fn on_disk(&self, workspace: &Path) -> Result<World, WorldError> {
        World::load(&self.world(workspace)?.source)
    }
}

/// The selection a held world's pin records ([`World::pin`]): the realm
/// the operated repository resolved to when the entry was queued, none
/// where the map named it none.
#[derive(Deserialize)]
struct Selection {
    realm: Option<String>,
}

/// A held world and the realm its pin selected, by its place in the map:
/// the facts an entry was queued with, which admission compares as they
/// were recorded, never as the disk resolves the repository's path now
/// (#430's H4).
pub(crate) struct Held {
    pub(crate) world: World,
    selected: Option<usize>,
}

impl Held {
    /// `world` with the realm `select` resolves in it, resolved once and
    /// kept by its place in the map.
    fn selecting(
        world: World,
        select: impl for<'w> FnOnce(&'w World) -> Option<&'w Realm>,
    ) -> Held {
        let selected = select(&world).and_then(|chosen| {
            world
                .map
                .realms
                .iter()
                .position(|realm| std::ptr::eq(realm, chosen))
        });
        Held { world, selected }
    }

    /// The realm selected, `None` for none.
    fn realm(&self) -> Option<&Realm> {
        self.selected.map(|at| &self.world.map.realms[at])
    }

    /// The world and the realm selected in it, as admission compares them.
    pub(crate) fn selection(&self) -> (&World, Option<&Realm>) {
        (&self.world, self.realm())
    }
}

/// The map a queued run starts under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MapSource {
    Unmapped,
    Ambient(HeldWorld),
    Named(HeldWorld),
}

impl MapSource {
    /// `map`, held with the realm `repo` operates in.
    fn of(map: &RunMap, repo: &Path) -> Result<MapSource, WorldError> {
        let held = |world: &World| world.pin(Some(repo)).map(HeldWorld);
        match map {
            RunMap::Unmapped => Ok(MapSource::Unmapped),
            RunMap::Ambient(world) => held(world).map(MapSource::Ambient),
            RunMap::Named(world) => held(world).map(MapSource::Named),
        }
    }

    /// The digest of the pin it holds, `None` for no map: two equal
    /// digests are the same map, house and dialect, read from the same
    /// files. What a latch records of the map on disk (#430).
    pub(crate) fn digest(&self) -> Option<String> {
        match self {
            MapSource::Unmapped => None,
            MapSource::Ambient(held) | MapSource::Named(held) => Some(held.digest()),
        }
    }

    /// The world the entry was queued with and the realm it selected, or
    /// `None` under no map.
    fn held(&self, workspace: &Path) -> Result<Option<Held>, WorldError> {
        match self {
            MapSource::Unmapped => Ok(None),
            MapSource::Ambient(held) | MapSource::Named(held) => held.held(workspace).map(Some),
        }
    }

    /// The map that would govern the entry now: the file its world was
    /// read from, as it is on disk; and for an entry queued under no map,
    /// the one `brokkr run` would find in its workspace now, or none (the
    /// operator's ruling of 2026-10-04: no map to a map is a change).
    ///
    /// Only a map path with nothing at it is no map. A path that holds
    /// anything else, a link that loops or dangles or a directory, is a
    /// map that cannot be read, never one that is absent, so the entry is
    /// held rather than admitted (#430). [`World::discover`] reads those
    /// as no map, which is `brokkr run`'s and not this edge's.
    fn now(&self, workspace: &Path) -> Result<RunMap, WorldError> {
        Ok(match self {
            MapSource::Unmapped => {
                let default = workspace.join(DEFAULT_MAP_FILE);
                match std::fs::symlink_metadata(&default) {
                    Err(absent) if absent.kind() == ErrorKind::NotFound => RunMap::Unmapped,
                    _ => RunMap::Ambient(World::load(&default)?),
                }
            }
            MapSource::Ambient(held) => RunMap::Ambient(held.on_disk(workspace)?),
            MapSource::Named(held) => RunMap::Named(held.on_disk(workspace)?),
        })
    }
}

/// An entry's two worlds: the one it was queued with, if any, and the
/// map that would govern it now, or why that cannot be read.
pub(crate) struct HeldAndNow {
    pub(crate) held: Option<Held>,
    pub(crate) now: Result<Now, WorldError>,
}

/// The map that would govern an entry now, if any, with the realm it
/// selects for the entry's repository, and the digest of the pin a re-pin
/// to it would write ([`MapSource::digest`]). Both are read off one
/// selection, so a path retargeted between them cannot pair one realm's
/// digest with another realm's facts (#430's H6).
pub(crate) struct Now {
    pub(crate) world: Option<Held>,
    pub(crate) digest: Option<String>,
}

impl Now {
    /// The map `now`, its realm resolved once by `select`.
    fn of(
        now: RunMap,
        select: impl for<'w> FnOnce(&'w World) -> Option<&'w Realm>,
    ) -> Result<Now, WorldError> {
        let Some(world) = now.into_world() else {
            return Ok(Now {
                world: None,
                digest: None,
            });
        };
        let held = Held::selecting(world, select);
        let digest = HeldWorld(held.world.pin_of(held.realm())?).digest();
        Ok(Now {
            world: Some(held),
            digest: Some(digest),
        })
    }
}

/// Every fact a queued launch is rebuilt from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueuedLaunch {
    pub encoding: Encoding,
    pub workspace: PathBuf,
    pub bundle: BundleSource,
    pub repo: Option<PathBuf>,
    pub secrets: Option<PathBuf>,
    pub feature: String,
    pub map: MapSource,
    pub dispatch: Option<PathBuf>,
}

impl QueuedLaunch {
    /// The launch `request` and `run` would make, as an entry holds it.
    /// What [`super::start`] refuses before reading anything, a named map
    /// beside a dispatch, is refused here too, and so is a world whose
    /// operated realm's house or dialect cannot be read. So is a workspace
    /// that is not absolute, which nothing could be anchored to.
    pub fn of(request: &LaunchRequest, run: &NewRun) -> Result<QueuedLaunch, LaunchError> {
        refuse_realms_with_dispatch(run)?;
        let workspace = anchor(&request.workspace)?;
        let at = |path: &PathBuf| workspace.join(path);
        Ok(QueuedLaunch {
            encoding: Encoding::V1,
            workspace: workspace.to_path_buf(),
            bundle: anchored(workspace, &request.bundle),
            repo: request.repo.as_ref().map(at),
            secrets: request.secrets.as_ref().map(at),
            feature: run.feature.clone(),
            map: MapSource::of(&run.map, &workspace.join(request.operated()))?,
            dispatch: run.dispatch.as_ref().map(at),
        })
    }

    /// The request and new run this entry was queued as, written to
    /// `journal` and looking for a boundary's tool on `host_path`: the
    /// inverse of [`QueuedLaunch::of`]. The dispatcher's start of an
    /// admitted entry (#430's third slice) is its caller. Every path is
    /// anchored to the workspace again, so a
    /// payload that holds a relative one is read as `of` would have
    /// written it, never against the directory that rebuilds it; and the
    /// repository is always named, the workspace when the entry names
    /// none, because an engine handed none operates its process's
    /// directory.
    pub fn rebuild(
        self,
        journal: PathBuf,
        host_path: OsString,
    ) -> Result<(LaunchRequest, NewRun), LaunchError> {
        let workspace = anchor(&self.workspace)?.to_path_buf();
        let at = |path: PathBuf| workspace.join(path);
        let map = match &self.map {
            MapSource::Unmapped => Ok(RunMap::Unmapped),
            MapSource::Ambient(held) => held.stood(&workspace).map(RunMap::Ambient),
            MapSource::Named(held) => held.stood(&workspace).map(RunMap::Named),
        }?;
        let request = LaunchRequest {
            bundle: anchored(&workspace, &self.bundle),
            journal,
            repo: Some(self.operated(&workspace)),
            secrets: self.secrets.map(at),
            host_path,
            workspace: self.workspace,
        };
        let run = NewRun {
            feature: self.feature,
            map,
            dispatch: self.dispatch.map(at),
        };
        Ok((request, run))
    }

    /// The bundle this entry would start, compiled from the request and
    /// map [`QueuedLaunch::rebuild`] makes as a start compiles it: what
    /// admission measures the providers, routes and boxes it seats by
    /// (#430's third slice). The crossing fence and the boundary's tool
    /// are the start's to judge, not this measure's.
    pub(crate) fn compiled(&self) -> Result<Bundle, LaunchError> {
        let (request, run) = self.clone().rebuild(PathBuf::new(), OsString::new())?;
        let dir = request.bundle.resolve()?;
        super::compile_for(
            &request.workspace,
            &dir,
            run.map.world(),
            request.operated(),
        )
    }

    /// Where the seats of the run this entry starts make their scratch
    /// trees: under the repository it operates, as the engine places them.
    pub(crate) fn scratch(&self) -> PathBuf {
        self.operated(&self.workspace)
            .join(crate::engine::SEAT_SCRATCH)
    }

    /// The repository the entry operates, anchored to `workspace`: the
    /// one it names, else the workspace.
    fn operated(&self, workspace: &Path) -> PathBuf {
        workspace.join(self.repo.as_deref().unwrap_or(workspace))
    }

    /// The world this entry was queued with and the realm its pin
    /// selected, beside the map that would
    /// govern it now ([`MapSource::now`]) and the realm it names the
    /// repository the entry operates: what admission compares (#430's
    /// realm-drift ruling). The
    /// map now is the one fault an operator can mend, so it comes back as
    /// its own result, refusals and all.
    pub(crate) fn held_and_now(&self) -> Result<HeldAndNow, LaunchError> {
        self.held_and_now_with(World::realm_for)
    }

    /// [`QueuedLaunch::held_and_now`], the realm the map now names the
    /// repository resolved by `select`, called once (#430's H6).
    pub(crate) fn held_and_now_with(
        &self,
        select: impl for<'w> FnOnce(&'w World, &Path) -> Option<&'w Realm>,
    ) -> Result<HeldAndNow, LaunchError> {
        let workspace = anchor(&self.workspace)?;
        let repo = self.operated(workspace);
        let now = self
            .map
            .now(workspace)
            .and_then(|map| Now::of(map, |world| select(world, &repo)));
        Ok(HeldAndNow {
            held: self.map.held(workspace)?,
            now,
        })
    }

    /// This entry under the map that would govern it now
    /// ([`MapSource::now`]), pinned afresh as [`QueuedLaunch::of`] pins
    /// one: what the operator's `brokkr queue repin` writes. Everything
    /// else the entry holds is kept. A map that cannot be read now is
    /// refused as `brokkr run` would refuse it. The release is
    /// [`crate::admission`]'s to write, so this stays inside the crate.
    pub(crate) fn repinned(mut self) -> Result<QueuedLaunch, LaunchError> {
        let workspace = anchor(&self.workspace)?.to_path_buf();
        let map = self.map.now(&workspace)?;
        self.map = MapSource::of(&map, &self.operated(&workspace))?;
        Ok(self)
    }

    /// The payload text. A path that is not UTF-8 cannot be written and
    /// is refused.
    pub fn encode(&self) -> Result<String, LaunchError> {
        serde_json::to_string(self).map_err(LaunchError::EncodeQueued)
    }

    /// Read a payload back; one in another encoding, or with a field this
    /// encoding does not know, is refused.
    pub fn decode(payload: &str) -> Result<QueuedLaunch, LaunchError> {
        serde_json::from_str(payload).map_err(LaunchError::DecodeQueued)
    }
}

/// `bundle` with its directory anchored to `workspace`.
fn anchored(workspace: &Path, bundle: &BundleSource) -> BundleSource {
    match bundle {
        BundleSource::Dir(dir) => BundleSource::Dir(workspace.join(dir)),
        BundleSource::Recipe { name, recipes_dir } => BundleSource::Recipe {
            name: name.clone(),
            recipes_dir: workspace.join(recipes_dir),
        },
    }
}

/// The workspace an entry's paths are anchored to: refused when it is not
/// absolute, because a relative one would resolve against whatever
/// directory reads the entry.
fn anchor(workspace: &Path) -> Result<&Path, LaunchError> {
    match workspace.is_absolute() {
        true => Ok(workspace),
        false => Err(LaunchError::QueuedWorkspaceRelative(
            workspace.to_path_buf(),
        )),
    }
}

#[cfg(test)]
mod tests;
