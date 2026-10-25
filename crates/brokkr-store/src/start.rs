//! A run's start (decision 0095 ruling 4): its `runs` row and its first
//! event, `run/started`, written in one `Immediate` transaction, so no
//! crash leaves a run row with no events, from the queue or from `brokkr
//! run`. A fence the caller passes runs inside that transaction before
//! either write, and a fence that refuses writes neither: a start from the
//! queue passes the queue's [`claim_stands`](crate::claim_stands), which
//! owns what it reads, and `brokkr run` passes none.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::summary::{Account, Summary};
use rusqlite::{params, Connection, Transaction};
use serde_json::Value;

use crate::chain::{fence, seal_and_insert};
use crate::summary::{self, Carried, Listed, Settled};
use crate::{import, now_rfc3339, origin, patiently, refuse_existing, seat_summary};
use crate::{Store, StoreError};

/// A run's `runs` row, as it is written: where it is driven from already
/// asked, and its manifest already serialized.
pub(crate) struct Row<'a> {
    pub(crate) run_id: &'a str,
    pub(crate) feature: &'a str,
    pub(crate) bundle_name: &'a str,
    pub(crate) manifest: &'a str,
    pub(crate) origin: Option<&'a str>,
}

/// Write `row`, refused as [`StoreError::RunExists`] where its run id is
/// taken, with the run's summary and seat rows as the run with no event:
/// inside a transaction the caller begins and commits.
pub(crate) fn insert_run(tx: &Connection, row: &Row<'_>) -> Result<Listed, StoreError> {
    let &Row {
        run_id,
        feature,
        bundle_name,
        manifest,
        origin,
    } = row;
    refuse_existing(tx, run_id)?;
    let listed = Listed::new(feature, &now_rfc3339());
    let created_at = listed.listing().created_at;
    tx.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at, origin_host)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![run_id, feature, bundle_name, manifest, created_at, origin],
    )?;
    let empty = Account::empty(run_id);
    summary::write(tx, run_id, &Summary::of(&empty, listed.listing()))?;
    seat_summary::write(tx, &empty, None)?;
    Ok(listed)
}

/// A fence a start runs inside its transaction before it writes.
type Fence<'a> = &'a mut dyn FnMut(&Transaction<'_>) -> Result<(), StoreError>;

/// [`Store::start_run`]'s one transaction, as a body [`patiently`] may run
/// again: the fence, then the run's row, then its `run/started` sealed onto
/// the empty head, with the run's summary carried over it.
fn start_once(
    conn: &mut Connection,
    row: &Row<'_>,
    started: &Value,
    fenced: &mut Option<Fence<'_>>,
) -> Result<(EventEnvelope, Settled), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if let Some(fenced) = fenced {
        fenced(&tx)?;
    }
    let listed = insert_run(&tx, row)?;
    fence(&tx, row.run_id, EventType::RunStarted, started, 1)??;
    let empty = (0, ZERO_HASH.to_string());
    let envelope = seal_and_insert(
        &tx,
        row.run_id,
        empty,
        EventType::RunStarted,
        started.clone(),
        None,
        None,
    )?;
    let carried = Carried {
        account: Account::empty(row.run_id),
        listed: Some(listed),
    };
    let sealed = std::slice::from_ref(&envelope);
    let settled = summary::judge(&tx, Some(&carried), (0, ZERO_HASH), sealed)?;
    tx.commit()?;
    Ok((envelope, settled))
}

impl Store {
    /// Start a run: its row, as [`Store::create_run`] writes one, and its
    /// `run/started`, carrying `started`, in one `Immediate` transaction,
    /// so both land or neither does. A `fence` runs inside that transaction
    /// before either write, and its refusal is the start's, with nothing
    /// written; `None` starts unfenced, as `brokkr run` does. The store
    /// carries the run's fold from its start.
    pub fn start_run(
        &mut self,
        run_id: &str,
        feature: &str,
        bundle_name: &str,
        manifest: &Value,
        started: Value,
        mut fence: Option<Fence<'_>>,
    ) -> Result<EventEnvelope, StoreError> {
        import::recordable(run_id)?;
        let manifest = serde_json::to_string(manifest)?;
        // Asked before the write lock is taken, never under it.
        let origin = origin::local_host();
        let row = Row {
            run_id,
            feature,
            bundle_name,
            manifest: &manifest,
            origin: origin.as_deref(),
        };
        let conn = &mut self.conn;
        let (envelope, settled) = patiently("start_run", self.patience, || {
            start_once(conn, &row, &started, &mut fence)
        })?;
        #[cfg(any(test, feature = "test-support"))]
        self.binding.carried();
        self.settle((run_id, 1), settled);
        Ok(envelope)
    }
}

#[cfg(test)]
mod tests;
