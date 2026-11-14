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
use uuid::Uuid;

use super::{fenced_append, EngineError, Head};
use crate::engine::broker::settlement;
use crate::engine::broker::settlement::release::{provenances, Proven, SettleArgs};
use crate::engine::SettleRefusal;
use crate::run_lock;

#[cfg(test)]
mod tests;

/// What `brokkr operator settle` says, as the operator typed it: why the
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
    let refused = |why: String| EngineError::SettleRefused(SettleRefusal::Refused(why));
    // The run's own record decides the cheap refusals first: a run that
    // has not ended, or that no acknowledgement stands on, is refused
    // before any coordination root is opened or any lock is taken.
    let events = store.load(run_id)?;
    let Some(_terminal) = events
        .iter()
        .find(|event| terminal_event(event.event_type).is_some())
    else {
        return Err(refused(format!(
            "run '{run_id}' has no terminal event; settle records that a run which has already \
             ended no longer holds its worktree"
        )));
    };
    let acknowledged = store.acknowledgement(run_id)?.is_some();
    // A repair moves the head past the acknowledgement and lapses it
    // (#511); the repair record is the operator's word that the run was
    // looked at, and stands in its place (decision 0108 ruling 4).
    let repaired = settlement::release::repaired(&events);
    if !(acknowledged | repaired) {
        return Err(refused(format!(
            "run '{run_id}' is not acknowledged, so it is not quarantined; answer it with \
             `brokkr operator acknowledge --run {run_id} --reason <why>` first"
        )));
    }
    let observed = crate::engine::broker::observed();
    let root = observed
        .home
        .as_deref()
        .and_then(observed.open)
        .ok_or_else(|| {
            refused(
                "this host names no protected coordination root, so no reservation can be read"
                    .to_string(),
            )
        })?;
    // The locks are taken in admission's own order, the admission lock and
    // then the run's, and the proof's facts are read and the record written
    // under them, so no driver of the run starts between the proof and the
    // write (decision 0108 ruling 3). A driver that appended between the
    // read above and these locks moves the head the fenced write names, and
    // the write refuses.
    let (reservations, _admission) =
        settlement::release::settle_locked(&root, run_id, store.path())
            .map_err(|unadmitted| refused(unadmitted.to_string()))?;
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
    let _ = read.expect("the record this verb just appended reads back");
    Ok(written)
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
