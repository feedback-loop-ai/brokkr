//! The suffix read's refusals over rows no writer here would leave (#354).
//!
//! Unit tests rather than a `tests/` target so the planting helpers reach
//! them without the `test-support` feature: a bare `cargo test -p
//! brokkr-store` runs every one.

use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventType};
use serde_json::error::Category;
use serde_json::json;

use crate::test_support;
use crate::{Store, StoreError};

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
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");
    let ndjson = std::fs::read_to_string(&path).unwrap();
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
    // The journal refuses such a row at insert (#354); a journal written
    // before that guard existed may still hold one, and that is the
    // journal these reads must refuse. So the plant drops the guard first.
    let conn = rusqlite::Connection::open(dir.join("forge.db")).unwrap();
    test_support::drop_seq_guard(&conn).unwrap();
    test_support::plant_event(&conn, run_id, &seq, "x", envelope).unwrap();
}

/// The whole read judges every row of the run, whatever seq the table
/// holds it at: a row below seq 1 is refused as it was before the
/// incremental read (#354), never skipped. Export refuses alike, and a
/// suffix read at any head refuses the run by name: a Replay holding
/// seq 2 when the row lands must not keep appending to it.
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
    // Each run's head at seq 2, taken before any row is planted: the head
    // a Replay would hold when the row lands.
    let heads: std::collections::HashMap<&str, String> =
        ["copy-at-zero", "unparseable", "negative"]
            .into_iter()
            .map(|run_id| (run_id, store.load(run_id).unwrap()[1].event_hash.clone()))
            .collect();
    let planted = std::collections::HashMap::from([
        ("copy-at-zero", 0),
        ("unparseable", 0),
        ("negative", -5),
    ]);
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
        // A suffix read never selects a row below seq 1, so at every head,
        // the empty one included, the row is refused by name.
        for (seq, hash) in [(0, ZERO_HASH), (2, heads[run_id].as_str())] {
            match store.load_after(run_id, seq, hash).unwrap_err() {
                StoreError::RowBeforeChain {
                    run_id: found,
                    seq: row,
                } => {
                    assert_eq!(
                        (found.as_str(), row),
                        (run_id, planted[run_id]),
                        "{run_id} at {seq}"
                    );
                }
                other => panic!("{run_id} at {seq}: {other:?}"),
            }
        }
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
