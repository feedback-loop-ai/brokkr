//! The standing loop (decision 0110 ruling 5): `brokkr dispatch` repeats
//! its pass every interval while it stands. Each tick runs one pass, which
//! reconciles first (decision 0095 ruling 3), hands the pass's report to a
//! seam, reaps each driver that has exited and reports it with what its run
//! reads now, then sleeps what is left of the interval.
//!
//! The loop ends on a pass's [`Ended`], whose [`Ending`] the verb exits
//! non-zero on, on a tick's report its seam could not write, or on a stop
//! its seam reports, after which no claim
//! follows. Everything else a pass meets is reported and judged again on
//! the next tick, because none of it latches (decision 0068's 2026-10-10
//! addendum). The loop never resumes, adopts, stops or concludes a run
//! (0068 ruling 7): a driver whose run still reads `running` is reported
//! for the operator's `resume`.
//!
//! Nothing here reads a clock, waits on a signal or holds a process: the
//! time, the sleep, the stop, the drivers' handles and the printing are
//! seams the verb supplies (decision 0071 ruling 1).

use std::ffi::OsString;
use std::io;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use brokkr_core::fold::{fold, Status};
use brokkr_store::{Claim, Store};

use super::lock::Dispatcher;
use super::pass::{pass, Ended, Ending, Passed, Seams};
use super::start::{Spawn, SpawnFailed, Spawned};
use crate::admission::Host;
use crate::EngineError;

/// A driver the dispatcher spawned that has exited, as its reap seam found
/// it without waiting: its run, the claim it was handed, its pid and how it
/// exited, or why its handle could not say, as when the system reaped it
/// first because SIGCHLD was inherited ignored.
#[derive(Debug)]
pub struct Exited {
    pub run: String,
    pub claim: Claim,
    pub pid: u32,
    pub exit: io::Result<ExitStatus>,
}

/// An exited driver as a tick reports it, beside what its run reads now:
/// a run that still reads `Running` has no driver, and needs the
/// operator's `resume`.
#[derive(Debug)]
pub struct Reaped {
    pub exited: Exited,
    pub reads: Result<Status, EngineError>,
}

/// What a tick hands its report seam, in order: the pass's report, then
/// each driver it reaped.
#[derive(Debug)]
pub enum Tick<'a> {
    Passed(&'a Passed),
    Reaped(&'a Reaped),
}

/// The loop ended on a stop its seam reported: the pass in flight claimed
/// nothing more, and every entry it started has its `spawn` row and driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stopped;

/// The seams the loop runs through, a parameter object (decision 0071
/// ruling 10). `stands` and `spawn` are each pass's (ruling 4's check and
/// the driver's start); `stopped` answers whether a stop was asked, which
/// also keeps a pass from claiming; `now` and `sleep` are the loop's only
/// clock; `reap` hands back each driver that exited, without waiting; and
/// `report` writes each tick's facts, a write that fails ending the loop
/// with [`Ending::Unreported`], since a driver's exit and a resume command
/// are printed nowhere else.
pub struct Stand<'a> {
    pub every: Duration,
    pub stands: &'a mut dyn FnMut() -> Result<(), Ending>,
    pub spawn: &'a mut dyn FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
    pub stopped: &'a dyn Fn() -> bool,
    pub now: &'a mut dyn FnMut() -> Instant,
    pub sleep: &'a mut dyn FnMut(Duration),
    pub reap: &'a mut dyn FnMut() -> Vec<Exited>,
    pub report: &'a mut dyn FnMut(Tick<'_>) -> io::Result<()>,
}

/// Stand over the journal `dispatcher` holds, with the machine `host`
/// names and the boundary tool search path `host_path`: a tick every
/// `stand.every`, measured from when its pass began, until a stop or a
/// pass's ending.
pub fn stand(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: &OsString,
    mut stand: Stand<'_>,
) -> Result<Stopped, Box<Ended>> {
    while !(stand.stopped)() {
        let began = (stand.now)();
        tick(store, dispatcher, host, host_path, &mut stand)?;
        if !(stand.stopped)() {
            let spent = (stand.now)().saturating_duration_since(began);
            let rest = stand.every.saturating_sub(spent);
            if !rest.is_zero() {
                (stand.sleep)(rest);
            }
        }
    }
    // A driver that exited during the last sleep is reported as exited,
    // not left running.
    reap(store, &mut stand).map_err(|error| unreported(error, Passed::default()))?;
    Ok(Stopped)
}

/// The loop ended because `error` kept a tick's report from being
/// written, after the pass that reported `passed`.
fn unreported(error: io::Error, passed: Passed) -> Box<Ended> {
    let ending = Ending::Unreported(error);
    Box::new(Ended { ending, passed })
}

/// One tick: the pass, its report, then each reaped driver's. A pass that
/// ended is reported, its drivers reaped, and its ending handed back. A
/// report that could not be written reaps nothing more, so each driver not
/// yet reaped is named as left running, and ends the loop.
fn tick(
    store: &mut Store,
    dispatcher: &Dispatcher,
    host: &Host<'_>,
    host_path: &OsString,
    stand: &mut Stand<'_>,
) -> Result<(), Box<Ended>> {
    let stopped = stand.stopped;
    let seams = Seams {
        stands: &mut *stand.stands,
        spawn: &mut *stand.spawn,
        claims: || !stopped(),
    };
    let passed = pass(store, dispatcher, host, host_path.clone(), seams);
    let report = match &passed {
        Ok(passed) => passed,
        Err(ended) => &ended.passed,
    };
    let written = (stand.report)(Tick::Passed(report)).and_then(|()| reap(store, stand));
    match (passed, written) {
        (Err(ended), _) => Err(ended),
        (Ok(passed), Err(error)) => Err(unreported(error, passed)),
        (Ok(_), Ok(())) => Ok(()),
    }
}

/// Report each driver that has exited, beside what its run reads now.
fn reap(store: &Store, stand: &mut Stand<'_>) -> io::Result<()> {
    for exited in (stand.reap)() {
        let reads = reads(store, &exited.run);
        (stand.report)(Tick::Reaped(&Reaped { exited, reads }))?;
    }
    Ok(())
}

/// What `run` reads now in the journal, folded from its events.
fn reads(store: &Store, run: &str) -> Result<Status, EngineError> {
    Ok(fold(&store.load(run)?)?.run.status())
}

#[cfg(test)]
mod tests;
