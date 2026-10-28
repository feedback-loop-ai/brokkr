//! The parks' sweep cursor (decision 0089 ruling 8): one ascending walk of
//! the runs whose park rows need a refold, so a writable open reads the
//! journal's parks once rather than once per stale run.

use std::collections::VecDeque;
use std::sync::LazyLock;

use rusqlite::{Connection, OptionalExtension};

use super::{OFF_HEAD, PARKED};
use crate::StoreError;

/// The first unwritten park row's run past the walk's cursor, sought
/// through the partial index of rows whose stamp is NULL.
pub(super) static PARK_STALE: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT run_id FROM event_index INDEXED BY event_index_park_stale
         WHERE type = '{PARKED}' AND facts_seq IS NULL AND run_id > ?1
         ORDER BY run_id LIMIT 1"
    )
});

/// Every run whose written park stamp names no head it has, through the
/// facts index: a missing trigger's leftovers, read once ahead of the
/// walk that refolds them.
static PARK_OFF_HEAD: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT run_id FROM event_index INDEXED BY event_index_park_facts
         WHERE type = '{PARKED}' AND {OFF_HEAD}
         ORDER BY run_id"
    )
});

/// One ascending walk of the runs whose park rows need a refold: the
/// unwritten rows are sought past the cursor, and the off-head rows are
/// read once ahead of it, since a refold writes every park at its head
/// and so cannot make one off it. The least run id of the two is handed
/// out, each run once.
pub(super) struct ParkWalk {
    after: String,
    off_head: VecDeque<String>,
    #[cfg(test)]
    off_head_steps: i64,
    #[cfg(test)]
    stale_steps: i64,
}

impl ParkWalk {
    pub(super) fn new(conn: &Connection) -> Result<Self, StoreError> {
        let mut walk = Self {
            after: String::new(),
            off_head: VecDeque::new(),
            #[cfg(test)]
            off_head_steps: 0,
            #[cfg(test)]
            stale_steps: 0,
        };
        let mut statement = conn.prepare_cached(&PARK_OFF_HEAD)?;
        let found = statement.query_map([], |row| row.get(0))?;
        let mut runs: Vec<String> = found.collect::<Result<_, _>>()?;
        runs.dedup();
        walk.off_head = runs.into();
        walk.note_off_head(&statement);
        Ok(walk)
    }

    /// The next run whose park row needs a refold, in run id order.
    pub(super) fn next(&mut self, conn: &Connection) -> Result<Option<String>, StoreError> {
        let stale = {
            let mut statement = conn.prepare_cached(&PARK_STALE)?;
            let found = statement.query_row([self.after.as_str()], |row| row.get(0));
            let found: Option<String> = found.optional()?;
            self.note_stale(&statement);
            found
        };
        let chosen = match (stale, self.off_head.front().cloned()) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (left, right) => left.or(right),
        };
        if let Some(run) = &chosen {
            if self.off_head.front() == Some(run) {
                self.off_head.pop_front();
            }
            self.after.clone_from(run);
        }
        Ok(chosen)
    }

    /// The VM instructions this walk's off-head scan and stale seeks
    /// stepped, for the test that binds its one-walk cost.
    #[cfg(test)]
    pub(super) fn steps(&self) -> (i64, i64) {
        (self.off_head_steps, self.stale_steps)
    }

    fn note_off_head(&mut self, statement: &rusqlite::Statement<'_>) {
        #[cfg(test)]
        {
            self.off_head_steps +=
                i64::from(statement.reset_status(rusqlite::StatementStatus::VmStep));
        }
        #[cfg(not(test))]
        let _ = statement;
    }

    fn note_stale(&mut self, statement: &rusqlite::Statement<'_>) {
        #[cfg(test)]
        {
            self.stale_steps +=
                i64::from(statement.reset_status(rusqlite::StatementStatus::VmStep));
        }
        #[cfg(not(test))]
        let _ = statement;
    }
}
