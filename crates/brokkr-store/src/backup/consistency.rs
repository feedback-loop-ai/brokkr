//! The event rows behind a snapshot's boundary: every row's SQL key and
//! stored hash must be its own envelope's, and no row may be unowned
//! (decision 0100 ruling 4). Backup and restore both call this before they
//! publish, so what a backup verifies is what a restored journal will use.

use brokkr_core::envelope::{verify_chain, EventEnvelope};
use rusqlite::params;

use super::BackupError;
use crate::{Store, StoreError};

/// Which stored fact of an event row a snapshot's envelope disagrees with
/// (decision 0100 ruling 4): the row's key, or the hash the store appends
/// under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowField {
    RunId,
    Seq,
    EventHash,
}

impl std::fmt::Display for RowField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RowField::RunId => "run_id",
            RowField::Seq => "seq",
            RowField::EventHash => "event_hash",
        })
    }
}

/// One event row's stored key and hash, beside the envelope it is read
/// with.
struct Position {
    run_id: String,
    seq: i64,
    event_hash: String,
}

/// Refuse a snapshot holding an event whose `run_id` no run owns: the
/// chains a reader verified are not the whole `events` table, and the
/// boundary declares what it counted, never what it happened to load. The
/// position is the table's own integer, reported as itself, never repaired
/// into a value it does not hold.
pub(super) fn refuse_unowned(store: &Store) -> Result<(), BackupError> {
    let count: u64 = store.conn.query_row(
        "SELECT count(*) FROM events WHERE run_id NOT IN (SELECT run_id FROM runs)",
        [],
        |row| row.get(0),
    )?;
    if count == 0 {
        return Ok(());
    }
    let (run_id, seq): (String, i64) = store.conn.query_row(
        "SELECT run_id, seq FROM events WHERE run_id NOT IN (SELECT run_id FROM runs)
         ORDER BY run_id, seq LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Err(BackupError::UnownedEvents { count, run_id, seq })
}

/// One run's whole chain, verified as [`Store::load`] verifies it, and then
/// every row's SQL identity and stored hash held to its own envelope. The
/// chain check comes first so an envelope defect keeps its existing
/// [`BackupError::Chain`] name; a row the chain cannot see, because only
/// its SQL key or stored hash is wrong, is refused here.
pub(super) fn chain(store: &Store, run_id: &str) -> Result<Vec<EventEnvelope>, BackupError> {
    let (events, positions) = rows(store, run_id)?;
    verify_chain(&events).map_err(|source| BackupError::Chain {
        run_id: run_id.to_string(),
        source,
    })?;
    for (position, envelope) in positions.into_iter().zip(&events) {
        check(position, envelope)?;
    }
    Ok(events)
}

/// A run's envelopes in the order the table holds them, each beside the
/// SQL key and stored hash the same row carries.
fn rows(store: &Store, run_id: &str) -> Result<(Vec<EventEnvelope>, Vec<Position>), BackupError> {
    let mut statement = store.conn.prepare(
        "SELECT run_id, seq, event_hash, envelope FROM events
         WHERE run_id = ?1 ORDER BY seq",
    )?;
    let mut rows = statement.query(params![run_id])?;
    let mut events = Vec::new();
    let mut positions = Vec::new();
    while let Some(row) = rows.next()? {
        let run_id: String = row.get(0)?;
        let seq: i64 = row.get(1)?;
        let event_hash: String = row.get(2)?;
        let text: String = row.get(3)?;
        events.push(serde_json::from_str(&text).map_err(StoreError::Json)?);
        positions.push(Position {
            run_id,
            seq,
            event_hash,
        });
    }
    Ok((events, positions))
}

/// Refuse the first stored fact that is not its envelope's: the row's
/// `run_id`, its `seq`, or the `event_hash` the store would have written
/// for the envelope on append. The hash is
/// [`EventEnvelope::compute_hash`], never a second digest.
fn check(position: Position, envelope: &EventEnvelope) -> Result<(), BackupError> {
    if envelope.run_id != position.run_id {
        return Err(BackupError::RowMismatch {
            found: position.run_id.clone(),
            run_id: position.run_id,
            seq: position.seq,
            field: RowField::RunId,
            expected: envelope.run_id.clone(),
        });
    }
    if u64::try_from(position.seq).ok() != Some(envelope.seq) {
        return Err(BackupError::RowMismatch {
            run_id: position.run_id,
            seq: position.seq,
            field: RowField::Seq,
            found: position.seq.to_string(),
            expected: envelope.seq.to_string(),
        });
    }
    let expected = envelope.compute_hash();
    if position.event_hash != expected {
        return Err(BackupError::RowMismatch {
            run_id: position.run_id,
            seq: position.seq,
            field: RowField::EventHash,
            found: position.event_hash,
            expected,
        });
    }
    Ok(())
}
