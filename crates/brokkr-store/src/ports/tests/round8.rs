//! Round 8 regression tests for decision 0126 Slice 1 (#1362):
//! `SeatRecordVersion` V1 fallback when `RunStarted` omits or mis-types
//! `/manifest/engine`, checking all 6 `PROJECTION_TABLES` for decoupled runs
//! after `DROP TABLE run_summary`, unconditional `projection_hashes` max-seq &
//! fork checks in `plan_decoupled_apply` and `rebuild_decoupled`, decoupled
//! runless checkpoint refold skip after `DROP TRIGGER
//! event_index_checkpoint_behind`, and unified `started_feature` across
//! legacy `manifest.feature` and pre-existing empty `runs.feature` on `reindex`.

use std::path::Path;

use rusqlite::Connection;

use super::regressions::sealed_event;
use super::round3::{started_event, summary_snapshot};
use super::round4::completed_vs_phase_pair;
use super::round7::{
    assert_v8_refused_under_v1, exec_raw_sql, projection_hash_rows, sqlite_journal,
};
use super::*;

fn summary_feature(path: &Path, run_id: &str) -> Option<String> {
    let conn = Connection::open(path).unwrap();
    conn.query_row(
        "SELECT feature FROM run_summary WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn journal_refuses_v8_record_when_run_started_omits_or_non_string_engine() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing-started-engine.db");
    let store = Store::open(&path).unwrap();
    let journal = sqlite_journal(&store);
    let v8_manifest = json!({"engine": "0.15.0", "bundle_name": "self"});

    for (run_id, started) in [
        ("r-no-engine", json!({"feature": "no-engine"})),
        (
            "r-num-engine",
            json!({"feature": "num-engine", "manifest": {"engine": 15}}),
        ),
    ] {
        journal.create_run(run_id, &v8_manifest).unwrap();
        journal
            .append_event(
                run_id,
                plain_event(EventType::RunStarted, started, None, "2026-11-25T10:00:00Z"),
            )
            .unwrap();
        assert_v8_refused_under_v1(&journal, run_id);
        assert!(journal.export_ndjson(run_id).is_ok());
    }
}

#[test]
fn decoupled_db_refuses_authoritative_writes_after_run_summary_drop() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("drop-run-summary.db");
    let store = Store::open(&path).unwrap();
    let manifest = sample_manifest("dec-drop");
    let e1 = started_event("dec-1", "dec-drop", "2026-11-25T10:00:00Z");

    SqlitePorts::new(&store)
        .apply_envelopes("dec-1", std::slice::from_ref(&e1))
        .unwrap();
    drop(store);

    exec_raw_sql(&path, "DROP TABLE run_summary;");

    let reopened = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&reopened);
    assert!(matches!(
        ports.create_run("dec-1", &manifest),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "dec-1"
    ));
    assert!(matches!(
        ports.import_run_stream("dec-1", &manifest, &[e1]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "dec-1"
    ));
}

#[test]
fn trigger_dropped_decoupled_db_keeps_progress_on_shorter_prefix_and_refuses_forked_rebuild() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("trigger-drop-seq3.db");
    let store = Store::open(&path).unwrap();
    let (e1, e2_fork, e2_phase) = completed_vs_phase_pair("r-trig3", "trig3");
    let e3 = sealed_event(
        "r-trig3",
        3,
        EventType::RunCompleted,
        json!({"outcome": "done"}),
        &e2_phase.event_hash,
        "2026-11-25T10:00:12Z",
    );
    SqlitePorts::new(&store)
        .apply_envelopes("r-trig3", &[e1.clone(), e2_phase.clone(), e3.clone()])
        .unwrap();
    drop(store);

    exec_raw_sql(&path, "DROP TRIGGER run_summary_of_an_event;");

    let reopened = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&reopened);
    let (head_before, folded_before, status_before, _, _) = summary_snapshot(&path, "r-trig3");
    assert_eq!((head_before, folded_before), (0, Some(3)));
    assert_eq!(status_before.as_deref(), Some("completed"));

    // (a) Shorter authentic prefix [e1, e2_phase] succeeds without regressing folded_seq or status.
    ports
        .apply_envelopes("r-trig3", &[e1.clone(), e2_phase.clone()])
        .unwrap();
    let (_, folded_after, status_after, _, _) = summary_snapshot(&path, "r-trig3");
    assert_eq!(folded_after, Some(3));
    assert_eq!(status_after.as_deref(), Some("completed"));

    // (b) rebuild_from with forked [e1, e2_fork] refuses with DivergentEventLog.
    let forked = MemEventLog::default();
    forked
        .import_run_stream("r-trig3", &sample_manifest("trig3"), &[e1.clone(), e2_fork])
        .unwrap();
    assert!(matches!(
        ports.rebuild_from(&forked),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-trig3"
    ));
    assert_eq!(projection_hash_rows(&path, "r-trig3").len(), 3);

    // Restore full stream, then verify ahead rebuild refuses if projection_hashes was wiped.
    ports
        .apply_envelopes("r-trig3", &[e1.clone(), e2_phase.clone(), e3])
        .unwrap();
    exec_raw_sql(
        &path,
        "DELETE FROM projection_hashes WHERE run_id = 'r-trig3';",
    );
    let shorter = MemEventLog::default();
    shorter
        .import_run_stream("r-trig3", &sample_manifest("trig3"), &[e1, e2_phase])
        .unwrap();
    assert!(matches!(
        ports.rebuild_from(&shorter),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-trig3"
    ));
}

pub(super) fn checkpoint_event(run_id: &str, seq: u64, prev: &EventEnvelope) -> EventEnvelope {
    EventEnvelope {
        run_id: run_id.to_string(),
        seq,
        event_id: format!("{run_id}-evt-{seq}"),
        event_schema_version: 1,
        event_type: EventType::EffectCheckpointed,
        payload: json!({
            "effect_id": "eff-1",
            "attempt_id": "att-1",
            "checkpoint": {"step": "claude-session-finished"}
        }),
        causation_id: Some(prev.event_id.clone()),
        correlation_id: run_id.to_string(),
        attempt_id: Some("att-1".to_string()),
        recorded_at: "2026-11-25T10:00:05Z".to_string(),
        previous_hash: prev.event_hash.clone(),
        event_hash: String::new(),
    }
    .sealed()
}

#[test]
fn decoupled_store_reopens_cleanly_after_checkpoint_behind_trigger_drop() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("decoupled-cp-trigger.db");
    let store = Store::open(&path).unwrap();
    let e1 = started_event("r-cp", "cp-trigger", "2026-11-25T10:00:00Z");
    let e2 = checkpoint_event("r-cp", 2, &e1);
    SqlitePorts::new(&store)
        .apply_envelopes("r-cp", &[e1, e2])
        .unwrap();
    drop(store);

    exec_raw_sql(&path, "DROP TRIGGER event_index_checkpoint_behind;");

    let reopened = Store::open(&path).unwrap();
    assert_eq!(SqlitePorts::new(&reopened).list_runs().unwrap().len(), 1);
    drop(reopened);
    let reopened_again = Store::open(&path).unwrap();
    assert_eq!(
        SqlitePorts::new(&reopened_again).list_runs().unwrap().len(),
        1
    );
}

#[test]
fn started_feature_handles_legacy_manifest_and_backfills_empty_runs_feature_on_reindex() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("started-feature.db");
    let mut store = Store::open(&path).unwrap();
    let manifest = json!({"engine": "0.15.0", "bundle_name": "self"});

    // Pre-existing co-located run with empty runs.feature and legacy payload["manifest"]["feature"].
    store.create_run("r-pre", "", "self", &manifest).unwrap();
    store
        .append_next(
            "r-pre",
            EventType::RunStarted,
            json!({"manifest": {"engine": "0.15.0", "feature": "from-reindex"}}),
            None,
            None,
        )
        .unwrap();
    // Co-located run with empty runs.feature and RunStarted omitting any feature stays empty.
    store.create_run("r-empty", "", "self", &manifest).unwrap();
    store
        .append_next("r-empty", EventType::RunStarted, json!({}), None, None)
        .unwrap();

    let ports = SqlitePorts::new(&store);
    let journal = sqlite_journal(&store);

    // Journal append with empty top-level "feature" falls back to legacy payload["manifest"]["feature"].
    journal.create_run("r-legacy", &manifest).unwrap();
    journal
        .append_event(
            "r-legacy",
            plain_event(
                EventType::RunStarted,
                json!({"feature": "", "manifest": {"engine": "0.15.0", "feature": "legacy-goal"}}),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    assert_eq!(
        summary_feature(&path, "r-legacy").as_deref(),
        Some("legacy-goal")
    );
    assert_eq!(
        store
            .list_runs()
            .unwrap()
            .into_iter()
            .find(|(id, _, _)| id == "r-pre")
            .unwrap()
            .1,
        ""
    );

    assert_eq!(journal.reindex().unwrap(), 3);
    let runs: BTreeMap<String, String> = ports
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|(id, feat, _)| (id, feat))
        .collect();
    assert_eq!(
        runs.get("r-legacy").map(String::as_str),
        Some("legacy-goal")
    );
    assert_eq!(runs.get("r-pre").map(String::as_str), Some("from-reindex"));
    assert_eq!(runs.get("r-empty").map(String::as_str), Some(""));
    assert_eq!(
        summary_feature(&path, "r-pre").as_deref(),
        Some("from-reindex")
    );
    assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());
    assert!(store.summary_disagreements().unwrap().is_empty());
}
