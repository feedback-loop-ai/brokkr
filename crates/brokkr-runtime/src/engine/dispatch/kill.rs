//! When a spawned driver is killed (fix round 0106b1r6, D1): every
//! attempt's length, counted from its watchdog's arming, or a cool-down
//! probe's one absolute deadline on the engine's clock. The probe's is
//! read again immediately before the fork, after the door, record,
//! charter and settlement work, and the watchdog is armed at that same
//! instant, so nothing that passes between the probe's start and its kill
//! moves the kill past the hold its route stands under.

use std::time::{Duration, Instant};

use brokkr_protocol::process::Deadline;
use time::OffsetDateTime;

/// A spawn's deadline, as the dispatch door carries it to the fork.
#[derive(Clone, Copy)]
pub(in crate::engine) enum SpawnDeadline<'a> {
    /// This long after the watchdog arms: every attempt's.
    After(Duration),
    /// A probe's: killed at `ends` on the engine's clock `now`.
    By {
        ends: OffsetDateTime,
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

impl SpawnDeadline<'_> {
    /// The watchdog's deadline, taken immediately before the fork: a
    /// probe's instant is read on the engine's clock once more, and one at
    /// or past it is not spawned.
    pub(super) fn armed(self) -> Result<Deadline, Unspawned> {
        match self {
            SpawnDeadline::After(length) => Ok(Deadline::After(length)),
            SpawnDeadline::By { ends, now } => {
                let left = ends - now();
                if left <= time::Duration::ZERO {
                    return Err(Unspawned::Late);
                }
                let armed = left.unsigned_abs();
                Ok(Deadline::At {
                    kill: Instant::now() + armed,
                    armed,
                })
            }
        }
    }
}
