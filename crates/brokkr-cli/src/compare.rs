//! `brokkr compare` — aligned outcome comparison of two runs: the payoff
//! of the recipe library and `brokkr rerun` ("same feature, different
//! strategy, what diverged?") as a single read-only command. Everything
//! is derived from the run's fold, its trail, its seat rows and the
//! stored manifest — no bundle recompilation, no writes, no
//! timestamp-derived semantics (`recorded_at` is evidence only). The
//! per-seat accounting and both divergences are `brokkr-core`'s and
//! `brokkr-view`'s derivations; this module reads the journal and prints
//! them.
//!
//! `brokkr costs` and `brokkr compare` open each run first, its chain
//! verified, then read its seat rows, and `compare` its trail, at the
//! head the opening verified (decision 0089 rulings 5 and 7). A run the
//! store says is kept is read whole, as before (ruling 8): either way
//! they print the same bytes.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, ParkReason, RunState};
use brokkr_core::seat::{row, SeatCost};
use brokkr_core::EventType;
use brokkr_store::{Opened, RunRead, SeatRows, Store};
use brokkr_view::{
    composed_view, first_divergence, resolution_divergence, resolutions, resolved, seat_costs,
    Parts, Resolution,
};
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

/// What `compare` reads of one run once it is opened: its trail (every
/// event but its checkpoints, or every event where it is read whole,
/// which the trail's figures read alike), its event count and last stamp,
/// its fold, its seats' accounting and what its sites resolved to.
struct Read {
    trail: Vec<EventEnvelope>,
    events: usize,
    last_recorded_at: Option<String>,
    state: RunState,
    seats: (BTreeMap<String, SeatCost>, f64),
    resolution: BTreeMap<String, Resolution>,
}

/// `run_id` read whole, as `compare` read every run before its rows
/// (ruling 8's kept read).
fn kept(store: &Store, run_id: &str) -> Result<Read> {
    let events = store
        .load(run_id)
        .context(format!("loading run '{run_id}'"))?;
    let state = fold(&events).context(format!("folding run '{run_id}'"))?;
    Ok(Read {
        events: events.len(),
        last_recorded_at: events.last().map(|e| e.recorded_at.clone()),
        state,
        seats: seat_costs(&events),
        resolution: resolutions(&events),
        trail: events,
    })
}

/// `run_id` read at `opened`, the head its opening verified: its trail,
/// its seat rows, its fold and its head's stamp, and the journal page
/// past the head, so none; `None` where the store says it is kept, its
/// fold refuses at the head, or its rows compose no view, for the run to
/// be read whole.
fn current(store: &Store, run_id: &str, opened: &Opened) -> Result<Option<Read>> {
    let read = store
        .run_read(run_id, opened, opened.seq)
        .context(format!("loading run '{run_id}'"))?;
    let RunRead::Current {
        trail,
        seats,
        page,
        causes,
        state: Some(state),
        last_at,
    } = read
    else {
        return Ok(None);
    };
    let parts = Parts {
        trail: &trail,
        seats: &seats,
        page: &page,
        causes: &causes,
        head_seq: opened.seq,
        last_at: &last_at,
    };
    let Some(view) = composed_view(&parts, None) else {
        return Ok(None);
    };
    Ok(Some(Read {
        events: view.event_count,
        last_recorded_at: Some(last_at),
        state: *state,
        seats: row::costs(&seats),
        resolution: resolved(view.participants),
        trail,
    }))
}

fn run_facts(store: &Store, run_id: &str) -> Result<RunFacts> {
    let opened = store
        .open_run(run_id)
        .context(format!("loading run '{run_id}'"))?;
    run_facts_at(store, run_id, &opened)
}

/// [`run_facts`] of `run_id`, which its opening verified to `opened`.
fn run_facts_at(store: &Store, run_id: &str, opened: &Opened) -> Result<RunFacts> {
    let manifest = store
        .manifest(run_id)
        .context(format!("loading manifest for run '{run_id}'"))?;
    let read = match current(store, run_id, opened)? {
        Some(read) => read,
        None => kept(store, run_id)?,
    };

    let feature = read
        .trail
        .first()
        .filter(|e| e.event_type == EventType::RunStarted)
        .and_then(|e| e.payload.get("feature"))
        .and_then(Value::as_str)
        .map(str::to_string);

    // The stored manifest is the pinned bundle identity; hashing it is
    // byte-identical to Bundle::manifest_digest at pin time.
    let digest = brokkr_core::canonical::sha256_hex(&manifest);

    let (trail, phases, attempts) = walked(&read.trail);
    let (seats, total_cost) = read.seats;
    let resolution = read.resolution;
    let state = read.state;
    let summary = Summary {
        bundle_name: manifest.get("bundle_name").cloned().unwrap_or(Value::Null),
        decision_trail: trail,
        events: read.events,
        feature,
        first_recorded_at: read.trail.first().map(|e| e.recorded_at.clone()),
        last_recorded_at: read.last_recorded_at,
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

/// What `trail` holds of a run's decisions and attempts: its decision
/// trail, the starts by the phase their request names, and the starts. A
/// checkpoint is none of them, so the trail and the run read whole give
/// the same. A null rule_id is a park fact (decision 0001); "park" is a
/// display convention only.
fn walked(events: &[EventEnvelope]) -> (Vec<String>, BTreeMap<String, u64>, u64) {
    let mut trail = Vec::new();
    let mut effect_phase: BTreeMap<String, String> = BTreeMap::new();
    let mut phases: BTreeMap<String, u64> = BTreeMap::new();
    let mut attempts: u64 = 0;
    for event in events {
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
    (trail, phases, attempts)
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

/// The bytes `brokkr compare` prints of the two runs `runs` names, which
/// their openings verified each to the head beside it (what ruling 9
/// bounds after the openings).
pub(crate) fn compared_at(store: &Store, runs: [(&str, &Opened); 2]) -> Result<String> {
    let [(run_a, at_a), (run_b, at_b)] = runs;
    let a = run_facts_at(store, run_a, at_a)?;
    let b = run_facts_at(store, run_b, at_b)?;
    Ok(serde_json::to_string_pretty(&document(run_a, a, run_b, b))?)
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
