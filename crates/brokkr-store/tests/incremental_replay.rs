//! The incremental read (#354): a reader that keeps what it folded pays
//! only for what landed since, and reaches the state a whole replay
//! reaches, byte for byte.
//!
//! The property is checked over every fixture journal at every split
//! point: `fold(prefix ++ suffix)` is `fold_onto(fold(prefix), suffix)`,
//! and `verify_chain(prefix ++ suffix)` is `verify_chain(prefix)` followed
//! by `verify_chain_after` from the prefix's head, errors included.

use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain, verify_chain_after, ChainError, EventEnvelope};
use brokkr_core::fold::{fold, fold_onto, FoldError, RunState};
use brokkr_store::{Store, StoreError};
use serde_json::json;

/// Every committed fixture journal, by name, with its text. The walk
/// refuses a directory it cannot read and a set without the fixture the
/// other replay tests pin, so an empty walk cannot pass for a property.
fn fixture_journals() -> Vec<(PathBuf, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/journals");
    let mut journals: Vec<(PathBuf, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ndjson")
        })
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect();
    journals.sort();
    let names: Vec<_> = journals
        .iter()
        .map(|(path, _)| path.file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    assert!(
        names.contains(&"tui-graph-the-selection-box-gets-80f98deb".to_string()),
        "{names:?}"
    );
    assert!(names.len() >= 6, "{names:?}");
    journals
}

fn parsed(ndjson: &str) -> Vec<EventEnvelope> {
    ndjson
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// A fold's outcome as comparable text: the whole state, or the refusal.
fn shown(outcome: Result<RunState, FoldError>) -> String {
    format!("{outcome:?}")
}

/// The head the events before `split` leave: the empty head at zero.
fn head_before(events: &[EventEnvelope], split: usize) -> (u64, String) {
    match split {
        0 => (0, ZERO_HASH.to_string()),
        _ => (events[split - 1].seq, events[split - 1].event_hash.clone()),
    }
}

#[test]
fn every_fixture_journal_folds_the_same_whole_or_carried_over_any_split() {
    for (path, ndjson) in fixture_journals() {
        let events = parsed(&ndjson);
        let run_id = events[0].run_id.clone();
        let whole_chain = verify_chain(&events);
        let whole = shown(fold(&events));
        for split in 0..=events.len() {
            let (prefix, suffix) = events.split_at(split);
            let (seq, hash) = head_before(&events, split);
            let carried_chain =
                verify_chain(prefix).and_then(|()| verify_chain_after(&run_id, seq, &hash, suffix));
            assert_eq!(carried_chain, whole_chain, "{} at {split}", path.display());
            if split == 0 {
                continue;
            }
            let carried = match fold(prefix) {
                Ok(state) => shown(fold_onto(state, suffix)),
                // A fold stops at its first refusal, so a refused prefix
                // is the whole journal's refusal.
                refused => shown(refused),
            };
            assert_eq!(carried, whole, "{} at {split}", path.display());
        }
    }
}

/// Replay through the store: every fixture the journal would adopt
/// exports back to its own bytes, and at every split the suffix the
/// store hands back, folded onto the prefix's state, is the whole run.
#[test]
fn every_adoptable_fixture_replays_byte_identically_and_incrementally() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (mut adopted, mut refused) = (Vec::new(), Vec::new());
    for (path, ndjson) in fixture_journals() {
        let run_id = match store.import_run(&ndjson, &json!({}), &path) {
            Ok(adoption) => adoption.run_id,
            Err(error) => {
                let name = path.file_stem().unwrap().to_str().unwrap().to_string();
                refused.push(format!("{name}: {error:?}"));
                continue;
            }
        };
        assert_eq!(store.export_ndjson(&run_id).unwrap(), ndjson);
        let events = store.load(&run_id).unwrap();
        let whole = shown(fold(&events));
        for split in 1..=events.len() {
            let state = fold(&events[..split]).unwrap();
            let suffix = store
                .load_after(&run_id, state.seq, &state.last_hash)
                .unwrap();
            assert_eq!(
                serde_json::to_string(&suffix).unwrap(),
                serde_json::to_string(&events[split..]).unwrap()
            );
            assert_eq!(
                shown(fold_onto(state, &suffix)),
                whole,
                "{run_id} at {split}"
            );
        }
        adopted.push(run_id);
    }
    // Two fixtures no journal would adopt: the one built broken, and the
    // hand-built one whose `run/started` carries no manifest. The fold
    // property above still holds them.
    assert_eq!(
        refused,
        [
            "conclude-broken-chain-hand-built: Verify(Chain(BadHash { seq: 11 }))",
            "reforging-the-road-back-hand-built: Unattested { field: \"manifest\" }",
        ]
    );
    assert_eq!(
        adopted,
        [
            "conclude-already-done-hand-built",
            "conclude-parked-hand-built",
            "conclude-stopped-mid-effect-hand-built",
            "tui-graph-the-selection-box-gets-80f98deb",
        ]
    );
}

/// A head the journal did not grow from is not continued: the suffix is
/// refused as a broken chain at its first event, and the caller never
/// folds another history onto its state.
#[test]
fn a_suffix_is_refused_unless_it_chains_onto_the_callers_head() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (path, ndjson) = fixture_journals()
        .into_iter()
        .find(|(path, _)| path.to_string_lossy().contains("tui-graph"))
        .unwrap();
    let run = store.import_run(&ndjson, &json!({}), &path).unwrap().run_id;
    let events = store.load(&run).unwrap();
    let (first, last) = (&events[0], &events[events.len() - 1]);
    let after_first = store.load_after(&run, 1, &first.event_hash).unwrap();
    assert_eq!(after_first.len(), 104);
    let after_last = store.load_after(&run, last.seq, &last.event_hash);
    assert_eq!(after_last.unwrap().len(), 0);
    let error = store.load_after(&run, 1, ZERO_HASH).unwrap_err();
    let StoreError::Chain(chain) = error else {
        panic!("{error}");
    };
    assert_eq!(
        chain,
        ChainError::BrokenChain {
            seq: 2,
            prev_seq: 1
        }
    );
}
