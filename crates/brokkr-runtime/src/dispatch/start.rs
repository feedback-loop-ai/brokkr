//! One entry's start from the queue, and what a refused start writes
//! (decision 0110 ruling 2; decision 0095 rulings 1, 4, 5 and 7): claim it
//! through the room the pass measured, rebuild the launch from its payload,
//! start the run under the claim, let the run's lock go, link the entry to
//! the run, write the `spawn` row and hand the run to the spawn seam in
//! place of a process. Every step is checked inside its own transaction by
//! the store, under this dispatcher's generation.
//!
//! A start's refusal is disposed of by a closed match on its variant, never
//! by a wildcard: the bundle moved and contention return the entry to its
//! place uncounted; the crossing fence, the compile (rebuilding from the
//! payload included) and an unboxable bundle count against the journal's
//! bound, holding the entry at it; every other refusal writes nothing and
//! ends the dispatcher, leaving the entry `launching` for the next
//! dispatcher's reconciliation. A `Contended` link or `spawn` row is left as
//! it stands for reconciliation and never reclaimed; a `Contended` disposal
//! is retried until it lands, with no other claim in between.

use std::ffi::OsString;
use std::num::NonZeroU32;

use brokkr_store::{
    Claim, EntryId, Generation, QueueEntry, QueueRefusal, StartRefusal, Store, StoreError,
};

use super::pass::{Contended, Ending, Held, Reclaimed, Seams, Stage, Started, Subject};
use super::Dispatcher;
use crate::admission::{AdmissionError, Reason, Room};
use crate::launch::{self, LaunchError, QueuedLaunch};
use crate::realms::WorldError;
use crate::EngineError;

/// The run, under its claim, that a spawn seam is handed once its `spawn`
/// row has landed: the entry, the claim, the generation that wrote the row
/// and the run's id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spawn {
    pub entry: EntryId,
    pub claim: Claim,
    pub generation: Generation,
    pub run: String,
}

/// What a spawn seam started, in place of a process: its pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spawned {
    pub pid: u32,
}

/// Why a spawn seam could not start a run. The run's `spawn` row stands,
/// so it is never spawned again; the report names the run it failed to
/// start, and 6b's view words the command that resumes it (decision 0071
/// ruling 7).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpawnFailed {
    #[error("{0}")]
    Refused(String),
}

/// One entry's start as its pass reports it.
#[derive(Debug)]
pub(crate) enum Outcome {
    /// The run exists, the entry is linked and its `spawn` row landed; the
    /// seam's answer is kept either way.
    Started(Started),
    /// A claim's re-check refused it: nothing was written, and the next
    /// entry is judged.
    ClaimRefused(ClaimRefused),
    /// A refused start returned the entry to its place, counted or not.
    Reclaimed(Reclaimed),
    /// A counted refusal reached the journal's bound and held the entry.
    Held(Held),
    /// A step was `Contended` past its patience: reported, and no further
    /// entry is claimed this pass.
    Contended(Contended),
    /// A claim could not be judged: reported, and no further entry is
    /// claimed this pass.
    Admission(AdmissionError),
}

/// A claim a re-check refused: the entry and the reason, with nothing
/// written.
#[derive(Debug)]
pub struct ClaimRefused {
    pub entry: EntryId,
    pub reason: Reason,
}

/// One admitted entry as its start is handed it: the entry, the launch to
/// rebuild and the room its claim re-checks. A parameter object, as decision
/// 0071 ruling 10 asks of a list this long.
#[derive(Debug)]
pub(crate) struct Admitted {
    pub(crate) entry: QueueEntry,
    pub(crate) launch: QueuedLaunch,
    pub(crate) room: Room,
}

/// Start one admitted entry: claim it, rebuild and start its run, drop the
/// engine so its lock goes, then link, `spawn` and hand it to the seam.
pub(crate) fn start<S, P>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    admitted: Admitted,
    bound: NonZeroU32,
    host_path: OsString,
    seams: &mut Seams<S, P>,
) -> Result<Outcome, Ending>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
{
    let Admitted {
        entry,
        launch,
        room,
    } = admitted;
    let id = entry.id;
    let generation = dispatcher.generation;
    (seams.stands)()?;
    let claim = match room.claim(store, generation, &claim_reason(id, generation)) {
        Ok(Ok(claim)) => claim,
        Ok(Err(reason)) => return Ok(Outcome::ClaimRefused(ClaimRefused { entry: id, reason })),
        Err(error) => match admission_refused(error) {
            AdmissionRefused::Store(error) => {
                return contend(error, Subject::Entry(id), Stage::Claim)
            }
            AdmissionRefused::Other(error) => return Ok(Outcome::Admission(error)),
        },
    };
    let remeasure = launch.clone();
    let (request, mut run) = match launch.rebuild(store.path().to_path_buf(), host_path) {
        Ok(built) => built,
        Err(error) => return dispose(store, claim, generation, bound, class(error), &remeasure),
    };
    run.claim = Some((claim, generation));
    let engine = match launch::start(request, run, &mut silent) {
        Ok(engine) => engine,
        Err(error) => return dispose(store, claim, generation, bound, class(error), &remeasure),
    };
    let run = engine.run_id.clone();
    // Decision 0108 ruling 1 and 0110 ruling 3: the engine holds the run's
    // lock until it is dropped, and the link waits for it to go.
    drop(engine);
    link_and_spawn(
        store,
        dispatcher,
        claim,
        &run,
        &mut seams.stands,
        &mut seams.spawn,
    )
}

/// Link `claim` to `run`, write its `spawn` row and hand it to the seam:
/// the steps after the run exists, kept apart so a test can stand a
/// contended `link` or `spawn` at its own step.
pub(crate) fn link_and_spawn(
    store: &mut Store,
    dispatcher: &Dispatcher,
    claim: Claim,
    run: &str,
    stands: &mut dyn FnMut() -> Result<(), Ending>,
    spawn: &mut dyn FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
) -> Result<Outcome, Ending> {
    let generation = dispatcher.generation;
    let entry = claim.parts().0;
    if let Err(error) = store.queue_link(claim, generation, run, &link_reason(claim, run)) {
        return contend(error, Subject::Claim(entry, claim), Stage::Link);
    }
    stands()?;
    if let Err(error) = store.queue_spawn(claim, generation, &spawn_reason(claim, run)) {
        return contend(error, Subject::Claim(entry, claim), Stage::Spawn);
    }
    let spawned = spawn(&Spawn {
        entry,
        claim,
        generation,
        run: run.to_string(),
    });
    Ok(Outcome::Started(Started {
        entry,
        claim,
        run: run.to_string(),
        spawned,
    }))
}

/// The reasons the dispatcher writes on each step, naming the generation,
/// the claim or the run as each is read.
fn claim_reason(entry: EntryId, generation: Generation) -> String {
    format!("dispatch generation {} claims entry {entry}", generation.0)
}

fn link_reason(claim: Claim, run: &str) -> String {
    let (entry, seq) = claim.parts();
    format!("dispatch: claim {seq} of entry {entry} started run '{run}'")
}

pub(crate) fn spawn_reason(claim: Claim, run: &str) -> String {
    let (entry, seq) = claim.parts();
    format!("dispatch: run '{run}' under claim {seq} of entry {entry} is handed to its driver")
}

/// A refused store step: contention is reported and left for
/// reconciliation; the other three classes end the dispatcher with nothing
/// more written.
fn contend(error: StoreError, subject: Subject, stage: Stage) -> Result<Outcome, Ending> {
    match refused(error) {
        Refused::Contended(error) => Ok(Outcome::Contended(Contended {
            subject,
            stage,
            error,
        })),
        Refused::Fenced(generation) => Err(Ending::Fenced(generation)),
        Refused::QueueStorage(refusal) => Err(Ending::QueueStorage(refusal)),
        Refused::Store(error) => Err(Ending::Store(error)),
    }
}

/// How a start's outcome is disposed of (decision 0110 ruling 2).
enum Class {
    /// The bundle moved: return it uncounted, naming the claim's digest.
    BundleMoved(String),
    /// The start was `Contended`: return it uncounted.
    Contended,
    /// A refusal that may recur on the same bytes: count it.
    Counted(StartRefusal),
    /// A newer dispatcher holds the journal: write nothing and end.
    Fenced(Generation),
    /// The queue storage is not one this binary writes.
    QueueStorage(QueueRefusal),
    /// Any other store refusal: write nothing and end.
    Store(StoreError),
    /// Any other start refusal: write nothing and end.
    Start(LaunchError),
}

/// Dispose of a refused start by its variant: reclaim uncounted, reclaim or
/// hold counted, or end with nothing more written.
fn dispose(
    store: &mut Store,
    claim: Claim,
    generation: Generation,
    bound: NonZeroU32,
    class: Class,
    remeasure: &QueuedLaunch,
) -> Result<Outcome, Ending> {
    let (entry, seq) = claim.parts();
    match class {
        Class::BundleMoved(measured) => {
            let started = match remeasure.compiled() {
                Ok(bundle) => bundle.manifest_digest(),
                Err(_) => "unreadable".into(),
            };
            let reason = format!(
                "start refused: claim {seq} of entry {entry} measured bundle {measured}, and this \
                 start compiled {started}"
            );
            reclaim(store, claim, generation, bound, None, reason)
        }
        Class::Contended => {
            let reason = format!("start contended: claim {seq} of entry {entry} wrote no run");
            reclaim(store, claim, generation, bound, None, reason)
        }
        Class::Counted(refusal) => reclaim(
            store,
            claim,
            generation,
            bound,
            Some(refusal),
            refusal.word().to_string(),
        ),
        Class::Fenced(fenced) => Err(Ending::Fenced(fenced)),
        Class::QueueStorage(refusal) => Err(Ending::QueueStorage(refusal)),
        Class::Store(error) => Err(Ending::Store(error)),
        Class::Start(error) => Err(Ending::Start(error)),
    }
}

/// Return the entry to its place, retrying a `Contended` disposal until it
/// lands under the store's own patience, with no other claim in between.
fn reclaim(
    store: &mut Store,
    claim: Claim,
    generation: Generation,
    bound: NonZeroU32,
    refused: Option<StartRefusal>,
    reason: String,
) -> Result<Outcome, Ending> {
    loop {
        match store.queue_reclaim(claim, generation, refused, bound, &reason) {
            Ok(count) => {
                return Ok(match refused {
                    Some(refusal) if count >= bound.get() => Outcome::Held(Held {
                        claim,
                        refusal,
                        count,
                    }),
                    _ => Outcome::Reclaimed(Reclaimed {
                        claim,
                        refused,
                        count,
                    }),
                });
            }
            Err(error) if error.is_contention() => continue,
            Err(error) => return Err(ended(error)),
        }
    }
}

/// An admission refusal, split into the store's error and the rest, so the
/// pass and a claim share one closed reading of `AdmissionError` (decision
/// 0071 rulings 2 and 5).
pub(crate) enum AdmissionRefused {
    Store(StoreError),
    Other(AdmissionError),
}

/// Split an `AdmissionError` as the pass and a claim both read it.
pub(crate) fn admission_refused(error: AdmissionError) -> AdmissionRefused {
    match error {
        AdmissionError::Store(error) => AdmissionRefused::Store(error),
        error @ (AdmissionError::Entry { .. }
        | AdmissionError::Fold { .. }
        | AdmissionError::Latch { .. }
        | AdmissionError::NothingLatched(_)
        | AdmissionError::MapMoved(_)
        | AdmissionError::LatchMoved { .. }
        | AdmissionError::Unmeasured(_)
        | AdmissionError::OtherJournal(_)
        | AdmissionError::Host(_)) => AdmissionRefused::Other(error),
    }
}

/// A store refusal, split into what the pass reports and what ends it.
pub(crate) enum Refused {
    Contended(StoreError),
    Fenced(Generation),
    QueueStorage(QueueRefusal),
    Store(StoreError),
}

/// Split a `StoreError` for the pass's report: contention is reported, a
/// newer holder fences the dispatcher out, unknown queue storage names its
/// version, and every other store error ends the dispatcher, a clock the
/// store cannot read included (decision 0110 ruling 2's closed disposal).
pub(crate) fn refused(error: StoreError) -> Refused {
    match error {
        StoreError::Contended { .. } => Refused::Contended(error),
        StoreError::Queue(refusal) => match queue_refusal(refusal) {
            Queue::Fenced(generation) => Refused::Fenced(generation),
            Queue::Storage(refusal) => Refused::QueueStorage(refusal),
            Queue::Other(refusal) => Refused::Store(StoreError::Queue(refusal)),
        },
        other @ (StoreError::AppendConflict { .. }
        | StoreError::AfterTerminal { .. }
        | StoreError::AlreadyAcknowledged { .. }
        | StoreError::Chain(_)
        | StoreError::Clock { .. }
        | StoreError::HeadMoved { .. }
        | StoreError::Json(_)
        | StoreError::RowBeforeChain { .. }
        | StoreError::RunExists(_)
        | StoreError::RunIdTooLong { .. }
        | StoreError::RunIdUnnameable(_)
        | StoreError::RunNotFound(_)
        | StoreError::SchemaMismatch { .. }
        | StoreError::SeatRecord(_)
        | StoreError::SeatTree(_)
        | StoreError::Sqlite(_)
        | StoreError::TokenCountOutOfRange(..)
        | StoreError::UnknownHead { .. }) => Refused::Store(other),
    }
}

/// The queue refusals that decide a class, and the rest.
enum Queue {
    Fenced(Generation),
    Storage(QueueRefusal),
    Other(QueueRefusal),
}

fn queue_refusal(refusal: QueueRefusal) -> Queue {
    match refusal {
        QueueRefusal::Superseded(generation) => Queue::Fenced(generation),
        QueueRefusal::SchemaMismatch { found } => {
            Queue::Storage(QueueRefusal::SchemaMismatch { found })
        }
        other @ (QueueRefusal::UnknownEntry(_)
        | QueueRefusal::UnknownWait(_)
        | QueueRefusal::DuplicateWait(_)
        | QueueRefusal::PastTheEnds { .. }
        | QueueRefusal::AlreadyHeld(_)
        | QueueRefusal::NotHeld(_)
        | QueueRefusal::Held(_)
        | QueueRefusal::Claimed { .. }
        | QueueRefusal::Dropped(_)
        | QueueRefusal::RunTaken { .. }
        | QueueRefusal::LatchMoved { .. }
        | QueueRefusal::PinMoved { .. }
        | QueueRefusal::Unmeasured { .. }
        | QueueRefusal::TableLost { .. }
        | QueueRefusal::CooldownTime { .. }
        | QueueRefusal::CooldownDuplicate(_)
        | QueueRefusal::CooldownRoundEvent { .. }
        | QueueRefusal::CooldownRoundWait { .. }
        | QueueRefusal::CooldownRoundNotNext(_)
        | QueueRefusal::CooldownRoundUnwritten(_)
        | QueueRefusal::CooldownRowUnwritten(_)
        | QueueRefusal::ProbeHoldUnwritten(_)
        | QueueRefusal::ProbeHoldUnending(_)
        | QueueRefusal::ProbeHoldUnstorable(_)
        | QueueRefusal::ProbeHoldUnsettled(_)
        | QueueRefusal::Launching(_)
        | QueueRefusal::InFlight(_)
        | QueueRefusal::Unseen(_)
        | QueueRefusal::Stale(_)
        | QueueRefusal::OtherGeneration(..)
        | QueueRefusal::BundleMoved(..)
        | QueueRefusal::Carried(..)) => Queue::Other(other),
    }
}

/// A store refusal at the end of a disposal, where contention has already
/// been retried away.
fn ended(error: StoreError) -> Ending {
    match refused(error) {
        Refused::Contended(error) => Ending::Store(error),
        Refused::Fenced(generation) => Ending::Fenced(generation),
        Refused::QueueStorage(refusal) => Ending::QueueStorage(refusal),
        Refused::Store(error) => Ending::Store(error),
    }
}

/// A `LaunchError`'s disposal class (decision 0110 ruling 2's table).
fn class(error: LaunchError) -> Class {
    match error {
        LaunchError::Unboxable(_) => Class::Counted(StartRefusal::Unboxable),
        LaunchError::Compile(_)
        | LaunchError::RecipeNotFound { .. }
        | LaunchError::QueuedWorkspaceRelative(_)
        | LaunchError::QueuedShelf { .. }
        | LaunchError::QueuedShelfUnplaced { .. }
        | LaunchError::Places(_) => Class::Counted(StartRefusal::Compile),
        LaunchError::World(world) => world_class(world),
        LaunchError::Store(StoreError::Queue(QueueRefusal::BundleMoved(_, measured))) => {
            Class::BundleMoved(measured)
        }
        LaunchError::Store(error) => store_class(error),
        LaunchError::Engine(EngineError::Store(StoreError::Queue(QueueRefusal::BundleMoved(
            _,
            measured,
        )))) => Class::BundleMoved(measured),
        LaunchError::Engine(error) => engine_class(error),
        error @ (LaunchError::EntryWithDispatch
        | LaunchError::ClaimWithDispatch
        | LaunchError::Entry(_)
        | LaunchError::Record(_)
        | LaunchError::ContinuedMissing { .. }
        | LaunchError::ContinuedLooperBound { .. }
        | LaunchError::ContinuedUnreadable { .. }
        | LaunchError::ContinuedNotConcluded { .. }
        | LaunchError::QueuedEntry { .. }
        | LaunchError::RealmsWithDispatch
        | LaunchError::ReadDispatch { .. }
        | LaunchError::ParseDispatch(_)
        | LaunchError::EncodeQueued(_)
        | LaunchError::DecodeQueued(_)
        | LaunchError::Dispatch(_)
        | LaunchError::SourceRun { .. }
        | LaunchError::NoFeature { .. }) => Class::Start(error),
    }
}

/// The world's crossing fence counts as a crossing; any other world
/// refusal is a compile.
fn world_class(error: WorldError) -> Class {
    match error {
        WorldError::Crossing { .. } | WorldError::CrossingMoved(_) => {
            Class::Counted(StartRefusal::Crossing)
        }
        WorldError::Missing(_)
        | WorldError::Unreadable { .. }
        | WorldError::Map(_)
        | WorldError::Places(_)
        | WorldError::Unpinned(_)
        | WorldError::RealmText { .. }
        | WorldError::RealmDialect { .. } => Class::Counted(StartRefusal::Compile),
    }
}

/// A store refusal carried by a launch, in the disposal's classes. A
/// `BundleMoved` is classified by its caller, before this, so the queue
/// refusal it would otherwise carry reaches the store class.
fn store_class(error: StoreError) -> Class {
    match refused(error) {
        Refused::Contended(_) => Class::Contended,
        Refused::Fenced(generation) => Class::Fenced(generation),
        Refused::QueueStorage(refusal) => Class::QueueStorage(refusal),
        Refused::Store(error) => Class::Store(error),
    }
}

/// The engine's refusals, in the disposal's classes.
fn engine_class(error: EngineError) -> Class {
    match error {
        EngineError::Store(error) => store_class(error),
        EngineError::World(world) => world_class(world),
        EngineError::BoundaryMismatch { .. }
        | EngineError::CapabilityMismatch { .. }
        | EngineError::CapabilityInputMoved { .. }
        | EngineError::CharterMoved { .. }
        | EngineError::UnbuiltBoundary { .. } => Class::Counted(StartRefusal::Compile),
        error @ (EngineError::Entry(_)
        | EngineError::EntryRecord(_)
        | EngineError::RunLock(_)
        | EngineError::Fold(_)
        | EngineError::ManifestMismatch { .. }
        | EngineError::ConcludeRefused { .. }
        | EngineError::AlreadyConcluded { .. }
        | EngineError::ConcludedBeneath { .. }
        | EngineError::SupersedeRefused(_)
        | EngineError::RepairRefused(_)
        | EngineError::SettleRefused(_)
        | EngineError::ClassifyRefused(_)
        | EngineError::CauseRefused(_)
        | EngineError::JournalMoved { .. }
        | EngineError::Invariant(..)
        | EngineError::HeldWait(_)
        | EngineError::Dispatch(_)
        | EngineError::Unreleased(_)
        | EngineError::Unspelled(_)) => Class::Start(LaunchError::Engine(error)),
    }
}

fn silent(_: &str) {}

#[cfg(test)]
pub(crate) mod tests;
