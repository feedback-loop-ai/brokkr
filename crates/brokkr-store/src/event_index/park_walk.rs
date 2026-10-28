//! The parks' sweep cursor (decision 0089 ruling 8): one ascending walk of
//! the runs whose park rows need a refold, so a writable open reads the
//! journal's parks once rather than once per stale run.

use std::sync::LazyLock;

use rusqlite::{params_from_iter, Connection, OptionalExtension};

use super::{OFF_HEAD, PARKED};
use crate::StoreError;

/// The first unwritten park row's run past the walk's cursor, sought
/// through the partial index of rows whose stamp is NULL.
pub(super) static PARK_STALE: LazyLock<String> = LazyLock::new(|| stale(true));

/// [`PARK_STALE`] before the walk has a cursor. A journal may hold the
/// empty run id, which `run_id > ''` cannot name, so the first step has
/// no cursor at all.
static PARK_STALE_FIRST: LazyLock<String> = LazyLock::new(|| stale(false));

/// The first run whose written park stamp names no head it has, past the
/// walk's cursor, through the facts index.
static PARK_OFF_HEAD: LazyLock<String> = LazyLock::new(|| off_head(true));

/// [`PARK_OFF_HEAD`] before the walk has a cursor, as
/// [`PARK_STALE_FIRST`] is for [`PARK_STALE`].
static PARK_OFF_HEAD_FIRST: LazyLock<String> = LazyLock::new(|| off_head(false));

fn stale(after: bool) -> String {
    let cursor = if after { "AND run_id > ?1" } else { "" };
    format!(
        "SELECT run_id FROM event_index INDEXED BY event_index_park_stale
         WHERE type = '{PARKED}' AND facts_seq IS NULL {cursor}
         ORDER BY run_id LIMIT 1"
    )
}

fn off_head(after: bool) -> String {
    let cursor = if after { "AND run_id > ?1" } else { "" };
    format!(
        "SELECT run_id FROM event_index INDEXED BY event_index_park_facts
         WHERE type = '{PARKED}' AND {OFF_HEAD} {cursor}
         ORDER BY run_id LIMIT 1"
    )
}

/// One ascending walk of the runs whose park rows need a refold: the
/// unwritten rows are sought past the cursor, and the off-head rows are
/// read through the same cursor with one lookahead, so neither the fleet's
/// ids nor the current suffix outlives the step that reads it. A read that
/// finds no off-head row past the cursor is final: a refold only writes a
/// park at its head, so it cannot make one off it.
pub(super) struct ParkWalk {
    after: Option<String>,
    off_head: Option<String>,
    off_head_done: bool,
    #[cfg(test)]
    off_head_steps: i64,
    #[cfg(test)]
    stale_steps: i64,
}

impl ParkWalk {
    pub(super) fn new() -> Self {
        Self {
            after: None,
            off_head: None,
            off_head_done: false,
            #[cfg(test)]
            off_head_steps: 0,
            #[cfg(test)]
            stale_steps: 0,
        }
    }

    /// The next run whose park row needs a refold, in run id order.
    pub(super) fn next(&mut self, conn: &Connection) -> Result<Option<String>, StoreError> {
        let stale = self.stale(conn)?;
        let off_head = self.off_head(conn)?;
        let chosen = match (stale, off_head) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (left, right) => left.or(right),
        };
        if let Some(run) = &chosen {
            if self.off_head.as_deref() == Some(run.as_str()) {
                self.off_head = None;
            }
            self.after = Some(run.clone());
        }
        Ok(chosen)
    }

    fn stale(&mut self, conn: &Connection) -> Result<Option<String>, StoreError> {
        let (sql, after) = match &self.after {
            Some(after) => (&*PARK_STALE, Some(after.as_str())),
            None => (&*PARK_STALE_FIRST, None),
        };
        let mut statement = conn.prepare_cached(sql)?;
        let found = statement
            .query_row(params_from_iter(after), |row| row.get(0))
            .optional()?;
        self.note_stale(&statement);
        Ok(found)
    }

    fn off_head(&mut self, conn: &Connection) -> Result<Option<String>, StoreError> {
        if self.off_head.is_some() || self.off_head_done {
            return Ok(self.off_head.clone());
        }
        let (sql, after) = match &self.after {
            Some(after) => (&*PARK_OFF_HEAD, Some(after.as_str())),
            None => (&*PARK_OFF_HEAD_FIRST, None),
        };
        let mut statement = conn.prepare_cached(sql)?;
        let found = statement
            .query_row(params_from_iter(after), |row| row.get(0))
            .optional()?;
        self.note_off_head(&statement);
        if found.is_none() {
            self.off_head_done = true;
        }
        self.off_head = found.clone();
        Ok(found)
    }

    /// The VM instructions this walk's off-head read and stale seeks
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
