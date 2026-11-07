//! One figure per realm and recipe (#568, the Library's design D6): how
//! often its runs reach `done`, what a run costs and spends in tokens, and
//! whether another recipe of the realm beats it on both. The console plots
//! these and judges only the quadrant a point falls in; each row is folded
//! from the facts one at a time and the figure keeps 64, by cut key.

use std::collections::BTreeMap;

use super::facts::{in_range, name_key, FactSource, Window};
use super::figures::cost_figure_of;
use super::ledger::Tally;
use super::ranked::ranked;
use super::{RecipeRow, ReportedCost};
use crate::{finish_usage, js, TokenUsage};

/// One recipe's accumulator: its runs, their endings, the cost they
/// reported, the harnesses behind it and the tokens their sites metered.
#[derive(Default)]
struct RecipeAcc {
    runs: usize,
    ledger: Tally,
    spent: Option<f64>,
    harnesses: BTreeMap<String, bool>,
    usages: Vec<TokenUsage>,
}

/// The recipes' rows, by realm and then recipe, over the runs the range
/// keeps; a run whose start names no recipe is in none. The pairs are fed
/// in key order, and the 64 that key order ranks first are kept.
pub(super) fn recipe_rows(source: &dyn FactSource, window: Window) -> (Vec<RecipeRow>, usize) {
    let mut groups: BTreeMap<(String, String), RecipeAcc> = BTreeMap::new();
    source.runs(&mut |run| {
        let Some(recipe) = run.recipe.as_deref() else {
            return;
        };
        let ended = run.finished.as_ref().map(|finished| finished.at);
        if !in_range(ended, run.open, window) {
            return;
        }
        let group = groups.entry(key(&run.realm, recipe)).or_default();
        group.runs += 1;
        if let Some(finished) = &run.finished {
            group.ledger.add(finished.ending);
        }
    });
    let keeps = |at: i64, _: Option<&str>| window.holds(at);
    source.spends(&mut |spend| {
        let Some(recipe) = spend.recipe else {
            return;
        };
        if !in_range(spend.ended, spend.open, window) || !window.holds(spend.at) {
            return;
        }
        let group = groups
            .get_mut(&key(spend.realm, recipe))
            .expect("the run that spent opened its recipe's group");
        group.spent = Some(group.spent.unwrap_or(0.0) + spend.usd);
    });
    source.sites(&mut |fact| {
        let Some(recipe) = fact.recipe else {
            return;
        };
        if !in_range(fact.ended, fact.open, window) {
            return;
        }
        let group = groups
            .get_mut(&key(fact.realm, recipe))
            .expect("the run that opened the site opened its recipe's group");
        if fact.site.ran(&keeps) {
            let provider = name_key(&fact.site.provider).to_string();
            *group.harnesses.entry(provider).or_default() |= fact.site.reported(&keeps);
        }
        if let Some(usage) = fact.site.usage(&keeps) {
            group.usages.push(usage);
        }
    });
    let (kept, recipes_more) = ranked(groups, |_| ());
    let mut rows: Vec<RecipeRow> = kept
        .into_iter()
        .map(|((realm, recipe), group)| row(realm, recipe, group))
        .collect();
    // A recipe none of whose runs was judged has no rate (decision 0064
    // ruling 7), so it is ranked against nothing and dominates nothing.
    let points: Vec<Point> = rows
        .iter()
        .map(|row| {
            (
                row.realm.as_str(),
                row.ledger.rate_percent.map(|_| row.done_rate),
                usd(&row.cost_per_run.cost),
            )
        })
        .collect();
    let frontier: Vec<Option<bool>> = points
        .iter()
        .map(|point| on_frontier(*point, &points))
        .collect();
    for (row, on) in rows.iter_mut().zip(frontier) {
        row.on_frontier = on;
    }
    (rows, recipes_more)
}

/// The cut key a recipe's runs group by.
fn key(realm: &str, recipe: &str) -> (String, String) {
    (name_key(realm).to_string(), name_key(recipe).to_string())
}

fn usd(cost: &ReportedCost) -> Option<f64> {
    match cost {
        ReportedCost::Reported { usd } => Some(*usd),
        ReportedCost::NotReported => None,
    }
}

/// A recipe as the frontier ranks it: its realm, its done rate where any of
/// its runs was judged, and its cost where one was reported.
type Point<'a> = (&'a str, Option<f64>, Option<f64>);

/// Whether no other rated, priced recipe of the realm has a done rate at
/// least as high and a cost at most as low, one of the two strictly better;
/// absent for a recipe with no rate or no reported cost. A recipe never
/// dominates itself, and an equal one does not dominate either, so a tie
/// keeps both.
fn on_frontier(point: Point, points: &[Point]) -> Option<bool> {
    let (realm, rate, cost) = point;
    let (rate, cost) = (rate?, cost?);
    let dominated = points.iter().any(|(other_realm, other_rate, other_cost)| {
        let Some((other_rate, other_cost)) = other_rate.zip(*other_cost) else {
            return false;
        };
        *other_realm == realm
            && other_rate >= rate
            && other_cost <= cost
            && (other_rate > rate || other_cost < cost)
    });
    Some(!dominated)
}

fn row(realm: String, recipe: String, group: RecipeAcc) -> RecipeRow {
    let RecipeAcc {
        runs,
        ledger,
        spent,
        harnesses,
        usages,
    } = group;
    let count = runs as f64;
    let ledger = ledger.finish();
    // The rate counts the outcomes of the work alone (decision 0064 ruling
    // 7); a recipe none of whose runs was judged reaches none.
    let (done, judged) = (ledger.shipped, ledger.judged);
    let done_rate = match judged {
        0 => 0.0,
        judged => done as f64 * 100.0 / judged as f64,
    };
    // The tokens a run spent: each kind summed over the range, then
    // averaged over its runs, so the cache hit is the share it read.
    let sum = crate::meter::sum_usage(usages.into_iter()).unwrap_or_default();
    let per_run =
        |tokens: Option<u64>| tokens.map(|tokens| js::round_half_up(tokens as f64 / count) as u64);
    let averaged = TokenUsage {
        input_tokens: per_run(sum.input_tokens),
        output_tokens: per_run(sum.output_tokens),
        cache_read_tokens: per_run(sum.cache_read_tokens),
        cache_write_tokens: per_run(sum.cache_write_tokens),
        ..TokenUsage::default()
    };
    RecipeRow {
        realm,
        recipe,
        runs,
        done,
        done_rate,
        done_text: ledger.rate_percent.map_or_else(
            || crate::ABSENT.to_string(),
            |_| format!("{}%", js::round_half_up(done_rate)),
        ),
        ledger,
        // A cost, a token and a provider's reporting are kept by the time
        // of the checkpoint that metered them, never by the attempt's
        // start.
        cost_per_run: cost_figure_of(harnesses, spent.map(|usd| usd / count)),
        tokens_per_run: crate::tokens::usage_cells(finish_usage(averaged).as_ref(), "").1,
        on_frontier: None,
    }
}
