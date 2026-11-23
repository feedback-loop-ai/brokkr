//! When a spawned driver is killed (fix rounds 0106b1r6, D1, and
//! 0106b1r9, E1): every attempt's length, counted from its watchdog's
//! arming, or a cool-down probe's one absolute deadline. The probe takes
//! that deadline once, as it begins, on both clocks at once: the engine's,
//! which its hold ends after and the fork reads, and the monotonic one,
//! which its watchdog kills at. The fork reads it once more, under the
//! registry's lock as the last step before the spawn, after the door,
//! record, charter and settlement work, and the watchdog's kill is that
//! same instant carried as it was taken, never "now plus what was left".
//! The fork refuses once either clock finds the deadline passed, and the
//! watchdog reads its instant again every slice, so neither a wall clock
//! stepped back nor a pause before a wait moves the kill past it (the
//! return of fix round 0106b1r12).
//! The route's hold ends on no deadline: only the probe's answer, its
//! renewed round or its tree's settlement ends it (fix round 0106b1r12, S1).

use std::path::Path;
use std::time::{Duration, Instant};

use brokkr_protocol::process::{By, DriverProcess, SpawnEnv, SpawnError};
use time::OffsetDateTime;

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
        Ok(ProbeDeadline { ends, kill })
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
