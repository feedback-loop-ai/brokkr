//! Every attempt's seats as the journal records them (decision 0016): the
//! sites an agent resolution chose, with the provider each ran on, the
//! model it reported, whether it reported a cost, the tokens it spent and
//! the phase it ran in; and every reported cost with its time and phase.
//! One walk, read by the analytics readout and the recipe's graph alike
//! (decision 0071 ruling 5).

use std::collections::BTreeMap;

use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use crate::{field, js, reported_cost, seat_usage, TokenUsage};

/// One invocation site of one attempt that an agent resolution chose.
pub(crate) struct Site {
    pub(crate) at: i64,
    /// The phase the run was in when the attempt started.
    pub(crate) phase: Option<String>,
    pub(crate) provider: String,
    pub(crate) model: Option<String>,
    pub(crate) reported: bool,
    /// What the site's records metered, by the rule [`seat_usage`] states.
    pub(crate) usage: Option<TokenUsage>,
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

/// A site's checkpoints, and the finishing one among them.
#[derive(Default)]
struct Records<'a> {
    checkpoints: Vec<&'a Value>,
    session: Option<&'a Value>,
}

#[derive(Default)]
struct Walk<'a> {
    phase: Option<&'a str>,
    costs: Vec<Spend>,
    sites: Vec<Site>,
    records: Vec<Records<'a>>,
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
                reported: false,
                usage: None,
            });
            self.records.push(Records::default());
        }
    }

    /// One record of an attempt: the model it names, and its cost and
    /// tokens when it is a checkpoint.
    fn recorded(&mut self, at: i64, attempt: &'a str, record: Option<&'a Value>, priced: bool) {
        let Some(record) = record else { return };
        let cost = priced.then(|| reported_cost(record)).flatten();
        self.costs.extend(cost.map(|usd| Spend {
            at,
            phase: self.phase.map(str::to_string),
            usd,
        }));
        let Some(slot) = self.by_site.get(&(attempt, field(record, "member"))) else {
            return;
        };
        let site = &mut self.sites[*slot];
        if let Some(model) = field(record, "model") {
            site.model = Some(model.to_string());
        }
        site.reported |= cost.is_some();
        if priced {
            let records = &mut self.records[*slot];
            records.checkpoints.push(record);
            if field(record, "step").is_some_and(|step| step.ends_with("-session-finished")) {
                records.session = Some(record);
            }
        }
    }
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
    let Walk {
        costs,
        sites,
        records,
        ..
    } = walk;
    let sites = sites.into_iter().zip(records).map(|(mut site, records)| {
        site.usage = seat_usage(records.checkpoints.into_iter(), records.session);
        site
    });
    Attempts {
        costs,
        sites: sites.collect(),
    }
}
