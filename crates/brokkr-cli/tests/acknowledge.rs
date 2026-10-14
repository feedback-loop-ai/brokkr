//! `brokkr operator acknowledge` from the command line (#511): what the
//! operator reads once the acknowledgement is recorded.

use std::path::Path;
use std::process::Command;

use brokkr_core::envelope::EventType;
use serde_json::json;

/// A run whose journal reads to its end but will not fold: an acceptance
/// that answers no command.
fn unanswered(db: &Path) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .create_run("unanswered", "f", "test", &json!({}))
        .unwrap();
    let started = json!({"feature": "f", "manifest": {}});
    let accepted = json!({"command_id": "none", "operator": "op", "reason": "r"});
    for (kind, payload) in [
        (EventType::RunStarted, started),
        (EventType::OperatorAccepted, accepted),
    ] {
        store
            .append_next("unanswered", kind, payload, None, None)
            .unwrap();
    }
}

#[test]
fn an_acknowledgement_says_where_the_run_lists_and_that_its_refusal_stands() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    unanswered(&db);
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .current_dir(dir.path())
        .args(["operator", "--run", "unanswered", "--reason", "seen"])
        .args(["--db", db.to_str().unwrap(), "acknowledge"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert_eq!(
        stderr,
        "acknowledged quarantined run unanswered at event 2; it lists under older until its \
         journal moves\nits refusal stands: event 2: operator/accepted without a matching \
         command\n"
    );
}
