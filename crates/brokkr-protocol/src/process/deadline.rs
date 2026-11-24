//! When a driver's watchdog kills it: a length counted from the moment the
//! watchdog arms, which every attempt takes, or one absolute instant that
//! no work before the watchdog arms can move, which a cool-down probe
//! takes so that its kill is never later than its route's hold ends (fix
//! rounds 0106b1r6, D1, and 0106b1r9, E1).
//!
//! A probe's instant is checked once more at the fork itself: under the
//! registry's lock, after the read of the process table that admits the
//! driver and as the last step before the spawn, on the caller's own clock
//! and on the monotonic one its kill is carried on, read in that order. One
//! already past on either is not spawned ([`SpawnError::Late`]), one that
//! passes while the caller's clock is read included. Its watchdog waits
//! toward that instant, in slices of at most [`SLICE`], each bound to an
//! absolute instant whose remainder is read as the wait's own argument, so
//! a pause before a wait, or between a slice's sample and its wait, never
//! moves its kill: past the target it kills at once (fix rounds
//! 0106b1r13, M1, and 0106b1r14, M2). A length's watchdog waits to the
//! instant its arming plus that length names, the same way.

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
    /// absolute deadline already past, which spawns nothing. It is past
    /// once either clock says so: the caller's, or the monotonic one its
    /// kill is carried on, so a wall clock stepped back or a pause between
    /// the caller's two readings never launches a driver whose kill
    /// instant has gone by (the return of fix round 0106b1r12).
    ///
    /// The monotonic clock is read after the caller's, so an instant that
    /// passes while the caller's clock is read refuses the spawn too (fix
    /// round 0106b1r13, M1).
    pub(super) fn at_fork(self) -> Option<Deadline> {
        self.at_fork_on(Instant::now)
    }

    /// [`Arm::at_fork`] on the monotonic clock `now`: the thread's own in
    /// production, injected by a test so the order of the two reads is
    /// proved on no wall time (fix round 0106b1r14, M2).
    fn at_fork_on(self, now: impl Fn() -> Instant) -> Option<Deadline> {
        match self {
            Arm::After(length) => Some(Deadline::After(length)),
            Arm::By(by) => {
                let left = (by.left)();
                let ahead = now() < by.kill;
                left.filter(|_| ahead).map(|armed| Deadline::At {
                    kill: by.kill,
                    armed,
                })
            }
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
            let recv = |wait| rx.recv_timeout(wait);
            if self.waited_with(|| {}, Instant::now, recv) {
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

    /// Wait through `recv`, the disarm channel's timed receive, until this
    /// deadline: `true` once it is reached, `false` once the watchdog is
    /// disarmed. The target is one absolute instant on the monotonic clock
    /// `now`, taken once as the watchdog arms: a length counts from that
    /// arming, and an absolute deadline is its kill. Every wait is bound to
    /// an instant, the target, or for an absolute deadline one [`SLICE`]
    /// past the sample and never past the target, and what remains to that
    /// instant is read on `now` as the wait's own argument, after `pause`,
    /// whatever passes between the sample and the wait (as a descheduled
    /// thread takes it). So no pause, before any wait, moves a kill past
    /// its target: one past it is followed by a look at the disarm channel
    /// that waits nothing, and the kill (fix round 0106b1r14, M2). The
    /// thread's own clock and channel are the production seam, with no
    /// pause; a test injects all three.
    fn waited_with(
        self,
        mut pause: impl FnMut(),
        now: impl Fn() -> Instant,
        mut recv: impl FnMut(Duration) -> Result<(), RecvTimeoutError>,
    ) -> bool {
        let target = self.kills(now());
        loop {
            let until = match self {
                Deadline::After(_) => target,
                Deadline::At { .. } => now()
                    .checked_add(SLICE)
                    .map_or(target, |slice| slice.min(target)),
            };
            pause();
            match recv(until.saturating_duration_since(now())) {
                Err(RecvTimeoutError::Timeout) => {}
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return false,
            }
            if now() >= target {
                return true;
            }
        }
    }
}

/// The longest an absolute deadline's watchdog waits before it reads its
/// target again.
const SLICE: Duration = Duration::from_millis(100);

#[cfg(test)]
mod tests;
