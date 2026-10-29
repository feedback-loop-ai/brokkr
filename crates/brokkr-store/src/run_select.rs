//! The run selector's key reads (decision 0089 ruling 11, slice 4): the
//! few rows decision 0015's rules need, read by key rather than as the
//! whole `runs` table.
//!
//! [`Store::latest_run`] is one row from the `(created_at)` index, ties
//! taken in `list_runs`' order; [`Store::run_candidates`] is one
//! primary-key seek for an exact id and otherwise a range over the
//! `run_id` key that stops at the first id the prefix no longer matches.
//! A journal opened read-only without the index still answers correctly
//! — SQLite scans — at the cost the index exists to remove.

use rusqlite::{params, Connection, OptionalExtension};

use crate::{patiently, schema, Store, StoreError};

/// The additive index `latest` reads: the newest run by `created_at`,
/// ties in `list_runs`' order. `IF NOT EXISTS`, so a writable open adds
/// it to an older journal, and `DATABASE_SCHEMA` does not move for it.
pub(crate) const INDEX: &str = "runs_created_at";

/// The index's statement, its name held once.
pub(crate) fn migration() -> String {
    format!("CREATE INDEX IF NOT EXISTS {INDEX} ON runs (created_at);")
}

/// Does the journal carry the index? A pure read of `sqlite_master`.
pub(crate) fn intact(conn: &Connection) -> Result<bool, StoreError> {
    schema::all_present(conn, "index", &[INDEX])
}

/// Add the index, inside the open's immediate transaction.
pub(crate) fn migrate(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(&migration())?;
    Ok(())
}

impl Store {
    /// The newest run by `created_at`, or `None` on an empty journal.
    ///
    /// `list_runs` lists runs by `created_at` and, where two stamps are
    /// equal, in the row's insertion order. `resolve` takes the LAST of
    /// equal maxima, so the one-row read takes the same: `rowid` is that
    /// insertion order, and both keys descending is the index walked
    /// backwards, one row.
    pub fn latest_run(&self) -> Result<Option<(String, String)>, StoreError> {
        patiently("latest_run", self.patience, || {
            let mut statement = self.conn.prepare(
                "SELECT run_id, created_at FROM runs
                 ORDER BY created_at DESC, rowid DESC LIMIT 1",
            )?;
            let found = statement
                .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
                .optional()?;
            self.note(&statement);
            Ok(found)
        })
    }

    /// The rows decision 0015's `resolve` needs for a selector that is
    /// not `latest`: the exact id by the primary key when it exists,
    /// otherwise every run whose id starts with `requested`, in
    /// `list_runs`' order.
    ///
    /// The prefix is a key range, not a pattern: `%` and `_` are
    /// ordinary characters, and the scan stops at the first id the
    /// prefix no longer matches. An empty selector reads nothing, since
    /// it is a prefix of every id and `resolve` refuses it before any
    /// candidate is considered.
    pub fn run_candidates(&self, requested: &str) -> Result<Vec<(String, String)>, StoreError> {
        patiently("run_candidates", self.patience, || {
            self.candidates_once(requested)
        })
    }

    fn candidates_once(&self, requested: &str) -> Result<Vec<(String, String)>, StoreError> {
        if requested.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(exact) = self.exact_run(requested)? {
            return Ok(vec![exact]);
        }
        self.prefixed_runs(requested)
    }

    fn exact_run(&self, requested: &str) -> Result<Option<(String, String)>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT run_id, created_at FROM runs WHERE run_id = ?1")?;
        let found = statement
            .query_row(params![requested], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        self.note(&statement);
        Ok(found)
    }

    fn prefixed_runs(&self, prefix: &str) -> Result<Vec<(String, String)>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT run_id, created_at, rowid FROM runs
             WHERE run_id >= ?1 ORDER BY run_id",
        )?;
        let mut found: Vec<(String, String, i64)> = Vec::new();
        {
            let mut rows = statement.query(params![prefix])?;
            while let Some(row) = rows.next()? {
                let run_id: String = row.get(0)?;
                if !run_id.starts_with(prefix) {
                    break;
                }
                found.push((run_id, row.get(1)?, row.get(2)?));
            }
        }
        self.note(&statement);
        // `list_runs` orders by the recorded stamp, then by insertion.
        found.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.2.cmp(&right.2)));
        Ok(found
            .into_iter()
            .map(|(run_id, created_at, _)| (run_id, created_at))
            .collect())
    }

    /// Count `statement`'s VM steps against this store, for the test that
    /// binds the selector's cost.
    fn note(&self, statement: &rusqlite::Statement<'_>) {
        #[cfg(test)]
        self.selector_steps.set(
            self.selector_steps.get()
                + i64::from(statement.reset_status(rusqlite::StatementStatus::VmStep)),
        );
        #[cfg(not(test))]
        let _ = statement;
    }

    /// Take the steps the selector's reads have taken since the last
    /// call: what the test counts the reads' cost with.
    #[cfg(test)]
    pub(crate) fn take_selector_steps(&self) -> i64 {
        self.selector_steps.replace(0)
    }
}

#[cfg(test)]
mod tests;
