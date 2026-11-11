//! Every figure of the readout, aggregated from the runs' facts over the
//! range's window, one fact at a time (decision 0089 ruling 7).

use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::facts::{
    model_key, name_key, shipped, FactSource, Finished, RunFacts, RunOrder, SiteOrder, Window,
};
use super::ledger::Tally;
use super::median::medians;
use super::ranked::Ranked;
use super::recipes::recipe_rows;
use super::words::{change_text, date, hours_text, listed, percent, span_text, stamp};
use super::{judged_unshipped, Ledger};
use super::{Analytics, AnalyticsError, CostFigure, Day, Headline, ModelShare, Query, RealmRow};
use super::{ReportedCost, Rework, RuleCount, Shipped, Span, StepWait, StoppedByRule, Waiting};
use super::{DAY_MILLIS, NOT_RECORDED};
use crate::attempts::{Keeps, Site};
use crate::{ABSENT, VIEW_VERSION};

/// Folds each run that finished inside `window`, one at a time.
fn finished_in<F: FnMut(&RunFacts, &Finished)>(
    source: &dyn FactSource,
    window: Window,
    mut fold: F,
) {
    source.runs(&mut |run| {
        if let Some(finished) = &run.finished {
            if window.holds(finished.at) {
                fold(&run, finished);
            }
        }
    });
}

fn shipped_in(source: &dyn FactSource, window: Window) -> usize {
    let mut count = 0;
    finished_in(source, window, |_, finished| {
        count += usize::from(shipped(finished));
    });
    count
}

/// The runs that finished inside `window`, by their endings.
fn ledger_in(source: &dyn FactSource, window: Window) -> Ledger {
    let mut tally = Tally::default();
    finished_in(source, window, |_, finished| {
        tally.add(finished.ending);
    });
    tally.finish()
}

/// The median of `sorted`, the mean of the middle two when it is even;
/// `None` of nothing.
pub(crate) fn middle(sorted: &[i64]) -> Option<i64> {
    let half = sorted.len() / 2;
    match (sorted.len(), sorted.len() % 2) {
        (0, _) => None,
        (_, 1) => Some(sorted[half]),
        _ => Some((sorted[half - 1] + sorted[half]) / 2),
    }
}

/// A duration, or the absence mark when there is none.
pub(crate) fn span(millis: Option<i64>) -> Span {
    Span {
        text: millis.map_or_else(|| ABSENT.to_string(), span_text),
        millis,
    }
}

/// The headline's median and each realm row's, selected in one set of
/// passes (decision 0089 ruling 7): target 0 is the whole read, and target
/// `n + 1` the realm at `realms[n]`.
fn medians_of(source: &dyn FactSource, realms: &[String]) -> Vec<Option<i64>> {
    let index: BTreeMap<&str, usize> = realms
        .iter()
        .enumerate()
        .map(|(position, realm)| (realm.as_str(), position + 1))
        .collect();
    medians(realms.len() + 1, &mut |emit: &mut dyn FnMut(usize, i64)| {
        source.durations(&mut |realm, duration| {
            emit(0, duration);
            if let Some(target) = index.get(name_key(realm)) {
                emit(*target, duration);
            }
        });
    })
}

/// Waiting on the operator and all parked time, inside `window`.
fn waited(source: &dyn FactSource, window: Window) -> (i64, i64) {
    let mut waiting = 0;
    let mut parked = 0;
    source.parks(&mut |park| {
        waiting += window.overlap(park.at, park.answered);
        parked += window.overlap(park.at, park.ended);
    });
    (waiting, parked)
}

fn waiting(source: &dyn FactSource, window: Window) -> Waiting {
    let (waiting_millis, parked_millis) = waited(source, window);
    let share_percent = percent(waiting_millis, parked_millis);
    Waiting {
        waiting_millis,
        parked_millis,
        text: hours_text(waiting_millis),
        share_percent,
        share_text: share_percent.map_or_else(
            || "nothing parked".to_string(),
            |share| format!("{share}% of all parked time"),
        ),
    }
}

fn stopped_by_rule(source: &dyn FactSource, window: Window) -> StoppedByRule {
    let mut count = 0;
    let mut rules: Ranked<Option<String>, usize, Reverse<usize>> = Ranked::new();
    let mut current: Option<(Option<String>, usize)> = None;
    let keep = |group: Option<(Option<String>, usize)>,
                rules: &mut Ranked<Option<String>, usize, Reverse<usize>>| {
        if let Some((rule, count)) = group {
            rules.push(rule, count, Reverse(count));
        }
    };
    source.runs_in(RunOrder::Rule, &mut |run| {
        let Some(finished) = &run.finished else {
            return;
        };
        if !window.holds(finished.at) {
            return;
        }
        let Some(stop) = &finished.hard_stop else {
            return;
        };
        count += 1;
        let rule = stop.rule.as_deref().map(name_key).map(str::to_string);
        match &mut current {
            Some((named, tally)) if *named == rule => *tally += 1,
            _ => {
                keep(current.take(), &mut rules);
                current = Some((rule, 1));
            }
        }
    });
    keep(current.take(), &mut rules);
    let (kept, rules_more) = rules.finish();
    let top = best_rule(kept);
    StoppedByRule {
        count,
        text: top.as_ref().map_or_else(String::new, |top| {
            format!(
                "{} by {}",
                top.count,
                top.rule_id.as_deref().unwrap_or(ABSENT)
            )
        }),
        top,
        rules_more,
    }
}

/// The rule the figure ranks first: the most frequent, the lesser id on a
/// tie.
fn best_rule(mut kept: Vec<(Option<String>, usize)>) -> Option<RuleCount> {
    kept.sort_by(|(left, left_count), (right, right_count)| {
        Reverse(left_count)
            .cmp(&Reverse(right_count))
            .then(left.cmp(right))
    });
    kept.into_iter()
        .next()
        .map(|(rule_id, count)| RuleCount { rule_id, count })
}

/// The sum of `costs`, `None` when there is none to sum.
pub(crate) fn total(costs: impl Iterator<Item = f64>) -> Option<f64> {
    costs.fold(None, |sum, cost| Some(sum.unwrap_or(0.0) + cost))
}

/// A cost figure folded one spend and one completed provider group at a
/// time: the reported cost and a bounded provider selection.
#[derive(Default)]
struct ProviderAcc {
    usd: Option<f64>,
    providers: Ranked<String, bool, ()>,
}

impl ProviderAcc {
    fn spend(&mut self, usd: f64) {
        self.usd = Some(self.usd.unwrap_or(0.0) + usd);
    }

    fn provider(&mut self, provider: &str, reported: bool) {
        self.providers.push(provider.to_string(), reported, ());
    }

    fn finish(self) -> CostFigure {
        let (kept, providers_more) = self.providers.finish();
        cost_parts(kept, self.usd, providers_more)
    }
}

/// What a provider group spans: one provider over every realm in scope, or
/// one provider of one realm.
#[derive(Clone, Copy)]
enum ProviderGroup {
    Whole,
    PerRealm,
}

impl ProviderGroup {
    /// The site order the source yields for the group: providers alone, or
    /// realms and then providers.
    fn order(self) -> SiteOrder {
        match self {
            ProviderGroup::Whole => SiteOrder::Provider,
            ProviderGroup::PerRealm => SiteOrder::RealmProvider,
        }
    }

    /// The realm a group belongs to, empty when it spans every realm.
    fn realm(self, realm: &str) -> &str {
        match self {
            ProviderGroup::Whole => "",
            ProviderGroup::PerRealm => name_key(realm),
        }
    }
}

/// Folds the sites `keeps` holds into completed provider groups, ordered by
/// the group's own order, and hands each group's realm, cut provider and
/// whether it reported a cost to `visit`. A provider's sites are contiguous
/// in that order, so the group is complete without a table of every
/// provider; a whole group ORs the flag over every realm.
fn provider_groups(
    source: &dyn FactSource,
    group: ProviderGroup,
    mut visit: impl FnMut(&str, &str, bool),
) {
    let mut current: Option<(String, String, bool)> = None;
    let flush = |group: Option<(String, String, bool)>, visit: &mut dyn FnMut(&str, &str, bool)| {
        if let Some((realm, provider, reported)) = group {
            visit(&realm, &provider, reported);
        }
    };
    source.sites_in(group.order(), &mut |fact| {
        let realm = group.realm(&fact.realm).to_string();
        let provider = name_key(&fact.provider).to_string();
        let reported = fact.reported;
        match &mut current {
            Some((kept_realm, kept_provider, kept_reported))
                if *kept_realm == realm && *kept_provider == provider =>
            {
                *kept_reported |= reported;
            }
            _ => {
                flush(current.take(), &mut visit);
                current = Some((realm, provider, reported));
            }
        }
    });
    flush(current.take(), &mut visit);
}

/// The figure of the harnesses `kept` holds, in key order.
pub(super) fn cost_parts(
    kept: Vec<(String, bool)>,
    usd: Option<f64>,
    providers_more: usize,
) -> CostFigure {
    let named = |reported: bool| -> Vec<String> {
        kept.iter()
            .filter(|(_, did)| *did == reported)
            .map(|(name, _)| name.clone())
            .collect()
    };
    let (reporting, silent) = (named(true), named(false));
    let note = match (reporting.is_empty(), silent.is_empty()) {
        (_, true) => None,
        (true, false) => Some(format!("{} report none", listed(&silent))),
        (false, false) => Some(format!(
            "{} only · {} report none",
            listed(&reporting),
            listed(&silent)
        )),
    };
    let cost = ReportedCost::of(usd);
    CostFigure {
        text: cost.text(),
        cost,
        reporting,
        silent,
        note,
        providers_more,
    }
}

/// A reported cost, `usd`, and the harnesses behind it of the `sites` that
/// ran in what `keeps` holds: those that reported a cost there and those
/// that reported none. The recipe's graph reads it as it did, with every
/// provider, while the readout cuts and bounds them (decision 0089 ruling
/// 7).
pub(crate) fn cost_figure<'a>(
    sites: impl Iterator<Item = &'a Site>,
    keeps: Keeps,
    usd: Option<f64>,
) -> CostFigure {
    let mut harnesses: BTreeMap<String, bool> = BTreeMap::new();
    for site in sites.filter(|site| site.ran(keeps)) {
        *harnesses.entry(site.provider.clone()).or_default() |= site.reported(keeps);
    }
    cost_parts(harnesses.into_iter().collect(), usd, 0)
}

/// The whole read's cost: its spends and provider groups, one at a time.
fn cost(source: &dyn FactSource, window: Window) -> CostFigure {
    let mut accumulator = ProviderAcc::default();
    source.spends(&mut |spend| {
        if window.holds(spend.at) {
            accumulator.spend(spend.usd);
        }
    });
    provider_groups(source, ProviderGroup::Whole, |_, provider, reported| {
        accumulator.provider(provider, reported);
    });
    accumulator.finish()
}

fn per_day(source: &dyn FactSource, window: Window) -> Vec<Day> {
    let first = window.from.div_euclid(DAY_MILLIS);
    let last = (window.to - 1).div_euclid(DAY_MILLIS);
    let mut days: Vec<Day> = (first..=last)
        .map(|day| Day {
            date: date(day),
            shipped: 0,
            stopped: 0,
        })
        .collect();
    finished_in(source, window, |_, finished| {
        let day = finished.at.div_euclid(DAY_MILLIS);
        // `finished_in` holds `finished.at` in `[from, to)`, so the day is
        // one of the rows built above; no bounds guard can be reached.
        // `each_range_holds_its_first_moment_and_not_now` pins both ends.
        let row = &mut days[(day - first) as usize];
        row.shipped += usize::from(shipped(finished));
        row.stopped += usize::from(judged_unshipped(finished.ending));
    });
    days
}

/// One step's wait, folded from the parks in cut phase then run order.
struct StepAcc {
    waiting_millis: i64,
    runs: usize,
    last_run: Option<String>,
}

fn waiting_by_step(source: &dyn FactSource, window: Window) -> (Vec<StepWait>, usize) {
    let mut steps: Ranked<Option<String>, StepAcc, Reverse<i64>> = Ranked::new();
    let mut current: Option<(Option<String>, StepAcc)> = None;
    let flush = |group: Option<(Option<String>, StepAcc)>,
                 steps: &mut Ranked<Option<String>, StepAcc, Reverse<i64>>| {
        if let Some((phase, step)) = group {
            let rank = Reverse(step.waiting_millis);
            steps.push(phase, step, rank);
        }
    };
    source.parks(&mut |park| {
        let waited = window.overlap(park.at, park.answered);
        if waited == 0 {
            return;
        }
        let phase = park.phase.clone();
        match &mut current {
            Some((named, step)) if *named == phase => {
                step.waiting_millis += waited;
                if step.last_run.as_deref() != Some(park.run.as_str()) {
                    step.runs += 1;
                    step.last_run = Some(park.run);
                }
            }
            _ => {
                flush(current.take(), &mut steps);
                current = Some((
                    phase,
                    StepAcc {
                        waiting_millis: waited,
                        runs: 1,
                        last_run: Some(park.run),
                    },
                ));
            }
        }
    });
    flush(current.take(), &mut steps);
    let (kept, waiting_by_step_more) = steps.finish();
    let mut rows: Vec<StepWait> = kept
        .into_iter()
        .map(|(phase, step)| StepWait {
            phase,
            waiting_millis: step.waiting_millis,
            runs: step.runs,
            text: format!("{} · {}", hours_text(step.waiting_millis), step.runs),
        })
        .collect();
    rows.sort_by(|left, right| {
        Reverse(left.waiting_millis)
            .cmp(&Reverse(right.waiting_millis))
            .then(left.phase.cmp(&right.phase))
    });
    (rows, waiting_by_step_more)
}

fn rework(source: &dyn FactSource, window: Window) -> Rework {
    let mut runs = 0;
    let mut visits = 0u64;
    let mut sent_back = 0;
    let mut first_back = 0;
    let mut passed = 0;
    finished_in(source, window, |run, _| {
        runs += 1;
        visits += run.implement_visits;
        sent_back += usize::from(run.reviews.back);
        if run.reviews.first {
            first_back += 1;
            passed += usize::from(run.reviews.second);
        }
    });
    let implement_visits = (runs > 0).then(|| visits as f64 / runs as f64);
    let second_try_percent = percent(passed as i64, first_back as i64);
    Rework {
        runs,
        implement_visits,
        implement_visits_text: implement_visits
            .map_or_else(|| ABSENT.to_string(), |mean| format!("{mean:.1}")),
        sent_back,
        second_try_passed: passed,
        second_try_of: first_back,
        second_try_text: second_try_percent
            .map_or_else(|| ABSENT.to_string(), |share| format!("{share}%")),
        second_try_percent,
    }
}

fn models(source: &dyn FactSource, window: Window) -> (Vec<ModelShare>, usize) {
    let mut models: Ranked<Option<String>, usize, Reverse<usize>> = Ranked::new();
    let mut current: Option<(Option<String>, usize)> = None;
    let mut total = 0;
    let flush = |group: Option<(Option<String>, usize)>,
                 models: &mut Ranked<Option<String>, usize, Reverse<usize>>| {
        if let Some((model, attempts)) = group {
            models.push(model, attempts, Reverse(attempts));
        }
    };
    source.sites_in(SiteOrder::Model, &mut |fact| {
        if !window.holds(fact.at) {
            return;
        }
        let model = fact.model.as_deref().map(model_key).map(str::to_string);
        total += 1;
        match &mut current {
            Some((named, attempts)) if *named == model => *attempts += 1,
            _ => {
                flush(current.take(), &mut models);
                current = Some((model, 1));
            }
        }
    });
    flush(current.take(), &mut models);
    let (kept, models_more) = models.finish();
    let mut shares: Vec<ModelShare> = kept
        .into_iter()
        .map(|(model, attempts)| {
            let percent = percent(attempts as i64, total as i64).unwrap_or(0);
            ModelShare {
                text: format!("{} {percent}%", model.as_deref().unwrap_or("not reported")),
                model,
                attempts,
                percent,
            }
        })
        .collect();
    shares.sort_by(|left, right| {
        Reverse(left.attempts)
            .cmp(&Reverse(right.attempts))
            .then(left.model.cmp(&right.model))
    });
    (shares, models_more)
}

/// A realm row's counts, folded one finished run at a time.
#[derive(Default)]
struct RealmAcc {
    shipped: usize,
    stopped: usize,
    ledger: Tally,
}

fn realm_counts(
    source: &dyn FactSource,
    window: Window,
    realms: &[String],
) -> BTreeMap<String, RealmAcc> {
    let mut counts: BTreeMap<String, RealmAcc> = realms
        .iter()
        .map(|realm| (realm.clone(), RealmAcc::default()))
        .collect();
    finished_in(source, window, |run, finished| {
        if let Some(count) = counts.get_mut(name_key(&run.realm)) {
            count.shipped += usize::from(shipped(finished));
            count.stopped += usize::from(judged_unshipped(finished.ending));
            count.ledger.add(finished.ending);
        }
    });
    counts
}

fn realm_waiting(
    source: &dyn FactSource,
    window: Window,
    realms: &[String],
) -> BTreeMap<String, i64> {
    let mut waiting: BTreeMap<String, i64> =
        realms.iter().map(|realm| (realm.clone(), 0)).collect();
    source.parks(&mut |park| {
        if let Some(total) = waiting.get_mut(name_key(&park.realm)) {
            *total += window.overlap(park.at, park.answered);
        }
    });
    waiting
}

fn realm_costs(
    source: &dyn FactSource,
    window: Window,
    realms: &[String],
) -> BTreeMap<String, ProviderAcc> {
    let mut costs: BTreeMap<String, ProviderAcc> = realms
        .iter()
        .map(|realm| (realm.clone(), ProviderAcc::default()))
        .collect();
    source.spends(&mut |spend| {
        if let Some(cost) = costs.get_mut(name_key(&spend.realm)) {
            if window.holds(spend.at) {
                cost.spend(spend.usd);
            }
        }
    });
    provider_groups(
        source,
        ProviderGroup::PerRealm,
        |realm, provider, reported| {
            if let Some(cost) = costs.get_mut(realm) {
                cost.provider(provider, reported);
            }
        },
    );
    costs
}

fn realm_rows(
    source: &dyn FactSource,
    window: Window,
    realms: &[String],
    medians: &[Option<i64>],
) -> Vec<RealmRow> {
    let mut counts = realm_counts(source, window, realms);
    let mut waiting = realm_waiting(source, window, realms);
    let mut costs = realm_costs(source, window, realms);
    realms
        .iter()
        .enumerate()
        .map(|(position, realm)| {
            let count = counts.remove(realm).unwrap_or_default();
            let waiting_millis = waiting.remove(realm).unwrap_or(0);
            let cost = costs.remove(realm).unwrap_or_default().finish();
            RealmRow {
                realm: realm.clone(),
                shipped: count.shipped,
                stopped: count.stopped,
                ledger: count.ledger.finish(),
                median: span(medians[position + 1]),
                waiting_millis,
                waiting_text: hours_text(waiting_millis),
                cost_text: cost.text,
                cost: cost.cost,
                unreadable_runs: source.scope().1.get(realm).copied().unwrap_or(0),
            }
        })
        .collect()
}

/// The analytics readout for `query`: every figure the console paints,
/// derived once from the hearths' journals, for the range ending at `now`.
pub fn analytics(query: &Query) -> Result<Analytics, AnalyticsError> {
    let (window, _) = super::windows(query)?;
    let source = super::whole_read(query, window);
    analytics_with(&source, query)
}

/// The analytics readout for `query` over `source`: today's whole read, or
/// the store's rows merged with the runs it reads whole (decision 0089
/// ruling 8).
pub fn analytics_with(source: &dyn FactSource, query: &Query) -> Result<Analytics, AnalyticsError> {
    let (window, before) = super::windows(query)?;
    // Offered in discovery order, which is the order `Ranked` keeps and
    // the rows are painted in: the first 64 realms, the rest counted.
    let mut kept_realms: Ranked<String, (), ()> = Ranked::new();
    let (named, _) = source.scope();
    for realm in named {
        kept_realms.push(realm.clone(), (), ());
    }
    let (kept, realms_more) = kept_realms.finish();
    let realms: Vec<String> = kept.into_iter().map(|(realm, ())| realm).collect();
    let medians = medians_of(source, &realms);
    let shipped = shipped_in(source, window);
    let previous = shipped_in(source, before);
    let change = shipped as i64 - previous as i64;
    let (models, models_more) = models(source, window);
    let (waiting_by_step, waiting_by_step_more) = waiting_by_step(source, window);
    let (recipes, recipes_more) = recipe_rows(source, window);
    Ok(Analytics {
        view_version: VIEW_VERSION,
        range: query.range,
        realm: query.realm.map(str::to_string),
        from: stamp(window.from),
        to: query.now.to_string(),
        headline: Headline {
            shipped: Shipped {
                count: shipped,
                previous,
                change,
                change_text: change_text(change, query.range),
            },
            median: span(medians[0]),
            waiting: waiting(source, window),
            stopped_by_rule: stopped_by_rule(source, window),
            cost: cost(source, window),
            ledger: ledger_in(source, window),
        },
        per_day: per_day(source, window),
        waiting_by_step,
        rework: rework(source, window),
        models,
        realms: realm_rows(source, window, &realms, &medians),
        recipes,
        quarantined: source.quarantined(),
        not_recorded: NOT_RECORDED,
        models_more,
        waiting_by_step_more,
        realms_more,
        recipes_more,
    })
}
