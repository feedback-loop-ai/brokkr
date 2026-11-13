//! Shape (B) of #440, through the shipped binary: a command that would
//! dispose of itself on a concluded run is refused before it writes, and
//! the run's read surfaces still read it.

use std::path::Path;
use std::process::{Command, Output};

use brokkr_core::envelope::EventType;
use serde_json::json;

fn brokkr(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

/// A run that has opened and done nothing else: `conclude` can close it
/// without a bundle, because a stop concludes wherever the run stands.
fn started(db: &Path) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store.create_run("late", "f", "test", &json!({})).unwrap();
    store
        .append_next(
            "late",
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
}

#[test]
fn an_operator_stop_after_a_conclusion_refuses_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let db = db.to_str().unwrap();
    started(Path::new(db));

    let concluded = brokkr(
        dir.path(),
        &[
            "conclude",
            "--run",
            "late",
            "--cause",
            "withdrawn",
            "--reason",
            "closing",
            "--db",
            db,
        ],
    );
    assert_eq!(
        concluded.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&concluded.stderr)
    );
    let before = brokkr_store::Store::open_read_only(Path::new(db)).unwrap();
    let before_events = before.load("late").unwrap();
    assert_eq!(
        before_events.last().unwrap().event_type,
        EventType::RunStopped
    );
    let before = (before_events.len(), before.head_hash("late").unwrap());

    let late = brokkr(
        dir.path(),
        &[
            "operator",
            "stop",
            "--run",
            "late",
            "--cause",
            "withdrawn",
            "--reason",
            "again",
            "--db",
            db,
        ],
    );
    assert_eq!(late.status.code(), Some(1), "{late:?}");
    assert_eq!(
        String::from_utf8_lossy(&late.stderr),
        "error: run 'late' is already concluded (stopped); stop appends nothing\n"
    );

    let after = brokkr_store::Store::open_read_only(Path::new(db)).unwrap();
    let after_events = after.load("late").unwrap();
    let after = (after_events.len(), after.head_hash("late").unwrap());
    assert_eq!(
        after, before,
        "the refused command appended nothing, not even operator/commanded"
    );

    let runs = brokkr(dir.path(), &["runs", "--json", "--db", db]);
    assert_eq!(runs.status.code(), Some(0), "{runs:?}");
    assert!(String::from_utf8_lossy(&runs.stdout).contains("late"));
    let inspect = brokkr(
        dir.path(),
        &["inspect", "--run", "late", "--json", "--db", db],
    );
    assert_eq!(inspect.status.code(), Some(0), "{inspect:?}");
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("stopped"));
}
