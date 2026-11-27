use std::error::Error;
use std::fs;
use std::os::unix::fs::symlink;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType, EVENT_SCHEMA_VERSION};
use brokkr_store::ports::{
    normalize_storage_path, reject_symlink, sqlite_sidecar_path, EventLog, FsEventLog,
    ProjectionStore, RunMetadata, SqliteProjectionStore,
};
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
    // Migrated manifests MUST remain byte-identical to source.manifest(run_id) (unmodified!).
    assert_eq!(
        fs::read(runs_dir.join("run-beta").join("manifest.json")).unwrap(),
        b"{}"
    );
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    assert_eq!(proj.list_runs().unwrap(), expected_runs);
    drop(proj);

    // Corrupt disposable index.db with garbage bytes and stale sidecars;
    // `brokkr journal reindex` atomically replaces it cleanly.
    fs::write(&index_path, b"not-a-sqlite-database-corrupt").unwrap();
    fs::write(sqlite_sidecar_path(&index_path, "-wal"), b"stale-wal").unwrap();
    fs::write(sqlite_sidecar_path(&index_path, "-shm"), b"stale-shm").unwrap();
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
    symlink(&plain_file, sqlite_sidecar_path(&sidecar_idx, "-wal")).unwrap();
    assert!(matches!(
        prepare_index_target(&sidecar_idx),
        Err(SplitError::SymlinkRefused(_))
    ));

    // Trailing slash on symlinked runs dir or dotted index path is normalized BEFORE checks.
    let sym_runs_trailing = PathBuf::from(format!("{}/", sym_runs.display()));
    assert!(matches!(
        ensure_existing_runs_dir(&sym_runs_trailing),
        Err(SplitError::SymlinkRefused(_))
    ));
    let dotted_sidecar_idx = dir.path().join("ghost").join("..").join("sidecar.db");
    assert!(matches!(
        prepare_index_target(&dotted_sidecar_idx),
        Err(SplitError::SymlinkRefused(_))
    ));
    assert!(!dir.path().join("ghost").exists());

    // Non-NotFound I/O error (ENOTDIR) propagates cleanly from ensure_existing_runs_dir & reject_symlink.
    assert!(matches!(
        ensure_existing_runs_dir(&plain_file.join("child")),
        Err(SplitError::Io(_))
    ));
    assert!(matches!(
        reject_symlink(&plain_file.join("child")),
        Err(StoreError::Io(_))
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

    // If removing stale -wal/-shm sidecar fails with non-NotFound (e.g. a directory),
    // reindex returns a typed SplitError::SidecarRemovalFailed cleanly.
    let bad_wal_idx = state_dir.join("bad-wal-index.db");
    let bad_wal = sqlite_sidecar_path(&bad_wal_idx, "-wal");
    fs::create_dir(&bad_wal).unwrap();
    assert!(matches!(
        reindex_paths(&runs_dir, &bad_wal_idx),
        Err(SplitError::SidecarRemovalFailed { ref path, .. }) if path == &bad_wal
    ));
    assert!(!bad_wal_idx.exists());
    fs::remove_dir(&bad_wal).unwrap();

    // If rebuild_from fails mid-way, StagedDb cleans up its temp file, error attributes damage
    // to events.ndjson (never temp staging DB), and existing valid_idx stays intact & readable!
    let events_file = runs_dir.join("r1").join("events.ndjson");
    fs::write(&events_file, "{corrupt\n").unwrap();
    assert!(matches!(
        reindex_paths(&runs_dir, &valid_idx),
        Err(SplitError::Store(StoreError::Damaged { ref journal, .. })) if journal == &events_file
    ));
    let preserved = SqliteProjectionStore::open(&valid_idx).unwrap();
    assert_eq!(
        preserved.list_runs().unwrap(),
        vec![(
            "r1".to_string(),
            "re-reindex".to_string(),
            "2026-11-26T10:00:00Z".to_string()
        )]
    );
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

    // Pin deterministic created_at order so run-alpha is GUARANTEED to stage first and run-beta fails second.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "UPDATE runs SET created_at = '2026-11-26T09:00:00Z' WHERE run_id = 'run-alpha';
         UPDATE runs SET created_at = '2026-11-26T10:00:00Z' WHERE run_id = 'run-beta';
         DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = '{\"corrupt\": true}' WHERE run_id = 'run-beta' AND seq = 2;",
    )
    .unwrap();
    drop(conn);
    let check_order = Store::open_read_only(&db).unwrap().list_runs().unwrap();
    assert_eq!(check_order[0].0, "run-alpha");
    assert_eq!(check_order[1].0, "run-beta");

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
        normalize_storage_path(Path::new("/a/./b/c/..")).unwrap(),
        PathBuf::from("/a/b")
    );
    let cur_target = empty_dir.join(".");
    let prepared_cur = prepare_migration_target(&cur_target).unwrap();
    assert_eq!(prepared_cur.normalized, empty_dir);
    assert!(!prepared_cur.in_place);

    let dotted_target = empty_dir.join("sub").join("..");
    let prepared = prepare_migration_target(&dotted_target).unwrap();
    assert_eq!(prepared.normalized, empty_dir);
    assert!(!prepared.in_place);
    assert!(!empty_dir.join("sub").exists());

    // Publishing into existing empty directory via `<empty>/sub/..` succeeds cleanly and
    // reports normalized paths in MigrateReport (never `<empty>/sub/../runs`).
    let report = migrate_report(&clean_store, &clean_db, &dotted_target).unwrap();
    assert_eq!((report.runs, report.events), (2, 3));
    assert_eq!(report.to, empty_dir.display().to_string());
    assert_eq!(
        report.runs_dir,
        empty_dir.join("runs").display().to_string()
    );
    assert_eq!(
        report.index,
        empty_dir.join("index.db").display().to_string()
    );
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
fn journal_migrate_preserves_tied_timestamp_order_and_immutable_manifests() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("tied.db");
    let mut store = Store::open(&db).unwrap();
    let m_z = json!({"engine": "0.15.0"});
    let m_a = json!({});
    store.create_run("run-z", "feat-z", "self", &m_z).unwrap();
    store.create_run("run-a", "feat-a", "self", &m_a).unwrap();
    drop(store);

    // Give both runs the exact same created_at so Store::list_runs() orders run-z before run-a.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute("UPDATE runs SET created_at = '2026-11-26T12:00:00Z'", [])
        .unwrap();
    drop(conn);

    let ro = Store::open_read_only(&db).unwrap();
    let expected = ro.list_runs().unwrap();
    assert_eq!(expected[0].0, "run-z");
    assert_eq!(expected[1].0, "run-a");

    let dest = dir.path().join("split-tied");
    let report = migrate_report(&ro, &db, &dest).unwrap();
    assert_eq!(report.runs, 2);
    assert_eq!(
        fs::read(dest.join("runs").join("run-z").join("manifest.json")).unwrap(),
        serde_json::to_vec(&m_z).unwrap()
    );
    assert_eq!(
        fs::read(dest.join("runs").join("run-a").join("manifest.json")).unwrap(),
        serde_json::to_vec(&m_a).unwrap()
    );
    let idx = SqliteProjectionStore::open(dest.join("index.db")).unwrap();
    assert_eq!(idx.list_runs().unwrap(), expected);
    drop(idx);

    // Reindexing later from runs/ preserves the exact same order and metadata!
    assert_eq!(
        reindex_paths(&dest.join("runs"), &dest.join("index.db")).unwrap(),
        (2, 0)
    );
    let rebuilt = SqliteProjectionStore::open(dest.join("index.db")).unwrap();
    assert_eq!(rebuilt.list_runs().unwrap(), expected);
}

#[test]
fn journal_migrate_verify_run_parity_and_display_text() {
    let dir = tempdir().unwrap();
    let parity_runs = dir.path().join("parity-runs");
    let fs_log = FsEventLog::open(&parity_runs).unwrap();
    let m = json!({"engine": "0.15.0", "feature": "p"});
    let meta = RunMetadata {
        feature: "p".to_string(),
        created_at: "2026-11-26T10:00:00Z".to_string(),
        list_order: Some(0),
    };
    fs_log.create_run_with_metadata("p1", &m, &meta).unwrap();
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
            (&json!({}), &meta, std::slice::from_ref(&e1), good_ndjson.as_str()),
        ),
        Err(SplitError::ParityMismatch { kind: ParityKind::Manifest, ref run_id }) if run_id == "p1"
    ));
    let wrong_meta = RunMetadata {
        feature: "other".to_string(),
        created_at: "2026-11-26T10:00:00Z".to_string(),
        list_order: Some(0),
    };
    assert!(matches!(
        verify_run_parity(
            &fs_log,
            &parity_runs,
            "p1",
            (&m, &wrong_meta, std::slice::from_ref(&e1), good_ndjson.as_str()),
        ),
        Err(SplitError::ParityMismatch { kind: ParityKind::RunMetadata, ref run_id }) if run_id == "p1"
    ));
    assert!(matches!(
        verify_run_parity(&fs_log, &parity_runs, "p1", (&m, &meta, &[], good_ndjson.as_str())),
        Err(SplitError::ParityMismatch { kind: ParityKind::EventStream, ref run_id }) if run_id == "p1"
    ));
    assert!(matches!(
        verify_run_parity(&fs_log, &parity_runs, "p1", (&m, &meta, &[e1], "wrong\n")),
        Err(SplitError::ParityMismatch { kind: ParityKind::CanonicalNdjson, ref run_id }) if run_id == "p1"
    ));
    assert!(matches!(
        verify_listing_parity(
            &[("p1".to_string(), "a".to_string(), "t".to_string())],
            &[("p1".to_string(), "b".to_string(), "t".to_string())],
        ),
        Err(SplitError::ParityMismatch { kind: ParityKind::Listing, ref run_id }) if run_id == "p1"
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
    use std::error::Error as _;

    // Pin SplitError Display wording once (decision 0071 ruling 8).
    let p = |s: &str| PathBuf::from(s);
    let cases: &[(SplitError, &str)] = &[
        (
            SplitError::RunsDirMissing(p("x")),
            "runs directory 'x' does not exist",
        ),
        (
            SplitError::SymlinkRefused(p("x")),
            "refusing symlink at 'x'",
        ),
        (
            SplitError::RunsPathNotDirectory(p("x")),
            "runs path 'x' is not a directory",
        ),
        (
            SplitError::IndexPathIsDirectory(p("x")),
            "index path 'x' is a directory",
        ),
        (SplitError::InvalidPath(p("")), "invalid path ''"),
        (
            SplitError::TargetNotDirectory(p("x")),
            "migration destination 'x' is not a directory",
        ),
        (
            SplitError::TargetNotEmpty(p("x")),
            "migration destination 'x' is not empty",
        ),
        (
            SplitError::TargetHasNoParent(p("/")),
            "migration destination '/' has no parent directory",
        ),
        (
            SplitError::ProjectionMismatch {
                expected_runs: 2,
                expected_events: 5,
                indexed_runs: 1,
                indexed_events: 4,
            },
            "migration projection mismatch: expected 2 runs / 5 events, indexed 1 / 4",
        ),
    ];
    for (err, expected) in cases {
        assert_eq!(err.to_string(), *expected);
        assert!(err.source().is_none());
    }

    for (kind, label) in [
        (ParityKind::Manifest, "manifest"),
        (ParityKind::EventStream, "event stream"),
        (ParityKind::CanonicalNdjson, "canonical NDJSON"),
        (ParityKind::RunMetadata, "run metadata"),
        (ParityKind::Listing, "run listing"),
    ] {
        let err = SplitError::ParityMismatch {
            kind,
            run_id: "r".to_string(),
        };
        assert_eq!(
            err.to_string(),
            format!("{label} parity mismatch for run 'r'")
        );
        assert!(err.source().is_none());
    }

    let sidecar_err = SplitError::SidecarRemovalFailed {
        path: p("/tmp/index.db-wal"),
        source: std::io::Error::other("is a directory"),
    };
    assert_eq!(
        sidecar_err.to_string(),
        "failed to remove SQLite sidecar '/tmp/index.db-wal': is a directory"
    );
    assert!(sidecar_err.source().is_some());

    let store_err = map_path_err(StoreError::RunNotFound("r".to_string()));
    assert_eq!(store_err.to_string(), "run 'r' not found");
    assert!(store_err.source().is_some());

    let io_err = SplitError::from(std::io::Error::other("boom"));
    assert_eq!(io_err.to_string(), "boom");
    assert!(io_err.source().is_some());

    let json_err = SplitError::from(serde_json::from_str::<Value>("{").unwrap_err());
    assert!(json_err.source().is_some());
    assert_eq!(
        json_err.to_string(),
        "EOF while parsing an object at line 1 column 1"
    );
}

#[test]
fn journal_reindex_readonly_inspection_and_rename_before_sidecar_cleanup() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let runs_dir = dir.path().join("runs");
    let fs_log = FsEventLog::open(&runs_dir).unwrap();
    let manifest = json!({"engine": "0.15.0", "feature": "ro-reidx"});
    fs_log.create_run("r1", &manifest).unwrap();
    let e1 = sealed(
        "r1",
        1,
        EventType::RunStarted,
        json!({"feature": "ro-reidx", "manifest": manifest}),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    fs_log
        .append_envelopes("r1", (0, ZERO_HASH), &[e1])
        .unwrap();

    // Existing valid DELETE-mode index stays byte-identical when reindex fails on corrupt source.
    let idx = dir.path().join("delete-index.db");
    assert_eq!(reindex_paths(&runs_dir, &idx).unwrap(), (1, 1));
    let conn = rusqlite::Connection::open(&idx).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode = DELETE", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode.to_ascii_lowercase(), "delete");
    drop(conn);
    let before = fs::read(&idx).unwrap();
    let bad_runs = dir.path().join("bad-runs");
    let bad_log = FsEventLog::open(&bad_runs).unwrap();
    bad_log.create_run("b1", &manifest).unwrap();
    fs::write(bad_runs.join("b1").join("manifest.json"), "{corrupt").unwrap();
    assert!(matches!(
        reindex_paths(&bad_runs, &idx),
        Err(SplitError::Store(StoreError::Damaged { .. }))
    ));
    assert_eq!(fs::read(&idx).unwrap(), before);

    // Read-only (0o444) damaged index can still be inspected read-only and replaced cleanly.
    let ro_corrupt = dir.path().join("ro-corrupt.db");
    fs::write(&ro_corrupt, b"not a sqlite file").unwrap();
    fs::set_permissions(&ro_corrupt, fs::Permissions::from_mode(0o444)).unwrap();
    assert_eq!(reindex_paths(&runs_dir, &ro_corrupt).unwrap(), (1, 1));

    // When staged rename fails, live target's -wal/-shm sidecars are NEVER unlinked beforehand.
    let live_idx = dir.path().join("live.db");
    assert_eq!(reindex_paths(&runs_dir, &live_idx).unwrap(), (1, 1));
    let live_wal = sqlite_sidecar_path(&live_idx, "-wal");
    fs::write(&live_wal, b"live-wal-bytes").unwrap();
    let target = prepare_index_target(&live_idx).unwrap();
    let mut staged = StagedDb::new(&target.parent).unwrap();
    let err = publish_staged_index_with(&mut staged, &target, |_, _| {
        Err(std::io::Error::other("injected rename failure"))
    })
    .unwrap_err();
    assert!(matches!(err, SplitError::Io(_)));
    assert_eq!(fs::read(&live_wal).unwrap(), b"live-wal-bytes");
}

#[test]
fn journal_migrate_reads_single_consistent_source_snapshot_across_concurrent_writes() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("snap.db");
    drop(seed_monolith(&db));
    let ro = Store::open_read_only(&db).unwrap();
    let staging = tempfile::Builder::new()
        .prefix(".brokkr-migrate-")
        .tempdir_in(dir.path())
        .unwrap();

    // Mutate forge.db via a second writable connection AFTER list_runs() runs inside the snapshot:
    // add a new run AND append an event to an existing unstarted run.
    let mut mutated = false;
    let (runs, events) = stage_migration_with(&ro, staging.path(), || {
        if !mutated {
            mutated = true;
            let mut appender = Store::open(&db).unwrap();
            appender
                .append_next(
                    "run-beta",
                    EventType::RunStarted,
                    json!({"feature": "beta-later", "manifest": {}}),
                    None,
                    None,
                )
                .unwrap();
            drop(appender);
            let mut creator = Store::open(&db).unwrap();
            creator
                .create_run("run-gamma", "gamma", "self", &json!({"feature": "gamma"}))
                .unwrap();
        }
    })
    .unwrap();
    assert!(mutated);
    // Snapshot captured the pre-write state (2 runs, 3 events) with zero torn reads or parity errors.
    assert_eq!((runs, events), (2, 3));
    let migrated_log = FsEventLog::open(staging.path().join("runs")).unwrap();
    assert_eq!(
        migrated_log.list_run_ids().unwrap(),
        vec!["run-alpha".to_string(), "run-beta".to_string()]
    );
    assert!(migrated_log.read_stream("run-beta", 1).unwrap().is_empty());
}

#[test]
fn journal_migrate_atomic_empty_dir_rename_and_in_place_rollback_errors() {
    let dir = tempdir().unwrap();
    let clean_db = dir.path().join("clean.db");
    let clean_store = seed_monolith(&clean_db);

    // Publishing into a non-cwd existing empty directory uses ONE atomic rename syscall.
    let atomic_empty = dir.path().join("atomic-empty");
    fs::create_dir(&atomic_empty).unwrap();
    let atomic_target = prepare_migration_target(&atomic_empty).unwrap();
    let atomic_staging = tempfile::Builder::new()
        .prefix(".brokkr-migrate-")
        .tempdir_in(&atomic_target.parent)
        .unwrap();
    stage_migration(&clean_store, atomic_staging.path()).unwrap();
    let mut single_rename_calls = 0usize;
    publish_staged_migration_with(atomic_staging, &atomic_target, |src, dst| {
        single_rename_calls += 1;
        fs::rename(src, dst)
    })
    .unwrap();
    assert_eq!(single_rename_calls, 1);
    assert!(atomic_empty.join("runs").is_dir());
    assert!(atomic_empty.join("index.db").is_file());

    // In-place fallback (e.g. cwd `.`): rolls back cleanly when rollback succeeds, and surfaces
    // PublishRollbackFailed (never discarding the rollback error) when rollback fails.
    let rollback_empty = dir.path().join("rollback-empty");
    fs::create_dir(&rollback_empty).unwrap();
    let mut rollback_target = prepare_migration_target(&rollback_empty).unwrap();
    rollback_target.in_place = true;
    for fail_rollback in [false, true] {
        let staging = tempfile::Builder::new()
            .prefix(".brokkr-migrate-")
            .tempdir_in(&rollback_target.parent)
            .unwrap();
        stage_migration(&clean_store, staging.path()).unwrap();
        let mut calls = 0usize;
        let err = publish_staged_migration_with(staging, &rollback_target, |src, dst| {
            calls += 1;
            if calls == 2 {
                return Err(std::io::Error::other("injected second rename failure"));
            }
            if calls == 3 && fail_rollback {
                return Err(std::io::Error::other("injected rollback failure"));
            }
            fs::rename(src, dst)
        })
        .unwrap_err();
        if fail_rollback {
            assert!(matches!(err, SplitError::PublishRollbackFailed { .. }));
            assert!(err.source().is_some());
            assert_eq!(
                err.to_string(),
                "migration publish failed (injected second rename failure) and rollback failed (injected rollback failure)"
            );
            fs::remove_dir_all(rollback_empty.join("runs")).unwrap();
        } else {
            assert!(matches!(err, SplitError::Io(_)));
            assert!(fs::read_dir(&rollback_empty).unwrap().next().is_none());
        }
    }
}
