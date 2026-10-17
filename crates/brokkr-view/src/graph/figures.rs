//! Every figure of the graph, from the runs' facts over the range's
//! window: its states and where they sit, its rules with their counts and
//! kinds, the drift of each run's table, and how the runs ended.

use std::collections::{BTreeMap, BTreeSet};

use super::facts::{GraphRun, Ruled, Scoped};
use super::{Edge, Ends, GraphError, GraphQuery, GraphTable, GraphView, LiveRun, NeverFired};
use super::{Link, Outcome, Place, RuleKind, SeatPin, State, Table, GRAPH_VIEW_VERSION};
use crate::analytics::{cost_figure, middle, span, stamp, total, Ending, RuleCount, Window};
use crate::clarify::{pins_of, Pins};
use crate::fleet::STOP_PHASE;
use crate::{add_token, finish_usage, js, RuleSeverity, TokenUsage};

/// The pinned file the table is written in, and the prefix a composed
/// recipe pins each base layer under — `brokkr_runtime`'s
/// `COMPOSE_PREFIX`, held to it by the CLI's test of a composed recipe.
const POLICY_FILE: &str = "policy.json";
const COMPOSE_PREFIX: &str = "@compose/";

/// The states in order and which of them are terminal.
struct Shape {
    phases: Vec<String>,
    terminal: BTreeSet<String>,
}

impl Shape {
    /// The table's, its initial state first and the rest in `phases` order
    /// (a composed recipe can list the state it starts in last), or — when
    /// it was not found — what the runs entered and ruled, in the order they
    /// first did, the states their ends left them in being the terminals.
    fn of(table: Option<GraphTable>, runs: &[GraphRun]) -> Shape {
        if let Some(table) = table {
            let machine = table.machine;
            let initial = machine
                .phases
                .iter()
                .filter(|phase| **phase == machine.initial);
            let rest = machine
                .phases
                .iter()
                .filter(|phase| **phase != machine.initial);
            return Shape {
                phases: initial.chain(rest).cloned().collect(),
                terminal: machine.terminal.iter().cloned().collect(),
            };
        }
        let mut phases: Vec<String> = Vec::new();
        let mut terminal = BTreeSet::new();
        for run in runs {
            let ruled = run
                .rulings
                .iter()
                .flat_map(|ruled| std::iter::once(&ruled.from).chain(ruled.next.as_ref()));
            let entered = run.stays.iter().map(|stay| &stay.phase);
            for phase in entered.chain(ruled) {
                if !phases.contains(phase) {
                    phases.push(phase.clone());
                }
            }
            if let (Some((_, Ending::Shipped | Ending::ByRule(_))), Some(state)) =
                (&run.finished, &run.state)
            {
                terminal.insert(state.clone());
            }
        }
        Shape { phases, terminal }
    }

    fn rank(&self, phase: &str) -> Option<usize> {
        self.phases.iter().position(|known| known == phase)
    }

    /// `stop` and every terminal but the last that completes a run sit
    /// beneath the row; that last one ends it (design D3).
    fn place(&self, phase: &str) -> Place {
        let mut completes = self
            .phases
            .iter()
            .filter(|known| self.terminal.contains(*known) && *known != STOP_PHASE);
        match (self.terminal.contains(phase), completes.next_back()) {
            (false, _) => Place::Row,
            (true, Some(last)) if last == phase => Place::End,
            (true, _) => Place::Beneath,
        }
    }

    fn kind(&self, from: &str, next: Option<&str>) -> RuleKind {
        match next {
            None => RuleKind::SelfLoop,
            Some(next) if self.terminal.contains(next) => RuleKind::Terminal,
            Some(next) if next == from => RuleKind::SelfLoop,
            Some(next) if self.rank(next) < self.rank(from) => RuleKind::Back,
            Some(_) => RuleKind::Forward,
        }
    }
}

/// Every ruling in the range, by rule id in the order each first fired,
/// with its count.
fn fired(runs: &[GraphRun], window: Window) -> Vec<(&Ruled, usize)> {
    let mut fired: Vec<(&Ruled, usize)> = Vec::new();
    let rulings = runs.iter().flat_map(|run| &run.rulings);
    for ruled in rulings.filter(|ruled| window.holds(ruled.at)) {
        match fired
            .iter_mut()
            .find(|(seen, _)| seen.rule_id == ruled.rule_id)
        {
            Some((_, count)) => *count += 1,
            None => fired.push((ruled, 1)),
        }
    }
    fired
}

fn edge(shape: &Shape, id: &str, from: &str, next: Option<&str>, count: usize) -> Edge {
    Edge {
        id: id.to_string(),
        from: from.to_string(),
        next: next.map(str::to_string),
        result: String::new(),
        when: Vec::new(),
        severity: None,
        count,
        kind: shape.kind(from, next),
        text: format!("{id} · {count}"),
    }
}

/// The table's rules with their counts, or the fired ones alone when the
/// table was not found; and the fired rules the table no longer holds.
fn rules(
    table: Option<GraphTable>,
    shape: &Shape,
    fired: &[(&Ruled, usize)],
) -> (Vec<Edge>, Vec<RuleCount>) {
    let Some(table) = table else {
        let edges = fired.iter().map(|(ruled, count)| Edge {
            result: ruled.result.clone(),
            severity: ruled.severity,
            ..edge(
                shape,
                &ruled.rule_id,
                &ruled.from,
                ruled.next.as_deref(),
                *count,
            )
        });
        return (edges.collect(), Vec::new());
    };
    let count = |id: &str| fired.iter().find(|(ruled, _)| ruled.rule_id == id);
    let edges = table.machine.rules.iter().map(|rule| Edge {
        result: rule.result.clone(),
        when: rule.when_words(),
        severity: rule.next.as_ref().and(RuleSeverity::named(&rule.severity)),
        ..edge(
            shape,
            &rule.id,
            &rule.from,
            rule.next.as_deref(),
            count(&rule.id).map_or(0, |(_, count)| *count),
        )
    });
    let held = |id: &str| table.machine.rules.iter().any(|rule| rule.id == id);
    let retired = fired.iter().filter(|(ruled, _)| !held(&ruled.rule_id));
    let retired = retired.map(|(ruled, count)| RuleCount {
        rule_id: Some(ruled.rule_id.clone()),
        count: *count,
    });
    (edges.collect(), retired.collect())
}

/// The pins of a manifest's files that `file` keeps, with their digests.
fn pinned(pins: &Pins, file: impl Fn(&str) -> bool) -> Vec<(&str, &str)> {
    let files = pins.files.iter().filter(|(name, _)| file(name));
    files
        .map(|(name, digest)| (name.as_str(), digest.as_str()))
        .collect()
}

/// How a run's pinned table differs from the resolved one: its own policy
/// file is another, or only a base layer it composes moved, which may
/// have left the table as it is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Drift {
    Table,
    Base,
}

impl Drift {
    /// `run`'s drift from `resolved`; none for a run that pinned neither
    /// a policy file nor a base layer, as there is nothing to compare.
    fn of(resolved: &Pins, run: &Pins) -> Option<Drift> {
        let own = |pins| pinned(pins, |file| file == POLICY_FILE);
        let layers = |pins| pinned(pins, |file| file.starts_with(COMPOSE_PREFIX));
        let (own_run, layers_run) = (own(run), layers(run));
        match (
            own_run.is_empty() && layers_run.is_empty(),
            own_run == own(resolved),
        ) {
            (true, _) => None,
            (false, false) => Some(Drift::Table),
            (false, true) => (layers_run != layers(resolved)).then_some(Drift::Base),
        }
    }

    fn words(self) -> &'static str {
        match self {
            Drift::Table => "ran under an earlier version of this table",
            Drift::Base => {
                "ran under an earlier version of a base this recipe composes, \
                 which may not have changed this table"
            }
        }
    }
}

/// The runs whose pinned table is not the resolved one, and the words for
/// them, those whose own policy file differs apart from those whose base
/// alone moved.
fn drift(table: Option<GraphTable>, runs: &[GraphRun]) -> (usize, Option<String>) {
    let Some(table) = table else {
        return (0, None);
    };
    let resolved = pins_of(table.manifest);
    let drifts: Vec<Drift> = runs
        .iter()
        .filter_map(|run| Drift::of(&resolved, &run.pins))
        .collect();
    let clauses: Vec<String> = [Drift::Table, Drift::Base]
        .into_iter()
        .filter_map(
            |kind| match drifts.iter().filter(|drift| **drift == kind).count() {
                0 => None,
                1 => Some(format!("1 run {}", kind.words())),
                many => Some(format!("{many} runs {}", kind.words())),
            },
        )
        .collect();
    (
        drifts.len(),
        (!clauses.is_empty()).then(|| clauses.join(" · ")),
    )
}

/// The rules as the graph draws them: one link per state and next state,
/// in the order its first rule holds, its outcomes by result.
fn links(rules: &[Edge]) -> Vec<Link> {
    let mut links: Vec<Link> = Vec::new();
    for rule in rules {
        let Some(next) = &rule.next else { continue };
        let at = match links
            .iter()
            .position(|link| link.from == rule.from && &link.next == next)
        {
            Some(at) => at,
            None => {
                links.push(Link {
                    from: rule.from.clone(),
                    next: next.clone(),
                    kind: rule.kind,
                    count: 0,
                    rules: 0,
                    outcomes: Vec::new(),
                    text: String::new(),
                });
                links.len() - 1
            }
        };
        let link = &mut links[at];
        link.count += rule.count;
        link.rules += 1;
        if rule.count == 0 {
            continue;
        }
        match link
            .outcomes
            .iter_mut()
            .find(|outcome| outcome.result == rule.result)
        {
            Some(outcome) => outcome.count += rule.count,
            None => link.outcomes.push(Outcome {
                result: rule.result.clone(),
                count: rule.count,
                text: String::new(),
            }),
        }
    }
    for link in &mut links {
        // The most runs first; a tie keeps the table's order.
        link.outcomes.sort_by_key(|outcome| std::cmp::Reverse(outcome.count));
        for outcome in &mut link.outcomes {
            outcome.text = format!("{} {}", outcome.result, outcome.count);
        }
        let words: Vec<&str> = link
            .outcomes
            .iter()
            .map(|outcome| outcome.text.as_str())
            .collect();
        link.text = words.join(" · ");
    }
    links
}

/// The 90th percentile of `sorted` by nearest rank; `None` of nothing.
pub(super) fn p90(sorted: &[i64]) -> Option<i64> {
    let rank = (sorted.len() * 9).div_ceil(10);
    rank.checked_sub(1).map(|index| sorted[index])
}

/// The tokens a state's seats metered, summed (a cache read is inside
/// input already and never added to it, the [`TokenUsage`] rule), read
/// apart as the run view's seats are: the one place the cache hit is
/// worked out.
fn tokens(usages: impl Iterator<Item = TokenUsage>) -> crate::TokenCells {
    let mut sum = TokenUsage::default();
    for usage in usages {
        add_token(&mut sum.input_tokens, usage.input_tokens);
        add_token(&mut sum.output_tokens, usage.output_tokens);
        add_token(&mut sum.cache_read_tokens, usage.cache_read_tokens);
    }
    crate::tokens::usage_cells(finish_usage(sum).as_ref(), "").1
}

/// The seats the latest run pinned for `phase`: those its manifest's
/// agents key under it (`design`, `design:positions:robustness`).
fn seats(latest: Option<&GraphRun>, phase: &str) -> Vec<SeatPin> {
    let agents = latest.into_iter().flat_map(|run| &run.pins.agents);
    let under = agents.filter(|(key, _)| key.split(':').next() == Some(phase));
    under
        .map(|(key, hired)| {
            let named: Vec<&str> = [&hired.provider, &hired.model]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect();
            let named = match named.is_empty() {
                true => "not recorded".to_string(),
                false => named.join(" "),
            };
            SeatPin {
                text: format!("{key} · {named}"),
                key: key.clone(),
                provider: hired.provider.clone(),
                model: hired.model.clone(),
            }
        })
        .collect()
}

fn state(shape: &Shape, runs: &[GraphRun], phase: &str, window: Window) -> State {
    let held = |at: i64, of: Option<&str>| of == Some(phase) && window.holds(at);
    let stays = runs.iter().flat_map(|run| &run.stays);
    let stays: Vec<_> = stays
        .filter(|stay| held(stay.at, Some(&stay.phase)))
        .collect();
    let mut spans: Vec<i64> = stays
        .iter()
        .filter_map(|stay| stay.left.map(|left| (left - stay.at).max(0)))
        .collect();
    spans.sort_unstable();
    let attempts = runs.iter().map(|run| &run.attempts);
    let sites = attempts.clone().flat_map(|attempts| &attempts.sites);
    let costs = attempts.flat_map(|attempts| &attempts.costs);
    let spent = total(
        costs
            .filter(|spend| held(spend.at, spend.phase.as_deref()))
            .map(|spend| spend.usd),
    );
    let entered = runs
        .iter()
        .filter(|run| {
            run.stays
                .iter()
                .any(|stay| held(stay.at, Some(&stay.phase)))
        })
        .count();
    State {
        name: phase.to_string(),
        place: shape.place(phase),
        entered,
        median: span(middle(&spans)),
        p90: span(p90(&spans)),
        tokens: tokens(sites.clone().filter_map(|site| site.usage(&held))),
        cost: cost_figure(sites, &held, spent),
        seats: seats(runs.iter().max_by_key(|run| run.started), phase),
    }
}

fn ends(runs: &[GraphRun]) -> Ends {
    let (mut done, mut operator, mut open) = (0, 0, 0);
    let mut by_rule: BTreeMap<Option<&str>, usize> = BTreeMap::new();
    for run in runs {
        match &run.finished {
            Some((_, Ending::Shipped)) => done += 1,
            Some((_, Ending::ByRule(rule))) => *by_rule.entry(rule.as_deref()).or_default() += 1,
            Some((_, Ending::ByOperator)) => operator += 1,
            None => open += 1,
        }
    }
    let mut stop: Vec<RuleCount> = by_rule
        .into_iter()
        .map(|(rule_id, count)| RuleCount {
            rule_id: rule_id.map(str::to_string),
            count,
        })
        .collect();
    stop.sort_by_key(|rule| -(rule.count as i64));
    let stopped = stop.iter().map(|rule| rule.count).sum();
    Ends {
        text: format!(
            "{done} done · {stopped} stop by a rule · {operator} stopped by you · {open} open"
        ),
        done,
        stop,
        stopped,
        operator,
        open,
    }
}

/// The recipe's graph for `query`: its table's states and rules with the
/// traffic of the runs the range ending at `now` keeps.
pub fn graph(query: &GraphQuery) -> Result<GraphView, GraphError> {
    let now =
        js::parse_millis(query.now).ok_or_else(|| GraphError::Clock(query.now.to_string()))?;
    let window = Window {
        from: now - query.range.millis(),
        to: now,
    };
    let Scoped { runs, carried } = Scoped::read(query, window);
    let known = carried.iter().chain(query.shelved).cloned();
    let recipes: Vec<String> = known.collect::<BTreeSet<String>>().into_iter().collect();
    if query.table.is_none() && !carried.contains(query.recipe) {
        return Err(GraphError::UnknownRecipe {
            recipe: query.recipe.to_string(),
            realm: query.realm.to_string(),
            known: recipes,
        });
    }
    let shape = Shape::of(query.table, &runs);
    let (rules, retired) = rules(query.table, &shape, &fired(&runs, window));
    let dormant = rules.iter().filter(|rule| rule.count == 0).count();
    let (drifted_runs, drift_text) = drift(query.table, &runs);
    Ok(GraphView {
        view_version: GRAPH_VIEW_VERSION,
        realm: query.realm.to_string(),
        recipe: query.recipe.to_string(),
        range: query.range,
        from: stamp(window.from),
        to: query.now.to_string(),
        recipes,
        table: match query.table {
            Some(_) => Table::Resolved,
            None => Table::NotFound {
                text: format!("this recipe's table was not found in {}", query.realm),
            },
        },
        runs: runs.len(),
        states: (shape.phases.iter())
            .map(|phase| state(&shape, &runs, phase, window))
            .collect(),
        never_fired: NeverFired {
            count: dormant,
            of: rules.len(),
            text: format!("{dormant} of {}", rules.len()),
        },
        links: links(&rules),
        rules,
        retired,
        drifted_runs,
        drift_text,
        ends: ends(&runs),
        live: (runs.iter().filter(|run| run.open))
            .map(|run| LiveRun {
                run_id: run.run_id.clone(),
                label: run.label.clone(),
                state: run.state.clone(),
            })
            .collect(),
    })
}
