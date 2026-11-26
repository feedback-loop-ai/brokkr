//! Round 10 regression tests for decision 0126 Slice 1 (#1362):
//! `NOT EXISTS` decoupling check under NULL `runs.run_id`, listed empty-stream
//! anchor preservation in `rebuild_decoupled`, non-rewriting `Skip` on healthy
//! Current runs, stale non-NULL `facts_seq` and stray nonpositive hash-row
//! healing on same-head `apply_envelopes`, legacy read-only `list_runs`
//! fallback without `run_summary`, and seq-1-only `started_feature_of`.

use super::regressions::sealed_event;
use super::round3::{started_event, summary_snapshot};
use super::round7::{exec_raw_sql, projection_hash_rows};
use super::round8::checkpoint_event;
use super::round9::{
    assert_decoupled_refuses_after_sql, checkpoint_facts_seq, mem_log_with, three_step_stream,
};
use super::*;

#[test]
fn null_runs_row_does_not_mask_runless_decoupled_projection() {
    assert_decoupled_refuses_after_sql(
        "null-run-id.db",
        "null-mask",
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES (NULL, 'ghost', 'self', '{}', '2026-11-25T10:00:00Z');",
    );
}

#[test]
fn decoupled_rebuild_refuses_listed_empty_stream_and_preserves_anchors() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty-anchor.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let e1 = started_event("r", "anchored", "2026-11-25T10:00:00Z");
    ports
        .apply_envelopes("r", std::slice::from_ref(&e1))
        .unwrap();

    let empty_listed = MemEventLog::default();
    empty_listed
        .create_run("r", &sample_manifest("anchored"))
        .unwrap();
    assert!(matches!(
        ports.rebuild_from(&empty_listed),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r"
    ));
    assert_eq!(summary_snapshot(&path, "r").0, 1);
    assert_eq!(projection_hash_rows(&path, "r"), vec![(1, e1.event_hash)]);

    let conflicting_e1 = started_event("r", "forged", "2026-11-25T10:05:00Z");
    assert!(matches!(
        ports.apply_envelopes("r", &[conflicting_e1]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r"
    ));
}

#[test]
fn decoupled_rebuild_skips_healthy_current_run_without_rewriting_rows() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("no-rewrite-current.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let (e1, e2, e3) = three_step_stream("r-cur", "cur-feat");
    let stream = [e1, e2, e3];
    ports.apply_envelopes("r-cur", &stream).unwrap();

    exec_raw_sql(
        &path,
        "CREATE TRIGGER guard_summary_del BEFORE DELETE ON run_summary
         BEGIN SELECT RAISE(ABORT, 'must not delete run_summary'); END;
         CREATE TRIGGER guard_summary_upd BEFORE UPDATE ON run_summary
         BEGIN SELECT RAISE(ABORT, 'must not update run_summary'); END;
         CREATE TRIGGER guard_hashes_del BEFORE DELETE ON projection_hashes
         BEGIN SELECT RAISE(ABORT, 'must not delete projection_hashes'); END;
         CREATE TRIGGER guard_index_del BEFORE DELETE ON event_index
         BEGIN SELECT RAISE(ABORT, 'must not delete event_index'); END;
         CREATE TRIGGER guard_seat_del BEFORE DELETE ON seat_head
         BEGIN SELECT RAISE(ABORT, 'must not delete seat_head'); END;",
    );

    let log = mem_log_with("r-cur", "cur-feat", &stream);
    assert_eq!(ports.rebuild_from(&log).unwrap(), 3);
    assert_eq!(summary_snapshot(&path, "r-cur").0, 3);
}

#[test]
fn decoupled_same_head_apply_heals_stale_nonnull_facts_seq_and_stray_zero_hash() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("stale-facts-and-zero-hash.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let e1 = started_event("r-fix4", "fix4", "2026-11-25T10:00:00Z");
    let e2 = sealed_event(
        "r-fix4",
        2,
        EventType::EffectRequested,
        json!({"effect_id": "eff-1", "kind": "seat"}),
        &e1.event_hash,
        "2026-11-25T10:00:02Z",
    );
    let e3 = checkpoint_event("r-fix4", 3, &e2);
    let e4 = sealed_event(
        "r-fix4",
        4,
        EventType::RunParked,
        json!({"reason": "review"}),
        &e3.event_hash,
        "2026-11-25T10:00:06Z",
    );
    let e5 = sealed_event(
        "r-fix4",
        5,
        EventType::PhaseEntered,
        json!({"phase": "verify"}),
        &e4.event_hash,
        "2026-11-25T10:00:08Z",
    );
    let stream = [e1, e2, e3, e4, e5];
    ports.apply_envelopes("r-fix4", &stream).unwrap();
    assert_eq!(checkpoint_facts_seq(&path, "r-fix4", 3), Some(3));
    assert_eq!(checkpoint_facts_seq(&path, "r-fix4", 4), Some(5));

    exec_raw_sql(
        &path,
        "UPDATE event_index SET facts_seq = 5 WHERE run_id = 'r-fix4' AND seq = 3;",
    );
    ports.apply_envelopes("r-fix4", &stream).unwrap();
    assert_eq!(checkpoint_facts_seq(&path, "r-fix4", 3), Some(3));

    exec_raw_sql(
        &path,
        "UPDATE event_index SET facts_seq = 4 WHERE run_id = 'r-fix4' AND seq = 4;",
    );
    ports.apply_envelopes("r-fix4", &stream).unwrap();
    assert_eq!(checkpoint_facts_seq(&path, "r-fix4", 4), Some(5));

    exec_raw_sql(
        &path,
        "INSERT INTO projection_hashes (run_id, seq, event_hash) VALUES ('r-fix4', 0, 'stray');",
    );
    assert_eq!(projection_hash_rows(&path, "r-fix4").len(), 6);
    ports.apply_envelopes("r-fix4", &stream).unwrap();
    assert_eq!(projection_hash_rows(&path, "r-fix4").len(), 5);
    assert!(projection_hash_rows(&path, "r-fix4")
        .iter()
        .all(|(seq, _)| *seq >= 1));
}

#[test]
fn list_runs_handles_legacy_read_only_journal_without_run_summary_table() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy-ro.db");
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch(crate::schema::MIGRATION_V1).unwrap();
    raw.execute(
        "INSERT INTO meta (key, value) VALUES ('database_schema', '1')",
        [],
    )
    .unwrap();
    drop(raw);

    let ro_empty = Store::open_read_only(&path).unwrap();
    assert!(SqlitePorts::new(&ro_empty).list_runs().unwrap().is_empty());
    assert_eq!(
        SqlitePorts::new(&ro_empty).list_runs().unwrap(),
        ro_empty.list_runs().unwrap()
    );
    drop(ro_empty);

    exec_raw_sql(
        &path,
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES ('r-leg', 'legacy-feat', 'self', '{}', '2026-11-25T10:00:00Z');",
    );
    let ro_populated = Store::open_read_only(&path).unwrap();
    let expected = vec![(
        "r-leg".to_string(),
        "legacy-feat".to_string(),
        "2026-11-25T10:00:00Z".to_string(),
    )];
    assert_eq!(
        SqlitePorts::new(&ro_populated).list_runs().unwrap(),
        expected
    );
    assert_eq!(ro_populated.list_runs().unwrap(), expected);
}

#[test]
fn late_run_started_past_seq_one_never_changes_listed_feature() {
    let dir = tempdir().unwrap();
    let e1_phase = sealed_event(
        "r-late",
        1,
        EventType::PhaseEntered,
        json!({"phase": "plan"}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    let e2_late_started = sealed_event(
        "r-late",
        2,
        EventType::RunStarted,
        json!({"feature": "late-hijack"}),
        &e1_phase.event_hash,
        "2026-11-25T10:00:05Z",
    );
    assert_eq!(
        super::super::sqlite::started_feature_of(std::slice::from_ref(&e2_late_started)),
        None
    );

    let stream = [e1_phase, e2_late_started];
    let dec = Store::open(&dir.path().join("late-dec.db")).unwrap();
    let dec_ports = SqlitePorts::new(&dec);
    dec_ports.apply_envelopes("r-late", &stream).unwrap();
    assert_eq!(dec_ports.list_runs().unwrap()[0].1, "");

    let coloc = Store::open(&dir.path().join("late-coloc.db")).unwrap();
    let coloc_ports = SqlitePorts::new(&coloc);
    coloc_ports
        .import_run_stream("r-late", &sample_manifest("manifest-feat"), &stream)
        .unwrap();
    assert_eq!(coloc_ports.list_runs().unwrap()[0].1, "manifest-feat");
    assert_eq!(coloc_ports.rebuild_from(&coloc_ports).unwrap(), 2);
    assert_eq!(coloc_ports.list_runs().unwrap()[0].1, "manifest-feat");
}
