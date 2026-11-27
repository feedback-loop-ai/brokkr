//! The one door every harness spawns through. It lives here so
//! `adapters.rs` stays inside its measured ceiling.

use std::io::Write;
use std::process::{Command, Stdio};

use thiserror::Error;

use super::stderr::{drain_stderr, StderrCapture};
use super::{harness_command, lost, HarnessCommandRefusal};
pub(super) use crate::process::spawned::Spawned;
use crate::secret;

/// Every harness spawns here: the shared [`harness_command`], the child's
/// stdio, and the one spawn fact every route reports. It spawns through
/// `process::spawned`, so the driver's reaper leaves it to this handle
/// (#1133).
pub(super) fn spawn_harness(
    command: &[String],
    workdir: &str,
    stdin: Stdio,
    stdout: Stdio,
    env: &[(&str, &str)],
    bindings: &[secret::BoundSecret],
) -> Result<Spawned, String> {
    let builder = harness_command(command, workdir, env, bindings)?;
    Ok(spawned(builder, stdin, stdout)?)
}

/// Why a harness did not start: its command could not be built, it could
/// not be spawned, or its prompt could not be written, each cause carried
/// and printed in the words the shared driver always printed (decision
/// 0071 ruling 8).
#[derive(Debug, Error)]
pub(super) enum Unstarted {
    #[error(transparent)]
    Command(#[from] HarnessCommandRefusal),
    #[error("could not invoke the agent CLI: {0}")]
    Spawn(std::io::Error),
    #[error("could not write the prompt: {0}")]
    Prompt(std::io::Error),
}

/// [`Unstarted`] at the text boundary every driver but agy's reads.
impl From<Unstarted> for String {
    fn from(unstarted: Unstarted) -> String {
        unstarted.to_string()
    }
}

/// `builder` given its stdio and spawned: [`spawn_harness`]'s own end.
fn spawned(mut builder: Command, stdin: Stdio, stdout: Stdio) -> Result<Spawned, Unstarted> {
    builder.stdin(stdin).stdout(stdout).stderr(Stdio::piped());
    let child = crate::process::spawned::spawn(&mut builder).map_err(Unstarted::Spawn)?;
    lost::spawned();
    Ok(child)
}

/// Spawn a harness with `prompt` on its stdin, its stdin closed, and its
/// stderr drained on its own thread. The stream folds share this prologue
/// so none can differ from another in how it feeds a child.
pub(super) fn spawn_fed(
    command: &[String],
    prompt: &str,
    workdir: &str,
    bindings: &[secret::BoundSecret],
) -> Result<(Spawned, StderrCapture), String> {
    Ok(spawn_fed_in(command, prompt, (workdir, &[]), bindings)?)
}

/// Build a harness command through [`harness_command`] (applying bindings and
/// [`super::mcp::unbind`]), then set `env` last so engine-owned variables
/// (such as agy's `HOME`, `PWD`, and update switch) cannot be replaced by
/// bindings or removals (#1200 J1, #1227).
pub(super) fn harness_command_in(
    command: &[String],
    workdir: &str,
    env: &[(&str, &str)],
    bindings: &[secret::BoundSecret],
) -> Result<Command, HarnessCommandRefusal> {
    let mut builder = harness_command(command, workdir, &[], bindings)?;
    builder.envs(env.iter().copied());
    Ok(builder)
}

/// Ask a harness binary for its version through the same command builder
/// ([`harness_command_in`], applying [`super::mcp::unbind`] and `env` with no
/// bound secrets) and the same registered child handling
/// (`process::spawned::output`) as every other driver spawn, wrapped in
/// [`lost::observing`] (proposed decision 0056 ruling 5, #1227).
pub(super) fn observed_version_in(
    command: &[String],
    workdir: &str,
    env: &[(&str, &str)],
) -> Option<String> {
    let output = lost::observing(|| {
        let mut builder = harness_command_in(command, workdir, env, &[]).ok()?;
        crate::process::spawned::output(&mut builder).ok()
    })?;
    if !output.status.success() {
        return None;
    }
    super::parse_observed_version(&output.stdout)
}

/// [`spawn_fed`] with `env` set for the child alone, beside its `workdir`:
/// the agy driver's engine HOME (#1200). It is set last, after the seat's
/// bindings and the attempt's removals, so no binding can replace it (J1);
/// with no `env`, as every other driver spawns, the child is the same.
pub(super) fn spawn_fed_in(
    command: &[String],
    prompt: &str,
    (workdir, env): (&str, &[(&str, &str)]),
    bindings: &[secret::BoundSecret],
) -> Result<(Spawned, StderrCapture), Unstarted> {
    let builder = harness_command_in(command, workdir, env, bindings)?;
    let piped = Stdio::piped;
    let mut child = spawned(builder, piped(), piped())?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin
        .write_all(prompt.as_bytes())
        .map_err(Unstarted::Prompt)?;
    drop(stdin);
    let stderr = drain_stderr(&mut child);
    Ok((child, stderr))
}
