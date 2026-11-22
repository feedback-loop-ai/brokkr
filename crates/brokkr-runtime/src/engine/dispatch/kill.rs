//! When a spawned driver is killed (fix rounds 0106b1r6, D1, and
//! 0106b1r9, E1): every attempt's length, counted from its watchdog's
//! arming, or a cool-down probe's one absolute deadline. The probe takes
//! that deadline once, as it begins, on both clocks at once: the engine's,
//! which its hold ends after and the fork reads, and the monotonic one,
//! which its watchdog kills at. The fork reads it once more, under the
//! registry's lock as the last step before the spawn, after the door,
//! record, charter and settlement work, and the watchdog's kill is that
//! same instant carried as it was taken, never "now plus what was left",
//! so nothing that passes between the probe's start and its kill moves the
//! kill past the hold its route stands under.

use std::path::Path;
use std::time::{Duration, Instant};

use brokkr_protocol::process::{
    By, DriverProcess, SpawnEnv, SpawnError, SHUTDOWN_GRACE, SHUTDOWN_SETTLE,
};
use time::OffsetDateTime;

/// How far a probe's stored hold outlasts its deadline and the shutdown
/// grace together (fix round 0106b1r10, H2). The stored end is only a
/// crash fallback: a live engine ends the hold on evidence, once its
/// settlement proves the probe's tree ended or its answer or renewed round
/// commits, and the stored end bounds how long a crashed holder blocks the
/// route. The margin is named once, here.
pub(in crate::engine) const HOLD_MARGIN: Duration = Duration::from_secs(60);

/// The longest a live engine's own work past the deadline may take before
/// its settlement can end the hold, allowed for the margin: the fork's
/// registration and the pipe readers' start before the watchdog arms, the
/// watchdog's wake, and the journal write that ends the hold.
const ARM_WORK: Duration = Duration::from_secs(10);

/// The forward step of the engine's wall clock, against the monotonic one
/// its watchdog waits on, that the margin absorbs while a probe is in
/// flight. A larger step, like a host suspend longer than the margin, can
/// end the fallback while a frozen driver lives: the named residual.
const CLOCK_STEP: Duration = Duration::from_secs(30);

/// What the margin must exceed: the shutdown grace, the killed tree's
/// settle and drain after it, the engine's own work past the deadline, and
/// the clock step it absorbs.
const MARGIN_BOUND: Duration = Duration::from_secs(
    SHUTDOWN_GRACE.as_secs()
        + SHUTDOWN_SETTLE.as_secs()
        + ARM_WORK.as_secs()
        + CLOCK_STEP.as_secs(),
);

const _: () = assert!(HOLD_MARGIN.as_secs() > MARGIN_BOUND.as_secs());

/// A probe whose timeout puts its deadline past what the host's monotonic
/// clock can represent (fix round 0106b1r10, H4): the seat accepts any
/// positive whole number of seconds, so it is refused here, typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "its {seconds}s timeout puts its deadline past what this host's clock can represent, so \
     nothing was held or dispatched"
)]
pub(in crate::engine) struct Unbounded {
    seconds: u64,
}

/// A probe's one deadline, taken as it begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) struct ProbeDeadline {
    /// On the engine's clock: what the fork reads.
    pub(in crate::engine) ends: OffsetDateTime,
    /// On the monotonic clock: where the probe's watchdog kills.
    pub(in crate::engine) kill: Instant,
    /// On the engine's clock: the hold's crash fallback, the deadline plus
    /// the shutdown grace plus [`HOLD_MARGIN`], which the store keeps as the
    /// hold's end (fix round 0106b1r10, H2).
    pub(in crate::engine) fallback: OffsetDateTime,
}

impl ProbeDeadline {
    /// `length` after the probe began: `began` on the monotonic clock, read
    /// before `now` on the engine's, so a pause between the two readings
    /// moves the kill earlier, never later.
    /// A length the monotonic clock cannot carry past `began` is refused
    /// in its own words before anything is held or dispatched, never
    /// added unchecked, which would panic (fix round 0106b1r10, H4).
    pub(in crate::engine) fn begun(
        began: Instant,
        now: OffsetDateTime,
        length: Duration,
    ) -> Result<Self, Unbounded> {
        let kill = began.checked_add(length).ok_or(Unbounded {
            seconds: length.as_secs(),
        })?;
        let seconds = i64::try_from(length.as_secs()).unwrap_or(i64::MAX);
        let ends = now.saturating_add(time::Duration::seconds(seconds));
        let past = (SHUTDOWN_GRACE + HOLD_MARGIN).as_secs() as i64;
        Ok(ProbeDeadline {
            ends,
            kill,
            fallback: ends.saturating_add(time::Duration::seconds(past)),
        })
    }

    /// What remains of it at `now` on the engine's clock: `None` at or
    /// past it.
    fn left(self, now: OffsetDateTime) -> Option<Duration> {
        let left = self.ends - now;
        (left > time::Duration::ZERO).then(|| left.unsigned_abs())
    }

    /// The deadline the spawn takes: the fork reads `left`, and the
    /// watchdog kills at this deadline's own instant, carried as it was
    /// taken.
    fn watched<'a>(self, left: &'a dyn Fn() -> Option<Duration>) -> By<'a> {
        By {
            kill: self.kill,
            left,
        }
    }
}

/// A spawn's deadline, as the dispatch door carries it to the fork.
#[derive(Clone, Copy)]
pub(in crate::engine) enum SpawnDeadline<'a> {
    /// This long after the watchdog arms: every attempt's.
    After(Duration),
    /// A probe's, read at the fork on the engine's clock `now`.
    By {
        deadline: ProbeDeadline,
        now: &'a (dyn Fn() -> OffsetDateTime + Send + Sync),
    },
}

/// Why a driver past every check was not spawned.
#[derive(Debug, thiserror::Error)]
pub(in crate::engine) enum Unspawned {
    /// A check or the spawn itself refused, in its own words.
    #[error("{0}")]
    Refused(String),
    /// The probe's deadline had passed by the fork.
    #[error("its deadline passed before its driver could start")]
    Late,
}

impl From<SpawnError> for Unspawned {
    fn from(error: SpawnError) -> Self {
        match error {
            SpawnError::Late { .. } => Unspawned::Late,
            SpawnError::EmptyCommand | SpawnError::Spawn { .. } | SpawnError::Unwatched { .. } => {
                Unspawned::Refused(error.to_string())
            }
        }
    }
}

impl SpawnDeadline<'_> {
    /// Spawn `argv` under this deadline: a probe's is read on the engine's
    /// clock at the fork itself, and one at or past it is not spawned.
    pub(super) fn spawn(
        self,
        argv: &[String],
        workdir: &Path,
        env: &SpawnEnv,
    ) -> Result<DriverProcess, Unspawned> {
        let spawned = match self {
            SpawnDeadline::After(length) => DriverProcess::spawn(argv, workdir, Some(length), env),
            SpawnDeadline::By { deadline, now } => {
                let left = || deadline.left(now());
                DriverProcess::spawn_by(argv, workdir, deadline.watched(&left), env)
            }
        };
        spawned.map_err(Unspawned::from)
    }
}

#[cfg(test)]
mod tests;
