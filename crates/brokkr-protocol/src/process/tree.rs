//! The attempt's process tree (#403). The driver leads a process group of
//! its own, so the harness under it, the harness's tool subprocesses and
//! any workspace-tool child are reached by one signal to the group, and
//! the attempt ends as one unit rather than as the one process the
//! engine holds a handle to.
//!
//! Identity. The group id is the leader's pid, and a pid is not reused
//! while its process is unreaped. The group is therefore SIGNALLED only
//! while the leader is unreaped (a zombie at worst): by the watchdog,
//! which `finish` joins before it reaps, and by `end`, before its reap.
//! After the reap the group is only PROBED, with signal 0, and a probe
//! that meets a reused id reads as a survivor: fail-closed, never a
//! signal to a stranger. Nothing is ever chosen by working directory or
//! repository, so a concurrent run in the same checkout is never touched.
//!
//! What leaves the group (a `setsid` daemon) is out of the group signal's
//! reach. One that kept the attempt's stdout or stderr is still seen,
//! because the pipe never reaches EOF, and `process.rs` parks the attempt
//! on it. One that also closed them is the named residual.

use std::process::Child;
use std::time::{Duration, Instant};

use rustix::io::Errno;
use rustix::process::{
    kill_process_group, test_kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions,
};
use thiserror::Error;

/// How often a bounded wait looks again.
const POLL: Duration = Duration::from_millis(10);

/// The bounds on ending an attempt. Each is a ceiling on waiting for a
/// driver that does not cooperate, never a delay a cooperative one pays.
#[derive(Debug, Clone, Copy)]
pub(super) struct Bounds {
    /// How long the leader has to exit on its own after `shutdown`.
    pub grace: Duration,
    /// How long the killed group has to be gone.
    pub settle: Duration,
    /// How long the pipes have to reach EOF once the group is killed.
    pub drain: Duration,
}

impl Bounds {
    pub(super) const DEFAULT: Bounds = Bounds {
        grace: Duration::from_secs(5),
        settle: Duration::from_secs(5),
        drain: Duration::from_secs(5),
    };
}

/// Why an attempt's end could not be certified: something of it may
/// still be running, so it parks rather than settles or retries.
#[derive(Debug, Error)]
pub(super) enum Unsettled {
    #[error("its process group {group} still had members after the kill")]
    Group { group: i32 },
    #[error("a process outside its group still held the driver's stdout")]
    Stdout,
    #[error("a process outside its group still held the driver's stderr")]
    Stderr,
}

/// SIGKILL every member of the group. Only while the leader is
/// unreaped (see the module comment).
pub(super) fn kill_group(group: Pid) {
    let _ = kill_process_group(group, Signal::KILL);
}

/// Does the group still have a member, a zombie included? Only a group
/// the kernel says is gone is gone; any other answer is a member.
pub(super) fn group_alive(group: Pid) -> bool {
    test_kill_process_group(group) != Err(Errno::SRCH)
}

/// Has the leader exited? Observed with `WNOWAIT`, so it stays unreaped
/// and the group id stays the attempt's.
fn exited(group: Pid) -> bool {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    matches!(waitid(WaitId::Pid(group), options), Ok(Some(_)))
}

/// End the tree: give the leader `grace` to exit on its own, kill the
/// group, reap the leader, then wait up to `settle` for `alive` to find
/// the group empty.
pub(super) fn end(
    child: &mut Child,
    bounds: &Bounds,
    alive: fn(Pid) -> bool,
) -> Result<(), Unsettled> {
    let group = Pid::from_child(child);
    let grace = Instant::now() + bounds.grace;
    while !exited(group) && Instant::now() < grace {
        std::thread::sleep(POLL);
    }
    kill_group(group);
    let _ = child.wait();
    let settle = Instant::now() + bounds.settle;
    while alive(group) {
        if Instant::now() >= settle {
            return Err(Unsettled::Group {
                group: group.as_raw_pid(),
            });
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}
