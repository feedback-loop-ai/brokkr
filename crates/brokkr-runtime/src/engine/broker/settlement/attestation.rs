//! Settlement's one exception to positive proof (U8e3; operator ruling
//! 2026-10-21, "A"), moved out of `settlement.rs`, which it would take past
//! its 800 lines (operator ruling of 2026-10-11): the operator's release of
//! a run's attempts that ended indeterminate, on the operator's journaled
//! attestation that every process each owned has ended.

use std::path::Path;
use std::time::{Duration, Instant};

use brokkr_core::EventEnvelope;
use brokkr_store::Store;

use super::{
    beside, ending, inventory, locked, recorded_settled, sealed, settled, startup, Budget, Commit,
    Ending, Refused, Reservation, Root, Standing, Unadmitted, Unknown, Unreleased, Witnessed,
    RESERVATION,
};
use crate::engine::EngineError;

/// U8e3 (operator ruling 2026-10-21, "A"): settlement's one exception to
/// positive proof, which nothing else may use. Every attempt the run `run`
/// holds standing, in a reservation naming `store`'s journal, released on
/// the operator's attestation that every process each owned has ended.
/// Under the admission lock in the coordination `root`, the reservations
/// are read whole and each attempt judged against the run's journal; then
/// `attest` journals the attestation on the journal as judged; and only
/// then is each attempt's cleanup sealed Settled, as released by the
/// operator, and the attempt settled. Nothing is folded, replayed or
/// journaled of an attempt, which stays as it ended. Refused, nothing
/// journaled or sealed, where no attempt of the run stands, where one has
/// not ended or ended other than indeterminate, or where one's every
/// driver is sealed Settled ([`Refused`]); and, as any seal is, where the
/// lock, the inventory or the seal fails, or a reservation is not read
/// whole. One committed without its directory proven synced was released,
/// its seal not proven durable.
pub(in crate::engine) fn attested(
    root: &Root,
    run: &str,
    store: &mut Store,
    attest: impl FnOnce(&mut Store, &[EventEnvelope]) -> Result<(), EngineError>,
) -> Result<(), EngineError> {
    let deadline = startup(Instant::now(), Duration::MAX);
    let lock = locked(root.dir(), deadline).map_err(Unadmitted::from);
    let _lock = lock.map_err(Unreleased::standing)?;
    let mut budget = Budget::new(deadline);
    let reservations = reserved_for(root, run, store.path(), &mut budget).map_err(Unadmitted::from);
    let reservations = reservations.map_err(Unreleased::standing)?;
    let events = store.load(run)?;
    judged(run, &reservations, &events).map_err(Unreleased::refused)?;
    attest(store, &events)?;
    let mut commit = Commit::Synced;
    for reservation in reservations {
        let released = released_by_operator(reservation).map_err(Unadmitted::from);
        let released = released.map_err(Unreleased::standing)?;
        let seal = sealed(root, &released, &budget).map_err(Unadmitted::from);
        commit = commit.and(seal.map_err(Unreleased::standing)?);
    }
    let unsynced = Unreleased::released(Unknown::Unsynced.into());
    Ok((commit == Commit::Synced).then_some(()).ok_or(unsynced)?)
}

/// Every reservation of the run `run` in the coordination `root` whose
/// journal is the one at `journal`, holding an attempt standing, as the
/// inventory reads them within `budget`, each read whole; refused where one
/// was read only through the view, for that is never sealed (decision
/// 0090).
fn reserved_for(
    root: &Root,
    run: &str,
    journal: &Path,
    budget: &mut Budget,
) -> Result<Vec<Reservation>, Unknown> {
    // No worktree's key is empty, so no run is read as an admission's own:
    // a record this build writes is read whole, and any other through the
    // view.
    let found = inventory(root.dir(), root.path(), ["", run], budget)?;
    let journal = beside(journal);
    let ours = found
        .standing
        .into_iter()
        .filter(|held| (held.run == run) & (held.journal == journal) & !held.attempts.is_empty());
    let whole = |held: Standing| {
        let at = root.path().join(&held.repo).join(run).join(RESERVATION);
        held.whole.ok_or(Unknown::Participant(at))
    };
    ours.map(whole).collect()
}

/// Why the operator's release of the run `run`'s standing attempts in
/// `reservations` is refused against its journal's `events`, if it is:
/// only an attempt that ended indeterminate, a driver of it never sealed
/// Settled, is the operator's to release.
fn judged(
    run: &str,
    reservations: &[Reservation],
    events: &[EventEnvelope],
) -> Result<(), Refused> {
    let unreserved = Refused::Unreserved(run.to_string());
    (!reservations.is_empty()).then_some(()).ok_or(unreserved)?;
    for (attempt, reserved) in reservations.iter().flat_map(|held| &held.attempts) {
        let attempt = attempt.clone();
        match (recorded_settled(reserved), ending(events, &attempt)) {
            (true, None) => Err(Refused::Recoverable(attempt)),
            (true, Some(_)) => Err(Refused::Settled(attempt)),
            (false, None) => Err(Refused::Running(attempt)),
            (false, Some(Ending::Certified)) => Err(Refused::Certified(attempt)),
            (false, Some(Ending::Uncertain)) => Ok(()),
        }?;
    }
    Ok(())
}

/// `reservation` with every attempt it holds standing released on the
/// operator's attestation: each one's cleanup sealed Settled, as released
/// by the operator, then settled as any proven attempt is ([`settled`]).
fn released_by_operator(mut reservation: Reservation) -> Result<Reservation, Unknown> {
    let attempts: Vec<String> = reservation.attempts.keys().cloned().collect();
    let standing = reservation.attempts.values_mut();
    standing.for_each(|reserved| reserved.cleanup = Witnessed::Settled);
    attempts
        .iter()
        .try_fold(reservation, |held, attempt| settled(held, attempt))
}
