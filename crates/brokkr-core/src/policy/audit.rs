//! Decision 0050's table checks as a diagnostic (#429): order, liveness,
//! presence and a bounded totality sweep over a loaded machine.
//!
//! The audit refuses nothing yet. The operator ruled on 2026-09-28 that
//! no check add a refusal or a transition to a table that loads today
//! before decision 0050 was ruled. It was accepted on 2026-09-29, and its
//! addendum orders the refusals' enactment, each in a slice of its own:
//! `docs/evidence/decision-0050-audit.md` lists them. `brokkr compile`
//! prints the audit beside its manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::{Map, Value};
use thiserror::Error;

use super::{conditions_met, vocabulary, Condition, Machine, Outcome, Rule, SEVERITY_ORDER};

/// The valuations one audit may sweep across a whole table. The largest
/// shipped table sweeps 1,072 (decision 0050), so the budget leaves room
/// for growth while a table whose guards multiply past it is diagnosed
/// instead of hanging compile.
pub const SWEEP_BUDGET: usize = 65_536;

/// The terminal a hard rule rules. Reaching it is not going on.
const STOP: &str = "stop";

/// What the audit found in one table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Audit {
    /// The `(phase, result)` groups the table rules.
    pub groups: usize,
    /// Every valuation the totality sweep evaluated.
    pub valuations: usize,
    pub findings: Vec<Finding>,
}

/// One finding, each a refusal decision 0050 proposes. Ruling 2's three
/// findings are read over the present, well-typed valuations of a group's
/// inputs, with every counter integral: an absent input satisfies no
/// condition, so a rule reported dead still fires when a seat omits an
/// input the earlier rules read. Absence is presence's (ruling 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// Ruling 2: `rule` fires on no present valuation, because `behind`
    /// precedes it in its group and matches wherever it matches.
    Shadowed { rule: String, behind: String },
    /// Ruling 2: `rule` fires on no present valuation, because the earlier
    /// rules `by` together match wherever it matches, and no one of them
    /// alone does.
    Covered { rule: String, by: Vec<String> },
    /// Ruling 2: `rule`'s guard holds on no present valuation of its
    /// inputs.
    Unsatisfiable { rule: String },
    /// Ruling 3: no transition edge reaches `phase` from `initial`.
    Unreachable { phase: String },
    /// Ruling 3: `phase` reaches no terminal phase and no parking rule.
    DeadEnd { phase: String },
    /// Ruling 1: `rule` advances without reading `inputs`, which a hard
    /// rule of its group reads and a seat supplies.
    Unread { rule: String, inputs: Vec<String> },
    /// Ruling 4: no rule rules this valuation of present, well-typed
    /// inputs, so the run would park with no rule named.
    Unruled {
        phase: String,
        result: String,
        valuation: Vec<(String, Setting)>,
    },
}

/// One input's value in a swept valuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Flag(bool),
    Count(u64),
    Word(&'static str),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuditError {
    #[error(
        "the policy sweep reaches {valuations} valuations at ({phase}, {result}), \
         over its budget of {budget}; the table was not swept"
    )]
    Budget {
        phase: String,
        result: String,
        valuations: usize,
        budget: usize,
    },
}

/// One `(phase, result)` group: its rules in table order, and the
/// domain of every input they read.
struct Group<'a> {
    phase: &'a str,
    result: &'a str,
    rules: Vec<&'a Rule>,
    axes: BTreeMap<&'a str, Vec<Setting>>,
}

impl Group<'_> {
    fn size(&self) -> usize {
        self.axes
            .values()
            .fold(1, |product, values| product.saturating_mul(values.len()))
    }

    /// The valuation at `index` of the sweep, the first axis turning
    /// fastest.
    fn valuation(&self, index: usize) -> Vec<(String, Setting)> {
        let mut rest = index;
        let mut valuation = Vec::with_capacity(self.axes.len());
        for (name, values) in &self.axes {
            valuation.push((name.to_string(), values[rest % values.len()]));
            rest /= values.len();
        }
        valuation
    }
}

impl Machine {
    /// Decision 0050's four table checks, reported and never refused.
    /// `engine_owned` names the inputs the engine always supplies, which
    /// presence (ruling 1) exempts. The sweep's size is measured before
    /// any valuation is evaluated, so a table over `budget` costs nothing.
    pub fn audit_with(
        &self,
        budget: usize,
        engine_owned: impl Fn(&str) -> bool,
    ) -> Result<Audit, AuditError> {
        let groups = self.groups();
        let mut total: usize = 0;
        for group in &groups {
            total = total.saturating_add(group.size());
            if total > budget {
                return Err(AuditError::Budget {
                    phase: group.phase.to_string(),
                    result: group.result.to_string(),
                    valuations: total,
                    budget,
                });
            }
        }
        let (dead, unruled): (Vec<Vec<Finding>>, Vec<Vec<Finding>>) =
            groups.iter().map(|group| self.sweep(group)).unzip();
        let mut findings: Vec<Finding> = dead.into_iter().flatten().collect();
        findings.extend(self.liveness());
        for group in &groups {
            findings.extend(self.unread(group, &engine_owned));
        }
        findings.extend(unruled.into_iter().flatten());
        Ok(Audit {
            groups: groups.len(),
            valuations: total,
            findings,
        })
    }

    fn groups(&self) -> Vec<Group<'_>> {
        let mut groups: BTreeMap<(&str, &str), Vec<&Rule>> = BTreeMap::new();
        for rule in &self.rules {
            groups
                .entry((rule.from.as_str(), rule.result.as_str()))
                .or_default()
                .push(rule);
        }
        groups
            .into_iter()
            .map(|((phase, result), rules)| Group {
                phase,
                result,
                axes: axes(&rules),
                rules,
            })
            .collect()
    }

    fn edges(&self) -> BTreeMap<&str, BTreeSet<&str>> {
        let mut edges: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for rule in &self.rules {
            if let Some(next) = &rule.next {
                edges
                    .entry(rule.from.as_str())
                    .or_default()
                    .insert(next.as_str());
            }
        }
        edges
    }

    /// Every phase reachable from `start` over edges that never enter
    /// `avoid`, `start` included.
    fn reach<'a>(&'a self, start: &'a str, avoid: Option<&str>) -> BTreeSet<&'a str> {
        let edges = self.edges();
        let mut seen = BTreeSet::from([start]);
        let mut frontier = vec![start];
        while let Some(node) = frontier.pop() {
            for &next in edges.get(node).into_iter().flatten() {
                if Some(next) != avoid && seen.insert(next) {
                    frontier.push(next);
                }
            }
        }
        seen
    }

    /// Ruling 3: every phase reachable from `initial`, and every
    /// non-terminal phase ends at a terminal or a parking rule.
    fn liveness(&self) -> Vec<Finding> {
        let reachable = self.reach(&self.initial, None);
        let ending = |phase: &str| {
            self.terminal.iter().any(|terminal| terminal == phase)
                || self
                    .rules
                    .iter()
                    .any(|rule| rule.from == phase && rule.next.is_none())
        };
        let mut findings = Vec::new();
        for phase in &self.phases {
            if !reachable.contains(phase.as_str()) {
                findings.push(Finding::Unreachable {
                    phase: phase.clone(),
                });
            } else if !self.reach(phase, None).into_iter().any(ending) {
                findings.push(Finding::DeadEnd {
                    phase: phase.clone(),
                });
            }
        }
        findings
    }

    /// Ruling 1: a rule that advances — its next phase reaches a
    /// non-stop terminal without re-entering the group's phase — reads
    /// every seat input a hard rule of its group reads. A rule that
    /// returns to its own phase, or parks, may read fewer.
    fn unread(&self, group: &Group<'_>, engine_owned: &impl Fn(&str) -> bool) -> Vec<Finding> {
        let denied: BTreeSet<&str> = group
            .rules
            .iter()
            .filter(|rule| rule.severity == "hard")
            .flat_map(|rule| rule.when.iter().map(Condition::name))
            .filter(|&name| !engine_owned(name))
            .collect();
        let mut findings = Vec::new();
        for rule in &group.rules {
            let Some(next) = rule.next.as_deref() else {
                continue;
            };
            if next == group.phase || !self.advances(next, group.phase) {
                continue;
            }
            let reads: BTreeSet<&str> = rule.when.iter().map(Condition::name).collect();
            let inputs: Vec<String> = denied.difference(&reads).map(|n| n.to_string()).collect();
            if !inputs.is_empty() {
                findings.push(Finding::Unread {
                    rule: rule.id.clone(),
                    inputs,
                });
            }
        }
        findings
    }

    fn advances(&self, start: &str, avoid: &str) -> bool {
        self.reach(start, Some(avoid))
            .into_iter()
            .any(|phase| phase != STOP && self.terminal.iter().any(|t| t == phase))
    }

    /// Rulings 2 and 4 from one sweep: every valuation of the group's
    /// axes, evaluated by the real evaluator. A valuation it cannot rule
    /// is unruled, and a rule that rules no valuation is dead. The axes
    /// sample every threshold at and around it, so every region of every
    /// guard form is walked and both answers are exact over present,
    /// well-typed inputs with integral counters: a vacuous guard,
    /// or earlier arms that together cover a later one, are found as
    /// surely as a single stronger guard. Returns `(dead, unruled)`.
    fn sweep(&self, group: &Group<'_>) -> (Vec<Finding>, Vec<Finding>) {
        let mut holds: Vec<Vec<bool>> = vec![Vec::new(); group.rules.len()];
        let mut winners: Vec<Option<usize>> = Vec::new();
        let mut unruled = Vec::new();
        for index in 0..group.size() {
            let valuation = group.valuation(index);
            let inputs: Map<String, Value> = valuation
                .iter()
                .map(|(name, setting)| (name.clone(), setting.value()))
                .collect();
            for (rule, holds) in group.rules.iter().zip(&mut holds) {
                holds.push(conditions_met(&rule.when, &inputs) == Ok(true));
            }
            winners.push(match self.evaluate(group.phase, group.result, &inputs) {
                Outcome::Ruling { rule_id, .. } | Outcome::Park { rule_id, .. } => {
                    group.rules.iter().position(|rule| rule.id == rule_id)
                }
                Outcome::NoRule { .. } => {
                    unruled.push(Finding::Unruled {
                        phase: group.phase.to_string(),
                        result: group.result.to_string(),
                        valuation,
                    });
                    None
                }
            });
        }
        let dead = (0..group.rules.len())
            .filter(|&rule| !winners.contains(&Some(rule)))
            .map(|rule| dead(group, rule, &holds, &winners))
            .collect();
        (dead, unruled)
    }
}

/// Ruling 2: why the group's rule at `rule`, which rules no swept
/// valuation, is dead. Its guard holds nowhere; or the first earlier
/// rule that holds wherever it holds shadows it; or the earlier rules
/// that won where it holds cover it together.
fn dead(group: &Group<'_>, rule: usize, holds: &[Vec<bool>], winners: &[Option<usize>]) -> Finding {
    let id = group.rules[rule].id.clone();
    let at: Vec<usize> = (0..winners.len())
        .filter(|&valuation| holds[rule][valuation])
        .collect();
    if at.is_empty() {
        return Finding::Unsatisfiable { rule: id };
    }
    if let Some(earlier) =
        (0..rule).find(|&earlier| at.iter().all(|&valuation| holds[earlier][valuation]))
    {
        return Finding::Shadowed {
            rule: id,
            behind: group.rules[earlier].id.clone(),
        };
    }
    let by: BTreeSet<usize> = at
        .iter()
        .filter_map(|&valuation| winners[valuation])
        .collect();
    Finding::Covered {
        rule: id,
        by: by.into_iter().map(|w| group.rules[w].id.clone()).collect(),
    }
}

/// The domain of every input a group reads: both flags, the counter at
/// zero and one below, at and above each threshold, the six severities,
/// the closed enumeration.
fn axes<'a>(rules: &[&'a Rule]) -> BTreeMap<&'a str, Vec<Setting>> {
    let mut counts: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    let mut axes: BTreeMap<&str, Vec<Setting>> = BTreeMap::new();
    for condition in rules.iter().flat_map(|&rule| &rule.when) {
        let name = condition.name();
        let words: &'static [&'static str] = match condition {
            Condition::CounterGte { threshold, .. } => {
                // Saturating: a negative threshold reads from zero.
                let at = *threshold as u64;
                counts
                    .entry(name)
                    .or_insert_with(|| BTreeSet::from([0]))
                    .extend([at.saturating_sub(1), at, at.saturating_add(1)]);
                continue;
            }
            Condition::Flag { .. } => {
                axes.insert(name, vec![Setting::Flag(true), Setting::Flag(false)]);
                continue;
            }
            Condition::SeverityAbove { .. } | Condition::SeverityAtMost { .. } => &SEVERITY_ORDER,
            Condition::EnumIn { name, .. } => vocabulary(name),
        };
        axes.insert(name, words.iter().copied().map(Setting::Word).collect());
    }
    for (name, values) in counts {
        axes.insert(name, values.into_iter().map(Setting::Count).collect());
    }
    axes
}

impl Condition {
    fn name(&self) -> &str {
        match self {
            Condition::CounterGte { name, .. }
            | Condition::SeverityAbove { name, .. }
            | Condition::SeverityAtMost { name, .. }
            | Condition::Flag { name, .. }
            | Condition::EnumIn { name, .. } => name,
        }
    }
}

impl Setting {
    fn value(self) -> Value {
        match self {
            Setting::Flag(flag) => Value::Bool(flag),
            Setting::Count(count) => Value::from(count),
            Setting::Word(word) => Value::from(word),
        }
    }
}

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            Setting::Flag(flag) => flag.to_string(),
            Setting::Count(count) => count.to_string(),
            Setting::Word(word) => word.to_string(),
        })
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            Finding::Shadowed { rule, behind } => format!(
                "{rule} is dead behind {behind}: its guard holds wherever {rule}'s \
                 does, and first match wins"
            ),
            Finding::Covered { rule, by } => format!(
                "{rule} is dead behind {}: together their guards hold wherever \
                 {rule}'s does, and first match wins",
                by.join(", ")
            ),
            Finding::Unsatisfiable { rule } => {
                format!("{rule} is dead: its guard holds on no valuation of its inputs")
            }
            Finding::Unreachable { phase } => {
                format!("phase '{phase}' is unreachable from the initial phase")
            }
            Finding::DeadEnd { phase } => {
                format!("phase '{phase}' reaches no terminal phase and no parking rule")
            }
            Finding::Unread { rule, inputs } => format!(
                "{rule} lets the run go on without reading {}, which a hard rule \
                 of its group reads",
                inputs.join(", ")
            ),
            Finding::Unruled {
                phase,
                result,
                valuation,
            } => {
                let settings: Vec<String> = valuation
                    .iter()
                    .map(|(name, setting)| format!("{name}={setting}"))
                    .collect();
                format!(
                    "no rule rules ({phase}, {result}) at {}",
                    settings.join(", ")
                )
            }
        })
    }
}

/// The report `brokkr compile` prints: the sweep's size, every finding
/// but the unruled valuations, and the first of those.
impl fmt::Display for Audit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (unruled, others): (Vec<&Finding>, Vec<&Finding>) = self
            .findings
            .iter()
            .partition(|finding| matches!(finding, Finding::Unruled { .. }));
        let mut report = format!(
            "policy sweep (decision 0050, proposed; reported, not refused): {} \
             valuations over {} groups, {} unruled\n",
            self.valuations,
            self.groups,
            unruled.len()
        );
        for finding in others.iter().chain(unruled.first()) {
            report.push_str(&format!("  {finding}\n"));
        }
        f.write_str(&report)
    }
}

#[cfg(test)]
mod tests;
