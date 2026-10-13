//! What one run's journal and folded state say, read once into the facts
//! every figure is aggregated from, and the runs a query's realm keeps.

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::fold::RunState;
use brokkr_core::residual::ResidualPhase;
use brokkr_core::{EventEnvelope, EventType};

use super::{AnalyticsRun, QuarantinedHearth, Query};
use crate::clarify::pinned_realm;
use crate::fleet::{last_rule, standing_of, STOP_PHASE};
use crate::{decided, field, js, reported_cost, Standing};

/// The phase whose visits count a run's rework.
const IMPLEMENT_PHASE: &str = "implement";

/// A half-open span of time: its first moment held, its last not.
#[derive(Clone, Copy)]
pub(super) struct Window {
    pub(super) from: i64,
    pub(super) to: i64,
}

impl Window {
    pub(super) fn holds(self, at: i64) -> bool {
        at >= self.from && at < self.to
    }

    pub(super) fn overlap(self, start: i64, end: i64) -> i64 {
        (end.min(self.to) - start.max(self.from)).max(0)
    }
}

/// How a finished run ended.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum Ending {
    Shipped,
    ByRule(Option<String>),
    ByOperator,
}

/// Where a review ruling sent the run.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Route {
    Back,
    On,
    Held,
}

pub(super) struct Park {
    pub(super) phase: Option<String>,
    pub(super) at: i64,
    pub(super) answered: i64,
    pub(super) ended: i64,
}

/// One invocation site of one attempt that an agent resolution chose.
pub(super) struct Site {
    pub(super) at: i64,
    pub(super) provider: String,
    pub(super) model: Option<String>,
    pub(super) reported: bool,
}

pub(super) struct RunFacts {
    pub(super) realm: String,
    readable: bool,
    pub(super) started: Option<i64>,
    pub(super) finished: Option<(i64, Ending)>,
    pub(super) parks: Vec<Park>,
    pub(super) implement_visits: u64,
    pub(super) reviews: Vec<Route>,
    pub(super) costs: Vec<(i64, f64)>,
    pub(super) sites: Vec<Site>,
}

fn at(event: &EventEnvelope) -> Option<i64> {
    js::parse_millis(&event.recorded_at)
}

fn ending(state: &RunState) -> Option<Ending> {
    match standing_of(state) {
        Standing::Shipped => Some(Ending::Shipped),
        Standing::Stopped => Some(Ending::ByRule(
            last_rule(state).map(|(rule, _)| rule.to_string()),
        )),
        Standing::OperatorStopped => Some(Ending::ByOperator),
        Standing::Running | Standing::Parked | Standing::Quarantined => None,
    }
}

/// When the run finished and how: its terminal event's time, read beside
/// the fold's standing.
fn finished(events: &[EventEnvelope], state: Option<&RunState>) -> Option<(i64, Ending)> {
    let ending = state.and_then(ending)?;
    let terminal = events.iter().rev().find(|event| {
        matches!(
            event.event_type,
            EventType::RunCompleted | EventType::RunStopped
        )
    })?;
    Some((at(terminal)?, ending))
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

/// Where each review ruling sent the run, in journal order: back to a
/// phase it had entered, on to one it had not, or held by a park or the
/// hard stop.
fn reviews(events: &[EventEnvelope]) -> Vec<Route> {
    let review = ResidualPhase::Review.as_str();
    let mut entered: BTreeSet<&str> = BTreeSet::new();
    let mut routes = Vec::new();
    for event in events {
        if event.event_type == EventType::PhaseEntered {
            entered.extend(field(&event.payload, "phase"));
        }
        let Some(ruled) = decided(&event.payload)
            .filter(|_| event.event_type == EventType::TransitionDecided)
            .filter(|ruled| ruled.from.and_then(|from| from.as_str()) == Some(review))
        else {
            continue;
        };
        routes.push(match ruled.next.and_then(|next| next.as_str()) {
            None | Some(STOP_PHASE) => Route::Held,
            Some(next) if entered.contains(next) => Route::Back,
            Some(_) => Route::On,
        });
    }
    routes
}

/// The sites an agent resolution chose (decision 0016), with the model
/// each reported and whether it reported a cost, and every reported cost
/// with its time: the checkpoint records [`reported_cost`] reads, as
/// `brokkr costs` sums them.
#[derive(Default)]
struct Seats<'a> {
    costs: Vec<(i64, f64)>,
    sites: Vec<Site>,
    by_site: BTreeMap<(&'a str, Option<&'a str>), usize>,
}

impl<'a> Seats<'a> {
    fn read(events: &'a [EventEnvelope]) -> Seats<'a> {
        let mut seats = Seats::default();
        for event in events {
            let (Some(time), Some(attempt)) = (at(event), field(&event.payload, "attempt_id"))
            else {
                continue;
            };
            let payload = &event.payload;
            match event.event_type {
                EventType::EffectStarted => seats.started(time, attempt, payload),
                EventType::EffectCheckpointed => {
                    seats.recorded(time, attempt, payload.get("checkpoint"), true);
                }
                EventType::EffectSucceeded => {
                    seats.recorded(time, attempt, payload.get("result"), false);
                }
                _ => {}
            }
        }
        seats
    }

    /// One attempt's provenance entries, a site each.
    fn started(&mut self, time: i64, attempt: &'a str, payload: &'a serde_json::Value) {
        let entries = payload.get("provenance").and_then(|list| list.as_array());
        for entry in entries.into_iter().flatten() {
            let Some(provider) = field(entry, "provider") else {
                continue;
            };
            self.by_site
                .insert((attempt, field(entry, "member")), self.sites.len());
            self.sites.push(Site {
                at: time,
                provider: provider.to_string(),
                model: None,
                reported: false,
            });
        }
    }

    /// One record of an attempt: the model it names, and its cost when it
    /// is a checkpoint.
    fn recorded(
        &mut self,
        time: i64,
        attempt: &'a str,
        record: Option<&'a serde_json::Value>,
        priced: bool,
    ) {
        let Some(record) = record else { return };
        let cost = priced.then(|| reported_cost(record)).flatten();
        self.costs.extend(cost.map(|cost| (time, cost)));
        let key = (attempt, field(record, "member"));
        if let Some(site) = self.by_site.get(&key).map(|slot| &mut self.sites[*slot]) {
            if let Some(model) = field(record, "model") {
                site.model = Some(model.to_string());
            }
            site.reported |= cost.is_some();
        }
    }
}

fn facts(run: &AnalyticsRun, label: &str, now: i64) -> RunFacts {
    let Seats { costs, sites, .. } = Seats::read(run.events);
    RunFacts {
        realm: pinned_realm(run.events).unwrap_or_else(|| label.to_string()),
        readable: run.state.is_some(),
        started: run.events.first().and_then(at),
        finished: finished(run.events, run.state),
        parks: parks(run.events, now),
        implement_visits: run
            .state
            .and_then(|state| state.visits.get(IMPLEMENT_PHASE).copied())
            .unwrap_or(0),
        reviews: reviews(run.events),
        costs,
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
