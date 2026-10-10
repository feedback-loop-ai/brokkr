//! Appending several events of one run in one transaction, each caused by
//! the one before it (#464): a writer that held rows behind a peer's lock
//! takes the lock once to land them all, not once per row.

use brokkr_core::envelope::{EventEnvelope, EventType, EVENT_SCHEMA_VERSION};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    head_of, now_rfc3339, patiently, seat_record, SeatRecordError, SeatRecordVersion, Store,
    StoreError, ENGINE_OF_RUN,
};

/// The seat-record fence (decision 0034, ruling 6), inside the transaction
/// that would write `payload` at `seq`: a checkpoint or a successful
/// result that violates the contract is refused HERE, at the seq it would
/// have taken, before it is sealed into the chain. The journal is
/// append-only, so a nonconforming record that landed could never be
/// corrected — only discovered at export, when the run was already wanted
/// as evidence, and by then every verb that makes evidence of it (export,
/// anchor, offline verify) refuses the whole run forever. The outer error
/// is the journal's; the inner one is the fence's verdict.
pub(crate) fn fence(
    tx: &Connection,
    run_id: &str,
    event_type: EventType,
    payload: &Value,
    seq: u64,
) -> Result<Result<(), SeatRecordError>, StoreError> {
    let Some(record) = seat_record::record_of(event_type, payload) else {
        return Ok(Ok(()));
    };
    // Which contract this run writes under is settled once, at
    // `run/started`, and `runs.manifest` is immutable by trigger — so the
    // engine named there is the same answer the export and verify sweeps
    // reach through the journal, read here inside the same transaction
    // the row would be sealed in. SQLite extracts the one string, so the
    // manifest is not parsed whole under the lock on every record (#354).
    let engine: Option<String> = tx
        .query_row(ENGINE_OF_RUN, params![run_id], |r| r.get(0))
        .optional()?
        .flatten();
    let version = SeatRecordVersion::of_manifest(engine.as_deref());
    Ok(seat_record::validate_seat_record(record, seq, version))
}

/// Seal `payload` as the run's next event after `head`, its seq and hash,
/// and insert it inside `tx`; a seq a peer already took is
/// [`StoreError::AppendConflict`].
pub(crate) fn seal_and_insert(
    tx: &Connection,
    run_id: &str,
    (last_seq, previous_hash): (u64, String),
    event_type: EventType,
    payload: Value,
    causation_id: Option<String>,
    attempt_id: Option<String>,
) -> Result<EventEnvelope, StoreError> {
    let envelope = EventEnvelope {
        run_id: run_id.to_string(),
        seq: last_seq + 1,
        event_id: uuid::Uuid::new_v4().to_string(),
        event_schema_version: EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id,
        correlation_id: run_id.to_string(),
        attempt_id,
        recorded_at: now_rfc3339(),
        previous_hash,
        event_hash: String::new(),
    }
    .sealed();
    let serialized = serde_json::to_string(&envelope)?;
    let inserted = tx.execute(
        "INSERT OR IGNORE INTO events (run_id, seq, event_hash, envelope)
         VALUES (?1, ?2, ?3, ?4)",
        params![run_id, envelope.seq as i64, envelope.event_hash, serialized],
    )?;
    if inserted == 0 {
        return Err(StoreError::AppendConflict { seq: envelope.seq });
    }
    Ok(envelope)
}

/// What one chained append wrote: the events it appended, in order, and,
/// when the seat-record fence refused one (decision 0034, ruling 6), that
/// refusal. The refused row is the one after the last appended, and
/// nothing after it was written.
#[derive(Debug)]
pub struct Chain {
    pub appended: Vec<EventEnvelope>,
    pub refused: Option<SeatRecordError>,
}

/// The rows a chained append writes, and how each becomes its event's
/// payload: only once the transaction holds the lock, so an attempt a
/// peer's lock refuses builds nothing.
pub struct Rows<'a, T> {
    pub rows: &'a [T],
    pub payload: &'a dyn Fn(&T) -> Value,
}

/// [`Store::append_chain`]'s one transaction, as a body [`patiently`] may
/// run again: every fact it writes is derived inside it, as
/// [`Store::append_next`]'s is. The rows before a refused one are
/// committed; any other error rolls back the whole chain.
fn chain_once<T>(
    conn: &mut Connection,
    run_id: &str,
    event_type: EventType,
    rows: &Rows<'_, T>,
    causation_id: Option<String>,
    attempt_id: Option<String>,
) -> Result<Chain, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut head = head_of(&tx, run_id)?;
    let mut cause = causation_id;
    let mut chain = Chain {
        appended: Vec::new(),
        refused: None,
    };
    for row in rows.rows {
        let payload = (rows.payload)(row);
        if let Err(refusal) = fence(&tx, run_id, event_type, &payload, head.0 + 1)? {
            chain.refused = Some(refusal);
            break;
        }
        let event = seal_and_insert(
            &tx,
            run_id,
            head,
            event_type,
            payload,
            cause,
            attempt_id.clone(),
        )?;
        head = (event.seq, event.event_hash.clone());
        cause = Some(event.event_id.clone());
        chain.appended.push(event);
    }
    tx.commit()?;
    Ok(chain)
}

impl Store {
    /// Append one event of `event_type` per row, in order, in one
    /// transaction: the first caused by `causation_id`, each next one by
    /// the event before it. The seat-record fence still judges every row,
    /// at the seq it would take; the rows before one it refuses land, and
    /// the refusal is returned beside them in the [`Chain`].
    pub fn append_chain<T>(
        &mut self,
        run_id: &str,
        event_type: EventType,
        rows: Rows<'_, T>,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<Chain, StoreError> {
        let conn = &mut self.conn;
        patiently("append", self.patience, || {
            chain_once(
                conn,
                run_id,
                event_type,
                &rows,
                causation_id.clone(),
                attempt_id.clone(),
            )
        })
    }

    /// [`Store::append_chain`], spending none of this store's patience: a
    /// peer that holds the write lock returns [`StoreError::Contended`] at
    /// once, and nothing was written. For a writer that keeps the rows and
    /// offers them again later, so a peer's lock costs it no wait (#394).
    pub fn append_chain_without_waiting<T>(
        &mut self,
        run_id: &str,
        event_type: EventType,
        rows: Rows<'_, T>,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<Chain, StoreError> {
        let patience = self.patience;
        self.set_patience(std::time::Duration::ZERO)?;
        let appended = self.append_chain(run_id, event_type, rows, causation_id, attempt_id);
        self.set_patience(patience)?;
        appended
    }
}

#[cfg(test)]
mod tests;
