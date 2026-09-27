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
use brokkr_core::envelope::{
    verify_chain, verify_chain_after, ChainError, EventEnvelope, EventType,
};
use brokkr_core::fold::{fold, fold_onto, FoldError, RunState};
use brokkr_store::{Store, StoreError};
use serde_json::error::Category;
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

/// A head the journal did not grow from is not continued, even when
/// nothing follows it: a run the journal lacks is not found, a head that
/// is not the run's row at its seq with its hash is unknown (a seq no row
/// could hold before the journal is asked), and a stored suffix that does
/// not chain onto the head refuses as a broken chain. The caller never
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
    assert_eq!(store.load_after(&run, 0, ZERO_HASH).unwrap().len(), 105);
    let missing = store.load_after("no-such-run", 0, ZERO_HASH);
    assert!(
        matches!(&missing, Err(StoreError::RunNotFound(id)) if id == "no-such-run"),
        "{missing:?}"
    );
    let unknown = [
        (run.as_str(), 1, ZERO_HASH),
        (&run, 0, &first.event_hash),
        (&run, last.seq, "garbage"),
        (&run, last.seq + 1, &last.event_hash),
        ("no-such-run", 1 << 63, ZERO_HASH),
        (&run, u64::MAX, &last.event_hash),
    ]
    .map(
        |(run_id, seq, hash)| match store.load_after(run_id, seq, hash) {
            Err(StoreError::UnknownHead { seq }) => seq,
            other => panic!("{other:?}"),
        },
    );
    assert_eq!(unknown, [1, 0, 105, 106, 1 << 63, u64::MAX]);
    let orphan = serde_json::to_string(&events[0]).unwrap();
    plant(dir.path(), &run, 106, &orphan);
    let error = store.load_after(&run, last.seq, &last.event_hash);
    let Err(StoreError::Chain(chain)) = error else {
        panic!("{error:?}");
    };
    assert_eq!(
        chain,
        ChainError::SeqGap {
            seq: 1,
            expected: 106
        }
    );
}

/// Seal a row straight into the table, past every fence the store keeps:
/// the schema admits any integer seq, and the append-only triggers guard
/// UPDATE and DELETE, not INSERT.
fn plant(dir: &Path, run_id: &str, seq: i64, envelope: &str) {
    rusqlite::Connection::open(dir.join("forge.db"))
        .unwrap()
        .execute(
            "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, ?2, 'x', ?3)",
            rusqlite::params![run_id, seq, envelope],
        )
        .unwrap();
}

/// The whole read judges every row of the run, whatever seq the table
/// holds it at: a row below seq 1 is refused as it was before the
/// incremental read (#354), never skipped, and export refuses alike.
#[test]
fn the_whole_read_refuses_a_row_the_chain_does_not_cover() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    for run_id in ["copy-at-zero", "unparseable", "negative"] {
        store
            .create_run(run_id, "feat", "self", &json!({}))
            .unwrap();
        let started = json!({"feature": "feat", "manifest": {}});
        store
            .append_next(run_id, EventType::RunStarted, started, None, None)
            .unwrap();
        let entered = json!({"phase": "intake"});
        store
            .append_next(run_id, EventType::PhaseEntered, entered, None, None)
            .unwrap();
        assert_eq!(store.load(run_id).unwrap().len(), 2);
    }
    let text = |run_id: &str, seq: usize| {
        serde_json::to_string(&store.load(run_id).unwrap()[seq - 1]).unwrap()
    };
    let (first, second) = (text("copy-at-zero", 1), text("negative", 2));
    plant(dir.path(), "copy-at-zero", 0, &first);
    plant(dir.path(), "unparseable", 0, "not an envelope");
    plant(dir.path(), "negative", -5, &second);
    for (run_id, refusal) in [
        (
            "copy-at-zero",
            Ok(ChainError::SeqGap {
                seq: 1,
                expected: 2,
            }),
        ),
        ("unparseable", Err((Category::Syntax, 1, 2))),
        (
            "negative",
            Ok(ChainError::SeqGap {
                seq: 2,
                expected: 1,
            }),
        ),
    ] {
        let load = judged(store.load(run_id).unwrap_err());
        assert_eq!(load, refusal, "{run_id}");
        let export = judged(store.export_ndjson(run_id).unwrap_err());
        assert_eq!(export, refusal, "{run_id}");
    }
}

/// A whole read's refusal, comparable: the chain defect, or where and
/// how the JSON of a row broke.
fn judged(error: StoreError) -> Result<ChainError, (Category, usize, usize)> {
    match error {
        StoreError::Chain(chain) => Ok(chain),
        StoreError::Json(json) => Err((json.classify(), json.line(), json.column())),
        other => panic!("{other:?}"),
    }
}
