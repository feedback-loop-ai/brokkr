//! The exact bytes `import`, `verify-run` and `bridge` print (#346): a
//! value comparison ignores key order, so each document is pinned as the
//! string its verb writes.

use brokkr_core::fold::fold;
use brokkr_core::EventType;
use brokkr_store::Arrival;

use super::*;

#[test]
fn import_prints_the_adoption_in_these_bytes() {
    let adoption = Adoption {
        run_id: "run-1".into(),
        events: 3,
        head_hash: "ab".repeat(32),
        arrival: Arrival {
            imported_at: "2026-10-21T00:00:00Z".into(),
            imported_from: "exported/run-1.ndjson".into(),
        },
    };
    assert_eq!(
        serde_json::to_string_pretty(&adopted(&adoption)).unwrap(),
        r#"{
  "adopted": "byte-identical",
  "chain": "verified",
  "events": 3,
  "imported_at": "2026-10-21T00:00:00Z",
  "imported_from": "exported/run-1.ndjson",
  "journal_head_hash": "abababababababababababababababababababababababababababababababab",
  "run_id": "run-1"
}"#
    );
}

#[test]
fn verify_run_prints_the_chain_and_the_state_in_these_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .create_run(
            "run",
            "feature",
            "bundle",
            &json!({"bundle_name": "bundle"}),
        )
        .unwrap();
    for (event_type, payload) in [
        (EventType::RunStarted, json!({"feature": "feature"})),
        (EventType::PhaseEntered, json!({"phase": "work"})),
    ] {
        store
            .append_next("run", event_type, payload, None, None)
            .unwrap();
    }
    let state = fold(&store.load("run").unwrap()).unwrap();
    assert_eq!(
        serde_json::to_string_pretty(&verified(&state)).unwrap(),
        r#"{
  "chain": "verified",
  "state": {
    "consecutive_failures": {},
    "cursor": "RequestEffect",
    "detail": null,
    "feature": "feature",
    "last_decision": null,
    "park_reason": null,
    "phase": "work",
    "run_id": "run",
    "seq": 2,
    "standing": "running",
    "status": "running",
    "strategy": null
  }
}"#
    );
}

/// The bridge's line stays compact: one round, one line.
#[test]
fn a_bridge_round_prints_one_compact_line_in_these_bytes() {
    let report = SyncReport {
        registered: true,
        submitted: 4,
        replayed: 1,
        commands: 2,
        last_command_cursor: 9,
        last_forge_sequence: 7,
    };
    assert_eq!(
        serde_json::to_string(&round("run-1", &report)).unwrap(),
        r#"{"commands":2,"last_forge_sequence":7,"registered":true,"replayed":1,"run_id":"run-1","submitted":4}"#
    );
}
