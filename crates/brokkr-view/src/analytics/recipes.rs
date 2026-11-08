//! One figure per realm and recipe (#568, the Library's design D6): how
//! often its runs reach `done`, what a run costs and spends in tokens, and
//! whether another recipe of the realm beats it on both. The console plots
//! these and judges only the quadrant a point falls in; each row is folded
//! from the facts one at a time and the figure keeps 64, by cut key.

use std::collections::BTreeMap;

use super::facts::{in_range, name_key, FactSource, RunOrder, SiteOrder, Window};
use super::figures::cost_parts;
use super::ledger::Tally;
use super::ranked::Ranked;
use super::RecipeRow;
use crate::meter::sum_usage;
use crate::{finish_usage, js, TokenUsage};

/// One recipe's accumulator: its runs, their endings, the cost they
/// reported, the tokens their sites metered and the providers behind them,
/// each bounded as the figure keeps it.
#[derive(Default)]
struct Retained {
    runs: usize,
    ledger: Tally,
    spent: Option<f64>,
    providers: Ranked<String, bool, ()>,
    usage: Option<TokenUsage>,
    /// The recipe's done rate where any run was judged, and its cost per
    /// run where any reported one; the frontier reads these as it folds.
    rate: Option<f64>,
    cost: Option<f64>,
    /// Whether a recipe of the same realm dominates this one. Every group
    /// updates it, kept or not (decision 0089 ruling 7).
    dominated: bool,
}

/// The recipes' rows, by realm and then recipe, over the runs the range
/// keeps; a run whose start names no recipe is in none. The first 64 cut
/// keys are kept, and every group, kept or not, decides the frontier of
/// the kept rows of its realm.
pub(super) fn recipe_rows(source: &dyn FactSource, window: Window) -> (Vec<RecipeRow>, usize) {
    let mut recipes: Ranked<(String, String), Retained, ()> = Ranked::new();
    let mut current: Option<((String, String), Retained)> = None;
    let offer = |group: Option<((String, String), Retained)>,
                 recipes: &mut Ranked<(String, String), Retained, ()>| {
        if let Some((key, mut group)) = group {
            let point = point(&group);
            group.rate = point.0;
            group.cost = point.1;
            recipes.for_each_mut(|kept_key, kept| {
                if kept_key.0 != key.0 {
                    return;
                }
                let kept_point = (kept.rate, kept.cost);
                if dominates(point, kept_point) {
                    kept.dominated = true;
                }
                if dominates(kept_point, point) {
                    group.dominated = true;
                }
            });
            recipes.push(key, group, ());
        }
    };
    source.runs_in(RunOrder::Recipe, &mut |run| {
        let Some(recipe) = run.recipe.as_deref() else {
            return;
        };
        let key = (
            name_key(&run.realm).to_string(),
            name_key(recipe).to_string(),
        );
        if current.as_ref().is_none_or(|(named, _)| *named != key) {
            offer(current.take(), &mut recipes);
            current = None;
        }
        let ended = run.finished.as_ref().map(|finished| finished.at);
        if !in_range(ended, run.open, window) {
            return;
        }
        let (_, group) = current.get_or_insert_with(|| (key, Retained::default()));
        group.runs += 1;
        if let Some(finished) = &run.finished {
            group.ledger.add(finished.ending);
        }
        for (at, usd) in &run.costs {
            if window.holds(*at) {
                group.spent = Some(group.spent.unwrap_or(0.0) + usd);
            }
        }
    });
    offer(current.take(), &mut recipes);
    let (mut kept, recipes_more) = recipes.finish();

    let index: BTreeMap<(String, String), usize> = kept
        .iter()
        .enumerate()
        .map(|(position, (key, _))| (key.clone(), position))
        .collect();
    add_sites(source, window, &mut kept, &index);

    let rows = kept
        .into_iter()
        .map(|((realm, recipe), group)| row(realm, recipe, group))
        .collect();
    (rows, recipes_more)
}

/// Adds the sites of the kept recipes: provider groups are complete in the
/// source's order, so each is offered once, and the tokens are summed into
/// one total per recipe rather than held per site.
fn add_sites(
    source: &dyn FactSource,
    window: Window,
    kept: &mut Vec<((String, String), Retained)>,
    index: &BTreeMap<(String, String), usize>,
) {
    let keeps = |at: i64, _: Option<&str>| window.holds(at);
    let mut provider: Option<((String, String, String), bool)> = None;
    let flush = |group: Option<((String, String, String), bool)>,
                 kept: &mut Vec<((String, String), Retained)>| {
        if let Some(((realm, recipe, name), reported)) = group {
            if let Some(position) = index.get(&(realm, recipe)) {
                kept[*position].1.providers.push(name, reported, ());
            }
        }
    };
    source.sites_in(SiteOrder::RecipeProvider, &mut |fact| {
        let Some(recipe) = fact.recipe else {
            return;
        };
        if !in_range(fact.ended, fact.open, window) {
            return;
        }
        let realm = name_key(fact.realm).to_string();
        let recipe = name_key(recipe).to_string();
        let provider_name = name_key(&fact.site.provider).to_string();
        if let Some(position) = index.get(&(realm.clone(), recipe.clone())) {
            if let Some(usage) = fact.site.usage(&keeps) {
                let group = &mut kept[*position].1;
                group.usage = sum_usage([group.usage.take(), Some(usage)].into_iter().flatten());
            }
        }
        if !fact.site.ran(&keeps) {
            return;
        }
        let reported = fact.site.reported(&keeps);
        let same = matches!(
            &provider,
            Some(((kept_realm, kept_recipe, kept_name), _))
                if *kept_realm == realm && *kept_recipe == recipe && *kept_name == provider_name
        );
        if same {
            if let Some((_, kept_reported)) = &mut provider {
                *kept_reported |= reported;
            }
        } else {
            flush(provider.take(), kept);
            provider = Some(((realm, recipe, provider_name), reported));
        }
    });
    flush(provider.take(), kept);
}

/// A recipe as the frontier ranks it: its done rate where any of its runs
/// was judged, and its cost where one was reported.
type Point = (Option<f64>, Option<f64>);

/// The point a recipe's counts make before its row is built.
fn point(group: &Retained) -> Point {
    let (shipped, judged) = group.ledger.counted();
    let rate = (judged > 0).then(|| shipped as f64 * 100.0 / judged as f64);
    let cost = group.spent.map(|spent| spent / group.runs as f64);
    (rate, cost)
}

/// Whether `left` dominates `right`: at least as high a done rate for at
/// most as low a cost, one strictly. A recipe with no rate or no reported
/// cost is off the plot, so it neither dominates nor is dominated.
fn dominates(left: Point, right: Point) -> bool {
    let (Some(left_rate), Some(left_cost)) = left else {
        return false;
    };
    let (Some(right_rate), Some(right_cost)) = right else {
        return false;
    };
    left_rate >= right_rate
        && left_cost <= right_cost
        && (left_rate > right_rate || left_cost < right_cost)
}

fn row(realm: String, recipe: String, group: Retained) -> RecipeRow {
    let Retained {
        runs,
        ledger,
        spent,
        providers,
        usage,
        rate,
        cost,
        dominated,
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
    let sum = usage.unwrap_or_default();
    let per_run =
        |tokens: Option<u64>| tokens.map(|tokens| js::round_half_up(tokens as f64 / count) as u64);
    let averaged = TokenUsage {
        input_tokens: per_run(sum.input_tokens),
        output_tokens: per_run(sum.output_tokens),
        cache_read_tokens: per_run(sum.cache_read_tokens),
        cache_write_tokens: per_run(sum.cache_write_tokens),
        ..TokenUsage::default()
    };
    let (providers, providers_more) = providers.finish();
    let on_frontier = match (rate, cost) {
        (Some(_), Some(_)) => Some(!dominated),
        _ => None,
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
        cost_per_run: cost_parts(providers, spent.map(|usd| usd / count), providers_more),
        tokens_per_run: crate::tokens::usage_cells(finish_usage(averaged).as_ref(), "").1,
        on_frontier,
    }
}
