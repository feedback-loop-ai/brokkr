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
/// returns or fails within PATIENCE (see `pid_within`).
pub(crate) fn exited_pid() -> u32 {
    pid_within(&mut Command::new("true"), PATIENCE, kill)
}

/// The fixture's stop for an overdue child: SIGKILL.
pub(crate) fn kill(child: &mut Child) {
    child.kill().unwrap()
}

/// The entry point's one body: the pid of `command`'s reaped child, or a
/// failure within `patience` that says which way the child was overdue.
pub(crate) fn pid_within(
    command: &mut Command,
    patience: Duration,
    stop: impl FnOnce(&mut Child),
) -> u32 {
    reaped_with(command, patience, stop).unwrap_or_else(|overdue| overdue.fail())
}

/// A fixture's child that did not end within its patience.
pub(crate) struct Overdue {
    pub(crate) pid: u32,
    pub(crate) patience: Duration,
    pub(crate) outcome: OverdueOutcome,
}

/// How an overdue child stood at the patience's end.
pub(crate) enum OverdueOutcome {
    /// It was stopped, and reaped within the patience; its status says
    /// whether the stop or its own exit ended it.
    Stopped(ExitStatus),
    /// It was not reaped within the patience; the child is still owned.
    Unreaped(Child),
}

/// The one home of what the fixture says of an overdue child.
impl fmt::Display for Overdue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (pid, patience) = (self.pid, self.patience);
        let end = match &self.outcome {
            OverdueOutcome::Stopped(status) if signalled(status) => {
                format!("it was stopped, and ended {status}")
            }
            OverdueOutcome::Stopped(status) => {
                format!("it was sent its stop, but ended by itself: {status}")
            }
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
    /// patience already spent: an unreaped child is sent SIGKILL and handed
    /// to a thread of its own that reaps it, so it is not left a zombie and
    /// the failing caller does not wait for it.
    pub(crate) fn fail(self) -> ! {
        let words = self.to_string();
        if let OverdueOutcome::Unreaped(mut child) = self.outcome {
            let _ = child.kill();
            // A thread the OS refuses leaves the child unreaped, never the
            // words below unsaid: the builder's spawn fails without panicking.
            let _ = std::thread::Builder::new().spawn(move || child.wait());
        }
        panic!("{words}")
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
        Some(status) => OverdueOutcome::Stopped(status),
        None => OverdueOutcome::Unreaped(child),
    };
    Err(Overdue {
        pid,
        patience,
        outcome,
    })
}

/// The child's status if a poll begun before `deadline` reaps it. The clock
/// is read before each poll, so a status first collected after the deadline
/// is never reported as within it: the reap a poll observed and the timing
/// the clock rules stay apart. No sleep runs past the deadline, and a poll
/// does not block.
pub(crate) fn exited_before(child: &mut Child, deadline: Instant) -> Option<ExitStatus> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        std::thread::sleep(POLL.min(deadline - now));
    }
}

/// Did a signal end the child? A stop that is a kill ends it so; a child
/// that exited by itself, before or despite its stop, has an exit code.
fn signalled(status: &ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    status.signal().is_some()
}
