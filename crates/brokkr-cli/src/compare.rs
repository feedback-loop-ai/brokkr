//! `brokkr compare` — aligned outcome comparison of two runs: the payoff
//! of the recipe library and `brokkr rerun` ("same feature, different
//! strategy, what diverged?") as a single read-only command. Everything
//! is derived from `fold(events)`, the raw journal, and the stored
//! manifest — no bundle recompilation, no writes, no timestamp-derived
//! semantics (`recorded_at` is evidence only). The per-seat accounting and
//! both divergences are `brokkr-view`'s derivations; this module reads
//! the journal and prints them.
//!
//! `brokkr costs` opens the run first, its chain verified, then reads its
//! seat rows at the head the opening verified (decision 0089 rulings 5
//! and 7). A run the store says is kept is read whole, as before (ruling
//! 8): either way it prints the same bytes.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use brokkr_core::fold::{fold, ParkReason};
use brokkr_core::seat::row;
use brokkr_core::EventType;
use brokkr_store::{Opened, SeatRows, Store};
use brokkr_view::{first_divergence, resolution_divergence, resolutions, seat_costs, Resolution};
use serde::Serialize;
use serde_json::{json, Value};

/// `brokkr costs`: the run's per-seat report under the id `--run`
/// resolved to — a full id, a unique prefix or `latest` (decision 0015).
/// These bytes are LaneTally's join surface. The run is opened, then its
/// seat rows read at the head the opening verified, or the run read whole
/// where they do not stand there.
pub(crate) fn costs(store: &Store, requested: &str) -> Result<Costs> {
    let run = crate::selector::resolve_run(store, requested)?;
    let opened = store.open_run(&run)?;
    costs_at(store, run, &opened)
}

/// `brokkr costs`' document of `run`, which its opening verified to
/// `opened` (what ruling 9 bounds after the opening).
pub(crate) fn costs_at(store: &Store, run: String, opened: &Opened) -> Result<Costs> {
    let (report, total) = match store.seat_rows_at(&run, opened)? {
        SeatRows::Current(rows) => row::costs(&rows),
        SeatRows::Stale => seat_costs(&store.load(&run)?),
    };
    Ok(Costs {
        run_id: run,
        seats: json!(report),
        total_cost_usd: total,
    })
}

// Every document below declares its fields in byte order, the order its
// keys were printed in when it was a `json!` map (#346). A field holding
// a `brokkr-view` record stays a `Value` made by `json!`: `SeatCost`,
// `Resolution` and `FirstDivergence` declare their fields in another
// order, and the map is what keeps their keys sorted.

/// `brokkr costs`'s document.
#[derive(Serialize)]
pub(crate) struct Costs {
    run_id: String,
    seats: Value,
    total_cost_usd: f64,
}

/// One run's section of `brokkr compare`'s document.
#[derive(Serialize)]
struct Summary {
    bundle_name: Value,
    decision_trail: Vec<String>,
    events: usize,
    feature: Option<String>,
    first_recorded_at: Option<String>,
    last_recorded_at: Option<String>,
    manifest: Pinned,
    park_reason: Option<String>,
    phase: Option<String>,
    phases_visited: BTreeMap<String, u64>,
    resolution: Value,
    seats: Value,
    status: &'static str,
    total_cost_usd: f64,
}

/// The pinned bundle identity a [`Summary`] names.
#[derive(Serialize)]
struct Pinned {
    sha256: String,
}

/// `brokkr compare`'s document: both runs' summaries keyed by run id.
#[derive(Serialize)]
struct Compared {
    comparison: Comparison,
    runs: BTreeMap<String, Summary>,
}

/// What the comparison found between the two runs.
#[derive(Serialize)]
struct Comparison {
    attempts_delta: i64,
    cost_delta_usd: f64,
    first_divergence: Value,
    resolution_divergence: Value,
    same_feature: bool,
    same_recipe: bool,
    status_pair: [&'static str; 2],
}

/// One run's section of the report plus the facts the comparison needs
/// beyond it.
struct RunFacts {
    summary: Summary,
    attempts: u64,
    /// Participant label → the provider-reported model, the boundary
    /// beside it, and the distinct agent-selection provenance.
    /// Computed by CALLING the `brokkr-view` derivation rather than
    /// re-deriving here, so `compare` cannot describe a fallback
    /// differently from every other readout.
    resolution: BTreeMap<String, Resolution>,
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
    let resolution = resolutions(&events);
    let summary = Summary {
        bundle_name: manifest.get("bundle_name").cloned().unwrap_or(Value::Null),
        decision_trail: trail,
        events: events.len(),
        feature,
        first_recorded_at: events.first().map(|e| e.recorded_at.clone()),
        last_recorded_at: events.last().map(|e| e.recorded_at.clone()),
        manifest: Pinned { sha256: digest },
        park_reason: state
            .run
            .park_reason()
            .map(ParkReason::as_str)
            .map(str::to_string),
        phase: state.phase,
        phases_visited: phases,
        resolution: json!(resolution),
        seats: json!(seats),
        status: state.run.status().as_str(),
        total_cost_usd: total_cost,
    };
    Ok(RunFacts {
        summary,
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
    println!(
        "{}",
        serde_json::to_string_pretty(&document(run_a, a, run_b, b))?
    );
    Ok(())
}

/// `brokkr compare`'s document: both runs' summaries keyed by run id, and
/// what the comparison found between them.
fn document(run_a: &str, a: RunFacts, run_b: &str, b: RunFacts) -> Compared {
    let (one, two) = (&a.summary, &b.summary);
    let comparison = Comparison {
        attempts_delta: b.attempts as i64 - a.attempts as i64,
        cost_delta_usd: two.total_cost_usd - one.total_cost_usd,
        first_divergence: json!(first_divergence(&one.decision_trail, &two.decision_trail)),
        resolution_divergence: json!(resolution_divergence(&a.resolution, &b.resolution)),
        same_feature: one.feature == two.feature,
        same_recipe: one.manifest.sha256 == two.manifest.sha256,
        status_pair: [one.status, two.status],
    };
    let mut runs = BTreeMap::new();
    runs.insert(run_a.to_string(), a.summary);
    runs.insert(run_b.to_string(), b.summary);
    Compared { comparison, runs }
}

#[cfg(test)]
mod tests;
