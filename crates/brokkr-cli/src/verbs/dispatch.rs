//! `brokkr dispatch` (decision 0110 rulings 3, 4 and 6): one pass with
//! `--once` under the journal's dispatch lock, each run it starts handed to
//! this binary as `dispatch drive`, a process in a session of its own; the
//! driver itself; the bound's command; and the holder's readout. The pass,
//! the binary's identity, the driver's log and the driver's checks are
//! brokkr-runtime's; this handler resolves the journal, starts each
//! driver's process, which needs one call between fork and exec, and
//! paints each line through [`Safe`](crate::render::Safe). The standing
//! loop is #430's slice 6c.

use std::ffi::OsString;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{anyhow, bail, Result};
use brokkr_runtime::argv::Executable;
use brokkr_runtime::dispatch::{self, Binary, Seams, Spawn, SpawnFailed, Spawned};
use brokkr_runtime::{launch, Engine};
use brokkr_store::{Attribution, EntryId};

use super::delivery::operator_name;
use super::queue::on_this_host;
use crate::cli_args::{DispatchArgs, DispatchCmd, DriveArgs, SetBoundArgs, StatusArgs};
use crate::{drive_to_end, open_journal, Access, Exit};

mod report;

/// The refusal of a dispatch without `--once`, until slice 6c builds the
/// standing loop.
const NO_LOOP: &str = "dispatch: the standing loop is not built yet (decision 0110 slice 6c); \
                       pass --once to run one pass";

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
        None => bail!(NO_LOOP),
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
    // A driver runs in its entry's workspace, so the journal it is handed
    // is named whole.
    let journal = std::path::absolute(args.journal.journal(workspace)?)?;
    let binary = Binary::started(&Executable::current())?;
    let mut store = open_journal(&journal, Access::Append)?;
    let held = match dispatch::take_dispatch(&mut store)? {
        Ok(held) => held,
        Err(refused) => return Err(anyhow!(refused)),
    };
    let seams = Seams {
        stands: || binary.stands(),
        spawn: |spawn: &Spawn| driver(&binary, &journal, spawn),
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
fn driver(binary: &Binary, journal: &Path, spawn: &Spawn) -> Result<Spawned, SpawnFailed> {
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
    let child = command.spawn().map_err(refused)?;
    Ok(Spawned { pid: child.id() })
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
