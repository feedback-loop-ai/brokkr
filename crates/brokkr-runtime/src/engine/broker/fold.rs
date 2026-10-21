//! An attempt's settled broker calls folded into its journal (decision 0065
//! slice two, U8d; D7, CC2, CR3, CR4).
//!
//! Every driver's report reaches this barrier before the engine admits its
//! result (U8e; D7), whatever its outcome, a failed, cancelled or timed-out
//! one included: a single seat's and each sequence step's through
//! `run_driver`, and each panel member's, a panel step's among them,
//! through `run_panel` once every member has ended. Evidence is read only
//! where the engine holds the attempt's lease and every process the attempt
//! owned is proven settled: this driver's cleanup `Cleanup::Settled`, and
//! no driver of the attempt witnessed unresolved on its lease
//! (`Held::unproven`), so a live broker's half-written record is never
//! read as corruption. An attempt not proven settled stands as its report
//! says, already indeterminate, its ledgers kept on disk and its lease
//! never released. The routes that do not read are fixed: a driver that
//! did not spawn started no broker, so its failed start stands and no
//! ledger is missing for it (CR3); an error returned before the report
//! admits no result, and leaves the lease standing; and an engine
//! restarted with the attempt in flight holds no lease and no proof that
//! its processes settled (D6), so it settles the attempt indeterminate
//! without reading, and replays nothing.
//!
//! The responses the attempt's ledgers cite are published first, through
//! publication's own reader (`artifacts`), so no checkpoint cites an
//! artifact that is not durable (CR2). Then each broker the site was
//! sealed with is folded in turn: its plan read as its inventory pins it,
//! its ledger read whole under CR3's bound and verified only by the
//! protocol's one reader (`recover`), each accepted call of the verified
//! prefix made exactly one settled checkpoint, its Terminal's outcome or
//! interrupted where it has none, and those appended through the confirmed
//! append ([`checkpoints::confirmed`]) before the next broker's ledger is
//! read. Nothing rides the seat's hold, and the harness's own MCP telemetry
//! is never a call here (`Calls::consume`).
//!
//! The session is judged apart from its calls: a verified Closed that
//! failed fails the attempt with its exact cause, whatever its calls, none
//! included, and no checkpoint stands for it. A ledger that does not verify
//! keeps its verified calls and fails the attempt with the protocol's cause
//! (`Invalid`); a plan whose ledger is missing, a ledger no plan pins, and
//! a forwarded call to a tool its plan does not grant each fail it in their
//! own words ([`Unfolded`]). An attempt not proven settled is not read:
//! what a live broker may still write is no evidence yet, and that end is
//! already indeterminate.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;

use brokkr_protocol::broker::ledger::{recover, Disposition, Failure, Invalid, Outcome};
use brokkr_protocol::broker::{Inventory, Pin, Plan};
use brokkr_protocol::native_controls::{SealedBroker, SealedServers};
use brokkr_protocol::{AttemptReport, Cleanup};
use brokkr_store::{SeatRecordError, Store};
use rustix::fs::Dir;
use thiserror::Error;

use super::admission::Held;
use super::settlement::{evidenced, witnessed};
use crate::engine::artifacts::{self, Evidence, Unpublished, LEDGER_BYTES_MAX};
use crate::engine::capability_calls::{self, BrokerCheckpoint};
use crate::engine::checkpoints::{self, Attempt, Confirmed};
use crate::engine::{Engine, EngineError, SiteSpawn};

/// What a plan's ledger is named with, beside the plan. `artifacts`
/// spells it again inline where it reads the citations, and may only
/// change its visibility in this unit (operator ruling of 2026-10-20).
const LEDGER: &str = ".ledger";

/// Why an attempt's broker calls do not let it settle as its driver said
/// (CR3, CR4): a plan's ledger missing, a ledger no plan pins, a forwarded
/// call to a tool its plan does not grant, the protocol's own cause for a
/// ledger that does not verify, the cause a verified session failed with,
/// evidence not bound to the attempt or not published, or a checkpoint the
/// seat-record fence refused.
#[derive(Debug, PartialEq, Eq, Error)]
enum Unfolded {
    #[error("broker ledger is missing for this attempt")]
    Missing,
    #[error("broker ledger is not expected for this attempt")]
    Unexpected,
    #[error("broker ledger claims an ungranted tool")]
    Ungranted,
    #[error(transparent)]
    Invalid(Invalid),
    #[error(transparent)]
    Failed(Failure),
    #[error(transparent)]
    Unpublished(Unpublished),
    #[error(transparent)]
    Fence(SeatRecordError),
}

/// Where an attempt at a brokered site began in its journal: the head
/// before its driver spawned, below which none of its rows can lie.
pub(in crate::engine) struct Begun((u64, String));

/// The settled checkpoints one ledger proves, in call order, and the first
/// refusal met reading it.
#[derive(Debug)]
struct Folded {
    rows: Vec<BrokerCheckpoint>,
    refusal: Option<Unfolded>,
}

impl Folded {
    /// No checkpoint, and `refusal`.
    fn refused(refusal: Unfolded) -> Folded {
        let rows = Vec::new();
        let refusal = Some(refusal);
        Folded { rows, refusal }
    }
}

/// One broker's settled checkpoints appended, and what the store made of
/// them.
type Append<'a> = &'a mut dyn FnMut(Vec<BrokerCheckpoint>) -> Result<Confirmed, EngineError>;

/// What folding an attempt's brokers came to: the first refusal met, and
/// how many settled checkpoints a lock that outlasted the settlement left
/// unjournaled.
#[derive(Debug, Default)]
struct Tally {
    refusal: Option<Unfolded>,
    stranded: usize,
}

impl Tally {
    /// Nothing journaled, and `refusal`.
    fn refused(refusal: Unfolded) -> Tally {
        let refusal = Some(refusal);
        Tally {
            refusal,
            stranded: 0,
        }
    }

    /// `refusal`, where none came first.
    fn refuse(&mut self, refusal: Unfolded) {
        self.refusal = self.refusal.take().or(Some(refusal));
    }

    /// One broker's `rows` handed to `append`, or only counted where an
    /// earlier broker's were stranded, since the lock that outlasted them
    /// outlasts these; whether nothing more is to be appended, as after a
    /// conflict or the fence's refusal.
    fn settle(
        &mut self,
        rows: Vec<BrokerCheckpoint>,
        append: Append<'_>,
    ) -> Result<bool, EngineError> {
        if self.stranded > 0 {
            self.stranded += rows.len();
            return Ok(false);
        }
        let halted = match append(rows)? {
            Confirmed::Journaled => None,
            Confirmed::Stranded(rows) => {
                self.stranded = rows;
                None
            }
            Confirmed::Conflict => Some(Unfolded::Invalid(Invalid::Conflict)),
            Confirmed::Refused(fence) => Some(Unfolded::Fence(fence)),
        };
        let halts = halted.is_some();
        halted.into_iter().for_each(|refusal| self.refuse(refusal));
        Ok(halts)
    }
}

impl Engine {
    /// Where the attempt about to run at the site `spawn` was sealed for
    /// begins in the journal: its head now, before its driver spawns; none
    /// where the site was sealed no broker, which leaves nothing to fold.
    pub(in crate::engine) fn begun(&self, spawn: &SiteSpawn) -> Result<Option<Begun>, EngineError> {
        if brokers(spawn).is_empty() {
            return Ok(None);
        }
        Ok(Some(Begun(self.store.head_hash(&self.run_id)?)))
    }

    /// `report`, the report of the driver that has just ended of the
    /// attempt `ids` names at the single site `spawn` was sealed for, which
    /// began where `begun` says: its cleanup witnessed on the lease first
    /// (#403, [`witnessed`]), then the report settled by that site's broker
    /// calls ([`Engine::fold_with`]), before its caller admits anything of
    /// it.
    pub(in crate::engine) fn folded(
        &mut self,
        ids: [&str; 2],
        spawn: &SiteSpawn,
        begun: Option<Begun>,
        report: AttemptReport,
    ) -> Result<AttemptReport, EngineError> {
        witnessed(&mut self.held, &report.cleanup);
        match begun {
            Some(begun) => self.fold_with(ids, spawn, &begun, report, |_| {}),
            None => Ok(report),
        }
    }

    /// `report` settled by the broker calls of the site `spawn` was sealed
    /// for, the attempt having begun at `begun`, `between` run before each
    /// append is tried, as a sink runs its own: where the attempt is not
    /// proven settled, as it stands; otherwise with each accepted call's
    /// checkpoint appended, and carrying what the calls and the sessions
    /// leave of its outcome ([`checkpoints::carry_settled`]). A lease the
    /// engine does not hold reads no evidence, which is then unbound; one
    /// that witnessed any driver of the attempt unresolved reads none
    /// either, since that driver's broker may still be writing.
    pub(super) fn fold_with<B: FnMut(&mut Store)>(
        &mut self,
        [effect_id, attempt_id]: [&str; 2],
        spawn: &SiteSpawn,
        Begun(begun): &Begun,
        mut report: AttemptReport,
        mut between: B,
    ) -> Result<AttemptReport, EngineError> {
        let unproven = self.held.as_ref().is_some_and(|held| held.unproven);
        if !matches!(report.cleanup, Cleanup::Settled) | unproven {
            return Ok(report);
        }
        let attempt = Attempt {
            run_id: &self.run_id,
            effect_id,
            attempt_id,
        };
        let (store, cause) = (&mut self.store, &mut self.current_cause);
        let mut append = |rows| {
            let journal = (&mut *store, &mut *cause);
            checkpoints::confirmed(journal, (&attempt, begun), rows, &mut between)
        };
        let tally = match &self.held {
            Some(held) => settled_calls(held, brokers(spawn), &mut append)?,
            None => Tally::refused(Unfolded::Unpublished(Unpublished::Unbound)),
        };
        let refusal = tally.refusal.as_ref();
        let refusal = refusal.map(|refusal| refusal as &dyn std::fmt::Display);
        checkpoints::carry_settled(&mut report, tally.stranded, refusal);
        Ok(report)
    }
}

/// The brokers `spawn`'s site was sealed with, none where it serves none.
fn brokers(spawn: &SiteSpawn) -> &[SealedBroker] {
    match spawn
        .serving
        .as_ref()
        .map(|serving| &serving.isolation.servers)
    {
        Some(SealedServers::Brokered(brokers)) => brokers,
        Some(SealedServers::Empty | SealedServers::Hands | SealedServers::NoModelSurface)
        | None => &[],
    }
}

/// The settled calls of the `brokers` of the attempt in flight under the
/// lease `held`, read from its evidence once the responses its ledgers
/// cite are published (CR2), each broker's handed to `append`
/// ([`ledgers`]); none, and the refusal, where they are not published or
/// the evidence is not the attempt's.
fn settled_calls(
    held: &Held,
    brokers: &[SealedBroker],
    append: Append<'_>,
) -> Result<Tally, EngineError> {
    let unbound = |unread| Ok(Tally::refused(Unfolded::Unpublished(unread)));
    let read = evidenced(held, |evidence| {
        match artifacts::published(evidence, held.root.as_ref()) {
            Ok(()) => ledgers(evidence, brokers, append),
            Err(unpublished) => unbound(unpublished),
        }
    });
    read.unwrap_or_else(unbound)
}

/// The settled calls of `brokers` in the attempt `evidence` locates: its
/// inventory read, refused where the directory holds a ledger it pins no
/// plan for ([`expected`]), and each broker's plan, pinned as it was
/// sealed, folded from its ledger ([`ledger`]) and handed to `append`
/// before the next broker's ledger is read, in the brokers' order.
/// Evidence that is not the attempt's, a conflict and the fence's refusal
/// stop the fold there.
fn ledgers(
    evidence: &Evidence<'_>,
    brokers: &[SealedBroker],
    append: Append<'_>,
) -> Result<Tally, EngineError> {
    let provisioned = artifacts::provisioned(evidence);
    let provisioned = provisioned.and_then(|provisioned| provisioned.ok_or(Unpublished::Unbound));
    let (dir, inventory) = match provisioned {
        Ok(provisioned) => provisioned,
        Err(unread) => return Ok(Tally::refused(Unfolded::Unpublished(unread))),
    };
    let mut tally = Tally::default();
    if let Err(unexpected) = expected(&dir, &inventory) {
        tally.refuse(unexpected);
    }
    for broker in brokers {
        let (name, plan, pin) = match sealed(&dir, evidence, &inventory, broker) {
            Ok(sealed) => sealed,
            Err(unread) => {
                tally.refuse(Unfolded::Unpublished(unread));
                return Ok(tally);
            }
        };
        let Folded { rows, refusal } = ledger(&dir, name, &plan, &pin.digest);
        refusal
            .into_iter()
            .for_each(|refusal| tally.refuse(refusal));
        if tally.settle(rows, append)? {
            return Ok(tally);
        }
    }
    Ok(tally)
}

/// The plan `broker` was sealed with, in the attempt directory `dir` of
/// the attempt `evidence` locates, whose `inventory` must pin it by the
/// very locator and digest the broker names, and which must serve the
/// broker's capability: its name, the plan and its pin.
fn sealed<'i>(
    dir: &OwnedFd,
    evidence: &Evidence<'_>,
    inventory: &'i Inventory,
    broker: &SealedBroker,
) -> Result<(&'i OsStr, Plan, &'i Pin), Unpublished> {
    let pinned = |pin: &&Pin| {
        (pin.locator.as_os_str() == OsStr::new(&broker.plan)) & (pin.digest == broker.plan_digest)
    };
    let pin = inventory.plans.iter().find(pinned);
    let pin = pin.ok_or(Unpublished::Unbound)?;
    let [_, _, attempt] = evidence.owner;
    let at = evidence.at.join(attempt);
    let (name, plan) = artifacts::pinned(dir, &at, pin, evidence.owner)?;
    let serves = plan.capability == broker.capability;
    serves
        .then_some((name, plan, pin))
        .ok_or(Unpublished::Unbound)
}

/// Refused where the attempt directory `dir` holds a ledger its
/// `inventory` pins no plan for: the inventory is the one source of the
/// ledgers expected (CR3).
fn expected(dir: &OwnedFd, inventory: &Inventory) -> Result<(), Unfolded> {
    let unread = || Unfolded::Unpublished(Unpublished::Unbound);
    let pinned = (inventory.plans.iter()).filter_map(|pin| pin.locator.file_name());
    let pinned: BTreeSet<OsString> = pinned.map(|name| artifacts::beside(name, LEDGER)).collect();
    for entry in Dir::read_from(dir).or(Err(unread()))? {
        let entry = entry.or(Err(unread()))?;
        let name = OsStr::from_bytes(entry.file_name().to_bytes());
        let ledger = name.as_bytes().ends_with(LEDGER.as_bytes());
        if ledger & !pinned.contains(name) {
            return Err(Unfolded::Unexpected);
        }
    }
    Ok(())
}

/// The settled calls the ledger of the plan named `name` in `dir`, sealed
/// at `digest`, proves (CC2, CR4): each accepted call of its verified
/// prefix, in call order, citing the response its Terminal cites, and the
/// first refusal. The ledger is read whole, within CR3's 64 MiB and 4,096
/// calls (operator ruling of 2026-10-20), and only this broker's. A
/// missing ledger proves none. A call forwarded to a tool the plan does
/// not grant stops the fold before it, since no broker forwards one; past
/// the prefix, the protocol's cause; and a verified closure that failed,
/// its cause, whatever the calls.
fn ledger(dir: &OwnedFd, name: &OsStr, plan: &Plan, digest: &str) -> Folded {
    let read = artifacts::record(dir, &artifacts::beside(name, LEDGER), LEDGER_BYTES_MAX);
    let read = read.map_err(Unfolded::Unpublished);
    let bytes = match read.and_then(|bytes| bytes.ok_or(Unfolded::Missing)) {
        Ok(bytes) => bytes,
        Err(unread) => return Folded::refused(unread),
    };
    let (lifecycle, invalid) = recover(&bytes, &plan.owner, digest);
    let cited: BTreeMap<u64, &str> = (lifecycle.cited())
        .map(|(call, sha256, _)| (call, sha256))
        .collect();
    let mut rows = Vec::with_capacity(lifecycle.calls().len());
    for (call, (tool, ended)) in (1..).zip(lifecycle.calls()) {
        let forwarded = matches!(ended, Some(Outcome::Succeeded | Outcome::Failed));
        if forwarded & !plan.tools.contains(tool) {
            let refusal = Some(Unfolded::Ungranted);
            return Folded { rows, refusal };
        }
        let cites = cited.get(&call).copied();
        rows.push(capability_calls::settled(plan, call, tool, *ended, cites));
    }
    let failed = match lifecycle.closed() {
        Some(Disposition::Failed(failure)) => Some(Unfolded::Failed(failure.clone())),
        Some(Disposition::Clean) | None => None,
    };
    let refusal = invalid.map(Unfolded::Invalid).or(failed);
    Folded { rows, refusal }
}

#[cfg(test)]
mod tests;
