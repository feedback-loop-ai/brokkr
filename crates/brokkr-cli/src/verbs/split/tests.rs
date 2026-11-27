use std::fs;
use std::os::unix::fs::symlink;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType, EVENT_SCHEMA_VERSION};
use brokkr_store::ports::{EventLog, FsEventLog, ProjectionStore, SqliteProjectionStore};
use brokkr_store::{Store, StoreError};
use serde_json::json;
use tempfile::tempdir;

use super::*;
use crate::cli_args::JournalArgs;
use crate::{run, Cli, Cmd, GroupedCmd};

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

    // Include an unstarted, manifest-bare run (manifest omits feature and created_at)
    // to prove migration preserves runs.feature and runs.created_at end-to-end.
    store
        .create_run("run-beta", "beta-bare", "self", &json!({}))
        .unwrap();
    store
}

#[test]
fn journal_migrate_and_reindex_end_to_end_preserve_ndjson_and_listing_parity() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let store = seed_monolith(&db);
    let expected_runs = store.list_runs().unwrap();
    let expected_alpha_ndjson = store.export_ndjson("run-alpha").unwrap();
    let expected_beta_ndjson = store.export_ndjson("run-beta").unwrap();
    drop(store);
    let before_bytes = fs::read(&db).unwrap();

    let split_dir = dir.path().join("split");
    let exit = run(Cli {
        command: Cmd::Grouped(GroupedCmd::Split(SplitCmd::Migrate(MigrateArgs {
            to: split_dir.clone(),
            journal: JournalArgs {
                realms: None,
                db: Some(db.clone()),
            },
        }))),
    })
    .unwrap();
    assert_eq!(exit, Exit::Completed.into());

    // Source database bytes untouched (opened read-only).
    assert_eq!(fs::read(&db).unwrap(), before_bytes);

    // Split layout holds exact runs, canonical NDJSON parity, and rebuilt index.db
    // whose list_runs() matches source Store::list_runs() including bare/unstarted runs.
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
    assert_eq!(proj.list_runs().unwrap(), expected_runs);
    drop(proj);

    // Corrupt disposable index.db with garbage bytes and stale sidecars;
    // `brokkr journal reindex` atomically replaces it cleanly.
    fs::write(&index_path, b"not-a-sqlite-database-corrupt").unwrap();
    fs::write(sidecar_path(&index_path, "-wal"), b"stale-wal").unwrap();
    fs::write(sidecar_path(&index_path, "-shm"), b"stale-shm").unwrap();
    let reindex_exit = run(Cli {
        command: Cmd::Grouped(GroupedCmd::Split(SplitCmd::Reindex(ReindexArgs {
            runs: Some(runs_dir.clone()),
            index: Some(index_path.clone()),
            journal: JournalArgs {
                realms: None,
                db: None,
            },
        }))),
    })
    .unwrap();
    assert_eq!(reindex_exit, Exit::Completed.into());
    let rebuilt = SqliteProjectionStore::open(&index_path).unwrap();
    assert_eq!(rebuilt.list_runs().unwrap(), expected_runs);

    // Refuses rebuilding over a monolithic forge.db without modifying it.
    let foreign_err = reindex_paths(&runs_dir, &db).unwrap_err();
    assert!(matches!(
        foreign_err,
        SplitError::Store(StoreError::ForeignDatabase { .. })
    ));
    assert_eq!(fs::read(&db).unwrap(), before_bytes);
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

    // Refuses absent runs dir, regular file runs path, symlink runs dir, symlink index,
    // directory index, empty index path, and symlinked sidecars.
    assert!(matches!(
        ensure_existing_runs_dir(&dir.path().join("missing")),
        Err(SplitError::RunsDirMissing(_))
    ));

    let plain_file = dir.path().join("plain");
    fs::write(&plain_file, "x").unwrap();
    assert!(matches!(
        ensure_existing_runs_dir(&plain_file),
        Err(SplitError::RunsPathNotDirectory(_))
    ));

    let sym_runs = dir.path().join("sym-runs");
    symlink(&runs_dir, &sym_runs).unwrap();
    assert!(matches!(
        ensure_existing_runs_dir(&sym_runs),
        Err(SplitError::SymlinkRefused(_))
    ));

    let sym_idx = dir.path().join("sym-index.db");
    symlink(&plain_file, &sym_idx).unwrap();
    assert!(matches!(
        prepare_index_target(&sym_idx),
        Err(SplitError::SymlinkRefused(_))
    ));

    let dir_idx = dir.path().join("dir-index");
    fs::create_dir(&dir_idx).unwrap();
    assert!(matches!(
        prepare_index_target(&dir_idx),
        Err(SplitError::IndexPathIsDirectory(_))
    ));

    assert!(matches!(
        prepare_index_target(Path::new("")),
        Err(SplitError::InvalidPath(_))
    ));

    let sidecar_idx = dir.path().join("sidecar.db");
    symlink(&plain_file, sidecar_path(&sidecar_idx, "-wal")).unwrap();
    assert!(matches!(
        prepare_index_target(&sidecar_idx),
        Err(SplitError::SymlinkRefused(_))
    ));

    // Non-NotFound I/O error (ENOTDIR) propagates cleanly from ensure_existing_runs_dir & refuse_symlink.
    assert!(matches!(
        ensure_existing_runs_dir(&plain_file.join("child")),
        Err(SplitError::Io(_))
    ));
    assert!(matches!(
        refuse_symlink(&plain_file.join("child")),
        Err(SplitError::Io(_))
    ));
}

#[test]
fn journal_reindex_valid_rebuild_and_staged_cleanup_on_failure() {
    let dir = tempdir().unwrap();
    let state_dir = dir.path().join(".forge");
    let runs_dir = state_dir.join("runs");
    let fs_log = FsEventLog::open(&runs_dir).unwrap();
    let manifest = json!({"engine": "0.15.0", "feature": "re-reindex"});
    fs_log.create_run("r1", &manifest).unwrap();
    let e1 = sealed(
        "r1",
        1,
        EventType::RunStarted,
        json!({"feature": "re-reindex", "manifest": manifest}),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    fs_log
        .append_envelopes("r1", (0, ZERO_HASH), &[e1])
        .unwrap();
    let valid_idx = state_dir.join("index.db");
    assert_eq!(reindex_paths(&runs_dir, &valid_idx).unwrap(), (1, 1));
    // Reindexing over an ALREADY valid index.db succeeds cleanly.
    assert_eq!(reindex_paths(&runs_dir, &valid_idx).unwrap(), (1, 1));

    // If rebuild_from fails mid-way, StagedDb cleans up its temp file and preserves existing index.db.
    fs::write(
        runs_dir.join("r1").join("events.ndjson"),
        "{corrupt
",
    )
    .unwrap();
    assert!(matches!(
        reindex_paths(&runs_dir, &valid_idx),
        Err(SplitError::Store(StoreError::Damaged { .. }))
    ));
    let staged_leftovers: Vec<_> = fs::read_dir(&state_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".brokkr-index-rebuild-")
        })
        .collect();
    assert!(staged_leftovers.is_empty());
}

#[test]
fn journal_migrate_fails_atomically_after_staging_valid_run_and_normalizes_targets() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = seed_monolith(&db);
    // Give the SECOND run events so run-alpha (FIRST) stages cleanly and run-beta (SECOND) fails!
    store
        .append_next(
            "run-beta",
            EventType::RunStarted,
            json!({"feature": "beta-bare", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "run-beta",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            None,
        )
        .unwrap();
    drop(store);

    // Corrupt the SECOND run's payload so migration stages run-alpha first, then fails on run-beta.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = '{\"corrupt\": true}' WHERE run_id = 'run-beta' AND seq = 2;",
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
    assert!(matches!(
        err.downcast_ref::<SplitError>(),
        Some(SplitError::Store(StoreError::Json(_)))
    ));
    // Atomic rollback: destination does not exist and no staging temp dir remains!
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
}

#[test]
fn journal_migrate_validates_and_normalizes_targets() {
    let dir = tempdir().unwrap();
    // Target validation: refuses empty path, root, symlink, non-dir, non-empty dir.
    assert!(matches!(
        prepare_migration_target(Path::new("")),
        Err(SplitError::InvalidPath(_))
    ));
    assert!(matches!(
        prepare_migration_target(Path::new("/")),
        Err(SplitError::TargetHasNoParent(_))
    ));

    let occupied = dir.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("file"), "data").unwrap();
    assert!(matches!(
        prepare_migration_target(&occupied),
        Err(SplitError::TargetNotEmpty(_))
    ));

    let plain = dir.path().join("file-target");
    fs::write(&plain, "x").unwrap();
    assert!(matches!(
        prepare_migration_target(&plain),
        Err(SplitError::TargetNotDirectory(_))
    ));

    let sym = dir.path().join("sym-target");
    symlink(&occupied, &sym).unwrap();
    assert!(matches!(
        prepare_migration_target(&sym),
        Err(SplitError::SymlinkRefused(_))
    ));

    // Path normalization: `<empty>/sub/..` normalizes to `<empty>` without creating `sub`!
    let clean_db = dir.path().join("clean.db");
    let clean_store = seed_monolith(&clean_db);
    let empty_dir = dir.path().join("empty-target");
    fs::create_dir(&empty_dir).unwrap();
    assert_eq!(
        lexically_normalize(Path::new("/a/./b/c/..")),
        PathBuf::from("/a/b")
    );
    assert_eq!(preserve_run_metadata(&json!(null), "f", "t"), json!(null));
    let cur_target = empty_dir.join(".");
    let prepared_cur = prepare_migration_target(&cur_target).unwrap();
    assert_eq!(prepared_cur.normalized, empty_dir);
    assert!(prepared_cur.existed_empty);

    let dotted_target = empty_dir.join("sub").join("..");
    let prepared = prepare_migration_target(&dotted_target).unwrap();
    assert_eq!(prepared.normalized, empty_dir);
    assert!(prepared.existed_empty);
    assert!(!empty_dir.join("sub").exists());

    // Publishing into existing empty directory via `<empty>/sub/..` succeeds cleanly.
    let (runs, events) = migrate_store(&clean_store, &dotted_target).unwrap();
    assert_eq!((runs, events), (2, 3));
    assert!(!empty_dir.join("sub").exists());
    assert!(empty_dir.join("runs").is_dir());
    assert!(empty_dir.join("index.db").is_file());
    let migrated_idx = SqliteProjectionStore::open(empty_dir.join("index.db")).unwrap();
    assert_eq!(
        migrated_idx.list_runs().unwrap(),
        clean_store.list_runs().unwrap()
    );
}

#[test]
fn journal_migrate_verify_run_parity_and_display_text() {
    let dir = tempdir().unwrap();
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

    assert!(matches!(
        verify_run_parity(
            &fs_log,
            &parity_runs,
            "p1",
            (&json!({}), std::slice::from_ref(&e1), good_ndjson.as_str()),
        ),
        Err(SplitError::ParityMismatch { kind: ParityKind::Manifest, ref run_id }) if run_id == "p1"
    ));
    assert!(matches!(
        verify_run_parity(&fs_log, &parity_runs, "p1", (&m, &[], good_ndjson.as_str())),
        Err(SplitError::ParityMismatch { kind: ParityKind::EventStream, ref run_id }) if run_id == "p1"
    ));
    assert!(matches!(
        verify_run_parity(&fs_log, &parity_runs, "p1", (&m, &[e1], "wrong\n")),
        Err(SplitError::ParityMismatch { kind: ParityKind::CanonicalNdjson, ref run_id }) if run_id == "p1"
    ));
    assert!(matches!(
        verify_projection_counts((2, 3, 1, 3)),
        Err(SplitError::ProjectionMismatch {
            expected_runs: 2,
            expected_events: 3,
            indexed_runs: 1,
            indexed_events: 3,
        })
    ));
}

#[test]
fn split_error_display_and_source_wording() {
    // Pin SplitError Display wording once (decision 0071 ruling 8).
    assert_eq!(
        SplitError::RunsDirMissing(PathBuf::from("x")).to_string(),
        "runs directory 'x' does not exist"
    );
    assert_eq!(
        SplitError::SymlinkRefused(PathBuf::from("x")).to_string(),
        "refusing symlink at 'x'"
    );
    assert_eq!(
        SplitError::RunsPathNotDirectory(PathBuf::from("x")).to_string(),
        "runs path 'x' is not a directory"
    );
    assert_eq!(
        SplitError::IndexPathIsDirectory(PathBuf::from("x")).to_string(),
        "index path 'x' is a directory"
    );
    assert_eq!(
        SplitError::InvalidPath(PathBuf::from("")).to_string(),
        "invalid path ''"
    );
    assert_eq!(
        SplitError::TargetNotDirectory(PathBuf::from("x")).to_string(),
        "migration destination 'x' is not a directory"
    );
    assert_eq!(
        SplitError::TargetNotEmpty(PathBuf::from("x")).to_string(),
        "migration destination 'x' is not empty"
    );
    assert_eq!(
        SplitError::TargetHasNoParent(PathBuf::from("/")).to_string(),
        "migration destination '/' has no parent directory"
    );
    assert_eq!(
        SplitError::ParityMismatch {
            kind: ParityKind::Manifest,
            run_id: "r".to_string()
        }
        .to_string(),
        "manifest parity mismatch for run 'r'"
    );
    assert_eq!(
        SplitError::ParityMismatch {
            kind: ParityKind::EventStream,
            run_id: "r".to_string()
        }
        .to_string(),
        "event stream parity mismatch for run 'r'"
    );
    assert_eq!(
        SplitError::ParityMismatch {
            kind: ParityKind::CanonicalNdjson,
            run_id: "r".to_string()
        }
        .to_string(),
        "canonical NDJSON parity mismatch for run 'r'"
    );
    assert_eq!(
        SplitError::ProjectionMismatch {
            expected_runs: 2,
            expected_events: 5,
            indexed_runs: 1,
            indexed_events: 4,
        }
        .to_string(),
        "migration projection mismatch: expected 2 runs / 5 events, indexed 1 / 4"
    );

    use std::error::Error as _;
    let store_err = SplitError::from(StoreError::RunNotFound("r".to_string()));
    assert_eq!(store_err.to_string(), "run 'r' not found");
    assert!(store_err.source().is_some());
    let io_err = SplitError::from(std::io::Error::other("boom"));
    assert_eq!(io_err.to_string(), "boom");
    assert!(io_err.source().is_some());
    let json_err = SplitError::from(serde_json::from_str::<Value>("{").unwrap_err());
    assert!(json_err.source().is_some());
    assert!(!json_err.to_string().is_empty());
    assert!(SplitError::InvalidPath(PathBuf::new()).source().is_none());
}
