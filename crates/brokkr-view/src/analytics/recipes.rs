//! One figure per realm and recipe (#568, the Library's design D6): how
//! often its runs reach `done`, what a run costs and spends in tokens, and
//! whether another recipe of the realm beats it on both. The console plots
//! these and judges only the quadrant a point falls in.

use std::collections::BTreeMap;

use super::facts::{in_range, RunFacts, Window};
use super::figures::{cost_figure, total};
use super::{Ledger, RecipeRow, ReportedCost};
use crate::{finish_usage, js, TokenUsage};

/// The recipes' rows, by realm and then recipe, over the runs the range
/// keeps; a run whose start names no recipe is in none.
pub(super) fn recipe_rows(runs: &[RunFacts], window: Window) -> Vec<RecipeRow> {
    let mut groups: BTreeMap<(&str, &str), Vec<&RunFacts>> = BTreeMap::new();
    for run in runs {
        let ended = run.finished.as_ref().map(|finished| finished.at);
        if let (Some(recipe), true) = (&run.recipe, in_range(ended, run.open, window)) {
            groups.entry((&run.realm, recipe)).or_default().push(run);
        }
    }
    let mut rows: Vec<RecipeRow> = groups
        .into_iter()
        .map(|((realm, recipe), runs)| row(realm, recipe, &runs, window))
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
    rows
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

fn row(realm: &str, recipe: &str, runs: &[&RunFacts], window: Window) -> RecipeRow {
    let count = runs.len() as f64;
    let finished = runs.iter().filter_map(|run| run.finished.as_ref());
    let ledger = Ledger::of(finished.map(|finished| finished.ending));
    // The rate counts the outcomes of the work alone (decision 0064 ruling
    // 7); a recipe none of whose runs was judged reaches none.
    let (done, judged) = (ledger.shipped, ledger.judged);
    let done_rate = match judged {
        0 => 0.0,
        judged => done as f64 * 100.0 / judged as f64,
    };
    let sites = runs.iter().flat_map(|run| &run.sites);
    // A cost, a token and a provider's reporting are kept by the time of
    // the checkpoint that metered them, never by the attempt's start.
    let keeps = |at: i64, _: Option<&str>| window.holds(at);
    let costs = runs.iter().flat_map(|run| &run.costs);
    let spent = total(
        costs
            .filter(|(at, _)| window.holds(*at))
            .map(|(_, usd)| *usd),
    );
    // The tokens a run spent: each kind summed over the range, then
    // averaged over its runs, so the cache hit is the share it read.
    let usages = sites.clone().filter_map(|site| site.usage(&keeps));
    let sum = crate::meter::sum_usage(usages).unwrap_or_default();
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
        realm: realm.to_string(),
        recipe: recipe.to_string(),
        runs: runs.len(),
        done,
        done_rate,
        done_text: ledger.rate_percent.map_or_else(
            || crate::ABSENT.to_string(),
            |_| format!("{}%", js::round_half_up(done_rate)),
        ),
        ledger,
        cost_per_run: cost_figure(sites, &keeps, spent.map(|usd| usd / count)),
        tokens_per_run: crate::tokens::usage_cells(finish_usage(averaged).as_ref(), "").1,
        on_frontier: None,
    }
}
