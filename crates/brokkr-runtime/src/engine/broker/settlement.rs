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
//!
//! Its engine also keeps one settlement record per driver in the attempt's
//! own entry (U8e2; operator ruling 2026-10-21, "A"): written Settling at
//! the dispatch door before the driver spawns ([`spawning`]), and sealed
//! Settled once its report carries #403's proof that its owned processes
//! settled ([`ended`]), before the fold reads its ledgers. An engine
//! restarted with the attempt in flight takes its lease again only where
//! every driver it recorded is sealed so ([`Engine::readopted`]).
//!
//! One exception, which nothing else may use (U8e3; operator ruling
//! 2026-10-21, "A"): the operator's release of a run's attempts that ended
//! with no such proof, on the operator's journaled attestation that every
//! process each owned has ended ([`attested`]). It folds nothing, replays
//! nothing and leaves each attempt as it ended.

use std::io::ErrorKind::NotFound;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_protocol::Cleanup;
use brokkr_store::Store;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::admission::{
    amend, beside, identity, release, reserved_at, sealed, settled, Authored, Held, Protects,
    Reservation, Reserved, Unadmitted, Unknown, Witnessed, Writer, RESERVATION,
};
use super::exclusion::reaches;
use super::fold::Begun;
use super::historical::regular;
use super::inventory::{inventory, Budget, Standing, Writing};
use super::root::{locked, read, Commit, Root};
use super::{observed, startup};
use crate::engine::artifacts::{self, Evidence, Unpublished};
use crate::engine::{Engine, EngineError};

mod attestation;
pub(in crate::engine) use attestation::attested;

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
/// not proven durable; or the responses its brokers retained were not
/// published (U8c; CR2), its lease standing or released as it says; or the
/// operator's release of it was refused ([`Refused`]; U8e3).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}: {}", said(*.lease, .cause), .cause)]
pub struct Unreleased {
    lease: Lease,
    cause: Cause,
}

/// Whether a failed settlement left its lease standing or released it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) enum Lease {
    Standing,
    Released,
}

/// Why a settlement failed: a seal refused or not proven, a retained
/// response not published, or the operator's release refused (U8e3).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
enum Cause {
    #[error(transparent)]
    Unadmitted(Unadmitted),
    #[error(transparent)]
    Unpublished(Unpublished),
    #[error(transparent)]
    Refused(Refused),
}

/// Why the operator's release of a run's attempts is refused, nothing of
/// it sealed (U8e3; operator ruling 2026-10-21): no attempt of the run's
/// stands; one has not ended; one ended a success or a failure, not
/// indeterminate; one whose every driver is sealed Settled is U8e2's to
/// recover, in flight or, once ended, no one's; or the journal moved
/// beneath the release.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Refused {
    #[error("run '{0}' holds no standing reservation, so there is nothing to release")]
    Unreserved(String),
    #[error(
        "attempt '{0}' has not ended; release only an attempt that ended, once every process \
         it owned has"
    )]
    Running(String),
    #[error(
        "attempt '{0}' ended a success or a failure, not indeterminate; release only an \
         attempt that ended indeterminate"
    )]
    Certified(String),
    #[error(
        "every driver of attempt '{0}' is sealed Settled, so an engine restarted with it in \
         flight recovers it: resume the run instead"
    )]
    Recoverable(String),
    #[error(
        "every driver of attempt '{0}' is sealed Settled, so it is not the operator's to \
         release, and it has ended, so resuming the run does not recover it: its reservation \
         stands"
    )]
    Settled(String),
    #[error(
        "the journal moved beneath the release, so something may still be driving the run: look \
         with `brokkr runs` before releasing"
    )]
    Moved,
}

/// What the operator is told of a failed settlement's `lease`, and of its
/// retained responses where its `cause` is that they were not published.
fn said(lease: Lease, cause: &Cause) -> &'static str {
    match (cause, lease) {
        (Cause::Refused(_), Lease::Standing | Lease::Released) => {
            "the operator's release was refused, and the run's writer lease stands"
        }
        (Cause::Unadmitted(_), Lease::Standing) => {
            "the attempt's writer lease stands, for its settlement was not sealed"
        }
        (Cause::Unadmitted(_), Lease::Released) => {
            "the attempt's writer lease was released, but a seal of its reservation was not \
             proven durable"
        }
        (Cause::Unpublished(_), Lease::Standing) => {
            "the attempt's retained responses were not published, and its writer lease stands"
        }
        (Cause::Unpublished(_), Lease::Released) => {
            "the attempt's retained responses were not published, though its writer lease \
             was released"
        }
    }
}

impl Unreleased {
    /// A settlement refused by `cause`, its lease left standing.
    pub(in crate::engine) fn standing(cause: Unadmitted) -> Unreleased {
        let (lease, cause) = (Lease::Standing, Cause::Unadmitted(cause));
        Unreleased { lease, cause }
    }

    /// A settlement whose release committed, a seal not proven by `cause`.
    pub(in crate::engine) fn released(cause: Unadmitted) -> Unreleased {
        let (lease, cause) = (Lease::Released, Cause::Unadmitted(cause));
        Unreleased { lease, cause }
    }

    /// A settlement whose attempt's retained responses were not published,
    /// refused by `cause`, its lease as `lease` says.
    pub(in crate::engine) fn unpublished(lease: Lease, cause: Unpublished) -> Unreleased {
        let cause = Cause::Unpublished(cause);
        Unreleased { lease, cause }
    }

    /// The operator's release refused by `refused`, the lease standing.
    pub(in crate::engine) fn refused(refused: Refused) -> Unreleased {
        let (lease, cause) = (Lease::Standing, Cause::Refused(refused));
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
    /// marked or released (R1 of U8a2a's fourth run). An attempt proven
    /// settled has the responses its brokers retained published first,
    /// under its lease ([`published`]); one refused fails the run by
    /// [`Unreleased::unpublished`] once the release is done or refused,
    /// saying which ([`leased`]) (U8c). Unlike a seal's, that refusal fails
    /// the run whether or not the fence lifted or evidence was at stake: a
    /// response a verified ledger cites, or evidence of the attempt that
    /// cannot be read as it was sealed, is refused, never reconciled (CR2).
    /// An attempt that provisioned no plan has no directory there to read,
    /// and publishes nothing; but one whose run's directory in the
    /// coordination root cannot be opened is refused as unbound, staked or
    /// not, where before U8c such a run's refused seal went unsaid.
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
        let proven = proven(&held, &events, attempt);
        // U8c (CR2): the attempt's cited responses are published under its
        // lease, before its release, and only once every process it owned
        // is proven settled, so no broker still writes what is read.
        let published = match proven {
            true => published(&held),
            false => Ok(()),
        };
        // How the release committed, where one was made.
        let released = match proven {
            true => proved(&held, deadline)
                .and_then(|fact| Ok(Some(fact.and(release(&held, deadline)?)))),
            false => Ok(None),
        };
        let lease = leased(&released);
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
            Err(unreleased) if held.staked => Err(unreleased.into()),
            _ => Ok(published.map_err(|cause| Unreleased::unpublished(lease, cause))?),
        }
    }
}

/// Whether the lease of a settlement whose release ended in `released`
/// stands or was released: released where a release committed, synced or
/// not.
fn leased(released: &Result<Option<Commit>, Unadmitted>) -> Lease {
    match released {
        Ok(Some(_)) => Lease::Released,
        Ok(None) | Err(_) => Lease::Standing,
    }
}

/// The responses the attempt in flight under the lease `held` retained,
/// published below its artifact root as admitted (U8c; CR2): its evidence
/// read in its own directory, below its run's in the coordination root the
/// lease stands in, from that root's handle, never from HOME again.
fn published(held: &Held) -> Result<(), Unpublished> {
    evidenced(held, |evidence| {
        artifacts::published(evidence, held.root.as_ref())
    })?
}

/// What `read` makes of the evidence of the attempt in flight under the
/// lease `held`: its own directory, below its run's in the coordination
/// root the lease stands in, from that root's handle, never from HOME
/// again; unbound where that run's directory cannot be opened.
pub(super) fn evidenced<T>(
    held: &Held,
    read: impl FnOnce(&Evidence<'_>) -> T,
) -> Result<T, Unpublished> {
    let (run, at) = reserved_at(held).or(Err(Unpublished::Unbound))?;
    let owner = [held.repo.as_str(), held.run.as_str(), held.attempt.as_str()];
    let evidence = Evidence {
        run: &run,
        at,
        owner,
    };
    Ok(read(&evidence))
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

/// One driver's settlement record in its attempt's reservation (U8e2;
/// operator ruling 2026-10-21): the label of the site it was spawned for,
/// the journal head its fold begins from, which [`Engine::begun`] took
/// before it spawned, none where its site was sealed no broker, and what
/// its engine witnessed of its cleanup: Settling from before it spawns,
/// Settled once its report carried #403's proof. It names no process and
/// starts no scanner: it makes durable a proof the engine already holds,
/// and is no launch model (D6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Driver {
    pub(super) site: String,
    pub(super) begun: Option<(u64, String)>,
    pub(super) cleanup: Witnessed,
}

/// The driver about to spawn at the site `site` of the attempt in flight
/// under the lease `held`, which began at `begun`: its own settlement
/// record written Settling in the attempt's reservation under the
/// admission lock, beside every earlier driver's, before it spawns (U8e2),
/// so a settled earlier driver never stands for it. Refused, and so never
/// spawned, where the record cannot be sealed or its seal is not proven
/// synced: a restart could otherwise find only earlier drivers' records.
/// Refused as no longer standing where the attempt's positive settlement
/// fact is sealed, as a restarted engine seals it the moment it adopts the
/// attempt ([`adopted`]): under the same lock, so a driver either spawns
/// before the adoption, which then finds it Settling and adopts nothing,
/// or never spawns (H1 of U8e2's first run).
pub(in crate::engine) fn spawning(
    held: &Held,
    site: &str,
    begun: Option<&Begun>,
) -> Result<(), Unadmitted> {
    let begun = begun.map(|Begun(head)| head.clone());
    let deadline = startup(Instant::now(), Duration::MAX);
    let commit = amend(held, deadline, |mut reservation| {
        let reserved = reservation.attempts.get_mut(&held.attempt);
        let reserved = reserved.filter(|reserved| reserved.cleanup != Witnessed::Settled);
        let (site, cleanup) = (site.to_string(), Witnessed::Settling);
        let driver = Driver {
            site,
            begun,
            cleanup,
        };
        reserved.ok_or(Unknown::Lost)?.drivers.push(driver);
        Ok(reservation)
    })?;
    let synced = commit == Commit::Synced;
    synced
        .then_some(())
        .ok_or(Unadmitted::from(Unknown::Unsynced))
}

/// The driver at the site `site` of the attempt in flight under the lease
/// `held` ended with `cleanup` (#403): witnessed on the lease
/// ([`witnessed`]), then, where settled, its own settlement record sealed
/// Settled ([`settled_driver`]), both before the fold reads its ledgers.
pub(in crate::engine) fn ended(held: &mut Option<Held>, site: &str, cleanup: &Cleanup) {
    witnessed(held, cleanup);
    settled_driver(held, site, cleanup);
}

/// Where `cleanup` is settled, the settlement record of the driver at the
/// site `site` of the attempt in flight under the lease `held`, the last
/// written for that site, sealed Settled under the admission lock taken
/// before the startup deadline (U8e2). A seal that fails, or a driver no
/// record was written for, leaves no record Settled, so a restart never
/// recovers the attempt by it; one committed without its directory proven
/// synced is kept on the lease, as [`witnessed_by`] keeps its own.
fn settled_driver(held: &mut Option<Held>, site: &str, cleanup: &Cleanup) {
    let (Some(held), Cleanup::Settled) = (held, cleanup) else {
        return;
    };
    let deadline = startup(Instant::now(), Duration::MAX);
    let sealed = amend(held, deadline, |mut reservation| {
        let reserved = reservation.attempts.get_mut(&held.attempt);
        let drivers = &mut reserved.ok_or(Unknown::Lost)?.drivers;
        let driver = drivers.iter_mut().rfind(|driver| driver.site == site);
        driver.ok_or(Unknown::Lost)?.cleanup = Witnessed::Settled;
        Ok(reservation)
    });
    held.unsynced |= sealed == Ok(Commit::Unsynced);
}

/// What a restarted engine adopts of an attempt it finds in flight: the
/// lease taken again and the settlement records it adopted, or none.
pub(super) type Adoption = Option<(Held, Vec<Driver>)>;

impl Engine {
    /// U8e2 (operator ruling 2026-10-21): the lease on the attempt `attempt`
    /// this restarted engine finds in flight, taken again under the
    /// admission lock in the coordination root it observes as dispatch
    /// does, where [`adopted`] proves every driver of it settled, and those
    /// drivers' records. None, and no lease held, where no record proves it,
    /// or no coordination root is protected, for then the attempt took no
    /// reservation (C1). Refused, nothing sealed, where the worktree, the
    /// lock, the reservation, the artifact root or the seal fails, so a
    /// failure to adopt never reads as proof that is missing (L1 of U8e2's
    /// second run).
    pub(super) fn readopted(&self, attempt: &str) -> Result<Adoption, Unadmitted> {
        let observed = observed();
        let Some(root) = observed.home.as_deref().and_then(observed.open) else {
            return Ok(None);
        };
        let (_, repo) = keyed(&self.workdir())?;
        let deadline = startup(Instant::now(), Duration::MAX);
        let ids = [repo.as_str(), self.run_id.as_str(), attempt];
        adopted(&Arc::new(root), ids, deadline)
    }
}

/// Where the reservation of the run `run` on the worktree keyed `repo`
/// stands below the coordination `root`: its directory, entered as
/// [`Root::below`] enters it, and its path, the one home of that location
/// (L2 of U8e2's second run). Where not `made`, a directory that is missing
/// is not made, and the reservation is lost: a restart reads only what
/// stands, under the admission lock no hand removes it beneath.
pub(super) fn reserved_in(
    root: &Root,
    [repo, run]: [&str; 2],
    made: bool,
) -> Result<(OwnedFd, PathBuf), Unknown> {
    let at = root.path().join(repo).join(run);
    let missing = std::fs::symlink_metadata(&at).is_err_and(|error| error.kind() == NotFound);
    (made | !missing).then_some(()).ok_or(Unknown::Lost)?;
    Ok((root.below(&[repo, run])?, at))
}

/// The worktree at `path` as it canonically resolves, and the key its
/// runs' reservations stand under in the coordination root: the one home
/// of that derivation, for admission and a restart alike (L4 of U8e2's
/// first run).
pub(super) fn keyed(path: &Path) -> Result<(PathBuf, String), Unknown> {
    let workdir = std::fs::canonicalize(path).or(Err(Unknown::Worktree))?;
    let repo = canonical::sha256_bytes(workdir.as_os_str().as_bytes());
    Ok((workdir, repo))
}

/// The lease on the attempt `attempt` of the run `run` on the worktree
/// keyed `repo`, taken again in the coordination `root` under the admission
/// lock taken before `deadline`: its run admits again while its earlier
/// attempt stands, and nothing new is reserved or made. Only where its
/// reservation names it standing, no driver of it was witnessed unresolved,
/// and it recorded drivers, each sealed Settled; with those records. Under
/// that same lock the attempt's positive settlement fact is sealed, which
/// every driver's record proves, so no door of the attempt spawns again
/// ([`spawning`]), and the lease keeps the records it adopted, which its
/// release requires unchanged ([`unchanged`]). None otherwise, and nothing
/// sealed: a driver that may still run, or a run that reserved nothing,
/// keeps the attempt indeterminate and any reservation standing, as before
/// U8e2. Refused, nothing sealed, where the lock, the reservation, the
/// artifact root or the seal fails (L1 of U8e2's second run). Its artifact
/// root's identity is the one it stands at now, since admission's is not
/// durable; and its settlement is held to the fence rule as evidence at
/// stake.
pub(super) fn adopted(
    root: &Arc<Root>,
    [repo, run, attempt]: [&str; 3],
    deadline: Instant,
) -> Result<Adoption, Unadmitted> {
    let _lock = locked(root.dir(), deadline)?;
    let Some(mut reservation) = standing(root, [repo, run])? else {
        return Ok(None);
    };
    let reserved = reservation.attempts.get(attempt);
    let Some(reserved) = reserved.filter(|reserved| recorded_settled(reserved)) else {
        return Ok(None);
    };
    let drivers = reserved.drivers.clone();
    let artifacts = &reservation.artifacts;
    let protects = reserved.protects == Protects::Evidence;
    let identified = protects.then(|| identity(artifacts).map(|id| (artifacts.clone(), id)));
    let identified = identified.transpose()?;
    let adopting = reservation.attempts.get_mut(attempt).into_iter();
    adopting.for_each(|reserved| reserved.cleanup = Witnessed::Settled);
    let commit = sealed(root, &reservation, &Budget::new(deadline))?;
    let held = Held {
        coordination: Arc::clone(root),
        repo: repo.to_string(),
        run: run.to_string(),
        attempt: attempt.to_string(),
        root: identified,
        unproven: false,
        staked: true,
        unsynced: commit == Commit::Unsynced,
        workdir: reservation.workdir.clone(),
        authored: Authored::default(),
        adopted: Some(drivers.clone()),
    };
    Ok(Some((held, drivers)))
}

/// The reservation of the run `run` on the worktree keyed `repo` in the
/// coordination `root`, read as it stands, nothing made: none where its
/// directory is missing, for its run reserved nothing there; refused where
/// it cannot be read, or names another run.
fn standing(root: &Root, [repo, run]: [&str; 2]) -> Result<Option<Reservation>, Unknown> {
    let (dir, at) = match reserved_in(root, [repo, run], false) {
        Err(Unknown::Lost) => return Ok(None),
        found => found?,
    };
    let reservation: Reservation = read(&dir, RESERVATION, &at)?;
    let ours = (reservation.repo == repo) & (reservation.run == run);
    let refused = Unknown::Participant(at.join(RESERVATION));
    ours.then_some(Some(reservation)).ok_or(refused)
}

/// Whether `reserved` proves every driver of its attempt settled: none
/// witnessed unresolved, and drivers recorded, each sealed Settled.
fn recorded_settled(reserved: &Reserved) -> bool {
    let settled = |driver: &Driver| driver.cleanup == Witnessed::Settled;
    let recorded = !reserved.drivers.is_empty();
    let unresolved = reserved.cleanup == Witnessed::Unresolved;
    !unresolved & recorded & reserved.drivers.iter().all(settled)
}

/// Refused where the lease `held` is one a restarted engine adopted and
/// its attempt's settlement records in `reservation` are no longer exactly
/// those it adopted, every one Settled: the release, under the admission
/// lock, never removes a driver that came after the adoption (H1 of U8e2's
/// first run). A lease admitted, not adopted, is not refused here.
pub(super) fn unchanged(reservation: &Reservation, held: &Held) -> Result<(), Unknown> {
    let reserved = reservation.attempts.get(&held.attempt);
    let drivers = reserved.map(|reserved| &reserved.drivers);
    let adopted = held.adopted.as_ref();
    let unchanged = adopted.is_none_or(|adopted| drivers == Some(adopted));
    unchanged.then_some(()).ok_or(Unknown::Unproven)
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
