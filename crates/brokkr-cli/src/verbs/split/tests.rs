use std::fs;
use std::os::unix::fs::symlink;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType, EVENT_SCHEMA_VERSION};
use brokkr_store::ports::{EventLog, FsEventLog, ProjectionStore, SqliteProjectionStore};
use brokkr_store::Store;
use serde_json::json;
use tempfile::tempdir;

use super::*;
use crate::cli_args::JournalArgs;
use crate::{run, Cli, Cmd};

fn sealed(
    run_id: &str,
    seq: u64,
    event_type: EventType,
    payload: serde_json::Value,
    prev: &str,
    ts: &str,
) -> EventEnvelope {
    EventEnvelope {
        run_id: run_id.to_string(),
        seq,
        event_id: format!("{run_id}-evt-{seq}"),
        event_schema_version: EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id: None,
        correlation_id: run_id.to_string(),
        attempt_id: None,
        recorded_at: ts.to_string(),
        previous_hash: prev.to_string(),
        event_hash: String::new(),
    }
    .sealed()
}

fn seed_monolith(db: &Path) -> Store {
    let mut store = Store::open(db).unwrap();
    let m1 = json!({"engine": "0.15.0", "feature": "alpha", "bundle_name": "self"});
    store.create_run("run-alpha", "alpha", "self", &m1).unwrap();
    store
        .append_next(
            "run-alpha",
            EventType::RunStarted,
            json!({"feature": "alpha", "manifest": m1}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "run-alpha",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next("run-alpha", EventType::RunCompleted, json!({}), None, None)
        .unwrap();

    // Include an empty run with manifest-only metadata too.
    let m2 = json!({"engine": "0.15.0", "feature": "beta", "bundle_name": "self"});
    store.create_run("run-beta", "beta", "self", &m2).unwrap();
    store
}

#[test]
fn journal_migrate_and_reindex_end_to_end_preserve_ndjson_parity() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let store = seed_monolith(&db);
    let expected_alpha_ndjson = store.export_ndjson("run-alpha").unwrap();
    let expected_beta_ndjson = store.export_ndjson("run-beta").unwrap();
    drop(store);
    let before_bytes = fs::read(&db).unwrap();

    let split_dir = dir.path().join("split");
    let exit = run(Cli {
        command: Cmd::Migrate(MigrateArgs {
            to: split_dir.clone(),
            journal: JournalArgs {
                realms: None,
                db: Some(db.clone()),
            },
        }),
    })
    .unwrap();
    assert_eq!(exit, Exit::Completed.into());

    // Source database bytes untouched (opened read-only).
    assert_eq!(fs::read(&db).unwrap(), before_bytes);

    // Split layout holds exact runs, canonical NDJSON parity, and rebuilt index.db.
    let runs_dir = split_dir.join("runs");
    let index_path = split_dir.join("index.db");
    let fs_log = FsEventLog::open(&runs_dir).unwrap();
    assert_eq!(
        fs_log.list_run_ids().unwrap(),
        vec!["run-alpha".to_string(), "run-beta".to_string()]
    );
    assert_eq!(
        fs::read_to_string(runs_dir.join("run-alpha").join("events.ndjson")).unwrap(),
        expected_alpha_ndjson
    );
    assert_eq!(
        fs::read_to_string(runs_dir.join("run-beta").join("events.ndjson")).unwrap(),
        expected_beta_ndjson
    );
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    assert_eq!(proj.list_runs().unwrap().len(), 2);
    drop(proj);

    // Delete disposable index.db and rebuild it cleanly via `brokkr journal reindex`.
    fs::remove_file(&index_path).unwrap();
    assert!(!index_path.exists());
    let reindex_exit = run(Cli {
        command: Cmd::Reindex(ReindexArgs {
            runs: Some(runs_dir.clone()),
            index: Some(index_path.clone()),
            journal: JournalArgs {
                realms: None,
                db: None,
            },
        }),
    })
    .unwrap();
    assert_eq!(reindex_exit, Exit::Completed.into());
    let rebuilt = SqliteProjectionStore::open(&index_path).unwrap();
    let listed = rebuilt.list_runs().unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed
        .iter()
        .any(|(id, feat, _)| id == "run-alpha" && feat == "alpha"));
    assert!(listed
        .iter()
        .any(|(id, feat, _)| id == "run-beta" && feat == "beta"));
}

#[test]
fn journal_reindex_uses_workspace_defaults_and_refuses_bad_paths() {
    let dir = tempdir().unwrap();
    let state_dir = dir.path().join(".forge");
    let db = state_dir.join("forge.db");
    let runs_dir = state_dir.join("runs");
    let fs_log = FsEventLog::open(&runs_dir).unwrap();
    let manifest = json!({"engine": "0.15.0", "feature": "default-path"});
    fs_log.create_run("r1", &manifest).unwrap();
    let e1 = sealed(
        "r1",
        1,
        EventType::RunStarted,
        json!({"feature": "default-path", "manifest": manifest}),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    fs_log
        .append_envelopes("r1", (0, ZERO_HASH), &[e1])
        .unwrap();

    // Default paths resolve beside --db's parent directory.
    let exit = reindex(
        dir.path(),
        ReindexArgs {
            runs: None,
            index: None,
            journal: JournalArgs {
                realms: None,
                db: Some(db),
            },
        },
    )
    .unwrap();
    assert_eq!(exit, Exit::Completed.into());
    let index = SqliteProjectionStore::open(state_dir.join("index.db")).unwrap();
    assert_eq!(index.list_runs().unwrap().len(), 1);

    // Refuses absent runs dir, regular file runs path, symlink runs dir, symlink index.
    let err = ensure_existing_runs_dir(&dir.path().join("missing")).unwrap_err();
    assert!(err.to_string().contains("does not exist"));

    let plain_file = dir.path().join("plain");
    fs::write(&plain_file, "x").unwrap();
    let err = ensure_existing_runs_dir(&plain_file).unwrap_err();
    assert!(err.to_string().contains("is not a directory"));

    let sym_runs = dir.path().join("sym-runs");
    symlink(&runs_dir, &sym_runs).unwrap();
    let err = ensure_existing_runs_dir(&sym_runs).unwrap_err();
    assert!(err.to_string().contains("refusing symlink"));

    let sym_idx = dir.path().join("sym-index.db");
    symlink(&plain_file, &sym_idx).unwrap();
    let err = refuse_symlink(&sym_idx).unwrap_err();
    assert!(err.to_string().contains("refusing symlink"));
}

#[test]
fn journal_migrate_fails_atomically_and_refuses_bad_targets() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("forge.db");
    drop(seed_monolith(&db));

    // Corrupt a run's payload in SQLite directly so migration fails mid-way.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = '{\"corrupt\": true}' WHERE run_id = 'run-alpha' AND seq = 2;",
    )
    .unwrap();
    drop(conn);

    let dest = dir.path().join("should-not-exist");
    let err = migrate(
        dir.path(),
        MigrateArgs {
            to: dest.clone(),
            journal: JournalArgs {
                realms: None,
                db: Some(db.clone()),
            },
        },
    )
    .unwrap_err();
    assert!(!err.to_string().is_empty());
    // Atomic failure: destination does not exist and no staging temp dir remains!
    assert!(!dest.exists());
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".brokkr-migrate-")
        })
        .collect();
    assert!(leftovers.is_empty());

    // Target validation: refuses symlink, non-dir, non-empty dir; accepts existing empty dir.
    let occupied = dir.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("file"), "data").unwrap();
    assert!(prepare_migration_target(&occupied)
        .unwrap_err()
        .to_string()
        .contains("is not empty"));

    let plain = dir.path().join("file-target");
    fs::write(&plain, "x").unwrap();
    assert!(prepare_migration_target(&plain)
        .unwrap_err()
        .to_string()
        .contains("is not a directory"));

    let sym = dir.path().join("sym-target");
    symlink(&occupied, &sym).unwrap();
    assert!(prepare_migration_target(&sym)
        .unwrap_err()
        .to_string()
        .contains("refusing symlink"));

    let empty_dir = dir.path().join("empty-target");
    fs::create_dir(&empty_dir).unwrap();
    assert!(prepare_migration_target(&empty_dir).is_ok());
}

#[test]
fn journal_migrate_verify_run_parity_refuses_mismatches() {
    let dir = tempdir().unwrap();
    // Unit-test verify_run_parity error branches directly.
    let parity_runs = dir.path().join("parity-runs");
    let fs_log = FsEventLog::open(&parity_runs).unwrap();
    let m = json!({"engine": "0.15.0", "feature": "p"});
    fs_log.create_run("p1", &m).unwrap();
    let e1 = sealed(
        "p1",
        1,
        EventType::RunStarted,
        json!({"feature": "p", "manifest": m}),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    fs_log
        .append_envelopes("p1", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();
    let good_ndjson = fs::read_to_string(parity_runs.join("p1").join("events.ndjson")).unwrap();
    assert!(verify_run_parity(
        &fs_log,
        &parity_runs,
        "p1",
        &json!({}),
        std::slice::from_ref(&e1),
        &good_ndjson
    )
    .unwrap_err()
    .to_string()
    .contains("manifest parity mismatch"));
    assert!(
        verify_run_parity(&fs_log, &parity_runs, "p1", &m, &[], &good_ndjson)
            .unwrap_err()
            .to_string()
            .contains("event stream parity mismatch")
    );
    assert!(
        verify_run_parity(&fs_log, &parity_runs, "p1", &m, &[e1], "wrong\n")
            .unwrap_err()
            .to_string()
            .contains("canonical NDJSON parity mismatch")
    );
}
