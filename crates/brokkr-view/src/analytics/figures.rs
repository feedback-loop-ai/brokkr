//! Every figure of the readout, aggregated from the runs' facts over the
//! range's window.

use std::collections::{BTreeMap, BTreeSet};

use super::facts::{Ending, Route, RunFacts, Scoped, Window};
use super::words::{change_text, date, hours_text, listed, percent, span_text, stamp};
use super::{Analytics, AnalyticsError, CostFigure, Day, Headline, ModelShare, Query, RealmRow};
use super::{ReportedCost, Rework, RuleCount, Shipped, Span, StepWait, StoppedByRule, Waiting};
use super::{DAY_MILLIS, NOT_RECORDED};
use crate::{js, ABSENT, VIEW_VERSION};

/// The runs that finished inside `window`, with when and how.
fn finished_in<'a>(
    runs: &'a [&'a RunFacts],
    window: Window,
) -> impl Iterator<Item = (i64, i64, &'a Ending)> + 'a {
    runs.iter().filter_map(move |run| {
        let (ended, ending) = run.finished.as_ref()?;
        window
            .holds(*ended)
            .then_some((run.started.unwrap_or(*ended), *ended, ending))
    })
}

fn shipped_in(runs: &[&RunFacts], window: Window) -> usize {
    finished_in(runs, window)
        .filter(|(_, _, ending)| **ending == Ending::Shipped)
        .count()
}

fn stopped_in(runs: &[&RunFacts], window: Window) -> usize {
    finished_in(runs, window)
        .filter(|(_, _, ending)| **ending != Ending::Shipped)
        .count()
}

fn median(runs: &[&RunFacts], window: Window) -> Span {
    let mut spans: Vec<i64> = finished_in(runs, window)
        .map(|(started, ended, _)| (ended - started).max(0))
        .collect();
    spans.sort_unstable();
    let middle = spans.len() / 2;
    let millis = if spans.is_empty() {
        None
    } else if spans.len() % 2 == 1 {
        Some(spans[middle])
    } else {
        Some((spans[middle - 1] + spans[middle]) / 2)
    };
    Span {
        text: millis.map_or_else(|| ABSENT.to_string(), span_text),
        millis,
    }
}

/// Waiting on the operator and all parked time, inside `window`.
fn waited(runs: &[&RunFacts], window: Window) -> (i64, i64) {
    let parks = runs.iter().flat_map(|run| &run.parks);
    parks.fold((0, 0), |(waiting, parked), park| {
        (
            waiting + window.overlap(park.at, park.answered),
            parked + window.overlap(park.at, park.ended),
        )
    })
}

fn waiting(runs: &[&RunFacts], window: Window) -> Waiting {
    let (waiting_millis, parked_millis) = waited(runs, window);
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

fn stopped_by_rule(runs: &[&RunFacts], window: Window) -> StoppedByRule {
    let mut count = 0;
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, ending) in finished_in(runs, window) {
        if let Ending::ByRule(rule) = ending {
            count += 1;
            *tally.entry(rule.as_deref().unwrap_or(ABSENT)).or_default() += 1;
        }
    }
    let top = tally
        .into_iter()
        .fold(None::<RuleCount>, |best, (rule_id, count)| match best {
            Some(best) if best.count >= count => Some(best),
            Some(_) | None => Some(RuleCount {
                rule_id: rule_id.to_string(),
                count,
            }),
        });
    StoppedByRule {
        count,
        text: top.as_ref().map_or_else(String::new, |top| {
            format!("{} by {}", top.count, top.rule_id)
        }),
        top,
    }
}

fn total(costs: impl Iterator<Item = f64>) -> Option<f64> {
    costs.fold(None, |sum, cost| Some(sum.unwrap_or(0.0) + cost))
}

fn spent(runs: &[&RunFacts], window: Window) -> ReportedCost {
    let costs = runs.iter().flat_map(|run| &run.costs);
    ReportedCost::of(total(
        costs
            .filter(|(at, _)| window.holds(*at))
            .map(|(_, cost)| *cost),
    ))
}

fn cost(runs: &[&RunFacts], window: Window) -> CostFigure {
    let mut harnesses: BTreeMap<&str, bool> = BTreeMap::new();
    let sites = runs.iter().flat_map(|run| &run.sites);
    for site in sites.filter(|site| window.holds(site.at)) {
        *harnesses.entry(&site.provider).or_default() |= site.reported;
    }
    let named = |reported: bool| -> Vec<String> {
        let held = harnesses.iter().filter(|(_, did)| **did == reported);
        held.map(|(name, _)| name.to_string()).collect()
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
    let cost = spent(runs, window);
    CostFigure {
        text: cost.text(),
        cost,
        reporting,
        silent,
        note,
    }
}

fn per_day(runs: &[&RunFacts], window: Window) -> Vec<Day> {
    let first = window.from.div_euclid(DAY_MILLIS);
    let last = (window.to - 1).div_euclid(DAY_MILLIS);
    (first..=last)
        .map(|day| {
            let on = |ended: i64| ended.div_euclid(DAY_MILLIS) == day;
            let finished = finished_in(runs, window).filter(|(_, ended, _)| on(*ended));
            let (shipped, stopped) =
                finished.fold((0, 0), |(shipped, stopped), (_, _, ending)| match ending {
                    Ending::Shipped => (shipped + 1, stopped),
                    Ending::ByRule(_) | Ending::ByOperator => (shipped, stopped + 1),
                });
            Day {
                date: date(day),
                shipped,
                stopped,
            }
        })
        .collect()
}

fn waiting_by_step(runs: &[&RunFacts], window: Window) -> Vec<StepWait> {
    let mut steps: BTreeMap<Option<&str>, (i64, usize)> = BTreeMap::new();
    for run in runs {
        let mut seen: BTreeSet<Option<&str>> = BTreeSet::new();
        for park in &run.parks {
            let waited = window.overlap(park.at, park.answered);
            if waited > 0 {
                let step = steps.entry(park.phase.as_deref()).or_default();
                step.0 += waited;
                step.1 += usize::from(seen.insert(park.phase.as_deref()));
            }
        }
    }
    let mut rows: Vec<StepWait> = steps
        .into_iter()
        .map(|(phase, (waiting_millis, runs))| StepWait {
            phase: phase.map(str::to_string),
            waiting_millis,
            runs,
            text: format!("{} · {runs}", hours_text(waiting_millis)),
        })
        .collect();
    rows.sort_by_key(|row| -row.waiting_millis);
    rows
}

fn rework(runs: &[&RunFacts], window: Window) -> Rework {
    let finished: Vec<&RunFacts> = runs
        .iter()
        .copied()
        .filter(|run| {
            run.finished
                .as_ref()
                .is_some_and(|(ended, _)| window.holds(*ended))
        })
        .collect();
    let visits: u64 = finished.iter().map(|run| run.implement_visits).sum();
    let implement_visits = (!finished.is_empty()).then(|| visits as f64 / finished.len() as f64);
    let sent_back = finished
        .iter()
        .filter(|run| run.reviews.contains(&Route::Back))
        .count();
    let first_back: Vec<&&RunFacts> = finished
        .iter()
        .filter(|run| run.reviews.first() == Some(&Route::Back))
        .collect();
    let passed = first_back
        .iter()
        .filter(|run| run.reviews.get(1) == Some(&Route::On))
        .count();
    let second_try_percent = percent(passed as i64, first_back.len() as i64);
    Rework {
        runs: finished.len(),
        implement_visits,
        implement_visits_text: implement_visits
            .map_or_else(|| ABSENT.to_string(), |mean| format!("{mean:.1}")),
        sent_back,
        second_try_passed: passed,
        second_try_of: first_back.len(),
        second_try_text: second_try_percent
            .map_or_else(|| ABSENT.to_string(), |share| format!("{share}%")),
        second_try_percent,
    }
}

fn models(runs: &[&RunFacts], window: Window) -> Vec<ModelShare> {
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    let sites = runs.iter().flat_map(|run| &run.sites);
    for site in sites.filter(|site| window.holds(site.at)) {
        *tally
            .entry(site.model.as_deref().unwrap_or("not reported"))
            .or_default() += 1;
    }
    let total: usize = tally.values().sum();
    let mut shares: Vec<ModelShare> = tally
        .into_iter()
        .map(|(model, attempts)| {
            let percent = percent(attempts as i64, total as i64).unwrap_or(0);
            ModelShare {
                text: format!("{model} {percent}%"),
                model: model.to_string(),
                attempts,
                percent,
            }
        })
        .collect();
    shares.sort_by_key(|share| -(share.attempts as i64));
    shares
}

fn realm_row(scoped: &Scoped, realm: &str, window: Window) -> RealmRow {
    let runs: Vec<&RunFacts> = scoped
        .runs
        .iter()
        .filter(|run| run.realm == realm)
        .collect();
    let (waiting_millis, _) = waited(&runs, window);
    let cost = spent(&runs, window);
    RealmRow {
        realm: realm.to_string(),
        shipped: shipped_in(&runs, window),
        stopped: stopped_in(&runs, window),
        median: median(&runs, window),
        waiting_millis,
        waiting_text: hours_text(waiting_millis),
        cost_text: cost.text(),
        cost,
        unreadable_runs: scoped.unreadable.get(realm).copied().unwrap_or(0),
    }
}

/// The analytics readout for `query`: every figure the console paints,
/// derived once from the hearths' journals, for the range ending at `now`.
pub fn analytics(query: &Query) -> Result<Analytics, AnalyticsError> {
    let now =
        js::parse_millis(query.now).ok_or_else(|| AnalyticsError::Clock(query.now.to_string()))?;
    let span = query.range.millis();
    let window = Window {
        from: now - span,
        to: now,
    };
    let before = Window {
        from: window.from - span,
        to: window.from,
    };
    let scoped = Scoped::read(query, now);
    let runs: Vec<&RunFacts> = scoped.runs.iter().collect();
    let shipped = shipped_in(&runs, window);
    let previous = shipped_in(&runs, before);
    let change = shipped as i64 - previous as i64;
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
            median: median(&runs, window),
            waiting: waiting(&runs, window),
            stopped_by_rule: stopped_by_rule(&runs, window),
            cost: cost(&runs, window),
        },
        per_day: per_day(&runs, window),
        waiting_by_step: waiting_by_step(&runs, window),
        rework: rework(&runs, window),
        models: models(&runs, window),
        realms: scoped
            .realms
            .iter()
            .map(|realm| realm_row(&scoped, realm, window))
            .collect(),
        quarantined: scoped.quarantined,
        not_recorded: NOT_RECORDED,
    })
}
