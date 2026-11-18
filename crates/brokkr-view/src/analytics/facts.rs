//! What one run's journal and folded state say, read once into the facts
//! every figure is aggregated from, and the runs a query's realm keeps.
//! What the writer keeps of a run for them, brokkr-core reads
//! ([`aggregate`]), so the readout and the run's summary row read it one
//! way (decision 0089 ruling 7).
//!
//! The seam is [`FactSource`]: today's whole read ([`WholeRead`]) is one
//! implementation, and a store's rows are the other. A source yields owned
//! facts rather than borrowed ones, so a source over rows can build each
//! fact as it reads it and a [`FactUnion`] can hold several sources' facts
//! to merge them in a figure's key order. The facts every stream carries
//! are the window's: a source is built for the query's range and clock, so
//! [`FactSource::parks`], [`FactSource::spends`] and
//! [`FactSource::sites_in`] hand over only what overlaps the window.

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::fold::{concluded, RunState};
use brokkr_core::summary::{aggregate, cut, parks as project_parks, MODEL_BYTES, NAME_BYTES};
use brokkr_core::EventEnvelope;

use super::figures::total;
use super::merge::Merged;
use super::{AnalyticsRun, QuarantinedHearth, Query};
use crate::attempts::{attempts, Attempts, Site, Spend};
use crate::TokenUsage;

pub(crate) use brokkr_core::summary::Finished;

/// A name a run's data states, cut where every figure groups it (decision
/// 0089 ruling 7): a phase, rule, provider, realm or recipe.
pub(crate) fn name_key(text: &str) -> &str {
    cut(text, NAME_BYTES)
}

/// A model, cut where every figure groups it: the seat record's bound.
pub(crate) fn model_key(text: &str) -> &str {
    cut(text, MODEL_BYTES)
}

/// An absent or stated name, cut the same way.
fn optional(text: Option<&str>) -> Option<&str> {
    text.map(name_key)
}

/// Whether a finished run completed: its ending implies `completed`
/// (decision 0064 ruling 6).
pub(crate) fn shipped(finished: &Finished) -> bool {
    finished.ending.status() == brokkr_core::fold::Status::Completed
}

/// Whether a run of `realm` is in the query's realm. Both names are cut
/// where a row groups them, so filtering by a row's printed realm selects
/// the same group the row was derived from (decision 0089 ruling 7).
pub fn realm_in_scope(wanted: Option<&str>, realm: &str) -> bool {
    wanted.is_none_or(|wanted| name_key(wanted) == name_key(realm))
}

/// A half-open span of time: its first moment held, its last not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub from: i64,
    pub to: i64,
}

impl Window {
    pub(crate) fn holds(self, at: i64) -> bool {
        at >= self.from && at < self.to
    }

    pub(crate) fn overlap(self, start: i64, end: i64) -> i64 {
        (end.min(self.to) - start.max(self.from)).max(0)
    }
}

struct Park {
    phase: Option<String>,
    at: i64,
    answered: i64,
    ended: i64,
}

/// Where a run's review rulings sent it: whether its first sent it back,
/// whether its second routed it on, and whether any sent it back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reviews {
    pub first: bool,
    pub second: bool,
    pub back: bool,
}

/// The facts one run contributes to the figures that fold runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunFacts {
    /// The hearth's label and the run's id: a run id is unique only within
    /// one journal, so a run's identity, and the tie a figure's key order
    /// breaks by, is both.
    pub hearth: String,
    pub run: String,
    pub realm: String,
    /// The recipe the run started under, when its start names one.
    pub recipe: Option<String>,
    /// Still running or parked: not concluded.
    pub open: bool,
    pub started: Option<i64>,
    pub finished: Option<Finished>,
    pub implement_visits: u64,
    pub reviews: Reviews,
    /// The run's reported costs inside the source's window, as `brokkr
    /// costs` reads them, summed in time then journal order; `None` where it
    /// reported none there. One sum a run, so no stream holds a run's costs
    /// (decision 0089 ruling 7). Only the recipe order's stream is folded
    /// for it ([`RunOrder::Recipe`]), so a source may leave it `None` on
    /// every other stream.
    pub spent: Option<f64>,
}

/// One park a figure folds, with the realm and the run it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParkFact {
    pub realm: String,
    /// The hearth's label: a run id is unique only within one journal, so
    /// a park's identity is (hearth, run), never the run id alone.
    pub hearth: String,
    /// The run's id: half the identity `waiting_by_step` counts a change
    /// of.
    pub run: String,
    pub phase: Option<String>,
    pub at: i64,
    pub answered: i64,
    pub ended: i64,
}

/// One reported cost a figure folds.
#[derive(Clone, Debug, PartialEq)]
pub struct SpendFact {
    pub realm: String,
    pub at: i64,
    pub usd: f64,
}

/// One seat site that ran in the window, with what its in-window
/// checkpoints reported and metered.
#[derive(Clone, Debug, PartialEq)]
pub struct SiteFact {
    /// The hearth's label and the run's id, the tie a site's key order
    /// breaks by, as a run's does.
    pub hearth: String,
    pub run: String,
    pub realm: String,
    pub recipe: Option<String>,
    pub open: bool,
    pub ended: Option<i64>,
    /// The site's start: the moment `models` keeps it by.
    pub at: i64,
    pub provider: String,
    pub model: Option<String>,
    pub reported: bool,
    pub usage: Option<TokenUsage>,
}

/// The order a grouped run figure folds runs in: by the rule that stopped
/// it, or by the realm and recipe it started under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOrder {
    Rule,
    Recipe,
}

/// The order a grouped site figure folds sites in: by model, by provider,
/// by realm and provider, or by realm, recipe and provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteOrder {
    Model,
    Provider,
    RealmProvider,
    RecipeProvider,
}

/// The facts every figure folds, one at a time: today's whole read, and
/// the store's rows. The seam spans the streams and the realm scope, which
/// must agree on the query and on a run's identity across them, so it is
/// one trait rather than closures (decision 0071 ruling 2).
///
/// A grouped figure pulls its stream already ordered by its own full key
/// (`run_place`, `park_place` and `site_place`): the cut key it groups
/// by, then the hearth and the run. It folds one completed group at a time
/// into a bounded [`super::ranked::Ranked`] and never holds every group,
/// and `waiting_by_step` counts a change of run, never a set of them; a run
/// id alone is not an identity, because two journals may each hold it.
///
/// An ordered stream is pulled: the source hands back an iterator that
/// borrows it exclusively while it is pulled (a statement's cursor, or the
/// whole read's own runs), so a [`FactUnion`] holds every source's stream
/// open at once, one head a source, and merges them without a call frame a
/// source (decision 0089 ruling 7). The order-free streams are visited one
/// fact at a time. The facts a stream yields are already the window's: the
/// source was built for the query's range and clock.
pub trait FactSource {
    /// The realm names the readout paints, and the runs no figure could
    /// read, by their cut realm key. A source that states neither leaves
    /// both empty; today only the whole read states them (decision 0089
    /// ruling A4), so [`WholeRead`], [`FactUnion`] and the caller that
    /// holds it override this.
    fn scope(&self) -> (&[String], &BTreeMap<String, usize>) {
        static NO_REALMS: &[String] = &[];
        static NO_UNREADABLE: BTreeMap<String, usize> = BTreeMap::new();
        (NO_REALMS, &NO_UNREADABLE)
    }
    /// The hearths no figure could read. A source that holds no hearth
    /// whole quarantines none; the whole read and the caller that holds it
    /// override this.
    fn quarantined(&self) -> Vec<QuarantinedHearth> {
        Vec::new()
    }
    fn runs(&self, visit: &mut dyn FnMut(RunFacts));
    /// The runs by `order`'s full key (`run_place`).
    fn runs_in(&mut self, order: RunOrder) -> Pulled<'_, RunFacts>;
    /// Each finished run's cut realm and span inside the source's window,
    /// one value a median pass (decision 0089 ruling 7).
    fn durations(&self, visit: &mut dyn FnMut(&str, i64));
    /// How many runs completed inside `window`: the headline's count, and
    /// the window before it for `previous`. Folded from [`Self::runs`]
    /// unless the source counts them itself, as a store's rows do.
    fn shipped_between(&self, window: Window) -> usize {
        let mut count = 0;
        self.runs(&mut |run| {
            let finished = run.finished.as_ref();
            count +=
                usize::from(finished.is_some_and(|ended| window.holds(ended.at) && shipped(ended)));
        });
        count
    }
    /// The parks by `park_place`.
    fn parks(&mut self) -> Pulled<'_, ParkFact>;
    fn spends(&self, visit: &mut dyn FnMut(SpendFact));
    /// The sites by `order`'s full key (`site_place`).
    fn sites_in(&mut self, order: SiteOrder) -> Pulled<'_, SiteFact>;
}

/// One ordered stream of a source's facts, pulled as a figure folds them.
pub type Pulled<'s, T> = Box<dyn Iterator<Item = T> + 's>;

/// A source's realm names and unreadable counts, as its `scope` hands
/// them over: one expression the sources share.
fn scope_of<'a>(
    realms: &'a [String],
    unreadable: &'a BTreeMap<String, usize>,
) -> (&'a [String], &'a BTreeMap<String, usize>) {
    (realms, unreadable)
}

/// The cut key a run groups by in `order`.
fn run_key(run: &RunFacts, order: RunOrder) -> (Option<&str>, &str, &str) {
    match order {
        RunOrder::Rule => (
            run.finished
                .as_ref()
                .and_then(|finished| finished.hard_stop.as_ref())
                .and_then(|stop| stop.rule.as_deref())
                .map(name_key),
            "",
            "",
        ),
        RunOrder::Recipe => (
            Some(name_key(&run.realm)),
            run.recipe.as_deref().map(name_key).unwrap_or(""),
            "",
        ),
    }
}

/// The cut key a site groups by in `order`.
fn site_key(site: &SiteFact, order: SiteOrder) -> (Option<&str>, &str, &str) {
    match order {
        SiteOrder::Model => (site.model.as_deref().map(model_key), "", ""),
        SiteOrder::Provider => (Some(name_key(&site.provider)), "", ""),
        SiteOrder::RealmProvider => (Some(name_key(&site.realm)), name_key(&site.provider), ""),
        SiteOrder::RecipeProvider => (
            Some(name_key(&site.realm)),
            site.recipe.as_deref().map(name_key).unwrap_or(""),
            name_key(&site.provider),
        ),
    }
}

/// A run's full key in `order` (#1068's F5): the cut key it groups by,
/// then its hearth and its id, so two runs that tie on the key across
/// hearths stand in one order whichever source yields them.
pub(crate) fn run_place(run: &RunFacts, order: RunOrder) -> Place<'_> {
    (run_key(run, order), run.hearth.as_str(), run.run.as_str())
}

/// A cut key a figure groups by, up to three names deep.
type Key<'a> = (Option<&'a str>, &'a str, &'a str);

/// A fact's full key: its cut key, then its hearth and run.
type Place<'a> = (Key<'a>, &'a str, &'a str);

/// A park's full key: its cut phase, then its hearth and run. A run's
/// parks tie on it and keep their source's order.
pub(crate) fn park_place(park: &ParkFact) -> Key<'_> {
    (
        optional(park.phase.as_deref()),
        park.hearth.as_str(),
        park.run.as_str(),
    )
}

/// A site's full key in `order`: the cut key it groups by, then its hearth
/// and run. A run's sites tie on it and keep their source's order.
pub(crate) fn site_place(site: &SiteFact, order: SiteOrder) -> Place<'_> {
    (
        site_key(site, order),
        site.hearth.as_str(),
        site.run.as_str(),
    )
}

/// The runs a range keeps: those that ended inside it, and those still
/// open. The one reading of "in range" the per-recipe figures and the
/// recipe's graph share.
pub(crate) fn in_range(finished: Option<i64>, open: bool, window: Window) -> bool {
    open || finished.is_some_and(|ended| window.holds(ended))
}

/// Whether the run is still running or parked: its fold has not
/// concluded it.
pub(crate) fn is_open(state: &RunState) -> bool {
    concluded(state).is_none()
}

/// Every park, its open moments read as `now`: it waits on the operator
/// until their next command, and stays parked until the run moves again.
/// Brokkr-core reads which moments those are (decision 0089 ruling 7).
fn parks(events: &[EventEnvelope], now: i64) -> Vec<Park> {
    project_parks(events)
        .into_iter()
        .map(|park| Park {
            phase: park.phase,
            at: park.at,
            answered: park.answered.or(park.ended).unwrap_or(now),
            ended: park.ended.unwrap_or(now),
        })
        .collect()
}

/// One run of today's whole read: what the figures fold, and the streams
/// the run contributes to them.
struct WholeRun {
    facts: RunFacts,
    /// Every reported cost the run's journal holds, each its time and
    /// amount, for the spends.
    costs: Vec<(i64, f64)>,
    parks: Vec<Park>,
    sites: Vec<Site>,
    readable: bool,
}

fn whole_run(run: &AnalyticsRun, label: &str, window: Window) -> WholeRun {
    // One reading of which events this run applies: a repaired tail is
    // kept and shown and never enters a figure (decision 0108 ruling 4).
    let applied = crate::live::derived(run.events, run.state);
    let events = applied.as_slice();
    let Attempts { costs, sites } = attempts(events);
    let read = aggregate(events, run.state);
    let mut parks = parks(events, window.to);
    // Phase order within the run, stable so the parks of one phase keep
    // their journal order.
    parks.sort_by(|left, right| {
        optional(left.phase.as_deref()).cmp(&optional(right.phase.as_deref()))
    });
    WholeRun {
        facts: RunFacts {
            hearth: label.to_string(),
            run: run
                .events
                .first()
                .map(|event| event.run_id.clone())
                .unwrap_or_default(),
            realm: read.realm.unwrap_or_else(|| label.to_string()),
            recipe: read.recipe,
            open: run.state.is_some_and(is_open),
            started: read.started_ms,
            finished: read.finished,
            implement_visits: read.implement_visits,
            reviews: Reviews {
                first: read.review_first,
                second: read.review_second,
                back: read.review_back,
            },
            spent: spent_in(&costs, window),
        },
        costs: costs.iter().map(|spend| (spend.at, spend.usd)).collect(),
        parks,
        sites,
        readable: run.state.is_some(),
    }
}

/// A run's costs inside `window`, summed in time then journal order, the
/// order the store's run-cost range reads them in; `None` where it
/// reported none there.
fn spent_in(costs: &[Spend], window: Window) -> Option<f64> {
    let mut kept: Vec<&Spend> = costs
        .iter()
        .filter(|spend| window.holds(spend.at))
        .collect();
    kept.sort_by_key(|spend| spend.at);
    total(kept.into_iter().map(|spend| spend.usd))
}

// ------------------------------------------------------------ the scope

/// Today's whole read: the runs, realms and refusals the query's realm
/// keeps, read from the hearths the fleet handed over.
pub struct WholeRead {
    runs: Vec<WholeRun>,
    realms: Vec<String>,
    unreadable: BTreeMap<String, usize>,
    quarantined: Vec<QuarantinedHearth>,
    window: Window,
}

impl WholeRead {
    /// Read every hearth whole, for `window`: the source today's
    /// `analytics` folds, and the fallback a migrated journal's reader
    /// falls back to (decision 0089 ruling 8).
    pub(super) fn read(query: &Query, window: Window) -> WholeRead {
        let mut scoped = WholeRead {
            runs: Vec::new(),
            realms: Vec::new(),
            unreadable: BTreeMap::new(),
            quarantined: Vec::new(),
            window,
        };
        for hearth in query.hearths {
            let named: Vec<&str> = match hearth.realms.is_empty() {
                true => vec![hearth.label],
                false => hearth.realms.iter().map(String::as_str).collect(),
            };
            if let Some(refusal) = hearth.refusal {
                if named.iter().any(|realm| realm_in_scope(query.realm, realm)) {
                    scoped.quarantined.push(QuarantinedHearth {
                        realm: hearth.label.to_string(),
                        journal: hearth.journal.to_string(),
                        refusal: refusal.to_string(),
                    });
                }
                continue;
            }
            for realm in named {
                scoped.name(query, realm);
            }
            for run in hearth.runs {
                scoped.keep(query, whole_run(run, hearth.label, window));
            }
        }
        scoped
    }

    /// Names `realm` once, cut where a row groups it (decision 0089 ruling
    /// 7): two names that agree to their bound are one group.
    fn name(&mut self, query: &Query, realm: &str) {
        if !realm_in_scope(query.realm, realm) {
            return;
        }
        let realm = name_key(realm);
        if !self.realms.iter().any(|named| named == realm) {
            self.realms.push(realm.to_string());
        }
    }

    /// A run in the query's realm is kept; one whose journal does not
    /// load or fold is counted, and no figure reads it.
    fn keep(&mut self, query: &Query, run: WholeRun) {
        if !realm_in_scope(query.realm, &run.facts.realm) {
            return;
        }
        self.name(query, &run.facts.realm);
        match run.readable {
            true => self.runs.push(run),
            false => {
                *self
                    .unreadable
                    .entry(name_key(&run.facts.realm).to_string())
                    .or_default() += 1;
            }
        }
    }
}

impl FactSource for WholeRead {
    fn scope(&self) -> (&[String], &BTreeMap<String, usize>) {
        scope_of(&self.realms, &self.unreadable)
    }

    fn quarantined(&self) -> Vec<QuarantinedHearth> {
        self.quarantined.clone()
    }

    fn runs(&self, visit: &mut dyn FnMut(RunFacts)) {
        for run in &self.runs {
            visit(run.facts.clone());
        }
    }

    /// The whole read keeps its runs (decision 0089 ruling 8), so it
    /// yields them through its own order of them.
    fn runs_in(&mut self, order: RunOrder) -> Pulled<'_, RunFacts> {
        let mut runs: Vec<&WholeRun> = self.runs.iter().collect();
        runs.sort_by(|left, right| {
            run_place(&left.facts, order).cmp(&run_place(&right.facts, order))
        });
        Box::new(runs.into_iter().map(|run| run.facts.clone()))
    }

    fn durations(&self, visit: &mut dyn FnMut(&str, i64)) {
        for run in &self.runs {
            let Some(finished) = &run.facts.finished else {
                continue;
            };
            if !self.window.holds(finished.at) {
                continue;
            }
            let started = run.facts.started.unwrap_or(finished.at);
            visit(&run.facts.realm, finished.at.saturating_sub(started).max(0));
        }
    }

    /// Each run's parks are already phase-sorted, so merging the runs by
    /// (cut phase, hearth, run) yields them in that order without
    /// collecting every park a second time: one cursor a run, in a set
    /// ordered by the key, so a park's advance costs a logarithm, not a
    /// scan of every cursor. A run id is unique only within its journal,
    /// so the hearth is part of the identity.
    fn parks(&mut self) -> Pulled<'_, ParkFact> {
        let runs: &[WholeRun] = &self.runs;
        let cursor = move |run: usize, index: usize| {
            let whole: &WholeRun = &runs[run];
            whole.parks.get(index).map(|park| {
                (
                    optional(park.phase.as_deref()),
                    whole.facts.hearth.as_str(),
                    whole.facts.run.as_str(),
                    run,
                    index,
                )
            })
        };
        let mut cursors: BTreeSet<(Option<&str>, &str, &str, usize, usize)> =
            (0..runs.len()).filter_map(|run| cursor(run, 0)).collect();
        Box::new(std::iter::from_fn(move || {
            let (_phase, _hearth, _run_id, run, index) = cursors.pop_first()?;
            let whole = &runs[run];
            let park = &whole.parks[index];
            cursors.extend(cursor(run, index + 1));
            Some(ParkFact {
                realm: whole.facts.realm.clone(),
                hearth: whole.facts.hearth.clone(),
                run: whole.facts.run.clone(),
                phase: optional(park.phase.as_deref()).map(str::to_string),
                at: park.at,
                answered: park.answered,
                ended: park.ended,
            })
        }))
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact)) {
        for run in &self.runs {
            for (at, usd) in &run.costs {
                if !self.window.holds(*at) {
                    continue;
                }
                visit(SpendFact {
                    realm: run.facts.realm.clone(),
                    at: *at,
                    usd: *usd,
                });
            }
        }
    }

    fn sites_in(&mut self, order: SiteOrder) -> Pulled<'_, SiteFact> {
        let (runs, window): (&[WholeRun], Window) = (&self.runs, self.window);
        let keeps = |at: i64, _: Option<&str>| window.holds(at);
        let mut sites: Vec<(usize, usize)> = runs
            .iter()
            .enumerate()
            .flat_map(|(run, whole)| (0..whole.sites.len()).map(move |site| (run, site)))
            .filter(|(run, site)| runs[*run].sites[*site].ran(&keeps))
            .collect();
        let fact = move |(run, site): (usize, usize)| {
            let whole = &runs[run];
            site_fact(whole, &whole.sites[site], window)
        };
        sites.sort_by(|left, right| {
            let (left_fact, right_fact) = (fact(*left), fact(*right));
            site_place(&left_fact, order)
                .cmp(&site_place(&right_fact, order))
                .then(left.cmp(right))
        });
        Box::new(sites.into_iter().map(fact))
    }
}

/// One site of `whole`'s run as the figures fold it, its in-window facts
/// read under the source's window.
fn site_fact(whole: &WholeRun, site: &Site, window: Window) -> SiteFact {
    let keeps = |at: i64, _: Option<&str>| window.holds(at);
    SiteFact {
        hearth: whole.facts.hearth.clone(),
        run: whole.facts.run.clone(),
        realm: whole.facts.realm.clone(),
        recipe: whole.facts.recipe.clone(),
        open: whole.facts.open,
        ended: whole.facts.finished.as_ref().map(|finished| finished.at),
        at: site.at,
        provider: site.provider.clone(),
        model: site.model.clone(),
        reported: site.reported(&keeps),
        usage: site.usage(&keeps),
    }
}

/// Several sources as one: a graph's hearths, or a migrated hearth's rows
/// and the runs it reads whole. Each ordered stream is the k-way merge of
/// the sources' streams in the figure's full key, one head a source held,
/// so a group never straddles two sources and nothing collects a figure's
/// facts; the order-free streams are the sources' in turn.
pub struct FactUnion<'a> {
    sources: Vec<Box<dyn FactSource + 'a>>,
    realms: Vec<String>,
    unreadable: BTreeMap<String, usize>,
}

impl<'a> FactUnion<'a> {
    /// The sources as one; a source may borrow what it reads from, such as
    /// a store's connection, for the union's length.
    pub fn new(sources: Vec<Box<dyn FactSource + 'a>>) -> FactUnion<'a> {
        let mut realms = Vec::new();
        let mut unreadable = BTreeMap::new();
        for source in &sources {
            let (named, failed) = source.scope();
            for realm in named {
                if !realms.contains(realm) {
                    realms.push(realm.clone());
                }
            }
            for (realm, count) in failed {
                *unreadable.entry(realm.clone()).or_default() += count;
            }
        }
        FactUnion {
            sources,
            realms,
            unreadable,
        }
    }
}

impl FactSource for FactUnion<'_> {
    fn quarantined(&self) -> Vec<QuarantinedHearth> {
        self.sources
            .iter()
            .flat_map(|source| source.quarantined())
            .collect()
    }

    fn scope(&self) -> (&[String], &BTreeMap<String, usize>) {
        scope_of(&self.realms, &self.unreadable)
    }

    fn runs(&self, visit: &mut dyn FnMut(RunFacts)) {
        for source in &self.sources {
            source.runs(visit);
        }
    }

    fn runs_in(&mut self, order: RunOrder) -> Pulled<'_, RunFacts> {
        let streams = self.sources.iter_mut().map(|source| source.runs_in(order));
        let by_place = |left: &RunFacts, right: &RunFacts, order: RunOrder| {
            run_place(left, order).cmp(&run_place(right, order))
        };
        Box::new(Merged::new(streams.collect(), by_place, order))
    }

    fn durations(&self, visit: &mut dyn FnMut(&str, i64)) {
        for source in &self.sources {
            source.durations(visit);
        }
    }

    fn shipped_between(&self, window: Window) -> usize {
        self.sources
            .iter()
            .map(|source| source.shipped_between(window))
            .sum()
    }

    fn parks(&mut self) -> Pulled<'_, ParkFact> {
        let streams = self.sources.iter_mut().map(|source| source.parks());
        let by_place =
            |left: &ParkFact, right: &ParkFact, (): ()| park_place(left).cmp(&park_place(right));
        Box::new(Merged::new(streams.collect(), by_place, ()))
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact)) {
        for source in &self.sources {
            source.spends(visit);
        }
    }

    fn sites_in(&mut self, order: SiteOrder) -> Pulled<'_, SiteFact> {
        let streams = self.sources.iter_mut().map(|source| source.sites_in(order));
        let by_place = |left: &SiteFact, right: &SiteFact, order: SiteOrder| {
            site_place(left, order).cmp(&site_place(right, order))
        };
        Box::new(Merged::new(streams.collect(), by_place, order))
    }
}

#[cfg(test)]
mod tests;
