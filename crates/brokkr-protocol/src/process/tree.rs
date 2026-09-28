//! The attempt's process tree (#403). Two means reach it, and the attempt
//! is over only when both read it gone.
//!
//! The group. The driver leads a session, and so a process group, of its
//! own, so the harness under it, the harness's tool subprocesses and any
//! workspace-tool child that stays in the group are reached by one signal
//! to the group. The
//! group id is the leader's pid, which is not reused while the leader is
//! unreaped, so the group is SIGNALLED only then: by the watchdog, which
//! `finish` joins before the reap, by `end` before its reap, and by the
//! stop handler (`attempts`) until `end` closes the attempt. After the
//! reap the group is only read from the process table, and a reused id
//! reads as a survivor: fail-closed, never a signal to a stranger.
//!
//! The recorded descendants. What leaves the group (a `setsid` child, as
//! Node's `detached` spawn makes, or a job that shell job control moves
//! to a group of its own) is out of the group signal's reach, so
//! `attempts` records every descendant of the attempt by pid and start
//! stamp while the tree is alive, reads the table once more at every
//! kill, and ends those identities with the group.
//!
//! Linux is closed. The engine and every driver are child subreapers. An
//! orphan of the tree goes to its driver while the driver runs, where the
//! tracker records it as a descendant, and to the engine once the driver
//! is gone, whatever its session or group. Every running child of the
//! engine outside the engine's own group that no live attempt leads,
//! records or began after is a stray. A child the engine spawns stays in
//! that group, and nothing of an attempt can join it: the driver's
//! session is its own, and `setpgid` refuses a group in another session.
//! A stray is attributed to the attempts that could have left it: those
//! it does not predate, whose leader no longer runs, and that had not yet
//! ended when it was born (its start stamp is no later than the youngest
//! one the table showed when it first found the attempt ended). One such
//! attempt ends it with its own tree. When there are several, the stray
//! is ended and every one of them parks. When there is none, the stray is
//! the engine's own (git's detached maintenance, say). Every read of the
//! table is taken under the registry's lock, in order. A signal to an
//! identity rides a pidfd
//! checked against the start stamp. So no descendant the engine can see
//! is left running while an attempt is certified settled. A kernel that
//! refuses the engine the subreaper or a pidfd leaves the second means
//! absent: a kill that saw any descendant then parks, naming the means
//! that was missing.
//!
//! Every read fails closed. A table that cannot be read, a row that
//! cannot be read or parsed, a `ps` that exits nonzero and a snapshot
//! without the engine's own row prove nothing; a row that vanished
//! between the listing and its read is gone. A read before the spawn that
//! fails refuses the spawn: without it, nothing tells the attempt's
//! orphans from what ran before it. A kill the kernel refuses on a live
//! identity leaves the cleanup unresolved.
//!
//! macOS has no subreaper and no pidfd. There the engine reads the table
//! synchronously at the kill, ends what it attributes, and parks on any
//! doubt. One residual remains, accepted by the operator's ruling of
//! 2026-09-28 (LINUX CLOSED, MACOS RESIDUAL ACCEPTED): a descendant that
//! leaves the group and whose parent exits between two reads of the
//! table, faster than the tracker's 100 ms interval, is reparented to
//! launchd unseen. Beside it, and not a limit of settlement: without a
//! pidfd, a pid reused between `ps`'s confirmation and the signal could
//! be signalled.
//!
//! Nothing is ever chosen by working directory or repository, so a
//! concurrent run in the same checkout is never touched.

use std::process::Child;
use std::time::{Duration, Instant};

use rustix::io::Errno;
use rustix::process::{getpid, kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions};
use thiserror::Error;

use super::attempts::{self, Attempt};
use super::table::{self, Entry, Identity, TableError};

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

/// The host calls ending an attempt makes, held as data so a test can
/// stand in a kill the kernel refuses, a table that never reads settled,
/// or a host that refuses the engine a means.
#[derive(Clone, Copy)]
pub(super) struct Host {
    pub kill_group: fn(Pid) -> rustix::io::Result<()>,
    pub kill: fn(&Identity) -> std::io::Result<()>,
    pub table: fn() -> Result<Vec<Entry>, TableError>,
    /// The means this host refused the engine, if one (`attempts`).
    pub missing: fn() -> Option<Unsettled>,
}

impl Host {
    pub(super) const REAL: Host = Host {
        kill_group,
        kill: table::kill,
        table: table::snapshot,
        missing: attempts::missing,
    };
}

/// Why an attempt's end could not be certified: something of it may
/// still be running, so it parks rather than settles or retries.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unsettled {
    #[error("its process group {group} could not be signalled: {}", std::io::Error::from_raw_os_error(*errno))]
    Kill { group: i32, errno: i32 },
    #[error("its descendant {pid} could not be signalled: {error}")]
    Signal { pid: i32, error: String },
    #[error("its driver {pid} was not reaped after the kill")]
    Reap { pid: i32 },
    #[error("its process group {group} still had members after the kill")]
    Group { group: i32 },
    #[error("its descendants {} were still running after the kill", listed(pids))]
    Descendants { pids: Vec<i32> },
    #[error(
        "processes {} the engine adopted could not be attributed to one attempt",
        listed(pids)
    )]
    Strays { pids: Vec<i32> },
    #[error("the process table could not be read: {error}")]
    Table { error: String },
    #[error("it had descendants, and the engine is no child subreaper here: {}", std::io::Error::from_raw_os_error(*.0))]
    Subreaper(i32),
    #[error("it had descendants, and this kernel gives the engine no pidfd: {}", std::io::Error::from_raw_os_error(*.0))]
    Pidfd(i32),
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
/// read the table and kill the group and every identity the attempt owns,
/// reap the leader, wait up to `settle` for the table to read every part
/// of it gone, and then refuse what still casts doubt on it.
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
    let closed = attempt.close(host);
    let settle = Instant::now() + bounds.settle;
    let reaped = reap(child, settle);
    closed?;
    reaped?;
    wait(settle, || attempt.survivors(host))?;
    attempt.doubts((host.missing)())
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

/// Look until `until` for `settled` to hold, and return what it last
/// found when it never does.
pub(super) fn wait(
    until: Instant,
    mut settled: impl FnMut() -> Result<(), Unsettled>,
) -> Result<(), Unsettled> {
    loop {
        match settled() {
            Ok(()) => return Ok(()),
            Err(unsettled) if Instant::now() >= until => return Err(unsettled),
            Err(_) => std::thread::sleep(POLL),
        }
    }
}

/// One read of `table`. A snapshot without the engine's own row lists
/// nothing it can be trusted for, however much it lists.
pub(super) fn read(table: fn() -> Result<Vec<Entry>, TableError>) -> Result<Vec<Entry>, Unsettled> {
    let me = getpid().as_raw_pid();
    table()
        .and_then(|entries| {
            let found = entries.iter().any(|entry| entry.id.pid == me);
            found
                .then_some(entries)
                .ok_or(TableError::NoSelf { pid: me })
        })
        .map_err(|error| Unsettled::Table {
            error: error.to_string(),
        })
}

/// ESRCH from a kill: nothing is left to signal.
pub(super) fn refused(error: rustix::io::Result<()>) -> Option<Errno> {
    error.err().filter(|errno| *errno != Errno::SRCH)
}
