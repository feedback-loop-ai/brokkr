//! The one door every harness spawns through. It lives here so
//! `adapters.rs` stays inside its measured ceiling.

use std::io::Write;
use std::process::Stdio;

use super::stderr::{drain_stderr, StderrCapture};
use super::{harness_command, io_context, lost};
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
    let mut builder = harness_command(command, workdir, env, bindings)?;
    builder.stdin(stdin).stdout(stdout).stderr(Stdio::piped());
    let child = crate::process::spawned::spawn(&mut builder);
    let child = io_context(child, "could not invoke the agent CLI")?;
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
    spawn_fed_in(command, prompt, (workdir, &[]), bindings)
}

/// [`spawn_fed`] with `env` set for the child alone, beside its `workdir`:
/// the agy driver's engine HOME (#1200).
pub(super) fn spawn_fed_in(
    command: &[String],
    prompt: &str,
    (workdir, env): (&str, &[(&str, &str)]),
    bindings: &[secret::BoundSecret],
) -> Result<(Spawned, StderrCapture), String> {
    let piped = Stdio::piped;
    let mut child = spawn_harness(command, workdir, piped(), piped(), env, bindings)?;
    let mut stdin = child.stdin.take().expect("piped");
    io_context(
        stdin.write_all(prompt.as_bytes()),
        "could not write the prompt",
    )?;
    drop(stdin);
    let stderr = drain_stderr(&mut child);
    Ok((child, stderr))
}
