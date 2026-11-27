//! Round 3 hardening tests for the local split backend (decision 0126 ruling 4,
//! Slice 2): path normalization before checks, post-open root swap & read-only
//! `FsEventLog`, pre-WAL `ForeignDatabase` refusal, `SqliteProjectionStore`
//! column/head/orphan verification and error attribution, cooldown conflict &
//! bounds checks, and exact `Display` wording pinning.

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use rusqlite::Connection;

use super::*;
use crate::ports::{
    lexically_normalize, normalize_storage_path, reject_symlink_path, sync_dir, FsEventLog,
    RunMetadata, SqliteCoordinationStore, SqliteProjectionStore,
};
use crate::{Damage, StoreError};

struct RestorePermissions(Vec<(PathBuf, u32)>);

impl Drop for RestorePermissions {
    fn drop(&mut self) {
        for (path, mode) in self.0.iter().rev() {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(*mode));
        }
    }
}

#[test]
fn path_normalization_strips_trailing_slash_and_resolves_parent_before_checks() {
    let dir = tempdir().unwrap();
    let real = dir.path().join("real");
    fs::create_dir(&real).unwrap();
    let sym = dir.path().join("sym");
    symlink(&real, &sym).unwrap();

    // Trailing slash on a symlink is stripped BEFORE symlink_metadata, so symlinks are refused.
    let sym_trailing = PathBuf::from(format!("{}/", sym.display()));
    assert!(matches!(
        FsEventLog::open(&sym_trailing),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        SqliteProjectionStore::open(&sym_trailing),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(&sym_trailing),
        Err(StoreError::SymlinkRefused(_))
    ));

    // `..` segments are resolved BEFORE symlink/sidecar checks and BEFORE create_dir_all.
    let outside = dir.path().join("outside.db");
    fs::write(&outside, b"sentinel").unwrap();
    symlink(&outside, dir.path().join("norm.db-wal")).unwrap();
    let dotted_db = dir
        .path()
        .join("nonexistent-sub")
        .join("..")
        .join("norm.db");
    assert!(matches!(
        SqliteProjectionStore::open(&dotted_db),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(!dir.path().join("nonexistent-sub").exists());

    // Valid dotted path opens cleanly without creating the cancelled intermediate segment.
    let dotted_runs = dir.path().join("ghost").join("..").join("runs-clean");
    let log = FsEventLog::open(&dotted_runs).unwrap();
    assert_eq!(log.root(), dir.path().join("runs-clean").as_path());
    assert!(!dir.path().join("ghost").exists());

    assert_eq!(
        lexically_normalize(Path::new("/a/./b/c/../d/")),
        PathBuf::from("/a/b/d")
    );
    assert!(matches!(
        normalize_storage_path(Path::new("nul\0byte")),
        Err(StoreError::InvalidPath(_))
    ));
    let root_meta = reject_symlink_path(Path::new("/")).unwrap().unwrap();
    assert!(root_meta.is_dir());
}

#[test]
fn fs_event_log_readonly_dir_legacy_missing_lock_and_root_swap() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("ro-feat");
    let meta = RunMetadata {
        feature: "ro-feat".to_string(),
        created_at: "2026-11-26T10:00:00Z".to_string(),
        list_order: Some(0),
    };
    log.create_run_with_metadata("r1", &manifest, &meta)
        .unwrap();
    let e1 = started_event("r1", "ro-feat", "2026-11-26T10:00:00Z");
    log.append_envelopes("r1", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();

    // Read-only runs/ directory works with `.lock` AND when legacy `.lock` is absent.
    let run_dir = root.join("r1");
    let lock_path = run_dir.join(".lock");
    assert!(lock_path.is_file());
    for remove_lock in [false, true] {
        if remove_lock {
            fs::remove_file(&lock_path).unwrap();
        }
        let mut perms = vec![
            (root.clone(), 0o755),
            (run_dir.clone(), 0o755),
            (run_dir.join("manifest.json"), 0o644),
            (run_dir.join("events.ndjson"), 0o644),
            (run_dir.join("run_meta.json"), 0o644),
        ];
        if !remove_lock {
            perms.push((lock_path.clone(), 0o644));
        }
        let _restore = RestorePermissions(perms);
        for file in ["manifest.json", "events.ndjson", "run_meta.json"] {
            fs::set_permissions(run_dir.join(file), fs::Permissions::from_mode(0o444)).unwrap();
        }
        if !remove_lock {
            fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o444)).unwrap();
        }
        fs::set_permissions(&run_dir, fs::Permissions::from_mode(0o555)).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();

        assert_eq!(log.head("r1").unwrap(), (1, e1.event_hash.clone()));
        assert_eq!(log.read_stream("r1", 1).unwrap().len(), 1);
        assert_eq!(log.manifest("r1").unwrap(), manifest);
        assert_eq!(log.run_metadata("r1").unwrap(), Some(meta.clone()));
        assert_eq!(log.list_run_ids().unwrap(), vec!["r1".to_string()]);
    }

    // Post-open root swap: symlink OR replacement real directory refuses with SymlinkRefused.
    let decoy = dir.path().join("decoy-root");
    fs::create_dir(&decoy).unwrap();
    let moved = dir.path().join("moved-root");
    fs::rename(&root, &moved).unwrap();
    symlink(&decoy, &root).unwrap();
    assert!(matches!(log.head("r1"), Err(StoreError::SymlinkRefused(_))));
    assert!(matches!(
        log.manifest("r1"),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        log.list_run_ids(),
        Err(StoreError::SymlinkRefused(_))
    ));
    assert!(matches!(
        log.create_run("r2", &manifest),
        Err(StoreError::SymlinkRefused(_))
    ));

    fs::remove_file(&root).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(matches!(
        log.list_run_ids(),
        Err(StoreError::SymlinkRefused(ref p)) if p == &root
    ));
}

#[test]
fn fs_event_log_nonregular_open_lock_race_and_sync_cleanup() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let _log = FsEventLog::open(&root).unwrap();

    // Non-regular file (directory) refused on BOTH pre-open and post-open checks.
    let non_reg = dir.path().join("as-dir");
    fs::create_dir(&non_reg).unwrap();
    let non_reg_err =
        crate::ports::fs_log::open_no_symlink_with(&non_reg, |p| fs::File::open(p)).unwrap_err();
    assert!(matches!(non_reg_err, StoreError::Io(_)));
    let absent_probe = dir.path().join("becomes-dir");
    let post_non_reg_err =
        crate::ports::fs_log::open_no_symlink_with(&absent_probe, |_| fs::File::open(&non_reg))
            .unwrap_err();
    assert!(matches!(post_non_reg_err, StoreError::Io(_)));

    // Concurrent `.create.lock` AlreadyExists race falls back to opening peer's regular file.
    let race_lock = dir.path().join("race.lock");
    let mut created_once = false;
    let locked = crate::ports::fs_log::open_lock_exclusive_with(&race_lock, |p, create_new| {
        if create_new && !created_once {
            created_once = true;
            fs::write(p, b"")?;
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }
        fs::OpenOptions::new().read(true).write(true).open(p)
    });
    assert!(locked.is_ok());

    // sync_dir_result suppresses InvalidInput ONLY on macOS; Linux propagates it.
    let einval = std::io::Error::from(std::io::ErrorKind::InvalidInput);
    #[cfg(target_os = "macos")]
    assert!(crate::ports::sync_dir_result(Err(einval)).is_ok());
    #[cfg(not(target_os = "macos"))]
    assert_eq!(
        crate::ports::sync_dir_result(Err(einval))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert!(crate::ports::sync_dir_result(Err(std::io::Error::other("io error"))).is_err());

    // Post-rename root fsync failure cleans up `&run_dir` (not the already-moved `&staging`).
    let sync_fail = crate::ports::fs_log::create_run_dir_atomic_with(
        &root,
        "sync-fail-run",
        |staging| {
            fs::write(staging.join("manifest.json"), b"{}").unwrap();
            fs::write(staging.join("events.ndjson"), b"").unwrap();
            fs::write(staging.join(".lock"), b"").unwrap();
            Ok(())
        },
        |_| Err(std::io::Error::other("injected root fsync error")),
    )
    .unwrap_err();
    assert!(matches!(sync_fail, StoreError::Io(_)));
    assert!(!root.join("sync-fail-run").exists());
    sync_dir(&root).unwrap();
}

#[test]
fn standalone_sqlite_refuses_delete_mode_monolith_without_wal_mutation_and_upgrades_legacy_index() {
    let dir = tempdir().unwrap();

    // Monolithic forge.db in DELETE journal mode must be refused BEFORE WAL switch.
    let delete_db = dir.path().join("delete-forge.db");
    let mut store = Store::open(&delete_db).unwrap();
    store
        .create_run("m1", "feat", "self", &sample_manifest("feat"))
        .unwrap();
    drop(store);
    let raw = Connection::open(&delete_db).unwrap();
    let switched: String = raw
        .query_row("PRAGMA journal_mode = DELETE", [], |r| r.get(0))
        .unwrap();
    assert_eq!(switched.to_ascii_lowercase(), "delete");
    drop(raw);
    let before_bytes = fs::read(&delete_db).unwrap();

    assert!(matches!(
        SqliteProjectionStore::open(&delete_db),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(&delete_db),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));
    assert_eq!(fs::read(&delete_db).unwrap(), before_bytes);
    let check = Connection::open(&delete_db).unwrap();
    let after_mode: String = check
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(after_mode.to_ascii_lowercase(), "delete");
    drop(check);

    // Coordination store also refuses non-zero user_version and foreign triggers.
    let uv_coord = dir.path().join("uv-coord.db");
    let conn = Connection::open(&uv_coord).unwrap();
    conn.pragma_update(None, "user_version", 7u32).unwrap();
    drop(conn);
    assert!(matches!(
        SqliteCoordinationStore::open(&uv_coord),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));

    let trig_coord = dir.path().join("trig-coord.db");
    drop(SqliteCoordinationStore::open(&trig_coord).unwrap());
    let conn = Connection::open(&trig_coord).unwrap();
    conn.execute_batch("CREATE TRIGGER evil_trig AFTER INSERT ON meta BEGIN SELECT 1; END;")
        .unwrap();
    drop(conn);
    assert!(matches!(
        SqliteCoordinationStore::open(&trig_coord),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));

    // Legacy Slice 2 disposable index with projection_schema = '1' and user_version = 0 is accepted & upgraded.
    let legacy_idx = dir.path().join("legacy-index.db");
    let proj = SqliteProjectionStore::open(&legacy_idx).unwrap();
    let e1 = started_event("r-leg", "leg", "2026-11-26T10:00:00Z");
    proj.apply_envelopes("r-leg", None, std::slice::from_ref(&e1))
        .unwrap();
    drop(proj);
    let raw = Connection::open(&legacy_idx).unwrap();
    raw.pragma_update(None, "user_version", 0u32).unwrap();
    drop(raw);

    let upgraded = SqliteProjectionStore::open(&legacy_idx).unwrap();
    assert_eq!(
        upgraded.list_runs().unwrap(),
        vec![(
            "r-leg".to_string(),
            "leg".to_string(),
            "2026-11-26T10:00:00Z".to_string()
        )]
    );
    drop(upgraded);
    let verify = Connection::open(&legacy_idx).unwrap();
    let uv: u32 = verify
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(uv, 2);

    // Concurrent fresh opens of SqliteProjectionStore never race on user_version vs table count.
    let race_idx = dir.path().join("race-index.db");
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let p = race_idx.clone();
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let store = SqliteProjectionStore::open(&p).unwrap();
            store.list_runs().unwrap()
        }));
    }
    for h in handles {
        assert!(h.join().unwrap().is_empty());
    }
}

#[test]
fn sqlite_projection_verifies_head_hash_and_all_columns_and_precedence() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();

    // envelopes[0].recorded_at takes precedence over manifest["created_at"].
    let manifest_with_created = json!({
        "engine": "0.15.0",
        "feature": "prec",
        "created_at": "1999-01-01T00:00:00Z"
    });
    let e1 = started_event("r-prec", "prec", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "r-prec",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    proj.apply_envelopes(
        "r-prec",
        Some(&manifest_with_created),
        &[e1.clone(), e2.clone()],
    )
    .unwrap();
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![(
            "r-prec".to_string(),
            "prec".to_string(),
            "2026-11-26T10:00:00Z".to_string()
        )]
    );

    let raw = Connection::open(&index_path).unwrap();

    // Tampering head_hash on head_seq == 0 refuses BOTH empty and non-empty batches.
    proj.apply_envelopes("r-zero", Some(&sample_manifest("z")), &[])
        .unwrap();
    raw.execute(
        "UPDATE projected_runs SET head_hash = 'tampered-zero' WHERE run_id = 'r-zero'",
        [],
    )
    .unwrap();
    assert_divergent(proj.apply_envelopes("r-zero", None, &[]), "r-zero");
    let z1 = started_event("r-zero", "z", "2026-11-26T10:00:00Z");
    assert_divergent(proj.apply_envelopes("r-zero", None, &[z1]), "r-zero");

    // Tampering head_hash when head_seq == 2 refuses BOTH empty batch and stale prefix [e1].
    raw.execute(
        "UPDATE projected_runs SET head_hash = 'tampered-head' WHERE run_id = 'r-prec'",
        [],
    )
    .unwrap();
    assert_divergent(proj.apply_envelopes("r-prec", None, &[]), "r-prec");
    assert_divergent(
        proj.apply_envelopes("r-prec", None, std::slice::from_ref(&e1)),
        "r-prec",
    );
    raw.execute(
        "UPDATE projected_runs SET head_hash = ?1 WHERE run_id = 'r-prec'",
        rusqlite::params![&e2.event_hash],
    )
    .unwrap();

    // Tampering non-hash projected columns refuses with DivergentEventLog.
    for (col, bad_val) in [
        ("event_id", "tampered-id"),
        ("event_type", "run/completed"),
        ("recorded_at", "1999-01-01T00:00:00Z"),
        ("payload", "{\"tampered\":true}"),
    ] {
        let orig: String = raw
            .query_row(
                &format!("SELECT {col} FROM projected_events WHERE run_id = 'r-prec' AND seq = 1"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        raw.execute(
            &format!("UPDATE projected_events SET {col} = ?1 WHERE run_id = 'r-prec' AND seq = 1"),
            rusqlite::params![bad_val],
        )
        .unwrap();
        assert_divergent(
            proj.apply_envelopes("r-prec", None, std::slice::from_ref(&e1)),
            "r-prec",
        );
        raw.execute(
            &format!("UPDATE projected_events SET {col} = ?1 WHERE run_id = 'r-prec' AND seq = 1"),
            rusqlite::params![orig],
        )
        .unwrap();
    }
}

#[test]
fn sqlite_projection_refuses_orphan_events_and_preserves_event_log_error_attribution() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    let e1 = started_event("r-orph", "orph", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "r-orph",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    let e3 = sealed_event(
        "r-orph",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-26T10:00:10Z",
    );
    proj.apply_envelopes("r-orph", None, &[e1.clone(), e2.clone()])
        .unwrap();

    // Orphan projected_events row with seq > head_seq refuses with DivergentEventLog (not SQLite UNIQUE error).
    let raw = Connection::open(&index_path).unwrap();
    raw.execute(
        "INSERT INTO projected_events (run_id, seq, event_id, event_type, event_hash, recorded_at, payload)
         VALUES ('r-orph', 3, 'ev-3', 'run/completed', 'orphan-hash', '2026-11-26T10:00:10Z', '{}')",
        [],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        proj.apply_envelopes("r-orph", None, &[e1, e2, e3]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-orph"
    ));

    // rebuild_from attributes FsEventLog errors to the event log file, NEVER to index_path.
    let runs_root = dir.path().join("runs");
    let fs_log = FsEventLog::open(&runs_root).unwrap();
    fs_log
        .create_run("bad-log", &sample_manifest("bad"))
        .unwrap();
    let events_file = runs_root.join("bad-log").join("events.ndjson");
    fs::write(&events_file, "{truncated\n").unwrap();
    let err = proj.rebuild_from(&fs_log).unwrap_err();
    assert!(matches!(
        err,
        StoreError::Damaged {
            ref journal,
            damage: Damage::Envelope { ref run_id, seq: 1 }
        } if journal == &events_file && run_id == "bad-log"
    ));
}

fn wait_effect_event(run_id: &str, waits: Value) -> EventEnvelope {
    sealed_event(
        run_id,
        1,
        EventType::EffectFailed,
        json!({
            "effect_id": "eff-1",
            "attempt_id": "att-1",
            "waits": waits
        }),
        ZERO_HASH,
        "2026-11-26T10:01:00Z",
    )
}

#[test]
fn coordination_cooldown_checks_expired_conflict_bounds_and_duplicate_inline_seat() {
    let dir = tempdir().unwrap();
    let coord = SqliteCoordinationStore::open(dir.path().join("queue.db")).unwrap();

    let wait_obj = |until: &str, rounds: u64, retry_after: Option<u64>, site: Option<&str>| {
        let mut obj = json!({
            "provider": "anthropic",
            "route": "claude-sonnet",
            "until": until,
            "words": "rate limited",
            "seconds": retry_after.unwrap_or(60),
            "rounds": rounds
        });
        if let Some(s) = site {
            obj["site"] = json!(s);
        }
        if let Some(r) = retry_after {
            obj["retry_after_seconds"] = json!(r);
        }
        obj
    };

    // Seed active cooldown, then send divergent expired event at same key -> DivergentEventLog.
    let active = wait_effect_event(
        "c-exp",
        json!([wait_obj("2026-11-26T10:15:00Z", 1, None, Some("implement"))]),
    );
    assert_eq!(coord.apply_cooldowns("c-exp", &[active]).unwrap(), 1);
    let expired_conflict = wait_effect_event(
        "c-exp",
        json!([wait_obj("2026-11-26T10:00:00Z", 1, None, Some("implement"))]),
    );
    assert!(matches!(
        coord.apply_cooldowns("c-exp", &[expired_conflict]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "c-exp"
    ));

    // Bounds-check u64 `rounds` and `retry_after_seconds` against i64::MAX.
    let overflow_rounds = wait_effect_event(
        "c-ovf-r",
        json!([wait_obj(
            "2026-11-26T10:15:00Z",
            u64::MAX,
            None,
            Some("implement")
        )]),
    );
    assert!(matches!(
        coord.apply_cooldowns("c-ovf-r", &[overflow_rounds]),
        Err(StoreError::CooldownFieldOutOfRange { field: "rounds", value }) if value == u64::MAX
    ));
    let overflow_retry = wait_effect_event(
        "c-ovf-retry",
        json!([wait_obj(
            "2026-11-26T10:15:00Z",
            1,
            Some(u64::MAX),
            Some("implement")
        )]),
    );
    assert!(matches!(
        coord.apply_cooldowns("c-ovf-retry", &[overflow_retry]),
        Err(StoreError::CooldownFieldOutOfRange { field: "retry_after_seconds", value })
            if value == u64::MAX
    ));

    // Duplicate inline-seat (`site: None`) entries in the same batch are rejected even when idempotent = true.
    let inline = wait_obj("2026-11-26T10:15:00Z", 1, None, None);
    let dup_inline = wait_effect_event("c-dup-inline", json!([inline.clone(), inline]));
    assert!(matches!(
        coord.apply_cooldowns("c-dup-inline", &[dup_inline]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "c-dup-inline"
    ));
}

#[test]
fn slice_two_store_error_and_damage_display_wording_pinned() {
    assert_eq!(
        StoreError::Io(std::io::Error::other("disk full")).to_string(),
        "io: disk full"
    );
    assert_eq!(
        StoreError::UnsafeRunId("..".to_string()).to_string(),
        "unsafe run id '..': must be a single path segment without '/', '\\', NUL, '.' or '..'"
    );
    assert_eq!(
        StoreError::SymlinkRefused(PathBuf::from("/tmp/sym")).to_string(),
        "refusing symlink at '/tmp/sym'"
    );
    assert_eq!(
        StoreError::InvalidPath(PathBuf::from("")).to_string(),
        "invalid storage path ''"
    );
    assert_eq!(
        StoreError::ForeignDatabase {
            path: PathBuf::from("/tmp/forge.db"),
            expected: "projection index",
        }
        .to_string(),
        "refusing foreign database at '/tmp/forge.db' (expected projection index)"
    );
    assert_eq!(
        StoreError::CooldownFieldOutOfRange {
            field: "rounds",
            value: u64::MAX,
        }
        .to_string(),
        format!(
            "provider cool-down field 'rounds' value {} exceeds 64-bit integer bound; nothing was written",
            u64::MAX
        )
    );
    assert_eq!(
        Damage::MissingManifest.to_string(),
        "run directory is missing a regular manifest.json"
    );
    assert_eq!(
        Damage::NonUtf8RunDir.to_string(),
        "event log contains a non-UTF-8 directory entry"
    );
    assert_eq!(
        Damage::MalformedJson {
            detail: "bad".to_string()
        }
        .to_string(),
        "malformed JSON: bad"
    );
}

fn assert_divergent(res: Result<(), StoreError>, expected_run: &str) {
    assert!(matches!(
        res,
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == expected_run
    ));
}

fn make_delete_db(path: &Path, sql: &str) -> Vec<u8> {
    let conn = Connection::open(path).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode = DELETE", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode.to_ascii_lowercase(), "delete");
    conn.execute_batch(sql).unwrap();
    drop(conn);
    fs::read(path).unwrap()
}

#[test]
fn queue_trigger_sql_tamper_and_coordination_strict_schema_pre_wal() {
    let dir = tempdir().unwrap();

    // 1. Tampered trigger body under a recognized trigger name refuses queue_intact AND open.
    let tampered = dir.path().join("tampered-trig.db");
    drop(SqliteCoordinationStore::open(&tampered).unwrap());
    let conn = Connection::open(&tampered).unwrap();
    conn.query_row("PRAGMA journal_mode = DELETE", [], |r| {
        r.get::<_, String>(0)
    })
    .unwrap();
    conn.execute_batch(
        "DROP TRIGGER queue_entries_kept;
         CREATE TRIGGER queue_entries_kept BEFORE DELETE ON queue_entries BEGIN SELECT 1; END;",
    )
    .unwrap();
    assert!(!crate::queue::queue_intact(&conn).unwrap());
    assert!(crate::queue::has_foreign_queue_triggers(&conn).unwrap());
    drop(conn);
    let before = fs::read(&tampered).unwrap();
    assert!(matches!(
        SqliteCoordinationStore::open(&tampered),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));
    assert_eq!(fs::read(&tampered).unwrap(), before);

    // 2. Unrelated DB with only `meta`, colliding wrong columns, foreign view, negative user_version.
    for (name, sql) in [
        (
            "only-meta.db",
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        ),
        (
            "wrong-cols.db",
            "CREATE TABLE meta (id INTEGER); CREATE TABLE queue_entries (wrong TEXT);",
        ),
        ("view-only.db", "CREATE VIEW v AS SELECT 1;"),
        ("neg-uv.db", "PRAGMA user_version = -1;"),
    ] {
        let path = dir.path().join(name);
        let bytes = make_delete_db(&path, sql);
        assert!(
            matches!(
                SqliteCoordinationStore::open(&path),
                Err(StoreError::ForeignDatabase {
                    expected: "coordination store",
                    ..
                })
            ),
            "{name} should be refused as ForeignDatabase"
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            bytes,
            "{name} must not mutate before WAL"
        );
    }

    // 3. Same table/column set but altered table SQL (e.g. dropped CHECK constraint on dispatch_settings).
    let altered = dir.path().join("altered-sql.db");
    drop(SqliteCoordinationStore::open(&altered).unwrap());
    let conn = Connection::open(&altered).unwrap();
    conn.query_row("PRAGMA journal_mode = DELETE", [], |r| {
        r.get::<_, String>(0)
    })
    .unwrap();
    conn.execute_batch(
        "DROP TRIGGER dispatch_settings_append_only_update;
         DROP TRIGGER dispatch_settings_append_only_delete;
         DROP TABLE dispatch_settings;
         CREATE TABLE dispatch_settings (
             seq INTEGER PRIMARY KEY,
             bound INTEGER NOT NULL,
             operator TEXT NOT NULL,
             reason TEXT NOT NULL,
             recorded_at TEXT NOT NULL
         );
         CREATE TRIGGER dispatch_settings_append_only_update
             BEFORE UPDATE ON dispatch_settings
             BEGIN SELECT RAISE(ABORT, 'dispatch settings are append-only'); END;
         CREATE TRIGGER dispatch_settings_append_only_delete
             BEFORE DELETE ON dispatch_settings
             BEGIN SELECT RAISE(ABORT, 'dispatch settings are append-only'); END;",
    )
    .unwrap();
    drop(conn);
    let before_alt = fs::read(&altered).unwrap();
    assert!(matches!(
        SqliteCoordinationStore::open(&altered),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));
    assert_eq!(fs::read(&altered).unwrap(), before_alt);
}

#[test]
fn projection_strict_schema_negative_user_version_and_readonly_inspect() {
    let dir = tempdir().unwrap();

    // Views, extra indexes, negative user_version, and colliding wrong-column tables are refused pre-WAL.
    for (name, sql) in [
        ("view-only.db", "CREATE VIEW extra_view AS SELECT 1;"),
        ("neg-uv.db", "PRAGMA user_version = -5;"),
        (
            "wrong-cols.db",
            "CREATE TABLE projection_meta (k TEXT);
             CREATE TABLE projected_runs (r TEXT);
             CREATE TABLE projected_events (e TEXT);
             CREATE INDEX projected_runs_created_idx ON projected_runs (r);",
        ),
    ] {
        let path = dir.path().join(name);
        let bytes = make_delete_db(&path, sql);
        assert!(
            matches!(
                SqliteProjectionStore::open(&path),
                Err(StoreError::ForeignDatabase {
                    expected: "projection index",
                    ..
                })
            ),
            "{name} should be refused"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(matches!(
            SqliteProjectionStore::inspect_read_only(&path),
            Err(StoreError::ForeignDatabase {
                expected: "projection index",
                ..
            })
        ));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    // Extra user index on an otherwise valid projection DB is refused pre-WAL.
    let extra_idx = dir.path().join("extra-idx.db");
    drop(SqliteProjectionStore::open(&extra_idx).unwrap());
    let conn = Connection::open(&extra_idx).unwrap();
    conn.query_row("PRAGMA journal_mode = DELETE", [], |r| {
        r.get::<_, String>(0)
    })
    .unwrap();
    conn.execute_batch("CREATE INDEX extra_user_idx ON projected_runs (feature);")
        .unwrap();
    drop(conn);
    let before = fs::read(&extra_idx).unwrap();
    assert!(matches!(
        SqliteProjectionStore::open(&extra_idx),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
    assert_eq!(fs::read(&extra_idx).unwrap(), before);

    // inspect_read_only accepts a valid DELETE-mode 0o444 index without mutating bytes,
    // and returns Damaged on a read-only 0o444 corrupt file.
    let ro_idx = dir.path().join("ro-valid.db");
    drop(SqliteProjectionStore::open(&ro_idx).unwrap());
    let conn = Connection::open(&ro_idx).unwrap();
    conn.query_row("PRAGMA journal_mode = DELETE", [], |r| {
        r.get::<_, String>(0)
    })
    .unwrap();
    conn.pragma_update(None, "user_version", 0i64).unwrap();
    drop(conn);
    let ro_bytes = fs::read(&ro_idx).unwrap();
    let _restore = RestorePermissions(vec![(ro_idx.clone(), 0o644)]);
    fs::set_permissions(&ro_idx, fs::Permissions::from_mode(0o444)).unwrap();
    SqliteProjectionStore::inspect_read_only(&ro_idx).unwrap();
    assert_eq!(fs::read(&ro_idx).unwrap(), ro_bytes);

    let corrupt = dir.path().join("ro-corrupt.db");
    fs::write(&corrupt, b"garbage").unwrap();
    fs::set_permissions(&corrupt, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(matches!(
        SqliteProjectionStore::inspect_read_only(&corrupt),
        Err(StoreError::Damaged {
            damage: Damage::NotADatabase,
            ..
        })
    ));
    fs::set_permissions(&corrupt, fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn projection_event_columns_tamper_refuses_event_id_attempt_id_and_member() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();

    // Event 1 has NULL attempt_id & member; Event 2 has Some(attempt_id) & Some(member).
    let e1 = started_event("r-cols", "cols", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "r-cols",
        2,
        EventType::EffectCheckpointed,
        json!({
            "attempt_id": "att-42",
            "checkpoint": {"member": "reviewer-a", "summary": "ok"}
        }),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    let e3 = sealed_event(
        "r-cols",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-26T10:00:10Z",
    );
    proj.apply_envelopes("r-cols", None, &[e1.clone(), e2.clone(), e3])
        .unwrap();

    let raw = Connection::open(&index_path).unwrap();
    let (eid, att, mem): (String, Option<String>, Option<String>) = raw
        .query_row(
            "SELECT event_id, attempt_id, member FROM projected_events WHERE run_id = 'r-cols' AND seq = 2",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(eid, e2.event_id);
    assert_eq!(att.as_deref(), Some("att-42"));
    assert_eq!(mem.as_deref(), Some("reviewer-a"));

    // Tampering event_id, attempt_id, and member (both Some -> wrong/NULL and NULL -> Some) refuses!
    let cases: &[(i64, &str, Option<&str>, Option<&str>)] = &[
        (2, "event_id", Some("evil-id"), Some(&e2.event_id)),
        (2, "attempt_id", Some("att-evil"), Some("att-42")),
        (2, "attempt_id", None, Some("att-42")),
        (2, "member", Some("evil-member"), Some("reviewer-a")),
        (2, "member", None, Some("reviewer-a")),
        (1, "attempt_id", Some("injected"), None),
        (1, "member", Some("injected"), None),
    ];
    for &(seq, col, bad, orig) in cases {
        raw.execute(
            &format!("UPDATE projected_events SET {col} = ?1 WHERE run_id = 'r-cols' AND seq = ?2"),
            rusqlite::params![bad, seq],
        )
        .unwrap();
        assert_divergent(
            proj.apply_envelopes("r-cols", None, &[e1.clone(), e2.clone()]),
            "r-cols",
        );
        raw.execute(
            &format!("UPDATE projected_events SET {col} = ?1 WHERE run_id = 'r-cols' AND seq = ?2"),
            rusqlite::params![orig, seq],
        )
        .unwrap();
    }
}

#[test]
fn projection_migrated_listing_precedence_matches_between_live_and_rebuild() {
    let dir = tempdir().unwrap();
    // Migrated listing precedence: unstarted run uses run_meta.json fallback; once RunStarted
    // commits, apply_envelopes and rebuild_from agree identically (RunStarted > manifest).
    let runs_root = dir.path().join("runs");
    let log = FsEventLog::open(&runs_root).unwrap();
    let bare_manifest = json!({"engine": "0.15.0"});
    let meta = RunMetadata {
        feature: "pre-start-meta".to_string(),
        created_at: "2026-11-26T08:00:00Z".to_string(),
        list_order: Some(0),
    };
    log.create_run_with_metadata("r-mig", &bare_manifest, &meta)
        .unwrap();
    let idx_live = SqliteProjectionStore::open(dir.path().join("live.db")).unwrap();
    let idx_rebuilt = SqliteProjectionStore::open(dir.path().join("rebuilt.db")).unwrap();
    idx_rebuilt.rebuild_from(&log).unwrap();
    assert_eq!(
        idx_rebuilt.list_runs().unwrap(),
        vec![(
            "r-mig".to_string(),
            "pre-start-meta".to_string(),
            "2026-11-26T08:00:00Z".to_string()
        )]
    );

    let start = started_event("r-mig", "started-wins", "2026-11-26T09:30:00Z");
    log.append_envelopes("r-mig", (0, ZERO_HASH), std::slice::from_ref(&start))
        .unwrap();
    idx_live
        .apply_envelopes("r-mig", Some(&bare_manifest), std::slice::from_ref(&start))
        .unwrap();
    idx_rebuilt.rebuild_from(&log).unwrap();
    let expected = vec![(
        "r-mig".to_string(),
        "started-wins".to_string(),
        "2026-11-26T09:30:00Z".to_string(),
    )];
    assert_eq!(idx_live.list_runs().unwrap(), expected);
    assert_eq!(idx_rebuilt.list_runs().unwrap(), expected);
}

#[test]
fn fs_event_log_malformed_json_attribution_and_rebuild_from_real_files() {
    let dir = tempdir().unwrap();
    let proj = SqliteProjectionStore::open(dir.path().join("index.db")).unwrap();

    // 1. Corrupt manifest.json -> attributed to manifest.json both directly and via rebuild_from.
    let r_mf = dir.path().join("runs-mf");
    let log_mf = FsEventLog::open(&r_mf).unwrap();
    log_mf.create_run("bad-mf", &sample_manifest("m")).unwrap();
    let mf_path = r_mf.join("bad-mf").join("manifest.json");
    fs::write(&mf_path, "{broken-manifest").unwrap();
    assert!(matches!(
        log_mf.manifest("bad-mf"),
        Err(StoreError::Damaged { ref journal, damage: Damage::MalformedJson { .. } })
            if journal == &mf_path
    ));
    assert!(matches!(
        proj.rebuild_from(&log_mf),
        Err(StoreError::Damaged { ref journal, damage: Damage::MalformedJson { .. } })
            if journal == &mf_path
    ));

    // 2. Corrupt run_meta.json -> attributed to run_meta.json both directly and via rebuild_from.
    let r_rm = dir.path().join("runs-rm");
    let log_rm = FsEventLog::open(&r_rm).unwrap();
    log_rm
        .create_run_with_metadata(
            "bad-rm",
            &sample_manifest("rm"),
            &RunMetadata {
                feature: "rm".to_string(),
                created_at: "2026-11-26T10:00:00Z".to_string(),
                list_order: Some(0),
            },
        )
        .unwrap();
    let rm_path = r_rm.join("bad-rm").join("run_meta.json");
    fs::write(&rm_path, r#"{"unknown_field": 1}"#).unwrap();
    assert!(matches!(
        log_rm.run_metadata("bad-rm"),
        Err(StoreError::Damaged { ref journal, damage: Damage::MalformedJson { .. } })
            if journal == &rm_path
    ));
    assert!(matches!(
        proj.rebuild_from(&log_rm),
        Err(StoreError::Damaged { ref journal, damage: Damage::MalformedJson { .. } })
            if journal == &rm_path
    ));

    // 3. Valid JSON that is not an EventEnvelope in events.ndjson -> attributed to events.ndjson.
    let r_ev = dir.path().join("runs-ev");
    let log_ev = FsEventLog::open(&r_ev).unwrap();
    log_ev.create_run("bad-ev", &sample_manifest("ev")).unwrap();
    let ev_path = r_ev.join("bad-ev").join("events.ndjson");
    fs::write(&ev_path, "{\"not\":\"envelope\"}\n").unwrap();
    assert!(matches!(
        proj.rebuild_from(&log_ev),
        Err(StoreError::Damaged { ref journal, damage: Damage::MalformedJson { .. } })
            if journal == &ev_path
    ));
}

#[test]
fn monolith_open_refuses_tampered_queue_trigger_as_damaged() {
    let dir = tempdir().unwrap();
    let forge_db = dir.path().join("forge.db");
    drop(Store::open(&forge_db).unwrap());

    let raw = Connection::open(&forge_db).unwrap();
    raw.execute_batch(
        "DROP TRIGGER queue_entries_kept;
         CREATE TRIGGER queue_entries_kept BEFORE DELETE ON queue_entries BEGIN SELECT 1; END;",
    )
    .unwrap();
    drop(raw);

    assert!(matches!(
        Store::open(&forge_db),
        Err(StoreError::Damaged {
            ref journal,
            damage: Damage::Malformed,
        }) if journal == &forge_db
    ));
}

#[test]
fn sqlite_like_underscore_view_and_non_regular_fifo_refused_pre_wal() {
    let dir = tempdir().unwrap();

    // `sqliteX_report` is NOT an internal `sqlite_` object and must be refused pre-WAL.
    let view_db = dir.path().join("sqlite-x-view.db");
    let before = make_delete_db(&view_db, "CREATE VIEW sqliteX_report AS SELECT 1;");
    assert!(matches!(
        SqliteProjectionStore::open(&view_db),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
    assert!(matches!(
        SqliteProjectionStore::inspect_read_only(&view_db),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
    assert!(matches!(
        SqliteCoordinationStore::open(&view_db),
        Err(StoreError::ForeignDatabase {
            expected: "coordination store",
            ..
        })
    ));
    assert_eq!(fs::read(&view_db).unwrap(), before);
    let check = Connection::open(&view_db).unwrap();
    let mode: String = check
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(mode.to_ascii_lowercase(), "delete");
    drop(check);

    // Non-regular targets (FIFO and directory) are refused immediately with InvalidPath.
    let fifo = dir.path().join("named.fifo");
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo available on POSIX hosts");
    assert!(status.success());
    let as_dir = dir.path().join("a-dir");
    fs::create_dir(&as_dir).unwrap();
    for bad in [&fifo, &as_dir] {
        assert_invalid_standalone_path(bad);
    }
}

#[test]
fn fs_event_log_root_swap_before_write_and_legacy_lock_mutual_exclusion() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("swap");
    log.create_run("r1", &manifest).unwrap();
    let e1 = started_event("r1", "swap", "2026-11-26T10:00:00Z");

    // 1. Swapping `root` before write refuses with SymlinkRefused and writes ZERO bytes.
    let decoy_root = dir.path().join("decoy-root");
    fs::create_dir_all(decoy_root.join("r1")).unwrap();
    fs::write(decoy_root.join("r1").join("events.ndjson"), b"").unwrap();
    let moved_root = dir.path().join("moved-root");
    let err = log
        .append_envelopes_with("r1", (0, ZERO_HASH), std::slice::from_ref(&e1), || {
            fs::rename(&root, &moved_root).unwrap();
            fs::rename(&decoy_root, &root).unwrap();
        })
        .unwrap_err();
    assert!(matches!(err, StoreError::SymlinkRefused(_)));
    assert!(fs::read(root.join("r1").join("events.ndjson"))
        .unwrap()
        .is_empty());
    assert!(fs::read(moved_root.join("r1").join("events.ndjson"))
        .unwrap()
        .is_empty());
    fs::rename(&root, &decoy_root).unwrap();
    fs::rename(&moved_root, &root).unwrap();

    // 2. Swapping `run_dir` before write also refuses with SymlinkRefused and writes ZERO bytes.
    let decoy_run = dir.path().join("decoy-run");
    fs::create_dir(&decoy_run).unwrap();
    fs::write(decoy_run.join("events.ndjson"), b"").unwrap();
    let moved_run = dir.path().join("moved-run");
    let run_dir = root.join("r1");
    let err = log
        .append_envelopes_with("r1", (0, ZERO_HASH), std::slice::from_ref(&e1), || {
            fs::rename(&run_dir, &moved_run).unwrap();
            fs::rename(&decoy_run, &run_dir).unwrap();
        })
        .unwrap_err();
    assert!(matches!(err, StoreError::SymlinkRefused(_)));
    assert!(fs::read(run_dir.join("events.ndjson")).unwrap().is_empty());
    assert!(fs::read(moved_run.join("events.ndjson"))
        .unwrap()
        .is_empty());
    fs::rename(&run_dir, &decoy_run).unwrap();
    fs::rename(&moved_run, &run_dir).unwrap();

    // 3. Legacy run directory without `.lock`: writer acquires exclusive lock on `manifest.json`
    // BEFORE creating `.lock`, strictly excluding concurrent shared readers.
    let lock_path = run_dir.join(".lock");
    fs::remove_file(&lock_path).unwrap();
    let shared = fs::File::open(run_dir.join("manifest.json")).unwrap();
    shared.lock_shared().unwrap();

    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done_bg = Arc::clone(&done);
    let root_bg = root.clone();
    let e1_bg = e1.clone();
    let handle = thread::spawn(move || {
        let bg = FsEventLog::open(&root_bg).unwrap();
        bg.append_envelopes("r1", (0, ZERO_HASH), &[e1_bg]).unwrap();
        done_bg.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    thread::sleep(std::time::Duration::from_millis(40));
    assert!(!done.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!lock_path.exists());

    drop(shared);
    handle.join().unwrap();
    assert!(done.load(std::sync::atomic::Ordering::SeqCst));
    assert!(lock_path.is_file());
    assert_eq!(log.head("r1").unwrap(), (1, e1.event_hash));
}

fn assert_malformed_json_at<T: std::fmt::Debug>(
    result: Result<T, StoreError>,
    expected_path: &Path,
) {
    assert!(
        matches!(
            result,
            Err(StoreError::Damaged {
                ref journal,
                damage: Damage::MalformedJson { .. },
            }) if journal == expected_path
        ),
        "expected MalformedJson at {}: {result:?}",
        expected_path.display()
    );
}

#[test]
fn projection_untouched_readonly_wal_and_legacy_v1_upgrade() {
    let dir = tempdir().unwrap();

    // 1. Read-only WAL `index.db` (0o444) inside a 0o555 directory succeeds with zero sidecars.
    let ro_dir = dir.path().join("ro-wal-dir");
    fs::create_dir(&ro_dir).unwrap();
    let ro_wal = ro_dir.join("index.db");
    let proj = SqliteProjectionStore::open(&ro_wal).unwrap();
    let e1 = started_event("r-ro", "ro", "2026-11-26T10:00:00Z");
    proj.apply_envelopes("r-ro", None, std::slice::from_ref(&e1))
        .unwrap();
    drop(proj);
    assert!(!crate::ports::sqlite_sidecar_path(&ro_wal, "-wal").exists());
    assert!(!crate::ports::sqlite_sidecar_path(&ro_wal, "-shm").exists());
    {
        let _restore = RestorePermissions(vec![(ro_dir.clone(), 0o755), (ro_wal.clone(), 0o644)]);
        fs::set_permissions(&ro_wal, fs::Permissions::from_mode(0o444)).unwrap();
        fs::set_permissions(&ro_dir, fs::Permissions::from_mode(0o555)).unwrap();
        SqliteProjectionStore::inspect_read_only(&ro_wal).unwrap();
        assert!(!crate::ports::sqlite_sidecar_path(&ro_wal, "-wal").exists());
        assert!(!crate::ports::sqlite_sidecar_path(&ro_wal, "-shm").exists());
    }

    // 2. Legacy Round 3 v1 layout (without event_id/attempt_id/member) is accepted untouched by
    // inspect_read_only and upgraded cleanly by open(); bad user_version is refused.
    let v1_path = dir.path().join("legacy-v1.db");
    let conn = Connection::open(&v1_path).unwrap();
    crate::ports::sqlite_projection::seed_legacy_v1_schema(&conn, 1);
    drop(conn);
    let v1_bytes = fs::read(&v1_path).unwrap();
    SqliteProjectionStore::inspect_read_only(&v1_path).unwrap();
    assert_eq!(fs::read(&v1_path).unwrap(), v1_bytes);
    let upgraded = SqliteProjectionStore::open(&v1_path).unwrap();
    upgraded
        .apply_envelopes("r-ro", None, std::slice::from_ref(&e1))
        .unwrap();
    assert_eq!(upgraded.list_runs().unwrap().len(), 1);
    drop(upgraded);
    let check = Connection::open(&v1_path).unwrap();
    let uv: i64 = check
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    let schema_ver: String = check
        .query_row(
            "SELECT value FROM projection_meta WHERE key = 'projection_schema'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!((uv, schema_ver.as_str()), (2, "2"));
    drop(check);

    let bad_v1 = dir.path().join("bad-v1.db");
    let bad_conn = Connection::open(&bad_v1).unwrap();
    crate::ports::sqlite_projection::seed_legacy_v1_schema(&bad_conn, 99);
    drop(bad_conn);
    assert!(matches!(
        SqliteProjectionStore::inspect_read_only(&bad_v1),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
}

#[test]
fn projection_listing_precedence_unification_and_unchanged_head_fill() {
    let dir = tempdir().unwrap();
    let runs_root = dir.path().join("runs-prec");
    let log = FsEventLog::open(&runs_root).unwrap();
    let bare = json!({"engine": "0.15.0"});
    let meta = RunMetadata {
        feature: "pre-start-only".to_string(),
        created_at: "2026-11-26T08:00:00Z".to_string(),
        list_order: Some(0),
    };
    assert_eq!(
        SqliteProjectionStore::resolve_listing(
            &[],
            Some(&json!({"feature": "manifest-loses"})),
            Some(&meta),
        ),
        ("pre-start-only", "2026-11-26T08:00:00Z")
    );
    log.create_run_with_metadata("r-uni", &bare, &meta).unwrap();
    let idx = SqliteProjectionStore::open(dir.path().join("uni.db")).unwrap();
    idx.rebuild_from(&log).unwrap();
    assert_eq!(idx.list_runs().unwrap()[0].1, "pre-start-only");

    // Once envelopes is non-empty, never inherit pre-start run_meta.json feature from existing.
    let no_feat = sealed_event(
        "r-uni",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-26T09:00:00Z",
    );
    idx.apply_envelopes("r-uni", None, std::slice::from_ref(&no_feat))
        .unwrap();
    assert_eq!(
        idx.list_runs().unwrap(),
        vec![(
            "r-uni".to_string(),
            String::new(),
            "2026-11-26T09:00:00Z".to_string()
        )]
    );

    // Unchanged head_seq with Some(manifest) fills empty feature, and does not overwrite non-empty.
    idx.apply_envelopes(
        "r-uni",
        Some(&json!({"feature": "late-manifest"})),
        std::slice::from_ref(&no_feat),
    )
    .unwrap();
    assert_eq!(idx.list_runs().unwrap()[0].1, "late-manifest");
    idx.apply_envelopes(
        "r-uni",
        Some(&json!({"feature": "should-not-overwrite"})),
        &[],
    )
    .unwrap();
    assert_eq!(idx.list_runs().unwrap()[0].1, "late-manifest");

    // Empty batch with Some(manifest) at unchanged head_seq > 0 also fills empty feature.
    let e_empty = sealed_event(
        "r-uni-empty",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-26T09:05:00Z",
    );
    idx.apply_envelopes("r-uni-empty", None, &[e_empty])
        .unwrap();
    assert_eq!(idx.list_runs().unwrap()[1].1, "");
    idx.apply_envelopes(
        "r-uni-empty",
        Some(&json!({"feature": "filled-on-empty-batch"})),
        &[],
    )
    .unwrap();
    assert_eq!(idx.list_runs().unwrap()[1].1, "filled-on-empty-batch");
}

#[test]
fn coordination_recordable_run_id_and_fs_non_utf8_error_attribution() {
    let dir = tempdir().unwrap();

    // 1. SqliteCoordinationStore::apply_cooldowns enforces import::recordable(run_id).
    let coord = SqliteCoordinationStore::open(dir.path().join("coord.db")).unwrap();
    assert!(matches!(
        coord.apply_cooldowns("", &[]),
        Err(StoreError::RunIdUnnameable(_))
    ));
    assert!(matches!(
        coord.apply_cooldowns("bad\0id", &[]),
        Err(StoreError::RunIdUnnameable(_))
    ));
    let long_id = "a".repeat(129);
    assert!(matches!(
        coord.apply_cooldowns(&long_id, &[]),
        Err(StoreError::RunIdTooLong { length: 129 })
    ));

    // 2. Non-UTF-8 corruption in manifest.json, run_meta.json, and events.ndjson preserves
    // exact file path in StoreError::Damaged instead of returning bare Io(InvalidData).
    let proj = SqliteProjectionStore::open(dir.path().join("idx.db")).unwrap();
    let bad_utf8: &[u8] = b"{\"feature\":\"\xff\"}";

    let r_mf = dir.path().join("utf8-mf");
    let log_mf = FsEventLog::open(&r_mf).unwrap();
    log_mf.create_run("u-mf", &sample_manifest("m")).unwrap();
    let mf_path = r_mf.join("u-mf").join("manifest.json");
    fs::write(&mf_path, bad_utf8).unwrap();
    assert_malformed_json_at(log_mf.manifest("u-mf"), &mf_path);

    let r_rm = dir.path().join("utf8-rm");
    let log_rm = FsEventLog::open(&r_rm).unwrap();
    log_rm
        .create_run_with_metadata(
            "u-rm",
            &sample_manifest("rm"),
            &RunMetadata {
                feature: "rm".to_string(),
                created_at: "2026-11-26T10:00:00Z".to_string(),
                list_order: Some(0),
            },
        )
        .unwrap();
    let rm_path = r_rm.join("u-rm").join("run_meta.json");
    fs::write(&rm_path, bad_utf8).unwrap();
    assert_malformed_json_at(log_rm.run_metadata("u-rm"), &rm_path);

    let r_ev = dir.path().join("utf8-ev");
    let log_ev = FsEventLog::open(&r_ev).unwrap();
    log_ev.create_run("u-ev", &sample_manifest("ev")).unwrap();
    let ev_path = r_ev.join("u-ev").join("events.ndjson");
    fs::write(&ev_path, b"\xff\n").unwrap();
    assert_malformed_json_at(log_ev.read_stream("u-ev", 1), &ev_path);
    assert_malformed_json_at(proj.rebuild_from(&log_ev), &ev_path);
}

#[test]
fn fs_event_log_legacy_reader_lock_handoff_and_openat_anchoring() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs-handoff");
    let log = FsEventLog::open(&root).unwrap();
    let manifest = sample_manifest("handoff");
    log.create_run("r1", &manifest).unwrap();
    let e1 = started_event("r1", "handoff", "2026-11-26T10:00:00Z");
    log.append_envelopes("r1", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();

    // 1. Legacy reader lock handoff: `.lock` is absent when reader starts, then appears while
    // acquiring `manifest.json`'s shared lock. Reader must drop `manifest.json` and switch to
    // `lock_shared()` on `.lock`, blocking behind an exclusive `.lock` holder without deadlock.
    let run_dir = root.join("r1");
    let lock_path = run_dir.join(".lock");
    fs::remove_file(&lock_path).unwrap();

    let entered_fallback = Arc::new(Barrier::new(2));
    let lock_created = Arc::new(Barrier::new(3));
    let release = Arc::new(Barrier::new(2));
    let entered_bg = Arc::clone(&entered_fallback);
    let created_bg = Arc::clone(&lock_created);
    let release_bg = Arc::clone(&release);
    let lock_path_bg = lock_path.clone();
    let holder = thread::spawn(move || {
        entered_bg.wait();
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&lock_path_bg)
            .unwrap();
        f.lock().unwrap();
        created_bg.wait();
        release_bg.wait();
        drop(f);
    });

    let manifest_path = run_dir.join("manifest.json");
    let handed_off = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let handed_off_r = Arc::clone(&handed_off);
    let created_r = Arc::clone(&lock_created);
    let root_r = root.clone();
    let reader = thread::spawn(move || {
        let rlog = FsEventLog::open(&root_r).unwrap();
        let events = rlog
            .read_run_events_with("r1", || {
                entered_fallback.wait();
                created_r.wait();
            })
            .unwrap();
        handed_off_r.store(true, std::sync::atomic::Ordering::SeqCst);
        events
    });

    // Wait until `.lock` is created & held exclusively while reader is in fallback, then prove
    // reader drops `manifest.json` (exclusive try_lock succeeds) while staying blocked on `.lock`.
    lock_created.wait();
    let manifest_probe = fs::File::open(&manifest_path).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match manifest_probe.try_lock() {
            Ok(()) => break,
            Err(_) if std::time::Instant::now() < deadline => {
                thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(err) => panic!("reader never dropped manifest.json lock on handoff: {err}"),
        }
    }
    assert!(!handed_off.load(std::sync::atomic::Ordering::SeqCst));
    drop(manifest_probe);

    release.wait();
    holder.join().unwrap();
    let events = reader.join().unwrap();
    assert!(handed_off.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(events.len(), 1);

    // 2. If `.lock` becomes a symlink while acquiring `manifest.json`, handoff refuses SymlinkRefused.
    let decoy = dir.path().join("decoy-lock");
    fs::write(&decoy, b"").unwrap();
    fs::remove_file(&lock_path).unwrap();
    let sym_err = log
        .read_run_events_with("r1", || {
            symlink(&decoy, &lock_path).unwrap();
        })
        .unwrap_err();
    assert!(matches!(sym_err, StoreError::SymlinkRefused(_)));
    fs::remove_file(&lock_path).unwrap();
}

#[test]
fn projection_live_listing_precedence_preserves_unstarted_and_started_metadata() {
    let dir = tempdir().unwrap();
    let runs_root = dir.path().join("runs-live");
    let log = FsEventLog::open(&runs_root).unwrap();
    let bare = json!({"engine": "0.15.0"});
    let meta = RunMetadata {
        feature: "unstarted-meta".to_string(),
        created_at: "2026-11-26T08:00:00Z".to_string(),
        list_order: Some(0),
    };
    log.create_run_with_metadata("r-unstarted", &bare, &meta)
        .unwrap();

    let idx = SqliteProjectionStore::open(dir.path().join("live.db")).unwrap();
    idx.rebuild_from(&log).unwrap();
    assert_eq!(
        idx.list_runs().unwrap(),
        vec![(
            "r-unstarted".to_string(),
            "unstarted-meta".to_string(),
            "2026-11-26T08:00:00Z".to_string(),
        )]
    );

    // 1. Empty batch with `Some(manifest)` on an existing `head_seq == 0` row preserves its
    // `run_meta.json` `(feature, created_at)` instead of erasing it.
    idx.apply_envelopes("r-unstarted", Some(&bare), &[])
        .unwrap();
    assert_eq!(
        idx.list_runs().unwrap(),
        vec![(
            "r-unstarted".to_string(),
            "unstarted-meta".to_string(),
            "2026-11-26T08:00:00Z".to_string(),
        )]
    );
    idx.apply_envelopes(
        "r-unstarted",
        Some(&json!({"engine": "0.15.0", "feature": "manifest-should-not-erase"})),
        &[],
    )
    .unwrap();
    assert_eq!(
        idx.list_runs().unwrap(),
        vec![(
            "r-unstarted".to_string(),
            "unstarted-meta".to_string(),
            "2026-11-26T08:00:00Z".to_string(),
        )]
    );

    // 2. Already-started run (`head_seq > 0`) whose feature came from `manifest` (RunStarted had
    // no feature) preserves its existing `feature` on subsequent `manifest = None` appends!
    let e1 = sealed_event(
        "r-started",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-26T10:00:00Z",
    );
    let e2 = sealed_event(
        "r-started",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:01:00Z",
    );
    idx.apply_envelopes(
        "r-started",
        Some(&json!({"engine": "0.15.0", "feature": "started-from-manifest"})),
        std::slice::from_ref(&e1),
    )
    .unwrap();
    idx.apply_envelopes("r-started", None, &[e1, e2]).unwrap();
    assert_eq!(
        idx.list_runs().unwrap()[1],
        (
            "r-started".to_string(),
            "started-from-manifest".to_string(),
            "2026-11-26T10:00:00Z".to_string(),
        )
    );
}

fn seed_v1_projection_db(path: &Path, events_columns: &str, user_version: i64) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS projection_meta (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS projected_runs (
             run_id TEXT PRIMARY KEY,
             feature TEXT NOT NULL,
             created_at TEXT NOT NULL,
             head_seq INTEGER NOT NULL,
             head_hash TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS projected_events (
             {events_columns},
             PRIMARY KEY (run_id, seq)
         );
         CREATE INDEX IF NOT EXISTS projected_runs_created_idx
             ON projected_runs (created_at, run_id);
         INSERT INTO projection_meta (key, value) VALUES ('projection_schema', '1');"
    ))
    .unwrap();
    conn.pragma_update(None, "user_version", user_version)
        .unwrap();
}

#[test]
fn projection_verbatim_round3_six_column_v1_upgrade_and_four_column_refusal() {
    let dir = tempdir().unwrap();
    // Verbatim Round 3 (2580d53af) 6-column legacy v1 DDL with user_version = 0 and 1 is
    // accepted untouched and upgraded cleanly; a bogus 4-column table is refused as ForeignDatabase.
    let six_cols = concat!(
        "run_id TEXT NOT NULL, seq INTEGER NOT NULL, event_type TEXT NOT NULL, ",
        "event_hash TEXT NOT NULL, recorded_at TEXT NOT NULL, payload TEXT NOT NULL"
    );
    for uv in [0i64, 1i64] {
        let v1_db = dir.path().join(format!("round3-v1-uv{uv}.db"));
        seed_v1_projection_db(&v1_db, six_cols, uv);
        SqliteProjectionStore::inspect_read_only(&v1_db).unwrap();
        drop(SqliteProjectionStore::open(&v1_db).unwrap());
    }

    let four_col = dir.path().join("bogus-4col.db");
    let four_cols = "run_id TEXT NOT NULL, seq INTEGER NOT NULL, event_hash TEXT NOT NULL, payload TEXT NOT NULL";
    seed_v1_projection_db(&four_col, four_cols, 1);
    assert!(matches!(
        SqliteProjectionStore::inspect_read_only(&four_col),
        Err(StoreError::ForeignDatabase {
            expected: "projection index",
            ..
        })
    ));
}

#[test]
fn fs_event_log_openat_error_branches_and_non_regular_children() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("runs-err");
    let log = FsEventLog::open(&root).unwrap();
    log.create_run("r1", &sample_manifest("err")).unwrap();
    let run_dir = root.join("r1");
    let e1 = started_event("r1", "err", "2026-11-26T10:00:00Z");

    // Symlink `.lock` on exclusive append -> SymlinkRefused.
    let decoy = dir.path().join("decoy");
    fs::write(&decoy, b"").unwrap();
    let lock_path = run_dir.join(".lock");
    fs::remove_file(&lock_path).unwrap();
    symlink(&decoy, &lock_path).unwrap();
    assert!(matches!(
        log.append_envelopes("r1", (0, ZERO_HASH), std::slice::from_ref(&e1)),
        Err(StoreError::SymlinkRefused(_))
    ));
    fs::remove_file(&lock_path).unwrap();

    // Read-only run_dir without `.lock` makes `open_or_create_lock_at` return Io(PermissionDenied).
    let run_dir_fd = fs::File::open(&run_dir).unwrap();
    {
        let _restore = RestorePermissions(vec![(run_dir.clone(), 0o755)]);
        fs::set_permissions(&run_dir, fs::Permissions::from_mode(0o555)).unwrap();
        let err =
            crate::ports::fs_log::open_or_create_lock_at(&run_dir_fd, &lock_path).unwrap_err();
        assert!(matches!(err, StoreError::Io(_)));
    }

    // Symlink `run_meta.json` on `run_metadata` -> SymlinkRefused.
    let meta_path = run_dir.join("run_meta.json");
    symlink(&decoy, &meta_path).unwrap();
    assert!(matches!(
        log.run_metadata("r1"),
        Err(StoreError::SymlinkRefused(_))
    ));
    fs::remove_file(&meta_path).unwrap();

    // Non-regular (directory) `run_meta.json` -> Io(other) from `open_regular_at`.
    fs::create_dir(&meta_path).unwrap();
    assert!(matches!(log.run_metadata("r1"), Err(StoreError::Io(_))));
    fs::remove_dir(&meta_path).unwrap();

    // Read-only `events.ndjson` (`0o444`) passes O_RDONLY verify then fails O_WRONLY|O_APPEND.
    let events_path = run_dir.join("events.ndjson");
    {
        let _restore = RestorePermissions(vec![(events_path.clone(), 0o644)]);
        fs::set_permissions(&events_path, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(matches!(
            log.append_envelopes("r1", (0, ZERO_HASH), std::slice::from_ref(&e1)),
            Err(StoreError::Io(_))
        ));
    }

    // Unreadable run directory (`0o000`) -> Io(PermissionDenied) from `open_dir_at`.
    let no_perm = root.join("r-noperm");
    fs::create_dir(&no_perm).unwrap();
    {
        let _restore = RestorePermissions(vec![(no_perm.clone(), 0o755)]);
        fs::set_permissions(&no_perm, fs::Permissions::from_mode(0o000)).unwrap();
        assert!(matches!(log.head("r-noperm"), Err(StoreError::Io(_))));
    }

    // 3. `open_or_create_lock_at` handles `AlreadyExists` race cleanly, and `openat` refuses
    // non-directory run_id as RunNotFound and symlink run_id as SymlinkRefused.
    let created_lock =
        crate::ports::fs_log::open_or_create_lock_at(&run_dir_fd, &lock_path).unwrap();
    drop(created_lock);
    let raced_lock = crate::ports::fs_log::open_or_create_lock_at(&run_dir_fd, &lock_path).unwrap();
    drop(raced_lock);

    fs::write(root.join("not-a-dir"), b"x").unwrap();
    assert!(matches!(
        log.head("not-a-dir"),
        Err(StoreError::RunNotFound(ref id)) if id == "not-a-dir"
    ));
    let outside_dir = dir.path().join("outside-run");
    fs::create_dir(&outside_dir).unwrap();
    symlink(&outside_dir, root.join("sym-run")).unwrap();
    assert!(matches!(
        log.head("sym-run"),
        Err(StoreError::SymlinkRefused(_))
    ));
    let nul_err = crate::ports::fs_openat::open_at(
        &run_dir_fd,
        "bad\0name",
        crate::ports::fs_openat::O_RDONLY,
    )
    .unwrap_err();
    assert_eq!(nul_err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn sqlite_projection_catches_deleted_interior_row_on_empty_and_replay_slices() {
    let dir = tempdir().unwrap();
    let index_path = dir.path().join("index.db");
    let proj = SqliteProjectionStore::open(&index_path).unwrap();
    let e1 = started_event("r-gap", "gap", "2026-11-26T10:00:00Z");
    let e2 = sealed_event(
        "r-gap",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-26T10:00:05Z",
    );
    let e3 = sealed_event(
        "r-gap",
        3,
        EventType::PhaseEntered,
        json!({"phase": "verify"}),
        &e2.event_hash,
        "2026-11-26T10:00:10Z",
    );
    proj.apply_envelopes("r-gap", None, &[e1.clone(), e2.clone(), e3.clone()])
        .unwrap();

    // Delete interior row seq = 2 while head_seq = 3 and head_hash = e3.event_hash stay intact.
    let raw = Connection::open(&index_path).unwrap();
    raw.execute(
        "DELETE FROM projected_events WHERE run_id = 'r-gap' AND seq = 2",
        [],
    )
    .unwrap();
    drop(raw);

    // Both empty batch and shorter replay slice [e1] (outside the gap) MUST report DivergentEventLog.
    assert_divergent(proj.apply_envelopes("r-gap", None, &[]), "r-gap");
    assert_divergent(
        proj.apply_envelopes("r-gap", None, std::slice::from_ref(&e1)),
        "r-gap",
    );

    // Equal-head full replay still repairs the missing interior row cleanly.
    proj.apply_envelopes("r-gap", None, &[e1.clone(), e2, e3.clone()])
        .unwrap();

    // Direct batch-reconciliation guard: if a custom backend's check_head passes while
    // verify_prefix reports missing prefix rows on a replay slice, reconcile still refuses.
    let stored = crate::ports::StoredProjectionRun {
        feature: "gap".into(),
        created_at: e1.recorded_at.clone(),
        head_seq: 3,
        head_hash: e3.event_hash.clone(),
    };
    let mut ctx = ();
    let res = crate::ports::reconcile_projection_batch(
        &mut ctx,
        "r-gap",
        None,
        std::slice::from_ref(&e1),
        Some(&stored),
        crate::ports::ProjectionBatchOps {
            check_head: |_, _, _| Ok(()),
            verify_prefix: |_, _, _| Ok(std::collections::BTreeSet::new()),
            insert_events: |_, _, _| Ok(()),
        },
    );
    assert!(matches!(
        res,
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-gap"
    ));
}
