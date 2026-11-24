//! What a recipe's measured runs say it needs (decision 0125 ruling 3):
//! of the rows every journal's history read gave, each recipe's newest
//! ended runs that carry `effect-usage/v1`, at most the window. A profile
//! needs a completed run in that window. Each phase uses the means of
//! its attempts' peaks, an attempt of several drivers
//! counted as their sum. The order those runs entered their phases says
//! which phases a running run may still enter (ruling 5).

use std::collections::{BTreeMap, BTreeSet};

use brokkr_store::{UsageEvent, UsageRow};
use serde::Deserialize;

/// A peak memory and a peak load: in bytes and thousandths of a core.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Peak {
    pub(crate) memory_bytes: u64,
    pub(crate) load_millicores: u64,
}

impl Peak {
    /// The larger of each resource.
    pub(super) fn max(self, other: Peak) -> Peak {
        Peak {
            memory_bytes: self.memory_bytes.max(other.memory_bytes),
            load_millicores: self.load_millicores.max(other.load_millicores),
        }
    }

    fn add(self, other: Peak) -> Peak {
        Peak {
            memory_bytes: self.memory_bytes.saturating_add(other.memory_bytes),
            load_millicores: self.load_millicores.saturating_add(other.load_millicores),
        }
    }
}

/// One driver's measures, as `effect-usage/v1` writes them, closed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Measures {
    peak_memory_bytes: u64,
    #[serde(rename = "mean_memory_bytes")]
    _mean_memory_bytes: u64,
    peak_load_millicores: u64,
    #[serde(rename = "mean_load_millicores")]
    _mean_load_millicores: u64,
    #[serde(rename = "cpu_ms")]
    _cpu_ms: u64,
    #[serde(rename = "wall_ms")]
    _wall_ms: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Members {
    members: BTreeMap<String, Measures>,
}

/// A seat's measures, or several drivers'.
#[derive(Deserialize)]
#[serde(untagged)]
enum Usage {
    Seat(Measures),
    Members(Members),
}

/// An attempt's peaks from its `usage` text: its own, or its drivers'
/// sum; `None` where the text is not `effect-usage/v1`.
fn attempt(usage: &str) -> Option<Peak> {
    let peak = |m: &Measures| Peak {
        memory_bytes: m.peak_memory_bytes,
        load_millicores: m.peak_load_millicores,
    };
    match serde_json::from_str::<Usage>(usage).ok()? {
        Usage::Seat(measures) => Some(peak(&measures)),
        Usage::Members(Members { members }) => (!members.is_empty())
            .then(|| members.values().map(peak).fold(Peak::default(), Peak::add)),
    }
}

/// One run as its rows read: when it was created, whether it completed,
/// the phases it entered in order, and each measured attempt's phase and peaks.
#[derive(Default)]
struct Run {
    created_at: String,
    completed: bool,
    entered: Vec<String>,
    attempts: Vec<(String, Peak)>,
}

impl Run {
    fn note(&mut self, event: &UsageEvent) {
        match event {
            UsageEvent::Completed => self.completed = true,
            UsageEvent::Entered(phase) => self.entered.push(phase.clone()),
            UsageEvent::Ended(usage) => {
                let phase = self.entered.last().cloned();
                if let Some((phase, peak)) = phase.zip(attempt(usage)) {
                    self.attempts.push((phase, peak));
                }
            }
        }
    }
}

/// What one recipe's window measured.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Profile {
    /// The runs in the window.
    pub(crate) runs: usize,
    /// Each phase's expected peaks: the means of its attempts' peaks.
    pub(crate) phases: BTreeMap<String, Peak>,
    /// The phases each run entered, in order.
    sequences: Vec<Vec<String>>,
}

impl Profile {
    /// The largest expected peak of every phase.
    pub(crate) fn peak(&self) -> Peak {
        self.phases
            .values()
            .fold(Peak::default(), |all, peak| all.max(*peak))
    }

    /// The largest expected peak of `current` and every phase a run of the
    /// window entered after it. Use every measured phase when the window
    /// never entered the current phase or measured any remaining phase.
    pub(crate) fn remaining(&self, current: Option<&str>) -> Peak {
        let Some(current) = current.filter(|phase| self.entered(phase)) else {
            return self.peak();
        };
        let mut ahead = BTreeSet::from([current]);
        for sequence in &self.sequences {
            let first = sequence.iter().position(|phase| phase == current);
            ahead.extend(
                first
                    .into_iter()
                    .flat_map(|at| &sequence[at..])
                    .map(String::as_str),
            );
        }
        self.phases
            .iter()
            .filter(|(phase, _)| ahead.contains(phase.as_str()))
            .map(|(_, peak)| *peak)
            .reduce(Peak::max)
            .unwrap_or_else(|| self.peak())
    }

    fn entered(&self, phase: &str) -> bool {
        self.sequences
            .iter()
            .flatten()
            .any(|entered| entered == phase)
    }

    /// The profile of `runs`, newest first, each already measured.
    fn of(runs: Vec<Run>) -> Profile {
        let mut sums: BTreeMap<String, (u128, u128, u128)> = BTreeMap::new();
        for (phase, peak) in runs.iter().flat_map(|run| &run.attempts) {
            let sum = sums.entry(phase.clone()).or_default();
            sum.0 += u128::from(peak.memory_bytes);
            sum.1 += u128::from(peak.load_millicores);
            sum.2 += 1;
        }
        let mean = |sum: u128, count: u128| u64::try_from(sum.div_ceil(count)).unwrap_or(u64::MAX);
        let phases = (sums.into_iter())
            .map(|(phase, (memory, load, count))| {
                let peak = Peak {
                    memory_bytes: mean(memory, count),
                    load_millicores: mean(load, count),
                };
                (phase, peak)
            })
            .collect();
        Profile {
            runs: runs.len(),
            phases,
            sequences: runs.into_iter().map(|run| run.entered).collect(),
        }
    }
}

/// Every recipe's profile, by name: only a recipe with a completed run
/// in its measured window has one.
pub(crate) fn learn(
    rows: impl IntoIterator<Item = UsageRow>,
    window: usize,
) -> BTreeMap<String, Profile> {
    let mut runs: BTreeMap<(String, String), Run> = BTreeMap::new();
    for row in rows {
        let run = runs.entry((row.recipe, row.run)).or_default();
        run.created_at = row.created_at;
        run.note(&row.event);
    }
    let mut recipes: BTreeMap<String, Vec<Run>> = BTreeMap::new();
    for ((recipe, _), run) in runs {
        if !run.attempts.is_empty() {
            recipes.entry(recipe).or_default().push(run);
        }
    }
    (recipes.into_iter())
        .filter_map(|(recipe, mut runs)| {
            runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            runs.truncate(window);
            runs.iter()
                .any(|run| run.completed)
                .then(|| (recipe, Profile::of(runs)))
        })
        .collect()
}

#[cfg(test)]
mod tests;
