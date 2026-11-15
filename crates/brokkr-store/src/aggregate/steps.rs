//! The test-only VM-step probes for the window statements (decision 0089
//! ruling 9, slice 4's unit U5b1): the production statement each read
//! prepares, at the values it binds, measured after its rows are read.

use rusqlite::{params, params_from_iter, Statement, StatementStatus};

use super::*;

/// Run `statement` to its end and give back the VM steps it took.
fn steps(statement: &mut Statement<'_>, bound: impl rusqlite::Params) -> Result<i64, StoreError> {
    let mut rows = statement.query(bound)?;
    while rows.next()?.is_some() {}
    drop(rows);
    Ok(i64::from(statement.get_status(StatementStatus::VmStep)))
}

impl Store {
    /// The VM steps [`Store::window_runs`]' statement takes.
    pub(crate) fn window_runs_steps(
        &self,
        from: i64,
        to: i64,
        order: RunOrder,
        realm: Option<&str>,
    ) -> Result<i64, StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(runs_sql(order, intact))?;
        steps(&mut statement, params_from_iter(bounds(from, to, realm)))
    }

    /// The VM steps [`Store::durations`]' statement takes.
    pub(crate) fn durations_steps(&self, from: i64, to: i64) -> Result<i64, StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(durations_sql(intact))?;
        steps(&mut statement, params![from, to, SHAPE.as_str()])
    }

    /// The VM steps [`Store::finished_between`]' statement takes.
    pub(crate) fn finished_between_steps(&self, from: i64, to: i64) -> Result<i64, StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(finished_sql(intact))?;
        steps(&mut statement, params![from, to, SHAPE.as_str()])
    }
}
