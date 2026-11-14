//! Shape (B) of #440, through the shipped binary: a command that would
//! dispose of itself on a concluded run is refused before it writes, and
//! the run's read surfaces still read it.

use std::path::Path;
use std::process::{Command, Output};

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
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

/// A run with an attempt in flight: the journal a live driver is working
/// under when `conclude` closes the run beneath it.
fn in_flight(db: &Path) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .create_run("beneath", "f", "test", &json!({}))
        .unwrap();
    for (event_type, payload, attempt) in [
        (
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {}}),
            None,
        ),
        (EventType::PhaseEntered, json!({"phase": "work"}), None),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "work"}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a"}),
            Some("a".to_string()),
        ),
    ] {
        store
            .append_next("beneath", event_type, payload, None, attempt)
            .unwrap();
    }
}

/// Shape (A) through the shipped binary: a run concluded while its
/// attempt was open. The driver's next checkpoint meets the store's
/// terminal gate as a typed refusal and writes nothing, and the read
/// surfaces still read the run as stopped.
#[test]
fn a_run_concluded_beneath_a_live_driver_reads_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let db = db.to_str().unwrap();
    in_flight(Path::new(db));
    let concluded = brokkr(
        dir.path(),
        &[
            "conclude",
            "--run",
            "beneath",
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

    let mut store = brokkr_store::Store::open(Path::new(db)).unwrap();
    let before = store.head_hash("beneath").unwrap();
    let error = store
        .append_next(
            "beneath",
            EventType::EffectCheckpointed,
            json!({"effect_id": "e", "attempt_id": "a",
                   "checkpoint": {"step": "seat-turn", "turn": 1}}),
            None,
            Some("a".into()),
        )
        .unwrap_err();
    assert!(
        matches!(
            error,
            brokkr_store::StoreError::AfterTerminal {
                status: Status::Stopped,
                event_type: EventType::EffectCheckpointed,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(store.head_hash("beneath").unwrap(), before);
    drop(store);

    let runs = brokkr(dir.path(), &["runs", "--json", "--db", db]);
    assert_eq!(runs.status.code(), Some(0), "{runs:?}");
    assert!(String::from_utf8_lossy(&runs.stdout).contains("beneath"));
    let inspect = brokkr(
        dir.path(),
        &["inspect", "--run", "beneath", "--json", "--db", db],
    );
    assert_eq!(inspect.status.code(), Some(0), "{inspect:?}");
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("stopped"));
}
