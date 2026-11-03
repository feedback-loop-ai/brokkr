//! Ruling 4's comparison of a run's summary row with its fold: the first
//! column that differs, then the lag a stale index leaves, then what the
//! seat rows read as.

use rusqlite::params;
use rusqlite::types::Value;

use super::{
    kept, standing, written, Account, Compared, Disagreement, Listed, Store, StoreError, Summary,
};

impl Store {
    /// `run_id`'s summary row against `account`'s fold, column by column:
    /// the first column that differs, else behind where the index lags its
    /// events (`stale`), else what its seat rows read as (`seats`).
    pub(super) fn columns_compared(
        &self,
        run_id: String,
        account: &Account,
        stale: bool,
        seats: Compared,
    ) -> Result<Compared, StoreError> {
        let listed = Listed::of(&self.conn, &run_id)?;
        let summary = Summary::of(account, listed.listing());
        let standing = standing(&self.conn, &run_id)?;
        let mut expected = written(&summary);
        expected.extend(kept(&summary, standing.as_ref()));
        Ok(match self.first_difference(&run_id, &expected)? {
            Some(column) => Compared::Disagrees(Disagreement::Column { run_id, column }),
            // A seat row's lag names the rows that lag; the index's is the
            // whole row's when no narrower name stands.
            None if matches!(seats, Compared::SeatsBehind) => seats,
            None if stale => Compared::Behind,
            None => seats,
        })
    }

    /// The first of `expected`'s columns whose value on `run_id`'s row is
    /// not the one expected.
    fn first_difference(
        &self,
        run_id: &str,
        expected: &[(&'static str, Value)],
    ) -> Result<Option<&'static str>, StoreError> {
        let names: Vec<&str> = expected.iter().map(|(column, _)| *column).collect();
        let select = format!(
            "SELECT {} FROM run_summary WHERE run_id = ?1",
            names.join(", ")
        );
        let found: Vec<Value> = self.conn.query_row(&select, params![run_id], |row| {
            (0..names.len()).map(|at| row.get(at)).collect()
        })?;
        Ok(expected
            .iter()
            .zip(found)
            .find(|((_, expected), found)| *expected != *found)
            .map(|((column, _), _)| *column))
    }
}
