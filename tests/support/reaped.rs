//! The one fixture for a pid no process holds: a child that ran to its end
//! and was reaped, within a bound (#430's 5c2 follow-ups, decision 0071
//! ruling 9). Each test binary includes this through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

/// How long a fixture's child may take to exit before the fixture fails.
pub(crate) const PATIENCE: Duration = Duration::from_secs(30);

/// How long a child the fixture stopped may take to be reaped before the
/// fixture fails rather than wait on it without a bound.
pub(crate) const REAP_GRACE: Duration = Duration::from_secs(1);

/// A pid the kernel has reaped: a child that ran `true` to completion.
pub(crate) fn exited_pid() -> u32 {
    reaped_within(&mut Command::new("true"), PATIENCE)
}

/// The pid of `command`'s child once it has exited successfully and been
/// reaped. A child still running after `patience` is killed and reaped
/// within `REAP_GRACE`, and the fixture fails saying so, rather than hang
/// the test.
pub(crate) fn reaped_within(command: &mut Command, patience: Duration) -> u32 {
    reaped_with(command, patience, |child| child.kill().unwrap())
}

/// `reaped_within` with the step that stops an overdue child given by the
/// caller, so a test can hold the child alive past the stop.
pub(crate) fn reaped_with(
    command: &mut Command,
    patience: Duration,
    stop: impl FnOnce(&mut Child),
) -> u32 {
    let mut child = command.spawn().unwrap();
    let pid = child.id();
    if let Some(status) = exited_within(&mut child, patience) {
        assert!(status.success(), "the fixture's child {pid} ended {status}");
        return pid;
    }
    stop(&mut child);
    match exited_within(&mut child, REAP_GRACE) {
        Some(_) => panic!("the fixture's child {pid} did not exit within {patience:?}; it was killed"),
        None => panic!(
            "the fixture's child {pid} did not exit within {patience:?}; it was stopped but not reaped within {REAP_GRACE:?}"
        ),
    }
}

/// The child's status if it is reaped before `patience` runs out. The
/// deadline is read before each poll, so an exit after it is not a success.
fn exited_within(child: &mut Child, patience: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + patience;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}
