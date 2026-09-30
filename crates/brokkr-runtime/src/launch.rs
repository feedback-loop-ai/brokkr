//! Starting, resuming and rerunning a run: the one admission every entry
//! point goes through (#350). `brokkr run`, `resume` and `rerun` build a
//! [`LaunchRequest`] and call [`start`], [`resume`] or [`rerun`]; the
//! dispatcher decision 0068 rules builds the same request without the
//! CLI. What comes back is an admitted [`Engine`], not yet driven.
//!
//! Every refusal has one site here, and each fires before the journal row
//! or the seat it guards exists: a named map beside a Looper dispatch,
//! a recipe that is not installed ([`BundleSource::resolve`]), a crossing
//! that moved under the world (`fence_crossings`), a bundle this host
//! cannot box ([`refuse_unboxable`]) and a dispatch envelope that cannot
//! be read or verified, or does not bound the bundle. Each entry point
//! meets them in the order the CLI always met them, so a request that
//! trips two is refused by the same one it always was — save one: the
//! envelope is now refused before the journal is opened, so beside a
//! peer's lock on that journal it is the envelope, not the contention,
//! that is reported.
//!
//! The crossing fence is the one refusal a caller may meet first on its
//! own: [`World::load`] refuses a moved crossing as the world is read,
//! which is where `brokkr run` and `rerun` meet it, before they say
//! anything about the map. `start` and `rerun` fence again here,
//! deliberately: a world held since it was read (a queued dispatch,
//! decision 0068) is fenced as the disk stands at admission.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use brokkr_core::dispatch::{DispatchEnvelopeV2, DispatchError};
use brokkr_core::realms::{Boundary, Realm};
use brokkr_store::{Store, StoreError};
use serde_json::Value;
use thiserror::Error;

use crate::boundary::{refuse_unboxable, Unboxable};
use crate::bundle::{CompileError, DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use crate::capabilities::{CapabilityContext, UNMAPPED};
use crate::engine::verify_dispatch_bundle_bounds;
use crate::realms::{World, WorldError};
use crate::{Bundle, Engine, EngineError};

mod queued;

pub use queued::{Encoding, HeldWorld, MapSource, QueuedLaunch};

/// What every launch is asked with.
#[derive(Debug, Clone)]
pub struct LaunchRequest {
    /// The tree `agents/` and `adapters/` are read from and a map's
    /// crossings are resolved against (decision 0023).
    pub workspace: PathBuf,
    /// The bundle to compile.
    pub bundle: BundleSource,
    /// The journal the run is written to; created when absent.
    pub journal: PathBuf,
    /// The operated repository. When `None`, a new run's bundle compiles
    /// in the realm the map names for the workspace, while the engine
    /// operates the directory it is constructed in (#368): one tree for
    /// the CLI, whose workspace is that directory. A caller whose
    /// workspace is elsewhere names the repository.
    pub repo: Option<PathBuf>,
    /// The operator's secrets store override (decision 0012).
    pub secrets: Option<PathBuf>,
    /// The search path a boundary's tool is looked for on.
    pub host_path: OsString,
}

impl LaunchRequest {
    /// The repository a new run's bundle compiles for and its world is
    /// pinned for: the one named, else the workspace.
    fn operated(&self) -> &Path {
        self.repo.as_deref().unwrap_or(&self.workspace)
    }
}

/// Where the bundle a launch compiles comes from. It is resolved inside
/// the launch, at the point each entry point always resolved it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BundleSource {
    /// A bundle directory, as named.
    Dir(PathBuf),
    /// A recipe installed under `recipes_dir`.
    Recipe { name: String, recipes_dir: PathBuf },
}

impl BundleSource {
    /// The directory to compile: a recipe that is not installed is
    /// refused.
    pub fn resolve(&self) -> Result<PathBuf, LaunchError> {
        match self {
            BundleSource::Dir(dir) => Ok(dir.clone()),
            BundleSource::Recipe { name, recipes_dir } => {
                let dir = recipes_dir.join(name);
                match dir.is_dir() {
                    true => Ok(dir),
                    false => Err(LaunchError::RecipeNotFound {
                        name: name.clone(),
                        recipes_dir: recipes_dir.clone(),
                    }),
                }
            }
        }
    }
}

/// The map a new run starts under (decision 0023 ruling 3).
#[derive(Debug, Default)]
pub enum RunMap {
    /// No map is in effect: the run has no world.
    #[default]
    Unmapped,
    /// A map lying in the workspace: adopted, but not an instruction.
    Ambient(World),
    /// The map the operator named with `--realms`.
    Named(World),
}

impl RunMap {
    fn world(&self) -> Option<&World> {
        match self {
            RunMap::Unmapped => None,
            RunMap::Ambient(world) | RunMap::Named(world) => Some(world),
        }
    }

    fn into_world(self) -> Option<World> {
        match self {
            RunMap::Unmapped => None,
            RunMap::Ambient(world) | RunMap::Named(world) => Some(world),
        }
    }
}

/// What a NEW run adds to its request.
#[derive(Debug, Default)]
pub struct NewRun {
    pub feature: String,
    /// The map the run is started under.
    pub map: RunMap,
    /// A Looper dispatch envelope to start the run under.
    pub dispatch: Option<PathBuf>,
}

/// Why a launch refused, with the operator's text.
#[derive(Debug, Error)]
pub enum LaunchError {
    /// A Looper-bound run pins a run-manifest/v2, which carries no world,
    /// so a map the operator NAMED is refused rather than half-honoured.
    #[error(
        "a run with --dispatch cannot pin the map named by --realms: the \
         Looper-bound run-manifest/v2 lineage carries no world, and dropping \
         the map silently would leave the run unable to say which one it \
         believed in. Run without --dispatch, or without --realms, until a \
         jointly agreed v2-lineage manifest version exists"
    )]
    RealmsWithDispatch,
    #[error(
        "recipe '{name}' not found under {}; install it with \
         `brokkr recipes add <source> --name {name}`",
        recipes_dir.display()
    )]
    RecipeNotFound { name: String, recipes_dir: PathBuf },
    #[error(transparent)]
    Unboxable(#[from] Unboxable),
    #[error("reading dispatch {}", path.display())]
    ReadDispatch {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing forge-dispatch/v2")]
    ParseDispatch(#[source] serde_json::Error),
    /// A launch that cannot be written as a queue entry (decision 0068).
    #[error("writing a queued launch")]
    EncodeQueued(#[source] serde_json::Error),
    /// A queue entry's payload this brokkr cannot read as a launch.
    #[error("reading a queued launch")]
    DecodeQueued(#[source] serde_json::Error),
    #[error(transparent)]
    Dispatch(#[from] DispatchError),
    #[error("loading source run '{run}'")]
    SourceRun {
        run: String,
        #[source]
        source: StoreError,
    },
    #[error("source run '{run}' has no run/started feature to re-run")]
    NoFeature { run: String },
    #[error(transparent)]
    World(#[from] WorldError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Engine(#[from] EngineError),
}

impl LaunchError {
    /// The contention this error carries, if that is what it is. The
    /// store and engine variants are `transparent`, so the error they
    /// hold is not a link of its own in a `source()` chain; this is the
    /// door through, asking the store's own predicate.
    pub fn contention(&self) -> Option<&StoreError> {
        match self {
            LaunchError::Store(error) => Some(error).filter(|error| error.is_contention()),
            LaunchError::Engine(error) => error.contention(),
            _ => None,
        }
    }
}

/// Start a new run. A dispatch's `note` about an ambient map it does not
/// pin is said through `notice`, because the runtime writes to no stream
/// of its own.
pub fn start(
    request: LaunchRequest,
    run: NewRun,
    notice: &mut dyn FnMut(&str),
) -> Result<Engine, LaunchError> {
    // Refused in the same breath as a missing or malformed map, before a
    // recipe is resolved, a bundle compiled, an envelope read or a
    // journal created.
    refuse_realms_with_dispatch(&run)?;
    let bundle = admit_new(&request, run.map.world())?;
    let envelope = match &run.dispatch {
        Some(path) => Some(read_dispatch(path, run.map.world(), &bundle, notice)?),
        None => None,
    };
    let store = Store::open(&request.journal)?;
    let (feature, repo) = (&run.feature, request.repo);
    let mut engine = match envelope {
        Some(envelope) => Engine::start_with_dispatch(store, bundle, feature, repo, envelope)?,
        None => Engine::start_in_world(store, bundle, feature, repo, run.map.into_world())?,
    };
    engine.secrets_file = request.secrets;
    Ok(engine)
}

/// Continue `run` under its exact pinned bundle, compiled in the world
/// and realm its own manifest pinned.
pub fn resume(request: LaunchRequest, run: &str) -> Result<Engine, LaunchError> {
    let store = Store::open(&request.journal)?;
    let manifest = store.manifest(run)?;
    let dir = request.bundle.resolve()?;
    let repo = request.repo.as_deref().unwrap_or(&request.workspace);
    let bundle = compile_from_manifest(&request.workspace, &dir, &manifest, repo)
        .map_err(|error| unreproducible(run, error))?;
    refuse_unboxable(&bundle, &request.host_path)?;
    let mut engine = Engine::resume(store, bundle, run, request.repo)?;
    // Decision 0057, on decision 0046's Addendum's terms: a resumed run
    // is fenced where a new one is, before `drive()` and so before any
    // seat spawns. The world it holds comes from its own manifest and has
    // met no disk (`World::from_manifest` resolves no crossing,
    // deliberately), so without this a run would carry on over bytes its
    // journal never saw. A Looper-bound run carries no world at all and
    // has nothing to fence.
    fence_crossings(engine.world.as_ref(), &request.workspace)?;
    engine.secrets_file = request.secrets;
    Ok(engine)
}

/// A past run's feature as a NEW run, in `world` (decision 0046 ruling
/// 1; design DD6): the journal named is the one the rerun is written to
/// (#374), and the bundle compiles against the operated repository's
/// realm.
pub fn rerun(
    request: LaunchRequest,
    run: &str,
    world: Option<World>,
) -> Result<Engine, LaunchError> {
    let store = Store::open(&request.journal)?;
    let feature = source_feature(&store, run)?;
    let bundle = admit_new(&request, world.as_ref())?;
    let mut engine = Engine::start_in_world(store, bundle, &feature, request.repo, world)?;
    engine.secrets_file = request.secrets;
    Ok(engine)
}

/// A named map beside a Looper dispatch: refused by `start`, and by a
/// queued launch before it is written, so no entry holds one.
fn refuse_realms_with_dispatch(run: &NewRun) -> Result<(), LaunchError> {
    match matches!(run.map, RunMap::Named(_)) && run.dispatch.is_some() {
        true => Err(LaunchError::RealmsWithDispatch),
        false => Ok(()),
    }
}

/// What a new run is admitted on: the crossings its world stands on, its
/// bundle compiled in the operated repository's realm, and a host that
/// can build the boundary that realm declares.
fn admit_new(request: &LaunchRequest, world: Option<&World>) -> Result<Bundle, LaunchError> {
    fence_crossings(world, &request.workspace)?;
    let dir = request.bundle.resolve()?;
    let bundle = compile_for(&request.workspace, &dir, world, request.operated())?;
    refuse_unboxable(&bundle, &request.host_path)?;
    Ok(bundle)
}

/// The crossing fence (decision 0057): every crossing the world draws is
/// re-read off the disk as it stands now, against `workspace`.
fn fence_crossings(world: Option<&World>, workspace: &Path) -> Result<(), WorldError> {
    match world {
        Some(world) => world.verify_crossings(workspace),
        None => Ok(()),
    }
}

/// The feature a rerun re-runs: the source run's `run/started` word.
fn source_feature(store: &Store, run: &str) -> Result<String, LaunchError> {
    let events = store.load(run).map_err(|source| LaunchError::SourceRun {
        run: run.to_string(),
        source,
    })?;
    events
        .first()
        .filter(|event| event.event_type == brokkr_core::EventType::RunStarted)
        .and_then(|event| event.payload.get("feature"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| LaunchError::NoFeature {
            run: run.to_string(),
        })
}

/// A Looper-bound run's envelope, read, parsed, verified against the
/// compiled bundle and held to bound every seat it can run, all before a
/// journal is opened. So an envelope that cannot stand is refused ahead
/// of a peer's lock on the journal, which is never waited on for it.
/// The bounds refusal is the engine's error, as it always read; the
/// engine keeps both checks as its own constructor's guard.
fn read_dispatch(
    path: &Path,
    world: Option<&World>,
    bundle: &Bundle,
    notice: &mut dyn FnMut(&str),
) -> Result<DispatchEnvelopeV2, LaunchError> {
    // A map merely lying in the workspace still names the journal this
    // world's fleet writes, so the run goes there — but it is not
    // pinned, and a dropped pin is said out loud rather than left to be
    // discovered in the manifest.
    if let Some(world) = world {
        notice(&format!(
            "note: {} is not pinned into this run: --dispatch writes a \
             run-manifest/v2, which carries no world",
            world.source.display()
        ));
    }
    let raw = std::fs::read_to_string(path).map_err(|source| LaunchError::ReadDispatch {
        path: path.to_path_buf(),
        source,
    })?;
    let envelope: DispatchEnvelopeV2 =
        serde_json::from_str(&raw).map_err(LaunchError::ParseDispatch)?;
    envelope.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
    verify_dispatch_bundle_bounds(&envelope, bundle).map_err(EngineError::from)?;
    Ok(envelope)
}

/// The one compile a run, `compile` and `recipes show` stand on: the
/// bundle at `dir`, in the realm `world` names for `repo` — its dialect
/// and its boundary, or `namespace` for a repository no map names
/// (decision 0046 ruling 1).
pub fn compile_for(
    workspace: &Path,
    dir: &Path,
    world: Option<&World>,
    repo: &Path,
) -> Result<Bundle, LaunchError> {
    let realm = world.and_then(|world| world.realm_for(repo));
    let name = realm.map_or(UNMAPPED, |realm| realm.name.as_str());
    compile_in_realm(workspace, dir, world, realm, name, repo)
}

/// The capability context one compile authorises against (decision 0065;
/// design D2): the OPERATED realm's grants — never a neighbouring realm's,
/// never the recipe's home — and the directory the operator's abstract
/// definitions and tool dialects live in. With a map that is the map
/// file's own directory, by the rule every other map-relative name
/// follows; without one it is the operated repository — what `--repo`
/// names, else the workspace — and the context grants nothing. A
/// repository the map does not name grants nothing either.
pub fn capability_context(
    workspace: &Path,
    world: Option<&World>,
    realm: Option<&Realm>,
    repo: &Path,
) -> CapabilityContext {
    let root = world
        .and_then(|world| workspace.join(&world.source).parent().map(PathBuf::from))
        .unwrap_or_else(|| repo.to_path_buf());
    CapabilityContext {
        realm: realm.map_or(UNMAPPED.to_string(), |realm| realm.name.clone()),
        grants: realm.map(|realm| realm.grants.clone()).unwrap_or_default(),
        root,
    }
}

/// Resume's compile: against the world and realm embedded in the run,
/// never against whatever the workspace's map happens to hold today.
fn compile_from_manifest(
    workspace: &Path,
    dir: &Path,
    manifest: &Value,
    repo: &Path,
) -> Result<Bundle, LaunchError> {
    let Some(world) = World::from_manifest(manifest)? else {
        // A run that pinned no world stood in no realm and held no grant.
        // Its definitions are re-read where they were read when it
        // started: under the operated repository, never the recipe's home
        // and never a map that has appeared in the workspace since.
        return Ok(Bundle::compile_unmapped(
            dir,
            &workspace.join(DEFAULT_AGENTS_DIR),
            &workspace.join(DEFAULT_ADAPTERS_DIR),
            Boundary::Namespace,
            repo,
        )?);
    };
    let name = manifest
        .pointer("/realms/realm")
        .and_then(Value::as_str)
        .unwrap_or(UNMAPPED);
    let realm = world.map.realms.iter().find(|realm| realm.name == name);
    // And the grants the run was started under, from the same pinned map
    // (decision 0065 ruling 8): a grant added to the workspace's map since
    // is not borrowed. The definitions and dialects are re-read from the
    // pinned source's directory and must reproduce the pinned digests, or
    // the manifest comparison refuses the resume with capabilities named.
    compile_in_realm(workspace, dir, Some(&world), realm, name, repo)
}

/// A resume whose pinned capability authority cannot be REPRODUCED here —
/// a definition or a tool dialect that is gone, or no longer what the
/// grant needs — is the run pinning a different bundle, and is refused
/// through that door with capabilities named (decision 0065 ruling 8;
/// design D7), not as a compile failure that reads like a broken recipe.
/// Every other failure passes through untouched. The reason is the
/// compiler's raw words, so the whole line the engine renders — its `run
/// '{run}' pins a different bundle: ` and this detail — is made through the
/// protocol's one refusal sink: one line, at most 512 scalar values
/// (rebuild unit 12-fix-f; design D6). The run id is the engine's own.
fn unreproducible(run: &str, error: LaunchError) -> LaunchError {
    match error {
        LaunchError::Compile(CompileError::Capability(reason)) => {
            let head = format!("run '{run}' pins a different bundle: ");
            let line = brokkr_protocol::native_controls::bounded_line(&format!(
                "{head}capabilities differ: the capability authority the run was started under \
                 cannot be reproduced here — {reason}; a grant, an abstract definition or a \
                 tool dialect was removed or edited since the run started"
            ));
            LaunchError::Engine(EngineError::ManifestMismatch {
                run_id: run.to_string(),
                detail: line.chars().skip(head.chars().count()).collect(),
            })
        }
        other => other,
    }
}

/// The bundle compiled in the realm `name`: its dialect, its boundary,
/// or none and `namespace` when the world names no such realm, and the
/// capability grants of that realm for the operated `repo`.
fn compile_in_realm(
    workspace: &Path,
    dir: &Path,
    world: Option<&World>,
    realm: Option<&Realm>,
    name: &str,
    repo: &Path,
) -> Result<Bundle, LaunchError> {
    let dialect = match world.zip(realm) {
        Some((world, realm)) => world.dialect_for_realm(realm)?,
        None => None,
    };
    let boundary = realm.map_or(Boundary::Namespace, Realm::boundary);
    Ok(Bundle::compile_with_capabilities(
        dir,
        &workspace.join(DEFAULT_AGENTS_DIR),
        &workspace.join(DEFAULT_ADAPTERS_DIR),
        Some(name),
        dialect,
        boundary,
        &capability_context(workspace, world, realm, repo),
    )?)
}

#[cfg(test)]
mod tests;
