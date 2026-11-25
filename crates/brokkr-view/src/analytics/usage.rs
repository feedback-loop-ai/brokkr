//! What each recipe's attempts used, per phase (decision 0112 ruling 4):
//! the minimum, average and maximum of each measure the engine recorded
//! on an attempt's ending under `effect-usage/v1`, means and peaks alike,
//! over every driver it measured — a seat's one, a panel's each measured
//! member, a sequence's each measured step driver. An ending without `usage`, every attempt journaled before the
//! extension among them, is counted apart as unmeasured, never as zero. A
//! `usage` that does not read is counted the same way: no figure is
//! guessed from it. `brokkr library recipes usage --json` emits these rows, and
//! its table only paints them (decision 0071 ruling 7).

use std::collections::BTreeMap;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::fold;
use brokkr_core::summary::aggregate;
use brokkr_core::EventEnvelope;
use serde::{Deserialize, Serialize};

/// One phase of one recipe: how many endings it journaled, how many of
/// them carried no usage, how many drivers were measured, and each
/// measure's spread over them; a measure is absent where none was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhaseUsage {
    pub recipe: String,
    pub phase: String,
    pub attempts: usize,
    pub unmeasured: usize,
    pub measured: usize,
    pub peak_memory_bytes: Option<Spread>,
    pub mean_memory_bytes: Option<Spread>,
    pub peak_load_millicores: Option<Spread>,
    pub mean_load_millicores: Option<Spread>,
    pub cpu_ms: Option<Spread>,
    pub wall_ms: Option<Spread>,
}

/// A measure's minimum, average (rounded down) and maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Spread {
    pub min: u64,
    pub average: u64,
    pub max: u64,
}

/// One driver's measures, as `effect-usage/v1` writes them.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Measures {
    peak_memory_bytes: u64,
    mean_memory_bytes: u64,
    peak_load_millicores: u64,
    mean_load_millicores: u64,
    cpu_ms: u64,
    wall_ms: u64,
}

/// Several drivers' measures: a panel's by member, a sequence's by step tag.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Members {
    members: BTreeMap<String, Measures>,
}

/// The `usage` object: a seat's measures, or a panel's.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Usage {
    Seat(Measures),
    Panel(Members),
}

/// An ending, as far as its usage.
#[derive(Deserialize)]
struct Ending {
    usage: Option<Usage>,
}

/// A `phase/entered`, as far as its phase.
#[derive(Deserialize)]
struct Entered {
    phase: String,
}

/// One phase's endings so far.
#[derive(Default)]
struct Tally {
    attempts: usize,
    unmeasured: usize,
    measured: Vec<Measures>,
}

impl Tally {
    fn note(&mut self, payload: &serde_json::Value) {
        self.attempts += 1;
        match Ending::deserialize(payload)
            .ok()
            .and_then(|ending| ending.usage)
        {
            None => self.unmeasured += 1,
            Some(Usage::Seat(measures)) => self.measured.push(measures),
            Some(Usage::Panel(panel)) => self.measured.extend(panel.members.into_values()),
        }
    }

    fn row(self, recipe: String, phase: String) -> PhaseUsage {
        let spread = |measure: fn(&Measures) -> u64| spread(self.measured.iter().map(measure));
        PhaseUsage {
            peak_memory_bytes: spread(|m| m.peak_memory_bytes),
            mean_memory_bytes: spread(|m| m.mean_memory_bytes),
            peak_load_millicores: spread(|m| m.peak_load_millicores),
            mean_load_millicores: spread(|m| m.mean_load_millicores),
            cpu_ms: spread(|m| m.cpu_ms),
            wall_ms: spread(|m| m.wall_ms),
            recipe,
            phase,
            attempts: self.attempts,
            unmeasured: self.unmeasured,
            measured: self.measured.len(),
        }
    }
}

/// The spread of `values`; `None` when there are none.
fn spread(values: impl Iterator<Item = u64>) -> Option<Spread> {
    let (mut count, mut sum) = (0u128, 0u128);
    let (mut min, mut max) = (u64::MAX, 0);
    for value in values {
        count += 1;
        sum += u128::from(value);
        min = min.min(value);
        max = max.max(value);
    }
    let average = u64::try_from(sum.checked_div(count)?).unwrap_or(u64::MAX);
    Some(Spread { min, average, max })
}

/// Every phase of every recipe `runs` started under, or of `recipe` alone,
/// by recipe and then phase. Each run is one journal's events; a run whose
/// start names no recipe is in none, and a repaired run's late tail is
/// never read (decision 0108 ruling 4).
pub fn recipe_usage(runs: &[Vec<EventEnvelope>], recipe: Option<&str>) -> Vec<PhaseUsage> {
    let mut phases: BTreeMap<(String, String), Tally> = BTreeMap::new();
    for events in runs {
        let state = fold(events).ok();
        let applied = crate::live::derived(events, state.as_ref());
        let events = applied.as_slice();
        let started = aggregate(events, None).recipe;
        let Some(name) = started.filter(|name| recipe.is_none_or(|wanted| name.as_str() == wanted))
        else {
            continue;
        };
        tally(&mut phases, &name, events);
    }
    phases
        .into_iter()
        .map(|((recipe, phase), tally)| tally.row(recipe, phase))
        .collect()
}

/// Note each ending of one run of `recipe` in the phase it ended in.
fn tally(phases: &mut BTreeMap<(String, String), Tally>, recipe: &str, events: &[EventEnvelope]) {
    let mut phase = None;
    for event in events {
        let ending = matches!(
            event.event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        );
        if event.event_type == EventType::PhaseEntered {
            phase = Entered::deserialize(&event.payload)
                .ok()
                .map(|entered| entered.phase);
        } else if let Some(phase) = phase.as_ref().filter(|_| ending) {
            let key = (recipe.to_string(), phase.clone());
            phases.entry(key).or_default().note(&event.payload);
        }
    }
}

#[cfg(test)]
mod tests;
