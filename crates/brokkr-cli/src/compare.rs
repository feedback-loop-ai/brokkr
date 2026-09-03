//! `brokkr compare` — aligned outcome comparison of two runs: the payoff
//! of the recipe library and `brokkr rerun` ("same feature, different
//! strategy, what diverged?") as a single read-only command. Everything
//! is derived from `fold(events)`, the raw journal, and the stored
//! manifest — no bundle recompilation, no writes, no timestamp-derived
//! semantics (`recorded_at` is evidence only).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use brokkr_core::fold::fold;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::Store;
use serde_json::{json, Map, Value};

/// Per-seat attempts/turns/cost/token usage from the journal — the aggregation
/// shared by `brokkr costs` and `brokkr compare`: attempts from
/// `effect/started` joined through `effect/requested.seat`, turns and
/// cost from `effect/checkpointed` payloads. Returns the per-seat
/// report map and the total cost.
pub fn seat_costs(events: &[EventEnvelope]) -> (Map<String, Value>, f64) {
    #[derive(Default)]
    struct Usage {
        input: Option<u64>,
        output: Option<u64>,
        cache_read: Option<u64>,
        cache_write: Option<u64>,
        /// A reported subset of `output`, summed on its own key and
        /// never folded into it a second time (decision 0035 ruling 4)
        /// — the same treatment `cache_read` gets inside `input`.
        reasoning: Option<u64>,
    }
    impl Usage {
        fn add_record(&mut self, record: &Value) {
            for (total, key) in [
                (&mut self.input, "input_tokens"),
                (&mut self.output, "output_tokens"),
                (&mut self.cache_read, "cache_read_tokens"),
                (&mut self.cache_write, "cache_write_tokens"),
                (&mut self.reasoning, "reasoning_output_tokens"),
            ] {
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
        turns_usage: Usage,
        finishing_usage: Usage,
    }

    #[derive(Default)]
    struct SeatAccounting {
        attempts: u64,
        turns: u64,
        cost: f64,
        models: BTreeSet<String>,
        efforts: BTreeSet<String>,
        usage: Usage,
    }

    let mut effect_seat: BTreeMap<String, String> = BTreeMap::new();
    let mut effects: BTreeMap<String, EffectAccounting> = BTreeMap::new();
    for event in events {
        let payload = &event.payload;
        match event.event_type {
            EventType::EffectRequested => {
                if let (Some(id), Some(seat)) = (
                    payload.get("effect_id").and_then(Value::as_str),
                    payload.get("seat").and_then(Value::as_str),
                ) {
                    effect_seat.insert(id.to_string(), seat.to_string());
                    effects.entry(id.to_string()).or_default();
                }
            }
            EventType::EffectStarted => {
                if let Some(id) = payload.get("effect_id").and_then(Value::as_str) {
                    if effect_seat.contains_key(id) {
                        effects.entry(id.to_string()).or_default().attempts += 1;
                    }
                }
            }
            EventType::EffectCheckpointed => {
                if let Some(accounting) = payload
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .filter(|id| effect_seat.contains_key(*id))
                    .and_then(|id| effects.get_mut(id))
                {
                    let checkpoint = &payload["checkpoint"];
                    accounting.turns += checkpoint
                        .get("num_turns")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    accounting.cost += checkpoint
                        .get("total_cost_usd")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    if let Some(model) = checkpoint.get("model").and_then(Value::as_str) {
                        accounting.models.insert(model.to_string());
                    }
                    if let Some(effort) = checkpoint.get("effort").and_then(Value::as_str) {
                        accounting.efforts.insert(effort.to_string());
                    }
                    if matches!(
                        checkpoint.get("step").and_then(Value::as_str),
                        Some("seat-turn" | "turn-completed")
                    ) {
                        accounting.turns_usage.add_record(checkpoint);
                    } else if [
                        "input_tokens",
                        "output_tokens",
                        "cache_read_tokens",
                        "cache_write_tokens",
                        "reasoning_output_tokens",
                    ]
                    .iter()
                    .any(|key| checkpoint.get(*key).is_some())
                    {
                        accounting.finishing_usage = Usage::default();
                        accounting.finishing_usage.add_record(checkpoint);
                    }
                }
            }
            EventType::EffectSucceeded => {
                let accounting = payload
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .filter(|id| effect_seat.contains_key(*id))
                    .and_then(|id| effects.get_mut(id));
                if let Some(accounting) = accounting {
                    let result = &payload["result"];
                    if let Some(model) = result.get("model").and_then(Value::as_str) {
                        accounting.models.insert(model.to_string());
                    }
                    if let Some(effort) = result.get("effort").and_then(Value::as_str) {
                        accounting.efforts.insert(effort.to_string());
                    }
                    if !accounting.finishing_usage.has_any() {
                        accounting.finishing_usage.add_record(result);
                    }
                    if accounting.turns == 0 {
                        accounting.turns =
                            result.get("num_turns").and_then(Value::as_u64).unwrap_or(0);
                    }
                    if accounting.cost == 0.0 {
                        accounting.cost = result
                            .get("total_cost_usd")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                    }
                }
            }
            _ => {}
        }
    }
    let mut seats: BTreeMap<String, SeatAccounting> = BTreeMap::new();
    for (effect_id, seat) in effect_seat {
        // `effect/requested` writes both maps together, so every seated
        // effect carries an accounting row: there is no third state here
        // to branch on.
        let effect = effects
            .get(&effect_id)
            .expect("every seated effect carries accounting");
        let entry = seats.entry(seat).or_default();
        entry.attempts += effect.attempts;
        entry.turns += effect.turns;
        entry.cost += effect.cost;
        entry.models.extend(effect.models.iter().cloned());
        entry.efforts.extend(effect.efforts.iter().cloned());
        entry.usage.merge(if effect.turns_usage.has_any() {
            &effect.turns_usage
        } else {
            &effect.finishing_usage
        });
    }
    let total: f64 = seats.values().map(|seat| seat.cost).sum();
    let report: Map<String, Value> = seats
        .into_iter()
        .map(|(seat, accounting)| {
            // The sentinels of decision 0031 are what a harness reports
            // when it names no model. A seat that also served a real
            // model reports that one alone; a seat that only ever
            // reported a sentinel keeps the sentinel.
            // The same reduction serves both axes: decision 0035 reuses
            // 0031's two sentinels for the effort rather than inventing
            // a second pair, so "a real value outranks a sentinel, and a
            // seat that only ever reported a sentinel keeps it" is one
            // rule stated once.
            let reduce = |mut values: BTreeSet<String>| {
                let mut named = values.clone();
                named.remove("not reported");
                named.remove("not applicable");
                if !named.is_empty() {
                    values = named;
                }
                match values.len() {
                    0 => "not reported".to_string(),
                    1 => values.into_iter().next().expect("one value"),
                    _ => values.into_iter().collect::<Vec<_>>().join(", "),
                }
            };
            let model = reduce(accounting.models);
            let effort = reduce(accounting.efforts);
            let mut record = Map::from_iter([
                ("attempts".to_string(), Value::from(accounting.attempts)),
                ("turns".to_string(), Value::from(accounting.turns)),
                ("cost_usd".to_string(), Value::from(accounting.cost)),
                // `model` is the provider's claim, not proof (decision
                // 0035 ruling 2); `effort` beside it is configuration,
                // never a report of what the model did. The one figure
                // here that meters that is `reasoning_output_tokens`.
                ("model".to_string(), Value::from(model)),
                ("effort".to_string(), Value::from(effort)),
            ]);
            for (key, value) in [
                ("input_tokens", accounting.usage.input),
                ("output_tokens", accounting.usage.output),
                ("cache_read_tokens", accounting.usage.cache_read),
                ("cache_write_tokens", accounting.usage.cache_write),
                ("reasoning_output_tokens", accounting.usage.reasoning),
            ] {
                if let Some(value) = value {
                    record.insert(key.to_string(), Value::from(value));
                }
            }
            (seat, Value::Object(record))
        })
        .collect();
    (report, total)
}

/// One run's section of the report plus the facts the comparison needs.
struct RunFacts {
    summary: Value,
    feature: Option<String>,
    digest: String,
    status: &'static str,
    trail: Vec<String>,
    total_cost: f64,
    attempts: u64,
    /// Participant label → the provider-reported model and the distinct
    /// agent-selection provenance.
    /// Computed by CALLING the `brokkr-view` derivation rather than
    /// re-deriving here, so `compare` cannot describe a fallback
    /// differently from every other readout.
    resolution: BTreeMap<String, Value>,
}

/// What each run's invocation sites resolved to, keyed by participant
/// label so a panel member and a sequence step line up across runs.
fn resolution_of(events: &[EventEnvelope]) -> BTreeMap<String, Value> {
    brokkr_view::run_view(events, None)
        .participants
        .into_iter()
        .map(|part| {
            let selected = part.provenance.map(|provenance| {
                json!({
                    "agent": provenance.agent,
                    "model": provenance.model,
                    "provider": provenance.provider,
                    "chain_index": provenance.chain_index,
                    "fallback": provenance.fallback,
                })
            });
            (
                part.label,
                json!({"model": part.model.text, "selected": selected}),
            )
        })
        .collect()
}

/// AC-18: a model difference is a FIRST-CLASS divergence, not a
/// footnote — reported unconditionally, including when `same_recipe` is
/// true. Comparing pinned plans instead of what actually ran would hide
/// precisely the fallback this exists to expose, and an absence on one
/// side is itself the finding: one run named an agent and the other did
/// not.
fn resolution_divergence(a: &BTreeMap<String, Value>, b: &BTreeMap<String, Value>) -> Value {
    let mut sites: Vec<&String> = a.keys().chain(b.keys()).collect();
    sites.sort();
    sites.dedup();
    let mut out = Map::new();
    for site in sites {
        let (left, right) = (a.get(site), b.get(site));
        if left == right {
            continue;
        }
        out.insert(
            site.clone(),
            json!({
                "a": left.cloned().unwrap_or(Value::Null),
                "b": right.cloned().unwrap_or(Value::Null),
            }),
        );
    }
    Value::Object(out)
}

fn run_facts(store: &Store, run_id: &str) -> Result<RunFacts> {
    let events = store
        .load(run_id)
        .context(format!("loading run '{run_id}'"))?;
    let manifest = store
        .manifest(run_id)
        .context(format!("loading manifest for run '{run_id}'"))?;
    let state = fold(&events).context(format!("folding run '{run_id}'"))?;

    let feature = events
        .first()
        .filter(|e| e.event_type == EventType::RunStarted)
        .and_then(|e| e.payload.get("feature"))
        .and_then(Value::as_str)
        .map(str::to_string);

    // The stored manifest is the pinned bundle identity; hashing it is
    // byte-identical to Bundle::manifest_digest at pin time.
    let digest = brokkr_core::canonical::sha256_hex(&manifest);

    // A null rule_id is a park fact (decision 0001); "park" is a display
    // convention only.
    let mut trail = Vec::new();
    let mut effect_phase: BTreeMap<String, String> = BTreeMap::new();
    let mut phases: BTreeMap<String, u64> = BTreeMap::new();
    let mut attempts: u64 = 0;
    for event in &events {
        let payload = &event.payload;
        match event.event_type {
            EventType::TransitionDecided => trail.push(
                payload
                    .get("rule_id")
                    .and_then(Value::as_str)
                    .unwrap_or("park")
                    .to_string(),
            ),
            EventType::EffectRequested => {
                if let (Some(id), Some(phase)) = (
                    payload.get("effect_id").and_then(Value::as_str),
                    payload.get("phase").and_then(Value::as_str),
                ) {
                    effect_phase.insert(id.to_string(), phase.to_string());
                }
            }
            EventType::EffectStarted => {
                attempts += 1;
                if let Some(phase) = payload
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .and_then(|id| effect_phase.get(id))
                {
                    *phases.entry(phase.clone()).or_default() += 1;
                }
            }
            _ => {}
        }
    }

    let (seats, total_cost) = seat_costs(&events);
    let resolution = resolution_of(&events);
    let status = crate::status_str(&state.status);
    let summary = json!({
        "feature": feature,
        "bundle_name": manifest.get("bundle_name").cloned().unwrap_or(Value::Null),
        "manifest": {"sha256": digest},
        "status": status,
        "phase": state.phase,
        "park_reason": state.park_reason,
        "decision_trail": trail,
        "phases_visited": phases,
        "seats": seats,
        "resolution": resolution,
        "total_cost_usd": total_cost,
        "first_recorded_at": events.first().map(|e| e.recorded_at.clone()),
        "last_recorded_at": events.last().map(|e| e.recorded_at.clone()),
        "events": events.len(),
    });
    Ok(RunFacts {
        summary,
        feature,
        digest,
        status,
        trail,
        total_cost,
        attempts,
        resolution,
    })
}

/// Null when the trails are equal; else the first differing position,
/// each side the rule id or "park". When one trail is a strict prefix of
/// the other, the shorter side renders as "end" at index = its length.
fn first_divergence(a: &[String], b: &[String]) -> Value {
    let step = |trail: &[String], i: usize| -> Value {
        trail.get(i).map(|s| json!(s)).unwrap_or(json!("end"))
    };
    for i in 0..a.len().max(b.len()) {
        if a.get(i) != b.get(i) {
            return json!({"index": i, "a": step(a, i), "b": step(b, i)});
        }
    }
    Value::Null
}

pub fn compare(run_a: &str, run_b: &str, db: &Path) -> Result<()> {
    let store = Store::open(db)?;
    let a = run_facts(&store, run_a)?;
    let b = run_facts(&store, run_b)?;
    let mut runs = Map::new();
    runs.insert(run_a.to_string(), a.summary);
    runs.insert(run_b.to_string(), b.summary);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "runs": runs,
            "comparison": {
                "same_feature": a.feature == b.feature,
                "same_recipe": a.digest == b.digest,
                "status_pair": [a.status, b.status],
                "first_divergence": first_divergence(&a.trail, &b.trail),
                "resolution_divergence": resolution_divergence(&a.resolution, &b.resolution),
                "cost_delta_usd": b.total_cost - a.total_cost,
                "attempts_delta": b.attempts as i64 - a.attempts as i64,
            }
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests;
