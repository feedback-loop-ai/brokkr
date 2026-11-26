//! `settle`: the operator's record that a quarantined run's writers are
//! proven ended, for when the broker's own settlement proof cannot be read
//! (decision 0108 ruling 3, in ruling 4's erratum shape after decision
//! 0047).
//!
//! The verb re-checks the release's facts now: the run is acknowledged, its
//! run lock is free (and is held from the check through the write), and
//! every attempt its reservation names is proven ended by the broker's
//! records — or, where a record proves nothing, by the process group it
//! names being gone from the host. If they hold, one `operator/commanded`
//! with command `settle` names them; if not, it refuses and writes nothing.
//! Once it is written, every cool-down probe hold naming an attempt it
//! proves ends, for the record is the operator's settlement of that probe's
//! tree (decision 0106 slice 0106b1, fix round 0106b1r12, S2). Where a
//! settle's record stands at the run's head, its holds' end having failed
//! or been interrupted, the verb run again ends them from that record and
//! writes nothing more.
//!
//! Nothing here folds the run: a run this verb exists for does not fold at
//! all. The events are read by type (the chain is verified, the fold is
//! not), the acknowledgement is the table beside the chain, and the record
//! is read back by type once written. `fold` admits the annotation on a
//! terminal run (`ROWS`' `Arm::Annotates`), so no acceptance follows it.

use std::collections::BTreeMap;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{terminal_event, SETTLE};
use brokkr_core::EventEnvelope;
use brokkr_store::Store;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use super::{fenced_append, EngineError, Head};
use crate::engine::broker::settlement;
use crate::engine::broker::settlement::release::{provenances, Proven, SettleArgs};
use crate::run_lock;

#[cfg(test)]
mod tests;

/// Why `settle` refused, nothing written (decision 0108 ruling 3): the typed
/// fact the proof refused on, or the run's own word before a proof was
/// taken. Every arm is a type, never text: the host's answer (`group` still
/// alive, `NoGroup`, `Unresolved`), the unread reservation's typed
/// [`crate::engine::Unadmitted`], and the run's own pre-proof refusals are
/// variants the caller matches (decision 0071 rulings 6 and 8; S8, R4).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SettleRefusal {
    #[error(transparent)]
    Live(#[from] crate::engine::Live),
    /// The reservation could not be read whole, exactly as admission refuses
    /// it: the typed participant or root, never flattened to its text.
    #[error(transparent)]
    Unadmitted(#[from] crate::engine::Unadmitted),
    #[error(
        "run '{run}' has no terminal event; settle records that a run which has already ended \
         no longer holds its worktree"
    )]
    NoTerminal { run: String },
    #[error(
        "run '{run}' is not acknowledged, so it is not quarantined; answer it with \
         `brokkr runs operator acknowledge --run {run} --reason <why>` first"
    )]
    Unacknowledged { run: String },
    #[error("this host names no protected coordination root, so no reservation can be read")]
    NoRoot,
}

/// What `brokkr runs operator settle` says, as the operator typed it: why the
/// release is recorded, and who recorded it.
pub struct Settle<'a> {
    pub reason: &'a str,
    pub operator: &'a str,
}

/// Record that the run `run_id`'s writers are proven ended, so its worktree
/// is released (decision 0108 ruling 3). One `operator/commanded` with
/// `command: "settle"` and the facts proven, and nothing else: no
/// `operator/accepted` follows, because there is nothing to execute.
///
/// Every refusal comes before the first append, so a refused settle leaves
/// the journal byte for byte as it found it. The write is fenced on the head
/// the read found (decision 0029) while the run's lock is held, so no
/// driver of the run moves the journal beneath the record.
pub fn operator_settle(
    store: &mut Store,
    run_id: &str,
    ask: &Settle<'_>,
) -> Result<EventEnvelope, EngineError> {
    settle_with(store, run_id, ask, |_| {})
}

/// [`operator_settle`] with `before_write` run after the read and before the
/// fenced append: the instant a peer's append makes the head stale, which
/// the tests stage there. Production passes a no-op.
fn settle_with(
    store: &mut Store,
    run_id: &str,
    ask: &Settle<'_>,
    before_write: impl FnOnce(&mut Store),
) -> Result<EventEnvelope, EngineError> {
    let refused = |why: SettleRefusal| EngineError::SettleRefused(why);
    // The run's own record decides the cheap refusals first: a run that
    // has not ended, or that no acknowledgement stands on, is refused
    // before any coordination root is opened or any lock is taken.
    let events = store.load(run_id)?;
    let Some(_terminal) = events
        .iter()
        .find(|event| terminal_event(event.event_type).is_some())
    else {
        return Err(refused(SettleRefusal::NoTerminal {
            run: run_id.to_string(),
        }));
    };
    if let Some(head) = replayed(store, run_id, &events)? {
        return Ok(head);
    }
    let acknowledged = store.acknowledgement(run_id)?.is_some();
    // A repair moves the head past the acknowledgement and lapses it
    // (#511); the repair record is the operator's word that the run was
    // looked at, and stands in its place (decision 0108 ruling 4).
    let repaired = settlement::release::repaired(&events);
    if !(acknowledged | repaired) {
        return Err(refused(SettleRefusal::Unacknowledged {
            run: run_id.to_string(),
        }));
    }
    let observed = crate::engine::broker::observed();
    let root = observed
        .home
        .as_deref()
        .and_then(observed.open)
        .ok_or_else(|| refused(SettleRefusal::NoRoot))?;
    // The locks are taken in admission's own order, the admission lock and
    // then the run's, and the proof's facts are read and the record written
    // under them, so no driver of the run starts between the proof and the
    // write (decision 0108 ruling 3). A driver that appended between the
    // read above and these locks moves the head the fenced write names, and
    // the write refuses.
    let (reservations, _admission) =
        settlement::release::settle_locked(&root, run_id, store.path())
            .map_err(|unadmitted| refused(SettleRefusal::Unadmitted(unadmitted)))?;
    let _held = run_lock::held_at(store.path(), run_id).map_err(EngineError::RunLock)?;
    let attempts = proved(&reservations)
        .map_err(|live| EngineError::SettleRefused(SettleRefusal::Live(live)))?;
    let payload = json!({
        "command_id": Uuid::new_v4().to_string(),
        "command": SETTLE,
        "operator": ask.operator,
        "args": SettleArgs {
            run: run_id.to_string(),
            attempts,
            reason: ask.reason.to_string(),
        },
    });
    let head = Head::tip(&events);
    before_write(store);
    let cause = &head.event_id;
    let written = fenced_append(
        store,
        run_id,
        &head,
        cause,
        EventType::OperatorCommanded,
        payload,
        None,
    )?;
    // The record must read back as the contract, by type: the run may not
    // fold, so the read is [`release::record`], never `fold`. The fenced
    // append just landed in this journal, so a store that cannot read it
    // back has lost a write it acknowledged; that invariant is what
    // `settle_writes_the_record_its_contract_names` proves, and a removal
    // of the append's args fails it.
    let back = store.load(run_id)?;
    let read = settlement::release::record(&back).filter(|record| record.seq == written.seq);
    let read = read.expect("the record this verb just appended reads back");
    end_proven_holds(store, run_id, &read.args)?;
    Ok(written)
}

/// The record is the operator's settlement of each attempt it proves, a
/// probe's tree among them: every probe hold of the run naming one ends now
/// (fix round 0106b1r12, S2). The record is committed first, in its fenced
/// append, so an end that fails or is interrupted leaves the record, which
/// writer admission already honours, and the holds standing; a later
/// settle replays the record ([`replayed`]) and ends them.
fn end_proven_holds(store: &mut Store, run_id: &str, args: &SettleArgs) -> Result<(), EngineError> {
    let proven: Vec<String> = (args.attempts.iter())
        .map(|proven| proven.attempt.clone())
        .collect();
    store.settle_probe_holds(run_id, &proven, time::OffsetDateTime::now_utc())?;
    Ok(())
}

/// Where the run's head is already a settle record, the one a settle wrote
/// before its holds' end failed or was interrupted (the return of fix round
/// 0106b1r12): its holds are ended from that durable record, under no fresh
/// acknowledgement, for its proof was taken under the locks when it was
/// written, and that record is returned; nothing more is appended. `None`
/// where the head is no settle record, so the verb proves and writes anew.
fn replayed(
    store: &mut Store,
    run_id: &str,
    events: &[EventEnvelope],
) -> Result<Option<EventEnvelope>, EngineError> {
    let record = settlement::release::record(events);
    let head = events.last();
    let Some((record, head)) = record
        .zip(head)
        .filter(|(record, head)| record.seq == head.seq)
    else {
        return Ok(None);
    };
    end_proven_holds(store, run_id, &record.args)?;
    Ok(Some(head.clone()))
}

/// The attempts every reservation of the run names standing, proven as the
/// release proves them, one per attempt id: a record another build wrote,
/// read only through the view, is refused where it names no settled attempt
/// (decision 0090; ruling 3).
fn proved(
    reservations: &[crate::engine::broker::admission::Reservation],
) -> Result<Vec<Proven>, settlement::release::Live> {
    let mut proven: BTreeMap<String, Proven> = BTreeMap::new();
    for reservation in reservations {
        for attempt in provenances(reservation)? {
            proven.insert(attempt.attempt.clone(), attempt);
        }
    }
    Ok(proven.into_values().collect())
}
