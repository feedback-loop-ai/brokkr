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
//! 0014).
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

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::fold::fold;
use brokkr_core::summary::{aggregate, cut, Finished, HardStop, NAME_BYTES};
use brokkr_store::aggregate::{RunOrder as RowRunOrder, WindowRun};
use brokkr_store::metered::{SiteOrder as RowSiteOrder, WindowSite};
use brokkr_store::Store;
use brokkr_view::{
    FactSource, FactUnion, ParkFact, Query, Reviews, RunFacts, RunOrder, SiteFact, SiteOrder,
    SpendFact, WholeRead, Window,
};

/// One hearth's rows, read once: the facts every figure folds, already in
/// the order it asks for, so a figure holds one group at a time.
pub(super) struct RowOnly {
    window: Window,
    runs: Vec<RunFacts>,
    by_rule: Vec<RunFacts>,
    by_recipe: Vec<RunFacts>,
    parks: Vec<ParkFact>,
    spends: Vec<SpendFact>,
    model: Vec<SiteFact>,
    provider: Vec<SiteFact>,
    realm_provider: Vec<SiteFact>,
    recipe_provider: Vec<SiteFact>,
}

impl RowOnly {
    /// Read `store`'s window and the equal window before it: the runs that
    /// finished in either and the runs still open, the parks that overlap
    /// the window, its spends and its sites.
    fn read(
        store: &Store,
        label: &str,
        window: Window,
        before: Window,
        realm: Option<&str>,
        whole: &BTreeSet<String>,
    ) -> Result<RowOnly, brokkr_store::StoreError> {
        Reading {
            store,
            label,
            window,
            before,
            realm,
            whole,
        }
        .read()
    }
}

/// The window's spend facts, and the same costs grouped by their run.
type Spends = (BTreeMap<String, Vec<(i64, f64)>>, Vec<SpendFact>);

/// One hearth's window as the row reads take it: the store, the hearth's
/// label, the window and the one before it, the query's realm, and the
/// runs the store leaves whole.
struct Reading<'a> {
    store: &'a Store,
    label: &'a str,
    window: Window,
    before: Window,
    realm: Option<&'a str>,
    whole: &'a BTreeSet<String>,
}

impl Reading<'_> {
    fn read(&self) -> Result<RowOnly, brokkr_store::StoreError> {
        let (spends_by_run, spends) = self.spends()?;
        Ok(RowOnly {
            window: self.window,
            runs: self.runs(RowRunOrder::Unordered, &spends_by_run)?,
            by_rule: self.runs(RowRunOrder::ByEndingRule, &spends_by_run)?,
            by_recipe: self.runs(RowRunOrder::ByRealmThenRecipe, &spends_by_run)?,
            parks: self.parks()?,
            spends,
            model: self.sites(RowSiteOrder::Model)?,
            provider: self.sites(RowSiteOrder::Provider)?,
            realm_provider: self.sites(RowSiteOrder::RealmProvider)?,
            recipe_provider: self.sites(RowSiteOrder::RecipeProvider)?,
        })
    }

    /// The window's spends, and the same costs grouped by their run.
    fn spends(&self) -> Result<Spends, brokkr_store::StoreError> {
        let mut by_run: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
        let mut spends = Vec::new();
        self.store
            .window_spends(self.window.from, self.window.to, None, |spend| {
                let named = spend.realm.unwrap_or(self.label);
                if !brokkr_view::realm_in_scope(self.realm, named) {
                    return;
                }
                by_run
                    .entry(spend.run_id.to_string())
                    .or_default()
                    .push((spend.at_ms, spend.usd));
                spends.push(SpendFact {
                    realm: named.to_string(),
                    at: spend.at_ms,
                    usd: spend.usd,
                });
            })?;
        Ok((by_run, spends))
    }

    /// The runs that finished in the window or the one before it, and the
    /// runs still open, in `order`.
    fn runs(
        &self,
        order: RowRunOrder,
        spends: &BTreeMap<String, Vec<(i64, f64)>>,
    ) -> Result<Vec<RunFacts>, brokkr_store::StoreError> {
        let mut runs = Vec::new();
        self.store
            .window_runs(self.before.from, self.window.to, order, None, |row| {
                if let Some(facts) = run_facts(row, self.label, spends, self.realm, self.whole) {
                    runs.push(facts);
                }
            })?;
        Ok(runs)
    }

    /// The parks that overlap the window, in cut phase then run order.
    fn parks(&self) -> Result<Vec<ParkFact>, brokkr_store::StoreError> {
        let mut parks = Vec::new();
        self.store.window_parks(
            self.window.from,
            self.window.to,
            self.window.to,
            None,
            |park| {
                if self.whole.contains(park.run_id) {
                    return;
                }
                let named = park.realm.unwrap_or(self.label);
                if !brokkr_view::realm_in_scope(self.realm, named) {
                    return;
                }
                parks.push(ParkFact {
                    realm: named.to_string(),
                    hearth: self.label.to_string(),
                    run: park.run_id.to_string(),
                    phase: park.phase.map(str::to_string),
                    at: park.at,
                    answered: park.answered.unwrap_or(park.ended),
                    ended: park.ended,
                });
            },
        )?;
        Ok(parks)
    }

    /// The sites that ran in the window, in `order`. A row the whole read
    /// opens no site by is mapped to no fact here, so the stream carries
    /// only the sites every read agrees on.
    fn sites(&self, order: RowSiteOrder) -> Result<Vec<SiteFact>, brokkr_store::StoreError> {
        let mut rows = Vec::new();
        self.store
            .window_sites(self.window.from, self.window.to, None, order, |site| {
                rows.push(site_fact(site, self.label));
            })?;
        Ok(rows
            .into_iter()
            .flatten()
            .filter(|fact| brokkr_view::realm_in_scope(self.realm, &fact.realm))
            .collect())
    }
}

/// One run's row as the figures fold it, its reported costs from the
/// window's spends.
fn run_facts(
    row: &WindowRun<'_>,
    label: &str,
    spends: &BTreeMap<String, Vec<(i64, f64)>>,
    realm: Option<&str>,
    whole: &BTreeSet<String>,
) -> Option<RunFacts> {
    if whole.contains(row.run_id) {
        return None;
    }
    let named = row.realm.unwrap_or(label);
    if !brokkr_view::realm_in_scope(realm, named) {
        return None;
    }
    Some(RunFacts {
        realm: named.to_string(),
        recipe: row.recipe.map(str::to_string),
        open: row.open,
        started: row.started_ms,
        finished: finished(row),
        implement_visits: row.implement_visits,
        reviews: Reviews {
            first: row.reviews.first,
            second: row.reviews.second,
            back: row.reviews.back,
        },
        costs: spends.get(row.run_id).cloned().unwrap_or_default(),
    })
}

/// The ending a run's row keeps, as the whole read's fold states it. The
/// hard stop is the row's flag — the fold's status and phase — so a stop
/// whose last ruling named no rule keeps its stop, and a rule a completed
/// run named is no stop.
fn finished(row: &WindowRun<'_>) -> Option<Finished> {
    Some(Finished {
        at: row.finished_ms?,
        ending: row.ending?,
        hard_stop: row.hard_stop.then(|| HardStop {
            rule: row.ending_rule.map(str::to_string),
        }),
    })
}

/// One site's row as the figures fold it. A row without the start or the
/// provider a whole read opens the site by is no site (the whole read
/// opens none), so it is left out.
fn site_fact(site: &WindowSite<'_>, label: &str) -> Option<SiteFact> {
    Some(SiteFact {
        realm: site.realm.unwrap_or(label).to_string(),
        recipe: site.recipe.map(str::to_string),
        open: site.open,
        ended: site.finished_ms,
        at: site.started_ms?,
        provider: site.provider?.to_string(),
        model: site.model.map(str::to_string),
        reported: site.reported,
        usage: site.usage.clone(),
    })
}

impl FactSource for RowOnly {
    fn runs(&self, visit: &mut dyn FnMut(RunFacts)) {
        for run in &self.runs {
            visit(run.clone());
        }
    }

    fn runs_in(&self, order: RunOrder, visit: &mut dyn FnMut(RunFacts)) {
        let runs = match order {
            RunOrder::Rule => &self.by_rule,
            RunOrder::Recipe => &self.by_recipe,
        };
        for run in runs {
            visit(run.clone());
        }
    }

    fn durations(&self, visit: &mut dyn FnMut(&str, i64)) {
        for run in &self.runs {
            let Some(finished) = &run.finished else {
                continue;
            };
            if finished.at < self.window.from || finished.at >= self.window.to {
                continue;
            }
            let started = run.started.unwrap_or(finished.at);
            visit(&run.realm, finished.at.saturating_sub(started).max(0));
        }
    }

    fn parks(&self, visit: &mut dyn FnMut(ParkFact)) {
        for park in &self.parks {
            visit(park.clone());
        }
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact)) {
        for spend in &self.spends {
            visit(spend.clone());
        }
    }

    fn sites_in(&self, order: SiteOrder, visit: &mut dyn FnMut(SiteFact)) {
        let sites = match order {
            SiteOrder::Model => &self.model,
            SiteOrder::Provider => &self.provider,
            SiteOrder::RealmProvider => &self.realm_provider,
            SiteOrder::RecipeProvider => &self.recipe_provider,
        };
        for site in sites {
            visit(site.clone());
        }
    }
}

/// One run the store leaves whole, and where its realm first stands: the
/// `runs` stamp as the whole read holds it, placed by the one key both
/// reads share (decision 0089 ruling A4's tie, `(created_at, run_id)`).
struct BehindRealm {
    created_at: String,
    run_id: String,
    realm: String,
}

/// One rows-read hearth: the facts its figures fold, and the realm list and
/// unreadable counts its rows state (decision 0089 ruling A4, slice 4's
/// unit U3c).
pub(super) struct RowSource {
    facts: FactUnion,
    scope: Scope,
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
/// realms, the windows a figure folds, the query's realm and clock, and the
/// range its whole fallback states (decision 0071 ruling 10's parameter
/// object).
struct Rows<'a> {
    store: &'a Store,
    label: &'a str,
    configured: &'a [String],
    window: Window,
    before: Window,
    realm: Option<&'a str>,
    range: brokkr_view::Range,
    now: &'a str,
}

/// The runs a store leaves whole, read one at a time from their own keys
/// and never the journal: slice 2's one-run opening (decision 0089 ruling
/// 8). A run that will not load refuses this whole fallback, so the caller
/// reads its hearth whole instead of silently leaving a figure empty. Each
/// behind run's realm and stamp come back with it, so the hearth's realm
/// list can place it where the whole read does (U3c).
fn read_whole(
    rows: &Rows<'_>,
    ids: &BTreeSet<String>,
    behind: &BTreeMap<String, String>,
) -> Result<(WholeRead, Vec<BehindRealm>), brokkr_store::StoreError> {
    let mut events = Vec::new();
    let mut states = Vec::new();
    let mut behind_realms = Vec::new();
    for run_id in ids {
        let (_opened, read) = rows.store.open_kept(run_id)?;
        let state = fold(&read).ok();
        if let Some(created_at) = behind.get(run_id) {
            behind_realms.push(BehindRealm {
                created_at: created_at.clone(),
                run_id: run_id.clone(),
                realm: match aggregate(&read, state.as_ref()).realm {
                    Some(realm) => realm,
                    None => rows.label.to_string(),
                },
            });
        }
        states.push(state);
        events.push(read);
    }
    let runs: Vec<brokkr_view::AnalyticsRun<'_>> = events
        .iter()
        .zip(&states)
        .map(|(events, state)| brokkr_view::AnalyticsRun {
            events,
            state: state.as_ref(),
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
    Ok((brokkr_view::whole_read(&query, rows.window), behind_realms))
}

/// One rows-read hearth's realm list and unreadable counts: configured
/// realms or the label first, then every run's realm in first-`created_at`
/// order through the store's covering index, the behind runs' realms placed
/// by their one-run stamp, the refused and acknowledged counts, and the
/// runs the one-run fallback opened and could not fold (decision 0089
/// ruling A4, slice 4's units U3c and U3c2).
fn store_scope(
    rows: &Rows<'_>,
    behind: &[BehindRealm],
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
    let mut places: Vec<(String, String, String)> = Vec::new();
    rows.store
        .realm_names(wanted, |created_at, run_id, named| {
            places.push((
                created_at.to_string(),
                run_id.to_string(),
                named.unwrap_or(rows.label).to_string(),
            ));
        })?;
    places.extend(behind.iter().map(|behind| {
        (
            behind.created_at.clone(),
            behind.run_id.clone(),
            behind.realm.clone(),
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

/// One migrated hearth's facts and scope: the rows, merged with the runs the
/// store leaves whole (decision 0089 ruling 8), and the store's own realm
/// list and counts for the hearth that reads them (U3c).
fn read_rows(rows: &Rows<'_>) -> Result<RowSource, brokkr_store::StoreError> {
    let mut behind = BTreeMap::new();
    let mut whole = BTreeSet::new();
    rows.store.behind_runs(|run_id, created_at| {
        behind.insert(run_id.to_string(), created_at.to_string());
        whole.insert(run_id.to_string());
    })?;
    rows.store
        .window_fallbacks(rows.window.from, rows.window.to, None, |fallback| {
            whole.insert(fallback.run_id.to_string());
        })?;
    let row = RowOnly::read(
        rows.store,
        rows.label,
        rows.window,
        rows.before,
        rows.realm,
        &whole,
    )?;
    let facts = if whole.is_empty() {
        // Nothing stands behind its row: the rows alone, still merged as
        // one source so each figure folds its own key order.
        FactUnion::new(vec![Box::new(row)])
    } else {
        let (fallback, behind) = read_whole(rows, &whole, &behind)?;
        let scope = store_scope(rows, &behind, fallback.scope().1)?;
        return Ok(RowSource {
            facts: FactUnion::new(vec![Box::new(row), Box::new(fallback)]),
            scope,
        });
    };
    let scope = store_scope(rows, &[], &BTreeMap::new())?;
    Ok(RowSource { facts, scope })
}

/// One hearth's facts as the readout folds them: the whole read for a
/// hearth that is not migrated, not ready or not openable, and the store's
/// rows for a migrated, ready one (decision 0089 ruling 8).
pub(super) enum HearthSource {
    Whole(WholeRead),
    Rows(RowSource),
}

impl HearthSource {
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
    ) -> HearthSource {
        HearthSource::Whole(whole_of(grouped, range, realm, now, window))
    }

    /// One migrated, ready hearth's rows alone, with no fleet read behind
    /// it: the caller has already opened the store read-only and found its
    /// aggregates ready and its listing readable (decision 0089 ruling 8's
    /// boundary, slice 4's unit U3c2). A refusal is the caller's to answer
    /// with the whole read; a behind run's one-run opening is still read,
    /// and its refusal is returned here.
    ///
    /// R1's limit, as the module doc states it: this path trusts the row.
    /// A chain broken behind the store's back after its row stood at its
    /// head is out of its reach, and the journal and integrity checks find
    /// it (`doctor`, #1000).
    pub(super) fn from_store(
        grouped: &brokkr_view::AnalyticsHearth<'_>,
        range: brokkr_view::Range,
        realm: Option<&str>,
        now: &str,
        window: Window,
        before: Window,
        store: &Store,
    ) -> Result<HearthSource, brokkr_store::StoreError> {
        let rows = read_rows(&Rows {
            store,
            label: grouped.label,
            configured: grouped.realms,
            window,
            before,
            realm,
            range,
            now,
        })?;
        Ok(HearthSource::Rows(rows))
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

impl FactSource for HearthSource {
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

    fn runs_in(&self, order: RunOrder, visit: &mut dyn FnMut(RunFacts)) {
        match self {
            HearthSource::Whole(whole) => whole.runs_in(order, visit),
            HearthSource::Rows(rows) => rows.facts.runs_in(order, visit),
        }
    }

    fn durations(&self, visit: &mut dyn FnMut(&str, i64)) {
        match self {
            HearthSource::Whole(whole) => whole.durations(visit),
            HearthSource::Rows(rows) => rows.facts.durations(visit),
        }
    }

    fn parks(&self, visit: &mut dyn FnMut(ParkFact)) {
        match self {
            HearthSource::Whole(whole) => whole.parks(visit),
            HearthSource::Rows(rows) => rows.facts.parks(visit),
        }
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact)) {
        match self {
            HearthSource::Whole(whole) => whole.spends(visit),
            HearthSource::Rows(rows) => rows.facts.spends(visit),
        }
    }

    fn sites_in(&self, order: SiteOrder, visit: &mut dyn FnMut(SiteFact)) {
        match self {
            HearthSource::Whole(whole) => whole.sites_in(order, visit),
            HearthSource::Rows(rows) => rows.facts.sites_in(order, visit),
        }
    }
}

#[cfg(test)]
mod tests;
