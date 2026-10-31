//! `brokkr backup` and `brokkr restore` through the binary (decision 0100,
//! the journal half of #431): the two verbs are reachable as the operator
//! types them, each prints its document and its one stderr line, and a
//! backup the store refuses fails the command with the refusal's words.

use brokkr_core::envelope::EventType;
use brokkr_store::test_support::append;
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::boundary_readouts::brokkr;

#[test]
fn backup_and_restore_run_as_typed_and_a_refused_backup_fails_in_its_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .create_run("alpha", "feature", "test", &json!({"files": {}}))
        .unwrap();
    append(
        &mut store,
        "alpha",
        EventType::PhaseEntered,
        json!({"phase": "work"}),
    );
    drop(store);
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();

    let (stdout, stderr, ok) = brokkr(
        dir.path(),
        &["backup", "--to", &path("backup"), "--db", &path("forge.db")],
    );
    assert!(ok, "{stderr}");
    let document: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(document["boundary"], json!({"runs": 1, "events": 1}));
    assert!(stderr.contains("(1 runs, 1 events)"), "{stderr}");

    let (stdout, stderr, ok) = brokkr(
        dir.path(),
        &[
            "restore",
            "--from",
            &path("backup"),
            "--to",
            &path("restored.db"),
        ],
    );
    assert!(ok, "{stderr}");
    let document: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        document,
        json!({"restored": path("restored.db"), "runs": 1, "events": 1})
    );
    assert!(stderr.contains("nothing was started"), "{stderr}");

    std::fs::write(dir.path().join("a-file"), "operator bytes").unwrap();
    let (_, stderr, ok) = brokkr(
        dir.path(),
        &["backup", "--to", &path("a-file"), "--db", &path("forge.db")],
    );
    assert!(!ok, "a backup into a file succeeded");
    assert!(
        stderr.contains("exists and is not a directory; nothing was written"),
        "{stderr}"
    );
}
