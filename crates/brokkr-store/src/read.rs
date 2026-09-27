//! The suffix read (#354): what landed on a run after a head the caller
//! already verified, and the row reader the whole read shares.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, EventEnvelope};
use rusqlite::{params, OptionalExtension};

use crate::{patiently, Store, StoreError, HEAD_OF_RUN};

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
        patiently("load", self.patience, || {
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

    /// A run's rows in order, parsed and not yet verified: every row of
    /// it, or those past `after`.
    pub(crate) fn rows(
        &self,
        run_id: &str,
        after: Option<i64>,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let texts = match after {
            None => self
                .conn
                .prepare("SELECT envelope FROM events WHERE run_id = ?1 ORDER BY seq")?
                .query_map(params![run_id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?,
            Some(seq) => self
                .conn
                .prepare("SELECT envelope FROM events WHERE run_id = ?1 AND seq > ?2 ORDER BY seq")?
                .query_map(params![run_id, seq], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?,
        };
        let events = texts
            .into_iter()
            .map(|raw| serde_json::from_str::<EventEnvelope>(&raw))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }
}
