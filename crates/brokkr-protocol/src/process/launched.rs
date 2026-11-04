//! A process run to its exit or a deadline outside the driver protocol:
//! the harness probe's launch (#484). It is one of the engine's live
//! attempts, so the tracker neither reaps it nor takes it for a stray, and
//! its tree ends as a driver's does.

use std::process::{Child, Command};
use std::time::Instant;

use super::attempts::Attempt;
use super::{tree, Bounds, Host, Unsettled};

/// A process run to its exit or a deadline outside the driver protocol:
/// the harness probe's launch (#484). It is one of the engine's live
/// attempts, so the tracker neither reaps it nor takes it for a stray
/// (`attempts`), and its tree ends as a driver's does (`tree::end`).
pub(crate) struct Launched {
    pub(crate) child: Child,
    attempt: Attempt,
}

impl Launched {
    /// Spawn `builder`, leading a session of its own (`attempts`).
    pub(crate) fn spawn(builder: &mut Command) -> std::io::Result<Self> {
        let (child, attempt) = Attempt::spawn(builder, Host::REAL)?;
        Ok(Launched { child, attempt })
    }

    /// Has the leader exited? Observed without reaping it, so the group
    /// id stays the launch's.
    pub(crate) fn exited(&self) -> bool {
        tree::exited(rustix::process::Pid::from_child(&self.child))
    }

    /// End the tree now: the group and every descendant recorded under
    /// it killed, the leader reaped, and the table read settled. The
    /// leader's exit code, `None` for a signal.
    pub(crate) fn end(mut self) -> Result<Option<i32>, Unsettled> {
        let (now, bounds) = (Instant::now(), &Bounds::DEFAULT);
        tree::end(&mut self.child, &self.attempt, now, bounds, Host::REAL)?;
        let status = self.child.wait().ok();
        Ok(status.as_ref().and_then(std::process::ExitStatus::code))
    }
}
