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
use std::time::Duration;

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

fn verify_run_stream(
    reader: &Store,
    run_id: &str,
    seen_ids: &mut std::collections::HashSet<String>,
) {
    // `load` verifies the chain and refuses to return a partial one.
    let events = reader.load(run_id).unwrap();
    assert_eq!(
        events.len(),
        EVENTS_PER_RUN,
        "{run_id} lost or gained an event"
    );
    brokkr_core::envelope::verify_chain(&events).unwrap();
    for event in &events {
        assert_eq!(event.correlation_id, run_id);
        assert!(
            seen_ids.insert(event.event_id.clone()),
            "an event id was reused across processes"
        );
    }
    let folded = brokkr_core::fold(&events).unwrap();
    assert_eq!(folded.seq, EVENTS_PER_RUN as u64);
    assert_eq!(folded.phase.as_deref(), Some("implement"));
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
        verify_run_stream(&reader, &format!("run-{writer}"), &mut every_event_id);
    }
}

// ---------------------------------------------------------------------------
// Slice 2 (decision 0126 R4): Multi-process FsEventLog stress tests
// ---------------------------------------------------------------------------

const FS_ROOT_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_ROOT";
const FS_MODE_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_MODE";
const FS_RUN_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_RUN";
const FS_WORKER_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_WORKER";
const FS_BARRIER_VAR: &str = "BROKKR_STORE_FS_CONCURRENT_BARRIER";

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

fn spawn_fs_children(
    root: &Path,
    mode: &str,
    barrier: Option<&Path>,
    targets: &[(String, usize)],
) -> Vec<std::process::Child> {
    targets
        .iter()
        .map(|(run_id, worker)| {
            let mut cmd = Command::new(std::env::current_exe().unwrap());
            cmd.args([
                "--exact",
                "concurrent_processes::fs_event_log_concurrent_child",
                "--nocapture",
            ])
            .env(FS_ROOT_VAR, root)
            .env(FS_MODE_VAR, mode)
            .env(FS_RUN_VAR, run_id)
            .env(FS_WORKER_VAR, worker.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            if let Some(dir) = barrier {
                cmd.env(FS_BARRIER_VAR, dir);
            }
            cmd.spawn().expect("spawn fs child")
        })
        .collect()
}

fn wait_fs_children(label: &str, children: Vec<std::process::Child>) {
    for (idx, child) in children.into_iter().enumerate() {
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{label} child {idx} failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
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
        let barrier = PathBuf::from(std::env::var(FS_BARRIER_VAR).expect("barrier dir"));

        // Phase A: synchronized contention on exact same expected_head = (1, hash1).
        let (head_seq, head_hash) = log.head(&run_id).expect("read initial head");
        assert_eq!(head_seq, 1);
        let racing_seq2 = sealed_proc_event(
            &run_id,
            2,
            EventType::PhaseEntered,
            json!({"worker": worker, "phase": "barrier-race"}),
            &head_hash,
        );
        std::fs::write(barrier.join(format!("ready-{worker}")), b"ready").unwrap();
        let go_path = barrier.join("go");
        while !go_path.exists() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let outcome = match log.append_envelopes(
            &run_id,
            (1, &head_hash),
            std::slice::from_ref(&racing_seq2),
        ) {
            Ok(()) => "ok",
            Err(brokkr_store::StoreError::HeadMoved {
                expected_seq: 1,
                found_seq,
            }) if found_seq >= 2 => "head_moved",
            Err(err) => panic!("unexpected barrier race error: {err}"),
        };
        std::fs::write(barrier.join(format!("outcome-{worker}")), outcome).unwrap();

        // Phase B: free-running multi-commit contention loop.
        let mut committed = 0usize;
        while committed < FS_CONTENDED_COMMITS {
            let (cur_seq, cur_hash) = log.head(&run_id).expect("read head");
            let env = sealed_proc_event(
                &run_id,
                cur_seq + 1,
                EventType::PhaseEntered,
                json!({"worker": worker, "n": committed}),
                &cur_hash,
            );
            match log.append_envelopes(&run_id, (cur_seq, &cur_hash), std::slice::from_ref(&env)) {
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
    let indep_targets: Vec<(String, usize)> =
        (0..WRITERS).map(|w| (format!("fs-run-{w}"), w)).collect();
    let indep_children = spawn_fs_children(&root, "independent", None, &indep_targets);
    wait_fs_children("independent", indep_children);

    // 2. N competing processes racing on one shared run via File::lock(),
    // starting with a barrier-synchronized race on expected_head = (1, hash1).
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

    let barrier = dir.path().join("barrier");
    std::fs::create_dir(&barrier).unwrap();
    let racer_targets: Vec<(String, usize)> =
        (0..WRITERS).map(|w| (shared.to_string(), w)).collect();
    let racers = spawn_fs_children(&root, "contended", Some(&barrier), &racer_targets);

    // Release all children simultaneously once every child has read head (1, hash1).
    for worker in 0..WRITERS {
        let ready = barrier.join(format!("ready-{worker}"));
        while !ready.exists() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    std::fs::write(barrier.join("go"), b"go").unwrap();
    wait_fs_children("contended", racers);

    let mut barrier_winners = 0usize;
    let mut barrier_head_moved = 0usize;
    for worker in 0..WRITERS {
        let outcome = std::fs::read_to_string(barrier.join(format!("outcome-{worker}"))).unwrap();
        match outcome.as_str() {
            "ok" => barrier_winners += 1,
            "head_moved" => barrier_head_moved += 1,
            other => panic!("unexpected outcome for worker {worker}: {other}"),
        }
    }
    assert_eq!(barrier_winners, 1);
    assert_eq!(barrier_head_moved, WRITERS - 1);

    // Verify all chains and rebuild disposable index.db.
    for writer in 0..WRITERS {
        let run_id = format!("fs-run-{writer}");
        assert_eq!(
            log.read_stream(&run_id, 1).unwrap().len(),
            FS_INDEPENDENT_EVENTS
        );
    }
    // 1 RunStarted + 1 barrier winner + WRITERS * FS_CONTENDED_COMMITS
    let expected_shared = 2 + WRITERS * FS_CONTENDED_COMMITS;
    let shared_events = log.read_stream(shared, 1).unwrap();
    assert_eq!(shared_events.len(), expected_shared);
    brokkr_core::envelope::verify_chain(&shared_events).unwrap();

    let index = SqliteProjectionStore::open(dir.path().join("index.db")).unwrap();
    let total = index.rebuild_from(&log).unwrap();
    assert_eq!(
        total,
        (WRITERS * FS_INDEPENDENT_EVENTS + expected_shared) as u64
    );
}
