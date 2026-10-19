//! Every exported journal under `fixtures/journals/`, imported into a
//! fresh store, reads as its fold (decision 0089 ruling 4): the import
//! writes each run's summary from the verified run it folds, at its head.
//! A journal the import refuses is planted as a writer from before the
//! summary would leave it, and the next open's catch-up judges it. The
//! fixtures are read, never regenerated. The engine scenario tests are
//! held to the same check by every writer store's drop (`test_support`).

use std::path::Path;

use brokkr_core::envelope::EventEnvelope;
use serde_json::{json, Value};

use crate::test_support::{lane, plant_envelope, plant_run};
use crate::Store;

#[test]
fn every_run_reads_as_its_fold() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/journals");
    let mut journals: Vec<_> = std::fs::read_dir(&fixtures)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ndjson")
        })
        .collect();
    journals.sort();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let (mut lanes, mut refused) = (Vec::new(), Vec::new());
    for journal in &journals {
        let ndjson = std::fs::read_to_string(journal).unwrap();
        let sidecar = journal.with_extension("manifest.json");
        let manifest: Value = std::fs::read_to_string(&sidecar)
            .map_or_else(|_| json!({}), |text| serde_json::from_str(&text).unwrap());
        let events: Vec<EventEnvelope> = ndjson
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let run_id = events[0].run_id.clone();
        if let Err(error) = store.import_run(&ndjson, &manifest, journal) {
            refused.push(error.to_string());
            plant_run(&store.conn, &run_id, "{}").unwrap();
            for event in &events {
                plant_envelope(&store.conn, event).unwrap();
            }
        }
        lanes.push(run_id);
    }
    assert_eq!(
        refused,
        [
            "chain: event 11: event_hash does not match canonical content",
            "refused: the export's run/started event carries no manifest — an adoption \
             derives the destination's runs row from the verified chain, never from the \
             sidecar manifest that no hash covers, and a readout that shows this run must \
             not show a blank where the chain said nothing",
        ]
    );
    drop(store);
    let store = Store::open(&path).unwrap();
    let lanes: Vec<(&str, String)> = lanes
        .iter()
        .map(|run| (run.as_str(), lane(&store, run).unwrap()))
        .collect();
    // The broken chain does not load, so no writer judges it: it stays
    // behind, for a reader to read whole and refuse live.
    let lane = |run, lane: &str| (run, lane.to_string());
    assert_eq!(
        lanes,
        [
            lane("conclude-already-done-hand-built", "finished"),
            lane("conclude-broken-chain-hand-built", "behind"),
            lane("conclude-parked-hand-built", "parked"),
            lane("conclude-stopped-mid-effect-hand-built", "running"),
            lane("reforging-the-road-back-hand-built", "finished"),
            lane("tui-graph-the-selection-box-gets-80f98deb", "running"),
        ]
    );
    assert_eq!(
        crate::test_support::every_run_reads_as_its_fold(&store),
        ["conclude-broken-chain-hand-built: behind its head"]
    );
}
