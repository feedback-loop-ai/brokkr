//! A quarantined run, acknowledged (#511; decision 0076 item 11): who
//! looked at a run whose journal does not fold or does not load, when,
//! and why, kept in a table beside the chain rather than in it.
//!
//! Appending to a chain that does not fold would make a second journal
//! that does not fold, and nothing here repairs or folds past the defect
//! (decision 0001). So an acknowledgement is workspace bookkeeping, like
//! the queue: no event carries it, no fold observes it, and an export,
//! which is events, leaves it behind.
//!
//! It is keyed by the head it was made against, the run's last event, and
//! stands only while that is still the head: a journal that moves after it
//! reads as unacknowledged again, with nothing written, and the row stays
//! as history. The write is fenced on that head the way
//! [`Store::append_next_if_head`] is (decision 0029).

use brokkr_core::canonical::ZERO_HASH;
use rusqlite::{params, Connection, OptionalExtension};

use crate::schema::all_present;
use crate::{head_of, now_rfc3339, patiently, Attribution, Store, StoreError};

/// The table and its two guards. `IF NOT EXISTS` throughout, so racing
/// openers that both found it missing agree.
const MIGRATION_ACKNOWLEDGEMENTS: &str = r#"
CREATE TABLE IF NOT EXISTS quarantine_acknowledgements (
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    head_seq INTEGER NOT NULL,
    head_hash TEXT NOT NULL,
    operator TEXT NOT NULL,
    reason TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (run_id, head_hash)
);
CREATE TRIGGER IF NOT EXISTS quarantine_acknowledgements_append_only_update
    BEFORE UPDATE ON quarantine_acknowledgements
    BEGIN SELECT RAISE(ABORT, 'quarantine acknowledgements are append-only'); END;
CREATE TRIGGER IF NOT EXISTS quarantine_acknowledgements_append_only_delete
    BEFORE DELETE ON quarantine_acknowledgements
    BEGIN SELECT RAISE(ABORT, 'quarantine acknowledgements are append-only'); END;
"#;

const TABLE: [&str; 1] = ["quarantine_acknowledgements"];

const GUARDS: [&str; 2] = [
    "quarantine_acknowledgements_append_only_update",
    "quarantine_acknowledgements_append_only_delete",
];

/// The acknowledgement that stands on a run: the one made against the
/// run's current last event, or against no event (seq 0 and
/// [`ZERO_HASH`], as [`Store::head_hash`] reads an empty run) while it has
/// none. One statement, so the head it compares with is the head it reads.
const STANDING: &str = "SELECT ack.head_seq, ack.head_hash, ack.operator, ack.reason,
        ack.recorded_at
    FROM quarantine_acknowledgements AS ack
    WHERE ack.run_id = ?1
      AND ack.head_seq = COALESCE((SELECT MAX(seq) FROM events WHERE run_id = ?1), 0)
      AND ack.head_hash = COALESCE((SELECT event_hash FROM events WHERE run_id = ?1
          ORDER BY seq DESC LIMIT 1), ?2)";

/// An operator's acknowledgement of a quarantined run, at the event that
/// was its last when it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acknowledgement {
    /// The run's last event when it was acknowledged.
    pub seq: u64,
    pub event_hash: String,
    pub operator: String,
    pub reason: String,
    pub recorded_at: String,
}

/// Does this journal carry the table and both its guards? Pure reads of
/// `sqlite_master`, so the steady-state open takes no write lock.
pub(crate) fn acknowledgements_intact(conn: &Connection) -> Result<bool, StoreError> {
    Ok(all_present(conn, "table", &TABLE)? && all_present(conn, "trigger", &GUARDS)?)
}

/// Create the table and its guards, on a connection already holding the
/// write lock.
pub(crate) fn migrate_acknowledgements(conn: &Connection) -> Result<(), StoreError> {
    Ok(conn.execute_batch(MIGRATION_ACKNOWLEDGEMENTS)?)
}

/// The acknowledgement standing on `run_id`. A journal opened read-only
/// from before the table existed has none, and is never migrated to ask.
pub(crate) fn standing(
    conn: &Connection,
    run_id: &str,
) -> Result<Option<Acknowledgement>, StoreError> {
    if !all_present(conn, "table", &TABLE)? {
        return Ok(None);
    }
    Ok(conn
        .query_row(STANDING, params![run_id, ZERO_HASH], |row| {
            Ok(Acknowledgement {
                seq: row.get::<_, i64>(0)? as u64,
                event_hash: row.get(1)?,
                operator: row.get(2)?,
                reason: row.get(3)?,
                recorded_at: row.get(4)?,
            })
        })
        .optional()?)
}

/// [`Store::acknowledge_quarantine`]'s one transaction, as a body
/// [`patiently`] may run again: the head is re-read inside it, under the
/// write lock, before anything is written.
fn acknowledge_once(
    conn: &mut Connection,
    run_id: &str,
    (seq, hash): (u64, &str),
    by: Attribution<'_>,
    recorded_at: &str,
) -> Result<Acknowledgement, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (found_seq, found_hash) = head_of(&tx, run_id)?;
    if found_seq != seq || found_hash != hash {
        // Dropping the transaction rolls it back: nothing is written.
        return Err(StoreError::HeadMoved {
            expected_seq: seq,
            found_seq,
        });
    }
    if let Some(standing) = standing(&tx, run_id)? {
        return Err(StoreError::AlreadyAcknowledged {
            run_id: run_id.to_string(),
            seq: standing.seq,
        });
    }
    tx.execute(
        "INSERT INTO quarantine_acknowledgements
            (run_id, head_seq, head_hash, operator, reason, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            run_id,
            seq as i64,
            hash,
            by.operator,
            by.reason,
            recorded_at
        ],
    )?;
    tx.commit()?;
    Ok(Acknowledgement {
        seq,
        event_hash: hash.to_string(),
        operator: by.operator.to_string(),
        reason: by.reason.to_string(),
        recorded_at: recorded_at.to_string(),
    })
}

impl Store {
    /// Record `by`'s acknowledgement of quarantined run `run_id` at
    /// `head`, the last event's seq and hash as the caller read them
    /// before deciding the run is quarantined. Whether it is, is the
    /// caller's to read: the store neither folds nor repairs.
    ///
    /// Fenced: a head that moved since it was read is
    /// [`StoreError::HeadMoved`] and writes nothing, and a head already
    /// acknowledged is [`StoreError::AlreadyAcknowledged`].
    pub fn acknowledge_quarantine(
        &mut self,
        run_id: &str,
        head: (u64, &str),
        by: Attribution<'_>,
    ) -> Result<Acknowledgement, StoreError> {
        let recorded_at = now_rfc3339();
        let conn = &mut self.conn;
        patiently("acknowledge_quarantine", self.patience, || {
            acknowledge_once(conn, run_id, head, by, &recorded_at)
        })
    }

    /// The acknowledgement standing on `run_id`: one made at its current
    /// last event. `None` when there is none, when the journal moved after
    /// it, and on a journal from before the table.
    pub fn acknowledgement(&self, run_id: &str) -> Result<Option<Acknowledgement>, StoreError> {
        patiently("acknowledgement", self.patience, || {
            standing(&self.conn, run_id)
        })
    }
}

#[cfg(test)]
mod tests;
