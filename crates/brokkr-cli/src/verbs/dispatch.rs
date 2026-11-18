//! `brokkr dispatch` (decision 0110 rulings 3 to 6): the standing loop,
//! or one pass with `--once`, under the journal's dispatch lock, each run
//! it starts handed to this binary as `dispatch drive`, a process in a
//! session of its own; the driver itself; the bound's command; and the
//! holder's readout. The pass, the loop's logic, the binary's identity, the
//! driver's log and the driver's checks are brokkr-runtime's; this handler
//! resolves the journal, starts each driver's process, which needs one
//! call between fork and exec, keeps each driver's handle for the loop to
//! reap, supplies the loop's clock and its stop on SIGINT or SIGTERM, and
//! paints each line through [`Safe`](crate::render::Safe).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use brokkr_runtime::argv::Executable;
use brokkr_runtime::dispatch::{
    self, Binary, Dispatcher, Exited, Seams, Spawn, SpawnFailed, Spawned, Stand, Tick,
};
use brokkr_runtime::{launch, Engine};
use brokkr_store::{Attribution, Claim, EntryId, Store};
use signal_hook::consts::{SIGINT, SIGTERM};

use super::delivery::operator_name;
use super::queue::on_this_host;
use crate::cli_args::{
    DispatchArgs, DispatchCmd, DriveArgs, JournalArgs, SetBoundArgs, StatusArgs,
};
use crate::{drive_to_end, open_journal, Access, Exit};

mod report;

/// The signals that stop the standing loop (decision 0110 ruling 5).
const STOPPING: [i32; 2] = [SIGINT, SIGTERM];

/// The longest a sleep goes without looking at the stop.
const BEAT: Duration = Duration::from_millis(50);

/// Run one `brokkr dispatch` command.
pub(crate) fn dispatch(
    workspace: &Path,
    args: DispatchArgs,
    out: &mut impl Write,
) -> Result<ExitCode> {
    match args.command {
        Some(DispatchCmd::Drive(args)) => drive(workspace, args),
        Some(DispatchCmd::SetBound(args)) => set_bound(workspace, args, out),
        Some(DispatchCmd::Status(args)) => status(workspace, args, out),
        None if args.once => once(workspace, args, out),
        None => standing(workspace, args, out),
    }
}

/// The boundary tool's search path every run this process starts reads,
/// as `brokkr run` hands it.
fn host_path() -> OsString {
    std::env::var_os("PATH").unwrap_or_default()
}

/// `brokkr dispatch --once`: take the journal's lock, run one pass whose
/// seams are this binary's identity and its driver's spawn, and paint the
/// pass's report. An ending exits non-zero.
fn once(workspace: &Path, args: DispatchArgs, out: &mut impl Write) -> Result<ExitCode> {
    let (journal, binary, mut store, held) = taken(workspace, args.journal)?;
    let seams = Seams {
        stands: || binary.stands(),
        spawn: |spawn: &Spawn| driver(&binary, &journal, spawn).map(|child| spawned(&child)),
        claims: || true,
    };
    let passed =
        on_this_host(|host| Ok(dispatch::pass(&mut store, &held, host, host_path(), seams)))?;
    let (lines, exit) = match passed {
        Ok(passed) => (report::passed(&passed, &journal), Exit::Completed),
        Err(ended) => (report::ended(&ended, &journal), Exit::Failed),
    };
    out.write_all(lines.as_bytes())?;
    Ok(exit.into())
}

/// The journal named whole, since a driver runs in its entry's workspace;
/// this binary's identity, recorded before anything else is read; and the
/// journal's store with its dispatch lock taken.
fn taken(workspace: &Path, journal: JournalArgs) -> Result<(PathBuf, Binary, Store, Dispatcher)> {
    let journal = std::path::absolute(journal.journal(workspace)?)?;
    let binary = Binary::started(&Executable::current())?;
    let mut store = open_journal(&journal, Access::Append)?;
    match dispatch::take_dispatch(&mut store)? {
        Ok(held) => Ok((journal, binary, store, held)),
        Err(refused) => Err(anyhow!(refused)),
    }
}

/// The drivers the standing loop spawned and has not reaped: each one's
/// run, claim and handle, by its pid.
type Drivers = BTreeMap<u32, (String, Claim, Child)>;

/// `brokkr dispatch`: the standing loop (decision 0110 ruling 5), a pass
/// every `--every` seconds, each tick's report painted as `--once` paints
/// it, then a line per driver reaped. The first SIGINT or SIGTERM stops it
/// once the entry in flight has its driver, and it exits zero; a second
/// ends the process at once. The drivers still running are named, never
/// signalled, and the ending's words come last.
fn standing(workspace: &Path, args: DispatchArgs, out: &mut impl Write) -> Result<ExitCode> {
    let stop = stopping()?;
    let every = Duration::from_secs(args.every);
    let (journal, binary, mut store, held) = taken(workspace, args.journal)?;
    let drivers = RefCell::new(Drivers::new());
    let mut spawn = |spawn: &Spawn| {
        let child = driver(&binary, &journal, spawn)?;
        let started = spawned(&child);
        drivers
            .borrow_mut()
            .insert(started.pid, (spawn.run.clone(), spawn.claim, child));
        Ok(started)
    };
    let mut report = |tick: Tick<'_>| {
        let lines = match tick {
            Tick::Passed(passed) => report::passed(passed, &journal),
            Tick::Reaped(reaped) => report::reaped(reaped, &journal),
        };
        // An unwritable stdout ends nothing: the journal holds every fact.
        out.write_all(lines.as_bytes()).ok();
    };
    let stand = Stand {
        every,
        stands: &mut || binary.stands(),
        spawn: &mut spawn,
        stopped: &|| stop.load(Ordering::SeqCst),
        now: &mut Instant::now,
        sleep: &mut |rest| nap(&stop, rest),
        reap: &mut || reaped(&mut drivers.borrow_mut()),
        report: &mut report,
    };
    let path = host_path();
    let stood = on_this_host(|host| Ok(dispatch::stand(&mut store, &held, host, &path, stand)))?;
    let mut lines = report::running(&drivers.into_inner());
    let exit = match stood {
        Ok(dispatch::Stopped) => {
            lines.push_str(&report::stopped());
            Exit::Completed
        }
        Err(ended) => {
            lines.push_str(&report::ending(&ended.ending));
            Exit::Failed
        }
    };
    out.write_all(lines.as_bytes())?;
    Ok(exit.into())
}

/// The stop the first of [`STOPPING`]'s signals asks, its handler setting
/// a flag and nothing else. A second while it is asked ends the process at
/// once, exiting 128 plus the signal's number: a crash, which the next
/// dispatcher's reconciliation settles (decision 0095 ruling 3).
fn stopping() -> Result<Arc<AtomicBool>> {
    let stop = Arc::new(AtomicBool::new(false));
    for signal in STOPPING {
        signal_hook::flag::register_conditional_shutdown(signal, 128 + signal, Arc::clone(&stop))?;
        signal_hook::flag::register(signal, Arc::clone(&stop))?;
    }
    Ok(stop)
}

/// Sleep `rest`, or less once `stop` is asked: the flag is read at least
/// every [`BEAT`].
fn nap(stop: &AtomicBool, rest: Duration) {
    let until = Instant::now() + rest;
    while !stop.load(Ordering::SeqCst) {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return;
        }
        std::thread::sleep(left.min(BEAT));
    }
}

/// Each driver in `drivers` that has exited, taken out of it without
/// waiting on any. A handle whose state does not read, as when SIGCHLD was
/// inherited ignored and the system reaped the driver itself, is taken out
/// too, with why: its run's journal says whether it needs a resume.
fn reaped(drivers: &mut Drivers) -> Vec<Exited> {
    let mut exited = Vec::new();
    drivers.retain(|pid, (run, claim, child)| {
        let exit = match child.try_wait() {
            Ok(None) => return true,
            Ok(Some(status)) => Ok(status),
            Err(error) => Err(error),
        };
        let (run, claim, pid) = (run.clone(), *claim, *pid);
        exited.push(Exited {
            run,
            claim,
            pid,
            exit,
        });
        false
    });
    exited
}

/// What the pass reads of a driver it started: its pid.
fn spawned(child: &Child) -> Spawned {
    Spawned { pid: child.id() }
}

/// The argv after this binary that drives `spawn`'s run.
fn drive_args(journal: &Path, spawn: &Spawn) -> Vec<OsString> {
    let (entry, seq) = spawn.claim.parts();
    let mut args: Vec<OsString> = ["dispatch", "drive", "--db"].map(OsString::from).into();
    args.push(journal.as_os_str().to_owned());
    args.extend(["--entry".into(), entry.to_string().into()]);
    args.extend(["--claim".into(), seq.to_string().into()]);
    args
}

/// Start `spawn`'s driver (decision 0110 ruling 3): `binary` as `dispatch
/// drive` of the run, in its entry's workspace, in a session of its own,
/// its stdin `/dev/null` and its stdout and stderr the run's log, with this
/// process's environment and no other descriptor of it. A spawn that fails
/// is reported, and its run is never spawned again.
fn driver(binary: &Binary, journal: &Path, spawn: &Spawn) -> Result<Child, SpawnFailed> {
    let log = dispatch::driver_log(journal, &spawn.run).map_err(refused)?;
    let errors = log.try_clone().map_err(refused)?;
    let cwd = dispatch::workspace(journal, spawn.entry).map_err(refused)?;
    let mut command = Command::new(binary.path());
    (command.args(drive_args(journal, spawn)))
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(errors);
    // SAFETY: `lead` runs between fork and exec, and makes one system call
    // and allocates nothing.
    unsafe { command.pre_exec(lead) };
    command.spawn().map_err(refused)
}

/// A driver's spawn refused, in the refusal's own words.
fn refused(error: impl std::fmt::Display) -> SpawnFailed {
    SpawnFailed::Refused(error.to_string())
}

/// Run in each driver between its fork and its exec: lead a session of
/// its own, so a signal to the dispatcher's session or group never reaches
/// it. The driver is not made a subreaper. Async-signal-safe: one system
/// call, and an error that allocates nothing. `tests` plays it in a child
/// that exits normally, since what it covers is lost at the exec.
fn lead() -> std::io::Result<()> {
    rustix::process::setsid()?;
    Ok(())
}

/// `brokkr dispatch drive`: resume the run `entry` is linked to under
/// `claim`, once its `spawn` row has landed, rebuilt from the entry's
/// payload as the start rebuilt it, and drive it as `brokkr resume
/// --no-view` does. Refused, writing nothing, otherwise.
fn drive(
    workspace: &Path,
    DriveArgs {
        journal,
        entry,
        claim,
    }: DriveArgs,
) -> Result<ExitCode> {
    let journal = journal.journal(workspace)?;
    let (request, run) = dispatch::driven(&journal, EntryId(entry), claim, host_path())?;
    let mut engine = launch::resume(request, &run)?;
    drive_to_end(&mut engine, Engine::drive)
}

/// `brokkr dispatch set-bound`: the journal's bound on counted refusals,
/// set as the operator's journaled command.
fn set_bound(
    workspace: &Path,
    SetBoundArgs {
        bound,
        reason,
        journal,
    }: SetBoundArgs,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let operator = operator_name();
    let by = Attribution {
        operator: &operator,
        reason: &reason,
    };
    store.set_dispatch_bound(bound, by)?;
    writeln!(out, "bound: {bound} counted refusals hold an entry")?;
    Ok(Exit::Completed.into())
}

/// `brokkr dispatch status`: the journal opened read-only, its latest
/// holder, whether its lock is held now, its bound and what a dispatcher
/// left unsettled. Writes and migrates nothing.
fn status(
    workspace: &Path,
    StatusArgs { journal }: StatusArgs,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let journal = journal.journal(workspace)?;
    let store = open_journal(&journal, Access::Read)?;
    let holder = store.dispatch_holder()?;
    let held = dispatch::dispatched(&journal);
    let bound = store.dispatch_bound()?;
    let unspawned = store.queue_unspawned()?;
    let lines = report::status(held, holder.as_ref(), bound, &unspawned);
    out.write_all(lines.as_bytes())?;
    Ok(Exit::Completed.into())
}

#[cfg(test)]
mod tests;
