//! Every attempt's seats as the journal records them (decision 0016): the
//! sites an agent resolution chose, with the provider each ran on, the
//! model it reported, and each checkpoint it metered — its time and phase,
//! its reported cost and its tokens; and every reported cost with its time
//! and phase. One walk, read by the analytics readout and the recipe's
//! graph alike (decision 0071 ruling 5).

use std::collections::BTreeMap;

use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use crate::meter::{self, sum_usage, Meter, Metering};
use crate::{field, js, reported_cost, TokenUsage};

/// One checkpoint of a site, read once at its edge.
struct Metered {
    at: i64,
    /// The phase the run was in when the checkpoint was recorded.
    phase: Option<String>,
    reported: bool,
    meter: Meter,
    /// Whether it is the site's finishing session record.
    session: bool,
}

/// One invocation site of one attempt that an agent resolution chose.
pub(crate) struct Site {
    pub(crate) at: i64,
    /// The phase the run was in when the attempt started.
    pub(crate) phase: Option<String>,
    pub(crate) provider: String,
    pub(crate) model: Option<String>,
    metered: Vec<Metered>,
    by: Metering,
}

/// Which moments a figure keeps, by a time and the phase the run was in
/// then: the window alone, or the window and one state.
pub(crate) type Keeps<'a> = &'a dyn Fn(i64, Option<&str>) -> bool;

impl Site {
    /// Whether the site ran in what `keeps` holds: it started there, or
    /// metered a checkpoint there.
    pub(crate) fn ran(&self, keeps: Keeps) -> bool {
        keeps(self.at, self.phase.as_deref()) || self.kept(keeps).next().is_some()
    }

    /// Whether a checkpoint `keeps` holds reported a cost.
    pub(crate) fn reported(&self, keeps: Keeps) -> bool {
        self.kept(keeps).any(|metered| metered.reported)
    }

    /// What the checkpoints `keeps` holds metered, by the rule
    /// [`Metering`] states.
    pub(crate) fn usage(&self, keeps: Keeps) -> Option<TokenUsage> {
        let counted = self.kept(keeps);
        sum_usage(counted.filter_map(|metered| metered.meter.counted(self.by, metered.session)))
    }

    fn kept<'a>(&'a self, keeps: Keeps<'a>) -> impl Iterator<Item = &'a Metered> + 'a {
        let metered = self.metered.iter();
        metered.filter(move |metered| keeps(metered.at, metered.phase.as_deref()))
    }
}

/// One reported cost: the checkpoint records [`reported_cost`] reads, as
/// `brokkr costs` sums them.
pub(crate) struct Spend {
    pub(crate) at: i64,
    pub(crate) phase: Option<String>,
    pub(crate) usd: f64,
}

/// What one run's attempts recorded.
pub(crate) struct Attempts {
    pub(crate) costs: Vec<Spend>,
    pub(crate) sites: Vec<Site>,
}

#[derive(Default)]
struct Walk<'a> {
    phase: Option<&'a str>,
    costs: Vec<Spend>,
    sites: Vec<Site>,
    by_site: BTreeMap<(&'a str, Option<&'a str>), usize>,
}

impl<'a> Walk<'a> {
    /// One attempt's provenance entries, a site each.
    fn started(&mut self, at: i64, attempt: &'a str, payload: &'a Value) {
        let entries = payload.get("provenance").and_then(Value::as_array);
        for entry in entries.into_iter().flatten() {
            let Some(provider) = field(entry, "provider") else {
                continue;
            };
            self.by_site
                .insert((attempt, field(entry, "member")), self.sites.len());
            self.sites.push(Site {
                at,
                phase: self.phase.map(str::to_string),
                provider: provider.to_string(),
                model: None,
                metered: Vec::new(),
                by: Metering::Session,
            });
        }
    }

    /// One record of an attempt: the model it names, and its cost and
    /// tokens when it is a checkpoint.
    fn recorded(&mut self, at: i64, attempt: &'a str, record: Option<&'a Value>, priced: bool) {
        let Some(record) = record else { return };
        let cost = priced.then(|| reported_cost(record)).flatten();
        let phase = self.phase.map(str::to_string);
        self.costs.extend(cost.map(|usd| Spend {
            at,
            phase: phase.clone(),
            usd,
        }));
        let Some(slot) = self.by_site.get(&(attempt, field(record, "member"))) else {
            return;
        };
        let site = &mut self.sites[*slot];
        if let Some(model) = field(record, "model") {
            site.model = Some(model.to_string());
        }
        if priced {
            let session =
                field(record, "step").is_some_and(|step| step.ends_with("-session-finished"));
            site.metered.push(Metered {
                at,
                phase,
                reported: cost.is_some(),
                meter: Meter::of(record),
                session,
            });
        }
    }
}

/// Only the last finishing record a site wrote is its session record.
fn settle(mut site: Site) -> Site {
    let last = site.metered.iter().rposition(|metered| metered.session);
    for (index, metered) in site.metered.iter_mut().enumerate() {
        metered.session = Some(index) == last;
    }
    meter::once(site.metered.iter_mut().map(|metered| &mut metered.meter));
    site.by = Metering::of(site.metered.iter().map(|metered| &metered.meter));
    site
}

/// Every site and reported cost of one run's journal.
pub(crate) fn attempts(events: &[EventEnvelope]) -> Attempts {
    let mut walk = Walk::default();
    for event in events {
        let payload = &event.payload;
        if event.event_type == EventType::PhaseEntered {
            walk.phase = field(payload, "phase");
            continue;
        }
        let (Some(at), Some(attempt)) = (
            js::parse_millis(&event.recorded_at),
            field(payload, "attempt_id"),
        ) else {
            continue;
        };
        match event.event_type {
            EventType::EffectStarted => walk.started(at, attempt, payload),
            EventType::EffectCheckpointed => {
                walk.recorded(at, attempt, payload.get("checkpoint"), true);
            }
            EventType::EffectSucceeded => walk.recorded(at, attempt, payload.get("result"), false),
            _ => {}
        }
    }
    Attempts {
        costs: walk.costs,
        sites: walk.sites.into_iter().map(settle).collect(),
    }
}
