//! Every exported journal under `fixtures/journals/`, imported into a
//! fresh store, reads as its fold (decision 0089 ruling 4): the import
//! writes each run's summary from the verified run it folds, at its head.
//! A journal the import refuses is planted as a writer from before the
//! summary would leave it, and the next open's catch-up judges it. The
//! fixtures are read, never regenerated. The engine scenario tests are
//! held to the same check by every writer store's drop (`test_support`).

use std::path::{Path, PathBuf};

use brokkr_core::envelope::{ChainError, EventEnvelope};
use serde_json::{json, Value};

use crate::test_support::{lane, plant_envelope, plant_run};
use crate::{Disagreement, Store, StoreError};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/journals")
}

/// `events` planted as a writer from before the summary leaves them: the
/// run and its events, and nothing folded.
fn plant_as_older_writer(store: &Store, run_id: &str, events: &[EventEnvelope]) {
    plant_run(&store.conn, run_id, "{}").unwrap();
    for event in events {
        plant_envelope(&store.conn, event).unwrap();
    }
}

#[test]
fn every_run_reads_as_its_fold() {
    let fixtures = fixtures();
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
            plant_as_older_writer(&store, &run_id, &events);
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
    // behind, for a reader to read whole and refuse live, and the doctor's
    // sweep, which reads every run whole, names it.
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
        ["conclude-broken-chain-hand-built: does not load: chain: event 11: event_hash does not \
          match canonical content"]
    );
}

const BROKEN: &str = "conclude-broken-chain-hand-built";

/// The doctor's sweep names `BROKEN`, and it alone, as a run that does
/// not load at its bad event, its row standing at `place`.
fn assert_only_the_broken_chain_is_named(store: &Store, place: &str) {
    let found = store.summary_disagreements().unwrap();
    assert!(
        matches!(
            found.as_slice(),
            [Disagreement::Unread {
                run_id,
                error: StoreError::Chain(ChainError::BadHash { seq: 11 }),
            }] if run_id == BROKEN
        ),
        "{place}: {found:?}"
    );
}

/// The broken chain, planted as an older writer's events, is named by the
/// doctor's sweep (decision 0089 ruling 7) behind its head, where no
/// writer's catch-up can fold it, and at its head, as a row folded before
/// the chain broke beneath it would stand.
#[test]
fn a_run_that_does_not_load_is_named_behind_its_head_and_at_it() {
    let ndjson = std::fs::read_to_string(fixtures().join(format!("{BROKEN}.ndjson"))).unwrap();
    let events: Vec<EventEnvelope> = ndjson
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    plant_as_older_writer(&Store::open(&path).unwrap(), BROKEN, &events);
    let store = Store::open(&path).unwrap();
    assert_eq!(lane(&store, BROKEN).as_deref(), Some("behind"));
    assert_only_the_broken_chain_is_named(&store, "behind its head");
    store
        .conn
        .execute(
            "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash",
            [],
        )
        .unwrap();
    assert_ne!(lane(&store, BROKEN).as_deref(), Some("behind"));
    assert_only_the_broken_chain_is_named(&store, "at its head");
}
