//! The dispatcher's one pass (decision 0110 ruling 1), under the journal's
//! dispatch lock: reconcile what a dispatcher left mid-start, sweep the
//! trees a closed effect leaves, read the journal's bound, judge the queue
//! as the operator word `dispatch`, and start each admitted entry in the
//! queue's order. One entry's claim, start, link and `spawn` row complete
//! before the next entry is claimed (decision 0095 ruling 7).
//!
//! What ends the dispatcher is a typed [`Ending`]; everything the next pass
//! may judge again is a [`Passed`] report. Contention and admission's
//! refusals are reported, an unreadable bound claims nothing, a claim its
//! re-check refuses writes nothing, and a sweep record refusal keeps its
//! report. Nothing here prints: the verb paints the report in slice 6b.

use std::ffi::OsString;
use std::num::NonZeroU32;

use brokkr_store::{Attribution, Claim, EntryId, Generation, StartRefusal, Store, StoreError};

use super::lock::Dispatcher;
use super::reconcile::{reconcile, Settled};
use super::start::{self, Spawn, SpawnFailed, Spawned};
use crate::admission::{self, AdmissionError, Host, Judged, Reason, Standing};
use crate::launch::LaunchError;
use crate::sweep::{self, SweepError, Swept};

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
/// (decision 0110 ruling 2).
#[derive(Debug, thiserror::Error)]
pub enum Ending {
    /// The journal or the sweep refused the dispatcher itself: it took
    /// another journal, or a step was not its to take.
    #[error(transparent)]
    Dispatcher(#[from] super::DispatcherError),
    /// `stands` ended the dispatcher: the binary it would spawn is not the
    /// one it started from, or a signal asked it to stop.
    #[error("dispatch: the dispatcher no longer stands")]
    Stood,
    /// A newer dispatcher holds the journal: nothing after was written.
    #[error("dispatch: the dispatcher was fenced out by generation {}", .0.0)]
    Fenced(Generation),
    /// A start refused outside decision 0095's counted and uncounted
    /// classes: nothing was written and the entry stays `launching`.
    #[error(transparent)]
    Start(#[from] LaunchError),
    /// The journal's queue storage is not one this binary writes.
    #[error(transparent)]
    QueueStorage(#[from] brokkr_store::QueueRefusal),
    /// Any other store error that is not `Contended`.
    #[error(transparent)]
    Store(#[from] StoreError),
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
}

/// An entry a counted refusal held at the journal's bound.
#[derive(Debug)]
pub struct Held {
    pub claim: Claim,
    pub refusal: StartRefusal,
    pub count: u32,
}

/// A step `Contended` past its patience: reported, and the pass claims no
/// further entry.
#[derive(Debug)]
pub struct Contended {
    pub entry: Option<EntryId>,
    pub claim: Option<Claim>,
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
    pub swept: Option<Swept>,
    pub sweep_refused: Option<SweepRefused>,
    pub bound: Option<NonZeroU32>,
    pub bound_refused: Option<StoreError>,
    pub admission_refused: Option<AdmissionError>,
    pub unstarted: Vec<Reason>,
    pub started: Vec<Started>,
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
/// it did.
pub fn pass<S, P>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: OsString,
    seams: Seams<S, P>,
) -> Result<Passed, Ending>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
{
    pass_with(store, dispatcher, host, host_path, seams, |_| {})
}

/// [`pass`], with a seam that records each step's order: the test that
/// binds ruling 1's fixed order, private so no production consumer carries
/// an observer.
fn pass_with<S, P>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: OsString,
    seams: Seams<S, P>,
    mut observe: impl FnMut(Stage),
) -> Result<Passed, Ending>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
{
    let mut seams = seams;
    let mut passed = Passed::default();
    observe(Stage::Reconcile);
    passed.reconciled = match reconcile(store, dispatcher) {
        Ok(Ok(settled)) => settled,
        Ok(Err(error)) => return Err(Ending::Dispatcher(error)),
        Err(error) => {
            store_refused(&mut passed, error, Stage::Reconcile)?;
            return Ok(passed);
        }
    };
    if !spawned_after_reconcile(store, dispatcher, &mut passed, &mut seams)? {
        return Ok(passed);
    }
    // 0106d spends an entry's retry budget here.
    observe(Stage::Sweep);
    sweep_once(store, dispatcher, &mut passed)?;
    observe(Stage::Bound);
    match store.dispatch_bound() {
        Ok(bound) => passed.bound = Some(bound),
        Err(error) => {
            passed.bound_refused = Some(error);
            return Ok(passed);
        }
    }
    let bound = passed.bound.expect("a read bound is kept");
    observe(Stage::Judge);
    let reason = format!(
        "dispatch generation {} judges the queue",
        dispatcher.generation.0
    );
    let by = Attribution {
        operator: "dispatch",
        reason: &reason,
    };
    let (judged, unstarted) = match admission::judge_within(store, host, by) {
        Ok(judged) => judged,
        Err(AdmissionError::Store(error)) => {
            return stop_with_refusal(&mut passed, error, Stage::Judge);
        }
        Err(error) => {
            passed.admission_refused = Some(error);
            return Ok(passed);
        }
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
        let outcome = start::start(
            store,
            dispatcher,
            admitted,
            bound,
            host_path.clone(),
            &mut seams,
        )?;
        if !record_start(&mut passed, outcome) {
            break;
        }
    }
    Ok(passed)
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
/// hand the run to the seam once, never twice (decision 0095 ruling 3).
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
    for settled in settled {
        match settled {
            Settled::Reclaimed(claim) => passed.reconciled.push(Settled::Reclaimed(claim)),
            Settled::ToSpawn(claim, run) => {
                (seams.stands)()?;
                if let Err(error) =
                    store.queue_spawn(claim, generation, &reconciled_spawn(claim, &run))
                {
                    passed.reconciled.push(Settled::ToSpawn(claim, run));
                    return Ok(!store_refused(passed, error, Stage::Spawn)?);
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

/// Run the sweep, keeping a record refusal's report and ending on a store
/// or dispatcher refusal.
fn sweep_once(
    store: &mut Store,
    dispatcher: &Dispatcher,
    passed: &mut Passed,
) -> Result<(), Ending> {
    record_sweep(passed, sweep::pass(store, dispatcher)).map(|_| ())
}

/// Keep what a sweep did, or what it refused: a record refusal is reported
/// beside the removals it made, and a store or dispatcher refusal ends the
/// pass.
fn record_sweep(passed: &mut Passed, swept: Result<Swept, SweepError>) -> Result<bool, Ending> {
    match swept {
        Ok(swept) => passed.swept = Some(swept),
        Err(SweepError::Record { report, cause }) => {
            passed.sweep_refused = Some(SweepRefused {
                report: *report,
                cause,
            });
        }
        Err(SweepError::Dispatcher(error)) => return Err(Ending::Dispatcher(error)),
        Err(SweepError::Store(error)) => return Ok(!store_refused(passed, error, Stage::Sweep)?),
    }
    Ok(true)
}

/// Stop the pass with a store refusal in `passed`: contention is reported
/// and the pass returns, and every other class ends the dispatcher.
fn stop_with_refusal(
    passed: &mut Passed,
    error: StoreError,
    stage: Stage,
) -> Result<Passed, Ending> {
    store_refused(passed, error, stage)?;
    Ok(std::mem::take(passed))
}

/// Report a store refusal, or end the dispatcher: contention is reported
/// and claims nothing more; a newer holder, unknown queue storage and every
/// other store error end it.
fn store_refused(passed: &mut Passed, error: StoreError, stage: Stage) -> Result<bool, Ending> {
    match start::refused(error) {
        start::Refused::Contended(error) => {
            passed.contended.push(Contended {
                entry: None,
                claim: None,
                stage,
                error,
            });
            Ok(true)
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

/// The `spawn` row's reason for a run reconciliation hands back.
fn reconciled_spawn(claim: Claim, run: &str) -> String {
    let (entry, seq) = claim.parts();
    format!("dispatch: run '{run}' under claim {seq} of entry {entry} is handed to its driver")
}

#[cfg(test)]
pub(crate) mod tests;
