//! The reservation seals admission takes: the join of a run's next
//! attempt, the reseal of every record read as standing by one atomic
//! exchange, and the first admission's one seal that may make a record
//! (decision 0108 ruling 3, #440 fix rounds 440d5 and 440d6).
//!
//! Moved out of `admission`, which this would take past its 800 lines
//! (decision 0071 ruling 4).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::engine::broker::inventory::{Budget, Standing};
use crate::engine::broker::root::{identified, Commit, Identity, Root};

use super::{Claim, Reservation, Unknown, RESERVATION};

/// The run's reservation with `claim`'s attempt standing in it: the one
/// `standing` holds for that very worktree and run, which the inventory
/// read whole, or a new one, and whether it is the one that stood: only a
/// first admission, with no record read, may make the record; a run's next
/// attempt reseals the very record it read (decision 0108 ruling 3, #440
/// fix round 440d6).
pub(super) fn joined(claim: &Claim, standing: Vec<Standing>) -> (Reservation, bool) {
    let found = standing.into_iter().find_map(|held| {
        let ours = (held.repo == claim.repo) & (held.run == claim.run);
        (ours & held.whole.is_some()).then_some(held.whole)
    });
    let (mut reservation, reused) = match found.flatten() {
        Some(whole) => (whole, true),
        None => (
            Reservation {
                repo: claim.repo.clone(),
                run: claim.run.clone(),
                workdir: claim.workdir.clone(),
                artifacts: claim.artifacts.clone(),
                journal: claim.journal.clone(),
                attempts: BTreeMap::new(),
                settled: BTreeSet::new(),
            },
            false,
        ),
    };
    let reserved = claim.reserved.clone();
    reservation.attempts.insert(claim.attempt.clone(), reserved);
    (reservation, reused)
}

/// The run's directory below `root` and the path of the record in it, for
/// a seal that reads the record as it stands.
fn record_at(
    root: &Root,
    reservation: &Reservation,
) -> Result<(std::os::fd::OwnedFd, PathBuf), Unknown> {
    let run = root.below(&[reservation.repo.as_str(), reservation.run.as_str()])?;
    let at = root.path().join(&reservation.repo).join(&reservation.run);
    Ok((run, at))
}

/// `reservation` sealed over the very record that stands at its name, which
/// this reads as it stands: the one seal every path that read a record as
/// standing takes, so a record gone when its exchange runs is refused and
/// never made again ([`Root::reseal`]; decision 0108 ruling 3, #440 fix
/// round 440d6). Its caller holds the admission lock, and the exchange
/// closes the window the lock does not: an unlink from outside the broker.
pub(in crate::engine::broker) fn resealed(
    root: &Root,
    reservation: &Reservation,
    budget: &Budget,
) -> Result<Commit, Unknown> {
    let (run, at) = record_at(root, reservation)?;
    let (_, stood): (Reservation, Identity) = identified(&run, RESERVATION, &at)?;
    let bytes = serde_json::to_vec(reservation).or(Err(Unknown::Unsealed))?;
    root.reseal(&run, RESERVATION, &bytes, budget, stood)
}

/// `reservation` sealed in its run's directory below `root`, making the
/// record where none stands: only a first admission, which read none, may
/// ([`Root::seal`]; decision 0108 ruling 3, #440 fix round 440d6).
pub(super) fn sealed(
    root: &Root,
    reservation: &Reservation,
    budget: &Budget,
    stood: Option<Identity>,
) -> Result<Commit, Unknown> {
    let (run, _) = record_at(root, reservation)?;
    let bytes = serde_json::to_vec(reservation).or(Err(Unknown::Unsealed))?;
    match stood {
        Some(stood) => root.reseal(&run, RESERVATION, &bytes, budget, stood),
        None => root.seal(&run, RESERVATION, &bytes, budget),
    }
}
