//! The derivations `brokkr costs` and `brokkr compare` print: per-seat
//! accounting from the journal, and where two runs diverged. Derived here
//! once, so no surface sums a seat's spend or aligns two trails its own
//! way (decision 0013; #351).

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::{EventEnvelope, EventType};
use serde::Serialize;
use serde_json::Value;

use crate::reported_cost;

/// The token counters one record can carry, by the seat-record names.
const USAGE_KEYS: [&str; 5] = [
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_output_tokens",
];

/// Each counter prints under its [`USAGE_KEYS`] name, and only when a
/// record reported it.
#[derive(Default, Serialize)]
struct Usage {
    #[serde(rename = "input_tokens", skip_serializing_if = "Option::is_none")]
    input: Option<u64>,
    #[serde(rename = "output_tokens", skip_serializing_if = "Option::is_none")]
    output: Option<u64>,
    #[serde(rename = "cache_read_tokens", skip_serializing_if = "Option::is_none")]
    cache_read: Option<u64>,
    #[serde(rename = "cache_write_tokens", skip_serializing_if = "Option::is_none")]
    cache_write: Option<u64>,
    /// A reported subset of `output`, summed on its own key and
    /// never folded into it a second time (decision 0035 ruling 4)
    /// — the same treatment `cache_read` gets inside `input`.
    #[serde(
        rename = "reasoning_output_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    reasoning: Option<u64>,
}

impl Usage {
    /// Every counter beside its seat-record name, in [`USAGE_KEYS`] order.
    fn counters(&mut self) -> [(&mut Option<u64>, &'static str); 5] {
        [
            (&mut self.input, USAGE_KEYS[0]),
            (&mut self.output, USAGE_KEYS[1]),
            (&mut self.cache_read, USAGE_KEYS[2]),
            (&mut self.cache_write, USAGE_KEYS[3]),
            (&mut self.reasoning, USAGE_KEYS[4]),
        ]
    }

    fn add_record(&mut self, record: &Value) {
        for (total, key) in self.counters() {
            if let Some(value) = record.get(key).and_then(Value::as_u64) {
                *total = Some(total.unwrap_or_default().saturating_add(value));
            }
        }
    }

    /// One list, as `add_record` and `merge` below already read one:
    /// every counter this record can hold, asked the same question.
    /// A `||` chain would make each counter its own branch and claim
    /// a harness shape nobody reports — "cache writes and nothing
    /// else" — as a case worth proving.
    fn has_any(&self) -> bool {
        [
            self.input,
            self.output,
            self.cache_read,
            self.cache_write,
            self.reasoning,
        ]
        .iter()
        .any(Option::is_some)
    }

    fn merge(&mut self, other: &Self) {
        for (total, value) in [
            (&mut self.input, other.input),
            (&mut self.output, other.output),
            (&mut self.cache_read, other.cache_read),
            (&mut self.cache_write, other.cache_write),
            (&mut self.reasoning, other.reasoning),
        ] {
            if let Some(value) = value {
                *total = Some(total.unwrap_or_default().saturating_add(value));
            }
        }
    }
}

#[derive(Default)]
struct EffectAccounting {
    attempts: u64,
    turns: u64,
    cost: f64,
    models: BTreeSet<String>,
    /// The efforts this effect was CONFIGURED with, gathered exactly
    /// as the models beside them are: both harnesses that echo one
    /// write it per turn, so an effect that changed level mid-thread
    /// reports both rather than the last.
    efforts: BTreeSet<String>,
    /// The boundaries the records that name a model carry beside it
    /// (decision 0046 ruling 3; design DD19), gathered exactly as
    /// the models are: from the seat records themselves, never from
    /// the `effect/started` entries (design DD14).
    boundaries: BTreeSet<String>,
    turns_usage: Usage,
    finishing_usage: Usage,
}

/// The word a record carries beside its model, if it names one: a
/// record without a model carries no boundary a reader may trust
/// (design DD19), so the pair is read together.
fn boundary_beside_model(record: &Value) -> Option<&str> {
    record.get("model").and_then(Value::as_str)?;
    record.get("boundary").and_then(Value::as_str)
}

impl EffectAccounting {
    /// The model, the boundary beside it and the effort a checkpoint or
    /// a result names.
    fn add_claims(&mut self, record: &Value) {
        if let Some(model) = record.get("model").and_then(Value::as_str) {
            self.models.insert(model.to_string());
        }
        if let Some(boundary) = boundary_beside_model(record) {
            self.boundaries.insert(boundary.to_string());
        }
        if let Some(effort) = record.get("effort").and_then(Value::as_str) {
            self.efforts.insert(effort.to_string());
        }
    }

    fn add_checkpoint(&mut self, checkpoint: &Value) {
        self.turns += checkpoint
            .get("num_turns")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        // Every attempt's report is summed: the rule the
        // view states once, and reads as its `cost` (#376).
        self.cost += reported_cost(checkpoint).unwrap_or(0.0);
        self.add_claims(checkpoint);
        if matches!(
            checkpoint.get("step").and_then(Value::as_str),
            Some("seat-turn" | "turn-completed")
        ) {
            self.turns_usage.add_record(checkpoint);
        } else if USAGE_KEYS.iter().any(|key| checkpoint.get(*key).is_some()) {
            self.finishing_usage = Usage::default();
            self.finishing_usage.add_record(checkpoint);
        }
    }

    fn add_result(&mut self, result: &Value) {
        self.add_claims(result);
        if !self.finishing_usage.has_any() {
            self.finishing_usage.add_record(result);
        }
        if self.turns == 0 {
            self.turns = result.get("num_turns").and_then(Value::as_u64).unwrap_or(0);
        }
        if self.cost == 0.0 {
            self.cost = reported_cost(result).unwrap_or(0.0);
        }
    }
}

/// Each seated effect's accounting, by effect id, beside the seat each
/// `effect/requested` names: attempts from `effect/started`, turns, cost
/// and usage from the checkpoints and the result. Evidence for an effect
/// no request seated is not read.
fn effects_of(
    events: &[EventEnvelope],
) -> (BTreeMap<String, String>, BTreeMap<String, EffectAccounting>) {
    let mut effect_seat: BTreeMap<String, String> = BTreeMap::new();
    let mut effects: BTreeMap<String, EffectAccounting> = BTreeMap::new();
    for event in events {
        let payload = &event.payload;
        let id = payload.get("effect_id").and_then(Value::as_str);
        if event.event_type == EventType::EffectRequested {
            if let (Some(id), Some(seat)) = (id, payload.get("seat").and_then(Value::as_str)) {
                effect_seat.insert(id.to_string(), seat.to_string());
                effects.entry(id.to_string()).or_default();
            }
            continue;
        }
        let Some(accounting) = id
            .filter(|id| effect_seat.contains_key(*id))
            .and_then(|id| effects.get_mut(id))
        else {
            continue;
        };
        match event.event_type {
            EventType::EffectStarted => accounting.attempts += 1,
            EventType::EffectCheckpointed => accounting.add_checkpoint(&payload["checkpoint"]),
            EventType::EffectSucceeded => accounting.add_result(&payload["result"]),
            _ => {}
        }
    }
    (effect_seat, effects)
}

#[derive(Default)]
struct SeatAccounting {
    attempts: u64,
    turns: u64,
    cost: f64,
    models: BTreeSet<String>,
    efforts: BTreeSet<String>,
    boundaries: BTreeSet<String>,
    usage: Usage,
}

impl SeatAccounting {
    fn add(&mut self, effect: &EffectAccounting) {
        self.attempts += effect.attempts;
        self.turns += effect.turns;
        self.cost += effect.cost;
        self.models.extend(effect.models.iter().cloned());
        self.efforts.extend(effect.efforts.iter().cloned());
        self.boundaries.extend(effect.boundaries.iter().cloned());
        self.usage.merge(if effect.turns_usage.has_any() {
            &effect.turns_usage
        } else {
            &effect.finishing_usage
        });
    }
}

/// The sentinels of decision 0031 are what a harness reports when it
/// names no model. A seat that also served a real model reports that one
/// alone; a seat that only ever reported a sentinel keeps the sentinel.
/// The same reduction serves both axes: decision 0035 reuses 0031's two
/// sentinels for the effort rather than inventing a second pair, so "a
/// real value outranks a sentinel, and a seat that only ever reported a
/// sentinel keeps it" is one rule stated once.
/// The boundary is reduced by the same rule (decision 0046 ruling 3),
/// with its own word for the empty set: `not recorded` is an explicit
/// absence — a journal written before the boundary was named — and never
/// a default.
fn reduce(mut values: BTreeSet<String>, none: &str) -> String {
    let mut named = values.clone();
    named.remove("not reported");
    named.remove("not applicable");
    if !named.is_empty() {
        values = named;
    }
    match values.len() {
        0 => none.to_string(),
        1 => values.into_iter().next().expect("one value"),
        _ => values.into_iter().collect::<Vec<_>>().join(", "),
    }
}

/// One seat's record, as `brokkr costs` and `brokkr compare` print it.
#[derive(Serialize)]
pub struct SeatCost {
    attempts: u64,
    turns: u64,
    cost_usd: f64,
    // `model` is the provider's claim, not proof (decision
    // 0035 ruling 2); `boundary` beside it is the plain word
    // its hands stood behind (decision 0046 ruling 3);
    // `effort` is configuration, never a report of what the
    // model did. The one figure here that meters that is
    // `reasoning_output_tokens`.
    model: String,
    boundary: String,
    effort: String,
    #[serde(flatten)]
    usage: Usage,
}

impl SeatCost {
    fn of(accounting: SeatAccounting) -> SeatCost {
        SeatCost {
            attempts: accounting.attempts,
            turns: accounting.turns,
            cost_usd: accounting.cost,
            model: reduce(accounting.models, "not reported"),
            boundary: reduce(accounting.boundaries, "not recorded"),
            effort: reduce(accounting.efforts, "not reported"),
            usage: accounting.usage,
        }
    }
}

/// Per-seat attempts/turns/cost/token usage from the journal — the aggregation
/// shared by `brokkr costs` and `brokkr compare`: attempts from
/// `effect/started` joined through `effect/requested.seat`, turns and
/// cost from `effect/checkpointed` payloads. Returns the per-seat
/// report map and the total cost.
pub fn seat_costs(events: &[EventEnvelope]) -> (BTreeMap<String, SeatCost>, f64) {
    let (effect_seat, effects) = effects_of(events);
    let mut seats: BTreeMap<String, SeatAccounting> = BTreeMap::new();
    for (effect_id, seat) in effect_seat {
        // `effect/requested` writes both maps together, so every seated
        // effect carries an accounting row: there is no third state here
        // to branch on.
        let effect = effects
            .get(&effect_id)
            .expect("every seated effect carries accounting");
        seats.entry(seat).or_default().add(effect);
    }
    let total: f64 = seats.values().map(|seat| seat.cost).sum();
    let report = seats
        .into_iter()
        .map(|(seat, accounting)| (seat, SeatCost::of(accounting)))
        .collect();
    (report, total)
}

/// One site's two sides where two runs resolved it differently; a side
/// that names no entry prints as null.
#[derive(Serialize)]
pub struct Sides<T> {
    a: Option<T>,
    b: Option<T>,
}

/// Where two decision trails first part: the position, and each side's
/// rule id or "park", or "end" where that trail stopped first.
#[derive(Serialize)]
pub struct FirstDivergence {
    index: usize,
    a: String,
    b: String,
}

/// AC-18: a model difference is a FIRST-CLASS divergence, not a
/// footnote — reported unconditionally, including when `same_recipe` is
/// true. Comparing pinned plans instead of what actually ran would hide
/// precisely the fallback this exists to expose, and an absence on one
/// side is itself the finding: one run named an agent and the other did
/// not. The comparison is structural over the whole entry, so a
/// boundary difference diverges exactly as a model difference does
/// (decision 0046 ruling 3) with no code written for it.
pub fn resolution_divergence<T: Clone + PartialEq>(
    a: &BTreeMap<String, T>,
    b: &BTreeMap<String, T>,
) -> BTreeMap<String, Sides<T>> {
    let mut sites: Vec<&String> = a.keys().chain(b.keys()).collect();
    sites.sort();
    sites.dedup();
    let mut out = BTreeMap::new();
    for site in sites {
        let (left, right) = (a.get(site), b.get(site));
        if left == right {
            continue;
        }
        out.insert(
            site.clone(),
            Sides {
                a: left.cloned(),
                b: right.cloned(),
            },
        );
    }
    out
}

/// None when the trails are equal; else the first differing position,
/// each side the rule id or "park". When one trail is a strict prefix of
/// the other, the shorter side renders as "end" at index = its length.
pub fn first_divergence(a: &[String], b: &[String]) -> Option<FirstDivergence> {
    let step = |trail: &[String], i: usize| -> String {
        trail.get(i).map_or("end", String::as_str).to_string()
    };
    (0..a.len().max(b.len()))
        .find(|&i| a.get(i) != b.get(i))
        .map(|i| FirstDivergence {
            index: i,
            a: step(a, i),
            b: step(b, i),
        })
}

#[cfg(test)]
mod tests;
