//! When a driver's watchdog kills it: a length counted from the moment the
//! watchdog arms, which every attempt takes, or one absolute instant that
//! no work before the watchdog arms can move, which a cool-down probe
//! takes so that its kill is never later than its route's hold ends (fix
//! round 0106b1r6, D1).

use std::time::{Duration, Instant};

use super::tree::Host;
use super::{DriverProcess, Limits, SpawnEnv, SpawnError};

impl DriverProcess {
    /// [`DriverProcess::spawn`] under `deadline`, which may be an absolute
    /// instant: its watchdog then kills at that instant, whatever passed
    /// between the caller's taking it and the watchdog's arming.
    pub fn spawn_by(
        command: &[String],
        workdir: &std::path::Path,
        deadline: Deadline,
        env: &SpawnEnv,
    ) -> Result<Self, SpawnError> {
        let limits = Limits::DEFAULT;
        Self::spawn_limited(command, workdir, Some(deadline), env, Host::REAL, limits)
    }
}

/// A driver's deadline, as [`DriverProcess::spawn_by`] arms it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    /// This long after the watchdog arms.
    After(Duration),
    /// At `kill` on the monotonic clock, however late the watchdog arms.
    /// `armed` is what remained to it when the caller took it, the length
    /// the kill's words name.
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
}

#[cfg(test)]
mod tests;
