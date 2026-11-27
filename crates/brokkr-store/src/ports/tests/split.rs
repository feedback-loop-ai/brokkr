//! Local split-backend tests (decision 0126 ruling 4, Slice 2):
//! - [`FsEventLog`] conformance, path-traversal & symlink safety, corruption
//!   detection, and multi-thread concurrency stress tests
//! - Trigger-free [`SqliteProjectionStore`] schema, monotonicity, divergence
//!   detection, feature precedence, and `rebuild_from`
//! - Isolated [`SqliteCoordinationStore`] queue isolation and idempotent
//!   provider cooldown catch-up
//! - End-to-end [`Journal<FsEventLog, SqliteProjectionStore, SqliteCoordinationStore>`]
//!   including NDJSON parity with [`Store`] and `ProjectionLag` healing

use std::fs;
use std::os::unix::fs::symlink;
use std::sync::{Arc, Barrier};
use std::thread;

use brokkr_core::fold::{FoldError, Status};
use rusqlite::Connection;

use super::*;
use crate::ports::{FsEventLog, SqliteCoordinationStore, SqliteProjectionStore};
use crate::{ManifestRefusal, QueueRefusal, Wait, WaitOn};

fn seed_two_events(
    log: &FsEventLog,
    run_id: &str,
    feature: &str,
) -> (EventEnvelope, EventEnvelope) {
    let e1 = started_event(run_id, feature, "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        run_id,
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    log.append_envelopes(run_id, (0, ZERO_HASH), &[e1.clone(), e2.clone()])
        .unwrap();
    (e1, e2)
}

fn waits_effect_payload(until_a: &str, until_b: &str) -> Value {
    json!({
        "effect_id": "eff-wait",
        "attempt_id": "att-wait",
        "waits": [
            {
                "site": "implement",
                "provider": "anthropic",
                "route": "claude-sonnet",
                "until": until_a,
                "words": "rate limited",
                "retry_after_seconds": 60,
                "seconds": 60,
                "rounds": 1
            },
            {
                "provider": "openai",
                "route": "gpt-5",
                "until": until_b,
                "words": "quota busy",
                "seconds": 30,
                "rounds": 1
            }
        ]
    })
}

fn sqlite_table_names(path: &std::path::Path) -> Vec<String> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    let rows = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows
}

fn sqlite_trigger_count(path: &std::path::Path) -> i64 {
    let conn = Connection::open(path).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger'",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn fs_event_log_conformance_and_invariants() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    assert_eq!(log.root(), root.as_path());
    let manifest = sample_manifest("fs-conformance");

    assert!(matches!(log.head("absent"), Err(StoreError::RunNotFound(id)) if id == "absent"));
    assert!(
        matches!(log.read_stream("absent", 1), Err(StoreError::RunNotFound(id)) if id == "absent")
    );
    assert!(matches!(log.manifest("absent"), Err(StoreError::RunNotFound(id)) if id == "absent"));
    assert!(matches!(
        log.append_envelopes("absent", (0, ZERO_HASH), &[]),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));

    fs::write(root.join("stray.txt"), "noise").unwrap();
    fs::create_dir(root.join(".staging-leftover")).unwrap();
    assert!(log.list_run_ids().unwrap().is_empty());
    let incomplete = root.join("incomplete-dir");
    fs::create_dir(&incomplete).unwrap();
    assert!(matches!(
        log.list_run_ids(),
        Err(StoreError::Damaged { .. })
    ));
    fs::remove_dir(&incomplete).unwrap();

    log.create_run("run-b", &manifest).unwrap();
    log.create_run("run-a", &manifest).unwrap();
    assert_eq!(
        log.list_run_ids().unwrap(),
        vec!["run-a".to_string(), "run-b".to_string()]
    );
    assert!(
        matches!(log.create_run("run-a", &manifest), Err(StoreError::RunExists(id)) if id == "run-a")
    );
    assert_eq!(log.manifest("run-a").unwrap(), manifest);
    assert_eq!(log.head("run-a").unwrap(), (0, ZERO_HASH.to_string()));
    assert!(log.read_stream("run-a", 1).unwrap().is_empty());

    log.append_envelopes("run-a", (0, ZERO_HASH), &[]).unwrap();
    assert!(matches!(
        log.append_envelopes("run-a", (1, ZERO_HASH), &[]),
        Err(StoreError::HeadMoved {
            expected_seq: 1,
            found_seq: 0
        })
    ));

    let (e1, e2) = seed_two_events(&log, "run-a", "fs-conformance");
    assert_eq!(log.head("run-a").unwrap(), (2, e2.event_hash.clone()));
    assert_eq!(
        serde_json::to_vec(&log.read_stream("run-a", 1).unwrap()).unwrap(),
        serde_json::to_vec(&vec![e1.clone(), e2.clone()]).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&log.read_stream("run-a", 2).unwrap()).unwrap(),
        serde_json::to_vec(&vec![e2.clone()]).unwrap()
    );
    assert!(log.read_stream("run-a", 3).unwrap().is_empty());

    assert!(matches!(
        log.append_envelopes("run-a", (2, "wrong-hash"), &[]),
        Err(StoreError::HeadMoved {
            expected_seq: 2,
            found_seq: 2
        })
    ));

    let foreign = sealed_event(
        "other-run",
        3,
        EventType::PhaseEntered,
        json!({"phase": "review"}),
        &e2.event_hash,
        "2026-11-26T10:00:06Z",
    );
    assert!(matches!(
        log.append_envelopes("run-a", (2, &e2.event_hash), &[foreign]),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 3 }))
    ));

    let bad_seat = sealed_event(
        "run-a",
        3,
        EventType::EffectCheckpointed,
        invalid_checkpoint_payload(),
        &e2.event_hash,
        "2026-11-26T10:00:07Z",
    );
    assert!(matches!(
        log.append_envelopes("run-a", (2, &e2.event_hash), &[bad_seat]),
        Err(StoreError::SeatRecord(_))
    ));
}

#[test]
fn fs_event_log_terminal_and_import_gates() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("fs-gates");
    log.create_run("run-a", &manifest).unwrap();
    log.create_run("run-b", &manifest).unwrap();
    let (e1, e2) = seed_two_events(&log, "run-a", "fs-gates");

    let bad_started_manifest = sealed_event(
        "run-b",
        1,
        EventType::RunStarted,
        json!({"feature": "x", "manifest": {"engine": 42}}),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    assert!(matches!(
        log.append_envelopes("run-b", (0, ZERO_HASH), &[bad_started_manifest]),
        Err(StoreError::InvalidManifest { ref run_id, reason: ManifestRefusal::EngineNotString }) if run_id == "run-b"
    ));

    let terminal = sealed_event(
        "run-a",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-26T10:00:08Z",
    );
    log.append_envelopes(
        "run-a",
        (2, &e2.event_hash),
        std::slice::from_ref(&terminal),
    )
    .unwrap();
    let after = sealed_event(
        "run-a",
        4,
        EventType::PhaseEntered,
        json!({"phase": "late"}),
        &terminal.event_hash,
        "2026-11-26T10:00:09Z",
    );
    assert!(matches!(
        log.append_envelopes("run-a", (3, &terminal.event_hash), &[after]),
        Err(StoreError::AfterTerminal { ref run_id, status: Status::Completed, event_type: EventType::PhaseEntered }) if run_id == "run-a"
    ));

    assert!(matches!(
        log.import_run_stream("imp-empty", &manifest, &[]),
        Err(StoreError::Import(ref err)) if matches!(err.as_ref(), ImportError::Verify(VerifyError::Fold(FoldError::Empty)))
    ));
    assert!(!root.join("imp-empty").exists());

    assert!(matches!(
        log.import_run_stream("../bad", &manifest, &[e1]),
        Err(StoreError::Import(ref err)) if matches!(err.as_ref(), ImportError::UnadoptableRunId(_))
    ));

    let imp1 = started_event("imp-ok", "imported-feat", "2026-11-26T11:00:00Z");
    let imp2 = sealed_event(
        "imp-ok",
        2,
        EventType::RunCompleted,
        json!({}),
        &imp1.event_hash,
        "2026-11-26T11:00:05Z",
    );
    log.import_run_stream("imp-ok", &manifest, &[imp1.clone(), imp2.clone()])
        .unwrap();
    assert_eq!(log.head("imp-ok").unwrap(), (2, imp2.event_hash.clone()));
    assert!(matches!(
        log.import_run_stream("imp-ok", &manifest, &[imp1, imp2]),
        Err(StoreError::RunExists(id)) if id == "imp-ok"
    ));
}

fn make_outside_dir(parent: &std::path::Path) -> std::path::PathBuf {
    let outside = parent.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("manifest.json"), "{}").unwrap();
    fs::write(outside.join("events.ndjson"), "").unwrap();
    outside
}

#[test]
fn fs_event_log_path_traversal_and_symlink_safety() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("safety");

    for bad in ["", ".", "..", "../escape", "a/b", "a\\b", "nul\0id"] {
        assert!(matches!(log.head(bad), Err(StoreError::UnsafeRunId(_))));
        assert!(matches!(
            log.read_stream(bad, 1),
            Err(StoreError::UnsafeRunId(_))
        ));
        assert!(matches!(log.manifest(bad), Err(StoreError::UnsafeRunId(_))));
        assert!(matches!(
            log.append_envelopes(bad, (0, ZERO_HASH), &[]),
            Err(StoreError::UnsafeRunId(_))
        ));
    }
    for bad in [".", "..", "../escape", "a/b", "a\\b"] {
        let err = log.create_run(bad, &manifest).unwrap_err();
        assert!(matches!(err, StoreError::UnsafeRunId(_)));
        if bad == "../escape" {
            assert!(err.to_string().contains("unsafe run id"));
        }
    }
    assert!(matches!(
        log.create_run("", &manifest),
        Err(StoreError::RunIdUnnameable(_))
    ));
    assert!(matches!(
        log.create_run("bad\0id", &manifest),
        Err(StoreError::RunIdUnnameable(_))
    ));

    let outside = make_outside_dir(dir.path());
    symlink(&outside, root.join("linked-run")).unwrap();

    assert!(matches!(
        log.list_run_ids(),
        Err(StoreError::SymlinkRefused(_))
    ));
    let sym_err = log.head("linked-run").unwrap_err();
    assert!(matches!(sym_err, StoreError::SymlinkRefused(_)));
    assert!(sym_err.to_string().contains("refusing symlink at"));
    assert!(matches!(
        log.read_stream("linked-run", 1),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        log.manifest("linked-run"),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        log.append_envelopes("linked-run", (0, ZERO_HASH), &[]),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        log.create_run("linked-run", &manifest),
        Err(StoreError::SymlinkRefused(_))
    ));
}

#[test]
fn fs_event_log_missing_and_symlinked_inner_files_fail_closed() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("safety-inner");
    let outside = make_outside_dir(dir.path());
    log.create_run("real-run", &manifest).unwrap();
    let run_dir = root.join("real-run");
    fs::remove_file(run_dir.join("manifest.json")).unwrap();
    let manifest_path = run_dir.join("manifest.json");
    assert!(matches!(
        log.head("real-run"),
        Err(StoreError::Damaged { ref journal, damage: crate::Damage::MissingManifest })
            if journal == &manifest_path
    ));
    assert!(matches!(
        log.read_stream("real-run", 1),
        Err(StoreError::Damaged { ref journal, damage: crate::Damage::MissingManifest })
            if journal == &manifest_path
    ));
    symlink(outside.join("manifest.json"), run_dir.join("manifest.json")).unwrap();
    assert!(matches!(
        log.list_run_ids(),
        Err(StoreError::SymlinkRefused(_))
    ));
    let assert_reads_refuse_symlink = || {
        assert!(matches!(
            log.head("real-run"),
            Err(StoreError::SymlinkRefused(_))
        ));
        assert!(matches!(
            log.read_stream("real-run", 1),
            Err(StoreError::SymlinkRefused(_))
        ));
        assert!(matches!(
            log.manifest("real-run"),
            Err(StoreError::SymlinkRefused(_))
        ));
    };
    assert_reads_refuse_symlink();

    fs::remove_file(run_dir.join("manifest.json")).unwrap();
    fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::remove_file(run_dir.join("events.ndjson")).unwrap();
    symlink(outside.join("events.ndjson"), run_dir.join("events.ndjson")).unwrap();
    assert_reads_refuse_symlink();

    fs::remove_file(run_dir.join(".lock")).unwrap();
    symlink(outside.join("manifest.json"), run_dir.join(".lock")).unwrap();
    assert!(matches!(
        log.head("real-run"),
        Err(StoreError::SymlinkRefused(_))
    ));
}

#[test]
fn fs_event_log_refuses_corrupt_truncated_and_before_chain_ndjson() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("corrupt-check");
    log.create_run("r1", &manifest).unwrap();
    let events_path = root.join("r1").join("events.ndjson");

    let e1 = started_event("r1", "corrupt-check", "2026-11-26T10:00:00Z");
    for raw in [
        serde_json::to_string(&e1).unwrap(),
        "{not-valid-json\n".to_string(),
    ] {
        fs::write(&events_path, raw).unwrap();
        assert!(matches!(
            log.read_stream("r1", 1),
            Err(StoreError::Damaged {
                ref journal,
                damage: crate::Damage::Envelope { ref run_id, seq: 1 }
            }) if journal == &events_path && run_id == "r1"
        ));
    }

    fs::write(&events_path, "{\"hello\": \"world\"}\n").unwrap();
    assert!(matches!(
        log.read_stream("r1", 1),
        Err(StoreError::Damaged { ref journal, damage: crate::Damage::MalformedJson { .. } })
            if journal == &events_path
    ));

    let mut seq0 = e1.clone();
    seq0.seq = 0;
    fs::write(
        &events_path,
        format!("{}\n", serde_json::to_string(&seq0).unwrap()),
    )
    .unwrap();
    assert!(
        matches!(log.read_stream("r1", 1), Err(StoreError::RowBeforeChain { ref run_id, seq: 0 }) if run_id == "r1")
    );

    fs::write(&events_path, "{\"seq\": -3}\n").unwrap();
    assert!(
        matches!(log.head("r1"), Err(StoreError::RowBeforeChain { ref run_id, seq: -3 }) if run_id == "r1")
    );

    let mut tampered = e1;
    tampered.payload = json!({"feature": "evil"});
    fs::write(
        &events_path,
        format!("{}\n", serde_json::to_string(&tampered).unwrap()),
    )
    .unwrap();
    assert!(matches!(
        log.read_stream("r1", 1),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));

    fs::write(root.join("r1").join("manifest.json"), "{\"engine\": 99}").unwrap();
    assert!(matches!(
        log.manifest("r1"),
        Err(StoreError::InvalidManifest { ref run_id, reason: ManifestRefusal::EngineNotString }) if run_id == "r1"
    ));

    let file_path = dir.path().join("not-a-dir");
    fs::write(&file_path, "x").unwrap();
    let io_err = FsEventLog::open(&file_path).err().unwrap();
    assert!(matches!(io_err, StoreError::Io(_)));
    assert!(io_err.to_string().starts_with("io: "));
}

#[test]
fn sqlite_projection_store_trigger_free_monotonic_and_rebuild() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("sub").join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    assert_eq!(proj.path(), index_path.as_path());

    assert_eq!(sqlite_trigger_count(&index_path), 0);
    assert_eq!(
        sqlite_table_names(&index_path),
        vec![
            "projected_events".to_string(),
            "projected_runs".to_string(),
            "projection_meta".to_string(),
        ]
    );
    let proj = SqliteProjectionStore::open(&index_path).unwrap();

    assert!(matches!(
        proj.apply_envelopes("r1", Some(&json!("not-object")), &[]),
        Err(StoreError::InvalidManifest { ref run_id, reason: ManifestRefusal::NotAnObject }) if run_id == "r1"
    ));

    let manifest_feat = json!({"engine": "0.15.0", "feature": "manifest-feat"});
    proj.apply_envelopes("r-empty", Some(&manifest_feat), &[])
        .unwrap();
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![(
            "r-empty".to_string(),
            "manifest-feat".to_string(),
            String::new()
        )]
    );
    let empty_e1 = started_event("r-empty", "now-started", "2026-11-26T09:30:00Z");
    proj.apply_envelopes("r-empty", None, std::slice::from_ref(&empty_e1))
        .unwrap();
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![(
            "r-empty".to_string(),
            "now-started".to_string(),
            "2026-11-26T09:30:00Z".to_string()
        )]
    );

    let e1 = started_event("r1", "started-feat", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "r1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    proj.apply_envelopes("r1", Some(&manifest_feat), &[e1.clone(), e2.clone()])
        .unwrap();

    proj.apply_envelopes("r1", None, std::slice::from_ref(&e1))
        .unwrap();
    proj.apply_envelopes("r1", None, &[]).unwrap();

    let forked_e1 = started_event("r1", "forked", "2026-11-26T10:00:00Z");
    assert!(matches!(
        proj.apply_envelopes("r1", None, std::slice::from_ref(&forked_e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));
    let forked_e2 = sealed_event(
        "r1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "fork"}),
        &forked_e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    let forked_e3 = sealed_event(
        "r1",
        3,
        EventType::RunCompleted,
        json!({}),
        &forked_e2.event_hash,
        "2026-11-26T10:00:06Z",
    );
    assert!(matches!(
        proj.apply_envelopes("r1", None, &[forked_e1, forked_e2, forked_e3]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));

    let e3 = sealed_event(
        "r1",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-26T10:00:10Z",
    );
    proj.apply_envelopes("r1", None, &[e1.clone(), e2.clone(), e3.clone()])
        .unwrap();
}

#[test]
fn sqlite_projection_store_rebuild_and_corrupt_head() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    let manifest_feat = json!({"engine": "0.15.0", "feature": "manifest-feat"});
    let e1 = started_event("r1", "started-feat", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "r1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    let e3 = sealed_event(
        "r1",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-26T10:00:10Z",
    );

    let fs_log = FsEventLog::open(dir.path().join("runs")).unwrap();
    fs_log.create_run("r1", &manifest_feat).unwrap();
    fs_log
        .append_envelopes("r1", (0, ZERO_HASH), &[e1, e2, e3])
        .unwrap();
    let m2 = json!({"engine": "0.15.0", "feature": "only-in-manifest"});
    let r2_e1 = sealed_event(
        "r2",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-26T09:00:00Z",
    );
    fs_log.create_run("r2", &m2).unwrap();
    fs_log
        .append_envelopes("r2", (0, ZERO_HASH), &[r2_e1])
        .unwrap();

    assert_eq!(proj.rebuild_from(&fs_log).unwrap(), 4);
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![
            (
                "r2".to_string(),
                "only-in-manifest".to_string(),
                "2026-11-26T09:00:00Z".to_string()
            ),
            (
                "r1".to_string(),
                "started-feat".to_string(),
                "2026-11-26T10:00:00Z".to_string()
            ),
        ]
    );

    let dup = InterceptingLog {
        inner: fs_log,
        on_head: None,
        on_list_run_ids: None,
        on_read_stream: None,
        override_run_ids: Some(vec!["r1".to_string(), "r1".to_string()]),
    };
    assert!(matches!(
        proj.rebuild_from(&dup),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));
    assert_eq!(proj.list_runs().unwrap().len(), 2);

    // Negative head_seq planted directly in projected_runs refuses as DivergentEventLog.
    let raw_idx = Connection::open(&index_path).unwrap();
    raw_idx
        .execute(
            "INSERT INTO projected_runs (run_id, feature, created_at, head_seq, head_hash)
             VALUES ('corrupt-seq', '', '', -1, '')",
            [],
        )
        .unwrap();
    drop(raw_idx);
    assert!(matches!(
        proj.apply_envelopes("corrupt-seq", None, &[]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "corrupt-seq"
    ));
}

#[test]
fn sqlite_coordination_store_isolated_and_idempotent_cooldowns() {
    let dir = tempdir().unwrap();
    let queue_path = dir.path().join("sub").join("queue.db");
    let coord = SqliteCoordinationStore::open(&queue_path).unwrap();
    assert_eq!(coord.path(), queue_path.as_path());

    let tables = sqlite_table_names(&queue_path);
    assert!(!tables.iter().any(|t| t == "runs" || t == "events"));
    assert!(tables.iter().any(|t| t == "queue_entries"));
    assert!(tables.iter().any(|t| t == "provider_cooldowns"));
    let coord = SqliteCoordinationStore::open(&queue_path).unwrap();

    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "bad",
                priority: 0,
                waits: &[]
            },
            "not-a-clock"
        ),
        Err(StoreError::Clock { .. })
    ));
    let id1 = coord
        .enqueue(
            &NewEntry {
                payload: "launch-low",
                priority: 1,
                waits: &[],
            },
            "2026-11-26T10:00:00Z",
        )
        .unwrap();
    let id2 = coord
        .enqueue(
            &NewEntry {
                payload: "launch-high",
                priority: 10,
                waits: &[Wait {
                    entry: id1,
                    on: WaitOn::Completed,
                }],
            },
            "2026-11-26T10:00:01Z",
        )
        .unwrap();
    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "bad-wait",
                priority: 0,
                waits: &[Wait {
                    entry: EntryId(999),
                    on: WaitOn::Completed
                }],
            },
            "2026-11-26T10:00:02Z",
        ),
        Err(StoreError::Queue(QueueRefusal::UnknownWait(EntryId(999))))
    ));
    let listed = coord.list_queue().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, id2);
    assert_eq!(listed[1].id, id1);
}

#[test]
fn sqlite_coordination_store_idempotent_cooldown_catch_up() {
    let dir = tempdir().unwrap();
    let coord = SqliteCoordinationStore::open(dir.path().join("queue.db")).unwrap();

    let fs_log = FsEventLog::open(dir.path().join("runs")).unwrap();
    let manifest = sample_manifest("cool-run");
    fs_log.create_run("cool-1", &manifest).unwrap();
    let e1 = started_event("cool-1", "cool-run", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "cool-1",
        2,
        EventType::EffectFailed,
        waits_effect_payload("2026-11-26T10:15:00Z", "2026-11-26T10:05:00Z"),
        &e1.event_hash,
        "2026-11-26T10:01:00Z",
    );
    let e3 = sealed_event(
        "cool-1",
        3,
        EventType::EffectIndeterminate,
        waits_effect_payload("2026-11-26T10:01:30Z", "2026-11-26T10:20:00Z"),
        &e2.event_hash,
        "2026-11-26T10:02:00Z",
    );
    fs_log
        .append_envelopes(
            "cool-1",
            (0, ZERO_HASH),
            &[e1.clone(), e2.clone(), e3.clone()],
        )
        .unwrap();

    assert!(matches!(
        coord.apply_cooldowns("wrong-run", std::slice::from_ref(&e2)),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 2 }))
    ));
    let mut seq0 = e2.clone();
    seq0.seq = 0;
    assert!(
        matches!(coord.apply_cooldowns("cool-1", &[seq0]), Err(StoreError::RowBeforeChain { ref run_id, seq: 0 }) if run_id == "cool-1")
    );
    let mut bad_hash = e2.clone();
    bad_hash.event_hash = "tampered".to_string();
    assert!(matches!(
        coord.apply_cooldowns("cool-1", &[bad_hash]),
        Err(StoreError::Chain(ChainError::BadHash { seq: 2 }))
    ));

    assert_eq!(coord.apply_cooldowns("cool-1", &[e2]).unwrap(), 2);
    let no_wait_fail = sealed_event(
        "cool-1",
        4,
        EventType::EffectFailed,
        json!({"effect_id": "eff-2", "attempt_id": "att-2"}),
        &e3.event_hash,
        "2026-11-26T10:02:30Z",
    );
    assert_eq!(
        coord
            .apply_cooldowns("cool-1", &[e1, e3, no_wait_fail])
            .unwrap(),
        1
    );
    assert_eq!(coord.sync_cooldowns_from(&fs_log).unwrap(), 0);

    let coord2 = SqliteCoordinationStore::open(dir.path().join("queue2.db")).unwrap();
    assert_eq!(coord2.sync_cooldowns_from(&fs_log).unwrap(), 3);
    assert_eq!(coord2.sync_cooldowns_from(&fs_log).unwrap(), 0);
    assert_eq!(
        coord2
            .active_cooldowns("2026-11-26T10:03:00Z")
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        coord2
            .active_cooldowns("2026-11-26T10:10:00Z")
            .unwrap()
            .len(),
        2
    );
    assert!(coord2
        .active_cooldowns("2026-11-26T10:25:00Z")
        .unwrap()
        .is_empty());
    assert!(matches!(
        coord2.active_cooldowns("bad-clock"),
        Err(StoreError::Clock { .. })
    ));
}

#[test]
fn split_journal_end_to_end_ndjson_parity_and_projection_lag_healing() {
    let dir = tempdir().unwrap();
    let fs_log = FsEventLog::open(dir.path().join("runs")).unwrap();
    let proj = SqliteProjectionStore::open(dir.path().join("index.db")).unwrap();
    let coord = SqliteCoordinationStore::open(dir.path().join("queue.db")).unwrap();
    let lagging = FailingProjectionStore::toggleable(&proj);
    let journal = Journal::new(&fs_log, &lagging, &coord);
    let manifest = sample_manifest("split-e2e");

    journal.create_run("split-1", &manifest).unwrap();
    let e1 = journal
        .append_event(
            "split-1",
            plain_event(
                EventType::RunStarted,
                started_payload("split-e2e"),
                None,
                "2026-11-26T12:00:00Z",
            ),
        )
        .unwrap();
    let e2 = journal
        .append_if_head(
            "split-1",
            (1, &e1.event_hash),
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                Some(&e1.event_id),
                "2026-11-26T12:00:01Z",
            ),
        )
        .unwrap();
    let e3 = journal
        .append_event(
            "split-1",
            plain_event(
                EventType::EffectRequested,
                json!({"effect_id": "eff-wait", "seat": "implementer"}),
                Some(&e2.event_id),
                "2026-11-26T12:00:02Z",
            ),
        )
        .unwrap();
    let e4 = journal
        .append_event(
            "split-1",
            plain_event(
                EventType::EffectStarted,
                json!({"effect_id": "eff-wait", "attempt_id": "att-wait"}),
                Some(&e3.event_id),
                "2026-11-26T12:00:03Z",
            ),
        )
        .unwrap();
    let e5 = journal
        .append_event(
            "split-1",
            plain_event(
                EventType::EffectFailed,
                waits_effect_payload("2026-11-26T12:30:00Z", "2026-11-26T12:45:00Z"),
                Some(&e4.event_id),
                "2026-11-26T12:00:05Z",
            ),
        )
        .unwrap();

    lagging.set_fail(true);
    let lag_err = journal
        .append_event(
            "split-1",
            plain_event(
                EventType::RunCompleted,
                json!({}),
                Some(&e5.event_id),
                "2026-11-26T12:00:10Z",
            ),
        )
        .unwrap_err();
    assert!(matches!(lag_err, StoreError::ProjectionLag { ref run_id, .. } if run_id == "split-1"));
    assert!(!lag_err.is_contention());
    assert_eq!(journal.load_verified("split-1").unwrap().len(), 6);

    lagging.set_fail(false);
    assert_eq!(journal.reindex().unwrap(), 6);
    assert_eq!(coord.sync_cooldowns_from(&fs_log).unwrap(), 2);
    assert_eq!(
        coord
            .active_cooldowns("2026-11-26T12:10:00Z")
            .unwrap()
            .len(),
        2
    );

    assert_split_export_import_parity(dir.path(), &journal, &manifest);
}

fn assert_split_export_import_parity(
    base: &std::path::Path,
    journal: &Journal<
        &FsEventLog,
        &FailingProjectionStore<&SqliteProjectionStore>,
        &SqliteCoordinationStore,
    >,
    manifest: &Value,
) {
    let exported = journal.export_ndjson("split-1").unwrap();
    let mut legacy = Store::open(&base.join("legacy.db")).unwrap();
    legacy
        .import_run(&exported, manifest, std::path::Path::new("split-1.ndjson"))
        .unwrap();
    assert_eq!(legacy.export_ndjson("split-1").unwrap(), exported);

    let dest_journal = Journal::new(
        FsEventLog::open(base.join("runs-dest")).unwrap(),
        SqliteProjectionStore::open(base.join("index-dest.db")).unwrap(),
        SqliteCoordinationStore::open(base.join("queue-dest.db")).unwrap(),
    );
    assert_eq!(
        dest_journal
            .import_ndjson("split-1", manifest, &exported)
            .unwrap(),
        6
    );
    assert_eq!(dest_journal.export_ndjson("split-1").unwrap(), exported);
    assert_eq!(
        dest_journal.projections().list_runs().unwrap(),
        vec![(
            "split-1".to_string(),
            "split-e2e".to_string(),
            "2026-11-26T12:00:00Z".to_string()
        )]
    );
}

#[test]
fn fs_event_log_concurrent_writers_stress() {
    let dir = tempdir().unwrap();
    let log = Arc::new(FsEventLog::open(dir.path().join("runs")).unwrap());

    let independent_writers = 8usize;
    let events_per_run = 20usize;
    let barrier = Arc::new(Barrier::new(independent_writers));
    let mut handles = Vec::with_capacity(independent_writers);
    for idx in 0..independent_writers {
        let log = Arc::clone(&log);
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let run_id = format!("indep-{idx}");
            let manifest = sample_manifest(&run_id);
            barrier.wait();
            log.create_run(&run_id, &manifest).unwrap();
            let mut head = (0u64, ZERO_HASH.to_string());
            for step in 1..=events_per_run {
                let env = if step == 1 {
                    started_event(&run_id, &run_id, "2026-11-26T14:00:00Z")
                } else {
                    sealed_event(
                        &run_id,
                        step as u64,
                        EventType::PhaseEntered,
                        json!({"phase": format!("p-{step}")}),
                        &head.1,
                        "2026-11-26T14:00:01Z",
                    )
                };
                log.append_envelopes(&run_id, (head.0, &head.1), std::slice::from_ref(&env))
                    .unwrap();
                head = (env.seq, env.event_hash);
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    for idx in 0..independent_writers {
        assert_eq!(
            log.read_stream(&format!("indep-{idx}"), 1).unwrap().len(),
            events_per_run
        );
    }
}

#[test]
fn fs_event_log_contended_writers_barrier_head_moved_stress() {
    let dir = tempdir().unwrap();
    let log = Arc::new(FsEventLog::open(dir.path().join("runs")).unwrap());
    let shared_run = "contended-run";
    log.create_run(shared_run, &sample_manifest("contended"))
        .unwrap();
    let s1 = started_event(shared_run, "contended", "2026-11-26T15:00:00Z");
    log.append_envelopes(shared_run, (0, ZERO_HASH), &[s1])
        .unwrap();

    let contenders = 6usize;
    let commits_each = 8usize;
    let (_, initial_hash) = log.head(shared_run).unwrap();
    let start = Arc::new(Barrier::new(contenders));
    let mut racers = Vec::with_capacity(contenders);
    for worker in 0..contenders {
        let log = Arc::clone(&log);
        let start = Arc::clone(&start);
        let initial_hash = initial_hash.clone();
        racers.push(thread::spawn(move || {
            let raced = sealed_event(
                shared_run,
                2,
                EventType::PhaseEntered,
                json!({"worker": worker, "phase": "barrier-seq2"}),
                &initial_hash,
                "2026-11-26T15:00:01Z",
            );
            start.wait();
            let won_seq2 = match log.append_envelopes(
                shared_run,
                (1, &initial_hash),
                std::slice::from_ref(&raced),
            ) {
                Ok(()) => true,
                Err(StoreError::HeadMoved {
                    expected_seq: 1,
                    found_seq,
                }) if found_seq >= 2 => false,
                Err(other) => panic!("unexpected barrier race error: {other}"),
            };
            let mut committed = 0usize;
            while committed < commits_each {
                let (head_seq, head_hash) = log.head(shared_run).unwrap();
                let next = sealed_event(
                    shared_run,
                    head_seq + 1,
                    EventType::PhaseEntered,
                    json!({"worker": worker, "n": committed}),
                    &head_hash,
                    "2026-11-26T15:00:02Z",
                );
                match log.append_envelopes(
                    shared_run,
                    (head_seq, &head_hash),
                    std::slice::from_ref(&next),
                ) {
                    Ok(()) => committed += 1,
                    Err(StoreError::HeadMoved { .. }) => {}
                    Err(other) => panic!("unexpected contention error: {other}"),
                }
            }
            won_seq2
        }));
    }
    let outcomes: Vec<bool> = racers.into_iter().map(|r| r.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|&&won| won).count(), 1);
    assert_eq!(outcomes.iter().filter(|&&won| !won).count(), contenders - 1);

    let shared_len = 2 + contenders * commits_each;
    let stream = log.read_stream(shared_run, 1).unwrap();
    assert_eq!(stream.len(), shared_len);
    brokkr_core::envelope::verify_chain(&stream).unwrap();
    let index = SqliteProjectionStore::open(dir.path().join("index.db")).unwrap();
    assert_eq!(index.rebuild_from(log.as_ref()).unwrap(), shared_len as u64);
}

#[test]
fn fs_event_log_cleanup_on_populate_error_and_fs_edge_cases() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();

    // Non-UTF-8 directory name inside root fails closed in list_run_ids with NonUtf8RunDir.
    let bad_utf8 = root.join(OsStr::from_bytes(b"bad\xffutf8"));
    fs::create_dir(&bad_utf8).unwrap();
    assert!(matches!(
        log.list_run_ids(),
        Err(StoreError::Damaged { ref journal, damage: crate::Damage::NonUtf8RunDir })
            if journal == &bad_utf8
    ));
    fs::remove_dir(&bad_utf8).unwrap();

    // Atomic cleanup: if populate fails after create_dir, staging dir is removed cleanly.
    let err = crate::ports::fs_log::create_run_dir_atomic_with(
        &root,
        "cleanup-run",
        |run_dir| {
            fs::write(run_dir.join("partial"), "x").unwrap();
            Err(StoreError::Io(std::io::Error::other(
                "injected write error",
            )))
        },
        crate::ports::sync_dir,
    )
    .unwrap_err();
    assert!(matches!(err, StoreError::Io(_)));
    assert!(!root.join("cleanup-run").exists());

    // Removing root makes create_dir fail with NotFound -> StoreError::Io.
    let vanished = dir.path().join("vanished");
    let vanished_log = FsEventLog::open(&vanished).unwrap();
    fs::remove_dir_all(&vanished).unwrap();
    assert!(matches!(
        vanished_log.create_run("r1", &sample_manifest("m")),
        Err(StoreError::Io(_))
    ));

    // Replacing root with a regular file mismatches the anchored root inode -> SymlinkRefused.
    fs::write(&vanished, "file").unwrap();
    assert!(matches!(
        vanished_log.head("r1"),
        Err(StoreError::SymlinkRefused(_))
    ));
}

#[test]
fn fs_event_log_open_dot_ids_toctou_and_append_rollback() {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;

    let dir = tempdir().unwrap();

    // Open refuses empty path, symlink root, and symlink parent.
    assert!(matches!(
        FsEventLog::open(Path::new("")),
        Err(StoreError::InvalidPath(_))
    ));
    let real_dir = dir.path().join("real");
    fs::create_dir(&real_dir).unwrap();
    let sym_root = dir.path().join("sym-root");
    symlink(&real_dir, &sym_root).unwrap();
    assert!(matches!(
        FsEventLog::open(&sym_root),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        FsEventLog::open(sym_root.join("nested")),
        Err(StoreError::SymlinkRefused(_))
    ));

    // Dot-prefixed run ids like `.draft` work consistently across create_run and list_run_ids,
    // while internal reserved names `.create.lock` and `.staging-*` are refused.
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("dot-feature");
    for reserved in [".create.lock", ".staging-123"] {
        assert!(matches!(
            log.create_run(reserved, &manifest),
            Err(StoreError::UnsafeRunId(_))
        ));
    }
    log.create_run(".draft", &manifest).unwrap();
    assert_eq!(log.list_run_ids().unwrap(), vec![".draft".to_string()]);
    let e1 = started_event(".draft", "dot-feature", "2026-11-26T10:00:00Z");
    log.append_envelopes(".draft", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();
    assert_eq!(log.head(".draft").unwrap(), (1, e1.event_hash.clone()));

    // Atomic append rollback: partial write + error restores original length cleanly.
    let events_path = root.join(".draft").join("events.ndjson");
    let before_len = fs::metadata(&events_path).unwrap().len();
    let mut file = OpenOptions::new().append(true).open(&events_path).unwrap();
    let rollback_err = crate::ports::fs_log::append_bytes_atomic_with(&mut file, |f| {
        f.write_all(b"{\"partial_corrupt\":")?;
        Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "simulated short write",
        ))
    })
    .unwrap_err();
    assert!(matches!(rollback_err, StoreError::Io(_)));
    drop(file);
    assert_eq!(fs::metadata(&events_path).unwrap().len(), before_len);
    assert_eq!(log.read_stream(".draft", 1).unwrap().len(), 1);

    // TOCTOU symlink / inode-swap refusal in open_no_symlink_with.
    let target = dir.path().join("toctou.txt");
    let decoy = dir.path().join("decoy.txt");
    fs::write(&target, "orig").unwrap();
    fs::write(&decoy, "decoy").unwrap();
    let swap_sym_err = crate::ports::fs_log::open_no_symlink_with(&target, |path| {
        let opened = fs::File::open(path)?;
        fs::remove_file(path)?;
        symlink(&decoy, path)?;
        Ok(opened)
    })
    .unwrap_err();
    assert!(matches!(swap_sym_err, StoreError::SymlinkRefused(_)));

    fs::remove_file(&target).unwrap();
    fs::write(&target, "orig2").unwrap();
    let swap_file_err = crate::ports::fs_log::open_no_symlink_with(&target, |path| {
        let opened = fs::File::open(path)?;
        fs::remove_file(path)?;
        fs::write(path, "replaced")?;
        Ok(opened)
    })
    .unwrap_err();
    assert!(matches!(swap_file_err, StoreError::Io(_)));
}

#[test]
fn standalone_sqlite_paths_symlinks_and_foreign_db_refusals() {
    use std::path::Path;

    let dir = tempdir().unwrap();

    // Empty and NUL paths refused as InvalidPath.
    for bad in [Path::new(""), Path::new("nul\0.db")] {
        assert_invalid_standalone_path(bad);
    }

    // Literal SQLite paths: `file:sym.db?mode=rwc` must NOT parse URI or touch `sym.db`.
    let outside = dir.path().join("outside.db");
    fs::write(&outside, b"untouched-sentinel").unwrap();
    let sym_db = dir.path().join("sym.db");
    symlink(&outside, &sym_db).unwrap();
    let uri_literal = dir.path().join("file:sym.db?mode=rwc");
    let proj_lit = SqliteProjectionStore::open(&uri_literal).unwrap();
    drop(proj_lit);
    assert_eq!(fs::read(&outside).unwrap(), b"untouched-sentinel");
    assert!(uri_literal.is_file());

    // Symlink target, symlink parent, and symlinked -wal/-shm sidecars refused.
    assert!(matches!(
        SqliteProjectionStore::open(&sym_db),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(&sym_db),
        Err(StoreError::SymlinkRefused(_))
    ));
    let real_parent = dir.path().join("real-parent");
    fs::create_dir(&real_parent).unwrap();
    let sym_parent = dir.path().join("sym-parent");
    symlink(&real_parent, &sym_parent).unwrap();
    assert!(matches!(
        SqliteProjectionStore::open(sym_parent.join("index.db")),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(sym_parent.join("queue.db")),
        Err(StoreError::SymlinkRefused(_))
    ));

    let wal_victim = dir.path().join("wal-victim.db");
    symlink(&outside, dir.path().join("wal-victim.db-wal")).unwrap();
    assert!(matches!(
        SqliteProjectionStore::open(&wal_victim),
        Err(StoreError::SymlinkRefused(_))
    ));
}

#[test]
fn standalone_sqlite_refuses_foreign_and_monolithic_databases() {
    let dir = tempdir().unwrap();
    // Foreign / monolithic databases refused by both SqliteProjectionStore and SqliteCoordinationStore.
    let monolith = dir.path().join("forge.db");
    let mut store = Store::open(&monolith).unwrap();
    store.create_run("m1", "feat", "self", &json!({})).unwrap();
    drop(store);
    assert!(matches!(
        SqliteProjectionStore::open(&monolith),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(&monolith),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));

    // Cross-store refusal: queue.db refused as projection index; index.db refused as coordination store.
    let idx_path = dir.path().join("index.db");
    let queue_path = dir.path().join("queue.db");
    drop(SqliteProjectionStore::open(&idx_path).unwrap());
    drop(SqliteCoordinationStore::open(&queue_path).unwrap());
    assert!(matches!(
        SqliteProjectionStore::open(&queue_path),
        Err(StoreError::ForeignDatabase { .. })
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(&idx_path),
        Err(StoreError::ForeignDatabase { .. })
    ));

    // Foreign user_version, triggers, partial schema, and wrong projection_schema refused.
    let uv_db = dir.path().join("uv.db");
    let conn = Connection::open(&uv_db).unwrap();
    conn.pragma_update(None, "user_version", 99u32).unwrap();
    drop(conn);
    assert!(matches!(
        SqliteProjectionStore::open(&uv_db),
        Err(StoreError::ForeignDatabase { .. })
    ));

    let trig_db = dir.path().join("trig.db");
    drop(SqliteProjectionStore::open(&trig_db).unwrap());
    let conn = Connection::open(&trig_db).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER bad_trig AFTER INSERT ON projected_runs BEGIN SELECT 1; END;",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        SqliteProjectionStore::open(&trig_db),
        Err(StoreError::ForeignDatabase { .. })
    ));

    let partial_db = dir.path().join("partial.db");
    drop(SqliteProjectionStore::open(&partial_db).unwrap());
    let conn = Connection::open(&partial_db).unwrap();
    conn.execute("DROP TABLE projected_events", []).unwrap();
    drop(conn);
    assert!(matches!(
        SqliteProjectionStore::open(&partial_db),
        Err(StoreError::ForeignDatabase { .. })
    ));

    let bad_ver_db = dir.path().join("bad-ver.db");
    drop(SqliteProjectionStore::open(&bad_ver_db).unwrap());
    let conn = Connection::open(&bad_ver_db).unwrap();
    conn.execute(
        "UPDATE projection_meta SET value = '999' WHERE key = 'projection_schema'",
        [],
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        SqliteProjectionStore::open(&bad_ver_db),
        Err(StoreError::ForeignDatabase { .. })
    ));
}

#[test]
fn sqlite_projection_integrity_repair_empty_run_id_and_locked_rebuild() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();

    // Refuse empty run_id in apply_envelopes.
    assert!(matches!(
        proj.apply_envelopes("", None, &[]),
        Err(StoreError::RunIdUnnameable(_))
    ));

    // Seed 2 events, then tamper projected_runs.head_hash at equal head -> DivergentEventLog.
    let e1 = started_event("p-int", "feat", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "p-int",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    let e3 = sealed_event(
        "p-int",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-26T10:00:10Z",
    );
    proj.apply_envelopes("p-int", None, &[e1.clone(), e2.clone()])
        .unwrap();

    let raw = Connection::open(&index_path).unwrap();
    raw.execute(
        "UPDATE projected_runs SET head_hash = 'tampered-head' WHERE run_id = 'p-int'",
        [],
    )
    .unwrap();
    assert!(matches!(
        proj.apply_envelopes("p-int", None, &[e1.clone(), e2.clone()]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "p-int"
    ));

    // Restore head_hash and drop projected_events rows to prove repair on equal head,
    // stale prefix, and advancing head.
    raw.execute(
        "UPDATE projected_runs SET head_hash = ?1 WHERE run_id = 'p-int'",
        rusqlite::params![&e2.event_hash],
    )
    .unwrap();
    // Deleting seq 1 while head_seq == 2 must NOT repair on a stale prefix (would admit B1 beside A2).
    raw.execute(
        "DELETE FROM projected_events WHERE run_id = 'p-int' AND seq = 1",
        [],
    )
    .unwrap();
    assert!(matches!(
        proj.apply_envelopes("p-int", None, std::slice::from_ref(&e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "p-int"
    ));
    let forked_e1 = started_event("p-int", "forked-b1", "2026-11-26T10:00:00Z");
    assert!(matches!(
        proj.apply_envelopes("p-int", None, std::slice::from_ref(&forked_e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "p-int"
    ));
    let count_events = || -> i64 {
        raw.query_row(
            "SELECT COUNT(*) FROM projected_events WHERE run_id = 'p-int'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    // Applying the full head-matching chain [e1, e2] repairs seq 1 cleanly because e2.event_hash == head_hash.
    proj.apply_envelopes("p-int", None, &[e1.clone(), e2.clone()])
        .unwrap();
    assert_eq!(count_events(), 2);

    raw.execute(
        "DELETE FROM projected_events WHERE run_id = 'p-int' AND seq = 2",
        [],
    )
    .unwrap();
    proj.apply_envelopes("p-int", None, &[e1.clone(), e2.clone()])
        .unwrap();
    assert_eq!(count_events(), 2);

    raw.execute("DELETE FROM projected_events WHERE run_id = 'p-int'", [])
        .unwrap();
    proj.apply_envelopes("p-int", None, &[e1.clone(), e2.clone(), e3.clone()])
        .unwrap();
    assert_eq!(count_events(), 3);
    drop(raw);
}

#[test]
fn sqlite_projection_rebuild_holds_immediate_write_lock_during_log_read() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    let (e1, e2, e3) = {
        let e1 = started_event("p-lock", "feat", "2026-11-26T10:00:00Z");
        let e2 = sealed_event(
            "p-lock",
            2,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            &e1.event_hash,
            "2026-11-26T10:00:05Z",
        );
        let e3 = sealed_event(
            "p-lock",
            3,
            EventType::RunCompleted,
            json!({}),
            &e2.event_hash,
            "2026-11-26T10:00:10Z",
        );
        (e1, e2, e3)
    };
    // Prove rebuild_from holds BEGIN IMMEDIATE before reading from `log`.
    let fs_log = FsEventLog::open(dir.path().join("runs")).unwrap();
    fs_log
        .create_run("p-lock", &sample_manifest("feat"))
        .unwrap();
    fs_log
        .append_envelopes("p-lock", (0, ZERO_HASH), &[e1, e2, e3])
        .unwrap();
    let peer = Connection::open(&index_path).unwrap();
    let observed_locked = std::cell::Cell::new(false);
    let intercepted = InterceptingLog {
        inner: fs_log,
        on_head: None,
        on_list_run_ids: Some(Box::new(|| {
            let err = peer.execute_batch("BEGIN IMMEDIATE").unwrap_err();
            assert!(matches!(
                err.sqlite_error_code(),
                Some(rusqlite::ErrorCode::DatabaseBusy)
            ));
            observed_locked.set(true);
        })),
        on_read_stream: None,
        override_run_ids: None,
    };
    assert_eq!(proj.rebuild_from(&intercepted).unwrap(), 3);
    assert!(observed_locked.get());
}

#[test]
fn sqlite_coordination_fail_closed_waits_and_divergent_cooldowns() {
    let dir = tempdir().unwrap();
    let coord = SqliteCoordinationStore::open(dir.path().join("queue.db")).unwrap();

    // Malformed `waits` payload must propagate StoreError::ProviderWait (never silently discarded).
    let bad_wait_event = sealed_event(
        "c-run",
        1,
        EventType::EffectFailed,
        json!({
            "effect_id": "eff-bad",
            "attempt_id": "att-bad",
            "waits": [{"provider": "", "route": "r", "until": "not-a-timestamp", "words": "x", "seconds": 10, "rounds": 1}]
        }),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    assert!(matches!(
        coord.apply_cooldowns("c-run", std::slice::from_ref(&bad_wait_event)),
        Err(StoreError::ProviderWait(_))
    ));

    // Also propagates in sync_cooldowns_from.
    let fs_log = FsEventLog::open(dir.path().join("runs")).unwrap();
    fs_log.create_run("c-run", &sample_manifest("c")).unwrap();
    fs_log
        .append_envelopes(
            "c-run",
            (0, ZERO_HASH),
            std::slice::from_ref(&bad_wait_event),
        )
        .unwrap();
    assert!(matches!(
        coord.sync_cooldowns_from(&fs_log),
        Err(StoreError::ProviderWait(_))
    ));

    // Divergent cooldown on conflict against existing row AND within same incoming slice.
    let good_a = sealed_event(
        "c-div",
        2,
        EventType::EffectFailed,
        waits_effect_payload("2026-11-26T10:15:00Z", "2026-11-26T10:05:00Z"),
        ZERO_HASH,
        "2026-11-26T10:01:00Z",
    );
    let good_b = sealed_event(
        "c-div",
        2,
        EventType::EffectFailed,
        waits_effect_payload("2026-11-26T10:30:00Z", "2026-11-26T10:05:00Z"),
        ZERO_HASH,
        "2026-11-26T10:01:00Z",
    );
    // Divergent duplicate-seq within the same incoming slice -> DivergentEventLog.
    assert!(matches!(
        coord.apply_cooldowns("c-div", &[good_a.clone(), good_b.clone()]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "c-div"
    ));

    // Commit identical duplicate-seq in one batch (inserts once, idempotent on second),
    // verify identical replay is Ok(0), and divergent replay is DivergentEventLog.
    assert_eq!(
        coord
            .apply_cooldowns("c-div", &[good_a.clone(), good_a.clone()])
            .unwrap(),
        2
    );
    assert_eq!(
        coord
            .apply_cooldowns("c-div", std::slice::from_ref(&good_a))
            .unwrap(),
        0
    );
    assert!(matches!(
        coord.apply_cooldowns("c-div", &[good_b]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "c-div"
    ));
}
