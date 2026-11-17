//! Whether a writer reservation of a run stands, read from what stands in
//! the coordination root and never from the engine's memory (decision 0106
//! ruling 4, "no standing writer reservation"; decision 0071 ruling 5).
//!
//! The engine's lease is no witness. [`Engine::settle`](crate::engine::Engine)
//! takes it off the engine before it reads the journal, so an error there
//! drops the lease with its reservation unreleased, and a release refused
//! for an attempt that staked nothing returns Ok with the reservation
//! standing. Only the root says. This is the broker read the spend-time
//! re-read of slice 0106d calls, so the writer and the dispatcher read one
//! fact in one home.

use std::time::{Duration, Instant};

use super::{inventory, locked_existing, observed, startup, Budget, Root, Unadmitted};

/// Whether a writer reservation of the run `run` is not PROVEN absent: one
/// stands for it in the coordination root as dispatch observes it, or the
/// root cannot be opened, locked or read. Fail closed: only a root that
/// reads, and holds no standing attempt of the run, says none stands.
pub(in crate::engine) fn reservation_stands(run: &str) -> bool {
    let observed = observed();
    let Some(root) = observed.home.as_deref().and_then(observed.open) else {
        return true;
    };
    stands_in(&root, run).unwrap_or(true)
}

/// Whether the run `run` holds an attempt standing in `root`, read under
/// the admission lock taken before the startup deadline, nothing written:
/// where no lock file stands, nothing was ever written below the root, so
/// nothing stands. Every reservation is read through the view, the run's
/// own as any other's, for only whether an attempt stands is asked.
fn stands_in(root: &Root, run: &str) -> Result<bool, Unadmitted> {
    let deadline = startup(Instant::now(), Duration::MAX);
    let Some(_lock) = locked_existing(root.dir(), deadline)? else {
        return Ok(false);
    };
    let mut budget = Budget::new(deadline);
    // No worktree's key is empty, so no run is read as an admission's own.
    let found = inventory(root.dir(), root.path(), ["", run], &mut budget)?;
    Ok(found
        .standing
        .iter()
        .any(|held| held.run == run && !held.attempts.is_empty()))
}
