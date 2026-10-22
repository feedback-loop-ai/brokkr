//! The durable engine loop. Every external effect is requested durably
//! before execution and completed, failed, cancelled, or marked
//! indeterminate by a later event (decision 0003). The loop derives its
//! next action purely from `fold(journal)` + the pinned bundle; nothing
//! in here decides a transition — only the policy does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::dispatch::{
    build_run_manifest_v2, bundle_manifest_from_run, DispatchEnvelopeV2, DispatchError,
};
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, ParkReason, Run, RunState};
use brokkr_core::gate_moved_head::{self, MovedHead};
use brokkr_core::realms::Boundary;
use brokkr_core::EventEnvelope;
use brokkr_protocol::hands::{HandsSpec, Sealed, SEALED_REACH};
use brokkr_protocol::native_controls::{
    flatten, pin_fault, reassemble, AllowIntent, Application, Expected, HandsIntent, LaunchRecord,
    LocalExpectation, Origin, SandboxIntent, SealedBroker, SealedServing, Segment,
    TemplateExpectation, ARTIFACT_ROOT, SERVING_INPUTS,
};
use brokkr_protocol::process::SpawnEnv;
use brokkr_protocol::AttemptOutcome;
use brokkr_store::{Store, StoreError};
use serde_json::{json, Map, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::agents::{Candidate, Lowering, ResultDoor};
use crate::argv::Executable;
use crate::bundle::{
    charters_as_started, charters_intact, Bundle, CharterPin, ExecutableBody, HandsState, Seat,
    SeatBody, SeatClass, SiteFacts, StepBody, ENGINE_VERSION,
};
use crate::{dialect::DialectPhase, phase_name};
// The test modules reach these through `use super::*`.
#[cfg(test)]
use crate::bundle::{Aggregate, Invocation, PanelMember, SequenceStep};
#[cfg(test)]
use brokkr_protocol::Cleanup;
use brokkr_protocol::{AttemptReport, CleanupEvidence};
use serde::Serialize;

pub mod artifacts;
mod boxed;
mod broker;
use boxed::{artifact_root, boxed_spawn, make_root};
// The test modules compare a box's argv through `use super::*`.
#[cfg(test)]
use boxed::hands_command;
mod capability_calls;
mod checkpoints;
mod decide;
mod dispatch;
use dispatch::{spawn_site, Door};
mod fence;
mod input;
mod isolation;
use capability_calls::Calls;
use checkpoints::Checkpoints;
pub use isolation::serving_inputs;
mod marks;
#[doc(hidden)]
pub use marks::SiteMarks;
mod operator;
mod panel;
mod replay;
mod sequence;
use operator::operator_stop_reason;
use panel::{panel_outcome, start_failure_sites};
// The test modules build a member's run through `use super::*`.
#[cfg(test)]
use panel::MemberRun;
// The test modules reach the racing twins, the fold, and the vocabulary
// they command and refuse in, through `use super::*`.
#[cfg(test)]
use brokkr_core::fold::{
    fold,
    OperatorCommand::{self, Retry, Stop},
    Refusal,
};
pub use operator::{
    apply_fenced_operator_command, conclude, operator_command, operator_supersede, CommandWord,
    ConcludeRefusal, FencedCommand, FencedCommandOutcome, Supersede,
};
#[cfg(test)]
use operator::{
    apply_fenced_racing, conclude_racing, operator_command_racing, operator_supersede_racing,
    riding_attempt, FENCE_ATTEMPTS,
};

fn nearest_change(context: &Value) -> Option<String> {
    [
        DialectPhase::Tasks.as_str(),
        DialectPhase::Design.as_str(),
        DialectPhase::Specify.as_str(),
        phase_name::TRIAGE,
        phase_name::INTAKE,
    ]
    .iter()
    .find_map(|phase| {
        context
            .pointer(&format!("/results/{phase}/inputs/change"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn expand_dialect_argv(argv: &[String], change: &str) -> Vec<String> {
    argv.iter()
        .map(|token| token.replace("{change}", change))
        .collect()
}

/// A dialect step's outcome, and into `evidence` what its terminal event
/// carries when its tree is not proven over (#403).
fn dialect_attempt_outcome(run: DriverRun, evidence: &mut Unproven) -> AttemptOutcome {
    match run {
        DriverRun::SpawnFailed(error) => AttemptOutcome::Failed { error },
        DriverRun::Ran(report) => {
            *evidence = Unproven::seat(&report);
            report.settled_outcome()
        }
    }
}

/// What an `effect/indeterminate` carries of an attempt not proven over
/// (#403), the fields `contracts/effect-cleanup.v1.schema.json` publishes:
/// nothing once proven over, so the event keeps its v1 bytes; a seat's own
/// evidence; or a panel's, by member. The member's own marker is a closed
/// seat record and carries none.
#[derive(Debug, Default, Serialize)]
#[serde(untagged)]
enum Unproven {
    #[default]
    Proven,
    Seat(CleanupEvidence),
    Panel {
        unresolved_members: BTreeMap<String, CleanupEvidence>,
    },
}

impl Unproven {
    fn seat(report: &AttemptReport) -> Self {
        report
            .cleanup_evidence()
            .map_or(Unproven::Proven, Unproven::Seat)
    }

    fn panel(reports: &[(String, AttemptReport)]) -> Self {
        let unresolved_members: BTreeMap<String, CleanupEvidence> = reports
            .iter()
            .filter_map(|(name, report)| Some((name.clone(), report.cleanup_evidence()?)))
            .collect();
        if unresolved_members.is_empty() {
            Unproven::Proven
        } else {
            Unproven::Panel { unresolved_members }
        }
    }
}

/// An `effect/indeterminate` payload: its v1 fields, and beside them what
/// of the attempt is not proven over.
#[derive(Serialize)]
struct Indeterminate<'a> {
    effect_id: &'a str,
    attempt_id: &'a str,
    reason: String,
    #[serde(flatten)]
    unproven: Unproven,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] brokkr_store::StoreError),
    #[error("fold: {0}")]
    Fold(#[from] brokkr_core::FoldError),
    #[error("run '{run_id}' pins a different bundle: {detail}")]
    ManifestMismatch { run_id: String, detail: String },
    /// `conclude` refused the run, for a reason the operator can act on.
    #[error("{}", .why.text(.run_id))]
    ConcludeRefused {
        run_id: String,
        why: ConcludeRefusal,
    },
    /// `supersede` refused a citation, and wrote nothing (decision 0047
    /// ruling 2). Unlike `retry` and `stop`, a refused supersede leaves
    /// no `operator/rejected` behind: there is no pending command to
    /// dispose of, so the refusal is the operator's to read at the
    /// terminal and the journal stays exactly as it was.
    #[error("supersede refused, nothing was written: {0}")]
    SupersedeRefused(String),
    /// Decision 0029: an event the engine decided on a fold met a head a
    /// peer had moved. Nothing was written, and the engine ends here
    /// rather than folding again: two writers believe they drive one run.
    #[error(
        "run '{run_id}': the journal moved beneath the fold this engine decided on (seq \
         {expected_seq}, now {found_seq}), so something else may be driving the run; nothing \
         was written — look with `brokkr runs` before resuming (decision 0029)"
    )]
    JournalMoved {
        run_id: String,
        expected_seq: u64,
        found_seq: u64,
    },
    /// A broken invariant of the engine or the fold, a defect and not a
    /// refusal: its name, and the words the operator has always read.
    #[error("engine: {1}")]
    Invariant(&'static str, String),
    #[error("dispatch: {0}")]
    Dispatch(#[from] brokkr_core::dispatch::DispatchError),
    /// Decision 0065 slice two, U8a2a (D6, M1): an attempt's settlement
    /// could not be sealed where the fence rule asks it; its lease stands.
    #[error(transparent)]
    Unreleased(#[from] broker::settlement::Unreleased),
    #[error("realms: {0}")]
    World(#[from] crate::realms::WorldError),
    /// Decision 0046 ruling 1, at the engine's own door: the bundle was
    /// compiled under one word and the world the run is started with
    /// resolves another for the operated repository. Refused before
    /// `create_run`, so the pinned word and the world cannot disagree
    /// (design DD3).
    #[error(
        "the bundle was compiled under the `{compiled}` boundary but the realms map this run \
         is started with resolves `{world}` for the operated repository; a run stands under \
         one word (decision 0046 ruling 1)"
    )]
    BoundaryMismatch { compiled: Boundary, world: Boundary },
    /// Decision 0065 ruling 3 at the same door: a bundle holds what the
    /// realm it was COMPILED in grants, so it starts only in a world whose
    /// operated realm grants exactly that. A different grant context is a
    /// different authority, refused before any row is written.
    #[error(
        "this bundle was compiled under the capabilities realm '{compiled_realm}' grants \
         ({compiled}), and the world it is started with resolves realm '{world_realm}' \
         granting {world} for the operated repository; a seat holds only what the realm it \
         runs in grants, so recompile in this world (decision 0065 ruling 3)"
    )]
    CapabilityMismatch {
        compiled_realm: String,
        compiled: String,
        world_realm: String,
        world: String,
    },
    /// The same door, for the bytes behind the grants (design D7): the
    /// manifest pins every abstract definition and tool dialect the compile
    /// consulted, by relative source and digest, and a resume must be able
    /// to reproduce them. A run is not journaled against an input that has
    /// already moved.
    #[error(
        "this bundle's capabilities were compiled against '{input}', which {problem} in the \
         operator configuration this run is started with; a run pins the abstract definitions \
         and tool dialects its holdings came from, so recompile in this world (decision 0065 \
         ruling 8)"
    )]
    CapabilityInputMoved { input: String, problem: String },
    /// Rebuild unit 19 (design D7; decision 0066 ruling 5): a charter the
    /// bundle bound moved since its compile — its owner, its target or its
    /// bytes. Checked at start before `create_run` and at resume before the
    /// run is driven, by the same bound read as the dispatch door.
    #[error(
        "a charter of {owner} moved since the compile ({key}); a run is started or resumed only \
         over the charters the bundle's identity names, so restore it, or recompile and start a \
         new run (decision 0066 ruling 5)"
    )]
    CharterMoved { owner: String, key: String },
    /// Decision 0046 ruling 6: `seatbelt` and `container` are named, pinned
    /// and admitted at compile, and built by slices (ii) and (iii). This
    /// engine composes nothing for either, and never simulates a boundary,
    /// so a bundle with hands under one refuses before any row is written.
    #[error(
        "the `{boundary}` boundary is built by slice ({slice}) of decision 0046 ruling 6, \
         not by this engine; the seats {seats:?} declare hands and cannot run under it here \
         — a realm may declare `harness` today"
    )]
    UnbuiltBoundary {
        boundary: Boundary,
        slice: &'static str,
        seats: Vec<String>,
    },
}

/// Which slice of decision 0046 ruling 6 builds a boundary this engine
/// does not: the refusing arm of the narrowing from five words to the
/// three composition is written over (design DD17). `None` for the
/// three this engine builds.
pub(crate) fn unbuilt_slice(boundary: Boundary) -> Option<&'static str> {
    built_boundary(boundary).err()
}

/// The three mechanisms implemented by this engine. The entry fence
/// narrows the realm vocabulary before composition (DD2, DD17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltBoundary {
    Namespace,
    Harness,
    Open,
}

fn built_boundary(boundary: Boundary) -> Result<BuiltBoundary, &'static str> {
    match boundary {
        Boundary::Namespace => Ok(BuiltBoundary::Namespace),
        Boundary::Harness => Ok(BuiltBoundary::Harness),
        Boundary::Open => Ok(BuiltBoundary::Open),
        Boundary::Seatbelt => Err("ii"),
        Boundary::Container => Err("iii"),
    }
}

/// The engine's entry fence for a boundary it does not build: refused
/// only when the bundle asks for a box at all, because a bundle with no
/// hands site asks nothing of the machine.
fn refuse_unbuilt(bundle: &Bundle) -> Result<(), EngineError> {
    match unbuilt_slice(bundle.boundary) {
        Some(slice) if !bundle.hands.is_empty() => Err(EngineError::UnbuiltBoundary {
            boundary: bundle.boundary,
            slice,
            seats: bundle.hands.keys().cloned().collect(),
        }),
        _ => Ok(()),
    }
}

/// Rebuild unit 19 (design D7): the start and resume doors' charter check.
/// Every binding the compile selected, library and layer alike, is read
/// again through its owner; a recompile is not what makes it pass. `Ok` is
/// the bindings checked, which a start records in its `run/started` event.
fn refuse_moved_charter(bundle: &Bundle) -> Result<Value, EngineError> {
    charters_intact(bundle).map_err(charter_moved)
}

fn charter_moved((owner, key): (String, String)) -> EngineError {
    EngineError::CharterMoved { owner, key }
}

impl EngineError {
    /// The contention this error carries, if that is what it is.
    ///
    /// [`EngineError::Store`] is `transparent`, so the store error it
    /// holds is NOT a link in its `source()` chain — a caller walking an
    /// `anyhow` chain would step straight past it. This is the door
    /// through, and it asks the store's own predicate rather than
    /// reading error text.
    pub fn contention(&self) -> Option<&brokkr_store::StoreError> {
        match self {
            EngineError::Store(error) if error.is_contention() => Some(error),
            _ => None,
        }
    }
}

/// The phase a running cursor stands in, or the breach worded as `site`.
fn phase_of(state: &RunState, site: &'static str) -> Result<String, EngineError> {
    let missing = || EngineError::Invariant("cursor has phase", site.into());
    state.phase.clone().ok_or_else(missing)
}

pub struct Engine {
    pub store: Store,
    pub bundle: Bundle,
    pub run_id: String,
    pub feature: String,
    /// The operated repository, resolved once by `operated_repo`: the
    /// tree seats work in and every git fact reads.
    pub repo: PathBuf,
    /// The world this run was invoked into (decision 0023), when a map
    /// was in effect. It is pinned into the run manifest at start, so
    /// this field is the run's *live* copy of a fact the journal already
    /// answers for; the facts a decision records are keyed by the realm
    /// the repository is, when the map names one.
    pub world: Option<crate::realms::World>,
    /// The event_id every append chains to as `causation_id` — refreshed
    /// to the journal head each drive iteration, then to each event this
    /// iteration appends, so causal links mirror the engine's actual
    /// decision order (rendered by the UI timeline).
    current_cause: Option<String>,
    /// Operator-side secrets store override (decision 0012). Defaults to
    /// `secrets.env` in the run-local directory. The engine only ever threads this
    /// PATH (plus declared names) into the driver start input — values
    /// are resolved by the exec driver at spawn time; no store read
    /// exists anywhere in brokkr-runtime.
    pub secrets_file: Option<PathBuf>,
    /// The folder's `state/` the run's results, scratch and secrets stand
    /// in (decision 0083), handed by the launch; `None` is `<workdir>/.forge`.
    pub(crate) state: Option<PathBuf>,
    /// Present only while this process is executing a gate-bearing site,
    /// and it holds exactly one observation: the head as that site's own
    /// span began. The span is the gate STEP inside a sequence and the
    /// whole effect for a gate single seat or panel — never both at once,
    /// so a sequence's steps are the only observation a sequence makes.
    /// The observations are compared at the end of the span that armed
    /// them; only a mismatch is journaled, as the raw evidence on the
    /// resulting park. A span that ends through an `Err` — appending no
    /// terminal event — clears it rather than leaving it for the next
    /// effect to spend.
    active_gate_head: Option<Option<String>>,
    /// The boundary every site with hands is composed from (decision
    /// 0046 ruling 1): the compiled bundle's word, set once at entry and
    /// never re-read from a map, so the pinned word and the world the run
    /// was started with cannot disagree (design DD3).
    pub boundary: Boundary,
    /// Whether the network prefix works on this machine, asked once per
    /// engine process at the first unboxed exec dispatch and remembered
    /// (decision 0046 ruling 4; design DD15). Never journaled.
    network_prefix: Option<bool>,
    replay: replay::Replay,
    /// The attempt's terminal event a peer's lock kept out, carried to
    /// the lawful end (#394). Never journaled as such.
    held_outcome: Option<checkpoints::HeldOutcome>,
    /// The head the next append must still find, as [`fence::tip_of`]
    /// took it (decision 0029); `None` while an attempt is in flight.
    fence: Option<(u64, String)>,
    /// The writer lease the attempt in flight was admitted under (U8a2a).
    held: Option<broker::Held>,
    /// The reach its admission sealed, which its every box is built from.
    reach: broker::Authored,
}

pub(crate) fn verify_dispatch_bundle_bounds(
    dispatch: &DispatchEnvelopeV2,
    bundle: &Bundle,
) -> Result<(), DispatchError> {
    let max_attempts = bundle
        .seats
        .values()
        .map(|seat| seat.limits.max_attempts)
        .max()
        .unwrap_or(1);
    fn body_parallel(body: &SeatBody) -> usize {
        match body {
            SeatBody::Invocation(invocation) => invocation.parallel(),
            SeatBody::Sequence { steps } => steps
                .iter()
                .map(|step| match &step.body {
                    StepBody::Invocation(invocation) => invocation.parallel(),
                    StepBody::Dialect { .. } => 1,
                })
                .max()
                .unwrap_or(1),
            SeatBody::Select { cases, default, .. } => cases
                .values()
                .chain(default.iter().map(Box::as_ref))
                .map(body_parallel)
                .max()
                .unwrap_or(1),
        }
    }
    let max_parallel = bundle
        .seats
        .values()
        .map(|seat| body_parallel(&seat.body))
        .max()
        .unwrap_or(1);
    if max_attempts > u64::from(dispatch.bounds.max_attempts)
        || max_parallel > dispatch.bounds.max_parallel_effects as usize
    {
        return Err(DispatchError::UnsafeBounds);
    }
    Ok(())
}

#[derive(Debug)]
pub struct DriveEnd {
    pub state: RunState,
    /// The best-effort acts at a conclusion that fell short — an anchor
    /// or keep-ref gap, one line each. Never fatal; the caller reports
    /// them, because the engine writes to no stream of its own.
    pub gaps: Vec<String>,
}

impl Engine {
    fn enter_phase(&mut self, phase: &str, state: &RunState) -> Result<(), EngineError> {
        let seat = self.bundle.seats.get(phase);
        if let Some(seat) = seat {
            if seat.body.selected(state.strategy.as_deref()).is_none() {
                self.append(
                    EventType::RunParked,
                    json!({
                        "reason": format!(
                            "SELECT-NO-DEFAULT: seat '{phase}' selects on strategy, but the journal carries no matching triage result and the seat has no default"
                        ),
                        "evidence": {}
                    }),
                    None,
                )?;
                return Ok(());
            }
        }
        let payload = self.phase_entered_payload(phase, state);
        self.append(EventType::PhaseEntered, payload, None)?;
        Ok(())
    }

    pub fn start(
        store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
    ) -> Result<Engine, EngineError> {
        Engine::start_in_world(store, bundle, feature, repo, None)
    }

    /// Start a run inside a world (decision 0023). The map is pinned by
    /// content hash AND embedded verbatim into the run manifest — which
    /// rides inside `run/started` — so "what world did this run believe
    /// in?" is answerable from the journal alone, forever, whatever
    /// later became of the file. With no map the manifest is byte-for-
    /// byte the one this engine has always written.
    pub fn start_in_world(
        mut store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
        world: Option<crate::realms::World>,
    ) -> Result<Engine, EngineError> {
        let slug: String = feature
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let slug = slug.chars().take(32).collect::<String>();
        let run_id = format!("{slug}-{}", &Uuid::new_v4().to_string()[..8]);
        // Decision 0046 ruling 1 at the door, before `create_run` and
        // before any row: the world this run is started with must resolve
        // the word the bundle was compiled under for the operated
        // repository — and no world resolves `namespace`, which is what
        // every run meant before the word existed (design DD3).
        refuse_unbuilt(&bundle)?;
        // The operated repository is what `workdir()` will answer: the
        // named one, else the directory the engine stands in.
        let operated = operated_repo(repo);
        let resolved = match &world {
            Some(world) => world.boundary_for(&operated),
            None => Boundary::Namespace,
        };
        if resolved != bundle.boundary {
            return Err(EngineError::BoundaryMismatch {
                compiled: bundle.boundary,
                world: resolved,
            });
        }
        // Decision 0065 ruling 3, fenced the same way and for the same
        // reason: the grants the operated realm declares TODAY are what
        // this run may hold, and a bundle compiled under any other grant
        // context — a neighbouring realm's, an edited map's, none at all
        // — is refused before `create_run`, so nothing is journaled and no
        // seat spawns.
        let operated_realm = world.as_ref().and_then(|world| world.realm_for(&operated));
        let world_realm = operated_realm.map_or(crate::capabilities::UNMAPPED, |realm| &realm.name);
        let world_grants: Map<String, Value> = operated_realm
            .into_iter()
            .flat_map(|realm| &realm.grants)
            .map(|(capability, grant)| (capability.clone(), grant.value()))
            .collect();
        // What is compared is the realm and its GRANTS, which is the
        // authority: a bundle that records none was compiled under none,
        // and a neighbouring realm granting the very same map is still a
        // different office-holder's permission, not this one's.
        let compiled = &bundle.manifest["capabilities"];
        let compiled_grants = compiled.get("grants").cloned().unwrap_or_else(|| json!({}));
        let compiled_realm = compiled["realm"]
            .as_str()
            .unwrap_or(crate::capabilities::UNMAPPED);
        if compiled_grants != json!(world_grants) || compiled_realm != world_realm {
            return Err(EngineError::CapabilityMismatch {
                compiled_realm: compiled_realm.to_string(),
                compiled: compiled_grants.to_string(),
                world_realm: world_realm.to_string(),
                world: Value::Object(world_grants).to_string(),
            });
        }
        // And the bytes those grants were judged against, read where the
        // compile read them: beside the map, else under the operated
        // repository (design D2).
        let operator_root = match &world {
            Some(world) => world.source.parent().map(PathBuf::from).unwrap_or_default(),
            None => operated.clone(),
        };
        if let Some((input, problem)) = moved_capability_input(compiled, &operator_root) {
            return Err(EngineError::CapabilityInputMoved { input, problem });
        }
        // And every charter the bundle bound, where its owner stands now
        // (rebuild unit 19): no run is journaled over one that moved. The
        // bindings checked are recorded with the run, so its resume, over a
        // bundle compiled again, is held to them (review return F1).
        let charters = refuse_moved_charter(&bundle)?;
        // Pinned for the same operated repository the fence judged, so a
        // run started from a mapped workspace with no `--repo` still
        // names its realm — and resumes under the word it was started
        // with, which `compile_from_manifest` reads off that pin.
        let manifest = match &world {
            Some(world) => world.pinned(&bundle.manifest, Some(&operated))?,
            None => bundle.manifest.clone(),
        };
        store.create_run(&run_id, feature, &bundle.name, &manifest)?;
        store.append_next(
            &run_id,
            EventType::RunStarted,
            bundle.run_started(feature, &manifest, charters),
            None,
            None,
        )?;
        Ok(Engine {
            store,
            boundary: bundle.boundary,
            bundle,
            run_id,
            feature: feature.to_string(),
            repo: operated,
            reach: Default::default(),
            world,
            current_cause: None,
            secrets_file: None,
            state: None,
            active_gate_head: None,
            network_prefix: None,
            replay: Default::default(),
            held_outcome: None,
            fence: None,
            held: None,
        })
    }

    /// Start a Looper-bound run under the exact wire run id and immutable
    /// dispatch envelope. The ordinary bundle manifest remains recoverable
    /// for resume compatibility, while the stored/exported manifest is v2.
    pub fn start_with_dispatch(
        mut store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
        dispatch: DispatchEnvelopeV2,
    ) -> Result<Engine, EngineError> {
        let run_id = dispatch.forge_run_id.clone();
        refuse_unbuilt(&bundle)?;
        dispatch.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
        verify_dispatch_bundle_bounds(&dispatch, &bundle)?;
        let charters = refuse_moved_charter(&bundle)?;
        let manifest = build_run_manifest_v2(&bundle.manifest, dispatch)?;
        store.create_run(&run_id, feature, &bundle.name, &manifest)?;
        store.append_next(
            &run_id,
            EventType::RunStarted,
            bundle.run_started(feature, &manifest, charters),
            None,
            None,
        )?;
        Ok(Engine {
            store,
            boundary: bundle.boundary,
            bundle,
            run_id,
            feature: feature.to_string(),
            repo: operated_repo(repo),
            reach: Default::default(),
            // A Looper-bound run pins a run-manifest/v2, whose bytes a
            // counterpart system reads; the map's pin belongs to the
            // v1→v5 local lineage. The CLI refuses the combination
            // rather than dropping a world silently.
            world: None,
            current_cause: None,
            secrets_file: None,
            state: None,
            active_gate_head: None,
            network_prefix: None,
            replay: Default::default(),
            held_outcome: None,
            fence: None,
            held: None,
        })
    }

    /// Resume uses the exact pinned bundle or refuses with a diagnostic.
    pub fn resume(
        mut store: Store,
        bundle: Bundle,
        run_id: &str,
        repo: Option<PathBuf>,
    ) -> Result<Engine, EngineError> {
        refuse_unbuilt(&bundle)?;
        let pinned = store.manifest(run_id)?;
        let pinned_bundle = bundle_manifest_from_run(&pinned)?;
        if let Some(dispatch) = brokkr_core::dispatch::dispatch_from_run(&pinned)? {
            dispatch.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
            verify_dispatch_bundle_bounds(&dispatch, &bundle)?;
        }
        // The boundary needs no second fence here: the manifest pins it
        // per hands site, so a bundle compiled under another word is a
        // manifest mismatch whose diff names `boundary` (design DD3).
        if pinned_bundle != bundle.manifest {
            let detail = manifest_diff(&pinned_bundle, &bundle.manifest);
            return Err(EngineError::ManifestMismatch {
                run_id: run_id.to_string(),
                detail,
            });
        }
        // An equal manifest is the pinned identity, not the pinned bytes:
        // each charter it names is read again through its owner, whether the
        // run pinned a world or none (rebuild unit 19), and must be the
        // binding the run STARTED over: the bundle here may be compiled
        // again, and a recompile binds an equal-byte retarget afresh without
        // moving the manifest (review return F1). A pin that does not answer
        // for itself is tampering, and refuses before any charter is read.
        let world = crate::realms::World::from_manifest(&pinned)?;
        let mut replay = replay::Replay::default();
        let feature = replay.caught_up(&mut store, run_id)?.feature;
        let started = replay
            .events
            .first()
            .filter(|event| event.event_type == EventType::RunStarted)
            .and_then(|event| event.payload.get("charters"));
        charters_as_started(&bundle, started).map_err(charter_moved)?;
        Ok(Engine {
            store,
            boundary: bundle.boundary,
            bundle,
            run_id: run_id.to_string(),
            feature: feature.unwrap_or("unknown".to_string()),
            repo: operated_repo(repo),
            reach: Default::default(),
            // Resume takes no map — and needs none. The world this run
            // believed in is pinned in the manifest just read, content
            // and all, so it is rehydrated from evidence rather than off
            // a disk that may have moved on. Without this a resumed run
            // would silently stop keying its facts by realm, changing
            // fact-shape mid-run depending on which verb was typed.
            world,
            current_cause: None,
            secrets_file: None,
            state: None,
            active_gate_head: None,
            network_prefix: None,
            replay,
            held_outcome: None,
            fence: None,
            held: None,
        })
    }

    /// Drive the run until it parks, completes, or stops.
    ///
    /// Or until a peer's lock on the shared journal outlasts the store's
    /// whole patience — which is a fourth ending, and it is an ending,
    /// not a death. See [`Engine::lawful_end_under_contention`].
    pub fn drive(&mut self) -> Result<DriveEnd, EngineError> {
        self.drive_racing(|_| {})
    }

    /// [`Engine::drive`] with the lawful end's window held open:
    /// `before_lawful_end` sees each error a turn ends on, before the
    /// lawful end is tried. Production passes a no-op; a test passes the
    /// peer that lets go of its lock there, so a terminal event the lock
    /// outlasted can only land by the lawful end.
    fn drive_racing(
        &mut self,
        mut before_lawful_end: impl FnMut(&EngineError),
    ) -> Result<DriveEnd, EngineError> {
        loop {
            match self.drive_once() {
                Ok(Some(end)) => return Ok(end),
                Ok(None) => {}
                Err(error) => {
                    before_lawful_end(&error);
                    if let Some(end) = self.lawful_end_under_contention(error)? {
                        return Ok(end);
                    }
                }
            }
        }
    }

    /// One turn of the loop: `Some(end)` when the run has reached its
    /// conclusion, `None` when there is more to do.
    fn drive_once(&mut self) -> Result<Option<DriveEnd>, EngineError> {
        // Lent to the turn, and handed back only by a turn that ends well.
        let mut replay = std::mem::take(&mut self.replay);
        let state = replay.caught_up(&mut self.store, &self.run_id)?;
        (self.current_cause, self.fence) = fence::tip_of(&replay.events);
        match &state.run {
            Run::Completed | Run::Stopped | Run::AwaitingOperator { .. } => {
                // Best-effort tamper-evidence: anchor the journal head
                // in refs/forge/<run>. Gaps are reported, never fatal
                // (the referee-era anchor-gap lore).
                let mut gaps = Vec::new();
                if let Err(e) = crate::anchor::anchor(&self.store, &self.repo, &self.run_id) {
                    gaps.push(format!("anchor gap for {}: {e}", self.run_id));
                }
                // And the exhibits the journal cites, kept
                // reachable past the branch delete and the gc
                // that follow a landing (decision 0028). Same
                // shape of act as the anchor: derived entirely
                // from the journal, writing refs and never
                // branches, so it crosses into no authority the
                // operator keeps. Best-effort in the same way —
                // a ref-planting gap is reported, never fatal.
                gaps.extend(crate::keep_refs::plant_or_report(
                    &self.store,
                    &self.repo,
                    &self.run_id,
                ));
                return Ok(Some(DriveEnd { state, gaps }));
            }
            Run::Running(cursor) => {
                self.advance_running(&replay.events, cursor.clone(), state)?;
            }
        }
        self.replay = replay;
        Ok(None)
    }

    /// An engine that meets contention on the shared journal ends
    /// lawfully or not at all — it never dies of it.
    ///
    /// [`StoreError::Contended`] is the store saying it waited its whole
    /// patience for a peer's write lock and wrote nothing. That is an
    /// accident of timing, not a verdict: the same call made later is
    /// the same call. Every other error — including the fenced-append
    /// refusals, which are verdicts — leaves here exactly as it arrived.
    ///
    /// Where the fold admits a park, the contention is SAID, in the
    /// journal, in the run's own words: `run/parked` naming the lock it
    /// lost. `brokkr_core::fold` admits `run/parked` at exactly two
    /// cursors, so where it does not, this returns the typed contention
    /// instead of forging an event the fold would refuse — an engine
    /// that ends on contention must leave a journal that still folds.
    /// Either way the run is intact and `brokkr resume` picks it up:
    /// nothing was written, so nothing was lost.
    ///
    /// An attempt still open here has stopped — its seat has ended, and
    /// its settlement could not land within
    /// [`checkpoints::SETTLING_PATIENCES`] (#394). When the engine holds
    /// the attempt's terminal event, that real outcome is tried again,
    /// with the same patiences; once it lands the lock has let go, and
    /// `None` says the drive goes on. Otherwise the attempt is settled
    /// now, `effect/indeterminate` naming the lock and given the same
    /// patiences, so the park that follows is lawful and the next resume
    /// finds the attempt settled. A lock that outlasts this too leaves
    /// nothing writable: the contention is handed back with the attempt
    /// open, and the next resume settles it as restarted. A stop riding
    /// the attempt keeps its own ending, and the contention is handed
    /// back.
    fn lawful_end_under_contention(
        &mut self,
        error: EngineError,
    ) -> Result<Option<DriveEnd>, EngineError> {
        let Some(store_error) = error.contention() else {
            return Err(error);
        };
        let reason = format!("journal contention: {store_error}");
        let held = self.held_outcome.take();
        let mut state = self.replay.caught_up(&mut self.store, &self.run_id)?;
        if let Run::Running(Cursor::EffectInFlight {
            effect_id,
            attempt_id,
            ..
        }) = state.run
        {
            (self.current_cause, self.fence) = fence::tip_of(&self.replay.events);
            if let Some(held) = held.filter(|held| held.attempt_id.as_ref() == Some(&attempt_id)) {
                self.land_held_outcome(held, &effect_id)?;
                return Ok(None);
            }
            let settled =
                json!({"effect_id": effect_id, "attempt_id": attempt_id, "reason": reason});
            self.append(EventType::EffectIndeterminate, settled, Some(attempt_id))?;
            state = self.replay.caught_up(&mut self.store, &self.run_id)?;
        }
        if !matches!(
            state.run,
            Run::Running(Cursor::Park { .. } | Cursor::ExecuteEffect { .. })
        ) {
            return Err(error);
        }
        (self.current_cause, self.fence) = fence::tip_of(&self.replay.events);
        let parked = json!({"reason": reason, "evidence": {}});
        self.append(EventType::RunParked, parked, None)?;
        let state = self.replay.caught_up(&mut self.store, &self.run_id)?;
        Ok(Some(DriveEnd {
            state,
            gaps: Vec::new(),
        }))
    }

    /// Journal the attempt's held outcome. A result the seat-record fence
    /// refuses now settles the attempt indeterminate with the refusal
    /// named: the failure its seat would have built was not held, and a
    /// park loses nothing.
    fn land_held_outcome(
        &mut self,
        held: checkpoints::HeldOutcome,
        effect_id: &str,
    ) -> Result<(), EngineError> {
        let attempt_id = held.attempt_id.clone();
        match self.append_raw(held.event_type, held.payload, held.attempt_id) {
            Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                let reason = format!(
                    "the journal refused the result a peer's lock had held back: {refusal}"
                );
                let settled =
                    json!({"effect_id": effect_id, "attempt_id": attempt_id, "reason": reason});
                self.append_raw(EventType::EffectIndeterminate, settled, attempt_id)
            }
            landed => landed,
        }
        .map(drop)
    }

    fn advance_running(
        &mut self,
        events: &[EventEnvelope],
        cursor: Cursor,
        state: RunState,
    ) -> Result<(), EngineError> {
        match cursor {
            Cursor::Start => {
                let initial = self.bundle.machine.initial.clone();
                self.enter_phase(&initial, &state)?;
            }
            Cursor::EnterPhase { phase } => {
                self.enter_phase(&phase, &state)?;
            }
            Cursor::RequestEffect => self.request_or_finish(&state)?,
            Cursor::ExecuteEffect {
                effect_id,
                seat,
                failed_attempts,
            } => {
                let limits = self
                    .bundle
                    .seats
                    .get(&seat)
                    .map(|seat| seat.limits)
                    .unwrap_or_default();
                if failed_attempts >= limits.max_attempts {
                    // Bounded retry exhausted (decision 0006): park with the
                    // last recorded error.
                    let last_error = events
                        .iter()
                        .rev()
                        .find(|event| {
                            event.event_type == EventType::EffectFailed
                                && event.payload.get("effect_id").and_then(Value::as_str)
                                    == Some(effect_id.as_str())
                        })
                        .and_then(|event| event.payload.get("error").and_then(Value::as_str))
                        .unwrap_or("no error recorded")
                        .to_string();
                    self.append(
                        EventType::RunParked,
                        json!({
                            "reason": format!(
                                "effect {effect_id} failed {failed_attempts} of {} attempt(s); \
                                 last error: {last_error}",
                                limits.max_attempts
                            ),
                            "evidence": {},
                        }),
                        None,
                    )?;
                } else {
                    // `execute` is the one place a gate head is armed, for
                    // every body shape, and an attempt that leaves it
                    // through an `Err` — a checkpoint that would not
                    // journal, say — appends no terminal event and so
                    // never reaches the compare. The observation dies with
                    // the span that took it: a stale head here would be
                    // spent on the NEXT effect's terminal event, which
                    // could then read a park no gate earned.
                    let dispatched = self.execute(events, &state, &effect_id, &seat);
                    if dispatched.is_err() {
                        self.active_gate_head = None;
                    }
                    dispatched?
                }
            }
            // Fresh process, no live driver: recovered only where every
            // driver is proven settled, never re-paid (U8e2).
            Cursor::EffectInFlight {
                effect_id,
                attempt_id,
                ..
            } => self.restarted(&effect_id, &attempt_id)?,
            Cursor::Decide { result, .. } => self.decide(&state, result)?,
            Cursor::Park { reason } => {
                let evidence = match reason {
                    ParkReason::GateMovedHead => gate_head_evidence(events),
                    ParkReason::Stated(_) => json!({}),
                };
                self.append(
                    EventType::RunParked,
                    json!({"reason": reason.as_str(), "evidence": evidence}),
                    None,
                )?;
            }
            Cursor::Stop => {
                self.append(
                    EventType::RunStopped,
                    json!({"reason": operator_stop_reason(events)}),
                    None,
                )?;
            }
        }
        Ok(())
    }

    fn append(
        &mut self,
        event_type: EventType,
        payload: Value,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, EngineError> {
        if matches!(
            event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        ) {
            let effect_id = payload
                .get("effect_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if let Some(moved) = self.finish_gate_head_check(effect_id, attempt_id.clone())? {
                return Ok(moved);
            }
        }
        self.append_raw(event_type, payload, attempt_id)
    }

    fn finish_gate_head_check(
        &mut self,
        effect_id: &str,
        attempt_id: Option<String>,
    ) -> Result<Option<EventEnvelope>, EngineError> {
        let Some(start) = self.active_gate_head.take() else {
            return Ok(None);
        };
        let end = git_head(&self.repo);
        if start == end {
            return Ok(None);
        }
        let moved = MovedHead {
            head_at_end: end,
            head_at_start: start,
        };
        // EffectIndeterminate has only a reason string in the frozen event
        // contract. Keep the evidence packed there and attach its structured
        // copy to run/parked rather than widening that contract in this slice.
        self.append_raw(
            EventType::EffectIndeterminate,
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "reason": gate_moved_head::encode(&moved),
            }),
            attempt_id,
        )
        .map(Some)
    }

    fn request_or_finish(&mut self, state: &RunState) -> Result<(), EngineError> {
        let phase = phase_of(state, "RequestEffect with no phase")?;
        if self.bundle.machine.terminal.contains(&phase) {
            if phase == "stop" {
                let reason = state
                    .last_decision
                    .as_ref()
                    .and_then(|d| d.rule_id.present())
                    .map(|r| format!("hard stop ruled by {r}"))
                    .unwrap_or("stopped".to_string());
                self.append(EventType::RunStopped, json!({"reason": reason}), None)?;
            } else {
                self.append(EventType::RunCompleted, json!({}), None)?;
            }
            return Ok(());
        }
        let effect_id = Uuid::new_v4().to_string();
        let input = self.seat_input(state, &phase, &effect_id)?;
        let digest = brokkr_core::canonical::sha256_hex(&input);
        let seq = state.seq + 1;
        self.append(
            EventType::EffectRequested,
            json!({
                "effect_id": effect_id,
                "phase": phase,
                "seat": phase,
                "idempotency_key": format!("{}:{seq}", self.run_id),
                "input_digest": digest,
            }),
            None,
        )?;
        Ok(())
    }

    /// The repository this run drives, always spelled absolutely. Seat
    /// input is composed from it — `result_path` above all — and every
    /// seat script reads its result path by matching an absolute
    /// `/*.json` line, so `--repo .` in a recipe library, whose results
    /// stand under the operated repository, would compose
    /// `./.forge/results/<effect>.json` and the seat would refuse its own
    /// result path in a fraction of a second, before running a gate,
    /// with no transcript to say why. `box_argv` resolves the same
    /// relative prefix for the same reason. Symlinks stay as the host
    /// spells them; a path that cannot be resolved at all is threaded as
    /// written, which is what the driver would have received anyway.
    fn workdir(&self) -> PathBuf {
        std::path::absolute(&self.repo).unwrap_or_else(|_| self.repo.clone())
    }

    /// The operator-side store path threaded to drivers: the CLI
    /// override, or `secrets.env` in the run-local directory.
    fn secrets_store_path(&self) -> PathBuf {
        self.secrets_file
            .clone()
            .unwrap_or_else(|| self.run_local().join(crate::places::SECRETS))
    }

    /// The directory a run's results, scratch and secrets stand in
    /// (decision 0083 ruling 3): the folder's state, or `<workdir>/.forge`.
    fn run_local(&self) -> PathBuf {
        crate::places::run_local(self.state.as_deref(), &self.workdir())
    }

    /// The attempt's `started` payload gains the selection's provenance,
    /// where there is one, and which boundary stood at every site of this
    /// attempt (decision 0046 ruling 3), beside the provenance and built
    /// from the same walk, present exactly when a site of the attempt
    /// declares hands.
    fn record_sites(
        &self,
        started: &mut Value,
        provenance: Option<Value>,
        body: ExecutableBody<'_>,
        site_name: &str,
        gate: bool,
    ) {
        if let Some(provenance) = provenance {
            started["provenance"] = provenance;
        }
        if let Some(entries) = self.boundary_entries(body, site_name, gate) {
            started["boundary"] = entries;
        }
    }

    /// A ship seat reads the already-open journal. When that journal is
    /// outside the worktree (the ordinary same-realm fire), mount only its
    /// parent and mount it read-only. This run-time resource is deliberately
    /// absent from the manifest and the requested-input digest.
    /// The hands and boundary markers (see [`SiteMarks`]).
    fn mark_hands(&self, label: &str, input: &mut Value) {
        self.marks().hands(label, input);
    }

    /// Decision 0065 rulings 4 and 5: what the serving candidate of this
    /// site holds, and the native controls its launch is composed with —
    /// both read from the ONE outcome compiled for that candidate, so the
    /// prompt, the argv and the manifest cannot disagree. A fallback link
    /// gets its own outcome, never its primary's.
    ///
    /// `native_controls` is ALWAYS written: the plan, or `null` where no
    /// outcome was computed for the site — which the model adapters refuse
    /// before any provider work rather than launching a harness on its
    /// own defaults. Written beside the result door ([`SiteMarks`]), outside
    /// the requested digest, for the same reason: a chain fallback moves it.
    ///
    /// `launch_arguments` rides beside them (decision 0066 ruling 4): the
    /// composed spawn's arguments in their two parts, what the recipe or
    /// its agent authored and what the engine appended for the boundary.
    /// All three are written LAST, after every merge of seat, context and
    /// member input, so nothing a recipe, a result or a returned capability
    /// response carries can mint or overwrite them; a fallback link brings
    /// its own plan AND its own parts.
    /// A panel or a sequence is no launch of its own — each member and step
    /// is marked with its own spawn — so its seat-level input carries none.
    ///
    /// The private launch record is sealed onto the spawn here and written
    /// beside them (decision 0065 slice one, design D5.7): the spawn's
    /// segments with the serving outcome's expected state. An input that
    /// already carries one refuses the spawn rather than being overwritten
    /// into looking sealed; a site no outcome serves carries none; and a
    /// site whose expected state cannot be sealed refuses its spawn. The
    /// typed serving inputs are sealed with it and written beside it under
    /// the same three rules (rebuild unit 14a2), with the boundary this
    /// engine stands under (rebuild unit 14a4c). No plan is provisioned
    /// here, so a site that brokers a holding refuses its spawn; dispatch
    /// marks with its attempt's own brokers ([`Engine::mark_serving`]).
    pub fn mark_capabilities(
        &self,
        label: &str,
        link: Option<&Candidate>,
        spawn: Option<&mut SiteSpawn>,
        input: &mut Value,
    ) {
        self.mark_serving(
            label,
            link,
            (&[], self.hands_for(label).as_ref()),
            spawn,
            input,
        );
    }

    /// [`Engine::mark_capabilities`] with the brokers the attempt's
    /// provisioning sealed for this site (U7d2), its hands segment
    /// expanded beside them before the record is sealed
    /// ([`isolation::beside`]), and the reach sealed for its `hands`
    /// written beside the artifact root (U8a2c).
    fn mark_serving(
        &self,
        label: &str,
        link: Option<&Candidate>,
        (brokers, hands): (&[SealedBroker], Option<&HandsSpec>),
        spawn: Option<&mut SiteSpawn>,
        input: &mut Value,
    ) {
        let facts = self.bundle.sites.get(label);
        let outcome = facts
            .and_then(|facts| facts.capabilities.as_ref())
            .and_then(|site| {
                site.serving(link.map(|link| (link.provider.as_str(), link.model.as_str())))
            });
        self.marks().capabilities(label, link, input);
        let Some(spawn) = spawn else {
            input["launch_arguments"] = Value::Null;
            return;
        };
        input["launch_arguments"] = spawn.launch_arguments();
        if input.get(LAUNCH_RECORD).is_some() {
            spawn.refusal.get_or_insert_with(|| {
                format!(
                    "dispatch refused: the input arrived carrying a private launch record \
                     ('{LAUNCH_RECORD}') before the engine sealed one; a recipe, a result or a \
                     context cannot supply the record, even one equal to the engine's (decision \
                     0065 slice one, design D5.7)"
                )
            });
        }
        // Rebuild unit 14a2: the typed serving inputs are sealed beside the
        // record and arrive no other way.
        if input.get(SERVING_INPUTS).is_some() {
            spawn.refusal.get_or_insert_with(|| {
                format!(
                    "dispatch refused: the input arrived carrying sealed serving inputs \
                     ('{SERVING_INPUTS}') before the engine sealed any; a recipe, a result or a \
                     context cannot supply them, even ones equal to the engine's (rebuild unit \
                     14a2; design D5.7)"
                )
            });
        }
        spawn.record = None;
        spawn.serving = None;
        let Some(outcome) = outcome else {
            return;
        };
        let (workdir, artifacts) = (self.workdir(), artifact_root(&self.run_local()));
        let reach = hands.and_then(|spec| self.reach.sealed(spec));
        let sealed = expected_state(outcome, link, facts).and_then(|expected| {
            let site = (link, facts);
            let boxed = (&self.bundle.executable, &*workdir, &*artifacts, reach);
            let served = isolation::served(spawn, site, self.boundary, boxed, brokers);
            let serving = served.map_err(|unsealed| unsealed.to_string())?;
            spawn.seal(expected)?;
            spawn.serving = Some(serving);
            Ok(())
        });
        match sealed {
            Ok(()) => {
                input["launch_arguments"] = spawn.launch_arguments();
                input[ARTIFACT_ROOT] = json!(artifacts.to_string_lossy());
                input[SEALED_REACH] = serde_json::to_value(reach).unwrap_or_default();
                input[LAUNCH_RECORD] = spawn.launch_record();
                input[SERVING_INPUTS] = spawn
                    .serving
                    .as_ref()
                    .map_or(Value::Null, SealedServing::value);
            }
            Err(reason) => {
                spawn.refusal.get_or_insert(reason);
            }
        }
    }

    /// The marks this engine writes into a site's driver input, under the
    /// boundary this run composes with.
    fn marks(&self) -> SiteMarks<'_> {
        SiteMarks {
            bundle: &self.bundle,
            boundary: self.boundary,
        }
    }

    /// The hands spec this site's canonical family resolved, if any. An
    /// unresolved or resolved no-hands site answers `None`; only the site
    /// facts distinguish the two.
    fn hands_for(&self, label: &str) -> Option<brokkr_protocol::hands::HandsSpec> {
        self.bundle
            .sites
            .get(label)
            .and_then(|facts| facts.hands_spec())
            .cloned()
    }

    /// Every executing model site of this attempt, with its structural
    /// identity, its owner identity and the session — if any — it may
    /// rejoin (proposed decision 0056 rulings 1 and 2).
    ///
    /// Asked once, here, for the same reason the single seat's question
    /// was always asked here: the answer is a pure function of the
    /// journal, the pinned bundle and the local origin, all of which are
    /// settled before anything spawns. Composite dispatch then reads the
    /// answers rather than re-deriving them on a worker thread that
    /// holds no journal.
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    fn site_plans(
        &self,
        events: &[EventEnvelope],
        body: ExecutableBody<'_>,
        seat_name: &str,
        case: Option<&str>,
        site_name: &str,
        gate: bool,
        selection: &Selection,
        chains: &ChainIndexes,
        started: &Value,
        started_here: bool,
    ) -> BTreeMap<Site, SitePlan> {
        let holds = pinned_bundle_holds(events, &self.bundle);
        let workdir = self.workdir();
        let mut plans = BTreeMap::new();
        for entry in resume::structural_sites(body, seat_name, case, gate) {
            let label = site_label(site_name, &entry.site);
            let context = resume::SiteContext::new(
                entry.key.digest(),
                resume::InstanceKey::new(
                    selection.get(&entry.site),
                    chains.get(&entry.site).copied(),
                    argv_for(selection, &entry.site, &entry.command),
                    &self.bundle.manifest,
                    &label,
                    ENGINE_VERSION,
                    entry.class,
                    self.site_boundary(&label),
                )
                .digest(),
                entry.class,
            );
            let offer = offer_for_site(
                events,
                &entry.key,
                &context,
                seat_name,
                started_here,
                holds,
                started,
            );
            let originating = offer
                .as_ref()
                .and_then(|_| resume::originating_root(events, &context.site_ref));
            // An agent-resolved site carries its assessment on the selected
            // candidate; an inline driver-bearing site carries the adapter's
            // own, read at compile time where the adapters were opened
            // (proposed decision 0056 ruling 5). A site no adapter answers
            // for keeps the null the driver reads as unmeasured, never as
            // implicit support; the shipped inline Codex work seat is the
            // preserved rejoin and its adapter does answer.
            let assessment = selection
                .get(&entry.site)
                .map(|candidate| candidate.resume.value())
                .or_else(|| {
                    self.bundle
                        .sites
                        .get(&label)
                        .and_then(|facts| facts.inline_resume.clone())
                })
                .unwrap_or(Value::Null);
            // The route-overlay binding (design D6 mechanism 1; AS3): the
            // seat's single `--patch` value bound to the compiled leaf
            // layer, computed where the private context is built and
            // independent of the resume gate. A value that does not bind
            // yields no member; it never fails the start.
            let route_overlay = route_overlay_binding(
                &self.bundle,
                argv_for(selection, &entry.site, &entry.command),
                &workdir,
            );
            plans.insert(
                entry.site,
                SitePlan {
                    context: context.offered(events, offer.as_ref()),
                    offer,
                    assessment,
                    originating,
                    route_overlay,
                },
            );
        }
        plans
    }

    /// The boundary that stands at one site: the run's word for a site
    /// with hands, none for a site without — the `None` every record of
    /// such a site spells as `not applicable` (decision 0046 ruling 3).
    fn site_boundary(&self, label: &str) -> Option<Boundary> {
        self.marks().has_hands(label).then_some(self.boundary)
    }

    /// Which boundary stood at every invocation site of this attempt
    /// (decision 0046 ruling 3; design DD5): one entry per site of the
    /// same `invocation_sites` walk that builds `provenance`, keyed by the
    /// same tag, carrying the site's word or the sentinel and whether the
    /// site is gate class — read where the compiler keeps it: the seat's
    /// selected class for a single, a panel member or a selected case,
    /// and the step's class for a step and its panel's members. `None`
    /// when no site of the attempt declares hands, so a run over a
    /// bundle that boxes nothing journals byte-identical payloads.
    fn boundary_entries(
        &self,
        body: ExecutableBody<'_>,
        site_name: &str,
        gate: bool,
    ) -> Option<Value> {
        let step_gate = |tag: &str| -> bool {
            match body {
                ExecutableBody::Sequence { steps } => steps
                    .iter()
                    .find(|step| tag == step.name || tag.starts_with(&format!("{}:", step.name)))
                    .map_or(gate, |step| step.class == SeatClass::Gate),
                _ => gate,
            }
        };
        let mut any_hands = false;
        let entries: Vec<Value> = invocation_sites(body)
            .into_iter()
            .map(|(site, _)| {
                let label = site_label(site_name, &site);
                let boundary = self.site_boundary(&label);
                any_hands |= boundary.is_some();
                json!({
                    "member": site,
                    "boundary": Boundary::recorded(boundary),
                    "gate": site.as_deref().map_or(gate, step_gate),
                })
            })
            .collect();
        any_hands.then_some(Value::Array(entries))
    }

    /// Compose one site's spawn from the boundary this run stands under
    /// (decision 0046 ruling 4; design DD18): the pure [`compose_site`]
    /// over the facts the engine holds, with the unboxed exec dispatch's
    /// fixed environment and network prefix prepared here — the two
    /// private directories created under the run's scratch, the probe
    /// asked once per engine process and remembered. `label` names the
    /// compiled site whose facts the composition reads — an inline site's
    /// lowered allow among them (rebuild unit 5b).
    #[expect(clippy::too_many_arguments, reason = "decision 0065 slice one, #288")]
    fn compose_at(
        &mut self,
        label: Option<&str>,
        attempt_id: &str,
        gate: bool,
        command: Vec<String>,
        hands: Option<&HandsSpec>,
        link: Option<&Candidate>,
        result_path: &str,
    ) -> SiteSpawn {
        let boundary = match hands {
            None => BuiltBoundary::Open,
            Some(_) => built_boundary(self.boundary).expect("the entry fence admitted these hands"),
        };
        let workdir = self.workdir();
        let unboxed = match hands {
            Some(spec) if !self.boundary.is_boxed() && is_exec_dispatch(&command) => {
                Some(self.unboxed(attempt_id, spec))
            }
            _ => None,
        };
        let class = if gate {
            SeatClass::Gate
        } else {
            SeatClass::Work
        };
        let facts = label.and_then(|label| self.bundle.sites.get(label));
        // U8a (CR2): a box is handed the run's artifact root, made here
        // before launch; one that cannot be made refuses the spawn.
        let artifacts = artifact_root(&self.run_local());
        let boxed = boundary == BuiltBoundary::Namespace;
        let unrooted = boxed.then(|| make_root(&artifacts).err()).flatten();
        // U8a2c: and the reach its admission sealed for these hands.
        let sealed = hands.and_then(|spec| self.reach.sealed(spec));
        let mut spawn = SiteSpawn {
            // Rebuild unit 18: the site's own charter binding rides its
            // spawn to the dispatch door.
            charter: facts.and_then(|facts| facts.charter.clone()),
            ..compose_site_at(
                facts,
                boundary,
                class,
                command,
                hands,
                link,
                &workdir,
                (&self.bundle.executable, &artifacts, sealed),
                &self.bundle.roots,
                result_path,
                unboxed.as_ref(),
            )
        };
        spawn.refusal = unrooted
            .map(|unrooted| unrooted.to_string())
            .or(spawn.refusal);
        spawn
    }

    /// What an unboxed exec dispatch starts in (design DD10, DD15): the
    /// fixed environment over two private directories under the run's
    /// scratch, and the network prefix when the probe — run once per
    /// engine process, in that environment, on Linux only — passed.
    fn unboxed(&mut self, attempt_id: &str, spec: &HandsSpec) -> Unboxed {
        let scratch = self.run_local().join(SEAT_SCRATCH).join(attempt_id);
        let private_home = scratch.join("home");
        let private_tmp = scratch.join("tmp");
        std::fs::create_dir_all(&private_home).ok();
        std::fs::create_dir_all(&private_tmp).ok();
        let engine_env: BTreeMap<String, String> = std::env::vars().collect();
        let identity = brokkr_protocol::hands::git_facts(&self.workdir()).identity;
        let env = brokkr_protocol::hands::unboxed_environment(
            &engine_env,
            &brokkr_protocol::hands::home_dir(&engine_env),
            spec,
            &identity,
            &private_home,
            &private_tmp,
        );
        let (uid, gid) = brokkr_protocol::hands::ids();
        let passes = self.network_narrowing(&env, uid, gid);
        Unboxed {
            env,
            prefix: network_prefix_if(passes, uid, gid),
        }
    }

    /// Whether the network prefix stands here: asked of the kernel once
    /// per engine process, in the dispatch's environment, and remembered
    /// (design DD15). Linux only; nothing anywhere reports the answer.
    #[cfg(target_os = "linux")]
    fn network_narrowing(&mut self, env: &BTreeMap<String, String>, uid: u32, gid: u32) -> bool {
        *self
            .network_prefix
            .get_or_insert_with(|| brokkr_protocol::hands::probe_network_prefix(env, uid, gid))
    }

    /// Off every other host: no `unshare` to ask, no probe spawned.
    #[cfg(not(target_os = "linux"))]
    fn network_narrowing(&mut self, _env: &BTreeMap<String, String>, _uid: u32, _gid: u32) -> bool {
        false
    }

    /// Conclude a single-driver attempt with its terminal effect event.
    /// The stderr tail rides on failed/indeterminate outcomes; a spawn
    /// failure has no stderr and carries none.
    fn conclude_single(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        run: DriverRun,
        selection: &Selection,
        boundary: Option<Boundary>,
    ) -> Result<(), EngineError> {
        let report = match run {
            DriverRun::SpawnFailed(error) => {
                // A driver binary that is absent satisfies the structural
                // predicate trivially: nothing was accepted and nothing
                // checkpointed, because nothing ran.
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": error,
                });
                start_failure_fields(&mut payload, selection, vec![None]);
                self.append(
                    EventType::EffectFailed,
                    payload,
                    Some(attempt_id.to_string()),
                )?;
                return Ok(());
            }
            DriverRun::Ran(report) => report,
        };
        let start_failure = failed_to_start(&report);
        let stderr_tail = stderr_tail(&report.stderr);
        let unproven = Unproven::seat(&report);
        match report.settled_outcome() {
            AttemptOutcome::Succeeded { result } => {
                let result = stamp_boundary(result, boundary);
                self.append_succeeded(effect_id, attempt_id, result, |refusal| {
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": format!("{refusal}; stderr tail: {stderr_tail}"),
                    })
                })?;
            }
            AttemptOutcome::Failed { error } => {
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": format!("{error}; stderr tail: {stderr_tail}"),
                });
                if start_failure {
                    start_failure_fields(&mut payload, selection, vec![None]);
                }
                self.append(
                    EventType::EffectFailed,
                    payload,
                    Some(attempt_id.to_string()),
                )?;
            }
            AttemptOutcome::Indeterminate { reason } => {
                self.append(
                    EventType::EffectIndeterminate,
                    json!(Indeterminate {
                        effect_id,
                        attempt_id,
                        reason: format!("{reason}; stderr tail: {stderr_tail}"),
                        unproven,
                    }),
                    Some(attempt_id.to_string()),
                )?;
            }
        }
        Ok(())
    }

    /// Spawn one driver and run one attempt, journaling its live
    /// checkpoints as they stream — member-tagged when `member_tag`
    /// names a sequence step. `plan` is this site's resume plan: the
    /// session it may rejoin, offered to a driver that declares it can
    /// receive one (decision 0030, proposed 0056 ruling 4), and the two
    /// engine stamps every checkpoint of a planned site carries. Every
    /// structural site of a body is planned — an exec single or member
    /// included, whose rows name `not applicable` as their model and are
    /// stamped like any other and never offered anything, since no root
    /// is ever confirmed on them. `None` only for a dialect step, which
    /// spawns no driver and is stamped with neither. Appends NO terminal
    /// effect event: the caller owns the attempt's conclusion.
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    fn run_driver(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        driver_seat: &str,
        spawn: &SiteSpawn,
        mut input: Value,
        deadline: std::time::Duration,
        member_tag: Option<&str>,
        plan: Option<&SitePlan>,
    ) -> Result<DriverRun, EngineError> {
        let (workdir, begun) = (self.workdir(), self.begun(spawn)?);
        let boundary = self.site_boundary(driver_seat);
        let session_ref = plan
            .and_then(|plan| plan.offer.as_ref())
            .map(|offer| offer.provider_id.clone());
        // The private start context (design D5): which resume assessment
        // this site selected, the harness facts its offered root was
        // opened under, and the owned target (provider ID plus the
        // persistence locator and recorded home read off the same
        // confirmed checkpoint). It
        // rides the existing `Start.input` object under one key, separate
        // from the rendered `context` and the phase inputs, and never
        // reaches the prompt.
        if let Some(plan) = plan {
            input["resume_context"] = resume::start_context(
                plan.assessment.clone(),
                plan.originating.as_ref(),
                plan.offer.as_ref(),
                plan.route_overlay.as_ref(),
            );
        }
        let stamp = plan.map(|plan| plan.context.clone());
        // The door returns the very input the driver is sent, its verified charter text in it.
        let door = (self.held.as_ref()).map(|held| Door::of(held, driver_seat, begun.as_ref()));
        let (process, input) =
            match spawn_site(&self.bundle, spawn, &input, &workdir, deadline, door) {
                Err(e) => return Ok(DriverRun::SpawnFailed(format!("driver did not spawn: {e}"))),
                Ok(started) => started,
            };
        // A checkpoint the seat-record fence refuses (decision 0034, ruling
        // 6) does not stop the driver, which would only lose its stderr,
        // but the attempt is already lost: no later checkpoint is journaled,
        // and the refusal becomes its outcome once the process ends.
        let attempt = checkpoints::Attempt {
            run_id: &self.run_id,
            effect_id,
            attempt_id,
        };
        let facts = self.bundle.sites.get(driver_seat);
        let mut calls = Calls::of(facts, spawn, attempt_id, stamp.as_ref());
        let sink = Checkpoints::new(&mut self.store, &mut self.current_cause, attempt);
        let report = sink.drive(
            |forward| {
                process.run_attempt_resuming(
                    ENGINE_VERSION,
                    effect_id,
                    attempt_id,
                    driver_seat,
                    input,
                    session_ref,
                    forward,
                )
            },
            |sink, mut checkpoint| {
                let call = calls.consume(&mut checkpoint);
                let checkpoint = match member_tag {
                    None => checkpoint,
                    Some(tag) => tag_member(checkpoint, tag),
                };
                let checkpoint = stamp_boundary(checkpoint, boundary);
                // The engine's two structural stamps, on the same terms
                // as the boundary: a record that names a model carries
                // them — every row a shipped driver forwards does, the
                // launch row with its root included — a record that names
                // none carries neither, and a driver's value never
                // survives.
                let checkpoint = match &stamp {
                    Some(context) => context.stamp(checkpoint),
                    None => resume::unstamped(checkpoint),
                };
                sink.offer("", checkpoint, call);
            },
        )?;
        let report = self.folded([effect_id, attempt_id, driver_seat], spawn, begun, report)?;
        Ok(DriverRun::Ran(report))
    }

    /// Append the attempt's successful result — or, when the journal
    /// refuses that result under the seat-record fence (decision 0034,
    /// ruling 6), the failure the refusal is. The failed payload is the
    /// caller's to build, because each seat shape carries its own facts
    /// on a failed attempt (a stderr tail, the sites that failed to
    /// start); the closure receives the refusal's text and nothing else.
    fn append_succeeded(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        result: Value,
        failed: impl FnOnce(String) -> Value,
    ) -> Result<(), EngineError> {
        match self.append(
            EventType::EffectSucceeded,
            json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
            Some(attempt_id.to_string()),
        ) {
            Ok(_) => Ok(()),
            Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                self.append(
                    EventType::EffectFailed,
                    failed(refusal.to_string()),
                    Some(attempt_id.to_string()),
                )?;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

/// The rule id the engine cites when the operator's command — not the
/// policy table — is what takes a run to stop. All-caps hyphenated, the
/// same vocabulary the table's own rule ids use (`WORK`, `SHIP-COMPLETE`),
/// so a reader of `run/stopped` can tell the two causes apart at a glance
/// and grep for either.
pub const OPERATOR_STOP_RULE: &str = "OPERATOR-STOP";

/// A driver invocation that got as far as running, or one that never
/// spawned. Spawn failures carry no stderr, and their terminal error
/// grows no stderr tail — the distinction keeps event payloads exactly
/// as they were before this enum existed.
enum DriverRun {
    SpawnFailed(String),
    Ran(AttemptReport),
}

/// The invocation-site tag an agent-resolved site is journaled under:
/// `None` for a single seat, `<member>` for a panel member, `<step>` for
/// a sequence step, `<step>:<member>` for a member inside a step panel —
/// exactly the tags the engine already uses for checkpoints.
type Site = Option<String>;

/// The label the site `site` of the seat `site_name` is composed and
/// marked under: the seat's own, or `<seat>:<tag>`.
fn site_label(site_name: &str, site: &Site) -> String {
    site.as_ref()
        .map_or_else(|| site_name.to_string(), |tag| format!("{site_name}:{tag}"))
}

/// The candidate chosen for each agent-resolved invocation site of this
/// attempt, derived from journaled facts BEFORE anything spawns. Inline
/// sites are absent from this map, which is what keeps their execute
/// path exactly as it was.
/// Everything this attempt decided per site before anything spawned: the
/// candidate each agent-resolved site runs, and — for each structural
/// site the body walk yields — the resume plan proposed decision 0056
/// gives it.
///
/// One value rather than two because they are one decision, taken at one
/// moment from one journal, and every composite dispatch that needs the
/// candidate needs the plan beside it. Inline sites are absent from
/// `candidates`, which is what keeps their execute path exactly as it
/// was. Only dialect steps are absent from `plans`: they spawn no driver
/// and so carry no stamp and receive no offer. An exec single or member
/// IS planned — its rows name `not applicable` as their model and are
/// stamped like a model site's — and is never offered anything, because
/// no exec row ever carries a confirmed root.
#[derive(Debug, Default)]
struct Selection {
    candidates: BTreeMap<Site, Candidate>,
    plans: BTreeMap<Site, SitePlan>,
    /// The brokers provisioning sealed, by site label (U7d2).
    brokers: BTreeMap<String, Vec<SealedBroker>>,
}

impl Selection {
    fn new() -> Selection {
        Selection::default()
    }

    fn get(&self, site: &Site) -> Option<&Candidate> {
        self.candidates.get(site)
    }

    fn insert(&mut self, site: Site, candidate: Candidate) {
        self.candidates.insert(site, candidate);
    }

    fn contains_key(&self, site: &Site) -> bool {
        self.candidates.contains_key(site)
    }

    /// This site's resume plan, or `None` for a site that has no model
    /// turn to plan for.
    fn plan(&self, site: &Site) -> Option<&SitePlan> {
        self.plans.get(site)
    }
}

/// How far along its chain each agent-resolved site walked for this
/// attempt, beside the candidate that choice produced. Part of the
/// site's owner identity: a chain fallback is a different instance, and
/// the session the first candidate opened is not the second's to rejoin.
type ChainIndexes = BTreeMap<Site, usize>;

/// One executing model site's resume plan, decided before anything
/// spawns: its two engine stamps, the session it may rejoin, and the
/// harness version that session was opened under.
#[derive(Debug)]
struct SitePlan {
    context: resume::SiteContext,
    /// The eligible session as an owned target: the provider ID that
    /// crosses `Body::Resume` plus the persistence locator recorded on
    /// the same confirmed checkpoint (design D6).
    offer: Option<resume::ResumeTarget>,
    /// The selected adapter's typed resume assessment for this site
    /// (design D5), carried into the driver's private start context.
    /// `Value::Null` where no adapter answers for the site — an inline
    /// command whose driver no declaration names — which the adapter
    /// reads as unmeasured, never as implicit support. An inline
    /// built-in model driver carries the assessment its adapter
    /// declares, so the preserved shipping Codex work seat is judged
    /// exactly as an agent-resolved one.
    assessment: Value,
    /// The harness facts (version and optional wrapper digest) the offered
    /// root was opened under, read off the same row the offer came from so
    /// the adapter can refuse a rejoin whose CLI or composite has moved
    /// underneath it (proposed decision 0056 ruling 5). Absent when no
    /// offer is made.
    originating: Option<resume::OriginatingRoot>,
    /// The engine's binding of this site's single `--patch` value to the
    /// compiled leaf layer (design D6 mechanism 1; AS3). `None` when the
    /// argv carries no `--patch` or the value does not bind; the adapter
    /// refuses a present `--patch` beside an absent binding.
    route_overlay: Option<resume::RouteOverlay>,
}

/// Every invocation site of a seat body, with the site's fallback chain.
/// Inline sites carry an empty chain.
fn invocation_sites(body: ExecutableBody<'_>) -> Vec<(Site, &[Candidate])> {
    match body {
        ExecutableBody::Invocation(invocation) => invocation.chains(None),
        ExecutableBody::Sequence { steps } => steps
            .iter()
            .flat_map(|step| match &step.body {
                StepBody::Invocation(invocation) => invocation.chains(Some(&step.name)),
                StepBody::Dialect { .. } => {
                    vec![(Some(step.name.clone()), &[] as &[Candidate])]
                }
            })
            .collect(),
    }
}

/// How far along its chain a site has walked: the number of PRIOR
/// attempts for this effect whose terminal event recorded a fail-to-start
/// for this site, clamped to the last candidate.
///
/// Derived by scanning the effect's own events, the way `advance_running`
/// already scans them for the last error — so `fold` and `RunState` are
/// untouched, and a crash between attempts cannot change which model runs
/// next.
fn chain_index(events: &[EventEnvelope], effect_id: &str, site: &Site, chain: usize) -> usize {
    let failures = events
        .iter()
        .filter(|event| {
            event.event_type == EventType::EffectFailed
                && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
        })
        .filter(|event| {
            event
                .payload
                .get("start_failure_sites")
                .and_then(Value::as_array)
                .is_some_and(|sites| sites.iter().any(|entry| site_matches(entry, site)))
        })
        .count();
    failures.min(chain.saturating_sub(1))
}

fn site_matches(entry: &Value, site: &Site) -> bool {
    match site {
        None => entry.is_null(),
        Some(tag) => entry.as_str() == Some(tag.as_str()),
    }
}

/// The per-site selection for this attempt, plus the provenance list the
/// `effect/started` payload carries. `None` when no site is
/// agent-resolved: a non-adopting run's journal gains no field at all.
fn select_candidates(
    events: &[EventEnvelope],
    effect_id: &str,
    body: ExecutableBody<'_>,
) -> (Selection, Option<Value>, ChainIndexes) {
    let mut selection = Selection::new();
    let mut chains = ChainIndexes::new();
    let mut provenance = Vec::new();
    for (site, chain) in invocation_sites(body) {
        if chain.is_empty() {
            continue;
        }
        let index = chain_index(events, effect_id, &site, chain.len());
        let candidate = chain[index].clone();
        provenance.push(json!({
            "member": site,
            "agent": candidate.agent,
            "model": candidate.model,
            // The other half of the hire, journaled beside the model it
            // was pinned with (decision 0035 ruling 5). Additive, like
            // `model` before it: a candidate an effortless provider
            // serves carries `null`, and a run journaled before this
            // decision carries no key at all — which is exactly what
            // ruling 6 wants the view to show as absent.
            "effort": candidate.effort,
            "provider": candidate.provider,
            "chain_index": index,
        }));
        chains.insert(site.clone(), index);
        selection.insert(site, candidate);
    }
    let provenance = (!provenance.is_empty()).then_some(Value::Array(provenance));
    (selection, provenance, chains)
}

/// Is the bundle this attempt runs under the one the run pinned at its
/// first event? The pin carries the engine version, every bundle file's
/// digest, each referenced charter and agent definition, and the adapter
/// declaration digest that authorised each driver — so this single
/// comparison is decision 0030 ruling 4's "an adapter edit, or an engine
/// upgrade between attempts spawns cold", asked of the journal rather
/// than argued from `Engine::resume` refusing the same thing one layer
/// out. Two doors, one answer, and this one is on the path that hands
/// over a session handle.
fn pinned_bundle_holds(events: &[EventEnvelope], bundle: &Bundle) -> bool {
    events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted)
        .and_then(|event| event.payload.get("manifest"))
        .and_then(|manifest| bundle_manifest_from_run(manifest).ok())
        .is_some_and(|pinned| pinned == bundle.manifest)
}

/// The session one site may rejoin, from this run's durable evidence.
///
/// A free function rather than a method because it reads nothing but its
/// arguments: the same journal, the same key and the same origin answer
/// the same way in any process, which is what makes an operator's retry
/// after a park derive the offer its predecessor would have (proposed
/// decision 0056 ruling 2). `admitted` is the engine's two gross checks
/// — the run started on this installation and the pinned bundle still
/// holds — passed in already answered because only the engine can ask
/// them.
///
/// The legacy comparison it hands down is decision 0030's own: a
/// checkpoint with no engine stamp predates proposed 0056, so the only
/// ownership evidence it has is the `driver` label and the `provenance`
/// of the attempt that wrote it. New evidence uses `instance_ref`
/// instead, which a SIBLING member's chain movement does not disturb.
fn offer_for_site(
    events: &[EventEnvelope],
    key: &resume::SiteKey,
    context: &resume::SiteContext,
    seat: &str,
    started_here: bool,
    pinned_bundle_holds: bool,
    started: &Value,
) -> Option<resume::ResumeTarget> {
    resume::eligible_offer(
        events,
        key,
        context,
        seat,
        started_here,
        pinned_bundle_holds,
        |event: &EventEnvelope| {
            event
                .attempt_id
                .as_deref()
                .and_then(|attempt| {
                    events
                        .iter()
                        .rev()
                        .filter(|e| e.event_type == EventType::EffectStarted)
                        .find(|e| e.attempt_id.as_deref() == Some(attempt))
                })
                .is_some_and(|opened| resume::legacy_instance_holds(&opened.payload, started))
        },
    )
}

/// The argv to spawn for one invocation site: the selected candidate's,
/// or the compiled command when the site is inline.
fn argv_for<'a>(selection: &'a Selection, site: &Site, inline: &'a [String]) -> &'a [String] {
    match selection.get(site) {
        Some(candidate) => &candidate.argv,
        None => inline,
    }
}

/// The seat's single `--patch` value, or `None`. One exact `--patch` with
/// a non-empty, non-flag value is the only shape; a second `--patch`, a
/// bare `--patch`, or any other `--patch…` spelling (an `=`-joined value
/// or a longer token) is ambiguous and yields no value. The engine's
/// outcome for every non-binding value is withholding the binding, never
/// failing the start (design D6 mechanism 1; AS3).
fn single_patch_value(command: &[String]) -> Option<&str> {
    let mut found: Option<&str> = None;
    let mut index = 0;
    while index < command.len() {
        let part = command[index].as_str();
        if part == "--patch" {
            if found.is_some() {
                return None;
            }
            let value = command.get(index + 1)?;
            if value.is_empty() || value.starts_with("--") {
                return None;
            }
            found = Some(value);
            index += 2;
            continue;
        }
        if part.starts_with("--patch") {
            return None;
        }
        index += 1;
    }
    found
}

/// Bind one seat's single `--patch` value to the compiled leaf layer
/// (design D6 mechanism 1; AS3). The value resolves relative to the
/// run's working directory, without an absolute path or `..` component
/// or symlink escape, to a regular file inside the compiled bundle's own
/// layer directory that is a `files` member of the compiled manifest;
/// the binding carries the argv value and that member's recorded digest.
/// A value that does not bind yields no member — an ancestor layer's
/// file, a same-shaped file outside the layer, a working-directory
/// shadow and the bundle-relative `./` expansion (already absolute) all
/// take this same outcome, and the adapter's pre-work failure to start is
/// the single refusal.
fn route_overlay_binding(
    bundle: &Bundle,
    command: &[String],
    workdir: &Path,
) -> Option<resume::RouteOverlay> {
    let value = single_patch_value(command)?;
    let relative = Path::new(value);
    if relative.is_absolute() {
        return None;
    }
    if relative
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    // Canonicalize both sides: an in-layer symlink whose target resolves
    // outside the layer is not a binding, and a working-directory shadow
    // canonicalizes outside the compiled layer too.
    let resolved = std::fs::canonicalize(workdir.join(relative)).ok()?;
    let layer = std::fs::canonicalize(&bundle.dir).ok()?;
    if !resolved.is_file() {
        return None;
    }
    let inside = resolved.strip_prefix(&layer).ok()?;
    // Manifest keys are component-joined with `/` on every platform
    // (bundle.rs's `walk_files`), so spell the looked-up key the same way.
    let key = inside
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?
        .join("/");
    let digest = bundle.manifest.get("files")?.get(key.as_str())?.as_str()?;
    Some(resume::RouteOverlay {
        value: value.to_string(),
        digest: digest.to_string(),
    })
}

/// The STRUCTURAL fail-to-start predicate (decision 0016): `Failed`, and
/// no `Accepted` ever received, and no checkpoint emitted. No stderr
/// sniffing and no "model not found" regex — the engine pattern-matching
/// a provider's prose to make a control decision is the control-plane
/// repair decision 0001 forbids.
///
/// Once `Accepted` arrives, this is false and fallback is unreachable by
/// construction: decision 0016's mid-session boundary is mechanised here
/// rather than described in a comment. A seat that ran for forty turns
/// and then hit a quota wall has produced work a different model does
/// not inherit, and it follows 0006 unchanged.
///
/// The fourth fact is the engine's own (decision 0053 ruling 5): an
/// attempt this watchdog KILLED is `Failed` because the kill made
/// non-completion determinate, not because anything refused to start.
/// The driver died with no chance to say which side of the boundary it
/// was on, so the ambiguity decision 0003 parks on holds — and without
/// this term a vendor that hangs a first turn would walk the chain down
/// every link, each one hanging for a full deadline, and journal
/// per-model start failures for one vendor-wide stall.
///
/// It reads the settled outcome (#403): an attempt whose tree is not
/// proven over is indeterminate, and no fallback starts beside it.
fn failed_to_start(report: &AttemptReport) -> bool {
    matches!(report.settled_outcome(), AttemptOutcome::Failed { .. })
        && !report.accepted
        && report.checkpoints.is_empty()
        && !report.deadline_killed
}

/// The fail-to-start fields for a terminal `effect/failed` payload —
/// absent entirely when no agent-resolved site failed to start, so an
/// inline seat's journal is byte-identical to what it always was.
fn start_failure_fields(payload: &mut Value, selection: &Selection, sites: Vec<Site>) {
    let journaled: Vec<Value> = sites
        .into_iter()
        .filter(|site| selection.contains_key(site))
        .map(|site| match site {
            None => Value::Null,
            Some(tag) => Value::String(tag),
        })
        .collect();
    if journaled.is_empty() {
        return;
    }
    payload["start_failure"] = Value::Bool(true);
    payload["start_failure_sites"] = Value::Array(journaled);
}

/// One composed invocation (decision 0046 ruling 4; design DD18): the
/// argv, the environment it starts in, and — for an exec dispatch, boxed
/// or not (#832) — the script directory the engine re-walks before the spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteSpawn {
    pub argv: Vec<String>,
    pub env: SpawnEnv,
    pub rewalk: Option<PathBuf>,
    pub refusal: Option<String>,
    /// Who supplied each token of `argv`, in order (decision 0065 slice
    /// one, design D5.7): the selected candidate's own segments, or an
    /// inline site's authored command, and what the engine composed for the
    /// boundary as `hands`. Carried from where each contribution is made,
    /// because the flattened argv has lost the difference and an author can
    /// spell whatever the engine can. Its concatenation is `argv`.
    pub segments: Vec<Segment>,
    /// The private launch record sealed for this spawn — its driver
    /// extras' segments beside the serving candidate's expected state —
    /// or `None` where no capability outcome serves the site. The dispatch
    /// door admits exactly this record and nothing else.
    pub record: Option<LaunchRecord>,
    /// The typed serving inputs sealed beside `record` (rebuild unit
    /// 14a2): what the final check rebuilds the command from, carried from
    /// the selected candidate's composition or the inline site's facts.
    /// `Some` exactly where `record` is, and the dispatch door admits
    /// exactly these inputs and nothing else.
    pub serving: Option<SealedServing>,
    /// The seat class this spawn was composed for, which selects the
    /// boundary fragment appended behind hands and so the one the serving
    /// inputs seal (rebuild unit 14a2); `None` for a spawn no composition
    /// classed.
    pub class: Option<SeatClass>,
    /// Rebuild unit 18 (design D7): the charter the compile bound to the
    /// site this spawn was composed for — its owner, reference, target and
    /// digest — carried from that site's facts so the dispatch door checks
    /// this site's own binding and no other. `None` for a spawn no compiled
    /// site owns, or a site without a charter.
    pub charter: Option<CharterPin>,
}

/// The engine-private input key the sealed launch record rides under,
/// written after every merge of seat, context and member input.
pub const LAUNCH_RECORD: &str = "launch_record";

impl SiteSpawn {
    /// The author's argv in the engine's own environment, no re-walk: what a site that
    /// is no exec dispatch spawns as, boxed (#832) or without hands (#823).
    pub fn inherit(argv: Vec<String>) -> SiteSpawn {
        SiteSpawn::of(vec![Segment::new(Origin::Authored, &argv)])
    }

    /// A spawn of exactly these segments, their concatenation its argv.
    fn of(segments: Vec<Segment>) -> SiteSpawn {
        SiteSpawn {
            argv: flatten(&segments),
            env: SpawnEnv::Inherit,
            rewalk: None,
            refusal: None,
            segments,
            record: None,
            serving: None,
            class: None,
            charter: None,
        }
    }

    /// Where the driver's extras begin, exactly as the driver will read
    /// them. The driver's launch starts behind whatever the engine put in
    /// front of it for the boundary — the box's exec prefix, the network
    /// prefix — which composition carries as the leading `hands` segments,
    /// never found by searching the flattened argv. Its extras follow the
    /// three-token `<engine> driver <kind>` verb, less an escape `--`
    /// directly behind the verb, which the driver's trailing-argument
    /// parser drops; any later `--` is an argument. An argv that ends at
    /// the verb has none.
    fn extras_start(&self) -> usize {
        let launch: usize = self
            .segments
            .iter()
            .take_while(|segment| segment.origin == Origin::Hands)
            .map(|segment| segment.argv.len())
            .sum();
        let verb = launch + 3;
        match self.argv.get(verb) {
            None => self.argv.len(),
            Some(escape) if escape == "--" => verb + 1,
            Some(_) => verb,
        }
    }

    /// The segments of the driver's extras, cut at [`Self::extras_start`]
    /// by position: a segment that ends at or before the cut supplied only
    /// the verb and is dropped, one that straddles it keeps its tail, and
    /// every later one — an empty one included — is kept whole.
    pub fn extras(&self) -> Vec<Segment> {
        let cut = self.extras_start();
        let mut start = 0;
        let mut extras = Vec::new();
        for segment in &self.segments {
            let end = start + segment.argv.len();
            if start >= cut {
                extras.push(segment.clone());
            } else if end > cut {
                extras.push(Segment::new(segment.origin, &segment.argv[cut - start..]));
            }
            start = end;
        }
        extras
    }

    /// The driver's private `launch_arguments`: the extras in their two
    /// legacy parts, the trailing run of `hands` segments the engine
    /// composed for the boundary and everything before it. A projection of
    /// the segments, never a count recovered from the flattened argv. The
    /// driver refuses a launch whose parts do not reassemble what it was
    /// handed.
    pub fn launch_arguments(&self) -> Value {
        let extras = self.extras();
        let managed_from = extras
            .iter()
            .rposition(|segment| segment.origin != Origin::Hands)
            .map_or(0, |last| last + 1);
        json!({
            "authored": flatten(&extras[..managed_from]),
            "managed": flatten(&extras[managed_from..]),
        })
    }

    /// Seal this spawn's private launch record: its extras' segments
    /// beside the expected state the serving outcome was resolved to.
    ///
    /// Rebuild unit 5c-fix (operator ruling of 2026-09-24, item 2): the
    /// permission template the spawn emits must be exactly the one the
    /// expected state records, or nothing is sealed. A template omitted,
    /// altered, added or relabelled on its way into the command is refused
    /// here, not copied into the record as if it were the engine's.
    pub fn seal(&mut self, expected: Expected) -> Result<(), String> {
        let recorded: &[String] = match &expected.template {
            TemplateExpectation::None => &[],
            TemplateExpectation::Declared(argv) => argv,
        };
        if self.emitted_template() != recorded {
            self.record = None;
            return Err(
                "dispatch refused: the permission template this spawn emits is not the one its \
                 expected state records from the adapter's declaration; a template omitted, \
                 altered or added on its way into the command is never sealed as the engine's \
                 (operator ruling of 2026-09-24, the permission template at inline sites; \
                 rebuild unit 5c-fix)"
                    .to_string(),
            );
        }
        if let Err(reason) = self.local_sandbox_agrees(&expected) {
            self.record = None;
            return Err(reason);
        }
        self.record = Some(LaunchRecord {
            segments: self.extras(),
            expected,
        });
        Ok(())
    }

    /// Rebuild unit 5d (operator ruling 2 of 2026-09-23; ruling of
    /// 2026-09-25): the engine's own `local` segments are parsed back under
    /// the harness's grammar, and the sandbox class they express must be
    /// exactly the one the expected state records for a local emission —
    /// the site's typed class where no hands carry it, and none where hands
    /// do, because an agent's class rides its hands fragment (design D5.3)
    /// and a site with hands and an inline class is refused at compile. A
    /// harness whose grammar models no `--sandbox` option — or that has no
    /// modelled grammar — has no option to express a class, so it expresses
    /// none, and a recorded class there is refused.
    fn local_sandbox_agrees(&self, expected: &Expected) -> Result<(), String> {
        let recorded = local_class(expected);
        let local = self.extras_of(Origin::Local);
        let grammar = brokkr_protocol::native_controls::grammar::grammar(
            &expected.identity.harness,
        )
        .filter(|grammar| {
            grammar
                .options
                .iter()
                .any(|spec| spec.canonical == "--sandbox")
        });
        let emitted = match grammar.map(|grammar| (grammar, grammar.parse(&local))) {
            None => Some(SandboxIntent::Unspecified),
            // The grammar admits `--sandbox` once, with one value: a second
            // is a parse problem, refused below, never a class to pick from.
            Some((_, Ok(command))) => match command
                .nodes
                .iter()
                .find(|node| node.name() == "--sandbox")
                .and_then(|node| node.values.first())
            {
                None => Some(SandboxIntent::Unspecified),
                Some(class) => {
                    crate::agents::Sandbox::parse(class).map(crate::agents::Sandbox::intent)
                }
            },
            Some((grammar, Err(problem))) => {
                return Err(format!(
                    "dispatch refused: the engine's own `local` segments of this spawn cannot be \
                     read under the '{}' grammar (argument {}: it {}), so the sandbox class they \
                     express cannot be checked against its expected state; an unreadable \
                     contribution is never sealed as the engine's (operator ruling 2 of \
                     2026-09-23; rebuild unit 5d)",
                    grammar.harness,
                    problem.at + 1,
                    problem.cause
                ))
            }
        };
        // The whole launch, the native plan the input hands the driver
        // included, is judged at the dispatch door (`verify_record`;
        // rebuild unit 5d-fix-b).
        match emitted == Some(recorded) {
            true => Ok(()),
            false => Err(
                "dispatch refused: the sandbox class this spawn's own `local` segments express \
                 is not the one its expected state records from the site's typed declaration; a \
                 class omitted, altered or added on its way into the command is never sealed as \
                 the engine's (operator ruling 2 of 2026-09-23; ruling of 2026-09-25, inline \
                 Codex sandbox classes; rebuild unit 5d)"
                    .to_string(),
            ),
        }
    }

    /// The permission template this spawn emits behind its driver verb, by
    /// carried origin and never by its bytes. Where an agent's driver
    /// template supplies the verb — the first segment behind the engine's
    /// boundary prefix — it is that segment's tail behind the verb, followed
    /// by every later `template` segment that is not a model or effort pin
    /// ([`pin_fault`]; rebuild unit 5c-fix-b): a pin is template-origin but
    /// is not the permission template, and anything else of that origin is
    /// emitted as one. Where the author supplies the verb — an inline site —
    /// every `template` segment of the extras is the engine's permission
    /// template, in order.
    fn emitted_template(&self) -> Vec<String> {
        let verb = self
            .segments
            .iter()
            .position(|segment| segment.origin != Origin::Hands);
        match verb.map(|at| (&self.segments[at], &self.segments[at + 1..])) {
            Some((driver, later)) if driver.origin == Origin::Template => {
                crate::agents::permission_template(&driver.argv)
                    .iter()
                    .chain(
                        later
                            .iter()
                            .filter(|segment| {
                                segment.origin == Origin::Template
                                    && pin_fault(&driver.argv, &segment.argv).is_some()
                            })
                            .flat_map(|segment| segment.argv.iter()),
                    )
                    .cloned()
                    .collect()
            }
            _ => self.extras_of(Origin::Template),
        }
    }

    /// The arguments of every segment of the driver's extras `origin`
    /// supplied, in order.
    fn extras_of(&self, origin: Origin) -> Vec<String> {
        let extras = self.extras().into_iter();
        let extras: Vec<_> = extras.filter(|segment| segment.origin == origin).collect();
        flatten(&extras)
    }

    /// The sealed record as the driver input carries it, `null` where none.
    pub fn launch_record(&self) -> Value {
        self.record
            .as_ref()
            .map_or(Value::Null, LaunchRecord::value)
    }
}

/// The dispatch door's judgment of the launch record an input carries
/// (decision 0065 slice one, design D5.7): exactly the record sealed for
/// this spawn, decoded strictly and reassembling the extras the driver is
/// about to be handed. A record where none was sealed, a missing or
/// malformed one, one whose segments were reordered or relabelled, and one
/// whose argv no longer reassembles each refuse before any provider work.
/// Equal bytes prove nothing here: the record is compared with the one the
/// engine sealed, not recognised by its contents.
pub fn verify_record(spawn: &SiteSpawn, input: &Value) -> Result<(), String> {
    let handed = input.get(LAUNCH_RECORD);
    let Some(sealed) = &spawn.record else {
        return match handed {
            None => verify_serving(spawn, input),
            Some(_) => Err(format!(
                "dispatch refused: the input carries a private launch record ('{LAUNCH_RECORD}') \
                 the engine sealed no record for, and a record is never accepted from anything \
                 but the dispatch that sealed it (decision 0065 slice one, design D5.7)"
            )),
        };
    };
    let record = LaunchRecord::decode(handed)?;
    if &record != sealed {
        return Err(
            "dispatch refused: the private launch record handed over is not the one the engine \
             sealed for this spawn; a record whose segments, origins or expected state differ is \
             never trusted by its shape (decision 0065 slice one, design D5.7)"
                .to_string(),
        );
    }
    reassemble(&record.segments, &spawn.argv[spawn.extras_start()..])?;
    verify_serving(spawn, input)?;
    inline_codex_door(&record, input)
}

/// The dispatch door's judgment of the serving inputs an input carries
/// (rebuild unit 14a2): exactly the inputs sealed beside this spawn's
/// record, decoded strictly — or none, where none were sealed. Inputs
/// handed where none were sealed, missing or malformed ones, and ones that
/// decode but differ from the sealed each refuse before any provider work.
fn verify_serving(spawn: &SiteSpawn, input: &Value) -> Result<(), String> {
    let handed = input.get(SERVING_INPUTS);
    let Some(sealed) = &spawn.serving else {
        return match handed {
            None => Ok(()),
            Some(_) => Err(format!(
                "dispatch refused: the input carries sealed serving inputs ('{SERVING_INPUTS}') \
                 the engine sealed none for, and they are never accepted from anything but the \
                 dispatch that sealed them (rebuild unit 14a2; design D5.7)"
            )),
        };
    };
    if &SealedServing::decode(handed)? != sealed {
        return Err(
            "dispatch refused: the serving inputs handed over are not the ones the engine sealed \
             beside this spawn's record; a dialect, pin or hands declaration that differs is never \
             trusted by its shape (rebuild unit 14a2; design D5.7, D6)"
                .to_string(),
        );
    }
    Ok(())
}

/// The composition a candidate carries, or none for one that never
/// composed: read once, for the expected state, the serving inputs and the
/// compile's pin check alike (unit 26c).
pub(crate) fn composed(link: &Candidate) -> Option<&crate::agents::Composition> {
    match &link.lowering {
        Lowering::Composed(composition) => Some(composition),
        Lowering::Unavailable | Lowering::Refused(_) => None,
    }
}

/// Rebuild unit 5d-fix-b (chief F1, F2, F4 and F5; decision 0046 ruling 4;
/// operator ruling 2 of 2026-09-23 and the ruling of 2026-09-25): where the
/// sealed class is the engine's own `local` emission, the whole launch the
/// driver is handed — the sealed extras and, last, the native plan the
/// input carries, as composition appends it — is judged by the same
/// [`judge_inline_codex_launch`] admission ran (rebuild unit 5d-fix-c1). The
/// door and the capture follow the admitted class, never the recorded door
/// alone: a `read-only` class is a gate's, so the input must name the
/// last-message door and the launch must capture into exactly the result
/// path the input hands over, and a `workspace-write` class is a work
/// seat's, which names no such door and captures nothing.
///
/// Rebuild unit 5d-fix-c1 (chief F1, F3 and F4 of run
/// `0065-rebuild-unit-5d-fix-b-see-t-8067eebc`): where the sealed
/// expectation denies a native power, the engine's plan is required — a
/// missing `native_controls` key or a plan with no argv refuses — and the
/// delivered launch must itself express each sealed denial, read by the
/// grammar ([`inline_codex_denials`]), never taken from the plan's own
/// claim. An unreadable plan refuses with a fixed cause: the decoder's text
/// can carry the plan's own strings and is never echoed. The seat is named
/// in the one bounded representation admission uses.
///
/// [`judge_inline_codex_launch`]: brokkr_protocol::native_controls::grammar::judge_inline_codex_launch
/// [`inline_codex_denials`]: brokkr_protocol::native_controls::grammar::inline_codex_denials
fn inline_codex_door(record: &LaunchRecord, input: &Value) -> Result<(), String> {
    use crate::agents::Sandbox;
    use brokkr_protocol::native_controls::{grammar, NativeExpectation};
    let intent = local_class(&record.expected);
    let Some(class) = [
        Sandbox::ReadOnly,
        Sandbox::WorkspaceWrite,
        Sandbox::DangerFullAccess,
    ]
    .into_iter()
    .find(|class| class.intent() == intent) else {
        return Ok(());
    };
    // Engine-written, but read from an input, so it is named bounded.
    let seat = input["seat"].as_str().unwrap_or_default();
    let named = crate::bundle::bounded_site(seat);
    let refuse = |cause: String| {
        format!(
            "dispatch refused: {cause} (decision 0046 ruling 4; operator ruling 2 of 2026-09-23; \
             ruling of 2026-09-25; rebuild unit 5d-fix-b)"
        )
    };
    let gate = class == Sandbox::ReadOnly;
    let door = input.get("result_delivery") == Some(&json!(ResultDoor::LastMessage.word()));
    if door != gate {
        return Err(refuse(format!(
            "the inline Codex launch of seat {named} is admitted '{}', a {}, but its input names \
             the {} result door; the door follows the admitted class, a gate's result reaching \
             the engine only through the last-message door and a work seat's only through the \
             file it writes",
            class.name(),
            if gate { "gate" } else { "work seat" },
            if door { "last-message" } else { "file" }
        )));
    }
    let plan = brokkr_protocol::native_controls::managed(input).map_err(|_| {
        refuse(format!(
            "the native plan of seat {named} is null or cannot be read, so the launch has no \
             capability authority; the reader's cause is not echoed, because it can carry the \
             plan's own text (rebuild unit 5d-fix-c1)"
        ))
    })?;
    let denied: &[String] = match &record.expected.native {
        NativeExpectation::Known { denied, .. } => denied,
        NativeExpectation::Unmeasured(_) => &[],
    };
    let unplanned = match &plan {
        None => Some("carries no native plan"),
        Some(plan) if plan.argv.is_empty() => Some("carries a native plan with no argv"),
        Some(_) => None,
    };
    if let (Some(unplanned), false) = (unplanned, denied.is_empty()) {
        return Err(refuse(format!(
            "the inline Codex launch of seat {named} {unplanned}, while its sealed expectation \
             denies {} native power(s); only the plan's OFF argv expresses a denial, so without \
             it the harness would run at its own defaults (decision 0065 ruling 4; rebuild unit \
             5d-fix-c1)",
            denied.len()
        )));
    }
    let native = plan.map(|plan| Segment::new(Origin::Native, &plan.argv));
    let segments: Vec<Segment> = record.segments.iter().cloned().chain(native).collect();
    let capture = gate.then(|| input["result_path"].as_str().unwrap_or_default());
    grammar::judge_inline_codex_launch(class.intent(), &segments, capture)
        .map_err(|cause| refuse(crate::bundle::inline_codex_refusal(seat, &cause)))?;
    let expressed = grammar::inline_codex_denials(&flatten(&segments));
    match denied
        .iter()
        .find(|power| !expressed.contains(&power.as_str()))
    {
        Some(power) => Err(refuse(format!(
            "the inline Codex launch of seat {named} does not express its sealed native denial \
             of {}; a denial is something the launch proves, so a plan whose OFF argv was \
             removed or changed is refused, never trusted by its claim (decision 0066; rebuild \
             unit 5d-fix-c1)",
            crate::bundle::bounded_site(power)
        ))),
        None => Ok(()),
    }
}

/// The sandbox class an expected state records for the engine's own
/// `local` emission (rebuild unit 5d): the site's typed class where no
/// hands carry it, and none where hands do, because an agent's class rides
/// its hands fragment (design D5.3).
fn local_class(expected: &Expected) -> SandboxIntent {
    match expected.hands {
        HandsIntent::Required => SandboxIntent::Unspecified,
        HandsIntent::None => expected.local.sandbox,
    }
}

/// The expected state of the site a spawn serves (design D5.7), sealed from
/// typed facts alone and never read back from any argv: the serving
/// outcome's identity and native expectation, and the local and hands
/// halves of the selected candidate's own composition — or, at an inline
/// site, of the site's judged local declaration and hands. A candidate that
/// never composed, and an inline site whose local declaration was never
/// judged or declares what no inline command lowers, has no expected state.
pub fn expected_state(
    outcome: &crate::capabilities::Outcome,
    link: Option<&Candidate>,
    facts: Option<&SiteFacts>,
) -> Result<Expected, String> {
    let refused = |problem: &str| {
        format!(
            "dispatch refused: {problem}, so no launch record can be sealed for this site; a \
             record is sealed from typed facts and never repaired into a default one (decision \
             0065 slice one, design D5.7)"
        )
    };
    let (local, hands, template) = match link {
        Some(link) => {
            let composition = composed(link)
                .ok_or_else(|| refused("the selected candidate carries no composition"))?;
            // Rebuild unit 5c-fix2: the adapter's declared template, the
            // composition's typed fact, and never the segment it emitted,
            // which the seal checks against it.
            (
                composition.local(),
                composition.intent.hands,
                composition.template.clone(),
            )
        }
        None => {
            let local = facts
                .and_then(|facts| facts.local.as_ref())
                .ok_or_else(|| refused("the site's local declaration was never judged"))?;
            // Rebuild unit 5b: an allow the compiler lowered is expected
            // as its declared names and the limits kept before they were
            // joined — never read back from the segment it produced.
            let lowered = facts.and_then(|facts| facts.inline_local.as_ref());
            // Rebuild unit 5d: a sandbox class the compiler lowered is
            // expected as the class the site declared — and only where the
            // lowering recorded that same class.
            let sandboxed = facts.and_then(|facts| facts.inline_sandbox.as_ref());
            // A lowered class beside no declared one is no arm's, and falls
            // to the refusal with every other mismatch (unit 26c).
            let declared = (&local.allow, lowered, local.sandbox, sandboxed);
            let (allow, application, sandbox) = match declared {
                (None, None, None, None) => (
                    AllowIntent::Unspecified,
                    Application::Unrestricted,
                    SandboxIntent::Unspecified,
                ),
                (Some(names), Some(lowered), None, None) => (
                    AllowIntent::Listed(names.clone()),
                    Application::Direct(lowered.limits.clone()),
                    SandboxIntent::Unspecified,
                ),
                (None, None, Some(class), _)
                    if sandboxed.is_some_and(|sandboxed| sandboxed.class == class) =>
                {
                    (
                        AllowIntent::Unspecified,
                        Application::Unrestricted,
                        class.intent(),
                    )
                }
                _ => {
                    return Err(refused(
                        "the inline site declares a typed local restriction no inline command \
                         lowers",
                    ))
                }
            };
            let lowers = lowered.is_some() || sandboxed.is_some();
            let template = inline_template(lowers, facts).map_err(refused)?;
            let hands = match facts.map(|facts| &facts.hands) {
                Some(HandsState::Hands(_)) => HandsIntent::Required,
                _ => HandsIntent::None,
            };
            let local = LocalExpectation {
                allow,
                sandbox,
                application,
            };
            (local, hands, template)
        }
    };
    Ok(Expected {
        identity: outcome.identity(),
        native: outcome.native.expected(),
        local,
        hands,
        template,
    })
}

/// The template an inline site is expected to emit (rebuild unit 5c-fix):
/// where its allow or (rebuild unit 5d) its sandbox lowers, the adapter's
/// declaration the compiler recorded beside the lowering — never the
/// segment emitted, which the seal checks against it; where nothing lowers,
/// none, as nothing is emitted.
fn inline_template(
    lowered: bool,
    facts: Option<&SiteFacts>,
) -> Result<TemplateExpectation, &'static str> {
    match (
        lowered,
        facts.and_then(|facts| facts.declared_template.as_ref()),
    ) {
        (false, _) => Ok(TemplateExpectation::None),
        (true, Some(declared)) => Ok(declared.clone()),
        (true, None) => Err(
            "the inline site's lowered restriction carries no recorded declaration \
             of its adapter's permission template",
        ),
    }
}

/// The judge's door at one site (decision 0046 ruling 4; design D23): the
/// harness's capture of the final message, `last-message`, or the file the
/// seat writes. A gate takes the capture where the selected link declares
/// `hands.harness.result` as `last-message` under `harness` with hands, or
/// — rebuild unit 5d, operator ruling of 2026-09-25 — where it is an inline
/// Codex gate whose read-only class the engine lowered onto the adapter's
/// gate fragment, which opens the door that fragment's adapter declares.
/// Every other site writes its file.
pub fn result_door(
    boundary: Boundary,
    gate: bool,
    facts: Option<&SiteFacts>,
    link: Option<&Candidate>,
) -> ResultDoor {
    let hands = matches!(facts.map(|facts| &facts.hands), Some(HandsState::Hands(_)));
    let harness = boundary == Boundary::Harness
        && hands
        && link.map(|link| link.harness.result) == Some(ResultDoor::LastMessage);
    let inline = link.is_none()
        && facts
            .and_then(|facts| facts.inline_sandbox.as_ref())
            .map(|sandboxed| sandboxed.door)
            == Some(ResultDoor::LastMessage);
    match gate && (harness || inline) {
        true => ResultDoor::LastMessage,
        false => ResultDoor::File,
    }
}

/// What the engine prepares for an unboxed exec dispatch and hands the
/// pure composition: the fixed environment and the network prefix when
/// the probe passed (empty otherwise).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Unboxed {
    pub env: BTreeMap<String, String>,
    pub prefix: Vec<String>,
}

/// The prefix when the probe passed, nothing when it did not — one pure
/// function of the answer and the ids, which the argv tests read.
pub(crate) fn network_prefix_if(passes: bool, uid: u32, gid: u32) -> Vec<String> {
    match passes {
        true => brokkr_protocol::hands::network_prefix(uid, gid),
        false => Vec::new(),
    }
}

/// Is this argv a `<engine> driver exec -- …` dispatch? The engine token
/// itself is not matched on, as the compiler never matches on it.
pub(crate) fn is_exec_dispatch(command: &[String]) -> bool {
    crate::argv::Dispatch::of(command).is_exec()
}

/// The directory of the first script compile expanded from `./` in an exec
/// dispatch's tail, `--` or not (#823); later tokens, even roots, are arguments.
fn script_directory(command: &[String], roots: &[PathBuf]) -> Option<PathBuf> {
    let tail = crate::argv::Dispatch::of(command).exec_tail()?;
    tail.iter().find_map(|part| {
        roots
            .iter()
            .find(|root| Path::new(part).strip_prefix(root).is_ok())
            .and_then(|_| Path::new(part).parent().map(Path::to_path_buf))
    })
}

/// The pin and the interpreter argument have different jobs (0048): the
/// canonical script directory is what the spawn re-walks, and the argv
/// stays exactly as compile spelled it, each segment keeping its origin.
/// Later arguments are not judged paths, and filename bytes, backslashes
/// included, stay exact. A site with no hands is unboxed, so it re-walks
/// too, under any word (#823); any argv but an exec dispatch carries none.
fn exec_segments(segments: Vec<Segment>, roots: &[PathBuf]) -> SiteSpawn {
    let mut spawn = SiteSpawn::of(segments);
    spawn.rewalk = script_directory(&spawn.argv, roots);
    spawn
}

/// Who supplied each token of the command a site is composed from: the
/// selected candidate's own segments, carried from the resolver — which
/// must be exactly the command handed over, or the spawn is refused — or,
/// at an inline site, the author, token for token. Bytes that equal an
/// engine composition stay authored: origin is carried, never recognised.
fn supplied(command: &[String], candidate: Option<&Candidate>) -> Result<Vec<Segment>, String> {
    let Some(candidate) = candidate else {
        return Ok(vec![Segment::new(Origin::Authored, command)]);
    };
    match &candidate.lowering {
        Lowering::Composed(composition) if flatten(&composition.segments) == command => {
            Ok(composition.segments.clone())
        }
        Lowering::Composed(_) => Err(
            "dispatch refused: the command handed to composition is not the selected \
             candidate's own composition, and an argument whose origin is not carried is never \
             trusted by its bytes (decision 0065 slice one, design D5.7)"
                .to_string(),
        ),
        Lowering::Unavailable | Lowering::Refused(_) => Err(
            "dispatch refused: the selected candidate carries no composition, so who supplied \
             its arguments is unknown (decision 0065 slice one, design D5.7)"
                .to_string(),
        ),
    }
}

/// `mapped`, whose trailing tokens map `segments` token for token behind
/// an engine-built prefix, as segments: the prefix `hands`, where there is
/// one, then each supplied segment over its own mapped tokens, by the
/// lengths it was supplied with.
fn behind(mapped: Vec<String>, segments: &[Segment]) -> Vec<Segment> {
    let prefix = mapped.len() - flatten(segments).len();
    let mut out = engine_hands(&mapped[..prefix]);
    let mut start = prefix;
    for segment in segments {
        let end = start + segment.argv.len();
        out.push(Segment::new(segment.origin, &mapped[start..end]));
        start = end;
    }
    out
}

/// What the engine composed for the boundary, as a `hands` segment — none
/// where it composed nothing.
fn engine_hands(tokens: &[String]) -> Vec<Segment> {
    match tokens.is_empty() {
        true => Vec::new(),
        false => vec![Segment::new(Origin::Hands, tokens)],
    }
}

/// Compose one site's argv and environment from the boundary the run
/// stands under (decision 0046 ruling 4; design DD18) — a pure function
/// the argv tests read directly:
///
/// - a site without hands: its command in the engine's environment, an
///   exec dispatch's script directory marked for the re-walk (#823);
/// - under a boundary Brokkr builds: today's path token for token —
///   `hands_command`'s box — in the engine's environment;
/// - under `harness` and `open`, an exec dispatch: the compiled command
///   behind the network prefix when the probe passed, in the fixed
///   environment, with the canonical script directory marked for the
///   spawn re-walk;
///   work or gate, the class unread (proposal D32);
/// - under `harness`, a model site: the adapter's `gate` or `work`
///   fragment by class appended to the unboxed resolution's argv with
///   `{result_path}` and `{brokkr}` expanded, no workspace tool served,
///   the engine's environment kept because the harness needs the
///   operator's keys;
/// - under `open`, a model site: the base driver argv and nothing of
///   Brokkr's.
///
/// `boxed` is what a box is handed: the engine executable, which also fills
/// a harness fragment's `{brokkr}` (#349), the run's artifact root (U8a)
/// and the reach admission sealed for these hands (U8a2c).
#[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
pub fn compose_site(
    boundary: BuiltBoundary,
    class: SeatClass,
    command: Vec<String>,
    hands: Option<&HandsSpec>,
    candidate: Option<&Candidate>,
    workdir: &Path,
    boxed: (&Executable, &Path, Option<&Sealed>),
    roots: &[PathBuf],
    result_path: &str,
    unboxed: Option<&Unboxed>,
) -> SiteSpawn {
    compose_site_at(
        None,
        boundary,
        class,
        command,
        hands,
        candidate,
        workdir,
        boxed,
        roots,
        result_path,
        unboxed,
    )
}

/// [`compose_site`] at a compiled site, with its facts: an inline site
/// whose typed allow the compiler lowered (rebuild unit 5b; design D5.3,
/// D5.7) is composed from its authored command and, behind it, the
/// adapter's declared permission template as the engine's own `template`
/// segment where the adapter declares one (rebuild unit 5c), then the
/// engine's own `local` segment — the order an agent's composition gives
/// them — each carried as the compiler recorded it and never recognised in
/// the argv. Rebuild unit 5d: an inline Codex seat's lowered sandbox class
/// is the engine's own `local` segment in the same place, the result path
/// filled into it as it is into a harness fragment. Rebuild unit 14a4a
/// (operator ruling (B) of 2026-09-27): an inline site with hands is served
/// like an agent — its recorded `hands.workspace` fragment follows every
/// other segment as the engine's own `hands` segment, as `agents::compose`
/// appends an agent's last, and [`hands_command`] expands its tokens. Every
/// other site is composed from its supplied segments alone.
#[expect(clippy::too_many_arguments, reason = "decision 0065 slice one, #288")]
pub fn compose_site_at(
    facts: Option<&SiteFacts>,
    boundary: BuiltBoundary,
    class: SeatClass,
    command: Vec<String>,
    hands: Option<&HandsSpec>,
    candidate: Option<&Candidate>,
    workdir: &Path,
    (engine, artifacts, sealed): (&Executable, &Path, Option<&Sealed>),
    roots: &[PathBuf],
    result_path: &str,
    unboxed: Option<&Unboxed>,
) -> SiteSpawn {
    let lowered = facts.and_then(|facts| facts.inline_local.as_ref());
    let sandboxed = facts.and_then(|facts| facts.inline_sandbox.as_ref());
    let handed = facts.and_then(|facts| facts.inline_hands.as_ref());
    let fill = |token: &String| token.replace("{result_path}", result_path);
    let filled = sandboxed.map(|sandboxed| Segment {
        origin: sandboxed.segment.origin,
        argv: sandboxed.segment.argv.iter().map(fill).collect(),
    });
    // Every arm carries these segments through its own composition, so the
    // spawn says who supplied each token it will launch (design D5.7). A
    // command that is not the selected candidate's composition keeps its
    // argv, all of it authored, and is refused at the door.
    let recorded = [lowered.is_some(), filled.is_some(), handed.is_some()];
    let (segments, refusal) = match supplied(&command, candidate) {
        Ok(_) if candidate.is_none() && recorded.contains(&true) => (
            std::iter::once(Segment::new(Origin::Authored, &command))
                .chain(facts.and_then(|facts| facts.inline_template.clone()))
                .chain(lowered.map(|lowered| lowered.segment.clone()))
                .chain(filled)
                .chain(handed.cloned())
                .collect(),
            None,
        ),
        Ok(segments) => (segments, None),
        Err(reason) => (vec![Segment::new(Origin::Authored, &command)], Some(reason)),
    };
    let site = Composing {
        boundary,
        class,
        hands,
        candidate,
        workdir,
        boxed: (engine, artifacts, sealed),
        roots,
        result_path,
        unboxed,
    };
    let mut spawn = compose_segments(&site, segments);
    spawn.refusal = spawn.refusal.or(refusal);
    spawn.class = Some(class);
    spawn
}

/// What [`compose_segments`] composes a site's segments under: the inputs
/// [`compose_site_at`] was handed beside its command, carried as one value
/// so the private composer restates neither public signature.
struct Composing<'a> {
    boundary: BuiltBoundary,
    class: SeatClass,
    hands: Option<&'a HandsSpec>,
    candidate: Option<&'a Candidate>,
    workdir: &'a Path,
    boxed: (&'a Executable, &'a Path, Option<&'a Sealed>),
    roots: &'a [PathBuf],
    result_path: &'a str,
    unboxed: Option<&'a Unboxed>,
}

fn compose_segments(site: &Composing<'_>, segments: Vec<Segment>) -> SiteSpawn {
    let Composing {
        boundary,
        class,
        hands,
        candidate,
        workdir: _,
        boxed,
        roots,
        result_path,
        unboxed,
    } = *site;
    let Some(spec) = hands else {
        return exec_segments(segments, roots);
    };
    let command = flatten(&segments);
    if boundary != BuiltBoundary::Namespace && is_exec_dispatch(&command) {
        let unboxed = unboxed.cloned().unwrap_or_default();
        let mut spawn = exec_segments(segments, roots);
        // The network prefix is the engine's, composed for the boundary.
        let mut prefixed = engine_hands(&unboxed.prefix);
        prefixed.append(&mut spawn.segments);
        spawn.segments = prefixed;
        spawn.argv.splice(..0, unboxed.prefix);
        spawn.env = SpawnEnv::Exactly(unboxed.env);
        return spawn;
    }
    match boundary {
        // A model seat's workspace fragment is already in its segments —
        // `agents::compose` appended it last, as `hands` — and
        // `hands_command` expands every token in place, behind the box's
        // own prefix for an exec dispatch, so each segment maps onto its
        // own tokens. An inline site — an exec dispatch included — has no
        // candidate: its argv is the author's, and a model site's hands are
        // the recorded fragment `compose_site_at` appended last (rebuild
        // unit 14a4a). An exec dispatch is re-walked as an unboxed one (#832).
        BuiltBoundary::Namespace => boxed_spawn(site, spec, command, segments),
        BuiltBoundary::Harness => {
            let brokkr = boxed.0.lossy();
            let fragment = candidate.and_then(|candidate| match class {
                SeatClass::Gate => candidate.harness.gate.as_deref(),
                SeatClass::Work => candidate.harness.work.as_deref(),
            });
            let fragment: Vec<String> = fragment
                .unwrap_or(&[])
                .iter()
                .map(|token| {
                    token
                        .replace("{result_path}", result_path)
                        .replace("{brokkr}", &brokkr)
                })
                .collect();
            let mut segments = segments;
            segments.extend(engine_hands(&fragment));
            SiteSpawn::of(segments)
        }
        BuiltBoundary::Open => SiteSpawn::of(segments),
    }
}

/// The stamp (decision 0046 ruling 3; design DD19): a record that names
/// a `model` carries `boundary` beside it — the site's word, or the
/// sentinel for a site without hands — replacing any value a driver
/// wrote; a record that names none carries no `boundary`, and a driver's
/// word on such a record is dropped. The trigger is the record's own key,
/// never a step name, because the engine is the only party that knows
/// which boundary it built.
pub(crate) fn stamp_boundary(record: Value, boundary: Option<Boundary>) -> Value {
    match record {
        Value::Object(mut object) => {
            if object.contains_key("model") {
                object.insert(
                    "boundary".into(),
                    Value::String(Boundary::recorded(boundary).to_string()),
                );
            } else {
                object.remove("boundary");
            }
            Value::Object(object)
        }
        other => other,
    }
}

/// Copy the seat-level secret-binding facts (decision 0012: declared
/// names + store path, never values) into a derived member/step driver
/// input, when the seat binds any.
fn copy_secret_binding_facts(input: &mut Value, seat_input: &Value) {
    if let Some(secrets) = seat_input.get("secrets") {
        input["secrets"] = secrets.clone();
        input["secrets_file"] = seat_input["secrets_file"].clone();
    }
}

/// Tag a checkpoint payload with the member/step it came from — the
/// `member` field the seats console groups by.
fn tag_member(checkpoint: Value, member: &str) -> Value {
    match checkpoint {
        Value::Object(mut object) => {
            object.insert("member".into(), Value::String(member.to_string()));
            Value::Object(object)
        }
        other => json!({"member": member, "value": other}),
    }
}

fn stderr_tail(stderr: &str) -> String {
    stderr
        .chars()
        .rev()
        .take(2000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// The first abstract definition or tool dialect a compiled `capabilities`
/// section pins that `root` no longer reproduces: its relative source, and
/// whether it is missing or changed. The digest is over the file's raw
/// bytes, exactly as the compile took it. `None` where every pin holds —
/// which a section that consulted nothing trivially does.
fn moved_capability_input(compiled: &Value, root: &Path) -> Option<(String, String)> {
    ["definitions", "dialects"]
        .into_iter()
        .filter_map(|records| compiled.get(records)?.as_object())
        .flat_map(Map::values)
        .find_map(|record| {
            let source = record["source"].as_str().unwrap_or_default();
            let problem = match std::fs::read(root.join(source)) {
                Ok(bytes)
                    if record["sha256"] == json!(brokkr_core::canonical::sha256_bytes(&bytes)) =>
                {
                    None
                }
                Ok(_) => Some("has changed since"),
                Err(_) => Some("is missing"),
            };
            Some((source.to_string(), problem?.to_string()))
        })
}

fn manifest_diff(pinned: &Value, current: &Value) -> String {
    let empty = Map::new();
    let pinned_files = pinned
        .get("files")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let current_files = current
        .get("files")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut diffs = Vec::new();
    for (path, digest) in pinned_files {
        match current_files.get(path) {
            None => diffs.push(format!("missing: {path}")),
            Some(d) if d != digest => diffs.push(format!("changed: {path}")),
            _ => {}
        }
    }
    for path in current_files.keys() {
        if !pinned_files.contains_key(path) {
            diffs.push(format!("added: {path}"));
        }
    }
    if !diffs.is_empty() {
        return diffs.join(", ");
    }
    // The boundary is part of the identity (decision 0046 ruling 1): a
    // run resumes only under the word it was started with, and the
    // refusal says which word moved rather than blaming a version.
    if pinned.get("boundary") != current.get("boundary") {
        return format!(
            "boundary differs: the run pinned {} and the bundle compiled under {}",
            pinned
                .get("boundary")
                .map_or("no boundary".to_string(), Value::to_string),
            current
                .get("boundary")
                .map_or("no boundary".to_string(), Value::to_string)
        );
    }
    // So is capability authority (decision 0065 ruling 8): a run resumes
    // only under the grants, definitions, dialects and native declarations
    // it was started with, and the refusal names capabilities and says
    // which record moved. A run pinned before the ruling carries no such
    // section, and is never rewritten to resume under authority it was not
    // started with.
    if pinned.get("capabilities") != current.get("capabilities") {
        let Some(was) = pinned.get("capabilities") else {
            return "capabilities differ: the run was pinned before decision 0065 and records no \
                    capability authority, which every bundle compiled now carries; a historical \
                    run is never rewritten to resume under authority it was not started with"
                .to_string();
        };
        let moved: Vec<&str> = ["realm", "grants", "definitions", "dialects", "sites"]
            .into_iter()
            .filter(|record| was.get(record) != current.pointer(&format!("/capabilities/{record}")))
            .collect();
        return format!(
            "capabilities differ: the run's pinned {} no longer match what the bundle compiles \
             to here — a grant, an abstract definition, a tool dialect or an adapter's native \
             declaration was added, removed or edited since the run started",
            moved.join(", ")
        );
    }
    "non-file manifest fields differ (engine or contract version)".to_string()
}

/// The `requires_artifacts` gate's verdict: every failing entry with its
/// class, in table order — the complete list, not first-fail, so one park
/// carries all the evidence. Empty means the gate passes. Entries are
/// strictly static workdir-relative paths, verified verbatim: no
/// substitution, no tokens (`{ } $ < >` are reserved, not assigned), and
/// nothing that could resolve outside the workdir. Every probe error
/// fails closed. This is the only place the gate touches the filesystem.
fn artifact_failures(workdir: &Path, required: &[String]) -> Vec<(String, &'static str)> {
    let mut failures = Vec::new();
    for entry in required {
        let lexically_valid = !entry.is_empty()
            && !entry.starts_with('/')
            && !entry.contains(['\\', '\0', '{', '}', '$', '<', '>'])
            && !entry
                .split('/')
                .any(|component| component == "." || component == "..");
        let class = if !lexically_valid {
            Some("invalid")
        } else {
            // metadata, not symlink_metadata: the gate asserts presence
            // of content, not provenance — a dangling symlink is missing.
            match std::fs::metadata(workdir.join(entry)) {
                Err(_) => Some("missing"),
                Ok(meta) if !meta.is_file() => Some("not-a-file"),
                Ok(meta) if meta.len() == 0 => Some("empty"),
                Ok(_) => None,
            }
        };
        if let Some(class) = class {
            failures.push((entry.clone(), class));
        }
    }
    failures
}

/// The single producer of the gate's journal-borne evidence string —
/// machine-stable contract, pinned character-exact by spec and proof.
fn artifact_problem(rule_id: &str, failures: &[(String, &'static str)]) -> String {
    let list = failures
        .iter()
        .map(|(entry, class)| format!("{class}: {entry}"))
        .collect::<Vec<_>>()
        .join("; ");
    format!("requires_artifacts unmet for rule {rule_id}: {list}")
}

impl Engine {
    /// The payload of `phase/entered`: the phase, and for the protected
    /// phase the repository head it was entered at (decision 0039), so
    /// the phase's own commits can later be told from the ones it judged.
    /// Optional and absent when the operated repository's head cannot be
    /// read, and published as `contracts/phase-entered-head.v1.schema.json`;
    /// `fold` never reads it.
    fn phase_entered_payload(&self, phase: &str, state: &RunState) -> Value {
        let mut payload = json!({"phase": phase});
        if let Some((_, Some(case))) = self
            .bundle
            .seats
            .get(phase)
            .and_then(|seat| seat.body.selected(state.strategy.as_deref()))
        {
            payload["case"] = Value::String(case.to_string());
        }
        let returning_implement = phase == phase_name::IMPLEMENT
            && state
                .visits
                .get(phase_name::IMPLEMENT)
                .copied()
                .unwrap_or(0)
                > 0;
        if phase == self.bundle.protected_phase || returning_implement {
            if let Some(head) = git_head(&self.repo) {
                payload["head"] = Value::String(head);
            }
        }
        payload
    }

    /// Decision 0039: whether every commit the protected phase added
    /// between the head it was entered at and `head`, the repository's
    /// head now, lies in the docs class the repository declared AT THAT
    /// ENTRY HEAD. `None` whenever the question has no honest answer —
    /// the phase's latest entry recorded no head or one that is not a
    /// commit id, the same head, nothing committed, a diff git cannot
    /// take, or no class declared at the entry head — and an absent
    /// input never satisfies a rule (decision 0004). A repository whose
    /// head cannot be read never reaches here: the caller records no
    /// head and asks nothing.
    ///
    /// The entry head is the LATEST entry's, taken as recorded: an older
    /// visit's head is never borrowed, because the phase's own commits
    /// are exactly the ones since it was last entered. The head is
    /// checked against the contract's shape before git sees it — the
    /// chain is unkeyed, and a journal row is not an argument list.
    fn fixes_docs_only(&self, repo: &std::path::Path, phase: &str, head: &str) -> Option<bool> {
        let events = self.store.load(&self.run_id).ok()?;
        let entered = events
            .iter()
            .rev()
            .find(|event| {
                event.event_type == EventType::PhaseEntered && event.payload["phase"] == phase
            })?
            .payload["head"]
            .as_str()
            .filter(|entered| is_commit_id(entered))?;
        if entered == head {
            return None;
        }
        let class = docs_class(repo, entered)?;
        let paths = crate::anchor::changed_paths(repo, entered, head)?;
        if paths.is_empty() {
            return None;
        }
        Some(
            paths
                .iter()
                .all(|path| class.iter().any(|pattern| pattern.is_match(path))),
        )
    }
}

/// The shape `contracts/phase-entered-head.v1.schema.json` promises:
/// forty lowercase hex digits, and nothing that could read as an option.
fn is_commit_id(candidate: &str) -> bool {
    candidate.len() == 40
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Where the repository declares its own delivery classes (decision 0038
/// ruling 3) — the same file the contribution gate reads.
const DELIVERY_CLASSES: &str = ".github/delivery-classes.json";

/// Where, in the run-local directory, each attempt's seat scratch tree is
/// made: what admission's scratch floor measures by default (#430).
pub(crate) const SEAT_SCRATCH: &str = "scratch";

/// Where, in the run-local directory, each seat's result file is written.
const RESULTS: &str = "results";

/// The docs class the repository declared at `head`: `classes.docs.paths`
/// of `.github/delivery-classes.json` as committed there, regular
/// expressions over the repository-relative path. Read from the commit
/// and never from the working tree, for the reason the gate reads the
/// base branch's copy: the tree at ruling is the judged phase's own, and
/// a phase that could widen the class its fixes are judged by has been
/// handed the gate. A change to the class file is then a path like any
/// other, classified by the class it was entered under. Absent at that
/// head, or malformed, means no class — and therefore no answer, never a
/// guess. The patterns are compiled here by the `regex` crate and joined
/// by the gate into jq's; a pattern only one dialect accepts is refused
/// here, and refusal reads as no class.
fn docs_class(repo: &std::path::Path, head: &str) -> Option<Vec<regex::Regex>> {
    let out = Command::new("git")
        .args(["cat-file", "blob", &format!("{head}:{DELIVERY_CLASSES}")])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value: Value = serde_json::from_slice(&out.stdout).ok()?;
    value["classes"]["docs"]["paths"]
        .as_array()?
        .iter()
        .map(|pattern| pattern.as_str().and_then(|s| regex::Regex::new(s).ok()))
        .collect()
}

/// The operated repository (#368): the one `--repo` named, else the
/// directory the engine is constructed in. Resolved once, so `workdir()`
/// and every git fact — the ship guards above all — read the same tree
/// instead of the guards switching off when the flag is omitted.
fn operated_repo(repo: Option<PathBuf>) -> PathBuf {
    repo.unwrap_or_else(|| std::env::current_dir().expect("cwd"))
}

/// The repository's observed HEAD, or nothing when there is no readable
/// git tree there. Public because `brokkr realms` reads out the same
/// fact the ship gate compares against — one reader, not two.
pub fn git_head(repo: &std::path::Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_dirty(repo: &std::path::Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(true) // unreadable repo counts as dirty: fail closed
}

/// Whether the EFFECT's own span carries the gate-head observation
/// (decision 0041 ruling 4). A gate single seat or an all-gate panel has
/// no inner spans, so the effect is the gate and the effect is watched.
/// A sequence never is: its Gate steps arm and compare at their own
/// ends, and `has_gate` — true of an all-gate sequence too — would
/// otherwise put a second, outer observation into the same one field,
/// an observation nothing compares at the right end and nothing accounts
/// for. One observation per gate step, and nothing outer.
fn arms_effect_gate_head(body: &ExecutableBody<'_>, seat: &Seat, strategy: Option<&str>) -> bool {
    !matches!(body, ExecutableBody::Sequence { .. })
        && seat.body.selected_is_gate(strategy, seat.has_gate)
}

/// Recover the raw observations carried by the most recent gate defect. The
/// indeterminate reason is an existing string field; the structured copy is
/// attached to `run/parked`, the contract's evidence envelope. Any JSON the
/// marker carries is passed through as it reads, as before the codec; typing
/// it as `MovedHead` would narrow what a journal may hold and awaits a ruling.
fn gate_head_evidence(events: &[EventEnvelope]) -> Value {
    events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectIndeterminate)
        .find_map(|event| {
            gate_moved_head::decode::<Value>(event.payload["reason"].as_str()?).and_then(Result::ok)
        })
        .unwrap_or(json!({}))
}

pub(crate) mod resume;

#[cfg(test)]
mod agent_tests;
#[cfg(test)]
mod artifact_gate_tests;
#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod cleanup_tests;
#[cfg(test)]
mod conclude_tests;
#[cfg(test)]
mod contention_tests;
#[cfg(test)]
mod gate_head_tests;
#[cfg(test)]
mod notice_tests;
#[cfg(test)]
mod operator_tests;
#[cfg(test)]
mod resume_tests;
#[cfg(test)]
mod secret_threading_tests;
#[cfg(test)]
pub(crate) mod tests;
