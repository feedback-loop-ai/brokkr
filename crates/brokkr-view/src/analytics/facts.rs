//! What one run's journal and folded state say, read once into the facts
//! every figure is aggregated from, and the runs a query's realm keeps.
//! What the writer keeps of a run for them, brokkr-core reads
//! ([`aggregate`]), so the readout and the run's summary row read it one
//! way (decision 0089 ruling 7).

use std::collections::BTreeMap;

use brokkr_core::fold::{concluded, RunState};
use brokkr_core::summary::aggregate;
use brokkr_core::{EventEnvelope, EventType};

use super::{AnalyticsRun, QuarantinedHearth, Query};
use crate::attempts::{attempts, Attempts, Site};
use crate::field;

pub(crate) use brokkr_core::summary::Ending;

/// A half-open span of time: its first moment held, its last not.
#[derive(Clone, Copy)]
pub(crate) struct Window {
    pub(crate) from: i64,
    pub(crate) to: i64,
}

impl Window {
    pub(crate) fn holds(self, at: i64) -> bool {
        at >= self.from && at < self.to
    }

    pub(super) fn overlap(self, start: i64, end: i64) -> i64 {
        (end.min(self.to) - start.max(self.from)).max(0)
    }
}

pub(super) struct Park {
    pub(super) phase: Option<String>,
    pub(super) at: i64,
    pub(super) answered: i64,
    pub(super) ended: i64,
}

/// Where a run's review rulings sent it: whether its first sent it back,
/// whether its second routed it on, and whether any sent it back.
pub(super) struct Reviews {
    pub(super) first: bool,
    pub(super) second: bool,
    pub(super) back: bool,
}

pub(super) struct RunFacts {
    pub(super) realm: String,
    /// The recipe the run started under, when its start names one.
    pub(super) recipe: Option<String>,
    readable: bool,
    /// Still running or parked: not concluded.
    pub(super) open: bool,
    pub(super) started: Option<i64>,
    pub(super) finished: Option<(i64, Ending)>,
    pub(super) parks: Vec<Park>,
    pub(super) implement_visits: u64,
    pub(super) reviews: Reviews,
    pub(super) costs: Vec<(i64, f64)>,
    pub(super) sites: Vec<Site>,
}

/// The runs a range keeps: those that ended inside it, and those still
/// open. The one reading of "in range" the per-recipe figures and the
/// recipe's graph share.
pub(crate) fn in_range(finished: Option<i64>, open: bool, window: Window) -> bool {
    open || finished.is_some_and(|ended| window.holds(ended))
}

fn at(event: &EventEnvelope) -> Option<i64> {
    brokkr_core::stamp::parse_millis(&event.recorded_at)
}

/// Whether the run is still running or parked: its fold has not
/// concluded it.
pub(crate) fn is_open(state: &RunState) -> bool {
    concluded(state).is_none()
}

fn is_operator(event_type: EventType) -> bool {
    matches!(
        event_type,
        EventType::OperatorCommanded | EventType::OperatorAccepted | EventType::OperatorRejected
    )
}

/// Every park: it waits on the operator until their next command, and
/// stays parked until the run moves again, at `now` if it has not.
fn parks(events: &[EventEnvelope], now: i64) -> Vec<Park> {
    let mut phase: Option<String> = None;
    let mut open: Option<(Park, Option<i64>)> = None;
    let mut parks = Vec::new();
    for event in events {
        let Some(time) = at(event) else { continue };
        if let Some((park, answered)) = open.as_mut() {
            if is_operator(event.event_type) {
                if event.event_type == EventType::OperatorCommanded && answered.is_none() {
                    *answered = Some(time);
                }
                continue;
            }
            park.ended = time;
            park.answered = answered.unwrap_or(time);
            parks.extend(open.take().map(|(park, _)| park));
        }
        match event.event_type {
            EventType::PhaseEntered => phase = field(&event.payload, "phase").map(str::to_string),
            EventType::RunParked => {
                let park = Park {
                    phase: phase.clone(),
                    at: time,
                    answered: now,
                    ended: now,
                };
                open = Some((park, None));
            }
            _ => {}
        }
    }
    parks.extend(open.map(|(mut park, answered)| {
        park.answered = answered.unwrap_or(now);
        park
    }));
    parks
}

fn facts(run: &AnalyticsRun, label: &str, now: i64) -> RunFacts {
    let Attempts { costs, sites } = attempts(run.events);
    let read = aggregate(run.events, run.state);
    RunFacts {
        realm: read.realm.unwrap_or_else(|| label.to_string()),
        recipe: read.recipe,
        readable: run.state.is_some(),
        open: run.state.is_some_and(is_open),
        started: read.started_ms,
        finished: read.finished,
        parks: parks(run.events, now),
        implement_visits: read.implement_visits,
        reviews: Reviews {
            first: read.review_first,
            second: read.review_second,
            back: read.review_back,
        },
        costs: costs.iter().map(|spend| (spend.at, spend.usd)).collect(),
        sites,
    }
}

// ------------------------------------------------------------ the scope

/// The runs, realms and refusals the query's realm keeps.
#[derive(Default)]
pub(super) struct Scoped {
    pub(super) runs: Vec<RunFacts>,
    pub(super) realms: Vec<String>,
    pub(super) unreadable: BTreeMap<String, usize>,
    pub(super) quarantined: Vec<QuarantinedHearth>,
}

impl Scoped {
    pub(super) fn read(query: &Query, now: i64) -> Scoped {
        let mut scoped = Scoped::default();
        for hearth in query.hearths {
            let named: Vec<&str> = match hearth.realms.is_empty() {
                true => vec![hearth.label],
                false => hearth.realms.iter().map(String::as_str).collect(),
            };
            if let Some(refusal) = hearth.refusal {
                if named.iter().any(|realm| in_scope(query, realm)) {
                    scoped.quarantined.push(QuarantinedHearth {
                        realm: hearth.label.to_string(),
                        journal: hearth.journal.to_string(),
                        refusal: refusal.to_string(),
                    });
                }
                continue;
            }
            for realm in named {
                scoped.name(query, realm);
            }
            for run in hearth.runs {
                scoped.keep(query, facts(run, hearth.label, now));
            }
        }
        scoped
    }

    fn name(&mut self, query: &Query, realm: &str) {
        if in_scope(query, realm) && !self.realms.iter().any(|named| named == realm) {
            self.realms.push(realm.to_string());
        }
    }

    /// A run in the query's realm is kept; one whose journal does not
    /// load or fold is counted, and no figure reads it.
    fn keep(&mut self, query: &Query, run: RunFacts) {
        if !in_scope(query, &run.realm) {
            return;
        }
        self.name(query, &run.realm);
        match run.readable {
            true => self.runs.push(run),
            false => *self.unreadable.entry(run.realm).or_default() += 1,
        }
    }
}

fn in_scope(query: &Query, realm: &str) -> bool {
    query.realm.is_none_or(|wanted| wanted == realm)
}
