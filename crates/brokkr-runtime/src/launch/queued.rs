//! A launch as a queue entry holds it (decision 0068 ruling 1): the plain
//! data of a [`LaunchRequest`] and its [`NewRun`], versioned, so the
//! dispatcher can rebuild the request without the CLI that queued it.
//!
//! A relative path it holds is relative to its `workspace`, the directory
//! the launch is made from, as it is for `brokkr run` in that directory.
//! Two fields of the request are not stored, because neither is the
//! entry's. The journal is the one the entry lives in. The search path a
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
    /// The world the entry was queued with.
    fn world(&self) -> Result<World, WorldError> {
        // Always a world: the pin is read as a manifest's `realms`.
        let unpinned = Err(WorldError::Unpinned("it holds no map".into()));
        World::from_manifest(&json!({ "realms": &self.0 }))?.map_or(unpinned, Ok)
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
    /// operated realm's house or dialect cannot be read.
    pub fn of(request: &LaunchRequest, run: &NewRun) -> Result<QueuedLaunch, LaunchError> {
        refuse_realms_with_dispatch(run)?;
        Ok(QueuedLaunch {
            encoding: Encoding::V1,
            workspace: request.workspace.clone(),
            bundle: request.bundle.clone(),
            repo: request.repo.clone(),
            secrets: request.secrets.clone(),
            feature: run.feature.clone(),
            map: MapSource::of(&run.map, request.operated())?,
            dispatch: run.dispatch.clone(),
        })
    }

    /// The request and new run this entry was queued as, written to
    /// `journal` and looking for a boundary's tool on `host_path`: the
    /// inverse of [`QueuedLaunch::of`]. Admission (#430's second slice)
    /// is its caller.
    pub fn rebuild(
        self,
        journal: PathBuf,
        host_path: OsString,
    ) -> Result<(LaunchRequest, NewRun), LaunchError> {
        let map = match &self.map {
            MapSource::Unmapped => Ok(RunMap::Unmapped),
            MapSource::Ambient(held) => held.world().map(RunMap::Ambient),
            MapSource::Named(held) => held.world().map(RunMap::Named),
        }?;
        let request = LaunchRequest {
            workspace: self.workspace,
            bundle: self.bundle,
            journal,
            repo: self.repo,
            secrets: self.secrets,
            host_path,
        };
        let run = NewRun {
            feature: self.feature,
            map,
            dispatch: self.dispatch,
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

#[cfg(test)]
mod tests;
