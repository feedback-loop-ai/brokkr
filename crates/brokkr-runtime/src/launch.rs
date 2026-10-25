//! One admission for start, resume and rerun (#350), shared by the CLI and
//! decision 0068's dispatcher. It returns an admitted, undriven engine.
//!
//! Before any new journal row or guarded seat exists, refuse dispatch/map
//! conflicts, uninstalled recipes, moved world crossings, unboxable bundles
//! and invalid or mismatched dispatch envelopes. Envelopes are checked before
//! opening the journal, so their refusal precedes store contention. Continued
//! runs use the engine's typed manifest reader to refuse Looper lineage,
//! including malformed claims; entry/dispatch conflicts precede journal reads.
//!
//! World::load fences crossings when reading a named map; start and rerun
//! fence again at admission because a queued world's files may have moved.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use brokkr_core::dispatch::{DispatchEnvelopeV2, DispatchError};
use brokkr_core::realms::{Boundary, Realm};
use brokkr_store::{Claim, Generation, Store, StoreError};
use serde_json::Value;
use thiserror::Error;

use crate::boundary::{refuse_unboxable, Unboxable};
use crate::bundle::{CompileError, RealmLaw};
use crate::capabilities::{CapabilityContext, UNMAPPED};
use crate::engine::verify_dispatch_bundle_bounds;
use crate::places::{state_of, Layout, Places, PlacesError, Shelves};
use crate::realms::{World, WorldError};
use crate::{Bundle, Engine, EngineError};

mod queued;

pub(crate) use queued::HeldAndNow;
pub use queued::{Encoding, HeldWorld, MapSource, QueuedLaunch};

/// What every launch is asked with.
#[derive(Debug, Clone)]
pub struct LaunchRequest {
    /// The tree a map's crossings are resolved against (decision 0023),
    /// whose [`Places`] name the agents, the adapters and the run-local
    /// state: a folder's root, or a library's working directory.
    pub workspace: PathBuf,
    /// The bundle to compile.
    pub bundle: BundleSource,
    /// The project shelf the operator selected, `--recipes-dir`, which the
    /// bundle composes across in place of the workspace's own (#697 ruling
    /// 3); `None` composes across the workspace's.
    pub shelf: Option<PathBuf>,
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
    pub entry: Option<EntryRequest>,
    pub feature: String,
    /// The map the run is started under.
    pub map: RunMap,
    /// A Looper dispatch envelope to start the run under.
    pub dispatch: Option<PathBuf>,
    /// The queue claim a dispatcher starts this run under, with the
    /// generation that took it (decision 0095 rulings 4, 5 and 8): the
    /// start is fenced by it and the run's manifest pins it. A direct
    /// `brokkr run` and [`QueuedLaunch::rebuild`] give none.
    pub claim: Option<(Claim, Generation)>,
}

/// A requested run entry, with selectors already resolved to canonical ids.
#[derive(Debug, Clone)]
pub struct EntryRequest {
    pub phase: String,
    pub continues: Vec<String>,
}

/// Why a launch refused, with the operator's text.
#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("--dispatch and --enter cannot be combined: a Looper-bound run takes no entry")]
    EntryWithDispatch,
    /// A queue claim beside a Looper dispatch, whose run-manifest/v2
    /// carries no `queue_entry` and whose start runs no claim's fence.
    #[error(
        "a queue claim cannot start a run with --dispatch: its run-manifest/v2 carries no \
         queue_entry, so the queue holds such an entry (decision 0095 ruling 4); nothing was \
         started"
    )]
    ClaimWithDispatch,
    #[error(transparent)]
    Entry(#[from] crate::bundle::entry::EntryError),
    #[error(transparent)]
    Record(#[from] brokkr_core::entry::EntryRead),
    #[error("continued run '{run_id}' is missing")]
    ContinuedMissing { run_id: String },
    #[error("continued run '{run_id}' is Looper-bound: its start pinned the run-manifest/v2 lineage, which a chain may not name")]
    ContinuedLooperBound { run_id: String },
    #[error("continued run '{run_id}' is unreadable: its pinned manifest claims the run-manifest/v2 lineage and does not read")]
    ContinuedUnreadable {
        run_id: String,
        #[source]
        source: DispatchError,
    },
    #[error("continued run '{run_id}' is not concluded: {}", .status.as_str())]
    ContinuedNotConcluded {
        run_id: String,
        status: brokkr_core::Status,
    },
    #[error("entry at '{phase}' cannot be queued: queued-launch/v1 carries no entry")]
    QueuedEntry { phase: String },

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
    /// A queued launch's workspace that its paths cannot be anchored to.
    #[error(
        "a queued launch names its workspace absolutely, and {} is relative",
        .0.display()
    )]
    QueuedWorkspaceRelative(PathBuf),
    /// A launch selecting a project shelf other than its workspace's own,
    /// which no queued launch holds (#697 ruling 3).
    #[error(
        "--recipes-dir selects the project shelf {}, and a queued launch composes only across \
         its workspace's own, {} (#697): queued-launch/v1 holds no selected shelf, so the entry \
         would be admitted and started across the workspace's shelf instead; queue it without \
         --recipes-dir, or start it with `brokkr run`; nothing was queued",
        .shelf.display(),
        .own.display()
    )]
    QueuedShelf { shelf: PathBuf, own: PathBuf },
    /// A queued recipe on a shelf the admitting process stands as neither
    /// its workspace's project shelf nor its personal shelf (#697 rulings 2
    /// and 5), whose layers' origins no compile here could judge.
    #[error(
        "queued recipe '{name}' stands on the shelf {}, which is neither its workspace's project \
         shelf, {}, nor the personal shelf this process places ({}) (#697): a queued launch \
         composes only across shelves its admitting process places, so the entry would compose \
         with no origin for that shelf's layers and past the refusal of a name on both shelves; \
         admit it under the XDG_CONFIG_HOME and HOME that queued it, or start it with `brokkr \
         run`",
        .shelf.display(),
        .own.display(),
        placed(.personal)
    )]
    QueuedShelfUnplaced {
        name: String,
        shelf: PathBuf,
        own: PathBuf,
        personal: Option<PathBuf>,
    },
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
    Places(#[from] crate::places::PlacesError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Engine(#[from] EngineError),
}

/// The personal shelf this process places, as a refusal names it.
fn placed(personal: &Option<PathBuf>) -> String {
    personal.as_ref().map_or_else(
        || "it places none".to_string(),
        |shelf| shelf.display().to_string(),
    )
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

impl EntryRequest {
    // Decision 0094: both entry checks precede opening the journal.
    fn admit(&self, bundle: &Bundle) -> Result<(), LaunchError> {
        crate::bundle::entry::admit(bundle, &self.phase)?;
        brokkr_core::entry::check_record(&self.phase, self.continues.iter().map(String::as_str))?;
        Ok(())
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
    let state = state_of(&request.workspace)?;
    let bundle = admit_new(&request, run.map.world())?;
    if let Some(entry) = &run.entry {
        entry.admit(&bundle)?;
    }
    let envelope = match &run.dispatch {
        Some(path) => Some(read_dispatch(path, run.map.world(), &bundle, notice)?),
        None => None,
    };
    let store = Store::open(&request.journal)?;
    let entry = run
        .entry
        .as_ref()
        .map(|request| continued_entry(&store, request))
        .transpose()?;
    let (feature, repo) = (&run.feature, request.repo);
    let begun = (run.map.into_world(), entry);
    let mut engine = match envelope {
        Some(envelope) => Engine::start_with_dispatch(store, bundle, feature, repo, envelope)?,
        None => Engine::start_claimed(store, bundle, feature, repo, begun, run.claim)?,
    };
    engine.secrets_file = request.secrets;
    engine.state = state;
    Ok(engine)
}

/// Continue `run` under its exact pinned bundle, compiled in the world
/// and realm its own manifest pinned.
pub fn resume(request: LaunchRequest, run: &str) -> Result<Engine, LaunchError> {
    let state = state_of(&request.workspace)?;
    let store = Store::open(&request.journal)?;
    resume_with(store, state, request, run)
}

fn resume_with(
    store: Store,
    state: Option<PathBuf>,
    request: LaunchRequest,
    run: &str,
) -> Result<Engine, LaunchError> {
    let manifest = store.manifest(run)?;
    let dir = request.bundle.resolve()?;
    let repo = request.repo.as_deref().unwrap_or(&request.workspace);
    let leaf = (dir.as_path(), request.shelf.as_deref());
    let bundle = compile_from_manifest(&request.workspace, leaf, &manifest, repo)
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
    engine.state = state;
    Ok(engine)
}

/// Retry only the named panel members, admitting under the same pinned bundle as `resume`.
pub fn operator_retry_members(
    request: LaunchRequest,
    run: &str,
    names: &[String],
    operator: &str,
    reason: &str,
) -> Result<crate::MemberRetryOutcome, LaunchError> {
    let state = state_of(&request.workspace)?;
    let mut store = Store::open(&request.journal)?;
    let read = match Engine::read_member_retry(&mut store, run)? {
        Ok(read) => read,
        Err(outcome) => return Ok(outcome),
    };
    let mut engine = resume_with(store, state, request, run)?;
    Ok(engine.retry_members(read, names, operator, reason)?)
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
    let state = state_of(&request.workspace)?;
    let store = Store::open(&request.journal)?;
    let feature = source_feature(&store, run)?;
    let bundle = admit_new(&request, world.as_ref())?;
    let mut engine = Engine::start_in_world(store, bundle, &feature, request.repo, world, None)?;
    engine.secrets_file = request.secrets;
    engine.state = state;
    Ok(engine)
}

/// A named map, an entry or a queue claim beside a Looper dispatch:
/// refused by `start`, and by a queued launch before it is written, so no
/// entry holds one.
fn refuse_realms_with_dispatch(run: &NewRun) -> Result<(), LaunchError> {
    if run.entry.is_some() && run.dispatch.is_some() {
        return Err(LaunchError::EntryWithDispatch);
    }
    if run.claim.is_some() && run.dispatch.is_some() {
        return Err(LaunchError::ClaimWithDispatch);
    }
    match matches!(run.map, RunMap::Named(_)) && run.dispatch.is_some() {
        true => Err(LaunchError::RealmsWithDispatch),
        false => Ok(()),
    }
}

/// Attribution only: no predecessor verdict is imported (decision 0094).
fn continued_entry(
    store: &Store,
    request: &EntryRequest,
) -> Result<brokkr_core::entry::RunEntry, LaunchError> {
    use brokkr_core::entry::{ContinuedRun, RunEntry};
    let mut continues = Vec::new();
    for id in &request.continues {
        let manifest = store.manifest(id).map_err(|error| match error {
            StoreError::RunNotFound(_) => LaunchError::ContinuedMissing { run_id: id.clone() },
            other => LaunchError::Store(other),
        })?;
        let dispatch = brokkr_core::dispatch::dispatch_from_run(&manifest).map_err(|source| {
            LaunchError::ContinuedUnreadable {
                run_id: id.clone(),
                source,
            }
        })?;
        if dispatch.is_some() {
            return Err(LaunchError::ContinuedLooperBound { run_id: id.clone() });
        }
        let state = brokkr_core::fold(&store.load(id)?).map_err(EngineError::from)?;
        if brokkr_core::fold::concluded(&state).is_none() {
            return Err(LaunchError::ContinuedNotConcluded {
                run_id: id.clone(),
                status: state.run.status(),
            });
        }
        continues.push(ContinuedRun {
            run_id: id.clone(),
            feature: source_feature(store, id)?,
        });
    }
    Ok(RunEntry {
        phase: request.phase.clone(),
        continues,
    })
}

/// What a new run is admitted on: the crossings its world stands on, its
/// bundle compiled in the operated repository's realm, and a host that
/// can build the boundary that realm declares.
fn admit_new(request: &LaunchRequest, world: Option<&World>) -> Result<Bundle, LaunchError> {
    fence_crossings(world, &request.workspace)?;
    let dir = request.bundle.resolve()?;
    let leaf = (dir.as_path(), request.shelf.as_deref());
    let bundle = compile_on(&request.workspace, leaf, world, request.operated())?;
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
    compile_on(workspace, (dir, None), world, repo)
}

/// [`compile_for`] of the bundle at `leaf`'s directory, composed across
/// `leaf`'s project shelf, the one the operator selected, else the
/// workspace's own (#697 ruling 3): what a run and `recipes show` compile,
/// so each composes across the shelf its `--recipe` was read from.
pub fn compile_on(
    workspace: &Path,
    leaf: (&Path, Option<&Path>),
    world: Option<&World>,
    repo: &Path,
) -> Result<Bundle, LaunchError> {
    let realm = world.and_then(|world| world.realm_for(repo));
    let name = realm.map_or(UNMAPPED, |realm| realm.name.as_str());
    compile_in_realm(workspace, leaf, world, realm, name, repo)
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
/// never against whatever the workspace's map happens to hold today, and
/// across the project shelf `leaf` names, as a new run's ([`compile_on`]).
fn compile_from_manifest(
    workspace: &Path,
    leaf: (&Path, Option<&Path>),
    manifest: &Value,
    repo: &Path,
) -> Result<Bundle, LaunchError> {
    let Some(world) = World::from_manifest(manifest)? else {
        // A run that pinned no world stood in no realm and held no grant.
        // Its definitions are re-read where they were read when it
        // started: under the operated repository, never the recipe's home
        // and never a map that has appeared in the workspace since.
        let (dir, shelf) = leaf;
        let (places, shelves) = shelved(workspace, shelf)?;
        return Ok(Bundle::compile_unmapped_on(
            (dir, &shelves),
            &places.agents,
            &places.adapters,
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
    compile_in_realm(workspace, leaf, Some(&world), realm, name, repo)
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
/// or none and `namespace` when the world names no such realm, the
/// world's provisional offices (decision 0075 ruling 5), and the
/// capability grants of that realm for the operated `repo`, composed
/// across `leaf`'s project shelf ([`compile_on`]).
fn compile_in_realm(
    workspace: &Path,
    (dir, shelf): (&Path, Option<&Path>),
    world: Option<&World>,
    realm: Option<&Realm>,
    name: &str,
    repo: &Path,
) -> Result<Bundle, LaunchError> {
    let dialect = match world.zip(realm) {
        Some((world, realm)) => world.dialect_for_realm(realm)?,
        None => None,
    };
    let (places, shelves) = shelved(workspace, shelf)?;
    Ok(Bundle::compile_as(
        &crate::argv::Executable::current(),
        (dir, &shelves),
        (&places.agents, &places.adapters),
        Some(name),
        dialect,
        RealmLaw::of(world, realm),
        &capability_context(workspace, world, realm, repo),
    )?)
}

/// The places of `workspace` and the shelves a compile from it composes
/// across (#697 ruling 3): the project `shelf` the operator selected, else
/// its own, and the personal shelf this process's environment places, if
/// it places one.
fn shelved(workspace: &Path, shelf: Option<&Path>) -> Result<(Places, Shelves), PlacesError> {
    let layout = Layout::discover(workspace)?;
    let personal = crate::places::personal_shelf_here().ok();
    Ok((layout.places(), Shelves::of(&layout, shelf, personal)))
}

#[cfg(test)]
pub(crate) mod tests;
