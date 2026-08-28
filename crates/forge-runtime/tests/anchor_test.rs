//! Anchoring: agreement verifies, a moved journal is detected, and the
//! ref chain grows without touching the worktree.

use forge_core::envelope::EventType;
use forge_runtime::{anchor, verify_anchor, AnchorError};
use forge_store::Store;
use serde_json::json;
use std::process::Command;

fn git_repo(dir: &std::path::Path) {
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "t@t"],
        vec!["config", "user.name", "t"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        assert!(Command::new("git")
            .args(&args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success());
    }
}

#[test]
fn anchor_verifies_detects_movement_and_chains() {
    let dir = tempfile::tempdir().unwrap();
    git_repo(dir.path());
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();

    let first = anchor(&store, dir.path(), "r1").unwrap();
    let report = verify_anchor(&store, dir.path(), "r1").unwrap();
    assert_eq!(report["verdict"], "anchored");
    assert_eq!(report["chain_length"], 1);

    // The journal moves: the stale anchor is detected, never accepted.
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    match verify_anchor(&store, dir.path(), "r1") {
        Err(AnchorError::Mismatch(detail)) => {
            assert!(
                detail.contains("journal moved") || detail.contains("moved after"),
                "{detail}"
            )
        }
        other => panic!("expected mismatch, got {other:?}"),
    }

    // Re-anchoring chains to the previous anchor.
    let second = anchor(&store, dir.path(), "r1").unwrap();
    assert_ne!(first, second);
    let report = verify_anchor(&store, dir.path(), "r1").unwrap();
    assert_eq!(report["chain_length"], 2);

    // The working tree was never touched.
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let clean = String::from_utf8_lossy(&status.stdout);
    let clean = clean
        .lines()
        .filter(|l| !l.contains("forge.db"))
        .collect::<Vec<_>>();
    assert!(clean.is_empty(), "worktree touched: {clean:?}");

    // Unanchored runs are reported as such.
    assert!(matches!(
        verify_anchor(&store, dir.path(), "nope"),
        Err(AnchorError::Store(_)) | Err(AnchorError::NoAnchor(_))
    ));
}
