//! One hearth's rows as the figures' facts, pulled from the store's window
//! statements as each figure folds them (decision 0089 ruling 7, slice 4's
//! unit U5b2b1): no figure's facts are collected, and a run's cost inside
//! the window is summed by the store's range of its key as the run is
//! pulled, so neither a run's costs nor the window's are held.
//!
//! Every statement runs under the hearth's snapshot, which the row source
//! holds for the readout's length ([`brokkr_store::snapshot::ReadSnapshot`]),
//! so the statements a figure pulls late agree run for run with the runs
//! read whole at the start. A statement or a row that refuses ends its
//! stream and marks the hearth refused, so its caller reads the hearth
//! whole instead (ruling 8's boundary).

use std::cell::Cell;

use brokkr_core::summary::{cut, Finished, HardStop, NAME_BYTES};
use brokkr_store::aggregate::pull::{Bounds, ParkPull, ParkRows, RunPull, RunRows};
use brokkr_store::aggregate::{RunOrder as RowRunOrder, WindowPark, WindowRun};
use brokkr_store::metered::{SiteOrder as RowSiteOrder, SitePull, SiteRows, WindowSite};
use brokkr_store::{Store, StoreError};
use brokkr_view::{
    realm_in_scope, FactSource, ParkFact, Pulled, Reviews, RunFacts, RunOrder, SiteFact, SiteOrder,
    SpendFact, Window,
};

/// One hearth's rows: the store, the hearth's label, the window, the
/// query's realm, and where a refusal is marked. The store's statements
/// leave out the runs it leaves whole, so nothing here holds a set of them.
/// The window before is read only for its count, by
/// [`FactSource::shipped_between`].
pub(super) struct Hearth<'a> {
    pub(super) store: &'a Store,
    pub(super) label: &'a str,
    pub(super) window: Window,
    pub(super) realm: Option<&'a str>,
    pub(super) refused: &'a Cell<bool>,
}

/// One hearth's rows as a [`FactSource`], and the statement each ordered
/// stream pulls while a figure folds it: a figure's stream borrows its
/// statement for as long as the figure pulls it, so every hearth's stream of
/// one figure is open at once for the union to merge. A statement is
/// finalized when the next stream of its kind replaces it, or when the
/// source is dropped at the readout's end.
pub(super) struct RowOnly<'a> {
    hearth: Hearth<'a>,
    runs: Option<RunPull<'a>>,
    parks: Option<ParkPull<'a>>,
    sites: Option<SitePull<'a>>,
}

impl<'a> RowOnly<'a> {
    pub(super) fn new(hearth: Hearth<'a>) -> RowOnly<'a> {
        RowOnly {
            hearth,
            runs: None,
            parks: None,
            sites: None,
        }
    }
}

impl Hearth<'_> {
    /// `read`'s value, or none once the hearth's read is marked refused.
    fn kept<T>(&self, read: Result<T, StoreError>) -> Option<T> {
        read.map_err(|_| self.refused.set(true)).ok()
    }

    /// The bounds a labelled window statement reads the window by: a run
    /// whose stored realm is NULL is ordered by the hearth's label, cut
    /// where a row groups it, as its facts name it.
    fn bounds(&self) -> Bounds<'_> {
        Bounds {
            from: self.window.from,
            to: self.window.to,
            realm: None,
            label: Some(cut(self.label, NAME_BYTES)),
        }
    }

    /// The next run of `rows` in the query's realm, its cost inside the
    /// window summed by the store where the stream folds it.
    fn next_run(&self, rows: &mut RunRows<'_>, costs: bool) -> Option<RunFacts> {
        while let Some(row) = self.kept(rows.next_run()).flatten() {
            if let Some(mut facts) = run_facts(&row, self.label, self.realm) {
                if costs {
                    let spent = self
                        .store
                        .run_spent(row.run_id, self.window.from, self.window.to);
                    facts.spent = self.kept(spent).flatten();
                }
                return Some(facts);
            }
        }
        None
    }

    /// The next park of `rows` in the query's realm.
    fn next_park(&self, rows: &mut ParkRows<'_>) -> Option<ParkFact> {
        while let Some(park) = self.kept(rows.next_park()).flatten() {
            if let Some(fact) = park_fact(&park, self.label, self.realm) {
                return Some(fact);
            }
        }
        None
    }

    /// The next site of `rows` in the query's realm that the whole read
    /// opens a site by.
    fn next_site(&self, rows: &mut SiteRows<'_>) -> Option<SiteFact> {
        while let Some(site) = self.kept(rows.next_site()).flatten() {
            if let Some(fact) = site_fact(&site, self.label) {
                if realm_in_scope(self.realm, &fact.realm) {
                    return Some(fact);
                }
            }
        }
        None
    }
}

impl FactSource for RowOnly<'_> {
    /// The runs that finished in the window, and the runs still open.
    fn runs(&self, visit: &mut dyn FnMut(RunFacts)) {
        let hearth = &self.hearth;
        let read = hearth.store.window_runs(
            hearth.window.from,
            hearth.window.to,
            RowRunOrder::Unordered,
            None,
            |row| {
                if let Some(facts) = run_facts(row, hearth.label, hearth.realm) {
                    visit(facts);
                }
            },
        );
        hearth.kept(read);
    }

    /// The runs `runs` reads, in `order`'s key; the recipe order carries
    /// each run's costs, the one figure that folds them. A statement that
    /// will not prepare is a stream of nothing, so every figure is folded
    /// and the refusal is the caller's.
    fn runs_in(&mut self, order: RunOrder) -> Pulled<'_, RunFacts> {
        let (order, costs) = match order {
            RunOrder::Rule => (RowRunOrder::ByEndingRule, false),
            RunOrder::Recipe => (RowRunOrder::ByRealmThenRecipe, true),
        };
        let hearth = &self.hearth;
        let prepared = hearth.store.prepare_runs(&hearth.bounds(), order);
        let Some(pull) = hearth.kept(prepared) else {
            return Box::new(std::iter::empty());
        };
        let mut rows = self.runs.insert(pull).rows();
        Box::new(std::iter::from_fn(move || {
            hearth.next_run(&mut rows, costs)
        }))
    }

    /// Each run that finished in the window, its span read by the store's
    /// one pass, a median pass a call (decision 0089 ruling 7).
    fn durations(&self, visit: &mut dyn FnMut(&str, i64)) {
        let hearth = &self.hearth;
        let read = hearth
            .store
            .durations(hearth.window.from, hearth.window.to, |realm, span| {
                let named = realm.unwrap_or(hearth.label);
                if realm_in_scope(hearth.realm, named) {
                    visit(named, span);
                }
            });
        hearth.kept(read);
    }

    /// The runs that completed in `window`, counted by the store one
    /// stored realm a group. The runs it leaves whole are not in its
    /// groups: the hearth's whole read beside these rows counts them
    /// (#1068's F4, U5b1's F4), so the hearth's sum is the whole read's.
    fn shipped_between(&self, window: Window) -> usize {
        let (hearth, mut count) = (&self.hearth, 0);
        let read = hearth
            .store
            .finished_between(window.from, window.to, |realm, shipped| {
                if realm_in_scope(hearth.realm, realm.unwrap_or(hearth.label)) {
                    count += shipped as usize;
                }
            });
        hearth.kept(read);
        count
    }

    /// The parks that overlap the window, in cut phase then run order.
    fn parks(&mut self) -> Pulled<'_, ParkFact> {
        let hearth = &self.hearth;
        let (from, to) = (hearth.window.from, hearth.window.to);
        let prepared = hearth.store.prepare_parks(from, to, to, None);
        let Some(pull) = hearth.kept(prepared) else {
            return Box::new(std::iter::empty());
        };
        let mut rows = self.parks.insert(pull).rows();
        Box::new(std::iter::from_fn(move || hearth.next_park(&mut rows)))
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact)) {
        let hearth = &self.hearth;
        let read =
            hearth
                .store
                .window_spends(hearth.window.from, hearth.window.to, None, |spend| {
                    let named = spend.realm.unwrap_or(hearth.label);
                    if realm_in_scope(hearth.realm, named) {
                        visit(SpendFact {
                            realm: named.to_string(),
                            at: spend.at_ms,
                            usd: spend.usd,
                        });
                    }
                });
        hearth.kept(read);
    }

    /// The sites that ran in the window, in `order`'s key.
    fn sites_in(&mut self, order: SiteOrder) -> Pulled<'_, SiteFact> {
        let order = match order {
            SiteOrder::Model => RowSiteOrder::Model,
            SiteOrder::Provider => RowSiteOrder::Provider,
            SiteOrder::RealmProvider => RowSiteOrder::RealmProvider,
            SiteOrder::RecipeProvider => RowSiteOrder::RecipeProvider,
        };
        let hearth = &self.hearth;
        let prepared = hearth.store.prepare_sites(&hearth.bounds(), order);
        let Some(pull) = hearth.kept(prepared) else {
            return Box::new(std::iter::empty());
        };
        let mut rows = self.sites.insert(pull).rows();
        Box::new(std::iter::from_fn(move || hearth.next_site(&mut rows)))
    }
}

/// One park's row as the figures fold it, or none for a park of another
/// realm. The store's park read leaves out the runs the whole read takes
/// (U5b1), and the hearth's reads share one snapshot (U5b2a), so a run is
/// either here or read whole, never both.
fn park_fact(park: &WindowPark<'_>, label: &str, realm: Option<&str>) -> Option<ParkFact> {
    let named = park.realm.unwrap_or(label);
    if !realm_in_scope(realm, named) {
        return None;
    }
    Some(ParkFact {
        realm: named.to_string(),
        hearth: label.to_string(),
        run: park.run_id.to_string(),
        phase: park.phase.map(str::to_string),
        at: park.at,
        answered: park.answered.unwrap_or(park.ended),
        ended: park.ended,
    })
}

/// One run's row as the figures fold it, with no cost: the recipe order's
/// stream reads it by the run's key.
fn run_facts(row: &WindowRun<'_>, label: &str, realm: Option<&str>) -> Option<RunFacts> {
    let named = row.realm.unwrap_or(label);
    if !realm_in_scope(realm, named) {
        return None;
    }
    Some(RunFacts {
        hearth: label.to_string(),
        run: row.run_id.to_string(),
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
        spent: None,
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
        hearth: label.to_string(),
        run: site.run_id.to_string(),
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

#[cfg(test)]
mod tests;
