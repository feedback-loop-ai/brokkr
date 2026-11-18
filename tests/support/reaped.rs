//! The one fixture for a pid no process holds: a child that ran to its end
//! and was reaped, within a bound (#430's 5c2 follow-ups, decision 0071
//! ruling 9). Each test binary includes this through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use std::process::Command;
use std::time::{Duration, Instant};

/// How long a fixture's child may take to exit before the fixture fails.
pub(crate) const PATIENCE: Duration = Duration::from_secs(30);

/// A pid the kernel has reaped: a child that ran `true` to completion.
pub(crate) fn exited_pid() -> u32 {
    reaped_within(&mut Command::new("true"), PATIENCE)
}

/// The pid of `command`'s child once it has exited successfully and been
/// reaped. A child still running after `patience` is killed and reaped,
/// and the fixture fails saying so, rather than hang the test.
pub(crate) fn reaped_within(command: &mut Command, patience: Duration) -> u32 {
    let mut child = command.spawn().unwrap();
    let pid = child.id();
    let deadline = Instant::now() + patience;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "the fixture's child {pid} ended {status}");
            return pid;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("the fixture's child {pid} did not exit within {patience:?}; it was killed");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
