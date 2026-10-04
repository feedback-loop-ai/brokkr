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
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    refuse_realms_with_dispatch, BundleSource, LaunchError, LaunchRequest, NewRun, RunMap,
};
use crate::realms::{World, WorldError};

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
/// answer for itself is refused. The crossings the world draws are fenced
/// at admission as the disk then stands, as a resumed run's are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HeldWorld(Value);

impl HeldWorld {
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
    /// inverse of [`QueuedLaunch::of`]. Admission (#430's second slice)
    /// is its caller. Every path is anchored to the workspace again, so a
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
            MapSource::Ambient(held) => held.world(&workspace).map(RunMap::Ambient),
            MapSource::Named(held) => held.world(&workspace).map(RunMap::Named),
        }?;
        let request = LaunchRequest {
            bundle: anchored(&workspace, &self.bundle),
            journal,
            repo: Some(self.repo.map_or_else(|| workspace.clone(), at)),
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
