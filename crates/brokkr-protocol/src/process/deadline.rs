//! When a driver's watchdog kills it: a length counted from the moment the
//! watchdog arms, which every attempt takes, or one absolute instant that
//! no work before the watchdog arms can move, which a cool-down probe
//! takes so that its kill is never later than its route's hold ends (fix
//! rounds 0106b1r6, D1, and 0106b1r9, E1).
//!
//! A probe's instant is checked once more at the fork itself: under the
//! registry's lock, after the read of the process table that admits the
//! driver and as the last step before the spawn, on the caller's own
//! clock. One already past is not spawned ([`SpawnError::Late`]).

use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rustix::process::Pid;

use super::tree::Host;
use super::{Attempt, DriverProcess, Limits, SpawnEnv, SpawnError};

impl DriverProcess {
    /// [`DriverProcess::spawn`] by the absolute deadline `by`: the fork
    /// reads `by.left` immediately before it spawns, refusing one already
    /// past, and the watchdog kills at `by.kill`, whatever passed between
    /// the caller's taking it and the watchdog's arming.
    pub fn spawn_by(
        command: &[String],
        workdir: &std::path::Path,
        by: By<'_>,
        env: &SpawnEnv,
    ) -> Result<Self, SpawnError> {
        let deadline = Some(Arm::By(by));
        Self::spawn_limited(command, workdir, deadline, env, Host::REAL, Limits::DEFAULT)
    }
}

/// An absolute deadline, as [`DriverProcess::spawn_by`] takes it.
#[derive(Clone, Copy)]
pub struct By<'a> {
    /// When the watchdog kills, on the monotonic clock: the caller's own
    /// deadline, which the spawn carries as it is and never derives again
    /// from a later reading of any clock.
    pub kill: Instant,
    /// What remains to the deadline on the caller's own clock, read at the
    /// fork under the registry's lock: `None` once it has passed, and the
    /// length the kill's words name otherwise.
    pub left: &'a dyn Fn() -> Option<Duration>,
}

/// A spawn's deadline before the fork: a length, or an absolute one the
/// fork reads last.
#[derive(Clone, Copy)]
pub(super) enum Arm<'a> {
    After(Duration),
    By(By<'a>),
}

impl Arm<'_> {
    /// The watchdog's deadline, as the fork takes it: `None` for an
    /// absolute deadline already past, which spawns nothing.
    pub(super) fn at_fork(self) -> Option<Deadline> {
        match self {
            Arm::After(length) => Some(Deadline::After(length)),
            Arm::By(by) => (by.left)().map(|armed| Deadline::At {
                kill: by.kill,
                armed,
            }),
        }
    }
}

/// A driver's deadline, as its watchdog arms it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Deadline {
    /// This long after the watchdog arms.
    After(Duration),
    /// At `kill` on the monotonic clock, however late the watchdog arms.
    /// `armed` is what remained to it at the fork, the length the kill's
    /// words name.
    At { kill: Instant, armed: Duration },
}

impl Deadline {
    /// How long the watchdog waits from now: read as it arms, so an
    /// instant's wait shrinks by whatever passed before.
    pub(super) fn left(self) -> Duration {
        match self {
            Deadline::After(length) => length,
            Deadline::At { kill, .. } => kill.saturating_duration_since(Instant::now()),
        }
    }

    /// When the watchdog kills, for a driver whose watchdog armed at
    /// `started`.
    pub(super) fn kills(self, started: Instant) -> Instant {
        match self {
            Deadline::After(length) => started + length,
            Deadline::At { kill, .. } => kill,
        }
    }

    /// The length the kill's words name.
    pub(super) fn armed(self) -> Duration {
        match self {
            Deadline::After(length) | Deadline::At { armed: length, .. } => length,
        }
    }

    /// The watchdog of the attempt `key`, whose driver is `child`: at this
    /// deadline it marks `timed_out` and signals the driver's group at
    /// once, waiting on neither the registry's lock nor the table's read,
    /// which another attempt may hold (fix round 0106b1r9, E1). The kill
    /// that reads the table and records what it shows follows the signal.
    /// The leader is unreaped until `finish` joins this thread, so its id
    /// still names its group when it is signalled. Dropping the sender
    /// disarms it.
    pub(super) fn watch(
        self,
        timed_out: &Arc<AtomicBool>,
        (child, key, host): (&Child, u64, Host),
    ) -> (mpsc::Sender<()>, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel::<()>();
        let timed_out = Arc::clone(timed_out);
        let group = Pid::from_child(child);
        let thread = std::thread::spawn(move || {
            if let Err(RecvTimeoutError::Timeout) = rx.recv_timeout(self.left()) {
                timed_out.store(true, Ordering::SeqCst);
                let _ = (host.kill_group)(group);
                // A refusal here is met again, and carried, by
                // `tree::end`, whose kill reads the table and signals the
                // same tree once more.
                drop(Attempt::kill(key, host));
            }
        });
        (tx, thread)
    }
}

#[cfg(test)]
mod tests;
