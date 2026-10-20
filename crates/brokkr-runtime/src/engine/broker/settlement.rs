//! When an admitted attempt's lease is released (decision 0065 slice two,
//! U8a2a; D6, #403): only on positive proof that every process it owned
//! has settled, never on an end in error, an engine's death, a free lock,
//! an event type alone or a field that is absent.
//!
//! The engine that ran the attempt holds that proof as #403 types it: the
//! [`Cleanup`] each of its drivers' reports carried, which it witnesses as
//! each driver ends. The moment it witnesses a driver whose cleanup is
//! unresolved, it seals that into the attempt's own entry of the protected
//! reservation ([`Witnessed::Unresolved`]), before the attempt's terminal
//! event is journaled, and never releases the attempt. Once the attempt's
//! body ran, its terminal event is journaled and every driver it ran ended
//! settled, it seals the positive settlement fact there
//! ([`Witnessed::Settled`], M1), and only then releases the attempt, which
//! release refuses without that fact. A fact that cannot be sealed keeps
//! the reservation standing, as a release that cannot be does, and under
//! the fence rule fails the run with that refusal ([`Unreleased`]; R2 of
//! run four's review).
//!
//! An engine that died before releasing leaves only durable facts: the
//! reservation, and the journal it names, which recovery reads only for
//! the crash between the positive fact and the release. Under the
//! admission lock, each later admission releases an attempt only where its
//! reservation holds that fact and its journal, opened read-only, shows a
//! terminal event #403 journals only once its tree is proven over — a
//! success or a failure, which [`brokkr_protocol::AttemptReport::settled_outcome`]
//! yields only under [`Cleanup::Settled`]. An indeterminate end proves
//! nothing there; nor does a journal that cannot be read, that is not a
//! regular file, or that any participant's writer could have written (R2).
//! Such an attempt keeps its reservation, and keeps refusing.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_protocol::Cleanup;
use brokkr_store::Store;
use thiserror::Error;

use super::admission::{
    amend, release, sealed, settled, Held, Reservation, Unadmitted, Unknown, Witnessed, Writer,
};
use super::exclusion::{everywhere, reaches};
use super::historical::regular;
use super::inventory::{Budget, Standing, Writing};
use super::root::{Commit, Root};
use super::startup;
use crate::engine::{Engine, EngineError};

/// How an attempt's terminal event ended it, as its settlement reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    /// A success or a failure: journaled only once its tree is proven over.
    Certified,
    /// An indeterminate end: nothing it carries, or lacks, proves the tree
    /// over.
    Uncertain,
}

/// Why an attempt's settlement fails its run (L1 of U8a2c's review): its
/// lease was never released — its owned processes not proven settled, or
/// its positive settlement fact or release not sealed — so its reservation
/// stands, refusing every later writer it would; or its release committed,
/// but a seal of its reservation, its release's own or an earlier one, was
/// not proven durable.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}: {}", .lease.said(), .cause)]
pub struct Unreleased {
    lease: Lease,
    cause: Unadmitted,
}

/// Whether a failed settlement left its lease standing or released it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lease {
    Standing,
    Released,
}

impl Lease {
    /// What the operator is told of it.
    fn said(self) -> &'static str {
        match self {
            Lease::Standing => {
                "the attempt's writer lease stands, for its settlement was not sealed"
            }
            Lease::Released => {
                "the attempt's writer lease was released, but a seal of its reservation was not \
                 proven durable"
            }
        }
    }
}

impl Unreleased {
    /// A settlement refused by `cause`, its lease left standing.
    pub(in crate::engine) fn standing(cause: Unadmitted) -> Unreleased {
        let lease = Lease::Standing;
        Unreleased { lease, cause }
    }

    /// A settlement whose release committed, a seal not proven by `cause`.
    pub(in crate::engine) fn released(cause: Unadmitted) -> Unreleased {
        let lease = Lease::Released;
        Unreleased { lease, cause }
    }
}

impl Engine {
    /// Release the lease of the attempt `attempt`, which ended in `ran`,
    /// where this engine proves it settled ([`proven`]): its positive
    /// settlement fact sealed first ([`proved`]), then the release. An
    /// attempt that ended in an error, or whose fact or release cannot be
    /// sealed, keeps its reservation standing; the latter fails the run by
    /// [`Unreleased`] where the fence lifted, or the evidence at stake as
    /// the attempt was admitted, asks it. So does a seal of its reservation
    /// committed without its directory proven synced, as it was admitted,
    /// marked or released (R1 of U8a2a's fourth run).
    pub(in crate::engine) fn settle(
        &mut self,
        attempt: &str,
        ran: Result<(), EngineError>,
    ) -> Result<(), EngineError> {
        let held = self.held.take();
        ran?;
        let Some(held) = held else {
            return Ok(());
        };
        let events = self.store.load(&self.run_id)?;
        let deadline = startup(Instant::now(), Duration::MAX);
        // How the release committed, where one was made.
        let released = match proven(&held, &events, attempt) {
            true => proved(&held, deadline)
                .and_then(|fact| Ok(Some(fact.and(release(&held, deadline)?)))),
            false => Ok(None),
        };
        let unsynced = || Unadmitted::from(Unknown::Unsynced);
        let settled = match (released, held.unsynced) {
            (Ok(Some(Commit::Synced) | None), false) => Ok(()),
            (Ok(Some(Commit::Synced)), true) | (Ok(Some(Commit::Unsynced)), _) => {
                Err(Unreleased::released(unsynced()))
            }
            (Ok(None), true) => Err(Unreleased::standing(unsynced())),
            (Err(refused), _) => Err(Unreleased::standing(refused)),
        };
        match settled {
            Err(unreleased) if everywhere() | held.staked => Err(unreleased.into()),
            _ => Ok(()),
        }
    }
}

/// The lease `held`'s attempt's positive settlement fact sealed in its
/// reservation under the admission lock taken before `deadline` (M1);
/// refused where the reservation no longer names the attempt standing, or
/// names its cleanup unresolved.
pub(super) fn proved(held: &Held, deadline: Instant) -> Result<Commit, Unadmitted> {
    amend(held, deadline, |mut reservation| {
        let reserved = reservation.attempts.get_mut(&held.attempt);
        let reserved = reserved.ok_or(Unknown::Lost)?;
        let settling = reserved.cleanup != Witnessed::Unresolved;
        settling.then_some(()).ok_or(Unknown::Unproven)?;
        reserved.cleanup = Witnessed::Settled;
        Ok(reservation)
    })
}

/// A driver of the attempt in flight under the lease `held` ended with
/// `cleanup` (#403): the lease is no longer proven settled where it is
/// unresolved, and its reservation is sealed saying so the first time
/// (R2), under the admission lock taken before the startup deadline. A
/// seal that fails leaves the engine's own copy unproven, so this engine
/// still never releases it; one committed without its directory proven
/// synced is kept on the lease, which then fails its run under the fence
/// rule as it settles (R1 of U8a2a's fourth run).
pub(in crate::engine) fn witnessed(held: &mut Option<Held>, cleanup: &Cleanup) {
    witnessed_by(held, cleanup, startup(Instant::now(), Duration::MAX));
}

/// [`witnessed`], its seal taken before `deadline`.
pub(super) fn witnessed_by(held: &mut Option<Held>, cleanup: &Cleanup, deadline: Instant) {
    let (Some(held), Cleanup::Unresolved { .. }) = (held, cleanup) else {
        return;
    };
    if !std::mem::replace(&mut held.unproven, true) {
        let marked = amend(held, deadline, |reservation| {
            Ok(unresolved(reservation, &held.attempt))
        });
        held.unsynced |= marked == Ok(Commit::Unsynced);
    }
}

/// `reservation` with its standing attempt `attempt` witnessed unresolved.
fn unresolved(mut reservation: Reservation, attempt: &str) -> Reservation {
    if let Some(reserved) = reservation.attempts.get_mut(attempt) {
        reserved.cleanup = Witnessed::Unresolved;
    }
    reservation
}

/// Whether this engine proves the attempt `attempt` it held under `held`
/// settled: every driver it ran ended settled, and its terminal event is
/// journaled in `events`.
fn proven(held: &Held, events: &[EventEnvelope], attempt: &str) -> bool {
    !held.unproven & ending(events, attempt).is_some()
}

/// How the attempt `attempt`'s terminal event in `events` ended it, where
/// one is journaled.
fn ending(events: &[EventEnvelope], attempt: &str) -> Option<Ending> {
    let ended = events.iter().rev().find(|event| {
        let terminal = matches!(
            event.event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        );
        terminal & (event.attempt_id.as_deref() == Some(attempt))
    })?;
    let uncertain = ended.event_type == EventType::EffectIndeterminate;
    Some(if uncertain {
        Ending::Uncertain
    } else {
        Ending::Certified
    })
}

/// Every reservation of `found`, each of its standing attempts its durable
/// facts prove over ([`certified`]) moved to its settled ones, all within
/// `budget` (M7): the reservation sealed again below `root` where the
/// inventory read it whole, and otherwise settled for this admission alone
/// and never sealed, as another build wrote it (decision 0090); and whether
/// every seal it made was synced.
pub(super) fn recovered(
    root: &Root,
    found: Vec<Standing>,
    incoming: &[Writer],
    budget: &Budget,
) -> Result<(Vec<Standing>, Commit), Unknown> {
    // Every participant's writers, the incoming attempt's and every
    // standing one's: recovery trusts a reservation's journal only where
    // none of them could have forged its unkeyed hash chain (R2).
    let mut writers: Vec<Writing> = incoming.iter().map(Writing::from).collect();
    writers.extend(found.iter().flat_map(Standing::writers).cloned());
    let writers: Vec<&Writing> = writers.iter().collect();
    let (mut standing, mut commit) = (Vec::with_capacity(found.len()), Commit::Synced);
    for mut reservation in found {
        let proven = certified(&reservation, &writers, budget);
        budget.check()?;
        if proven.is_empty() {
            standing.push(reservation);
            continue;
        }
        let Some(whole) = reservation.whole.take() else {
            standing.push(reservation.released(&proven));
            continue;
        };
        let settle = |held, attempt: &String| settled(held, attempt);
        let recovered = proven.iter().try_fold(whole, settle)?;
        commit = commit.and(sealed(root, &recovered, budget)?);
        standing.push(Standing::from(recovered));
    }
    Ok((standing, commit))
}

/// The standing attempts of `reservation` whose engine sealed their
/// positive settlement fact and whose terminal events its journal, opened
/// read-only, certifies; none where it cannot be read.
fn certified(reservation: &Standing, writers: &[&Writing], budget: &Budget) -> Vec<String> {
    if reservation.attempts.is_empty() {
        return Vec::new();
    }
    let events =
        journal(&reservation.journal, &reservation.run, writers, budget).unwrap_or_default();
    let certified = (reservation.proved())
        .filter(|attempt| ending(&events, attempt) == Some(Ending::Certified));
    certified.cloned().collect()
}

/// The run `run`'s events in the journal at `path`, read only, waiting on
/// it no longer than `budget` leaves. None where any participant's writer
/// could reach the journal, its directory or a WAL sibling: an unkeyed
/// hash chain a writer can forge proves nothing against it (R2); and none
/// where the deadline passed, or the journal or a sibling that stands is
/// not a regular file of one link, opened without following a link or
/// blocking, so a FIFO in its place cannot hold the lock past the deadline
/// (L5).
fn journal(
    path: &Path,
    run: &str,
    writers: &[&Writing],
    budget: &Budget,
) -> Option<Vec<EventEnvelope>> {
    if reachable(path, writers) {
        return None;
    }
    budget.check().ok()?;
    let standing = |at: &PathBuf| std::fs::symlink_metadata(at).is_err() | regular(at).is_some();
    let siblings = [sibling(path, "-wal"), sibling(path, "-shm")];
    (regular(path).is_some() & siblings.iter().all(standing)).then_some(())?;
    let mut store = Store::open_read_only(path).ok()?;
    store.set_patience(budget.left()).ok()?;
    store.load(run).ok()
}

/// The SQLite sibling of the journal at `path` that ends in `suffix`.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// Whether `path`, its directory, or its SQLite `-wal`/`-shm` siblings lie
/// within any participant's writer reach.
fn reachable(path: &Path, writers: &[&Writing]) -> bool {
    let mut targets = vec![
        path.to_path_buf(),
        sibling(path, "-wal"),
        sibling(path, "-shm"),
    ];
    targets.extend(path.parent().map(Path::to_path_buf));
    targets.iter().any(|target| reaches(writers, target))
}
