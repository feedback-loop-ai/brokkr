//! `forge compare` — aligned outcome comparison of two runs: the payoff
//! of the recipe library and `forge rerun` ("same feature, different
//! strategy, what diverged?") as a single read-only command. Everything
//! is derived from `fold(events)`, the raw journal, and the stored
//! manifest — no bundle recompilation, no writes, no timestamp-derived
//! semantics (`recorded_at` is evidence only).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use forge_core::fold::fold;
use forge_core::{EventEnvelope, EventType};
use forge_store::Store;
use serde_json::{json, Map, Value};

/// Per-seat attempts/turns/cost from the journal — the aggregation
/// shared by `forge costs` and `forge compare`: attempts from
/// `effect/started` joined through `effect/requested.seat`, turns and
/// cost from `effect/checkpointed` payloads. Returns the per-seat
/// report map and the total cost.
pub fn seat_costs(events: &[EventEnvelope]) -> (Map<String, Value>, f64) {
    let mut effect_seat: BTreeMap<String, String> = BTreeMap::new();
    let mut seats: BTreeMap<String, (u64, u64, f64)> = BTreeMap::new();
    for event in events {
        let payload = &event.payload;
        match event.event_type {
            EventType::EffectRequested => {
                if let (Some(id), Some(seat)) = (
                    payload.get("effect_id").and_then(Value::as_str),
                    payload.get("seat").and_then(Value::as_str),
                ) {
                    effect_seat.insert(id.to_string(), seat.to_string());
                }
            }
            EventType::EffectStarted => {
                if let Some(seat) = payload
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .and_then(|id| effect_seat.get(id))
                {
                    seats.entry(seat.clone()).or_default().0 += 1;
                }
            }
            EventType::EffectCheckpointed => {
                let seat = payload
                    .get("effect_id")
                    .and_then(Value::as_str)
                    .and_then(|id| effect_seat.get(id))
                    .cloned();
                if let Some(seat) = seat {
                    let checkpoint = &payload["checkpoint"];
                    let entry = seats.entry(seat).or_default();
                    entry.1 += checkpoint
                        .get("num_turns")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    entry.2 += checkpoint
                        .get("total_cost_usd")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                }
            }
            _ => {}
        }
    }
    let total: f64 = seats.values().map(|(_, _, c)| c).sum();
    let report: Map<String, Value> = seats
        .into_iter()
        .map(|(seat, (attempts, turns, cost))| {
            (
                seat,
                json!({"attempts": attempts, "turns": turns, "cost_usd": cost}),
            )
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
    /// Participant label → what actually served it (decision 0016).
    /// Computed by CALLING the `forge-view` derivation rather than
    /// re-deriving here, so `compare` cannot describe a fallback
    /// differently from every other readout.
    resolution: BTreeMap<String, Value>,
}

/// What each run's invocation sites resolved to, keyed by participant
/// label so a panel member and a sequence step line up across runs.
fn resolution_of(events: &[EventEnvelope]) -> BTreeMap<String, Value> {
    forge_view::run_view(events, None)
        .participants
        .into_iter()
        .filter_map(|part| {
            part.provenance.map(|provenance| {
                (
                    part.label,
                    json!({
                        "agent": provenance.agent,
                        "model": provenance.model,
                        "provider": provenance.provider,
                        "chain_index": provenance.chain_index,
                        "fallback": provenance.fallback,
                    }),
                )
            })
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
    let digest = forge_core::canonical::sha256_hex(&manifest);

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
