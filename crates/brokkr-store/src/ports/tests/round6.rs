//! Round 6 regression tests for decision 0126 Slice 1 (#1362):
//! decoupled per-event `projection_hashes` prefix & ahead-rebuild verification,
//! inherent `Store` writer exclusivity on decoupled projection DBs, co-located
//! empty-batch ownership & empty-import folding, unfolded decoupled prefix
//! non-regression, empty-stream stale projection cleanup, and co-located
//! `list_runs` parity before first fold.

use std::path::Path;

use rusqlite::Connection;

use super::regressions::{sealed_event, sqlite_table_count};
use super::round3::{started_event, summary_snapshot};
use super::round4::completed_vs_phase_pair;
use super::*;

fn projection_hashes(path: &Path, run_id: &str) -> Vec<(i64, String)> {
    query_run_pairs(
        path,
        "SELECT seq, event_hash FROM projection_hashes WHERE run_id = ?1 ORDER BY seq",
        run_id,
    )
}

fn phase_event_with_id(
    run_id: &str,
    seq: u64,
    event_id: &str,
    phase: &str,
    prev_hash: &str,
    recorded_at: &str,
) -> EventEnvelope {
    EventEnvelope {
        run_id: run_id.to_string(),
        seq,
        event_id: event_id.to_string(),
        event_schema_version: 1,
        event_type: EventType::PhaseEntered,
        payload: json!({"phase": phase}),
        causation_id: None,
        correlation_id: run_id.to_string(),
        attempt_id: None,
        recorded_at: recorded_at.to_string(),
        previous_hash: prev_hash.to_string(),
        event_hash: String::new(),
    }
    .sealed()
}

#[test]
fn decoupled_apply_envelopes_refuses_resealed_shorter_prefix_with_identical_index_fields() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("prefix-hash.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let e1 = started_event("r1", "prefix-hash", "2026-11-25T10:00:00Z");
    let e2 = phase_event_with_id(
        "r1",
        2,
        "same-event-id-2",
        "implement",
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    let e3 = sealed_event(
        "r1",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2.event_hash,
        "2026-11-25T10:00:10Z",
    );
    ports
        .apply_envelopes("r1", &[e1.clone(), e2.clone(), e3.clone()])
        .unwrap();
    assert_eq!(
        projection_hashes(&path, "r1"),
        vec![
            (1, e1.event_hash.clone()),
            (2, e2.event_hash.clone()),
            (3, e3.event_hash.clone()),
        ]
    );

    // Same seq, event_type, attempt_id, member, recorded_at, and event_id — only payload differs.
    let e2_resealed = phase_event_with_id(
        "r1",
        2,
        "same-event-id-2",
        "verify",
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    assert_ne!(e2.event_hash, e2_resealed.event_hash);
    assert!(matches!(
        ports.apply_envelopes("r1", &[e1.clone(), e2_resealed]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));

    // Authentic shorter prefix succeeds without regressing head_seq or hashes.
    ports.apply_envelopes("r1", &[e1, e2]).unwrap();
    assert_eq!(summary_snapshot(&path, "r1").0, 3);
    assert_eq!(projection_hashes(&path, "r1").len(), 3);
}

#[test]
fn decoupled_rebuild_from_refuses_forked_behind_log_against_ahead_projection() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ahead-fork.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let (e1, e2_completed, e2_phase) = completed_vs_phase_pair("r1", "ahead-fork");
    let e3 = sealed_event(
        "r1",
        3,
        EventType::RunCompleted,
        json!({}),
        &e2_phase.event_hash,
        "2026-11-25T10:00:10Z",
    );
    ports
        .apply_envelopes("r1", &[e1.clone(), e2_phase.clone(), e3])
        .unwrap();

    let forked = MemEventLog::default();
    forked
        .import_run_stream(
            "r1",
            &sample_manifest("ahead-fork"),
            &[e1.clone(), e2_completed],
        )
        .unwrap();
    assert!(matches!(
        ports.rebuild_from(&forked),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));
    assert_eq!(summary_snapshot(&path, "r1").0, 3);
    assert_eq!(projection_hashes(&path, "r1").len(), 3);

    let matching = MemEventLog::default();
    matching
        .import_run_stream("r1", &sample_manifest("ahead-fork"), &[e1, e2_phase])
        .unwrap();
    assert_eq!(ports.rebuild_from(&matching).unwrap(), 2);
    assert_eq!(summary_snapshot(&path, "r1").0, 3);
}

#[test]
fn inherent_store_writers_refuse_decoupled_projection_db() {
    let dir = tempdir().unwrap();
    let dec_path = dir.path().join("decoupled.db");
    let mut dec_store = Store::open(&dec_path).unwrap();
    let manifest = sample_manifest("exclusive");

    let e1 = started_event("dec-1", "exclusive", "2026-11-25T10:00:00Z");
    SqlitePorts::new(&dec_store)
        .apply_envelopes("dec-1", &[e1])
        .unwrap();

    assert!(matches!(
        dec_store.create_run("native-1", "exclusive", "bundle", &manifest),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "native-1"
    ));
    assert!(matches!(
        dec_store.start_run(
            "native-2",
            "exclusive",
            "bundle",
            &manifest,
            started_payload("exclusive"),
            None,
        ),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "native-2"
    ));
    assert!(matches!(
        SqlitePorts::new(&dec_store).import_run_stream("imp-empty", &manifest, &[]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "imp-empty"
    ));

    let src_path = dir.path().join("source.db");
    let mut src_store = Store::open(&src_path).unwrap();
    src_store
        .start_run(
            "imp-1",
            "exclusive",
            "bundle",
            &manifest,
            started_payload("exclusive"),
            None,
        )
        .unwrap();
    let ndjson = src_store.export_ndjson("imp-1").unwrap();
    assert!(matches!(
        dec_store.import_run(&ndjson, &manifest, Path::new("imp-1.ndjson")),
        Err(ImportError::Store(StoreError::DivergentEventLog { ref run_id })) if run_id == "imp-1"
    ));
    assert_eq!(sqlite_table_count(&dec_path, "runs"), 0);
    assert_eq!(sqlite_table_count(&dec_path, "events"), 0);
}

#[test]
fn colocated_empty_apply_refuses_foreign_run_and_empty_import_seeds_folded_summary() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty-guards.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("empty-guards");

    ports.create_run("local-1", &manifest).unwrap();
    assert!(matches!(
        ports.apply_envelopes("foreign-1", &[]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "foreign-1"
    ));
    ports.apply_envelopes("local-1", &[]).unwrap();

    ports
        .import_run_stream("empty-imp", &sample_manifest("empty-feat"), &[])
        .unwrap();
    let (head_seq, folded_seq, _, _, _) = summary_snapshot(&path, "empty-imp");
    assert_eq!((head_seq, folded_seq), (0, Some(0)));
    assert!(!store.started_here("empty-imp").unwrap());
    assert!(store.summary_disagreements().unwrap().is_empty());

    let raw = Connection::open(&path).unwrap();
    let seat_head: (i64, String) = raw
        .query_row(
            "SELECT folded_seq, folded_hash FROM seat_head WHERE run_id = 'empty-imp'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(seat_head, (0, ZERO_HASH.to_string()));
    let lag_rows: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM seat_lag WHERE run_id = 'empty-imp'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lag_rows, 0);
}

#[test]
fn decoupled_apply_envelopes_preserves_unfolded_head_on_shorter_prefix_and_folds_at_same_head() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unfolded-prefix.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let (e1, e2, _) = completed_vs_phase_pair("u1", "unfolded-prefix");
    ports
        .apply_envelopes("u1", &[e1.clone(), e2.clone()])
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE run_summary SET folded_seq = 0 WHERE run_id = 'u1'",
        [],
    )
    .unwrap();
    drop(raw);

    // Shorter matching prefix must not regress head_seq from 2 down to 1.
    ports
        .apply_envelopes("u1", std::slice::from_ref(&e1))
        .unwrap();
    let (head_seq, folded_seq, _, _, _) = summary_snapshot(&path, "u1");
    assert_eq!((head_seq, folded_seq), (2, Some(0)));

    // Shorter divergent prefix must still refuse.
    let e1_bad = started_event("u1", "different-feature", "2026-11-25T10:00:00Z");
    assert!(matches!(
        ports.apply_envelopes("u1", &[e1_bad]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "u1"
    ));

    // Same-head stream re-folds without regression.
    ports.apply_envelopes("u1", &[e1, e2]).unwrap();
    let (head_seq, folded_seq, status, _, _) = summary_snapshot(&path, "u1");
    assert_eq!((head_seq, folded_seq), (2, Some(2)));
    assert_eq!(status.as_deref(), Some("completed"));
}

#[test]
fn decoupled_rebuild_from_clears_omitted_stale_projection() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty-stale.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let stale_e1 = started_event("r-omitted", "stale", "2026-11-25T09:00:00Z");
    let live_e1 = started_event("r-live", "live", "2026-11-25T10:00:00Z");
    ports.apply_envelopes("r-omitted", &[stale_e1]).unwrap();
    ports
        .apply_envelopes("r-live", std::slice::from_ref(&live_e1))
        .unwrap();

    let log = MemEventLog::default();
    log.import_run_stream("r-live", &sample_manifest("live"), &[live_e1])
        .unwrap();

    assert_eq!(ports.rebuild_from(&log).unwrap(), 1);
    assert_eq!(
        ports.list_runs().unwrap(),
        vec![(
            "r-live".to_string(),
            "live".to_string(),
            "2026-11-25T10:00:00Z".to_string(),
        )]
    );
    assert!(projection_hashes(&path, "r-omitted").is_empty());
    assert_eq!(sqlite_table_count(&path, "run_summary"), 1);
    assert_eq!(sqlite_table_count(&path, "event_index"), 1);
    assert_eq!(sqlite_table_count(&path, "projection_hashes"), 1);
}

#[test]
fn colocated_list_runs_matches_store_before_and_after_first_fold() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("list-before-fold.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let ts = "2026-11-25T10:00:00Z";

    let ez = started_event("run-z", "feat-z", ts);
    let ea = started_event("run-a", "feat-a", ts);
    ports
        .import_run_stream(
            "run-z",
            &sample_manifest("feat-z"),
            std::slice::from_ref(&ez),
        )
        .unwrap();
    ports
        .import_run_stream(
            "run-a",
            &sample_manifest("feat-a"),
            std::slice::from_ref(&ea),
        )
        .unwrap();

    // Before first fold: run_summary feature/created_at are still NULL, yet ports.list_runs()
    // must match store.list_runs() 100%.
    let expected = vec![
        ("run-z".to_string(), "feat-z".to_string(), ts.to_string()),
        ("run-a".to_string(), "feat-a".to_string(), ts.to_string()),
    ];
    assert_eq!(store.list_runs().unwrap(), expected);
    assert_eq!(ports.list_runs().unwrap(), expected);

    ports.apply_envelopes("run-z", &[ez]).unwrap();
    ports.apply_envelopes("run-a", &[ea]).unwrap();
    assert_eq!(ports.list_runs().unwrap(), expected);
}

#[test]
fn schema_repair_restores_dropped_projection_hashes_table() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("repair-hashes.db");
    let store = Store::open(&path).unwrap();
    drop(store);

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TABLE projection_hashes;").unwrap();
    drop(raw);

    let reopened = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&reopened);
    let e1 = started_event("rep-1", "repaired", "2026-11-25T10:00:00Z");
    ports
        .apply_envelopes("rep-1", std::slice::from_ref(&e1))
        .unwrap();
    assert_eq!(projection_hashes(&path, "rep-1"), vec![(1, e1.event_hash)]);
}
