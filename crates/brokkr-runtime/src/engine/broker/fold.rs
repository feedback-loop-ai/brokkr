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
//! admits no result, and leaves the lease standing. An engine restarted
//! with the attempt in flight holds no lease of its own (D6): it takes the
//! attempt's again, and folds each of its drivers from the journal head
//! that driver's settlement record keeps, only where every driver's record
//! was sealed Settled on #403's proof (U8e2; [`Engine::restarted`]);
//! otherwise it settles the attempt indeterminate without reading. Either
//! way it replays nothing.
//!
//! The responses the attempt's ledgers cite are published first, through
//! publication's own reader (`artifacts`), so no checkpoint cites an
//! artifact that is not durable (CR2). Then each broker the site was
//! sealed with is folded in turn: its plan read as its inventory pins it,
//! its ledger read whole under CR3's bound and verified only by the
//! protocol's one reader (`recover`), each accepted call of the verified
//! prefix made exactly one settled checkpoint, its Terminal's outcome or
//! interrupted where it has none, and those appended through the confirmed
//! append ([`confirmed`]) before the next broker's ledger is
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

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_protocol::broker::ledger::{recover, Disposition, Failure, Invalid, Outcome};
use brokkr_protocol::broker::{Inventory, Pin, Plan};
use brokkr_protocol::native_controls::{SealedBroker, SealedServers};
use brokkr_protocol::{AttemptReport, Cleanup};
use brokkr_store::{SeatRecordError, Store, StoreError};
use rustix::fs::Dir;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

use super::admission::Held;
use super::settlement::{ended, evidenced, Unreleased};
use crate::engine::artifacts::{self, Evidence, Unpublished, LEDGER_BYTES_MAX};
use crate::engine::capability_calls::{self, BrokerCheckpoint, SettledEvent};
use crate::engine::checkpoints::{self, Attempt, Confirmed, SETTLING_PATIENCES};
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
pub(in crate::engine) struct Begun(pub(super) (u64, String));

/// Which brokers a driver's calls are folded from: those its spawn's site
/// was sealed with, or, for an engine restarted with its attempt in flight,
/// every plan its attempt's inventory pins for the site its settlement
/// record names, each as it was sealed (U8e2).
#[derive(Clone, Copy)]
enum Brokers<'a> {
    Sealed(&'a [SealedBroker]),
    Pinned(&'a str),
}

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

/// What folding an attempt's brokers came to: the first refusal met, the
/// cause the first verified session that failed failed with, met first or
/// after another refusal, and how many settled checkpoints a lock that
/// outlasted the settlement left unjournaled.
#[derive(Debug, Default)]
struct Tally {
    refusal: Option<Unfolded>,
    failed: Option<Failure>,
    stranded: usize,
}

impl Tally {
    /// Nothing journaled, and `refusal`.
    fn refused(refusal: Unfolded) -> Tally {
        let mut tally = Tally::default();
        tally.refuse(refusal);
        tally
    }

    /// `refusal`, where none came first; and its failed session's cause,
    /// where it is one and none came first, whichever refusal did.
    fn refuse(&mut self, refusal: Unfolded) {
        self.failed = self.failed.take().or(refusal.failed().cloned());
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
    /// attempt `ids` names at the single site, labelled as `ids` last
    /// names it, that `spawn` was sealed for, which began where `begun`
    /// says: its cleanup witnessed on the lease first, and its settlement
    /// record sealed where it settled (#403, U8e2; [`ended`]), then the
    /// report settled by that site's broker calls ([`Engine::settled_by`]),
    /// before its caller admits anything of it.
    pub(in crate::engine) fn folded(
        &mut self,
        [effect, attempt, site]: [&str; 3],
        spawn: &SiteSpawn,
        begun: Option<Begun>,
        report: AttemptReport,
    ) -> Result<AttemptReport, EngineError> {
        ended(&mut self.held, site, &report.cleanup);
        self.settled_by([effect, attempt], spawn, begun, report)
    }

    /// `report`, of a driver whose end is already witnessed, settled by the
    /// broker calls of the site `spawn` was sealed for ([`Engine::fold_with`])
    /// where it began at `begun`; as it stands where its site was sealed no
    /// broker.
    pub(in crate::engine) fn settled_by(
        &mut self,
        ids: [&str; 2],
        spawn: &SiteSpawn,
        begun: Option<Begun>,
        report: AttemptReport,
    ) -> Result<AttemptReport, EngineError> {
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
        ids: [&str; 2],
        spawn: &SiteSpawn,
        Begun(begun): &Begun,
        mut report: AttemptReport,
        between: B,
    ) -> Result<AttemptReport, EngineError> {
        let unproven = self.held.as_ref().is_some_and(|held| held.unproven);
        if !matches!(report.cleanup, Cleanup::Settled) | unproven {
            return Ok(report);
        }
        let sealed = Brokers::Sealed(brokers(spawn));
        let tally = self.tally(ids, sealed, begun, between)?;
        let refusal = tally.refusal.as_ref();
        let refusal = refusal.map(|refusal| refusal as &dyn std::fmt::Display);
        checkpoints::carry_settled(&mut report, tally.stranded, refusal);
        Ok(report)
    }

    /// The calls of the driver of the attempt `ids` names that `which`
    /// names the brokers of, the driver having begun at the journal head
    /// `begun`, each broker's handed to the confirmed append, which lands
    /// each only on the head it was judged on ([`confirmed`]), with
    /// `between` run before each try, each read through [`settled_calls`];
    /// none, and unbound, where the engine holds no lease.
    fn tally<B: FnMut(&mut Store)>(
        &mut self,
        [effect_id, attempt_id]: [&str; 2],
        which: Brokers<'_>,
        begun: &(u64, String),
        mut between: B,
    ) -> Result<Tally, EngineError> {
        let attempt = Attempt {
            run_id: &self.run_id,
            effect_id,
            attempt_id,
        };
        let (store, cause, fence) = (&mut self.store, &mut self.current_cause, &mut self.fence);
        let mut append = |rows| {
            let journal = (&mut *store, &mut *cause, &mut *fence);
            let rows = attempt.settled(rows);
            confirmed(journal, (&attempt, begun), &rows, &mut between)
        };
        match &self.held {
            Some(held) => settled_calls(held, which, &mut append),
            None => Ok(Tally::refused(Unfolded::Unpublished(Unpublished::Unbound))),
        }
    }

    /// U8e2 (operator ruling 2026-10-21): the attempt `attempt` of the
    /// effect `effect` that this restarted engine finds in flight, ended.
    /// Where its every driver's settlement record is sealed Settled, its
    /// lease is taken again ([`Engine::readopted`]), each driver's calls
    /// folded from the head its record keeps, never one taken now, through
    /// the brokers its inventory pins for the driver's site, each accepted
    /// call appended once: the confirmed append finds what a commit whose
    /// acknowledgement was lost already wrote. A verified Closed that
    /// failed fails the attempt with its exact cause; otherwise it is
    /// indeterminate as before, its calls journaled, and its settlement then
    /// releases it ([`Engine::settle`]). Anything else, a driver still
    /// Settling or no record at all, is indeterminate with nothing read and
    /// the reservation standing. An adoption that fails is handed up with
    /// its refusal, the reservation standing and nothing journaled, so the attempt
    /// stays in flight for a later restart (L1 of U8e2's second run).
    /// Nothing is spawned, provisioned or replayed.
    /// The end is appended on the fence this engine's fold armed, moved
    /// only over the calls it appended itself ([`confirmed`]): a peer's
    /// append before them refuses the calls, and one after them the end (M1
    /// of U8e2's first run, J1 of its second; decision 0029).
    pub(in crate::engine) fn restarted(
        &mut self,
        effect: &str,
        attempt: &str,
    ) -> Result<(), EngineError> {
        let adoption = self.readopted(attempt).map_err(Unreleased::standing)?;
        let Some((held, drivers)) = adoption else {
            return self.ended_restarted([effect, attempt], &[]);
        };
        self.held = Some(held);
        let mut tallies = Vec::with_capacity(drivers.len());
        for driver in &drivers {
            let Some(begun) = &driver.begun else {
                continue;
            };
            let pinned = Brokers::Pinned(&driver.site);
            tallies.push(self.tally([effect, attempt], pinned, begun, |_| {})?);
        }
        let ran = self.ended_restarted([effect, attempt], &tallies);
        self.settle(attempt, ran)
    }

    /// The attempt `ids` names, which a restarted engine found in flight,
    /// journaled as its drivers' `tallies` leave it ([`restarted`]):
    /// indeterminate, saying why its completion cannot be established, or
    /// failed by the cause its calls fail it with.
    fn ended_restarted(
        &mut self,
        [effect, attempt]: [&str; 2],
        tallies: &[Tally],
    ) -> Result<(), EngineError> {
        let (event, key, said) = restarted(tallies);
        let ended = json!({"effect_id": effect, "attempt_id": attempt, key: said});
        self.append(event, ended, Some(attempt.to_string()))
            .map(drop)
    }
}

/// Why a restarted engine cannot establish the completion of an attempt it
/// found in flight: its driver's own report died with the engine.
const RESTARTED: &str =
    "engine restarted while the attempt was in flight; completion cannot be established";

/// What the calls of a restarted attempt's drivers, folded into `tallies`,
/// leave of it (CR4): the event that ends it, the key its payload says it
/// under, and what it says. Its calls are judged apart from its harness,
/// whose own result died with the engine that ran it and is never
/// admitted. Failed by the first verified Closed that failed, with exactly
/// its cause, whatever refusal any broker met before it (M2 of U8e2's first
/// run); otherwise indeterminate, naming the first refusal the fold met
/// beside why completion cannot be established (L1); and where a lock left
/// calls unjournaled, naming them first, as a running engine does, with
/// that cause beside them.
fn restarted(tallies: &[Tally]) -> (EventType, &'static str, String) {
    let stranded = tallies.iter().map(|tally| tally.stranded).sum();
    let failed = tallies.iter().find_map(|tally| tally.failed.as_ref());
    let refused = tallies.iter().find_map(|tally| tally.refusal.as_ref());
    let cause = failed.map(ToString::to_string);
    let cause = cause.or(refused.map(ToString::to_string));
    let indeterminate = |reason| (EventType::EffectIndeterminate, "reason", reason);
    match (stranded, failed, cause) {
        (0, Some(failure), _) => (EventType::EffectFailed, "error", failure.to_string()),
        (0, None, Some(cause)) => indeterminate(format!("{RESTARTED}: {cause}")),
        (0, None, None) => indeterminate(RESTARTED.to_string()),
        (stranded, _, cause) => {
            let reasons = std::iter::once(checkpoints::outlasted(stranded)).chain(cause);
            indeterminate(reasons.collect::<Vec<_>>().join("; "))
        }
    }
}

impl Unfolded {
    /// The cause a verified session failed with, where that is this
    /// refusal; none for every other.
    fn failed(&self) -> Option<&Failure> {
        match self {
            Unfolded::Failed(failure) => Some(failure),
            Unfolded::Missing
            | Unfolded::Unexpected
            | Unfolded::Ungranted
            | Unfolded::Invalid(_)
            | Unfolded::Unpublished(_)
            | Unfolded::Fence(_) => None,
        }
    }
}

/// What an engine's read of the journal, or its appends, came to
/// ([`confirmed`]), short of rows still to append.
enum Landing {
    /// Every row is committed, as derived.
    Journaled,
    /// CR3's conflicting duplicate.
    Conflict,
    /// The seat-record fence refused one; those before it are committed.
    Refused(SeatRecordError),
    /// The journal holds, past the head at the first of these seqs, an
    /// event the fold may not append beside, up to its head at the second.
    Moved(u64, u64),
}

/// The rows a read left to append, and the head the read found, onto
/// which alone the first may land.
type Unjournaled<'r> = (Vec<&'r SettledEvent>, (u64, String));

/// The journal an engine appends an attempt's calls to: its store, the
/// cause its next event names, and its fence (decision 0029).
type Journal<'a> = (
    &'a mut Store,
    &'a mut Option<String>,
    &'a mut Option<(u64, String)>,
);

/// Append `rows`, settled checkpoints the engine derived for `attempt`,
/// which began at the journal's head `begun`, once its driver has stopped
/// (U8d, U8e2; CR4; decision 0029): never through the hold, within
/// [`SETTLING_PATIENCES`], and each row alone, compare-and-append, onto
/// the head the read that judged it unjournaled found
/// ([`Store::append_next_if_head`]). One seam for a running engine, whose
/// fence the attempt's opening disarmed, and for a restarted one, whose
/// fold armed it. Before every try what the journal committed since
/// `begun` is read again, verified onto it ([`Store::load_after`]), never
/// the run whole, so a row a try committed whose acknowledgement was lost
/// is found, not appended twice. A peer that writes between that read and
/// the write, a restarted engine appending the same call or `conclude`
/// ending the run, moves the head: nothing is written, and the journal is
/// read and judged again, never appended blind, so a call a peer appended
/// is found, not appended twice (H1 of U8e2's third run, J1 of its
/// second). Past an armed fence an event that is none of the rows whole,
/// and past `begun` under a disarmed one an event the attempt's own record
/// never holds ([`unread`]), refuses before anything more is appended, for
/// the fold this engine decided on no longer stands
/// ([`EngineError::JournalMoved`]). An armed fence moves onto each row
/// appended, a disarmed one stays so ([`each`]). `between` runs before
/// every read, as a sink's does, and a lock that outlasts the settlement
/// strands what is left. No rows ask nothing.
fn confirmed<B: FnMut(&mut Store)>(
    journal: Journal<'_>,
    (attempt, begun): (&Attempt<'_>, &(u64, String)),
    rows: &[SettledEvent],
    mut between: B,
) -> Result<Confirmed, EngineError> {
    let Some(mut left) = (!rows.is_empty()).then_some(rows.len()) else {
        return Ok(Confirmed::Journaled);
    };
    let (store, cause, fence) = journal;
    let landed = checkpoints::within_patiences(SETTLING_PATIENCES, || loop {
        between(&mut *store);
        let (pending, head) = match unread(store, attempt, (begun, fence), rows)? {
            Ok(unjournaled) => unjournaled,
            Err(judged) => return Ok(judged),
        };
        left = pending.len();
        #[cfg(test)]
        tests::peer_after_the_read();
        let journal = (&mut *store, &mut *cause, &mut *fence);
        match each(journal, attempt, head, &pending, &mut left) {
            Err(StoreError::HeadMoved { .. }) => continue,
            appended => return appended,
        }
    });
    match landed {
        Ok(Landing::Journaled) => Ok(Confirmed::Journaled),
        Ok(Landing::Conflict) => Ok(Confirmed::Conflict),
        Ok(Landing::Refused(refusal)) => Ok(Confirmed::Refused(refusal)),
        Ok(Landing::Moved(expected_seq, found_seq)) => Err(EngineError::JournalMoved {
            run_id: attempt.run_id.to_string(),
            expected_seq,
            found_seq,
        }),
        Err(contended) if contended.is_contention() => Ok(Confirmed::Stranded(left)),
        Err(failure) => Err(failure.into()),
    }
}

/// What the journal past `begun`, read again and verified onto it, leaves
/// of `rows` to append, and the head it was read at ([`confirmed`]):
/// nothing where it holds them all; refused where it holds one with another
/// payload; and moved where, past the head an armed `fence` holds, it holds
/// an event that is none of them, or, past `begun` where the fence is
/// disarmed, one the attempt's own record never holds ([`foreign`]).
fn unread<'r>(
    store: &Store,
    attempt: &Attempt<'_>,
    ((seq, hash), fence): (&(u64, String), &Option<(u64, String)>),
    rows: &'r [SettledEvent],
) -> Result<Result<Unjournaled<'r>, Landing>, StoreError> {
    let journal = store.load_after(attempt.run_id, *seq, hash)?;
    let Ok(pending) = capability_calls::unjournaled(&journal, attempt.attempt_id, rows) else {
        return Ok(Err(Landing::Conflict));
    };
    let head = journal
        .last()
        .map(|last| (last.seq, last.event_hash.clone()));
    let head = head.unwrap_or_else(|| (*seq, hash.clone()));
    let fenced = fence.as_ref().map_or(*seq, |(fenced, _)| *fenced);
    let row = |event: &EventEnvelope| {
        let ours = event.event_type == EventType::EffectCheckpointed;
        let ours = ours & (event.attempt_id.as_deref() == Some(attempt.attempt_id));
        let settled = SettledEvent::deserialize(&event.payload).ok();
        ours & settled.is_some_and(|settled| rows.contains(&settled))
    };
    let moved = |event: &EventEnvelope| match fence {
        Some(_) => !row(event),
        None => foreign(event.event_type),
    };
    let mut past = journal.iter().filter(|event| event.seq > fenced);
    if past.any(moved) {
        return Ok(Err(Landing::Moved(fenced, head.0)));
    }
    Ok(match pending.is_empty() {
        true => Err(Landing::Journaled),
        false => Ok((pending, head)),
    })
}

/// Whether an event of `kind` is one the open attempt's own record never
/// holds while its fence is disarmed (decision 0029): its checkpoints and
/// its members' and steps' markers are, as is an operator's command riding
/// it to its boundary; an end of the attempt or of the run, and anything
/// only a fold decides, is a peer's.
fn foreign(kind: EventType) -> bool {
    match kind {
        EventType::EffectCheckpointed
        | EventType::OperatorCommanded
        | EventType::OperatorAccepted
        | EventType::OperatorRejected => false,
        EventType::RunStarted
        | EventType::PhaseEntered
        | EventType::EffectRequested
        | EventType::EffectStarted
        | EventType::EffectSucceeded
        | EventType::EffectFailed
        | EventType::EffectIndeterminate
        | EventType::TransitionDecided
        | EventType::RunParked
        | EventType::RunCompleted
        | EventType::RunStopped => true,
    }
}

/// `pending` appended in order, each alone onto the head the one before it
/// left, the first onto `head` ([`Store::append_next_if_head`]), behind
/// the journal's cause, `left` counting down, and an armed fence moved
/// onto each: the read found nothing past the fence but the attempt's own
/// calls, and the head check let nothing land since, so the engine's next
/// fenced append still lands only where no peer wrote. A disarmed fence
/// stays disarmed. A head a peer moved writes nothing more and is handed
/// up, as is a storage failure; a row the seat-record fence refuses ends
/// it there.
fn each(
    (store, cause, fence): Journal<'_>,
    attempt: &Attempt<'_>,
    mut head: (u64, String),
    pending: &[&SettledEvent],
    left: &mut usize,
) -> Result<Landing, StoreError> {
    let (run, event) = (attempt.run_id, EventType::EffectCheckpointed);
    for row in pending {
        let (id, payload) = (Some(attempt.attempt_id.to_string()), row.payload());
        let appended =
            store.append_next_if_head(run, head.0, &head.1, event, payload, cause.clone(), id);
        let appended = match appended {
            Ok(appended) => appended,
            Err(StoreError::SeatRecord(refusal)) => return Ok(Landing::Refused(refusal)),
            Err(failure) => return Err(failure),
        };
        *left -= 1;
        *cause = Some(appended.event_id.clone());
        head = (appended.seq, appended.event_hash);
        *fence = fence.take().map(|_| head.clone());
    }
    Ok(Landing::Journaled)
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

/// The settled calls of the brokers `which` names of the attempt in flight
/// under the lease `held`, read from its evidence once the responses its
/// ledgers cite are published (CR2), each broker's handed to `append`
/// ([`ledgers`]); none, and the refusal, where they are not published or
/// the evidence is not the attempt's.
fn settled_calls(
    held: &Held,
    which: Brokers<'_>,
    append: Append<'_>,
) -> Result<Tally, EngineError> {
    let unbound = |unread| Ok(Tally::refused(Unfolded::Unpublished(unread)));
    let read = evidenced(held, |evidence| {
        match artifacts::published(evidence, held.root.as_ref()) {
            Ok(()) => ledgers(evidence, which, append),
            Err(unpublished) => unbound(unpublished),
        }
    });
    read.unwrap_or_else(unbound)
}

/// The settled calls of the brokers `which` names in the attempt
/// `evidence` locates: its inventory read, refused where the directory
/// holds a ledger it pins no plan for ([`expected`]), and each broker's
/// plan, pinned as it was sealed ([`plans`]), folded from its ledger
/// ([`ledger`]) and handed to `append` before the next broker's ledger is
/// read, in the brokers' order. Evidence that is not the attempt's, a
/// conflict and the fence's refusal stop the fold there.
fn ledgers(
    evidence: &Evidence<'_>,
    which: Brokers<'_>,
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
    for found in plans(&dir, evidence, &inventory, which) {
        let (name, plan, pin) = match found {
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

/// One plan an attempt's inventory pins, read as it pins it: its name, the
/// plan and its pin; or why it cannot be read so.
type Found<'i> = Result<(&'i OsStr, Plan, &'i Pin), Unpublished>;

/// The plans of the brokers `which` names in the attempt directory `dir`
/// of the attempt `evidence` locates, as its `inventory` pins them, in the
/// brokers' order: each sealed broker's ([`sealed`]); or, for a restarted
/// engine, every plan the inventory pins whose owner is the site the
/// driver's record names, in the inventory's order, which is the order
/// provisioning sealed them in (U8e2). A plan that cannot be read is
/// refused where it is met, whichever site it was for.
fn plans<'i>(
    dir: &OwnedFd,
    evidence: &Evidence<'_>,
    inventory: &'i Inventory,
    which: Brokers<'_>,
) -> Vec<Found<'i>> {
    match which {
        Brokers::Sealed(brokers) => (brokers.iter())
            .map(|broker| sealed(dir, evidence, inventory, broker))
            .collect(),
        Brokers::Pinned(site) => (inventory.plans.iter())
            .map(|pin| read(dir, evidence, pin))
            .filter(|found| {
                found
                    .as_ref()
                    .map_or(true, |(_, plan, _)| plan.owner.site == site)
            })
            .collect(),
    }
}

/// The plan `broker` was sealed with, in the attempt directory `dir` of
/// the attempt `evidence` locates, whose `inventory` must pin it by the
/// very locator and digest the broker names, and which must serve the
/// broker's capability.
fn sealed<'i>(
    dir: &OwnedFd,
    evidence: &Evidence<'_>,
    inventory: &'i Inventory,
    broker: &SealedBroker,
) -> Found<'i> {
    let pinned = |pin: &&Pin| {
        (pin.locator.as_os_str() == OsStr::new(&broker.plan)) & (pin.digest == broker.plan_digest)
    };
    let pin = inventory.plans.iter().find(pinned);
    let (name, plan, pin) = read(dir, evidence, pin.ok_or(Unpublished::Unbound)?)?;
    let serves = plan.capability == broker.capability;
    serves
        .then_some((name, plan, pin))
        .ok_or(Unpublished::Unbound)
}

/// The plan `pin` pins in the attempt directory `dir` of the attempt
/// `evidence` locates, read as it pins it ([`artifacts::pinned`]).
fn read<'i>(dir: &OwnedFd, evidence: &Evidence<'_>, pin: &'i Pin) -> Found<'i> {
    let [_, _, attempt] = evidence.owner;
    let at = evidence.at.join(attempt);
    let (name, plan) = artifacts::pinned(dir, &at, pin, evidence.owner)?;
    Ok((name, plan, pin))
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
