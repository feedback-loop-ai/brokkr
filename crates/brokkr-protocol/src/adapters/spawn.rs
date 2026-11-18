//! The one door every harness spawns through. It lives here so
//! `adapters.rs` stays inside its measured ceiling.

use std::process::Stdio;

use super::{harness_command, io_context, lost};
use crate::secret;

/// Every harness spawns here: the shared [`harness_command`], the child's
/// stdio, and the one spawn fact every route reports. On Linux the child
/// is registered with the driver's reaper as it spawns, so the reaper
/// leaves it to this handle (#1133).
pub(super) fn spawn_harness(
    command: &[String],
    workdir: &str,
    stdin: Stdio,
    stdout: Stdio,
    env: &[(&str, &str)],
    bindings: &[secret::BoundSecret],
) -> Result<std::process::Child, String> {
    let mut builder = harness_command(command, workdir, env, bindings)?;
    builder.stdin(stdin).stdout(stdout).stderr(Stdio::piped());
    #[cfg(target_os = "linux")]
    let child = crate::process::reaper::spawn(&mut builder);
    #[cfg(not(target_os = "linux"))]
    let child = builder.spawn();
    let child = io_context(child, "could not invoke the agent CLI")?;
    lost::spawned();
    Ok(child)
}
