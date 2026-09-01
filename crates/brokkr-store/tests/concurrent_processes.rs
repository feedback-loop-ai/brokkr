//! Many fires, one journal — at the process level.
//!
//! The in-crate test proves concurrent writers with threads, and for
//! most of SQLite's locking that is the same thing. It is not the same
//! thing everywhere: POSIX advisory locks are held per *process*, so
//! SQLite's unix layer mediates same-process contention with its own
//! in-memory bookkeeping and only reaches the real file locks between
//! processes. Same-realm parallel burns are separate `brokkr` processes,
//! so the claim has to be proved where it will actually be spent.
//!
//! The writers here are re-executions of this very test binary, which
//! keeps the store code under test exactly as it ships and needs no
//! helper crate: the parent spawns N children at one journal, each child
//! drives its own run, and the parent then reads every chain back.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::json;

const WRITERS: usize = 4;
const TURNS: usize = 12;
const EVENTS_PER_RUN: usize = 1 + TURNS * 5;

const DB_VAR: &str = "BROKKR_STORE_CONCURRENT_DB";
const RUN_VAR: &str = "BROKKR_STORE_CONCURRENT_RUN";

/// The child writer. Inert in an ordinary test run — it only does
/// anything when the parent below re-executes this binary with the two
/// environment variables set.
#[test]
fn concurrent_writer_child() {
    let (Ok(db), Ok(run_id)) = (std::env::var(DB_VAR), std::env::var(RUN_VAR)) else {
        return;
    };
    write_one_run(Path::new(&db), &run_id);
}

fn write_one_run(db: &Path, run_id: &str) {
    let mut store = Store::open(db).expect("a racing open still opens");
    store
        .create_run(run_id, "feat", "self", &json!({"files": {}}))
        .expect("a racing create_run still creates");
    store
        .append_next(
            run_id,
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .expect("a racing first append still appends");
    for turn in 0..TURNS {
        let effect_id = format!("effect-{turn}");
        for (event_type, payload) in [
            (EventType::PhaseEntered, json!({"phase": "implement"})),
            (
                EventType::EffectRequested,
                json!({"effect_id": effect_id, "seat": "implementer"}),
            ),
            (
                EventType::EffectStarted,
                json!({"effect_id": effect_id, "attempt_id": format!("attempt-{turn}")}),
            ),
            (
                EventType::EffectSucceeded,
                json!({"effect_id": effect_id, "result": "complete"}),
            ),
            (
                EventType::TransitionDecided,
                json!({
                    "from": "implement",
                    "result": "complete",
                    "rule_id": "WORK",
                    "next": "implement",
                    "severity": null,
                    "inputs": {},
                    "problem": null,
                }),
            ),
        ] {
            store
                .append_next(run_id, event_type, payload, None, None)
                .expect("an append to this process's own run never conflicts");
        }
    }
}

#[test]
fn parallel_processes_writing_different_runs_share_one_journal() {
    // Guard against the child re-entering the parent test.
    if std::env::var(DB_VAR).is_ok() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let db: PathBuf = dir.path().join("realm.db");
    // Nothing pre-creates the journal: the processes race to bring it
    // into existence, WAL conversion and migration included.
    let children: Vec<_> = (0..WRITERS)
        .map(|writer| {
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "concurrent_writer_child", "--nocapture"])
                .env(DB_VAR, &db)
                .env(RUN_VAR, format!("run-{writer}"))
                .spawn()
                .expect("the test binary re-executes")
        })
        .collect();
    for (writer, mut child) in children.into_iter().enumerate() {
        let status = child.wait().expect("child writer is waitable");
        assert!(status.success(), "writer {writer} failed: {status}");
    }

    let reader = Store::open(&db).unwrap();
    assert_eq!(reader.list_runs().unwrap().len(), WRITERS);
    let mut every_event_id = std::collections::HashSet::new();
    for writer in 0..WRITERS {
        let run_id = format!("run-{writer}");
        // `load` verifies the chain and refuses to return a partial one.
        let events = reader.load(&run_id).unwrap();
        assert_eq!(
            events.len(),
            EVENTS_PER_RUN,
            "{run_id} lost or gained an event"
        );
        let mut previous = ZERO_HASH.to_string();
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event.seq, index as u64 + 1, "{run_id} seq {index}");
            assert_eq!(event.run_id, run_id, "{run_id} carries a foreign run_id");
            assert_eq!(event.correlation_id, run_id);
            assert_eq!(event.previous_hash, previous, "{run_id} chain broke");
            previous = event.event_hash.clone();
            assert!(
                every_event_id.insert(event.event_id.clone()),
                "an event id was reused across processes"
            );
        }
        let state = brokkr_core::fold(&events).unwrap();
        assert_eq!(state.phase.as_deref(), Some("implement"));
        assert_eq!(state.seq, events.len() as u64);
    }
}
