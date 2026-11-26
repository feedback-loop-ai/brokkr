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
use std::process::{Command, Stdio};

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
                json!({"effect_id": effect_id, "result": {"result": "complete"}}),
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
                .args([
                    "--exact",
                    "concurrent_processes::concurrent_writer_child",
                    "--nocapture",
                ])
                .env(DB_VAR, &db)
                .env(RUN_VAR, format!("run-{writer}"))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the test binary re-executes")
        })
        .collect();
    for (writer, child) in children.into_iter().enumerate() {
        let ran = child.wait_with_output().expect("child writer is waitable");
        let said = String::from_utf8_lossy(&ran.stdout);
        let told = String::from_utf8_lossy(&ran.stderr);
        assert!(ran.status.success(), "writer {writer} failed: {said}{told}");
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

// ---------------------------------------------------------------------------
// Slice 2 (decision 0126 R4): Multi-process FsEventLog stress tests
// ---------------------------------------------------------------------------

const FS_ROOT_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_ROOT";
const FS_MODE_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_MODE";
const FS_RUN_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_RUN";
const FS_WORKER_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_WORKER";

const FS_INDEPENDENT_EVENTS: usize = 15;
const FS_CONTENDED_COMMITS: usize = 6;

fn sealed_proc_event(
    run_id: &str,
    seq: u64,
    event_type: EventType,
    payload: serde_json::Value,
    prev: &str,
) -> brokkr_core::envelope::EventEnvelope {
    brokkr_core::envelope::EventEnvelope {
        run_id: run_id.to_string(),
        seq,
        event_id: uuid::Uuid::new_v4().to_string(),
        event_schema_version: brokkr_core::envelope::EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id: None,
        correlation_id: run_id.to_string(),
        attempt_id: None,
        recorded_at: "2026-11-26T16:00:00Z".to_string(),
        previous_hash: prev.to_string(),
        event_hash: String::new(),
    }
    .sealed()
}

#[test]
fn fs_event_log_concurrent_child() {
    let (Ok(root), Ok(mode), Ok(run_id)) = (
        std::env::var(FS_ROOT_VAR),
        std::env::var(FS_MODE_VAR),
        std::env::var(FS_RUN_VAR),
    ) else {
        return;
    };
    use brokkr_store::ports::{EventLog, FsEventLog};
    let log = FsEventLog::open(Path::new(&root)).expect("fs log opens");
    if mode == "independent" {
        let manifest = json!({"engine": "0.15.0", "feature": run_id});
        log.create_run(&run_id, &manifest).expect("create_run");
        let mut head = (0u64, ZERO_HASH.to_string());
        for seq in 1..=FS_INDEPENDENT_EVENTS {
            let env = if seq == 1 {
                sealed_proc_event(
                    &run_id,
                    1,
                    EventType::RunStarted,
                    json!({"feature": run_id, "manifest": manifest}),
                    ZERO_HASH,
                )
            } else {
                sealed_proc_event(
                    &run_id,
                    seq as u64,
                    EventType::PhaseEntered,
                    json!({"phase": format!("step-{seq}")}),
                    &head.1,
                )
            };
            log.append_envelopes(&run_id, (head.0, &head.1), std::slice::from_ref(&env))
                .expect("independent fs append never conflicts");
            head = (env.seq, env.event_hash);
        }
    } else if mode == "contended" {
        let worker = std::env::var(FS_WORKER_VAR).unwrap_or_default();
        let mut committed = 0usize;
        while committed < FS_CONTENDED_COMMITS {
            let (head_seq, head_hash) = log.head(&run_id).expect("read head");
            let env = sealed_proc_event(
                &run_id,
                head_seq + 1,
                EventType::PhaseEntered,
                json!({"worker": worker, "n": committed}),
                &head_hash,
            );
            match log.append_envelopes(&run_id, (head_seq, &head_hash), std::slice::from_ref(&env))
            {
                Ok(()) => committed += 1,
                Err(brokkr_store::StoreError::HeadMoved { .. }) => {}
                Err(err) => panic!("unexpected contended error: {err}"),
            }
        }
    }
}

#[test]
fn parallel_processes_fs_event_log_independent_and_contended() {
    if std::env::var(FS_ROOT_VAR).is_ok() {
        return;
    }
    use brokkr_store::ports::{EventLog, FsEventLog, ProjectionStore, SqliteProjectionStore};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("runs");

    // 1. N independent processes writing independent runs.
    let children: Vec<_> = (0..WRITERS)
        .map(|writer| {
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "concurrent_processes::fs_event_log_concurrent_child",
                    "--nocapture",
                ])
                .env(FS_ROOT_VAR, &root)
                .env(FS_MODE_VAR, "independent")
                .env(FS_RUN_VAR, format!("fs-run-{writer}"))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn child")
        })
        .collect();
    for (writer, child) in children.into_iter().enumerate() {
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "independent child {writer} failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // 2. N competing processes racing on one shared run via File::lock().
    let log = FsEventLog::open(&root).unwrap();
    let shared = "fs-shared";
    let manifest = json!({"engine": "0.15.0", "feature": "shared"});
    log.create_run(shared, &manifest).unwrap();
    let first = sealed_proc_event(
        shared,
        1,
        EventType::RunStarted,
        json!({"feature": "shared", "manifest": manifest}),
        ZERO_HASH,
    );
    log.append_envelopes(shared, (0, ZERO_HASH), &[first])
        .unwrap();

    let racers: Vec<_> = (0..WRITERS)
        .map(|writer| {
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "concurrent_processes::fs_event_log_concurrent_child",
                    "--nocapture",
                ])
                .env(FS_ROOT_VAR, &root)
                .env(FS_MODE_VAR, "contended")
                .env(FS_RUN_VAR, shared)
                .env(FS_WORKER_VAR, writer.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn racer")
        })
        .collect();
    for (writer, child) in racers.into_iter().enumerate() {
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "contended child {writer} failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // Verify all chains and rebuild disposable index.db.
    for writer in 0..WRITERS {
        let run_id = format!("fs-run-{writer}");
        assert_eq!(
            log.read_stream(&run_id, 1).unwrap().len(),
            FS_INDEPENDENT_EVENTS
        );
    }
    assert_eq!(
        log.read_stream(shared, 1).unwrap().len(),
        1 + WRITERS * FS_CONTENDED_COMMITS
    );
    let index = SqliteProjectionStore::open(dir.path().join("index.db")).unwrap();
    let total = index.rebuild_from(&log).unwrap();
    assert_eq!(
        total,
        (WRITERS * FS_INDEPENDENT_EVENTS + 1 + WRITERS * FS_CONTENDED_COMMITS) as u64
    );
}
