//! The suffix read (#354): what landed on a run after a head the caller
//! already verified, and the row reader the whole read shares.

use std::sync::LazyLock;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, EventEnvelope};
use rusqlite::{params, Connection, OptionalExtension};

use crate::damage::envelope;
use crate::event_index::{last_text, payload, seqs, CHECKPOINTED};
use crate::{patiently, Store, StoreError, HEAD_OF_RUN};

/// The rows of each `[run_id, seq]` pair of the JSON array `?1` at or
/// below its seq, one primary-key range each, in the array's order.
const ROWS_THROUGH: &str = "SELECT (SELECT count(*) FROM events
         WHERE run_id = head.value ->> 0 AND seq <= head.value ->> 1)
     FROM json_each(?1) AS head ORDER BY head.key";

/// A run's checkpoints between two seqs, both held, in seq order: the
/// rows a frame's trail omits, read through the primary key's range within
/// the window it names.
static CHECKPOINTS_BETWEEN: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index
         WHERE run_id = ?1 AND seq >= ?2 AND seq <= ?3 AND type = '{CHECKPOINTED}'
         ORDER BY seq"
    )
});

/// A run's checkpoints under the attempts of the JSON array `?2` that are
/// the engine's own `panel-member-finished` marker, at or below a seq, in
/// seq order: one query over every attempt (#1212, R1), read through the
/// stream index and filtered on the stored checkpoint before any envelope
/// is decoded, so a member-tagged seat turn or a step record is never read
/// (fix round 0109a1r8, R1). `member` alone cannot tell them apart:
/// `tag_member` writes it on every panel member's checkpoint.
static MEMBER_MARKERS: LazyLock<String> = LazyLock::new(|| {
    // The checkpoint object serde reads: the payload's last `checkpoint`,
    // as the index's own projection reads it, never `json_extract`'s first.
    let checkpoint = format!(
        "(SELECT value FROM {} WHERE key = 'checkpoint' ORDER BY id DESC LIMIT 1)",
        payload("e.envelope")
    );
    let entries =
        format!("json_each(CASE WHEN json_valid({checkpoint}) THEN {checkpoint} ELSE '{{}}' END)");
    format!(
        "SELECT i.seq FROM event_index AS i INDEXED BY event_index_stream
         JOIN events AS e ON e.run_id = i.run_id AND e.seq = i.seq
         WHERE i.run_id = ?1 AND i.attempt_id IN (SELECT value FROM json_each(?2))
           AND i.member IS NOT NULL
           AND i.seq <= ?3 AND i.type = '{CHECKPOINTED}'
           AND {step} = 'panel-member-finished' AND {authored} = 'engine'
         ORDER BY i.seq",
        step = last_text(&entries, "step"),
        authored = last_text(&entries, "authored"),
    )
});

impl Store {
    /// What landed on a run after a head the caller already loaded and
    /// verified, `(seq, hash)`: the events past `seq`, verified as the
    /// continuation of exactly that head, so a reader that keeps what it
    /// read — the engine, turn after turn — re-reads and re-hashes the
    /// suffix rather than the run. Events are append-only by trigger, so
    /// the prefix the caller holds is still the journal's; a suffix that
    /// does not chain onto `hash` refuses like a broken chain in
    /// [`Store::load`]. Empty when nothing has landed since, the head
    /// confirmed even then: a run the journal lacks is `RunNotFound`, and
    /// a head not its row at `seq` with `hash` is `UnknownHead`.
    pub fn load_after(
        &self,
        run_id: &str,
        seq: u64,
        hash: &str,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let head = i64::try_from(seq).map_err(|_| StoreError::UnknownHead { seq })?;
        patiently("load", self.patience, &self.path, || {
            let (stored, lowest): (Option<String>, Option<i64>) = self
                .conn
                .query_row(HEAD_OF_RUN, params![run_id, head], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()?
                .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
            // The empty head at 0 is every run's, and is no row.
            let stored = (head == 0).then(|| ZERO_HASH.into()).or(stored);
            if stored.as_deref() != Some(hash) {
                return Err(StoreError::UnknownHead { seq });
            }
            // A row below seq 1 lies before every chain a head extends, the
            // empty head's included: refused by name, never skipped.
            if let Some(seq) = lowest.filter(|seq| *seq < 1) {
                return Err(StoreError::RowBeforeChain {
                    run_id: run_id.to_string(),
                    seq,
                });
            }
            let events = self.rows(run_id, Some(head))?;
            verify_chain_after(run_id, seq, hash, &events)?;
            Ok(events)
        })
    }

    /// The event a run holds at `seq`, or `None` where no row is there:
    /// the one event a readout needs from a summary row whose head stands
    /// (decision 0103 slice 4f), read by its primary key and never the
    /// run whole.
    pub fn last_event(&self, run_id: &str, seq: u64) -> Result<Option<EventEnvelope>, StoreError> {
        let seq = i64::try_from(seq).map_err(|_| StoreError::UnknownHead { seq })?;
        patiently("last_event", self.patience, &self.path, || {
            let raw: Option<String> = self
                .conn
                .query_row(
                    "SELECT envelope FROM events WHERE run_id = ?1 AND seq = ?2",
                    params![run_id, seq],
                    |row| row.get(0),
                )
                .optional()?;
            raw.map(|raw| envelope(run_id, seq, &raw)).transpose()
        })
    }

    /// A run's checkpoints in `from..=to`, in seq order, each read by its
    /// key: the rows a frame's trail omits, read within a window the caller
    /// bounds (decision 0089 ruling 9, #1212). A journal that reads no
    /// index must read the run whole instead ([`Store::reads_index`]).
    pub fn envelopes_of_checkpoints(
        &self,
        run_id: &str,
        from: u64,
        to: u64,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let found = patiently("index", self.patience, &self.path, || {
            seqs(&self.conn, &CHECKPOINTS_BETWEEN, params![run_id, from, to])
        })?;
        self.envelopes_at(run_id, &found)
    }

    /// A run's checkpoints under the attempts `attempts` names that are the
    /// engine's own `panel-member-finished` marker, at or below `to`, in seq
    /// order, found in one query through the stream index and each read by
    /// its key: the markers a verdicts window's cut dropped, which a verdict
    /// inside that window is read with (decision 0089 ruling 9, #1212). The
    /// query still parses each named attempt's member-tagged checkpoints up
    /// to `to`, so its work grows with those attempts, not with the window.
    /// A member-tagged seat turn and a step record name a member too but
    /// carry no marker's word and no authorship, so neither is read. A
    /// journal that reads no index must read the run whole instead
    /// ([`Store::reads_index`]).
    pub fn envelopes_of_member_markers(
        &self,
        run_id: &str,
        attempts: &[&str],
        to: u64,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let attempts = serde_json::to_string(attempts)?;
        let found = patiently("index", self.patience, &self.path, || {
            seqs(&self.conn, &MEMBER_MARKERS, params![run_id, attempts, to])
        })?;
        self.envelopes_at(run_id, &found)
    }

    /// How many rows each run of `heads` holds at or below its seq, in
    /// `heads`' order: one query, counted over the `(run_id, seq)` key with
    /// no envelope read. A chain verified from seq 1 to a head holds
    /// exactly that head's seq of rows, so a reader that keeps what it read
    /// and is about to ask only past the head (#744) learns here whether a
    /// row below it is gone. A run the journal lacks holds none.
    pub fn rows_through(&self, heads: &[(&str, u64)]) -> Result<Vec<u64>, StoreError> {
        let heads = serde_json::to_string(heads)?;
        patiently("count", self.patience, &self.path, || {
            Ok(self
                .conn
                .prepare(ROWS_THROUGH)?
                .query_map(params![heads], |r| r.get::<_, u64>(0))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }

    /// A run's rows in order, parsed and not yet verified: every row of
    /// it, or those past `after`.
    pub(crate) fn rows(
        &self,
        run_id: &str,
        after: Option<i64>,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        rows_in(&self.conn, run_id, after)
    }
}

/// [`Store::rows`] on `conn`, so a transaction reads them as it stands.
pub(crate) fn rows_in(
    conn: &Connection,
    run_id: &str,
    after: Option<i64>,
) -> Result<Vec<EventEnvelope>, StoreError> {
    const ALL: &str = "SELECT envelope, seq FROM events WHERE run_id = ?1 ORDER BY seq";
    const AFTER: &str =
        "SELECT envelope, seq FROM events WHERE run_id = ?1 AND seq > ?2 ORDER BY seq";
    // The seq beside each envelope names a row whose envelope is not JSON.
    let row = |r: &rusqlite::Row<'_>| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?));
    let texts = match after {
        None => conn
            .prepare(ALL)?
            .query_map(params![run_id], row)?
            .collect::<Result<Vec<_>, _>>()?,
        Some(seq) => conn
            .prepare(AFTER)?
            .query_map(params![run_id, seq], row)?
            .collect::<Result<Vec<_>, _>>()?,
    };
    texts
        .iter()
        .map(|(raw, seq)| envelope(run_id, *seq, raw))
        .collect()
}

#[cfg(test)]
mod tests;
