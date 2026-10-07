//! `brokkr compare` — aligned outcome comparison of two runs: the payoff
//! of the recipe library and `brokkr rerun` ("same feature, different
//! strategy, what diverged?") as a single read-only command. Everything
//! is derived from `fold(events)`, the raw journal, and the stored
//! manifest — no bundle recompilation, no writes, no timestamp-derived
//! semantics (`recorded_at` is evidence only). The per-seat accounting and
//! both divergences are `brokkr-view`'s derivations; this module reads
//! the journal and prints them.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use brokkr_core::fold::fold;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::Store;
use brokkr_view::{first_divergence, resolution_divergence, seat_costs};
use serde_json::{json, Map, Value};

/// `brokkr costs`: the run's per-seat report under the id `--run`
/// resolved to — a full id, a unique prefix or `latest` (decision 0015).
/// These bytes are LaneTally's join surface.
pub(crate) fn costs(store: &Store, requested: &str) -> Result<Value> {
    let run = crate::selector::resolve_run(store, requested)?;
    let (report, total) = seat_costs(&store.load(&run)?);
    Ok(json!({
        "run_id": run,
        "seats": report,
        "total_cost_usd": total,
    }))
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
    /// Participant label → the provider-reported model, the boundary
    /// beside it, and the distinct agent-selection provenance.
    /// Computed by CALLING the `brokkr-view` derivation rather than
    /// re-deriving here, so `compare` cannot describe a fallback
    /// differently from every other readout.
    resolution: BTreeMap<String, Value>,
}

/// What each run's invocation sites resolved to, keyed by participant
/// label so a panel member and a sequence step line up across runs.
/// The served pair comes through the pair helper's JSON face (decision
/// 0046 ruling 3; design DD12): `model` and `boundary` as siblings.
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
            let mut entry = crate::render::served_json(&part.served);
            entry["selected"] = selected.unwrap_or(Value::Null);
            (part.label, entry)
        })
        .collect()
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
    let status = state.status.as_str();
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

pub(crate) fn compare(run_a: &str, run_b: &str, db: &Path) -> Result<()> {
    let store = crate::open_journal(db, crate::Access::Read)?;
    let run_a = &crate::selector::resolve_run(&store, run_a)?;
    let run_b = &crate::selector::resolve_run(&store, run_b)?;
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
