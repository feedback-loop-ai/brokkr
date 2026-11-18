//! The one door every child a driver may spawn goes through (#1133). On
//! Linux a serving driver holds each child it spawns here in its reaper's
//! registry, so the reaper leaves the child's status to its own handle,
//! until the handle reaps it or goes; anywhere else, and in any process that does not
//! serve, a spawn here is the plain spawn.

use std::io::Result;
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, ExitStatus, Output, Stdio};

/// A child spawned through the door: its handle, and on Linux the claim
/// that releases the reaper's hold on it when the handle goes.
#[derive(Debug)]
pub(crate) struct Spawned {
    child: Child,
    #[cfg(target_os = "linux")]
    _release: super::reaper::Release,
}

impl Deref for Spawned {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.child
    }
}

impl DerefMut for Spawned {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

impl Spawned {
    /// [`Child::wait_with_output`], the hold released once it returns.
    pub(crate) fn wait_with_output(self) -> Result<Output> {
        self.child.wait_with_output()
    }

    /// [`Child::wait`], the hold released once it returns the status: a
    /// handle kept past its reap holds no pid an orphan may reuse.
    pub(crate) fn wait(&mut self) -> Result<ExitStatus> {
        self.child.wait().inspect(|_| self.reaped())
    }

    /// [`Child::try_wait`], the hold released once it returns a status.
    pub(crate) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child.try_wait().inspect(|status| {
            if status.is_some() {
                self.reaped();
            }
        })
    }

    /// Release the reaper's hold: the handle reaped its child.
    fn reaped(&mut self) {
        #[cfg(target_os = "linux")]
        self._release.release();
    }

    /// The handle and its claim, for a holder that keeps the two apart and
    /// must drop the claim after the handle's last wait.
    #[cfg(target_os = "linux")]
    pub(crate) fn into_parts(self) -> (Child, super::reaper::Release) {
        (self.child, self._release)
    }
}

/// Spawn `command`, held by the driver's reaper where one runs.
pub(crate) fn spawn(command: &mut Command) -> Result<Spawned> {
    #[cfg(target_os = "linux")]
    let (child, _release) = super::reaper::spawn(command)?;
    #[cfg(not(target_os = "linux"))]
    let child = command.spawn()?;
    Ok(Spawned {
        child,
        #[cfg(target_os = "linux")]
        _release,
    })
}

/// [`Command::output`] through [`spawn`]: the standard input closed, and
/// the two outputs captured. It sets all three streams, so a command that
/// sets its own runs [`spawn`] and waits for itself.
pub(crate) fn output(command: &mut Command) -> Result<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    spawn(command)?.wait_with_output()
}

/// [`Command::status`] through [`spawn`].
pub(crate) fn status(command: &mut Command) -> Result<ExitStatus> {
    spawn(command)?.wait()
}

/// Whether `command` runs to a success with all three streams closed:
/// the probes that ask only whether a tool works here.
pub(crate) fn succeeds(command: &mut Command) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    status(command).is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests;
