//! What `brokkr costs` and `brokkr compare` print of each seat: its
//! attempts, turns, cost and token usage, and the models, efforts and
//! boundaries its records name (#351). Each effect's accounting is folded
//! here once (decision 0071 ruling 5), from the journal for
//! [`seat_costs`] and by the writer's seat projection, which keeps it on
//! its request's own row (decision 0089 slice 3), so the rows read back
//! the seats `seat_costs` gives ([`row::costs`](crate::seat::row::costs)).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;
use crate::seat::reported_cost;

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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Usage {
    #[serde(rename = "input_tokens", skip_serializing_if = "Option::is_none")]
    pub(crate) input: Option<u64>,
    #[serde(rename = "output_tokens", skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<u64>,
    #[serde(rename = "cache_read_tokens", skip_serializing_if = "Option::is_none")]
    pub(crate) cache_read: Option<u64>,
    #[serde(rename = "cache_write_tokens", skip_serializing_if = "Option::is_none")]
    pub(crate) cache_write: Option<u64>,
    /// A reported subset of `output`, summed on its own key and
    /// never folded into it a second time (decision 0035 ruling 4)
    /// — the same treatment `cache_read` gets inside `input`.
    #[serde(
        rename = "reasoning_output_tokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) reasoning: Option<u64>,
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

    /// Every counter, in [`USAGE_KEYS`] order.
    pub(crate) fn counts(&self) -> [Option<u64>; 5] {
        [
            self.input,
            self.output,
            self.cache_read,
            self.cache_write,
            self.reasoning,
        ]
    }

    /// One list, as `add_record` and `merge` below already read one:
    /// every counter this record can hold, asked the same question.
    /// A `||` chain would make each counter its own branch and claim
    /// a harness shape nobody reports — "cache writes and nothing
    /// else" — as a case worth proving.
    fn has_any(&self) -> bool {
        self.counts().iter().any(Option::is_some)
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

/// A cost, the same by its bits alone: a result reporting -0.0 where an
/// effect's records priced nothing replaces the 0.0 it equals as a float,
/// so the row that keeps it is not the row it was.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(transparent)]
pub(crate) struct Cost(pub(crate) f64);

impl PartialEq for Cost {
    fn eq(&self, other: &Cost) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

/// One effect's accounting, as its events fold it in journal order.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EffectAccounting {
    pub(crate) attempts: u64,
    pub(crate) turns: u64,
    pub(crate) cost: Cost,
    pub(crate) models: BTreeSet<String>,
    /// The efforts this effect was CONFIGURED with, gathered exactly
    /// as the models beside them are: both harnesses that echo one
    /// write it per turn, so an effect that changed level mid-thread
    /// reports both rather than the last.
    pub(crate) efforts: BTreeSet<String>,
    /// The boundaries the records that name a model carry beside it
    /// (decision 0046 ruling 3; design DD19), gathered exactly as
    /// the models are: from the seat records themselves, never from
    /// the `effect/started` entries (design DD14).
    pub(crate) boundaries: BTreeSet<String>,
    pub(crate) turns_usage: Usage,
    pub(crate) finishing_usage: Usage,
}

/// The word a record carries beside its model, if it names one: a
/// record without a model carries no boundary a reader may trust
/// (design DD19), so the pair is read together.
fn boundary_beside_model(record: &Value) -> Option<&str> {
    record.get("model").and_then(Value::as_str)?;
    record.get("boundary").and_then(Value::as_str)
}

impl EffectAccounting {
    /// One event of a seated effect: a start is an attempt, and its
    /// checkpoints and results are read; no other event is.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        let payload = &event.payload;
        match event.event_type {
            EventType::EffectStarted => self.attempts += 1,
            EventType::EffectCheckpointed => self.add_checkpoint(&payload["checkpoint"]),
            EventType::EffectSucceeded => self.add_result(&payload["result"]),
            EventType::RunStarted
            | EventType::PhaseEntered
            | EventType::EffectRequested
            | EventType::EffectFailed
            | EventType::EffectIndeterminate
            | EventType::TransitionDecided
            | EventType::OperatorCommanded
            | EventType::OperatorAccepted
            | EventType::OperatorRejected
            | EventType::RunParked
            | EventType::RunCompleted
            | EventType::RunStopped => {}
        }
    }

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
        // The contract bounds a count below and never above, so a sum
        // past `u64::MAX` saturates as the token counts do: a sealed
        // journal folds, never panics or wraps (decision 0071 ruling 8).
        self.turns = self.turns.saturating_add(
            checkpoint
                .get("num_turns")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        );
        // Every attempt's report is summed: the rule the
        // view states once, and reads as its `cost` (#376).
        self.cost.0 += reported_cost(checkpoint).unwrap_or(0.0);
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
        if self.cost.0 == 0.0 {
            self.cost.0 = reported_cost(result).unwrap_or(0.0);
        }
    }
}

/// Each seated effect's accounting, by effect id, beside the seat the
/// last `effect/requested` naming one names: attempts from
/// `effect/started`, turns, cost and usage from the checkpoints and the
/// result. Evidence for an effect no request seated is not read.
fn effects_of(events: &[EventEnvelope]) -> BTreeMap<String, (String, EffectAccounting)> {
    let mut effects: BTreeMap<String, (String, EffectAccounting)> = BTreeMap::new();
    for event in events {
        let payload = &event.payload;
        let id = field(payload, "effect_id");
        if event.event_type == EventType::EffectRequested {
            if let (Some(id), Some(seat)) = (id, field(payload, "seat")) {
                let effect = effects.entry(id.to_string()).or_default();
                effect.0 = seat.to_string();
            }
            continue;
        }
        if let Some((_, accounting)) = id.and_then(|id| effects.get_mut(id)) {
            accounting.note(event);
        }
    }
    effects
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
        self.attempts = self.attempts.saturating_add(effect.attempts);
        self.turns = self.turns.saturating_add(effect.turns);
        self.cost += effect.cost.0;
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
    // One value joins as itself.
    match values.len() {
        0 => none.to_string(),
        _ => values.into_iter().collect::<Vec<_>>().join(", "),
    }
}

/// One seat's record, as `brokkr costs` and `brokkr compare` print it,
/// equal to another only where its cost is the other's by its bits.
#[derive(Debug, PartialEq, Serialize)]
pub struct SeatCost {
    attempts: u64,
    turns: u64,
    cost_usd: Cost,
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
            cost_usd: Cost(accounting.cost),
            model: reduce(accounting.models, "not reported"),
            boundary: reduce(accounting.boundaries, "not recorded"),
            effort: reduce(accounting.efforts, "not reported"),
            usage: accounting.usage,
        }
    }
}

/// Each seat's record and the total cost, from `effects`, each seated
/// effect's seat and accounting in effect id order: the one sum
/// [`seat_costs`] and the rows' reader make.
pub(crate) fn summed<'a>(
    effects: impl Iterator<Item = (&'a str, &'a EffectAccounting)>,
) -> (BTreeMap<String, SeatCost>, f64) {
    let mut seats: BTreeMap<String, SeatAccounting> = BTreeMap::new();
    for (seat, effect) in effects {
        seats.entry(seat.to_string()).or_default().add(effect);
    }
    let total: f64 = seats.values().map(|seat| seat.cost).sum();
    let report = seats
        .into_iter()
        .map(|(seat, accounting)| (seat, SeatCost::of(accounting)))
        .collect();
    (report, total)
}

/// Per-seat attempts/turns/cost/token usage from the journal — the aggregation
/// shared by `brokkr costs` and `brokkr compare`: attempts from
/// `effect/started` joined through `effect/requested.seat`, turns and
/// cost from `effect/checkpointed` payloads. Returns the per-seat
/// report map and the total cost.
pub fn seat_costs(events: &[EventEnvelope]) -> (BTreeMap<String, SeatCost>, f64) {
    let effects = effects_of(events);
    summed(
        effects
            .values()
            .map(|(seat, accounting)| (seat.as_str(), accounting)),
    )
}

#[cfg(test)]
mod tests;
