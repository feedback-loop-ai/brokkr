//! What one run's journal says about the recipe's graph, read once: the
//! states it entered and when it left each, the rules that ruled it, how
//! it ended, what it pinned and what its seats spent; and the runs a
//! query's realm and recipe keep.

use std::collections::BTreeSet;

use brokkr_core::decision::Decision;
use brokkr_core::summary::aggregate;
use brokkr_core::{EventEnvelope, EventType};
use serde::Deserialize;

use super::GraphQuery;
use crate::analytics::{in_range, is_open, Ending, Window};
use crate::attempts::{attempts, Attempts};
use crate::clarify::{pins, Pins};
use crate::{field, title, RuleSeverity};

/// One stay in a state: when the run entered it, and when it entered the
/// next or reached its terminal event, if it has.
pub(super) struct Stay {
    pub(super) phase: String,
    pub(super) at: i64,
    pub(super) left: Option<i64>,
}

/// One ruling that named its rule.
pub(super) struct Ruled {
    pub(super) at: i64,
    pub(super) rule_id: String,
    pub(super) from: String,
    pub(super) next: Option<String>,
    pub(super) result: String,
    pub(super) severity: Option<RuleSeverity>,
}

pub(super) struct GraphRun {
    pub(super) run_id: String,
    pub(super) label: String,
    pub(super) started: Option<i64>,
    pub(super) finished: Option<(i64, Ending)>,
    pub(super) open: bool,
    /// The state the fold leaves the run in.
    pub(super) state: Option<String>,
    pub(super) stays: Vec<Stay>,
    pub(super) rulings: Vec<Ruled>,
    pub(super) pins: Pins,
    pub(super) attempts: Attempts,
}

fn at(event: &EventEnvelope) -> Option<i64> {
    brokkr_core::stamp::parse_millis(&event.recorded_at)
}

/// Every stay, each closed by the next entry or the terminal event.
fn stays(events: &[EventEnvelope]) -> Vec<Stay> {
    let mut stays: Vec<Stay> = Vec::new();
    for event in events {
        let entered = event.event_type == EventType::PhaseEntered;
        let ended = matches!(
            event.event_type,
            EventType::RunCompleted | EventType::RunStopped
        );
        let Some(time) = at(event).filter(|_| entered || ended) else {
            continue;
        };
        if let Some(open) = stays.last_mut().filter(|stay| stay.left.is_none()) {
            open.left = Some(time);
        }
        if let (true, Some(phase)) = (entered, field(&event.payload, "phase")) {
            stays.push(Stay {
                phase: phase.to_string(),
                at: time,
                left: None,
            });
        }
    }
    stays
}

/// Every `transition/decided` that named its rule, typed as the core
/// records it.
fn rulings(events: &[EventEnvelope]) -> Vec<Ruled> {
    let decided = events
        .iter()
        .filter(|event| event.event_type == EventType::TransitionDecided);
    decided
        .filter_map(|event| {
            let decision = Decision::deserialize(&event.payload).ok()?;
            Some(Ruled {
                at: at(event)?,
                rule_id: decision.rule_id.present()?.clone(),
                next: decision.next.present().cloned(),
                severity: decision.severity.present().copied(),
                from: decision.from,
                result: decision.result,
            })
        })
        .collect()
}

/// The runs of the query's recipe in its realm that the range keeps, and
/// every recipe the realm's runs carry.
pub(super) struct Scoped {
    pub(super) runs: Vec<GraphRun>,
    pub(super) carried: BTreeSet<String>,
}

impl Scoped {
    pub(super) fn read(query: &GraphQuery, window: Window) -> Scoped {
        let mut scoped = Scoped {
            runs: Vec::new(),
            carried: BTreeSet::new(),
        };
        let readable = query
            .hearths
            .iter()
            .filter(|hearth| hearth.refusal.is_none());
        for hearth in readable {
            for run in hearth.runs {
                let (Some(state), Some(first)) = (run.state, run.events.first()) else {
                    continue;
                };
                let read = aggregate(run.events, Some(state));
                let realm = read.realm.unwrap_or_else(|| hearth.label.to_string());
                let Some(carried) = read.recipe.filter(|_| realm == query.realm) else {
                    continue;
                };
                let finished = read.finished;
                let open = is_open(state);
                let ended = finished.as_ref().map(|(at, _)| *at);
                if carried == query.recipe && in_range(ended, open, window) {
                    scoped.runs.push(GraphRun {
                        run_id: first.run_id.clone(),
                        label: state
                            .feature
                            .as_deref()
                            .map_or_else(|| first.run_id.clone(), title),
                        started: read.started_ms,
                        finished,
                        open,
                        state: state.phase.clone(),
                        stays: stays(run.events),
                        rulings: rulings(run.events),
                        pins: pins(run.events),
                        attempts: attempts(run.events),
                    });
                }
                scoped.carried.insert(carried);
            }
        }
        scoped
    }
}
