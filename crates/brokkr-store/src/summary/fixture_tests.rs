//! Every exported journal under `fixtures/journals/`, imported into a
//! fresh store, reads as its fold (decision 0089 ruling 4): the import
//! writes each run's summary from the verified run it folds, at its head.
//! A journal the import refuses is planted as a writer from before the
//! summary would leave it, and the next open's catch-up judges it. The
//! fixtures are read, never regenerated. The engine scenario tests are
//! held to the same check by every writer store's drop (`test_support`).

use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};

use crate::test_support::{
    fixture_journals as fixtures, import_fixtures, lane,
    plant_older_writer as plant_as_older_writer,
};
use crate::{Disagreement, Store, StoreError};

#[test]
fn every_run_reads_as_its_fold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let (lanes, refused) = import_fixtures(&mut store);
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

/// An import's inserts are indexed by the trigger as an append's are: a
/// fixture's trail is its every event but its checkpoints, read from the
/// journal's own lines.
#[test]
fn an_import_is_indexed_as_its_inserts_land() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    import_fixtures(&mut store);
    let run = "reforging-the-road-back-hand-built";
    let ndjson = std::fs::read_to_string(fixtures().join(format!("{run}.ndjson"))).unwrap();
    let trail: Vec<u64> = ndjson
        .lines()
        .map(|line| serde_json::from_str::<EventEnvelope>(line).unwrap())
        .filter(|event| event.event_type != EventType::EffectCheckpointed)
        .map(|event| event.seq)
        .collect();
    assert_eq!(trail.len(), 58);
    assert_eq!(store.index_trail(run).unwrap(), trail);
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
