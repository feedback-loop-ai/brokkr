//! A window's runs and parks pulled one row at a time (decision 0089 ruling
//! 7, slice 4's unit U5b2b1): the readout merges several hearths' streams
//! of one figure, so it asks each for its next row when the merge needs
//! it, rather than taking every row as the store steps.
//!
//! A pull hands `read` a cursor over the figure's statement, and the
//! statement is finalized when `read` returns. Every row is borrowed from
//! the cursor until the next is asked for; nothing of the window is held.
//! The order is the statement's: the figure's own key, so a reader merges
//! streams without sorting them. Each pull reads under whatever snapshot
//! the caller holds ([`crate::snapshot::ReadSnapshot`]).

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Rows, StatementStatus};

use super::{park_bounds, park_sql, runs_sql, window_park, window_run, RunOrder};
use super::{WindowPark, WindowRun};
use crate::event_index::{CHECKPOINTED, RUN_COSTS};
use crate::seat_summary::{stale_intact, SHAPE};
use crate::{Store, StoreError};

/// The bounds a window statement reads by (decision 0071 ruling 10's
/// parameter object): the window `[from, to)`, the stored realm key it
/// narrows to where given, and the hearth's label, which orders a run whose
/// stored realm is NULL where the caller names that run's realm, so the
/// stream stands in the caller's key order. Without a label a NULL realm
/// orders first.
#[derive(Debug, Clone, Copy)]
pub struct Bounds<'a> {
    pub from: i64,
    pub to: i64,
    pub realm: Option<&'a str>,
    pub label: Option<&'a str>,
}

impl Bounds<'_> {
    /// The five values a labelled window statement binds: the window, the
    /// realm key or NULL, this build's seat shape and the label or NULL.
    pub(crate) fn values(&self) -> [Value; 5] {
        let text = |text: Option<&str>| text.map_or(Value::Null, |text| Value::Text(text.into()));
        [
            Value::Integer(self.from),
            Value::Integer(self.to),
            text(self.realm),
            Value::Text(SHAPE.to_string()),
            text(self.label),
        ]
    }
}

/// The window a park read reads by: the window, the moment an open park's
/// end is read as, and the stored realm key it narrows to where given.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParkWindow<'a> {
    pub(crate) from: i64,
    pub(crate) to: i64,
    pub(crate) now: i64,
    pub(crate) realm: Option<&'a str>,
}

/// A window's runs, pulled in their statement's order.
pub struct RunRows<'s> {
    rows: Rows<'s>,
}

impl RunRows<'_> {
    /// The next run, borrowed from its row until the next is asked for;
    /// `None` once the window holds no more.
    pub fn next_run(&mut self) -> Result<Option<WindowRun<'_>>, StoreError> {
        match self.rows.next()? {
            Some(row) => Ok(Some(window_run(row)?)),
            None => Ok(None),
        }
    }
}

/// A window's parks, pulled in cut phase then run order.
pub struct ParkRows<'s> {
    rows: Rows<'s>,
}

impl ParkRows<'_> {
    /// The next park, borrowed from its row until the next is asked for;
    /// `None` once the window holds no more.
    pub fn next_park(&mut self) -> Result<Option<WindowPark<'_>>, StoreError> {
        match self.rows.next()? {
            Some(row) => Ok(Some(window_park(row)?)),
            None => Ok(None),
        }
    }
}

/// One run's reported costs inside a window, by the run's key: one range
/// of [`RUN_COSTS`], so the run's history outside the window is never
/// walked, in time then event order, the order the whole read sums them in.
static RUN_SPENDS: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        "SELECT usd FROM event_index INDEXED BY {RUN_COSTS}
          WHERE run_id = ?1 AND type = '{CHECKPOINTED}' AND usd IS NOT NULL
            AND at_ms >= ?2 AND at_ms < ?3
          ORDER BY at_ms, seq"
    )
});

impl Store {
    /// Hand `read` the runs [`Store::window_runs`] reads, in `order`, as a
    /// cursor it pulls. A statement that will not open is refused before
    /// `read` is called; a row that will not read is the cursor's refusal.
    pub fn pull_runs<T>(
        &self,
        bounds: &Bounds<'_>,
        order: RunOrder,
        read: impl FnOnce(&mut RunRows<'_>) -> T,
    ) -> Result<T, StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(runs_sql(order, intact))?;
        let rows = statement.query(params_from_iter(bounds.values()))?;
        let read = read(&mut RunRows { rows });
        Ok(read)
    }

    /// Hand `read` the parks [`Store::window_parks`] reads, as a cursor it
    /// pulls, `now` the moment an open park's end is read as.
    pub fn pull_parks<T>(
        &self,
        from: i64,
        to: i64,
        now: i64,
        realm: Option<&str>,
        read: impl FnOnce(&mut ParkRows<'_>) -> T,
    ) -> Result<T, StoreError> {
        let window = ParkWindow {
            from,
            to,
            now,
            realm,
        };
        self.park_rows(&window, read, |_| {})
    }

    /// [`Store::pull_parks`] with the statement's VM steps handed to
    /// `stepped` once `read` returns, so the test that binds the ranges'
    /// cost measures the production statement itself.
    pub(crate) fn park_rows<T>(
        &self,
        window: &ParkWindow<'_>,
        read: impl FnOnce(&mut ParkRows<'_>) -> T,
        stepped: impl FnOnce(i64),
    ) -> Result<T, StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(park_sql(intact))?;
        let bounds = park_bounds(window.from, window.to, window.now, window.realm);
        let rows = statement.query(params_from_iter(bounds))?;
        let read = read(&mut ParkRows { rows });
        stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
        Ok(read)
    }

    /// One run's reported cost inside `[from, to)`, summed as each is read,
    /// in time then event order; `None` where it reported none there. The
    /// readout joins a run's cost to the run as it pulls it, through the
    /// run's own key, and holds neither the run's costs nor the window's
    /// (decision 0089 ruling 7). The statement is the connection's cached
    /// one, prepared once a readout.
    pub fn run_spent(&self, run_id: &str, from: i64, to: i64) -> Result<Option<f64>, StoreError> {
        self.run_spent_stepped(run_id, from, to, |_| {})
    }

    /// [`Store::run_spent`] with the statement's VM steps handed to
    /// `stepped`, so the test that binds the range's cost measures the
    /// production statement itself.
    pub(crate) fn run_spent_stepped(
        &self,
        run_id: &str,
        from: i64,
        to: i64,
        stepped: impl FnOnce(i64),
    ) -> Result<Option<f64>, StoreError> {
        let mut statement = self.conn.prepare_cached(&RUN_SPENDS)?;
        statement.reset_status(StatementStatus::VmStep);
        let mut spent = None;
        let mut rows = statement.query(params![run_id, from, to])?;
        while let Some(row) = rows.next()? {
            spent = Some(spent.unwrap_or(0.0) + row.get::<_, f64>(0)?);
        }
        drop(rows);
        stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
        Ok(spent)
    }
}
