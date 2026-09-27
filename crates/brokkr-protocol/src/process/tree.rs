//! The attempt's process tree (#403). Two means reach it, and the attempt
//! is over only when both read it gone.
//!
//! The group. The driver leads a process group of its own, so the harness
//! under it, the harness's tool subprocesses and any workspace-tool child
//! that stays in the group are reached by one signal to the group. The
//! group id is the leader's pid, which is not reused while the leader is
//! unreaped, so the group is SIGNALLED only then: by the watchdog, which
//! `finish` joins before the reap, by `end` before its reap, and by the
//! stop handler (`attempts`) until `end` closes the attempt. After the
//! reap the group is only read from the process table, and a reused id
//! reads as a survivor: fail-closed, never a signal to a stranger.
//!
//! The recorded descendants. What leaves the group (a `setsid` child, as
//! Node's `detached` spawn makes) is out of the group signal's reach, so
//! `attempts` records every descendant of the attempt by pid and start
//! stamp while the tree is alive, and ends those identities with the
//! group. On Linux the engine is a child subreaper, so an orphan of the
//! tree is adopted by the engine rather than by init; one adopted in a
//! session of its own that no attempt recorded cannot be attributed, and
//! its presence parks the attempt.
//!
//! The residual: a descendant born, moved out of the group and orphaned
//! between two reads of the table. On Linux it parks the attempt if it
//! started a session of its own, and is not seen if it left only its
//! group (shell job control). On macOS an orphan goes to launchd and is
//! not seen either way, and a pid reused between `ps` and the signal is
//! a residual too: there is no pidfd there.
//!
//! Nothing is ever chosen by working directory or repository, so a
//! concurrent run in the same checkout is never touched.

use std::process::Child;
use std::time::{Duration, Instant};

use rustix::io::Errno;
use rustix::process::{kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions};
use thiserror::Error;

use super::attempts::Attempt;
use super::table::{self, Entry};

/// How often a bounded wait looks again.
const POLL: Duration = Duration::from_millis(10);

/// The bounds on ending an attempt. Each is a ceiling on waiting for a
/// driver that does not cooperate, never a delay a cooperative one pays.
#[derive(Debug, Clone, Copy)]
pub(super) struct Bounds {
    /// How long the driver has to take `shutdown` and exit on its own.
    pub grace: Duration,
    /// How long the killed tree has to be reaped and gone.
    pub settle: Duration,
    /// How long the pipes have to reach EOF once the tree is killed.
    pub drain: Duration,
}

impl Bounds {
    pub(super) const DEFAULT: Bounds = Bounds {
        grace: Duration::from_secs(5),
        settle: Duration::from_secs(5),
        drain: Duration::from_secs(5),
    };
}

/// The host calls `end` makes, held as data so a test can stand in a kill
/// the kernel refuses or a table that never reads settled.
#[derive(Clone, Copy)]
pub(super) struct Host {
    pub kill_group: fn(Pid) -> rustix::io::Result<()>,
    pub table: fn() -> std::io::Result<Vec<Entry>>,
}

impl Host {
    pub(super) const REAL: Host = Host {
        kill_group,
        table: table::snapshot,
    };
}

/// Why an attempt's end could not be certified: something of it may
/// still be running, so it parks rather than settles or retries.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unsettled {
    #[error("its process group {group} could not be signalled: {}", std::io::Error::from_raw_os_error(*errno))]
    Kill { group: i32, errno: i32 },
    #[error("its driver {pid} was not reaped after the kill")]
    Reap { pid: i32 },
    #[error("its process group {group} still had members after the kill")]
    Group { group: i32 },
    #[error("its descendants {} were still running after the kill", listed(pids))]
    Descendants { pids: Vec<i32> },
    #[error(
        "processes {} the engine adopted belong to no attempt's record",
        listed(pids)
    )]
    Strays { pids: Vec<i32> },
    #[error("the process table could not be read: {error}")]
    Table { error: String },
    #[error("a process outside its tree still held the driver's stdout")]
    Stdout,
    #[error("a process outside its tree still held the driver's stderr")]
    Stderr,
}

/// A cleanup reason is journaled in the operator's words.
impl serde::Serialize for Unsettled {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

fn listed(pids: &[i32]) -> String {
    let pids: Vec<String> = pids.iter().map(i32::to_string).collect();
    pids.join(", ")
}

/// SIGKILL every member of the group. Only while the leader is
/// unreaped (see the module comment).
pub(super) fn kill_group(group: Pid) -> rustix::io::Result<()> {
    kill_process_group(group, Signal::KILL)
}

/// Has the leader exited? Observed with `WNOWAIT`, so it stays unreaped
/// and the group id stays the attempt's.
fn exited(group: Pid) -> bool {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    matches!(waitid(WaitId::Pid(group), options), Ok(Some(_)))
}

/// End the tree: wait until `grace` for the leader to exit on its own,
/// kill the group and the recorded descendants, reap the leader, then
/// wait up to `settle` for the table to read every part of it gone.
pub(super) fn end(
    child: &mut Child,
    attempt: &Attempt,
    grace: Instant,
    bounds: &Bounds,
    host: Host,
) -> Result<(), Unsettled> {
    let group = Pid::from_child(child);
    while !exited(group) && Instant::now() < grace {
        std::thread::sleep(POLL);
    }
    let refused = attempt.close(host.kill_group);
    let settle = Instant::now() + bounds.settle;
    let reaped = reap(child, settle);
    if let Some(errno) = refused {
        return Err(Unsettled::Kill {
            group: group.as_raw_pid(),
            errno: errno.raw_os_error(),
        });
    }
    reaped?;
    gone(attempt, settle, host.table)
}

/// Reap the killed leader, waiting no later than `until`.
fn reap(child: &mut Child, until: Instant) -> Result<(), Unsettled> {
    while !matches!(child.try_wait(), Ok(Some(_))) {
        if Instant::now() >= until {
            return Err(Unsettled::Reap {
                pid: Pid::from_child(child).as_raw_pid(),
            });
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

/// Wait until `until` for the table to show nothing of the attempt
/// running. A table that cannot be read proves nothing.
fn gone(
    attempt: &Attempt,
    until: Instant,
    table: fn() -> std::io::Result<Vec<Entry>>,
) -> Result<(), Unsettled> {
    loop {
        let read = table().map_err(|error| Unsettled::Table {
            error: error.to_string(),
        });
        match read.and_then(|entries| attempt.survivors(&entries)) {
            Ok(()) => return Ok(()),
            Err(unsettled) if Instant::now() >= until => return Err(unsettled),
            Err(_) => std::thread::sleep(POLL),
        }
    }
}

/// ESRCH from a group kill: the group has no member left to signal.
pub(super) fn refused(error: rustix::io::Result<()>) -> Option<Errno> {
    error.err().filter(|errno| *errno != Errno::SRCH)
}
