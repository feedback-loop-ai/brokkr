//! The pulled reads (slice 4's unit U5b2b1): a labelled read orders a run
//! whose stored realm is NULL where the caller names its realm, a pull
//! yields what the visiting read does, and a run's costs are read by its
//! key inside the window alone.

use super::*;
use crate::aggregate::pull::Bounds;
use crate::metered::SiteOrder;

/// A finished run whose seat, run by `provider`, reported `usd` at each of
/// two checkpoints, a second apart.
fn spent_steps(provider: &str, usd: f64) -> Vec<(EventType, serde_json::Value)> {
    let checkpoint = json!({"effect_id": "fx", "attempt_id": "attempt",
        "checkpoint": {"step": "claude-session-finished", "total_cost_usd": usd}});
    let mut steps = finished_steps("spent");
    steps[3] = (
        EventType::EffectStarted,
        json!({"attempt_id": "attempt", "effect_id": "fx",
               "provenance": [{"agent": "a", "provider": provider, "model": "m"}]}),
    );
    steps.insert(4, (EventType::EffectCheckpointed, checkpoint.clone()));
    steps.insert(5, (EventType::EffectCheckpointed, checkpoint));
    steps
}

/// Three runs of realms `alpha`, NULL and `zeta`, each with a seat.
fn three_realms() -> (tempfile::TempDir, Store) {
    let (dir, store) = store();
    for (run_id, realm, start) in [
        ("a-run", Some("alpha"), 0),
        ("m-run", None, 20),
        ("z-run", Some("zeta"), 40),
    ] {
        let events = chain(run_id, realm, start, &spent_steps("claude", 0.25));
        plant_older_writer(&store, run_id, &events);
    }
    (dir, refolded(store))
}

/// The window the three runs finished in.
fn minute() -> (i64, i64) {
    let from = parse_millis("2026-01-01T00:00:00Z").unwrap();
    (from, from + 60_000)
}

/// The run ids a labelled read yields, by realm then recipe, and by realm
/// then provider for the sites.
fn pulled(store: &Store, label: Option<&str>) -> (Vec<String>, Vec<String>) {
    let (from, to) = minute();
    let bounds = Bounds {
        from,
        to,
        realm: None,
        label,
    };
    let runs = store
        .pull_runs(&bounds, RunOrder::ByRealmThenRecipe, |runs| {
            let mut ids = Vec::new();
            while let Some(run) = runs.next_run().unwrap() {
                ids.push(run.run_id.to_string());
            }
            ids
        })
        .unwrap();
    let sites = store
        .pull_sites(&bounds, SiteOrder::RealmProvider, |sites| {
            let mut ids = Vec::new();
            while let Some(site) = sites.next_site().unwrap() {
                ids.push(site.run_id.to_string());
            }
            ids
        })
        .unwrap();
    (runs, sites)
}

/// A run whose stored realm is NULL stands where the caller's label does,
/// in the runs' and the sites' realm orders alike; without a label it
/// stands first, as the stored order puts a NULL.
#[test]
fn a_labelled_read_orders_a_null_realm_by_its_label() {
    let (_dir, store) = three_realms();
    let ids = |names: [&str; 3]| names.map(str::to_string).to_vec();
    let labelled = ids(["a-run", "m-run", "z-run"]);
    assert_eq!(pulled(&store, Some("mid")), (labelled.clone(), labelled));
    let stored = ids(["m-run", "a-run", "z-run"]);
    assert_eq!(pulled(&store, None), (stored.clone(), stored));
}

/// A run's costs are its checkpoints' inside the window, in its order,
/// and none of another run's.
#[test]
fn a_runs_costs_are_its_own_inside_the_window() {
    let (_dir, store) = three_realms();
    let costs = |run_id: &str, from: i64, to: i64| {
        let mut costs = Vec::new();
        store
            .run_spends(run_id, from, to, |at, usd| costs.push((at, usd)))
            .unwrap();
        costs
    };
    let (from, to) = minute();
    let start = from + 20_000;
    assert_eq!(
        costs("m-run", from, to),
        [(start + 4_000, 0.25), (start + 5_000, 0.25)]
    );
    assert_eq!(
        costs("m-run", start + 5_000, to),
        [(start + 5_000, 0.25)],
        "a checkpoint before the window is not read"
    );
    assert_eq!(costs("no-such-run", from, to), []);
}

/// A pull of the parks yields the parks the visiting read does.
#[test]
fn a_pull_of_the_parks_is_the_visiting_read() {
    let (_dir, store) = store();
    let parked = chain("parked", Some("realm"), 0, &parked_steps("review"));
    plant_older_writer(&store, "parked", &parked);
    let store = refolded(store);
    let (from, to) = minute();
    let pulled = store
        .pull_parks(from, to, to, None, |parks| {
            let mut pulled = Vec::new();
            while let Some(park) = parks.next_park().unwrap() {
                pulled.push(OwnedPark::of(&park));
            }
            pulled
        })
        .unwrap();
    assert_eq!(pulled.len(), 1, "{pulled:?}");
    assert_eq!(pulled, window_parks(&store, from, to, to, None));
}
