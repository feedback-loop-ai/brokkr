//! The store's rows as the analytics readout's facts (decision 0089
//! ruling 7, slice 4's unit U4): each hearth read over its own journal,
//! one row at a time, in the order its figure folds.
//!
//! A migrated, ready hearth's figures come from [`RowOnly`]; a hearth whose
//! rows are not ready, and a run whose row is behind its head or whose seat
//! facts are stale, come from [`brokkr_view::WholeRead`] instead (ruling
//! 8). The two are merged in a figure's own key order, so a group never
//! straddles the row stream and the runs read whole. A rows-read hearth
//! takes its realm list and unreadable-run counts from the store's covering
//! index, and every other keeps the whole read's (decision 0089 ruling A4,
//! slice 4's unit U3c).
//!
//! This module reads a migrated journal only; `brokkr-store`'s window
//! reads are its one source of rows, and nothing here writes (decision
//! 0014). One hearth's reads run under one snapshot, so the store's lanes
//! and the whole read of each lagging run agree run for run, and no set of
//! run ids is held to keep them apart (slice 4's unit U5b2a). The rows are
//! pulled as each figure folds them, so the snapshot is held from the
//! hearth's first read until the readout ends, and nothing of the window
//! is collected (slice 4's unit U5b2b1).
//!
//! R1's limit (decision 0089 ruling 8's boundary, slice 4's unit U3c2): a
//! migrated, ready hearth is not loaded run by run, and "unreadable" is
//! what the store records — the `refused` and `acknowledged` lanes, and a
//! behind run whose one-run opening refuses, which sends the hearth whole
//! (U4). A run the store leaves whole whose opening succeeds and whose fold
//! refuses is counted under the realm its events name, exactly as the whole
//! read counts it. A chain broken behind the store's back after its row
//! stood at its head is out of this path's reach: the row path trusts the
//! row, and the journal and integrity checks find it (`doctor`, #1000).

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::BTreeMap;

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, RunState};
use brokkr_core::summary::{aggregate, cut, NAME_BYTES};
use brokkr_store::snapshot::ReadSnapshot;
use brokkr_store::Store;
use brokkr_view::{
    FactSource, FactUnion, ParkFact, Pulled, Query, RunFacts, RunOrder, SiteFact, SiteOrder,
    SpendFact, WholeRead, Window,
};

mod row_only;

use row_only::{Hearth, RowOnly};

/// One run the store leaves whole, and where its realm first stands: the
/// `runs` stamp as the whole read holds it, placed by the one key both
/// reads share (decision 0089 ruling A4's tie, `(created_at, run_id)`).
struct BehindRealm<'a> {
    created_at: &'a str,
    run_id: &'a str,
    realm: String,
}

/// One run the store leaves whole, read whole as the store names it (ruling
/// 8): its opening's events and the fold they state, and for a run of the
/// `behind` lane the `runs` stamp the store streams with its name. These are
/// the whole read's own runs, the only per-run data the row path holds.
struct Held {
    run_id: String,
    behind: Option<String>,
    events: Vec<EventEnvelope>,
    state: Option<RunState>,
}

/// Where in a hearth's read a seam stands (decision 0071 ruling 2's `*_with`
/// closure seam): after the `behind` lane is read whole, after the
/// fallbacks are, and after the row source is built, before any figure
/// pulls a window statement's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    Behind,
    Fallbacks,
    Rows,
}

/// The runs the store leaves whole, each opened as the store's statement
/// streams its name, so no set of their ids stands beside them. A run that
/// will not open ends the hearth's row read: the first refusal is kept and
/// the rest of the stream is left unread.
struct Holding<'a> {
    store: &'a Store,
    held: Vec<Held>,
    failed: Option<brokkr_store::StoreError>,
}

impl Holding<'_> {
    /// Read `run_id` whole, with its `behind` stamp when the lane names one.
    fn keep(&mut self, run_id: &str, behind: Option<&str>) {
        if self.failed.is_some() {
            return;
        }
        match self.store.open_kept(run_id) {
            Ok((_opened, events)) => {
                let state = fold(&events).ok();
                self.held.push(Held {
                    run_id: run_id.to_string(),
                    behind: behind.map(str::to_string),
                    events,
                    state,
                });
            }
            Err(error) => self.failed = Some(error),
        }
    }
}

/// One rows-read hearth: the facts its figures fold, the realm list and
/// unreadable counts its rows state (decision 0089 ruling A4, slice 4's
/// unit U3c), and the hearth's snapshot, held while its figures pull its
/// rows: from the hearth's first read until the source is dropped, when
/// the readout ends (U5b2b1). Its fields drop in order, the facts before
/// the snapshot.
pub(super) struct RowSource<'a> {
    facts: FactUnion<'a>,
    scope: Scope,
    _snapshot: ReadSnapshot<'a>,
}

/// A hearth's realm names and unreadable counts before their source joins
/// the fleet: the whole read's naming rules — the query's realm filter, the
/// cut and one name once — applied to the store's keys (decision 0089
/// ruling 7).
#[derive(Default)]
struct Scope {
    realms: Vec<String>,
    unreadable: BTreeMap<String, usize>,
}

impl Scope {
    /// Name `named` once, cut where a row groups it, when the query keeps it.
    fn name(&mut self, wanted: Option<&str>, named: &str) {
        if !brokkr_view::realm_in_scope(wanted, named) {
            return;
        }
        let named = cut(named, NAME_BYTES);
        if !self.realms.iter().any(|realm| realm == named) {
            self.realms.push(named.to_string());
        }
    }

    /// Count `count` unreadable runs named `named`, by the cut key.
    fn unreadable(&mut self, wanted: Option<&str>, named: &str, count: u64) {
        if !brokkr_view::realm_in_scope(wanted, named) {
            return;
        }
        *self
            .unreadable
            .entry(cut(named, NAME_BYTES).to_string())
            .or_default() += count as usize;
    }
}

/// One rows-read hearth's request: the store, its label and configured
/// realms, the window a figure folds, the query's realm and clock, the
/// range its whole fallback states, and where a statement that refuses
/// while a figure pulls it marks the hearth refused (decision 0071 ruling
/// 10's parameter object).
pub(super) struct Rows<'a> {
    pub(super) store: &'a Store,
    pub(super) label: &'a str,
    pub(super) configured: &'a [String],
    pub(super) window: Window,
    pub(super) realm: Option<&'a str>,
    pub(super) range: brokkr_view::Range,
    pub(super) now: &'a str,
    pub(super) refused: &'a Cell<bool>,
}

/// The runs a store leaves whole, in the whole read's own order, each read
/// from its own key and never the journal: slice 2's one-run opening
/// (decision 0089 ruling 8). `behind_runs` and `window_fallbacks` name them
/// one at a time and name no run twice, since the fallback leaves the
/// behind lane to the first. A run that will not load refuses this whole
/// fallback, so the caller reads its hearth whole instead of silently
/// leaving a figure empty.
fn hold_whole(
    rows: &Rows<'_>,
    between: &mut impl FnMut(Stage),
) -> Result<Vec<Held>, brokkr_store::StoreError> {
    let mut holding = Holding {
        store: rows.store,
        held: Vec::new(),
        failed: None,
    };
    rows.store.behind_runs(|run_id, created_at| {
        holding.keep(run_id, Some(created_at));
    })?;
    between(Stage::Behind);
    rows.store
        .window_fallbacks(rows.window.from, rows.window.to, None, |fallback| {
            holding.keep(fallback.run_id, None);
        })?;
    between(Stage::Fallbacks);
    if let Some(error) = holding.failed {
        return Err(error);
    }
    // The whole read folds its runs in id order, as it always has.
    holding
        .held
        .sort_by(|left, right| left.run_id.cmp(&right.run_id));
    Ok(holding.held)
}

/// The whole read of `held`, and each behind run's realm and stamp with it,
/// so the hearth's realm list can place it where the whole read does (U3c).
fn read_whole<'a>(rows: &Rows<'_>, held: &'a [Held]) -> (WholeRead, Vec<BehindRealm<'a>>) {
    let behind = held
        .iter()
        .filter_map(|run| {
            let created_at = run.behind.as_deref()?;
            let realm = aggregate(&run.events, run.state.as_ref()).realm;
            Some(BehindRealm {
                created_at,
                run_id: &run.run_id,
                realm: realm.unwrap_or_else(|| rows.label.to_string()),
            })
        })
        .collect();
    let runs: Vec<brokkr_view::AnalyticsRun<'_>> = held
        .iter()
        .map(|run| brokkr_view::AnalyticsRun {
            events: &run.events,
            state: run.state.as_ref(),
        })
        .collect();
    let runs: &[brokkr_view::AnalyticsRun<'_>] = &runs;
    let hearth = brokkr_view::AnalyticsHearth {
        label: rows.label,
        realms: &[],
        journal: "",
        runs,
        refusal: None,
    };
    let query = Query {
        hearths: std::slice::from_ref(&hearth),
        range: rows.range,
        realm: rows.realm,
        now: rows.now,
    };
    (brokkr_view::whole_read(&query, rows.window), behind)
}

/// One rows-read hearth's realm list and unreadable counts: configured
/// realms or the label first, then every run's realm in first-`created_at`
/// order through the store's covering index, the behind runs' realms placed
/// by their one-run stamp, the refused and acknowledged counts, and the
/// runs the one-run fallback opened and could not fold (decision 0089
/// ruling A4, slice 4's units U3c and U3c2).
fn store_scope(
    rows: &Rows<'_>,
    behind: &[BehindRealm<'_>],
    unfolded: &BTreeMap<String, usize>,
) -> Result<Scope, brokkr_store::StoreError> {
    let mut scope = Scope::default();
    if rows.configured.is_empty() {
        scope.name(rows.realm, rows.label);
    } else {
        for named in rows.configured {
            scope.name(rows.realm, named);
        }
    }
    // A realm is cut once, where the store keeps it (U3a's M2): the query's
    // name is cut before it names a stored key.
    let wanted = rows.realm.map(|realm| cut(realm, NAME_BYTES));
    // The store's names are one realm once; a behind run's are borrowed from
    // the run the hearth already holds, so no run's id is copied again.
    let mut places: Vec<(Cow<'_, str>, Cow<'_, str>, Cow<'_, str>)> = Vec::new();
    rows.store
        .realm_names(wanted, |created_at, run_id, named| {
            places.push((
                Cow::Owned(created_at.to_string()),
                Cow::Owned(run_id.to_string()),
                Cow::Owned(named.unwrap_or(rows.label).to_string()),
            ));
        })?;
    places.extend(behind.iter().map(|behind| {
        (
            Cow::Borrowed(behind.created_at),
            Cow::Borrowed(behind.run_id),
            Cow::Borrowed(behind.realm.as_str()),
        )
    }));
    places.sort_by(|left, right| {
        super::place(&left.0, &left.1).cmp(&super::place(&right.0, &right.1))
    });
    for (_, _, named) in places {
        scope.name(rows.realm, &named);
    }
    rows.store.refused_counts(wanted, |named, count| {
        scope.unreadable(rows.realm, named.unwrap_or(rows.label), count);
    })?;
    // A run the store leaves whole whose one-run opening succeeds and whose
    // fold refuses is unreadable exactly as the whole read counts it: the
    // fallback's count, under the realm its events name (U3c2).
    for (named, count) in unfolded {
        scope.unreadable(rows.realm, named, *count as u64);
    }
    Ok(scope)
}

/// One migrated hearth's facts and scope, `between` run at each [`Stage`],
/// the instant a writer's commit lands, which the tests stage: the rows,
/// merged with the runs the store leaves whole (decision 0089 ruling 8),
/// and the store's own realm list and counts for the hearth that reads
/// them (U3c).
///
/// Every read of the hearth runs under one snapshot ([`Store::snapshot`],
/// U5b2a): the store's lanes, the window statements and the whole read of
/// each lagging run agree run for run, whatever a writer commits between
/// them, so a run is read whole or from its row and no id set is held to
/// keep the two apart. The window statements run as each figure pulls
/// them, so the source holds the snapshot from before its first read until
/// it is dropped, when the readout ends (U5b2b1); a WAL checkpoint waits
/// behind it that long.
pub(super) fn read_rows_with<'a>(
    rows: &Rows<'a>,
    mut between: impl FnMut(Stage),
) -> Result<RowSource<'a>, brokkr_store::StoreError> {
    let snapshot = rows.store.snapshot()?;
    let held = hold_whole(rows, &mut between)?;
    let row = RowOnly::new(Hearth {
        store: rows.store,
        label: rows.label,
        window: rows.window,
        realm: rows.realm,
        refused: rows.refused,
    });
    between(Stage::Rows);
    let (facts, scope) = match held.is_empty() {
        // Nothing stands behind its row: the rows alone, still one source
        // of a union, as every hearth's.
        true => (
            FactUnion::new(vec![Box::new(row)]),
            store_scope(rows, &[], &BTreeMap::new())?,
        ),
        false => {
            let (fallback, behind) = read_whole(rows, &held);
            let scope = store_scope(rows, &behind, fallback.scope().1)?;
            (
                FactUnion::new(vec![Box::new(row), Box::new(fallback)]),
                scope,
            )
        }
    };
    Ok(RowSource {
        facts,
        scope,
        _snapshot: snapshot,
    })
}

/// One hearth's facts as the readout folds them: the whole read for a
/// hearth that is not migrated, not ready or not openable, and the store's
/// rows for a migrated, ready one (decision 0089 ruling 8).
pub(super) enum HearthSource<'a> {
    Whole(WholeRead),
    Rows(RowSource<'a>),
}

impl<'a> HearthSource<'a> {
    /// One hearth read whole: the fleet's runs folded as today, and every
    /// figure, realm name and unreadable count the whole read's (decision
    /// 0089 rulings 7 and 8). R1's limit belongs to the row path alone:
    /// see [`HearthSource::from_store`].
    pub(super) fn read(
        grouped: &brokkr_view::AnalyticsHearth<'_>,
        range: brokkr_view::Range,
        realm: Option<&str>,
        now: &str,
        window: Window,
    ) -> HearthSource<'a> {
        HearthSource::Whole(whole_of(grouped, range, realm, now, window))
    }

    /// One migrated, ready hearth's rows alone, with no fleet read behind
    /// it: the caller has already opened the store read-only and found its
    /// aggregates ready and its listing readable (decision 0089 ruling 8's
    /// boundary, slice 4's unit U3c2). A refusal is the caller's to answer
    /// with the whole read: a behind run's one-run opening refuses here, and
    /// a window statement that refuses as a figure pulls it marks
    /// `rows.refused`.
    ///
    /// R1's limit, as the module doc states it: this path trusts the row.
    /// A chain broken behind the store's back after its row stood at its
    /// head is out of its reach, and the journal and integrity checks find
    /// it (`doctor`, #1000).
    pub(super) fn from_store(
        rows: &Rows<'a>,
    ) -> Result<HearthSource<'a>, brokkr_store::StoreError> {
        Ok(HearthSource::Rows(read_rows_with(rows, |_| {})?))
    }
}

/// The whole read of one hearth, for the caller that holds it whole.
fn whole_of(
    grouped: &brokkr_view::AnalyticsHearth<'_>,
    range: brokkr_view::Range,
    realm: Option<&str>,
    now: &str,
    window: Window,
) -> WholeRead {
    let query = Query {
        hearths: std::slice::from_ref(grouped),
        range,
        realm,
        now,
    };
    brokkr_view::whole_read(&query, window)
}

impl FactSource for HearthSource<'_> {
    fn scope(&self) -> (&[String], &BTreeMap<String, usize>) {
        match self {
            HearthSource::Whole(whole) => whole.scope(),
            HearthSource::Rows(rows) => (&rows.scope.realms, &rows.scope.unreadable),
        }
    }

    fn quarantined(&self) -> Vec<brokkr_view::QuarantinedHearth> {
        match self {
            HearthSource::Whole(whole) => whole.quarantined(),
            HearthSource::Rows(rows) => rows.facts.quarantined(),
        }
    }

    fn runs(&self, visit: &mut dyn FnMut(RunFacts)) {
        match self {
            HearthSource::Whole(whole) => whole.runs(visit),
            HearthSource::Rows(rows) => rows.facts.runs(visit),
        }
    }

    fn runs_in(&mut self, order: RunOrder) -> Pulled<'_, RunFacts> {
        match self {
            HearthSource::Whole(whole) => whole.runs_in(order),
            HearthSource::Rows(rows) => rows.facts.runs_in(order),
        }
    }

    fn durations(&self, visit: &mut dyn FnMut(&str, i64)) {
        match self {
            HearthSource::Whole(whole) => whole.durations(visit),
            HearthSource::Rows(rows) => rows.facts.durations(visit),
        }
    }

    fn shipped_between(&self, window: Window) -> usize {
        match self {
            HearthSource::Whole(whole) => whole.shipped_between(window),
            HearthSource::Rows(rows) => rows.facts.shipped_between(window),
        }
    }

    fn parks(&mut self) -> Pulled<'_, ParkFact> {
        match self {
            HearthSource::Whole(whole) => whole.parks(),
            HearthSource::Rows(rows) => rows.facts.parks(),
        }
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact)) {
        match self {
            HearthSource::Whole(whole) => whole.spends(visit),
            HearthSource::Rows(rows) => rows.facts.spends(visit),
        }
    }

    fn sites_in(&mut self, order: SiteOrder) -> Pulled<'_, SiteFact> {
        match self {
            HearthSource::Whole(whole) => whole.sites_in(order),
            HearthSource::Rows(rows) => rows.facts.sites_in(order),
        }
    }
}
