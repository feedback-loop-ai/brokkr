//! Decision 0065 slice two: a checkpoint's capability-call evidence as
//! the read surfaces carry it. Rows are planted through the store's
//! append fence, so each is a record the seat-record contract of its
//! engine admits, and read back through the binary's `inspect --json`,
//! whose checkpoints are the one derivation in `brokkr-view` (U4g).

use std::path::Path;

use brokkr_core::EventType;
use brokkr_store::Store;
use brokkr_view::capability_calls::UNRECORDED;
use serde_json::{json, Value};

use crate::boundary_readouts::brokkr;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// One `research` seat under `engine` whose checkpoints are `checkpoints`.
fn plant(db: &Path, run_id: &str, engine: &str, checkpoints: &[Value]) {
    let manifest = json!({"engine": engine});
    let mut events = vec![
        (
            EventType::RunStarted,
            json!({"feature": "call it", "manifest": manifest}),
        ),
        (EventType::PhaseEntered, json!({"phase": "research"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "research", "phase": "research"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a1", "driver": "d"}),
        ),
    ];
    events.extend(checkpoints.iter().map(|checkpoint| {
        let payload = json!({"effect_id": "e1", "attempt_id": "a1", "checkpoint": checkpoint});
        (EventType::EffectCheckpointed, payload)
    }));
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, "call it", "test", &manifest)
        .unwrap();
    for (kind, payload) in events {
        // The append fence judges each row by its engine's seat record.
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    }
}

/// `inspect --json`'s checkpoint call evidence for `run_id`, in order.
fn evidence(dir: &Path, db: &Path, run_id: &str) -> Vec<Value> {
    let db = db.to_str().unwrap();
    let (out, err, ok) = brokkr(dir, &["inspect", "--run", run_id, "--db", db, "--json"]);
    assert!(ok, "{err}");
    let view: Value = serde_json::from_str(&out).unwrap();
    let rows = view["participants"][0]["checkpoints"].as_array().unwrap();
    rows.iter()
        .map(|row| row["capability_call"].clone())
        .collect()
}

fn docs(tool: &str, call: &str, state: &str) -> Value {
    json!({"step": "seat-turn", "tool": tool, "capability": "library-docs",
           "dialect": "cap-library-docs", "call_id": call, "call_state": state})
}

/// A 0.12 journal's native observation and four settled broker states
/// pass the v6 fence and read back exactly; the refusal is a refusal.
#[test]
fn inspect_carries_every_recorded_call_state_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let db = root.join("forge.db");
    let mut succeeded = docs("lookup", "a1:b:1", "succeeded");
    succeeded["response_sha256"] = json!(DIGEST);
    plant(
        &db,
        "attributed",
        "0.12.0",
        &[
            json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch",
                   "capability": "web-search", "dialect": "claude-native-search",
                   "call_id": "a1:toolu_1", "call_state": "observed"}),
            succeeded,
            docs("lookup", "a1:b:2", "failed"),
            docs("admin", "a1:b:3", "refused"),
            docs("lookup", "a1:b:4", "interrupted"),
        ],
    );
    let rows = evidence(&root, &db, "attributed");
    let call = |capability: &str, dialect: &str, tool: &str, id: &str, state: &str| {
        json!({"capability": capability, "dialect": dialect, "tool": tool,
               "call_id": id, "state": state, "response_sha256": null})
    };
    let lib = |tool, id, state| call("library-docs", "cap-library-docs", tool, id, state);
    let mut kept = lib("lookup", "a1:b:1", "succeeded");
    kept["response_sha256"] = json!(DIGEST);
    let calls: Vec<_> = rows.iter().map(|row| row["call"].clone()).collect();
    assert_eq!(
        calls,
        [
            call(
                "web-search",
                "claude-native-search",
                "WebSearch",
                "a1:toolu_1",
                "observed"
            ),
            kept,
            lib("lookup", "a1:b:2", "failed"),
            lib("admin", "a1:b:3", "refused"),
            lib("lookup", "a1:b:4", "interrupted"),
        ]
    );
    assert_eq!(
        rows[3]["cell"],
        json!({"text": "library-docs via cap-library-docs · admin refused",
               "absent": false, "note": null})
    );
}

/// A 0.10 journal's `WebSearch` checkpoint, valid under v5, stays
/// unrecorded beside an ordinary tool: absence is never backfilled.
#[test]
fn inspect_leaves_a_historical_native_tool_unrecorded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let db = root.join("forge.db");
    plant(
        &db,
        "historical",
        "0.10.0",
        &[
            json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch"}),
            json!({"step": "seat-turn", "turn": 2, "tool": "Read", "target": "a.rs"}),
        ],
    );
    let unrecorded = json!({"call": null,
        "cell": {"text": brokkr_view::ABSENT, "absent": true, "note": UNRECORDED}});
    assert_eq!(
        evidence(&root, &db, "historical"),
        [unrecorded.clone(), unrecorded]
    );
}
