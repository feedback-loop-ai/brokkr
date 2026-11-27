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
    lexically_normalize, normalize_storage_path, sync_dir, FsEventLog, RunMetadata,
    SqliteCoordinationStore, SqliteProjectionStore,
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
}

#[test]
fn fs_event_log_post_open_root_swap_readonly_dir_nonregular_and_sync_cleanup() {
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

    // Read-only runs/ directory: head, read_stream, manifest, run_metadata, list_run_ids all work.
    let run_dir = root.join("r1");
    let lock_path = run_dir.join(".lock");
    assert!(lock_path.is_file());
    {
        let _restore = RestorePermissions(vec![
            (root.clone(), 0o755),
            (run_dir.clone(), 0o755),
            (lock_path.clone(), 0o644),
            (run_dir.join("manifest.json"), 0o644),
            (run_dir.join("events.ndjson"), 0o644),
            (run_dir.join("run_meta.json"), 0o644),
        ]);
        for file in ["manifest.json", "events.ndjson", "run_meta.json", ".lock"] {
            fs::set_permissions(run_dir.join(file), fs::Permissions::from_mode(0o444)).unwrap();
        }
        fs::set_permissions(&run_dir, fs::Permissions::from_mode(0o555)).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();

        assert_eq!(log.head("r1").unwrap(), (1, e1.event_hash.clone()));
        assert_eq!(log.read_stream("r1", 1).unwrap().len(), 1);
        assert_eq!(log.manifest("r1").unwrap(), manifest);
        assert_eq!(log.run_metadata("r1").unwrap(), Some(meta.clone()));
        assert_eq!(log.list_run_ids().unwrap(), vec!["r1".to_string()]);
    }

    // Non-regular file (directory) refused on BOTH pre-open and post-open checks in open_no_symlink_with.
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

    // Concurrent `.create.lock` AlreadyExists race falls back to opening the peer's regular file.
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

    // sync_dir_result ignores macOS EINVAL and propagates other I/O errors.
    assert!(crate::ports::sync_dir_result(Err(std::io::Error::from(
        std::io::ErrorKind::InvalidInput
    )))
    .is_ok());
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

    // Post-open root swap: replacing `root` with a symlink refuses every operation with SymlinkRefused.
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

    // Replacing `root` with a different real directory (new dev/ino) fails closed with Io.
    fs::remove_file(&root).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(matches!(log.list_run_ids(), Err(StoreError::Io(_))));
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
    assert_eq!(uv, 1);

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
    assert!(matches!(
        proj.apply_envelopes("r-zero", None, &[]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-zero"
    ));
    let z1 = started_event("r-zero", "z", "2026-11-26T10:00:00Z");
    assert!(matches!(
        proj.apply_envelopes("r-zero", None, &[z1]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-zero"
    ));

    // Tampering head_hash when head_seq == 2 refuses BOTH empty batch and stale prefix [e1].
    raw.execute(
        "UPDATE projected_runs SET head_hash = 'tampered-head' WHERE run_id = 'r-prec'",
        [],
    )
    .unwrap();
    assert!(matches!(
        proj.apply_envelopes("r-prec", None, &[]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-prec"
    ));
    assert!(matches!(
        proj.apply_envelopes("r-prec", None, std::slice::from_ref(&e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-prec"
    ));
    raw.execute(
        "UPDATE projected_runs SET head_hash = ?1 WHERE run_id = 'r-prec'",
        rusqlite::params![&e2.event_hash],
    )
    .unwrap();

    // Tampering non-hash projected columns (event_type, recorded_at, payload) refuses with DivergentEventLog.
    for (col, bad_val) in [
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
        assert!(matches!(
            proj.apply_envelopes("r-prec", None, std::slice::from_ref(&e1)),
            Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-prec"
        ));
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
        "INSERT INTO projected_events (run_id, seq, event_type, event_hash, recorded_at, payload)
         VALUES ('r-orph', 3, 'run/completed', 'orphan-hash', '2026-11-26T10:00:10Z', '{}')",
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
}
