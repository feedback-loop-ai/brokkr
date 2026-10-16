//! `brokkr broker survey` (decision 0065 slice two, U7c0b; MB3, SD4,
//! SC1): the engine's provisioning observer, the counterpart of `serve`'s
//! own observer. The engine runs it as a killable child in a process group
//! of its own under the attempt's startup deadline (U7c), so a blocked
//! filesystem call ends with the group and leaves no worker behind.
//!
//! It reads one closed, bounded [`Survey`] line on standard input, ties its
//! life to the engine's ([`tethered`]), resolves the server's program
//! against the engine's HOME, hashes the bootstrap (this very binary)
//! itself, observes the box as provisioning does ([`ServerBox::provision`])
//! and writes one [`Surveyed`] line on standard output, exiting nonzero
//! with the cause where it refused, a request it could not read included
//! ([`Unread`]). It writes nothing else on any stream. It carries no
//! authority: it reads no store, plan or secret, and writes no file beyond
//! the box's own scratch identity, removed before it replies.

use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use brokkr_core::canonical;
use brokkr_protocol::broker::{Bootstrap, Provisioned, Refusal, Survey, Surveyed, Tree, Unread};
use brokkr_protocol::hands::{ServerBox, ServerProfile, ServerProgram};
use rustix::process::{getpgid, getpid, kill_current_process_group, Signal};

/// The most bytes a request line may hold, its newline included: MB3's
/// request bound.
const REQUEST_BYTES_MAX: u64 = 1 << 20;

/// Survey the box the request on standard input names and write the one
/// reply: success where it was observed, failure where it refused or the
/// request did not read.
pub(super) fn run() -> anyhow::Result<ExitCode> {
    // Its own statement, so standard input's lock is let go before the
    // watcher takes it.
    let read = request(&mut std::io::stdin().lock());
    let reply = match read {
        Ok(survey) => match tethered() {
            Ok(()) => surveyed(&survey, &std::env::current_exe),
            Err(cause) => Surveyed::Refused(cause),
        },
        Err(cause) => Surveyed::Unread(cause),
    };
    let mut line = serde_json::to_vec(&reply)?;
    line.push(b'\n');
    std::io::stdout().lock().write_all(&line)?;
    Ok(match reply {
        Surveyed::Provisioned(_) => ExitCode::SUCCESS,
        Surveyed::Refused(_) | Surveyed::Unread(_) => ExitCode::FAILURE,
    })
}

/// The one request line `input` holds, within [`REQUEST_BYTES_MAX`] bytes
/// its newline included: a closed record, nothing after it on the line but
/// whitespace; oversized where no newline comes within the bound,
/// truncated where the input ends before one does, malformed where the
/// line holds a key twice, a field the record does not name or anything
/// else that is not that record. Nothing after the newline is read.
pub(super) fn request(input: &mut impl BufRead) -> Result<Survey, Unread> {
    let mut line = Vec::new();
    let read = input.take(REQUEST_BYTES_MAX).read_until(b'\n', &mut line);
    read.map_err(|_| Unread::Input)?;
    let ended = line.last() == Some(&b'\n');
    let full = line.len() as u64 == REQUEST_BYTES_MAX;
    match (ended, full) {
        (true, true | false) => serde_json::from_slice(&line).or(Err(Unread::Malformed)),
        (false, true) => Err(Unread::Oversized),
        (false, false) => Err(Unread::Truncated),
    }
}

/// Tie this survey's life, and its whole group's, to the engine that
/// started it, as `serve`'s observer is tied to its broker
/// (`session::tethered`): the engine sends nothing after the request line
/// and holds its end of the input open until it has read the reply, so the
/// rest of the input ends only once the engine has the reply or has ended
/// by whatever means: a stop signal before its stop handler exists, that
/// handler's own stop, or its death. The watcher then kills the group,
/// whatever the survey started included. The group killed is only ever the
/// survey's own: one that leads no group of its own, as the engine makes
/// it, refuses before any watcher is armed, so its caller's group is never
/// killed.
fn tethered() -> Result<(), Refusal> {
    let leads = getpgid(None) == Ok(getpid());
    leads.then_some(()).ok_or(Refusal::Identity)?;
    let watcher = std::thread::Builder::new().spawn(|| {
        std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink()).ok();
        kill_current_process_group(Signal::KILL).ok();
    });
    watcher.map(drop).or(Err(Refusal::Identity))
}

/// The reply to `survey`, `this` naming the running binary.
pub(super) fn surveyed(survey: &Survey, this: &dyn Fn() -> std::io::Result<PathBuf>) -> Surveyed {
    match provisioned(survey, this) {
        Ok(provisioned) => Surveyed::Provisioned(provisioned),
        Err(cause) => Surveyed::Refused(cause),
    }
}

/// What provisioning seals of `survey`'s box, or MB3's first cause: a
/// HOME that is not absolute or does not resolve
/// ([`super::canonical_home`]) leaves the program tree unresolved, as a
/// launch that names no program does; a bootstrap that does not read
/// leaves identity unprotected; then the box's own causes.
fn provisioned(
    survey: &Survey,
    this: &dyn Fn() -> std::io::Result<PathBuf>,
) -> Result<Provisioned, Refusal> {
    let home = super::canonical_home(&survey.home).ok_or(Refusal::ProgramTree)?;
    let program = ServerProgram::resolve(&survey.launch, &home)?;
    let bootstrap = bootstrap(this).ok_or(Refusal::Identity)?;
    let profile = ServerProfile {
        reach: &survey.reach,
        network: &survey.network,
        bootstrap: &bootstrap.path,
        writers: &survey.writers,
    };
    let sources = ServerBox::provision(&program, &profile)?;
    let tree = match program.tree() {
        Tree::System {} => Tree::System {},
        Tree::Package { root } => Tree::Package { root: root.clone() },
    };
    Ok(Provisioned {
        executable: program.executable().to_path_buf(),
        tree,
        bootstrap,
        sources,
    })
}

/// The running binary `this` names, canonical, and the sha256 of its
/// bytes.
fn bootstrap(this: &dyn Fn() -> std::io::Result<PathBuf>) -> Option<Bootstrap> {
    let path = this().and_then(std::fs::canonicalize).ok()?;
    let bytes = std::fs::read(&path).ok()?;
    Some(Bootstrap {
        digest: canonical::sha256_bytes(&bytes),
        path,
    })
}

#[cfg(test)]
mod tests;
