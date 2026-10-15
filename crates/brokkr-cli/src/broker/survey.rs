//! `brokkr broker survey` (decision 0065 slice two, U7c0b; MB3, SD4,
//! SC1): the engine's provisioning observer, the counterpart of `serve`'s
//! own observer. The engine runs it as a killable child in a process group
//! of its own under the attempt's startup deadline (U7c), so a blocked
//! filesystem call ends with the group and leaves no worker behind.
//!
//! It reads one closed, bounded [`Survey`] on standard input, resolves the
//! server's program against the engine's HOME, hashes the bootstrap (this
//! very binary) itself, observes the box as provisioning does
//! ([`ServerBox::provision`]) and writes one [`Surveyed`] line on standard
//! output, exiting nonzero with the cause where it refused, a request it
//! could not read included ([`Unread`]). It writes nothing else on any
//! stream. It carries no authority: it reads no store, plan or secret, and
//! writes no file beyond the box's own scratch identity, removed before it
//! replies.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use brokkr_core::canonical;
use brokkr_protocol::broker::{Bootstrap, Provisioned, Refusal, Survey, Surveyed, Tree, Unread};
use brokkr_protocol::hands::{ServerBox, ServerProfile, ServerProgram};

/// The most bytes a request may hold: MB3's request bound.
const REQUEST_BYTES_MAX: u64 = 1 << 20;

/// Survey the box the request on standard input names and write the one
/// reply: success where it was observed, failure where it refused or the
/// request did not read.
pub(super) fn run() -> anyhow::Result<ExitCode> {
    let reply = match request(&mut std::io::stdin().lock()) {
        Ok(survey) => surveyed(&survey, &std::env::current_exe),
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

/// The one request `input` holds whole, within [`REQUEST_BYTES_MAX`]: a
/// closed record, nothing after it but whitespace; truncated where the
/// input ends inside it, malformed where it holds a key twice, a field the
/// record does not name or anything else that is not that record.
pub(super) fn request(input: &mut impl Read) -> Result<Survey, Unread> {
    let mut bytes = Vec::new();
    let read = input.take(REQUEST_BYTES_MAX + 1).read_to_end(&mut bytes);
    read.map_err(|_| Unread::Input)?;
    let within = bytes.len() as u64 <= REQUEST_BYTES_MAX;
    within.then_some(()).ok_or(Unread::Oversized)?;
    serde_json::from_slice(&bytes).map_err(|cause| match cause.is_eof() {
        true => Unread::Truncated,
        false => Unread::Malformed,
    })
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
