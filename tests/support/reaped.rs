//! The one fixture for a pid no process holds: a child that ran to its end
//! and was reaped, within a bound (#430's 5c2 follow-ups, decision 0071
//! ruling 9). Each test binary includes this through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use std::fmt;
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

/// How long a fixture's child may take to exit, and an overdue one to be
/// stopped and reaped, before the fixture fails.
pub(crate) const PATIENCE: Duration = Duration::from_secs(30);

/// The most of a patience kept back for reaping a child the fixture
/// stopped; it never takes more than half the patience.
pub(crate) const REAP_GRACE: Duration = Duration::from_secs(1);

/// How often the fixture polls its child.
const POLL: Duration = Duration::from_millis(5);

/// A pid the kernel has reaped: a child that ran `true` to completion. It
/// returns or fails within PATIENCE: an overdue child fails it at once,
/// saying which way it was overdue.
pub(crate) fn exited_pid() -> u32 {
    reaped_with(&mut Command::new("true"), PATIENCE, |child| {
        child.kill().unwrap()
    })
    .unwrap_or_else(|mut overdue| overdue.fail())
}

/// A fixture's child that did not end within its patience.
pub(crate) struct Overdue {
    pub(crate) pid: u32,
    pub(crate) patience: Duration,
    pub(crate) outcome: OverdueOutcome,
}

/// How an overdue child stood at the patience's end.
pub(crate) enum OverdueOutcome {
    /// The stop ended it, and it was reaped within the patience.
    Killed(ExitStatus),
    /// It was not reaped within the patience; the child is still owned, so
    /// the caller kills and reaps it.
    Unreaped(Child),
}

/// The one home of what the fixture says of an overdue child.
impl fmt::Display for Overdue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (pid, patience) = (self.pid, self.patience);
        let end = match &self.outcome {
            OverdueOutcome::Killed(status) => format!("it was killed, and ended {status}"),
            OverdueOutcome::Unreaped(_) => "it was stopped but not reaped within it".to_owned(),
        };
        write!(
            f,
            "the fixture's child {pid} did not exit within {patience:?}; {end}"
        )
    }
}

impl Overdue {
    /// Fail saying how the child was overdue, taking no time past the
    /// patience already spent: an unreaped child is sent SIGKILL and not
    /// waited for. The child stays the overdue's own, for a caller that
    /// catches the failure to reap it.
    pub(crate) fn fail(&mut self) -> ! {
        if let OverdueOutcome::Unreaped(child) = &mut self.outcome {
            let _ = child.kill();
        }
        panic!("{self}")
    }
}

/// The pid of `command`'s child once it has exited successfully and been
/// reaped, all within one `patience`. A child still running when only the
/// grace is left is stopped by `stop`, and the fixture gives it back as
/// overdue by the same deadline, rather than hang the test.
pub(crate) fn reaped_with(
    command: &mut Command,
    patience: Duration,
    stop: impl FnOnce(&mut Child),
) -> Result<u32, Overdue> {
    let deadline = Instant::now() + patience;
    let grace = REAP_GRACE.min(patience / 2);
    let mut child = command.spawn().unwrap();
    let pid = child.id();
    if let Some(status) = exited_before(&mut child, deadline - grace) {
        assert!(status.success(), "the fixture's child {pid} ended {status}");
        return Ok(pid);
    }
    stop(&mut child);
    let outcome = match exited_before(&mut child, deadline) {
        Some(status) => OverdueOutcome::Killed(status),
        None => OverdueOutcome::Unreaped(child),
    };
    Err(Overdue {
        pid,
        patience,
        outcome,
    })
}

/// The child's status if it is reaped before `deadline`. The clock is read
/// after each poll, so an exit seen once the deadline has passed is not a
/// success, and no sleep runs past the deadline.
pub(crate) fn exited_before(child: &mut Child, deadline: Instant) -> Option<ExitStatus> {
    loop {
        let status = child.try_wait().unwrap();
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        if status.is_some() {
            return status;
        }
        std::thread::sleep(POLL.min(deadline - now));
    }
}
