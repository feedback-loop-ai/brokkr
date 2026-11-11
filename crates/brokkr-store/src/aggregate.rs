//! The aggregate readouts' reads (decision 0089 ruling 7, slice 4): the
//! window's runs and parks, the preceding window's count, the medians'
//! duration pass and the runs behind their rows, each streamed one row at a
//! time through a closure of plain values.
//!
//! The writer keeps the facts on `run_summary` and `event_index` (rulings
//! 2 and 7); these reads visit them by the index each figure folds through,
//! never the run whole. The store carries no `brokkr-view` type: a run is
//! the stored keys as they stand, cut where the writer cut them, and the
//! caller falls back to a hearth's label where the stored realm is NULL, as
//! `analytics`' `facts` does today.
//!
//! A park is read by its span class, `span_class = c AND at_ms >= from -
//! 2^c AND at_ms < to` for every closed class, then rejected unless its
//! reach passed the window's start, and the open parks by class 64. Since a
//! closed park's reach is less than its class's power of two, every park
//! that reaches the window is in its class's range and no park far before
//! the window is visited. Class 63's bound saturates at [`i64::MIN`].
//!
//! The reads have no production caller until slice 4's unit U4 lands them
//! in `brokkr-cli`; they are `pub` on [`Store`] so it can, and each is in
//! the public API snapshot until then.

use std::sync::LazyLock;

use brokkr_core::ending::Ending;
use brokkr_core::fold::Status;
use brokkr_core::page::Lane;
use rusqlite::types::{Value, ValueRef};
use rusqlite::Connection;
use rusqlite::{params, params_from_iter, Row, StatementStatus};

use crate::event_index::{finished, index_intact, BACKFILLED as INDEXED, PARKED};
use crate::summary::{summary_intact, BACKFILLED as SUMMARISED};
use crate::{Store, StoreError};

/// The order a window's runs are folded in (decision 0089 ruling 7): the
/// two key orders the figures group by, or no order when the caller keeps
/// its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOrder {
    /// The index's own order, which no figure reads.
    Unordered,
    /// By the stored ending rule, then the run's id.
    ByEndingRule,
    /// By the stored realm, then recipe, then the run's id.
    ByRealmThenRecipe,
}

/// Where a run's review rulings sent it, as `run_summary` keeps the three
/// flags (ruling 7): its first sent it back, its second routed it on, and
/// any sent it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowReviews {
    pub first: bool,
    pub second: bool,
    pub back: bool,
}

/// One run a window reads, borrowed from its row: the stored aggregate
/// columns (ruling 7), the ending as its columns keep it, and whether the
/// run is still open.
#[derive(Debug, Clone, Copy)]
pub struct WindowRun<'a> {
    pub run_id: &'a str,
    /// The realm its start named, NULL where it named none: the caller
    /// falls back to the hearth's label as `analytics`' facts does.
    pub realm: Option<&'a str>,
    pub recipe: Option<&'a str>,
    pub ending_rule: Option<&'a str>,
    /// Whether the fold stood at the policy's hard stop there: status
    /// `stopped`, at phase `stop`, whatever rule the last ruling named.
    pub hard_stop: bool,
    pub started_ms: Option<i64>,
    pub finished_ms: Option<i64>,
    /// The ending, in decision 0064's twelve, parsed at this edge.
    pub ending: Option<Ending>,
    pub implement_visits: u64,
    pub reviews: WindowReviews,
    /// Still running, parked or stopping, never concluded.
    pub open: bool,
}

/// One park a window reads, borrowed from its row: the run and realm it
/// belongs to, the phase it was entered from, its moments (an open park's
/// end read as `now`), and whether it is open.
#[derive(Debug, Clone, Copy)]
pub struct WindowPark<'a> {
    pub run_id: &'a str,
    pub realm: Option<&'a str>,
    pub phase: Option<&'a str>,
    pub at: i64,
    pub answered: Option<i64>,
    pub ended: i64,
    pub open: bool,
}

impl Store {
    /// Whether the aggregates' rows are read (ruling 8): the summary's
    /// aggregate columns and the event index's park facts are present,
    /// intact and backfilled, their `meta` marks finished, and no park row
    /// is still waiting for its facts. The caller reads a hearth whole
    /// when this is false.
    pub fn aggregates_ready(&self) -> Result<bool, StoreError> {
        if !summary_intact(&self.conn)? || !index_intact(&self.conn)? {
            return Ok(false);
        }
        let marked = finished(&self.conn, SUMMARISED)? && finished(&self.conn, INDEXED)?;
        Ok(marked && !parks_unfinished(&self.conn)?)
    }

    /// Every run whose summary row is behind its head, streamed by its
    /// id through the `behind` lane's index: a run a writer did not fold,
    /// or one whose aggregate stamp is not its folded head's. The caller
    /// reads each one whole (ruling 8).
    pub fn behind_runs(&self, mut visit: impl FnMut(&str)) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&BEHIND)?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            visit(required_text(row, 0)?);
        }
        Ok(())
    }

    /// The runs a window reads (ruling 7): those that finished inside it,
    /// through the `(finished_ms)` index, and those still open, through
    /// the lane index; a behind row is never among them, read whole
    /// instead. `order` is the figure's key order, `realm` narrows to one
    /// stored realm key where given.
    pub fn window_runs(
        &self,
        from: i64,
        to: i64,
        order: RunOrder,
        realm: Option<&str>,
        mut visit: impl FnMut(&WindowRun<'_>),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(runs_sql(order))?;
        let bounds = run_bounds(from, to, realm);
        let mut rows = statement.query(params_from_iter(bounds))?;
        while let Some(row) = rows.next()? {
            visit(&window_run(row)?);
        }
        Ok(())
    }

    /// The shipped runs that finished in the equal window before the
    /// headline's (ruling 7): the headline's `previous` counts the runs
    /// that completed, as `brokkr-view`'s `shipped_in` does, so one count
    /// over the `(finished_ms)` index, holding one integer.
    pub fn finished_between(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
    ) -> Result<u64, StoreError> {
        let count: i64 = self
            .conn
            .query_row(&COUNT_FINISHED, params![from, to, realm], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// One pass of the median's selection: every finished run's stored
    /// realm key and its span in the window, through the `(finished_ms)`
    /// index, streamed and holding nothing between passes.
    pub fn durations(
        &self,
        from: i64,
        to: i64,
        realm: Option<&str>,
        mut visit: impl FnMut(Option<&str>, i64),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&DURATIONS)?;
        let mut rows = statement.query(params![from, to, realm])?;
        while let Some(row) = rows.next()? {
            visit(optional_text(row, 0)?, row.get(1)?);
        }
        Ok(())
    }

    /// The parks that overlap a window (ruling 7): one range per span
    /// class through the parks' partial index, the open parks by class 64
    /// with their end read as `now`, ordered by cut phase then run so a
    /// figure that counts runs per phase counts a change of run.
    pub fn window_parks(
        &self,
        from: i64,
        to: i64,
        now: i64,
        realm: Option<&str>,
        visit: impl FnMut(&WindowPark<'_>),
    ) -> Result<(), StoreError> {
        self.window_parks_with(from, to, now, realm, visit, |_| {})
    }

    /// [`Store::window_parks`] with the statement's VM steps handed to
    /// `stepped`, so the test that binds the ranges' cost measures the
    /// production statement itself.
    fn window_parks_with(
        &self,
        from: i64,
        to: i64,
        now: i64,
        realm: Option<&str>,
        mut visit: impl FnMut(&WindowPark<'_>),
        mut stepped: impl FnMut(i64),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&PARK_SQL)?;
        let bounds = park_bounds(from, to, now, realm);
        let mut rows = statement.query(params_from_iter(bounds))?;
        while let Some(row) = rows.next()? {
            visit(&window_park(row)?);
        }
        drop(rows);
        stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
        Ok(())
    }

    /// The VM steps [`Store::window_parks`]' one statement takes, for the
    /// test that holds the ranges to the window alone.
    #[cfg(test)]
    pub(crate) fn window_parks_steps(
        &self,
        from: i64,
        to: i64,
        now: i64,
        realm: Option<&str>,
    ) -> Result<i64, StoreError> {
        let mut steps = 0;
        self.window_parks_with(from, to, now, realm, |_| {}, |taken| steps = taken)?;
        Ok(steps)
    }
}

/// Whether any park row still waits for its dated facts: the partial index
/// of unwritten parks answers one seek, a read.
fn parks_unfinished(conn: &Connection) -> Result<bool, StoreError> {
    Ok(conn.query_row(&PARK_STALE, [], |row| row.get(0))?)
}

/// The park rows still waiting for their facts, in the partial index's own
/// predicate: the type's name has one home in the index's module ([`PARKED`]).
static PARK_STALE: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index WHERE type = '{PARKED}' AND facts_seq IS NULL)"
    )
});

/// `lane`'s word as `brokkr-core` spells it, the vocabulary's one home
/// (decision 0071 rulings 2 and 5), used by every predicate here.
fn lane_is(lane: Lane) -> String {
    format!("lane = '{}'", lane.as_str())
}

/// The rows outside `lane`: the behind rows a window and its figures read
/// whole instead of counting stale columns.
fn outside(lane: Lane) -> String {
    format!("lane <> '{}'", lane.as_str())
}

/// The lanes that are still open and read whole: a stopping run is a
/// running one with an accepted stop.
fn open_lanes() -> String {
    [Lane::Running, Lane::Parked, Lane::Stopping]
        .map(|lane| format!("'{}'", lane.as_str()))
        .join(", ")
}

/// The endings that completed their run, as brokkr-core's one reading of
/// the vocabulary states it (decision 0064 ruling 6): the preceding count
/// counts shipped runs, never every finished one.
fn shipped_endings() -> String {
    Ending::ALL
        .into_iter()
        .filter(|ending| ending.status() == Status::Completed)
        .map(|ending| format!("'{}'", ending.word()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every behind row's id, by the lane's index.
static BEHIND: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT run_id FROM run_summary WHERE {} ORDER BY run_id",
        lane_is(Lane::Behind)
    )
});

/// The preceding window's shipped count through the `(finished_ms)` index,
/// less a row read whole instead.
static COUNT_FINISHED: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT count(*) FROM run_summary
    WHERE finished_ms >= ?1 AND finished_ms < ?2 AND {behind}
      AND ending IN ({shipped}) AND (?3 IS NULL OR realm = ?3)",
        behind = outside(Lane::Behind),
        shipped = shipped_endings(),
    )
});

/// The median's one pass: each finished run's realm and span, a start that
/// does not parse reading as its end, the span floored at zero as
/// `analytics`' `median` floors it.
static DURATIONS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT realm,
        max(finished_ms - coalesce(started_ms, finished_ms), 0)
    FROM run_summary
    WHERE finished_ms >= ?1 AND finished_ms < ?2 AND {behind} AND (?3 IS NULL OR realm = ?3)",
        behind = outside(Lane::Behind)
    )
});

/// The runs a window reads: those that finished in it, less a behind row,
/// and those still open, through the lane index. A stopping run is a
/// running one with an accepted stop, still open, so its lane is read too.
/// Each row also states whether its fold stood at the policy's hard stop,
/// exactly as `brokkr-core`'s `hard_stop` decides it: `stopped`, at the
/// `stop` phase. The comparison is `IS`, not `=`, so a stopped run that
/// never entered a phase (its `phase` column NULL) reads false rather than
/// SQL NULL, which the row reader would refuse. The `phase` column is the
/// writer's cut at [`brokkr_core::summary::NAME_BYTES`], and a cut that
/// reads `stop` is the whole `stop` — a longer phase cannot cut to it — so
/// `phase_whole` need not be read.
fn runs_statement(order: &str) -> String {
    format!(
        "SELECT run_id, realm, recipe, ending_rule, started_ms, finished_ms, ending,
                implement_visits, review_first, review_second, review_back,
                (status = '{stopped}' AND phase IS 'stop') AS hard_stop, 0 AS open
           FROM run_summary
           WHERE finished_ms >= ?1 AND finished_ms < ?2 AND {behind}
             AND (?3 IS NULL OR realm = ?3)
         UNION ALL
         SELECT run_id, realm, recipe, ending_rule, started_ms, finished_ms, ending,
                implement_visits, review_first, review_second, review_back,
                (status = '{stopped}' AND phase IS 'stop') AS hard_stop, 1 AS open
           FROM run_summary
           WHERE lane IN ({open})
             AND (?3 IS NULL OR realm = ?3)
         {order}",
        stopped = Status::Stopped.as_str(),
        behind = outside(Lane::Behind),
        open = open_lanes(),
    )
}

static RUNS_UNORDERED: LazyLock<String> = LazyLock::new(|| runs_statement(""));
static RUNS_BY_ENDING: LazyLock<String> =
    LazyLock::new(|| runs_statement("ORDER BY ending_rule, run_id"));
static RUNS_BY_REALM: LazyLock<String> =
    LazyLock::new(|| runs_statement("ORDER BY realm, recipe, run_id"));

/// The statement an order folds, its keys' order and the run's id for a
/// stable tie.
fn runs_sql(order: RunOrder) -> &'static str {
    match order {
        RunOrder::Unordered => &RUNS_UNORDERED,
        RunOrder::ByEndingRule => &RUNS_BY_ENDING,
        RunOrder::ByRealmThenRecipe => &RUNS_BY_REALM,
    }
}

/// The bound values `runs_sql` takes: the window, and the realm key or
/// NULL for every realm.
fn run_bounds(from: i64, to: i64, realm: Option<&str>) -> [Value; 3] {
    [
        Value::Integer(from),
        Value::Integer(to),
        realm.map_or(Value::Null, |realm| Value::Text(realm.to_string())),
    ]
}

/// One run's borrowed row.
fn window_run<'a>(row: &'a Row<'_>) -> Result<WindowRun<'a>, rusqlite::Error> {
    Ok(WindowRun {
        run_id: required_text(row, 0)?,
        realm: optional_text(row, 1)?,
        recipe: optional_text(row, 2)?,
        ending_rule: optional_text(row, 3)?,
        started_ms: row.get(4)?,
        finished_ms: row.get(5)?,
        ending: ending_of(optional_text(row, 6)?)?,
        implement_visits: row.get::<_, i64>(7)? as u64,
        reviews: WindowReviews {
            first: row.get::<_, i64>(8)? != 0,
            second: row.get::<_, i64>(9)? != 0,
            back: row.get::<_, i64>(10)? != 0,
        },
        hard_stop: row.get::<_, i64>(11)? != 0,
        open: row.get::<_, i64>(12)? != 0,
    })
}

/// One park's borrowed row.
fn window_park<'a>(row: &'a Row<'_>) -> Result<WindowPark<'a>, rusqlite::Error> {
    Ok(WindowPark {
        run_id: required_text(row, 0)?,
        realm: optional_text(row, 1)?,
        phase: optional_text(row, 2)?,
        at: row.get(3)?,
        answered: row.get(4)?,
        ended: row.get(5)?,
        open: row.get::<_, i64>(6)? != 0,
    })
}

/// The stored ending's word read into decision 0064's twelve at the edge,
/// as the page's reader reads the same column: a word outside them is a
/// row no writer writes, refused rather than carried untyped across the
/// crate boundary.
fn ending_of(word: Option<&str>) -> Result<Option<Ending>, rusqlite::Error> {
    let Some(word) = word else {
        return Ok(None);
    };
    Ending::named(word).map(Some).map_err(|why| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(why))
    })
}

/// The text at `at`, which every row's key names; [`crate::metered`]'s
/// rows read it too.
pub(crate) fn required_text<'a>(row: &'a Row<'_>, at: usize) -> Result<&'a str, rusqlite::Error> {
    Ok(row.get_ref(at)?.as_str()?)
}

/// The text at `at`, NULL where the column holds none.
pub(crate) fn optional_text<'a>(
    row: &'a Row<'_>,
    at: usize,
) -> Result<Option<&'a str>, rusqlite::Error> {
    match row.get_ref(at)? {
        ValueRef::Null => Ok(None),
        value => Ok(Some(value.as_str()?)),
    }
}

/// The parks' one statement: every closed class's range of the partial
/// index, then the open parks, one `ORDER BY` over their rows.
static PARK_SQL: LazyLock<String> = LazyLock::new(park_statement);

/// The bound values [`park_statement`] takes: `from`, `to`, `now`, the
/// realm key or NULL, then each closed class's saturated start.
fn park_bounds(from: i64, to: i64, now: i64, realm: Option<&str>) -> Vec<Value> {
    let mut bounds = vec![
        Value::Integer(from),
        Value::Integer(to),
        Value::Integer(now),
        realm.map_or(Value::Null, |realm| Value::Text(realm.to_string())),
    ];
    bounds.extend((1u8..=63).map(|class| Value::Integer(class_start(from, class))));
    bounds
}

/// `from - 2^class`, saturated at [`i64::MIN`]: class 63's power of two is
/// past a signed integer, and the range is every park the window can see.
fn class_start(from: i64, class: u8) -> i64 {
    let reach = 1i128 << class;
    (from as i128 - reach).max(i64::MIN as i128) as i64
}

/// The parks' ranges, each arm seeking the partial index and the run
/// summary's key, then one sort by phase and run.
fn park_statement() -> String {
    let mut ranges: Vec<String> = (1u8..=63)
        .map(|class| {
            let start = 4 + usize::from(class);
            format!(
                "SELECT e.run_id, s.realm, e.phase, e.at_ms, e.answered_ms,
                        coalesce(e.ended_ms, ?3) AS ended_ms, 0 AS open, e.seq AS seq
                   FROM event_index AS e INDEXED BY event_index_park
                   JOIN run_summary AS s ON s.run_id = e.run_id
                   WHERE e.type = '{PARKED}' AND e.span_class = {class}
                     AND e.at_ms >= ?{start} AND e.at_ms < ?2
                     AND max(e.answered_ms, e.ended_ms) > ?1
                     AND (?4 IS NULL OR s.realm = ?4)"
            )
        })
        .collect();
    ranges.push(format!(
        "SELECT e.run_id, s.realm, e.phase, e.at_ms, e.answered_ms,
                coalesce(e.ended_ms, ?3) AS ended_ms, 1 AS open, e.seq AS seq
           FROM event_index AS e INDEXED BY event_index_park
           JOIN run_summary AS s ON s.run_id = e.run_id
           WHERE e.type = '{PARKED}' AND e.span_class = 64 AND e.at_ms < ?2
             AND (?4 IS NULL OR s.realm = ?4)"
    ));
    // One sort over the ranges' rows: the phase, the run, the park's
    // start, and its seq for a stable tie. The materialized CTE collects
    // the arms first, so SQLite sorts once rather than sorting every arm
    // beside its merge.
    format!(
        "WITH parks AS MATERIALIZED ({}) SELECT * FROM parks
         ORDER BY phase, run_id, at_ms, seq",
        ranges.join(" UNION ALL ")
    )
}

#[cfg(test)]
mod tests;
