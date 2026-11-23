//! When a driver's watchdog kills it: a length counted from the moment the
//! watchdog arms, which every attempt takes, or one absolute instant that
//! no work before the watchdog arms can move, which a cool-down probe
//! takes so that its kill is never later than its route's hold ends (fix
//! rounds 0106b1r6, D1, and 0106b1r9, E1).
//!
//! A probe's instant is checked once more at the fork itself: under the
//! registry's lock, after the read of the process table that admits the
//! driver and as the last step before the spawn, on the caller's own clock
//! and on the monotonic one its kill is carried on. One already past on
//! either is not spawned ([`SpawnError::Late`]). Its watchdog reads that
//! instant again at least every [`SLICE`], so a pause before a wait moves
//! its kill by one slice at most (the return of fix round 0106b1r12).

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
    pub(super) fn at_fork(self) -> Option<Deadline> {
        match self {
            Arm::After(length) => Some(Deadline::After(length)),
            Arm::By(by) => {
                let ahead = Instant::now() < by.kill;
                (by.left)().filter(|_| ahead).map(|armed| Deadline::At {
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
        let pause = armed_pause();
        let thread = std::thread::spawn(move || {
            if self.waited(&rx, pause) {
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

    /// Wait on `rx` until this deadline: `true` once it is reached, `false`
    /// once the watchdog is disarmed. An absolute deadline is waited in
    /// slices of at most [`SLICE`], its target read again on the monotonic
    /// clock after each, so a pause between a sample and its wait, as a
    /// descheduled thread takes it, delays the kill by one slice at most,
    /// never by the whole sampled wait (the return of fix round 0106b1r12).
    /// `pause` is that pause, taken after the first sample: nothing outside
    /// `test-support` pauses.
    fn waited(self, rx: &mpsc::Receiver<()>, pause: Duration) -> bool {
        let slice = match self {
            Deadline::After(_) => Duration::MAX,
            Deadline::At { .. } => SLICE,
        };
        let mut wait = self.left();
        std::thread::sleep(pause);
        loop {
            match rx.recv_timeout(wait.min(slice)) {
                Err(RecvTimeoutError::Timeout) => {}
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return false,
            }
            wait = match self {
                Deadline::After(_) => return true,
                Deadline::At { .. } => self.left(),
            };
            if wait.is_zero() {
                return true;
            }
        }
    }
}

/// The longest an absolute deadline's watchdog waits before it reads its
/// target again: the most a pause before a wait can delay its kill.
const SLICE: Duration = Duration::from_millis(100);

#[cfg(feature = "test-support")]
thread_local! {
    /// What the watchdogs armed on this thread pause between sampling their
    /// wait and waiting it.
    static PAUSE: std::cell::Cell<Duration> = const { std::cell::Cell::new(Duration::ZERO) };
}

/// Make every watchdog armed on this thread from here pause `pause`
/// between sampling what remains to its deadline and waiting it, as a
/// descheduled watchdog thread does, so its kill lands that long after the
/// deadline; `Duration::ZERO` lifts it. For the engine's tests of a late
/// kill (fix round 0106b1r11, L3): another thread's watchdogs are unmoved.
#[cfg(feature = "test-support")]
pub fn pause_watchdogs_armed_here(pause: Duration) {
    PAUSE.with(|armed| armed.set(pause));
}

/// The pause a watchdog armed on this thread takes: none outside
/// `test-support`.
fn armed_pause() -> Duration {
    #[cfg(feature = "test-support")]
    let pause = PAUSE.with(std::cell::Cell::get);
    #[cfg(not(feature = "test-support"))]
    let pause = Duration::ZERO;
    pause
}

#[cfg(test)]
mod tests;
