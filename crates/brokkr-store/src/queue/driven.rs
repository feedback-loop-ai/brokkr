//! What a dispatcher's driver and `brokkr dispatch status` read of the
//! queue (decision 0110 rulings 3 and 6), each a plain read on this
//! store's connection that takes no holder and writes nothing, so it runs
//! on a journal opened read-only.
//!
//! A driver resumes only the run its entry is linked to under the claim
//! it was handed, once that entry's `spawn` row has landed. Status names
//! what a dispatcher left unsettled: each entry still launching, and each
//! one linked with no `spawn` row, as reconciliation reads them (decision
//! 0095 ruling 3).

use rusqlite::{params, OptionalExtension};

use super::claim::{standing_row, Step, STANDING};
use super::storage::queue_stored;
use super::{Claim, EntryId, Standing};
use crate::{patiently, Store, StoreError};

impl Store {
    /// The run `entry` is linked to under its claim `seq`, once the entry's
    /// `spawn` row has landed: the one `brokkr dispatch drive` resumes.
    /// `None` where the entry is not linked under that claim, or has no
    /// `spawn` row.
    pub fn queue_spawned(&self, entry: EntryId, seq: i64) -> Result<Option<String>, StoreError> {
        patiently("queue_spawned", self.patience, || {
            queue_stored(&self.conn)?;
            let query = "SELECT run_id FROM queue_claims
                WHERE seq = ?1 AND entry_id = ?2 AND run_id IS NOT NULL
                AND EXISTS (SELECT 1 FROM queue_commands WHERE entry_id = ?2 AND command = ?3)";
            let words = params![seq, entry.0, Step::Spawn.word()];
            Ok((self.conn.query_row(query, words, |row| row.get(0))).optional()?)
        })
    }

    /// Each entry a dispatcher left unsettled, under its latest claim, by
    /// entry: launching, with no run, or linked to its run with no `spawn`
    /// row.
    pub fn queue_unspawned(&self) -> Result<Vec<(Claim, Option<String>)>, StoreError> {
        patiently("queue_unspawned", self.patience, || {
            queue_stored(&self.conn)?;
            let words = params![Standing::Launching.word(), Step::Spawn.word()];
            let mut rows = self.conn.prepare(STANDING)?;
            let read = rows.query_map(words, standing_row)?;
            Ok(read.collect::<Result<_, _>>()?)
        })
    }
}

#[cfg(test)]
mod tests;
