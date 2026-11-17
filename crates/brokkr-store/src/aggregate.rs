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
//! Every window statement leaves out, in SQL, the runs the whole read
//! takes: the `exclusion` module holds that one fragment, and
//! `window_fallbacks` reads the same one, so a run is either the
//! fallback's or a statement's and never both. A statement learns the arm
//! by asking the stale-seat set's intact check once and preparing the
//! set's or the rule's text.
//!
//! `window_runs` and `window_parks` are called by the row path today.
//! `durations` and `finished_between` have no production caller until
//! slice 4's unit U5b2, the streamed row source; until it lands, the tests
//! alone call them. They are `pub` on [`Store`] so it can, and each is in
//! the public API snapshot until then.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use brokkr_core::ending::Ending;
use brokkr_core::fold::Status;
use brokkr_core::page::Lane;
use brokkr_core::policy::audit::STOP;
use rusqlite::types::{Value, ValueRef};
use rusqlite::Connection;
use rusqlite::{params, OptionalExtension, Row, StatementStatus};

use crate::event_index::{finished, index_intact, BACKFILLED as INDEXED, PARKED};
use crate::seat_summary::{stale_intact, SHAPE};
use crate::summary::{summary_intact, BACKFILLED as SUMMARISED};
use crate::{patiently, Store, StoreError};

pub(crate) mod exclusion;
pub mod pull;

/// The order a window's runs are folded in (decision 0089 ruling 7): the
/// two key orders the figures group by, or no order when the caller keeps
/// its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOrder {
    /// The index's own order, which no figure reads.
    Unordered,
    /// By the stored ending rule, then the run's id.
    ByEndingRule,
    /// By the stored realm (a NULL one read as the hearth's label where the
    /// read names one), then recipe (a NULL one as the empty name), then
    /// the run's id.
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

/// The partial index the readiness probe reads: one entry a `runs` row
/// whose `run_id`, `feature` or `created_at` is not text, which
/// [`Store::list_runs`] cannot read. `IF NOT EXISTS`, so a writable open
/// adds it to an older journal and `DATABASE_SCHEMA` does not move for it;
/// an older binary keeps it, since no repair drops an index on `runs`.
/// [`unreadable_intact`] compares the text, so an index another build's
/// definition made is remade rather than trusted.
pub(crate) const UNREADABLE_INDEX: &str = "runs_unreadable";

/// The rows [`Store::list_runs`] cannot read: one of its three columns is
/// not text. `typeof` answers for a NULL, an integer, a real and a blob
/// alike; `IS NULL` would see only the first.
const UNREADABLE: &str = "typeof(run_id) <> 'text'
     OR typeof(feature) <> 'text'
     OR typeof(created_at) <> 'text'";

/// The probe's statement: one index entry, or none. `LIMIT 1` holds it to
/// that one, so neither its heap nor its steps grow with the runs.
static RUNS_UNREADABLE: LazyLock<String> =
    LazyLock::new(|| format!("SELECT 1 FROM runs WHERE {UNREADABLE} LIMIT 1"));

/// The index's statement, `if_not_exists` for the repair's own `CREATE`,
/// false for the text SQLite stores and [`unreadable_intact`] reads back
/// (SQLite stores neither that clause nor a trailing semicolon). The
/// predicate is the probe's own, so SQLite answers the probe from the
/// index's entries alone.
fn unreadable_index_sql(if_not_exists: bool) -> String {
    let clause = if if_not_exists { "IF NOT EXISTS " } else { "" };
    format!("CREATE INDEX {clause}{UNREADABLE_INDEX} ON runs (run_id) WHERE {UNREADABLE}")
}

/// Remake the probe's index, inside the open's immediate transaction: an
/// index another build's text made is dropped, never trusted, so the
/// probe's bound survives a changed definition.
pub(crate) fn migrate_unreadable(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(&format!(
        "DROP INDEX IF EXISTS {UNREADABLE_INDEX};\n{}",
        unreadable_index_sql(true)
    ))?;
    Ok(())
}

/// Does the journal carry the probe's index with this build's definition?
/// A pure read of `sqlite_master`: a name alone would trust an index whose
/// text another build wrote, which the probe's predicate cannot read
/// through.
pub(crate) fn unreadable_intact(conn: &Connection) -> Result<bool, StoreError> {
    let index: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [UNREADABLE_INDEX],
            |row| row.get(0),
        )
        .optional()?;
    Ok(index.as_deref() == Some(unreadable_index_sql(false).as_str()))
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

    /// Whether [`Store::list_runs`] would read this journal: no `runs` row
    /// holds a `run_id`, `feature` or `created_at` that is not text. One
    /// entry of the partial index the open keeps, so the probe's heap and
    /// VM steps do not grow with the runs; a journal opened read-only
    /// without the index still answers, by scanning.
    ///
    /// The one shape it cannot see is a text value that is not valid UTF-8:
    /// SQLite stores it as text, so this says `true` where the listing
    /// refuses. That is damage behind the store's back — the ruled limit of
    /// analytics' reach (decision 0089 ruling 8's boundary; `doctor` and the
    /// journal integrity checks find it).
    pub fn runs_readable(&self) -> Result<bool, StoreError> {
        self.runs_readable_with(|_| {})
    }

    /// [`Store::runs_readable`] with the statement's VM steps handed to
    /// `stepped`, so the test that binds the probe measures the production
    /// statement itself.
    fn runs_readable_with(&self, mut stepped: impl FnMut(i64)) -> Result<bool, StoreError> {
        patiently("runs_readable", self.patience, || {
            let mut statement = self.conn.prepare(RUNS_UNREADABLE.as_str())?;
            let found = statement.query([])?.next()?.is_some();
            stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
            Ok(!found)
        })
    }

    /// The VM steps [`Store::runs_readable`]' one statement takes, for the
    /// test that holds the probe to the index.
    #[cfg(test)]
    pub(crate) fn runs_readable_steps(&self) -> Result<i64, StoreError> {
        let mut steps = 0;
        self.runs_readable_with(|taken| steps = taken)?;
        Ok(steps)
    }

    /// Every run whose summary row is behind its head, streamed by its id
    /// through the `behind` lane's index and its `runs` stamp: a run a
    /// writer did not fold, or one whose aggregate stamp is not its folded
    /// head's. The caller reads each one whole (ruling 8), and needs the
    /// stamp to place its realm among the runs it reads (U3c).
    pub fn behind_runs(&self, mut visit: impl FnMut(&str, &str)) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&BEHIND)?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            visit(required_text(row, 0)?, required_text(row, 1)?);
        }
        Ok(())
    }

    /// Every non-behind run's realm key, one realm once, in first-`created_at`
    /// order, through the covering index that answers `not_behind` (decision
    /// 0089 ruling A4, slice 4's unit U3c). A NULL realm is yielded as it
    /// stands: the caller names it the hearth's label. `realm` filters the
    /// stored keys and leaves the NULL rows for that naming, since the store
    /// does not know the label.
    pub fn realm_names(
        &self,
        realm: Option<&str>,
        visit: impl FnMut(&str, &str, Option<&str>),
    ) -> Result<(), StoreError> {
        self.realm_names_with(realm, visit, |_| {})
    }

    /// [`Store::realm_names`] with the statement's VM steps handed to
    /// `stepped`, so the test that binds the index-only scan measures the
    /// production statement itself.
    fn realm_names_with(
        &self,
        realm: Option<&str>,
        mut visit: impl FnMut(&str, &str, Option<&str>),
        mut stepped: impl FnMut(i64),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&REALM_NAMES)?;
        let mut rows = statement.query(params![realm])?;
        let mut named: BTreeSet<Option<String>> = BTreeSet::new();
        while let Some(row) = rows.next()? {
            let realm = optional_text(row, 2)?;
            if named.insert(realm.map(str::to_string)) {
                visit(required_text(row, 0)?, required_text(row, 1)?, realm);
            }
        }
        drop(rows);
        stepped(i64::from(statement.get_status(StatementStatus::VmStep)));
        Ok(())
    }

    /// The VM steps [`Store::realm_names`]' one statement takes, for the
    /// test that holds it to the index alone.
    #[cfg(test)]
    pub(crate) fn realm_names_steps(&self, realm: Option<&str>) -> Result<i64, StoreError> {
        let mut steps = 0;
        self.realm_names_with(realm, |_, _, _| {}, |taken| steps = taken)?;
        Ok(steps)
    }

    /// Every non-behind run in the refused or acknowledged lane, counted by
    /// its realm key through the same covering index (decision 0089 ruling
    /// A4, slice 4's unit U3c): the fold refused it and no figure reads it.
    /// A NULL realm is yielded as it stands; the caller names it the
    /// hearth's label and filters by its own realm.
    pub fn refused_counts(
        &self,
        realm: Option<&str>,
        mut visit: impl FnMut(Option<&str>, u64),
    ) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(&REFUSED_COUNTS)?;
        let mut rows = statement.query(params![realm])?;
        let mut counts: BTreeMap<Option<String>, u64> = BTreeMap::new();
        while let Some(row) = rows.next()? {
            *counts
                .entry(optional_text(row, 0)?.map(str::to_string))
                .or_default() += 1;
        }
        drop(rows);
        for (realm, count) in counts {
            visit(realm.as_deref(), count);
        }
        Ok(())
    }

    /// The runs a window reads (ruling 7): those that finished inside it,
    /// through the `(finished_ms)` index, and those still open, through
    /// the lane index; a run the whole read takes is left out in SQL, so a
    /// caller streams without a Rust set of run ids. `order` is the
    /// figure's key order, `realm` narrows to one stored realm key where
    /// given.
    pub fn window_runs(
        &self,
        from: i64,
        to: i64,
        order: RunOrder,
        realm: Option<&str>,
        mut visit: impl FnMut(&WindowRun<'_>),
    ) -> Result<(), StoreError> {
        let bounds = pull::Bounds {
            from,
            to,
            realm,
            label: None,
        };
        self.pull_runs(&bounds, order, |runs| {
            while let Some(run) = runs.next_run()? {
                visit(&run);
            }
            Ok(())
        })?
    }

    /// The shipped runs that finished in the equal window before the
    /// headline's (ruling 7), streamed one stored realm key at a time:
    /// each group is `(realm, count)`, so the caller names a NULL realm
    /// after its label and filters by its own realm scope as it does every
    /// other row. The headline's `previous` sums the groups, which
    /// counts the runs that completed, as `brokkr-view`'s `shipped_in`
    /// does, through the `(finished_ms)` index.
    ///
    /// Its caller is U5b2, the streamed row source; until that lands, the
    /// tests alone call it.
    pub fn finished_between(
        &self,
        from: i64,
        to: i64,
        mut visit: impl FnMut(Option<&str>, u64),
    ) -> Result<(), StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(finished_sql(intact))?;
        let mut rows = statement.query(params![from, to, SHAPE.as_str()])?;
        while let Some(row) = rows.next()? {
            visit(optional_text(row, 0)?, row.get::<_, i64>(1)? as u64);
        }
        Ok(())
    }

    /// One pass of the median's selection: every finished run's stored
    /// realm key and its span in the window, through the `(finished_ms)`
    /// index, streamed and holding nothing between passes. A run the whole
    /// read takes is left out in SQL. Its caller is U5b2, the streamed row
    /// source; until that lands, the tests alone call it.
    pub fn durations(
        &self,
        from: i64,
        to: i64,
        mut visit: impl FnMut(Option<&str>, i64),
    ) -> Result<(), StoreError> {
        let intact = stale_intact(&self.conn)?;
        let mut statement = self.conn.prepare(durations_sql(intact))?;
        let mut rows = statement.query(params![from, to, SHAPE.as_str()])?;
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
        stepped: impl FnOnce(i64),
    ) -> Result<(), StoreError> {
        let window = pull::ParkWindow {
            from,
            to,
            now,
            realm,
        };
        self.park_rows(
            &window,
            |parks| {
                while let Some(park) = parks.next_park()? {
                    visit(&park);
                }
                Ok(())
            },
            stepped,
        )?
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

/// The rows a window never reads: the behind rows it reads whole instead of
/// counting stale columns, and the refused or acknowledged rows whose fold
/// states no figure ([`Lane::Refused`] and [`Lane::Acknowledged`]).
/// `prefix` names the row where a statement aliases it.
pub(crate) fn outside(prefix: &str) -> String {
    [Lane::Behind, Lane::Refused, Lane::Acknowledged]
        .map(|lane| format!("{prefix}lane <> '{}'", lane.as_str()))
        .join(" AND ")
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

/// Every behind row's id and `runs` stamp, by the lane's index.
static BEHIND: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT run_summary.run_id, runs.created_at
           FROM run_summary JOIN runs ON runs.run_id = run_summary.run_id
          WHERE {}
          ORDER BY run_summary.run_id",
        lane_is(Lane::Behind)
    )
});

/// Every non-behind row's order and realm, each realm once, in
/// `(created_at, run_id)` order: the covering index's own order, and the
/// tie the whole read states too (U3c's ruled tie).
static REALM_NAMES: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT created_at, run_id, realm FROM run_summary
          WHERE {not_behind} AND (?1 IS NULL OR realm = ?1 OR realm IS NULL)
          ORDER BY created_at, run_id",
        not_behind = crate::summary::not_behind("")
    )
});

/// The refused and acknowledged rows' realms: both lanes are a row whose
/// fold wrote refusal words, and only a behind row can hide one from the
/// count, so [`not_behind`](crate::summary::not_behind) is the whole lane
/// test on the covering index.
static REFUSED_COUNTS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT realm FROM run_summary
          WHERE {not_behind} AND refusal_words IS NOT NULL
            AND (?1 IS NULL OR realm = ?1 OR realm IS NULL)",
        not_behind = crate::summary::not_behind("")
    )
});

/// The preceding window's shipped count, one row a stored realm key,
/// through the `(finished_ms)` index and leaving out the runs the whole
/// read takes. `intact` names the arm the open chose.
static FINISHED: LazyLock<[String; 2]> =
    LazyLock::new(|| [finished_statement(true), finished_statement(false)]);

fn finished_statement(intact: bool) -> String {
    format!(
        "WITH {whole}
    SELECT r.realm, count(*) FROM run_summary AS r
    WHERE r.finished_ms >= ?1 AND r.finished_ms < ?2 AND {behind}
      AND r.ending IN ({shipped}) AND {not_whole}
    GROUP BY r.realm",
        whole = exclusion::clause(intact, "?3"),
        behind = outside("r."),
        shipped = shipped_endings(),
        not_whole = exclusion::not_whole("r.run_id"),
    )
}

/// The statement [`Store::finished_between`] prepares for the arm the
/// open chose.
fn finished_sql(intact: bool) -> &'static str {
    FINISHED[usize::from(!intact)].as_str()
}

/// The median's one pass, one statement an arm: each finished run's realm
/// and span, a start that does not parse reading as its end, the span
/// floored at zero as `analytics`' `median` floors it, and the runs the
/// whole read takes left out.
static DURATIONS: LazyLock<[String; 2]> =
    LazyLock::new(|| [durations_statement(true), durations_statement(false)]);

fn durations_statement(intact: bool) -> String {
    format!(
        "WITH {whole}
    SELECT r.realm, max(r.finished_ms - coalesce(r.started_ms, r.finished_ms), 0)
    FROM run_summary AS r
    WHERE r.finished_ms >= ?1 AND r.finished_ms < ?2 AND {behind} AND {not_whole}",
        whole = exclusion::clause(intact, "?3"),
        behind = outside("r."),
        not_whole = exclusion::not_whole("r.run_id"),
    )
}

/// The statement [`Store::durations`] prepares for the arm the open chose.
fn durations_sql(intact: bool) -> &'static str {
    DURATIONS[usize::from(!intact)].as_str()
}

/// The runs a window reads: those that finished in it, less a behind,
/// refused or acknowledged row, and those still open, through the lane
/// index. A stopping run is a running one with an accepted stop, still
/// open, so its lane is read too.
/// Each row also states whether its fold stood at the policy's hard stop,
/// exactly as `brokkr-core`'s `hard_stop` decides it: `stopped`, at
/// `brokkr_core::policy::audit::STOP`, the one name the core gives the
/// phase. The comparison is `IS`, not `=`, so a stopped run that never
/// entered a phase (its `phase` column NULL) reads false rather than SQL
/// NULL, which the row reader would refuse. The `phase` column is the
/// writer's cut at [`brokkr_core::summary::NAME_BYTES`], and a cut that
/// reads `stop` is the whole `stop` — a longer phase cannot cut to it — so
/// `phase_whole` need not be read.
///
/// The two last columns are the realm and recipe keys a figure groups by
/// ([`pull::Bounds`]): a NULL realm read as the hearth's label `?5`, and a
/// NULL recipe as the empty name, so the order is the figure's own.
fn runs_statement(order: &str, intact: bool) -> String {
    format!(
        "WITH {whole}
         SELECT r.run_id, r.realm, r.recipe, r.ending_rule, r.started_ms, r.finished_ms,
                r.ending, r.implement_visits, r.review_first, r.review_second, r.review_back,
                (r.status = '{stopped}' AND r.phase IS '{stop}') AS hard_stop, 0 AS open,
                coalesce(r.realm, ?5) AS realm_key, coalesce(r.recipe, '') AS recipe_key
           FROM run_summary AS r
           WHERE r.finished_ms >= ?1 AND r.finished_ms < ?2 AND {behind} AND {not_whole}
             AND (?3 IS NULL OR r.realm = ?3)
         UNION ALL
         SELECT r.run_id, r.realm, r.recipe, r.ending_rule, r.started_ms, r.finished_ms,
                r.ending, r.implement_visits, r.review_first, r.review_second, r.review_back,
                (r.status = '{stopped}' AND r.phase IS '{stop}') AS hard_stop, 1 AS open,
                coalesce(r.realm, ?5) AS realm_key, coalesce(r.recipe, '') AS recipe_key
           FROM run_summary AS r
           WHERE r.lane IN ({open}) AND {not_whole}
             AND (?3 IS NULL OR r.realm = ?3)
         {order}",
        whole = exclusion::clause(intact, "?4"),
        stopped = Status::Stopped.as_str(),
        stop = STOP,
        behind = outside("r."),
        open = open_lanes(),
        not_whole = exclusion::not_whole("r.run_id"),
    )
}

/// The three orders' statements, for the set's arm and the rule's.
static RUNS: LazyLock<[[String; 2]; 3]> = LazyLock::new(|| {
    let each = |order: &str| [runs_statement(order, true), runs_statement(order, false)];
    [
        each(""),
        each("ORDER BY ending_rule, run_id"),
        each("ORDER BY realm_key, recipe_key, run_id"),
    ]
});

/// The statement an order folds on the arm the open chose: the order's
/// keys and the run's id for a stable tie.
fn runs_sql(order: RunOrder, intact: bool) -> &'static str {
    let at = match order {
        RunOrder::Unordered => 0,
        RunOrder::ByEndingRule => 1,
        RunOrder::ByRealmThenRecipe => 2,
    };
    RUNS[at][usize::from(!intact)].as_str()
}

/// The four values every window read binds: the window, the realm key or
/// NULL for every realm, and this build's seat shape.
pub(crate) fn bounds(from: i64, to: i64, realm: Option<&str>) -> [Value; 4] {
    [
        Value::Integer(from),
        Value::Integer(to),
        realm.map_or(Value::Null, |realm| Value::Text(realm.to_string())),
        Value::Text(SHAPE.to_string()),
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

/// The parks' statements, one per arm: every closed class's range of the
/// partial index, then the open parks, one `ORDER BY` over their rows.
static PARKS: LazyLock<[String; 2]> =
    LazyLock::new(|| [park_statement(true), park_statement(false)]);

/// The statement [`Store::window_parks`] prepares for the arm the open
/// chose.
fn park_sql(intact: bool) -> &'static str {
    PARKS[usize::from(!intact)].as_str()
}

/// The bound values [`park_statement`] takes: `from`, `to`, `now`, the
/// realm key or NULL, each closed class's saturated start, and this
/// build's seat shape.
fn park_bounds(from: i64, to: i64, now: i64, realm: Option<&str>) -> Vec<Value> {
    let mut bounds = vec![
        Value::Integer(from),
        Value::Integer(to),
        Value::Integer(now),
        realm.map_or(Value::Null, |realm| Value::Text(realm.to_string())),
    ];
    bounds.extend((1u8..=63).map(|class| Value::Integer(class_start(from, class))));
    bounds.push(Value::Text(SHAPE.to_string()));
    bounds
}

/// `from - 2^class`, saturated at [`i64::MIN`]: class 63's power of two is
/// past a signed integer, and the range is every park the window can see.
fn class_start(from: i64, class: u8) -> i64 {
    let reach = 1i128 << class;
    (from as i128 - reach).max(i64::MIN as i128) as i64
}

/// The closed span classes as one table, `(class, start)`: the parks'
/// statement iterates it outermost and seeks the partial index once a
/// class, so the exclusion is written once rather than copied into
/// sixty-three arms (ruling 9's aggregate exception, as amended on
/// 2026-11-13). Class 64, the open parks, stays its own arm because its
/// only bound is the window's end.
fn park_classes() -> String {
    let rows: Vec<String> = (1u8..=63)
        .map(|class| format!("({class}, ?{})", 4 + usize::from(class)))
        .collect();
    format!("classes(class, start) AS (VALUES {})", rows.join(", "))
}

/// The parks' ranges: the classes' table sought outermost, each class a
/// key probe of the partial index and the run summary's key, then the open
/// parks, then one sort by phase and run. The run the whole read takes is
/// left out by [`exclusion`]'s one fragment; the shape it binds is `?68`,
/// after the sixty-three class starts.
fn park_statement(intact: bool) -> String {
    let unread = outside("s.");
    let not_whole = exclusion::not_whole("s.run_id");
    format!(
        "WITH {whole}, {classes}, parks AS MATERIALIZED (
            SELECT e.run_id, s.realm, e.phase, e.at_ms, e.answered_ms,
                    coalesce(e.ended_ms, ?3) AS ended_ms, 0 AS open, e.seq AS seq
               FROM classes AS c
               CROSS JOIN event_index AS e INDEXED BY event_index_park
               JOIN run_summary AS s ON s.run_id = e.run_id
              WHERE e.type = '{PARKED}' AND e.span_class = c.class
                AND e.at_ms >= c.start AND e.at_ms < ?2
                AND max(e.answered_ms, e.ended_ms) > ?1
                AND (?4 IS NULL OR s.realm = ?4) AND {unread} AND {not_whole}
            UNION ALL
            SELECT e.run_id, s.realm, e.phase, e.at_ms, e.answered_ms,
                    coalesce(e.ended_ms, ?3) AS ended_ms, 1 AS open, e.seq AS seq
              FROM event_index AS e INDEXED BY event_index_park
              JOIN run_summary AS s ON s.run_id = e.run_id
             WHERE e.type = '{PARKED}' AND e.span_class = 64 AND e.at_ms < ?2
               AND (?4 IS NULL OR s.realm = ?4) AND {unread} AND {not_whole}
         ) SELECT * FROM parks ORDER BY phase, run_id, at_ms, seq",
        whole = exclusion::clause(intact, "?68"),
        classes = park_classes(),
    )
}

#[cfg(test)]
mod steps;

#[cfg(test)]
mod tests;
