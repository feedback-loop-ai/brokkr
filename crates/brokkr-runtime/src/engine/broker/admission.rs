//! D6's writer admission (decision 0065 slice two, U8a2a; CR2, MB3): one
//! exclusive lease on the canonical worktree per run, sealed as a durable
//! reservation in the protected inventory before the attempt's first
//! writer starts, and released only once its owned processes are proven
//! settled. It is moved out of `broker.rs`, which this would take past its
//! 800 lines (operator ruling of 2026-10-11); the coordination root's
//! handle, the inventory's traversal, the comparison, historical roots and
//! settlement are its siblings.
//!
//! One short-lived host admission lock orders every admission and every
//! release: `admission.lock`, an owner-only regular file of one link in
//! the protected `HOME/.local/state/brokkr/capabilities/` root, held by
//! `flock` only while one of them reads and writes the inventory, never
//! while anything runs. Admission's core is handed that root already
//! opened ([`super::root::Root`]); production resolves it from HOME only
//! once the claim is made. Under the lock, every reservation and every
//! historical root the inventory holds is read ([`super::inventory`]),
//! attempts whose durable settlement facts prove them over are released
//! ([`super::settlement::recovered`]), and the incoming attempt's complete
//! writer reach — every site of its body, each member, step and fallback
//! by the hands its label holds, whether or not it brokers anything — is
//! compared with every participant ([`super::exclusion`]) and with every
//! MCP server source a standing attempt reserved ([`super::sources`]). Its
//! reservation is then sealed (written beside, synced, renamed in, its
//! directory synced), and an artifact root that protects evidence for the first time
//! recorded as historical, all before the lock is released and before
//! anything spawns, within one startup deadline that provisioning then
//! shares. The common dispatch door rechecks that it still stands, and
//! that every boxed writer's reach rebuilt from its authored hands is still
//! the reach sealed ([`recheck`]).
//!
//! A reservation keeps the name of every attempt it ever admitted: each
//! still standing with the reach it reserved, or settled. Settlement moves
//! an attempt from one to the other under the same lock, and never removes
//! the reservation or a historical root's record.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{self, HandsSpec, UNPROTECTED_EVIDENCE};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::exclusion::{compared, staked, Comparison};
use super::historical::recorded;
use super::inventory::{inventory, Budget, Standing};
use super::root::{locked, read, Commit, Root};
use super::settlement::{keyed, recovered, reserved_in, unchanged, Driver};
use super::sources::{records_unopened, unserved, Served, Unroutable};
use super::{startup, Observed};
use crate::bundle::ExecutableBody;
use crate::engine::boxed::{artifact_root, make_root, Unrooted};
use crate::engine::{
    built_boundary, invocation_sites, site_label, BuiltBoundary, Engine, EngineError, EventType,
    Selection,
};

#[cfg(test)]
pub(in crate::engine) mod tests;

/// The host admission lock, in the protected root.
pub(super) const LOCK: &str = "admission.lock";

/// A run's reservation, in its directory below the protected root.
pub(super) const RESERVATION: &str = "reservation.json";

/// What a record is written as before it is renamed in.
pub(super) const STAGED: &str = ".new";

/// Why an attempt is not admitted, or no longer is at the door: CR2's
/// cause where the artifact root cannot be kept from every managed writer,
/// MB3's identity cause where participation cannot be established.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Unadmitted {
    #[error("dispatch refused: {UNPROTECTED_EVIDENCE}: {0}")]
    Evidence(#[from] Conflict),
    #[error("dispatch refused: {identity}: {0}", identity = Refusal::Identity)]
    Identity(Unknown),
    /// The artifact root cannot be made or observed: U8a's own refusal.
    #[error(transparent)]
    Unrooted(#[from] Unrooted),
}

/// What keeps protected evidence or the coordination root from being kept
/// from every managed writer (CR2; the operator's ruling of 2026-10-18).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Conflict {
    #[error("run '{0}' holds this worktree, its writers not proven settled")]
    Held(String),
    #[error(
        "site '{site}' of run '{run}' writes outside a workspace box, which cannot be kept from \
         protected evidence or the coordination root"
    )]
    Unboxed { site: String, run: String },
    #[error(
        "site '{site}' of run '{run}' can reach '{}', which evidence or coordination depends on",
        .root.display()
    )]
    Reaches {
        site: String,
        run: String,
        root: PathBuf,
    },
    #[error(
        "site '{site}' of run '{run}' binds '{}', which a managed writable root could replace \
         to redirect the box it is rebuilt into",
        .bind.display()
    )]
    Replaceable {
        site: String,
        run: String,
        bind: PathBuf,
    },
    #[error("'{}' is visible at a second mount point, or its mount cannot be read", .0.display())]
    Shown(PathBuf),
    #[error(
        "the artifact '{}' is not one file of one link holding the bytes its name digests",
        .0.display()
    )]
    Corrupt(PathBuf),
    #[error("the artifact root '{}' was replaced since its attempt was admitted", .0.display())]
    Replaced(PathBuf),
    #[error(
        "'{}', which a box of the attempt is built from, no longer resolves as when its attempt \
         was admitted",
        .0.display()
    )]
    Redirected(PathBuf),
}

/// What keeps participation from being established (MB3).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Unknown {
    #[error("the capability coordination root is not protected")]
    Root,
    #[error("the operated worktree does not resolve")]
    Worktree,
    #[error("the admission lock is not this user's own single file")]
    Lock,
    #[error("admission did not complete before its startup deadline")]
    Expired,
    #[error("the protected inventory exceeds its entry bound")]
    Bounds,
    #[error("the protected inventory holds an unknown participant at '{}'", .0.display())]
    Participant(PathBuf),
    #[error(
        "the protected inventory holds a reservation at '{}' in format {format}, which this \
         build does not read: conclude its run with the build that wrote it, or use a build \
         that reads it",
        .0.display(),
        format = .1
    )]
    Formatted(PathBuf, u64),
    #[error("the attempt's reservation could not be sealed")]
    Unsealed,
    #[error("the attempt's reservation was committed, but its directory was not proven synced")]
    Unsynced,
    #[error("the attempt's reservation no longer stands")]
    Lost,
    #[error("the attempt's reservation holds no proof that its owned processes settled")]
    Unproven,
    #[error(
        "site '{site}' of run '{run}' can write '{}', which an MCP server's box or lease rests on",
        .served.display()
    )]
    Serving {
        site: String,
        run: String,
        served: PathBuf,
    },
    #[error(
        "site '{site}' of run '{run}' writes outside a workspace box, its privilege not \
         confined, beside an MCP server's sources"
    )]
    Unconfined { site: String, run: String },
    #[error("the mount table does not prove where '{}' lies beside a managed writer", .0.display())]
    Unmounted(PathBuf),
    /// No HOME names where the root lies, so no boxed writer is proved apart.
    #[error(
        "this host names no HOME, so no boxed writer is proved apart from its coordination root"
    )]
    Homeless,
    #[error("the route of '{}', which an MCP server's box is built from, {}", .0.display(), .1)]
    Unrouted(PathBuf, Unroutable),
}

impl From<Unknown> for Unadmitted {
    fn from(unknown: Unknown) -> Unadmitted {
        Unadmitted::Identity(unknown)
    }
}

/// One run's durable reservation of its worktree: the worktree's key, the
/// run, the worktree and artifact root as they resolve, the journal its
/// settlement facts are appended to (M7), each attempt not yet proven
/// settled with the reach it reserved, and each attempt that was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reservation {
    pub(super) repo: String,
    pub(super) run: String,
    pub(super) workdir: PathBuf,
    pub(super) artifacts: PathBuf,
    pub(super) journal: PathBuf,
    pub(super) attempts: BTreeMap<String, Reserved>,
    pub(super) settled: BTreeSet<String>,
}

/// What one attempt reserved: whether it protects evidence, the reach of
/// every writer its body may start, what its engine witnessed of its
/// drivers' cleanup, sealed as it witnesses it (R2; M1), the sources its
/// MCP servers are built from, none where it brokers nothing (U8a2b), and
/// each driver it spawned's own settlement record, in spawn order (U8e2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reserved {
    pub(super) protects: Protects,
    pub(super) writers: Vec<Writer>,
    pub(super) cleanup: Witnessed,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) serves: Vec<Served>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) drivers: Vec<Driver>,
}

/// What the engine has witnessed of an attempt's drivers' cleanup (#403):
/// nothing yet; a driver that ended with its tree not proven over; or, the
/// positive settlement fact, every driver it ran ended settled, which it
/// seals once its body ran and before it releases the attempt (M1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Witnessed {
    Settling,
    Unresolved,
    Settled,
}

/// Whether an attempt protects its artifact root: where it holds evidence
/// already, or a holding the attempt brokers retains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Protects {
    Nothing,
    Evidence,
}

/// One site's writer, by its label, and what it can write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Writer {
    pub(super) site: String,
    pub(super) reach: Reach,
}

/// What a writer can write, and the sources its box binds: a workspace
/// box's writable roots (the worktree, a Git common directory outside it,
/// each read-write bind) and its read-only and overlay bind sources, each
/// as it resolves; every source hands binds apart (the worktree, a Git
/// common directory outside it, each bind) as authored, unresolved (R1);
/// and the reach its every box is built from, sealed ([`hands::Sealed`]:
/// the worktree, its Git directory and Git common directory as Git names
/// them, and each bind's source, each by canonical path and by device and
/// inode; operator ruling 2026-10-18, after U8a2a's third run); or
/// everything its user can, outside one. The authored sources are kept so
/// a source a managed writer could replace with a symlink before its box
/// is rebuilt is refused at admission. The dispatch door observes the
/// sealed reach again before every spawn ([`recheck`]), and hands before
/// every box (U8a2c).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Reach {
    Boxed {
        writable: Vec<PathBuf>,
        readable: Vec<PathBuf>,
        sources: Vec<PathBuf>,
        sealed: Box<hands::Sealed>,
    },
    Unbounded,
}

/// One attempt's claim on its worktree, which admission compares and
/// seals, and the authored hands its boxed writers' reach was resolved
/// from, which the door rebuilds it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Claim {
    pub(super) repo: String,
    pub(super) run: String,
    pub(super) attempt: String,
    pub(super) workdir: PathBuf,
    pub(super) artifacts: PathBuf,
    pub(super) journal: PathBuf,
    pub(super) reserved: Reserved,
    pub(super) authored: Authored,
}

/// What each boxed writer's reach is rebuilt from at the door (R1): each
/// boxed writer's authored hands beside the reach admission sealed from
/// them, the HOME a bind's `~/` named sealed in it, which the engine builds
/// every box of those hands from (U8a2c).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::engine) struct Authored {
    pub(super) boxes: Vec<(HandsSpec, Reach)>,
}

impl Authored {
    /// The reach sealed for a box of the hands `spec`; none where admission
    /// sealed none for them, and so no box of them is built.
    pub(in crate::engine) fn sealed(&self, spec: &HandsSpec) -> Option<&hands::Sealed> {
        self.boxes.iter().find_map(|(authored, reach)| match reach {
            Reach::Boxed { sealed, .. } if authored == spec => Some(&**sealed),
            Reach::Boxed { .. } | Reach::Unbounded => None,
        })
    }
}

impl Claim {
    /// Whether the claim protects its artifact root.
    pub(super) fn protects(&self) -> bool {
        self.reserved.protects == Protects::Evidence
    }
}

/// An admitted attempt's lease, as the engine holds it to the door and to
/// its release: the coordination root it stands in and where in it, the
/// artifact root's identity as admitted where the attempt protects it,
/// whether any driver it ran ended with its tree not proven over (#403),
/// whether evidence was at stake when it was admitted, whether a seal of
/// its reservation committed without its directory proven synced (R1 of
/// U8a2a's fourth run, which fails its run under the fence rule), what the
/// door rebuilds its boxed writers' reach from, and, where a restarted
/// engine adopted it, the settlement records it adopted (U8e2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::engine) struct Held {
    pub(super) coordination: Arc<Root>,
    pub(super) repo: String,
    pub(super) run: String,
    pub(super) attempt: String,
    pub(super) root: Option<(PathBuf, (u64, u64))>,
    pub(super) unproven: bool,
    pub(super) staked: bool,
    pub(super) unsynced: bool,
    pub(super) workdir: PathBuf,
    pub(super) authored: Authored,
    pub(super) adopted: Option<Vec<Driver>>,
}

/// Where admission reads its coordination: the HOME a claim's binds name,
/// and the coordination root, resolved only once the claim is made; none
/// where it cannot be protected or made.
pub(super) type Coordination<'r> = (PathBuf, &'r dyn Fn() -> Option<Root>);

impl Engine {
    /// Admit the attempt its effect, attempt and site name ([`admit`]) on
    /// the host as dispatch `observed` it before anything of it spawns,
    /// within the startup deadline of the attempt that began at `began`
    /// with the bound `bound`, which provisioning then shares ([`startup`];
    /// M8). Whether it was admitted: the lease is then held on the engine
    /// for the dispatch door and the settlement; otherwise the attempt
    /// failed with the refusal before any driver started. The root is
    /// opened under the observed HOME only once the claim is made; a HOME
    /// that does not resolve names no root, and none is resolved from
    /// anywhere else.
    pub(in crate::engine) fn admit(
        &mut self,
        ids: [&str; 3],
        body: ExecutableBody<'_>,
        selection: &Selection,
        (began, bound): (Instant, Duration),
        observed: &Observed,
    ) -> Result<bool, EngineError> {
        let deadline = startup(began, bound);
        let (home, open) = (observed.home.as_deref(), observed.open);
        let root = || home.and_then(open);
        let spelled = home.map(Path::to_path_buf).unwrap_or_default();
        self.admit_in((spelled, &root), ids, body, selection, deadline)
    }

    /// [`Engine::admit`] under `coordination`, before `deadline`. A claim
    /// admitted under a lease holds it on the engine; one that proceeds
    /// holding none (C1), each boxed writer kept from the root's spelled path,
    /// takes no reservation; one refused fails before any driver starts. The
    /// reach the claim sealed for each boxed writer is the one every box of
    /// the attempt is built from (U8a2c); a claim refused leaves none.
    pub(super) fn admit_in(
        &mut self,
        (home, root): Coordination<'_>,
        [effect, attempt, site]: [&str; 3],
        body: ExecutableBody<'_>,
        selection: &Selection,
        deadline: Instant,
    ) -> Result<bool, EngineError> {
        self.reach = Authored::default();
        let claim = match self.claim(&home, [attempt, site], body, selection) {
            Ok(claim) => claim,
            Err(refusal) => return self.refuse_admission(effect, attempt, refusal),
        };
        self.reach = claim.authored.clone();
        let admitted = match coordinated(root(), &claim) {
            Ok(Some(root)) => admit(&Arc::new(root), &claim, deadline),
            Ok(None) => match records_unopened(&claim, &home, deadline) {
                Ok(()) => return self.unreserved(),
                Err(unknown) => Err(unknown.into()),
            },
            Err(unknown) => Err(unknown.into()),
        };
        match admitted {
            Ok(held) => {
                self.held = Some(held);
                Ok(true)
            }
            Err(refusal) => self.refuse_admission(effect, attempt, refusal),
        }
    }

    /// The attempt `attempt` of `effect` failed by `refusal` before its driver.
    fn refuse_admission(
        &mut self,
        effect: &str,
        attempt: &str,
        refusal: Unadmitted,
    ) -> Result<bool, EngineError> {
        let failed = serde_json::json!({
            "effect_id": effect,
            "attempt_id": attempt,
            "error": refusal.to_string(),
        });
        self.append(EventType::EffectFailed, failed, Some(attempt.to_string()))?;
        Ok(false)
    }

    /// Proceed holding no writer lease (C1): the root is not protected, nothing
    /// is at stake and every boxed writer is kept from the root's path, so the
    /// door rechecks nothing and settlement releases nothing. The absence of a
    /// reservation tells a later broker nothing: this writer may be live. No
    /// journal marker either: the seat-record fence (decision 0034) refuses a
    /// checkpoint before the driver, which would lose the attempt.
    fn unreserved(&mut self) -> Result<bool, EngineError> {
        self.held = None;
        Ok(true)
    }

    /// The attempt `attempt` of the site `site`'s claim, under `home`: the
    /// canonical worktree's key, the artifact root as it resolves, made
    /// where the attempt protects it, the journal, and every site of
    /// `body` as a writer.
    fn claim(
        &self,
        home: &Path,
        [attempt, site]: [&str; 2],
        body: ExecutableBody<'_>,
        selection: &Selection,
    ) -> Result<Claim, Unadmitted> {
        let (workdir, repo) = keyed(&self.workdir())?;
        let spelled = artifact_root(&self.run_local());
        let slots = self.slots(site, selection);
        let protects = if slots.iter().any(|slot| slot.retained()) || evidence(&spelled)? {
            make_root(&spelled)?;
            Protects::Evidence
        } else {
            Protects::Nothing
        };
        let (writers, boxes) = self.writers(home, &workdir, site, body);
        let cleanup = Witnessed::Settling;
        Ok(Claim {
            repo,
            run: self.run_id.clone(),
            attempt: attempt.to_string(),
            artifacts: beside(&spelled),
            journal: beside(self.store.path()),
            workdir,
            reserved: Reserved {
                protects,
                writers,
                cleanup,
                serves: Vec::new(),
                drivers: Vec::new(),
            },
            authored: Authored { boxes },
        })
    }

    /// Every site of `body` as a writer: boxed to its hands' writable reach
    /// where its label holds hands under `namespace`, otherwise unbounded;
    /// and each boxed writer's authored hands beside its reach. A member or
    /// a step is composed from its site's own hands, which the ship phase's
    /// journal bind widens here, so those are sealed beside them too: every
    /// box the attempt composes is built from a reach sealed here (U8a2c).
    fn writers(
        &self,
        home: &Path,
        workdir: &Path,
        site: &str,
        body: ExecutableBody<'_>,
    ) -> (Vec<Writer>, Vec<(HandsSpec, Reach)>) {
        let (git, store) = (hands::git_facts(workdir), || self.kept_store().sealed());
        let boxed = built_boundary(self.boundary) == Ok(BuiltBoundary::Namespace);
        let (mut writers, mut boxes) = (Vec::new(), Vec::new());
        for (at, _) in invocation_sites(body) {
            let label = site_label(site, &at);
            let reach = match self.runtime_hands(&label).filter(|_| boxed) {
                Some(spec) => {
                    if let Some(own) = self.hands_for(&label).filter(|own| *own != spec) {
                        let sealed = boxed_reach(&own, home, workdir, &git, store());
                        boxes.push((own, sealed));
                    }
                    let reach = boxed_reach(&spec, home, workdir, &git, store());
                    boxes.push((spec, reach.clone()));
                    reach
                }
                None => Reach::Unbounded,
            };
            writers.push(Writer { site: label, reach });
        }
        (writers, boxes)
    }
}

/// The reach of a box of the authored hands `spec` as hands builds it now
/// ([`super::reach`]), its bound sources apart, and every path it is built
/// from sealed ([`hands::seal`]), the secrets `store` the run keeps (U8h)
/// among them. Hands binds the Git common directory apart, read-write, only
/// outside the worktree, where it is a writable root of its own; inside, it
/// is the worktree's own, neither a source nor a writable root here.
pub(super) fn boxed_reach(
    spec: &HandsSpec,
    home: &Path,
    workdir: &Path,
    git: &hands::GitFacts,
    store: (&Path, &Path),
) -> Reach {
    let mut reach = super::reach(spec, home, workdir, git);
    let (inner, apart): (Vec<_>, Vec<_>) =
        (git.common_dir.iter().cloned()).partition(|common| common.starts_with(workdir));
    if let Some(common) = inner.into_iter().map(super::resolved).next() {
        let own = |(index, root): &(usize, &PathBuf)| (*index == 0) | (**root != common);
        let kept = reach.writable.iter().enumerate().filter(own);
        reach.writable = kept.map(|(_, root)| root.clone()).collect();
    }
    let binds = spec
        .binds
        .iter()
        .map(|bind| super::spelled(&bind.path, home));
    let sources = std::iter::once(workdir.to_path_buf())
        .chain(apart)
        .chain(binds);
    Reach::Boxed {
        writable: reach.writable,
        readable: reach.readable,
        sources: sources.collect(),
        sealed: Box::new(hands::seal(spec, home, workdir, git, Some(store))),
    }
}

/// C1 (operator ruling 2026-10-18): the coordination root `root` resolved
/// for `claim`; or, where none could be protected or made, none, so the
/// dispatch proceeds holding no lease, only where the claim protects
/// nothing and its boxed writers are then kept from the root's path
/// ([`records_unopened`]); else MB3's identity cause. An attempt that brokers
/// still holds a lease past the lifted fence (`sources::held_extended`).
pub(super) fn coordinated(root: Option<Root>, claim: &Claim) -> Result<Option<Root>, Unknown> {
    match root {
        Some(root) => Ok(Some(root)),
        None if claim.protects() => Err(Unknown::Root),
        None => Ok(None),
    }
}

/// `path` as it resolves: canonical, or its parent's canonical path with
/// its own name where only the parent resolves, or as spelled.
pub(super) fn beside(path: &Path) -> PathBuf {
    let parent = || {
        let name = path.file_name()?;
        Some(std::fs::canonicalize(path.parent()?).ok()?.join(name))
    };
    std::fs::canonicalize(path)
        .ok()
        .or_else(parent)
        .unwrap_or_else(|| path.to_path_buf())
}

/// Whether the artifact root `root` holds evidence: anything at all in
/// it. A root that is missing holds none; one that is not a directory of
/// its own, or cannot be read, refuses with U8a's cause.
fn evidence(root: &Path) -> Result<bool, Unrooted> {
    let unread = |error: std::io::Error| Unrooted::Unmade(root.to_path_buf(), error.kind());
    let found = match std::fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        found => found.map_err(unread)?,
    };
    if !found.file_type().is_dir() {
        return Err(Unrooted::Foreign(root.to_path_buf()));
    }
    Ok(std::fs::read_dir(root).map_err(unread)?.next().is_some())
}

/// Admit `claim` in the opened coordination `root` before `deadline`:
/// under the admission lock, the inventory read, proven attempts
/// recovered, every participant compared with it, then its artifact root
/// recorded where it protects evidence for the first time and its
/// reservation sealed beside the run's earlier attempts, all before the
/// lock is released. The lease held, or why there is none.
pub(super) fn admit(
    root: &Arc<Root>,
    claim: &Claim,
    deadline: Instant,
) -> Result<Held, Unadmitted> {
    admit_with(root, claim, deadline, &|| {})
}

/// [`admit`], `between` run between the comparison and the seal: the house
/// `_with` seam a test holds the comparison open at, or spends the deadline
/// in; nothing in production.
pub(super) fn admit_with(
    root: &Arc<Root>,
    claim: &Claim,
    deadline: Instant,
    between: &dyn Fn(),
) -> Result<Held, Unadmitted> {
    let _lock = locked(root.dir(), deadline)?;
    let mut budget = Budget::new(deadline);
    let found = inventory(
        root.dir(),
        root.path(),
        [&claim.repo, &claim.run],
        &mut budget,
    )?;
    let (standing, mut commit) = recovered(root, found.standing, &claim.reserved.writers, &budget)?;
    let comparison = Comparison {
        claim,
        standing: &standing,
        historical: &found.historical,
        coordination: root,
    };
    compared(&comparison, &mut budget)?;
    unserved(claim, &standing, root, &budget)?;
    let staked = staked(&comparison);
    between();
    let artifacts = match claim.reserved.protects {
        Protects::Evidence => Some((claim.artifacts.clone(), identity(&claim.artifacts)?)),
        Protects::Nothing => None,
    };
    if claim.protects() & !found.historical.contains(&claim.artifacts) {
        commit = commit.and(recorded(root, claim, &budget)?);
    }
    commit = commit.and(sealed(root, &joined(claim, standing), &budget)?);
    Ok(Held {
        coordination: Arc::clone(root),
        repo: claim.repo.clone(),
        run: claim.run.clone(),
        attempt: claim.attempt.clone(),
        root: artifacts,
        unproven: false,
        staked,
        unsynced: commit == Commit::Unsynced,
        workdir: claim.workdir.clone(),
        authored: claim.authored.clone(),
        adopted: None,
    })
}

/// The run's reservation with `claim`'s attempt standing in it: the one
/// `standing` holds for that very worktree and run, which the inventory
/// read whole, or a new one.
fn joined(claim: &Claim, standing: Vec<Standing>) -> Reservation {
    let mine = |held: &Reservation| (held.repo == claim.repo) & (held.run == claim.run);
    let mut reservation = standing
        .into_iter()
        .filter_map(|held| held.whole)
        .find(mine)
        .unwrap_or_else(|| Reservation {
            repo: claim.repo.clone(),
            run: claim.run.clone(),
            workdir: claim.workdir.clone(),
            artifacts: claim.artifacts.clone(),
            journal: claim.journal.clone(),
            attempts: BTreeMap::new(),
            settled: BTreeSet::new(),
        });
    let reserved = claim.reserved.clone();
    reservation.attempts.insert(claim.attempt.clone(), reserved);
    reservation
}

/// Whether the lease `held` still stands at the dispatch door: its
/// reservation, read under the admission lock, still names its attempt
/// standing, every path each boxed writer's box is built from, observed
/// again from its authored hands and from Git, still resolves as sealed (R1;
/// operator ruling 2026-10-18, after U8a2a's third run), under the fence
/// rule, and the artifact root it protects is still the directory admitted.
pub(in crate::engine) fn recheck(held: &Held) -> Result<(), Unadmitted> {
    let standing: Reservation = {
        let deadline = startup(Instant::now(), Duration::MAX);
        let _lock = locked(held.coordination.dir(), deadline)?;
        let (run, at) = reserved_at(held)?;
        read(&run, RESERVATION, &at).or(Err(Unknown::Lost))?
    };
    let ours = (standing.repo == held.repo) & (standing.run == held.run);
    let ours = ours & standing.attempts.contains_key(&held.attempt);
    ours.then_some(()).ok_or(Unknown::Lost)?;
    // Hands builds every box from the reach sealed, and refuses one that
    // no longer resolves as sealed (U8a2c); a path that resolves elsewhere
    // now than when it was sealed — a bind source swapped, the worktree's
    // Git pointer or `commondir` rewritten — was redirected since
    // admission, and the door refuses it before the driver too, as the
    // comparison's refusals are, where evidence is at stake.
    if let Some(moved) = redirected(held).filter(|_| held.staked) {
        return Err(Conflict::Redirected(moved).into());
    }
    let Some((root, admitted)) = &held.root else {
        return Ok(());
    };
    let replaced = || Conflict::Replaced(root.clone());
    (identity(root).ok().as_ref() == Some(admitted))
        .then_some(())
        .ok_or_else(replaced)?;
    Ok(())
}

/// The first path of `held`'s boxed writers' sealed reach that, observed
/// again as hands observes it before every box ([`hands::drifted`]), no
/// longer resolves as sealed; a box held with none sealed is redirected by
/// its worktree, for what was never sealed never rechecks clean; none
/// where every one resolves as sealed.
fn redirected(held: &Held) -> Option<PathBuf> {
    (held.authored.boxes.iter()).find_map(|(spec, was)| match was {
        Reach::Boxed { sealed, .. } => hands::drifted(sealed, spec, &held.workdir),
        Reach::Unbounded => Some(held.workdir.clone()),
    })
}

/// The directory `held`'s reservation stands in, and where.
pub(super) fn reserved_at(held: &Held) -> Result<(std::os::fd::OwnedFd, PathBuf), Unknown> {
    reserved_in(&held.coordination, [&held.repo, &held.run], true)
}

/// Release the lease `held` under the admission lock, taken before
/// `deadline`: its attempt moved from its run's standing attempts to its
/// settled ones, and the reservation sealed again, only where the
/// reservation holds the attempt's positive settlement fact (M1) and an
/// adopted lease's settlement records stand as adopted ([`unchanged`]).
pub(super) fn release(held: &Held, deadline: Instant) -> Result<Commit, Unadmitted> {
    amend(held, deadline, |reservation| {
        unchanged(&reservation, held)?;
        settled(reservation, &held.attempt)
    })
}

/// The lease `held`'s reservation, read and sealed again as `amended`
/// leaves it, under the admission lock taken before `deadline`; or, where
/// `amended` refuses it, left as it stands.
pub(super) fn amend(
    held: &Held,
    deadline: Instant,
    amended: impl FnOnce(Reservation) -> Result<Reservation, Unknown>,
) -> Result<Commit, Unadmitted> {
    let _lock = locked(held.coordination.dir(), deadline)?;
    let (run, at) = reserved_at(held)?;
    let reservation: Reservation = read(&run, RESERVATION, &at)?;
    let budget = Budget::new(deadline);
    Ok(sealed(&held.coordination, &amended(reservation)?, &budget)?)
}

/// `reservation` with its standing attempt `attempt` settled, where its
/// engine sealed the attempt's positive settlement fact in it; refused
/// otherwise, the attempt left standing.
pub(super) fn settled(mut reservation: Reservation, attempt: &str) -> Result<Reservation, Unknown> {
    let proven = reservation.attempts.get(attempt);
    let proven = proven.is_some_and(|reserved| reserved.cleanup == Witnessed::Settled);
    proven.then_some(()).ok_or(Unknown::Unproven)?;
    reservation.attempts.remove(attempt);
    reservation.settled.insert(attempt.to_string());
    Ok(reservation)
}

/// The directory `path`'s device and inode, where it is a directory of its
/// own.
pub(super) fn identity(path: &Path) -> Result<(u64, u64), Unrooted> {
    let found = std::fs::symlink_metadata(path);
    let found = found.map_err(|error| Unrooted::Unmade(path.to_path_buf(), error.kind()))?;
    let own = found.file_type().is_dir();
    own.then_some((found.dev(), found.ino()))
        .ok_or_else(|| Unrooted::Foreign(path.to_path_buf()))
}

/// `reservation` sealed in its run's directory below `root`
/// ([`Root::seal`]).
pub(super) fn sealed(
    root: &Root,
    reservation: &Reservation,
    budget: &Budget,
) -> Result<Commit, Unknown> {
    let run = root.below(&[reservation.repo.as_str(), reservation.run.as_str()])?;
    let bytes = serde_json::to_vec(reservation).or(Err(Unknown::Unsealed))?;
    root.seal(&run, RESERVATION, &bytes, budget)
}
