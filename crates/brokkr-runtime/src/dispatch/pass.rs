//! The dispatcher's one pass (decision 0110 ruling 1), under the journal's
//! dispatch lock: reconcile what a dispatcher left mid-start, sweep the
//! trees a closed effect leaves, read the journal's bound, judge the queue
//! as the operator word `dispatch`, and start each admitted entry in the
//! queue's order. One entry's claim, start, link and `spawn` row complete
//! before the next entry is claimed (decision 0095 ruling 7).
//!
//! What ends the dispatcher is a typed [`Ending`], carried by the [`Ended`]
//! that also holds the [`Passed`] report of everything the pass did before
//! it ended, so 6c's loop never matches on text and never loses a spawn
//! failure. Everything the next pass may judge again is a [`Passed`] report.
//! Contention and admission's refusals are reported, an unreadable bound
//! claims nothing, a claim its re-check refuses writes nothing, and a sweep
//! record refusal keeps its report. Nothing here prints: the verb paints
//! the report in slice 6b.

use std::ffi::OsString;
use std::num::NonZeroU32;

use brokkr_store::{Attribution, Claim, EntryId, Generation, StartRefusal, Store, StoreError};

use super::lock::Dispatcher;
use super::reconcile::{reconcile, Settled};
use super::start::{self, Spawn, SpawnFailed, Spawned};
use crate::admission::{self, AdmissionError, Host, Judged, Reason, Standing};
use crate::launch::LaunchError;
use crate::sweep::{self, SweepError, Swept};

/// The queue admission judged, and the reasons no entry waits on.
type JudgedQueue = (Vec<Judged>, Vec<Reason>);

/// The two seams a pass is handed in place of a process and a binary check
/// (decision 0110 ruling 1): `stands` is ruling 4's check, called before
/// each claim and before each `spawn` row, and `spawn` starts one run once
/// its `spawn` row has landed.
pub struct Seams<S, P> {
    pub stands: S,
    pub spawn: P,
}

/// A step of the pass, as a test's seam records its order and a report
/// names where a contention stood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Reconcile,
    Sweep,
    Bound,
    Judge,
    Start,
    Claim,
    Link,
    Spawn,
}

/// Why the dispatcher ended, typed apart from what its pass reported
/// (decision 0110 ruling 2). [`Ended`] pairs it with the report.
#[derive(Debug, thiserror::Error)]
pub enum Ending {
    /// The journal or the sweep refused the dispatcher itself: it took
    /// another journal, or a step was not its to take.
    #[error(transparent)]
    Dispatcher(super::DispatcherError),
    /// `stands` ended the dispatcher: a signal asked it to stop.
    #[error("dispatch: the dispatcher no longer stands")]
    Stood,
    /// The brokkr the dispatcher was started from is no longer the file at
    /// its path (decision 0110 ruling 4): nothing more was claimed.
    #[error(
        "dispatch: the brokkr at {} is not the one this dispatcher started from; it claimed \
         nothing after; start the dispatcher again",
        .0.display()
    )]
    Replaced(std::path::PathBuf),
    /// A newer dispatcher holds the journal: nothing after was written.
    /// The generation named is the stale one this dispatcher held.
    #[error(
        "dispatch: generation {} was fenced out; it is not the journal's latest holder",
        .0.0
    )]
    Fenced(Generation),
    /// A start refused outside decision 0095's counted and uncounted
    /// classes: nothing was written and the entry stays `launching`.
    #[error(transparent)]
    Start(LaunchError),
    /// The journal's queue storage is not one this binary writes.
    #[error(transparent)]
    QueueStorage(brokkr_store::QueueRefusal),
    /// Any other store error that is not `Contended`.
    #[error(transparent)]
    Store(StoreError),
}

/// A pass that ended: the typed [`Ending`] and the [`Passed`] report of
/// everything it did before it ended. The report is never dropped, so a
/// spawn failure, a started pid, a hold or a reclaim the pass already saw
/// reaches 6b even where a later step ends the dispatcher.
#[derive(Debug, thiserror::Error)]
#[error("{ending}")]
pub struct Ended {
    pub ending: Ending,
    pub passed: Passed,
}

/// What a contention stood on: the pass itself, one entry's claim, or one
/// entry's linked claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Pass,
    Entry(EntryId),
    Claim(EntryId, Claim),
}

/// How reading the journal's bound went: the bound read, or the store's
/// refusal that left it unreadable, which claims nothing this pass.
#[derive(Debug)]
pub enum BoundOutcome {
    Read(NonZeroU32),
    Unreadable(StoreError),
}

/// What one sweep did: its report, or its record refusal's report and
/// cause.
#[derive(Debug)]
pub enum SweepOutcome {
    Swept(Swept),
    Refused(SweepRefused),
}

/// One run reconciliation handed back to spawn: its claim, its run and the
/// seam's answer.
#[derive(Debug)]
pub struct Spawning {
    pub claim: Claim,
    pub run: String,
    pub spawned: Result<Spawned, SpawnFailed>,
}

/// One entry this pass started: its entry, claim and run, with the spawn
/// seam's answer.
#[derive(Debug)]
pub struct Started {
    pub entry: EntryId,
    pub claim: Claim,
    pub run: String,
    pub spawned: Result<Spawned, SpawnFailed>,
}

/// An entry a refused start returned to its place.
#[derive(Debug)]
pub struct Reclaimed {
    pub claim: Claim,
    /// The counted refusal this disposal carried, `None` for an uncounted
    /// one.
    pub refused: Option<StartRefusal>,
    /// The entry's count of counted refusals since its latest release.
    pub count: u32,
    /// The reason journaled: the class word and the start's own words.
    pub reason: String,
}

/// An entry a counted refusal held at the journal's bound, with the reason
/// journaled: the class word and the start's own words.
#[derive(Debug)]
pub struct Held {
    pub claim: Claim,
    pub refusal: StartRefusal,
    pub count: u32,
    pub reason: String,
}

/// A run this pass started and did not hand to its driver: its link or its
/// `spawn` row did not land, or the dispatcher ended between them. The
/// next pass's reconciliation links and spawns it (decision 0095 ruling 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Left {
    pub claim: Claim,
    pub run: String,
}

/// A step `Contended` past its patience: reported. A contention on an
/// entry's claim, link or `spawn` row leaves no further entry claimed; a
/// pass-level one (reconcile, sweep, bound or judge) is reported and the
/// pass goes on where it can.
#[derive(Debug)]
pub struct Contended {
    pub subject: Subject,
    pub stage: Stage,
    pub error: StoreError,
}

/// A sweep that removed trees but could not record them: its report is
/// kept beside the refusal's words.
#[derive(Debug)]
pub struct SweepRefused {
    pub report: Swept,
    pub cause: String,
}

/// What one pass did. Nothing here is printed: the verb paints it (slice
/// 6b), and the next pass judges everything the report names again.
#[derive(Debug, Default)]
pub struct Passed {
    pub reconciled: Vec<Settled>,
    pub sweep: Option<SweepOutcome>,
    pub bound: Option<BoundOutcome>,
    pub admission_refused: Option<AdmissionError>,
    pub unstarted: Vec<Reason>,
    pub started: Vec<Started>,
    pub left: Vec<Left>,
    pub spawned: Vec<Spawning>,
    pub reclaimed: Vec<Reclaimed>,
    pub held: Vec<Held>,
    pub claims_refused: Vec<start::ClaimRefused>,
    pub contended: Vec<Contended>,
    /// Entries admission still judged waiting, and held, when the pass read
    /// the queue.
    pub waiting: usize,
    pub held_entries: usize,
}

/// Run one pass under the dispatcher's lock, with the machine `host` names
/// and the boundary tool search path `host_path`: reconcile, sweep, read
/// the bound, judge the queue and start each admitted entry, reporting what
/// it did. A typed [`Ending`] carries the report [`Ended::passed`].
pub fn pass<S, P>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: OsString,
    seams: Seams<S, P>,
) -> Result<Passed, Box<Ended>>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
{
    let watch = Watch {
        observe: |_| {},
        launch: crate::launch::start,
    };
    pass_with(store, dispatcher, host, host_path, seams, watch)
}

/// The private seams a test hands [`pass_with`]: `observe` records each
/// step's order, and `launch` starts each run, a fault in its place where
/// a test stands a refused start at the pass's own step.
struct Watch<O> {
    observe: O,
    launch: start::Launch,
}

/// [`pass`], with the private seams of [`Watch`]: the tests that bind
/// ruling 1's fixed order and ruling 2's disposals, private so no
/// production consumer carries an observer or a fault.
fn pass_with<S, P, O>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: OsString,
    seams: Seams<S, P>,
    mut watch: Watch<O>,
) -> Result<Passed, Box<Ended>>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
    O: FnMut(Stage),
{
    let mut passed = Passed::default();
    match run(
        store,
        dispatcher,
        host,
        host_path,
        seams,
        &mut passed,
        &mut watch,
    ) {
        Ok(()) => Ok(passed),
        Err(ending) => Err(Box::new(Ended { ending, passed })),
    }
}

/// The pass's steps, keeping everything they did in `passed` so an ending
/// can carry it.
fn run<S, P, O>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: OsString,
    mut seams: Seams<S, P>,
    passed: &mut Passed,
    watch: &mut Watch<O>,
) -> Result<(), Ending>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
    O: FnMut(Stage),
{
    let observe = &mut watch.observe;
    observe(Stage::Reconcile);
    if !reconcile_once(store, dispatcher, passed)? {
        return Ok(());
    }
    if !spawned_after_reconcile(store, dispatcher, passed, &mut seams)? {
        return Ok(());
    }
    // 0106d spends an entry's retry budget here.
    observe(Stage::Sweep);
    sweep_once(store, dispatcher, passed)?;
    observe(Stage::Bound);
    let Some(bound) = read_bound(store, passed)? else {
        return Ok(());
    };
    observe(Stage::Judge);
    let Some((judged, unstarted)) = judge_queue(store, dispatcher, host, passed)? else {
        return Ok(());
    };
    passed.unstarted = unstarted;
    passed.waiting = judged
        .iter()
        .filter(|judged| standing(judged) == Some(Standing::Waiting))
        .count();
    passed.held_entries = judged
        .iter()
        .filter(|judged| standing(judged) == Some(Standing::Held))
        .count();
    let shared = start::Shared {
        bound,
        host_path: &host_path,
        launch: watch.launch,
    };
    for judged in judged {
        let Judged {
            entry,
            launch,
            room,
            ..
        } = judged;
        let Some(room) = room else {
            continue;
        };
        observe(Stage::Start);
        let admitted = start::Admitted {
            entry,
            launch,
            room,
        };
        let left = &mut passed.left;
        let outcome = start::start(store, dispatcher, admitted, &shared, &mut seams, left)?;
        if !record_start(passed, outcome) {
            break;
        }
    }
    Ok(())
}

/// Reconcile what a dispatcher left mid-start. A contention is reported and
/// claims nothing more this pass; a dispatcher or store refusal ends it.
fn reconcile_once(
    store: &mut Store,
    dispatcher: &Dispatcher,
    passed: &mut Passed,
) -> Result<bool, Ending> {
    match reconcile(store, dispatcher) {
        Ok(Ok(settled)) => {
            passed.reconciled = settled;
            Ok(true)
        }
        Ok(Err(error)) => Err(Ending::Dispatcher(error)),
        Err(error) => {
            store_refused(passed, error, Stage::Reconcile, Subject::Pass)?;
            Ok(false)
        }
    }
}

/// Take one entry's start into the report. A contention or a claim that
/// could not be judged stops the pass's claims; every other outcome lets
/// the next entry be judged.
fn record_start(passed: &mut Passed, outcome: start::Outcome) -> bool {
    match outcome {
        start::Outcome::Started(started) => passed.started.push(started),
        start::Outcome::ClaimRefused(refused) => passed.claims_refused.push(refused),
        start::Outcome::Reclaimed(reclaimed) => passed.reclaimed.push(reclaimed),
        start::Outcome::Held(held) => passed.held.push(held),
        start::Outcome::Contended(contended) => {
            passed.contended.push(contended);
            return false;
        }
        start::Outcome::Admission(error) => {
            passed.admission_refused = Some(error);
            return false;
        }
    }
    true
}

/// The runs reconciliation handed back to spawn: write each `spawn` row and
/// hand the run to the seam once, never twice (decision 0095 ruling 3). A
/// store refusal is reported or ends the pass, and every settled item is
/// kept in the report for the next pass.
fn spawned_after_reconcile<S, P>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    passed: &mut Passed,
    seams: &mut Seams<S, P>,
) -> Result<bool, Ending>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
{
    let generation = dispatcher.generation;
    let settled = std::mem::take(&mut passed.reconciled);
    let mut pending = settled.into_iter();
    while let Some(settled) = pending.next() {
        match settled {
            Settled::Reclaimed(claim) => passed.reconciled.push(Settled::Reclaimed(claim)),
            Settled::ToSpawn(claim, run) => {
                // A stand that ends keeps this item and every later one,
                // as a store refusal below does: `Ended` never drops them.
                if let Err(ending) = (seams.stands)() {
                    passed.reconciled.push(Settled::ToSpawn(claim, run));
                    passed.reconciled.extend(pending);
                    return Err(ending);
                }
                if let Err(error) =
                    store.queue_spawn(claim, generation, &start::spawn_reason(claim, &run))
                {
                    passed.reconciled.push(Settled::ToSpawn(claim, run));
                    passed.reconciled.extend(pending);
                    store_refused(
                        passed,
                        error,
                        Stage::Spawn,
                        Subject::Claim(claim.parts().0, claim),
                    )?;
                    return Ok(false);
                }
                let spawned = (seams.spawn)(&Spawn {
                    entry: claim.parts().0,
                    claim,
                    generation,
                    run: run.clone(),
                });
                passed.spawned.push(Spawning {
                    claim,
                    run: run.clone(),
                    spawned,
                });
                passed.reconciled.push(Settled::ToSpawn(claim, run));
            }
        }
    }
    Ok(true)
}

/// Run the sweep. Its record refusal keeps the report and the pass goes on;
/// a contended store refusal is reported and the pass goes on; any other
/// store or dispatcher refusal ends it.
fn sweep_once(
    store: &mut Store,
    dispatcher: &Dispatcher,
    passed: &mut Passed,
) -> Result<(), Ending> {
    record_sweep(passed, sweep::pass(store, dispatcher))
}

/// Keep a sweep's report or its record refusal, and split a store refusal:
/// contention is reported and the pass goes on, and every other store error
/// ends the dispatcher.
fn record_sweep(passed: &mut Passed, swept: Result<Swept, SweepError>) -> Result<(), Ending> {
    match swept {
        Ok(swept) => passed.sweep = Some(SweepOutcome::Swept(swept)),
        Err(SweepError::Record { report, cause }) => {
            passed.sweep = Some(SweepOutcome::Refused(SweepRefused {
                report: *report,
                cause,
            }));
        }
        Err(SweepError::Dispatcher(error)) => return Err(Ending::Dispatcher(error)),
        Err(SweepError::Store(error)) => {
            store_refused(passed, error, Stage::Sweep, Subject::Pass)?;
        }
    }
    Ok(())
}

/// Read the journal's bound once. An unreadable bound is reported and
/// claims nothing this pass, and a contention is reported and does the
/// same; a newer holder or unknown queue storage ends the dispatcher.
fn read_bound(store: &mut Store, passed: &mut Passed) -> Result<Option<NonZeroU32>, Ending> {
    match store.dispatch_bound() {
        Ok(bound) => {
            passed.bound = Some(BoundOutcome::Read(bound));
            Ok(Some(bound))
        }
        Err(error) => bound_refused(passed, error),
    }
}

/// Split the store's refusal of a bound read: contention is reported and
/// claims nothing, a newer holder or unknown queue storage ends the
/// dispatcher, and a bound that does not read is reported as unreadable,
/// claiming nothing.
fn bound_refused(passed: &mut Passed, error: StoreError) -> Result<Option<NonZeroU32>, Ending> {
    match start::refused(error) {
        start::Refused::Contended(error) => {
            passed.contended.push(Contended {
                subject: Subject::Pass,
                stage: Stage::Bound,
                error,
            });
            Ok(None)
        }
        start::Refused::Fenced(generation) => Err(Ending::Fenced(generation)),
        start::Refused::QueueStorage(refusal) => Err(Ending::QueueStorage(refusal)),
        start::Refused::Store(error) => {
            passed.bound = Some(BoundOutcome::Unreadable(error));
            Ok(None)
        }
    }
}

/// Judge the queue as the operator word `dispatch`. A store refusal is
/// reported or ends the dispatcher; every other admission refusal is
/// reported and claims nothing this pass.
fn judge_queue(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    passed: &mut Passed,
) -> Result<Option<JudgedQueue>, Ending> {
    let reason = format!(
        "dispatch generation {} judges the queue",
        dispatcher.generation.0
    );
    let by = Attribution {
        operator: "dispatch",
        reason: &reason,
    };
    match admission::judge_within(store, host, by) {
        Ok(judged) => Ok(Some(judged)),
        Err(error) => match start::admission_refused(error) {
            start::AdmissionRefused::Store(error) => {
                store_refused(passed, error, Stage::Judge, Subject::Pass)?;
                Ok(None)
            }
            start::AdmissionRefused::Other(error) => {
                passed.admission_refused = Some(error);
                Ok(None)
            }
        },
    }
}

/// Report a store refusal, or end the dispatcher: contention is reported
/// and claims nothing more; a newer holder, unknown queue storage and every
/// other store error end it.
fn store_refused(
    passed: &mut Passed,
    error: StoreError,
    stage: Stage,
    subject: Subject,
) -> Result<(), Ending> {
    match start::refused(error) {
        start::Refused::Contended(error) => {
            passed.contended.push(Contended {
                subject,
                stage,
                error,
            });
            Ok(())
        }
        start::Refused::Fenced(generation) => Err(Ending::Fenced(generation)),
        start::Refused::QueueStorage(refusal) => Err(Ending::QueueStorage(refusal)),
        start::Refused::Store(error) => Err(Ending::Store(error)),
    }
}

/// The standing admission found for `judged`, `None` once it is claimed or
/// started.
fn standing(judged: &Judged) -> Option<Standing> {
    judged.verdict.as_ref().map(|verdict| verdict.standing())
}

#[cfg(test)]
pub(crate) mod tests;
