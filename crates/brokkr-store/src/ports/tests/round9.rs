//! Round 9 regression tests for decision 0126 Slice 1 (#1362):
//! unified `plan_decoupled_projection` anchor checks across `apply_envelopes`
//! and `rebuild_decoupled`, ahead-skip monotonicity on trigger-dropped DBs,
//! same-head healing of dropped `event_index` / stale checkpoint `facts_seq` /
//! `seat_summary`, unconditional runless `run_summary` / `projection_hashes`
//! refusal even when unrelated local `runs` exist, and `RunStarted` feature
//! precedence over `manifest["feature"]` across co-located and decoupled modes.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use super::regressions::sealed_event;
use super::round3::{started_event, summary_snapshot};
use super::round4::completed_vs_phase_pair;
use super::round7::{exec_raw_sql, projection_hash_rows, sqlite_journal};
use super::round8::checkpoint_event;
use super::*;

pub(super) fn three_step_stream(
    run_id: &str,
    feature: &str,
) -> (EventEnvelope, EventEnvelope, EventEnvelope) {
    let (e1, _, e2_phase) = completed_vs_phase_pair(run_id, feature);
    let e3 = sealed_event(
        run_id,
        3,
        EventType::RunCompleted,
        json!({"outcome": "done"}),
        &e2_phase.event_hash,
        "2026-11-25T10:00:12Z",
    );
    (e1, e2_phase, e3)
}

pub(super) fn mem_log_with(run_id: &str, feature: &str, events: &[EventEnvelope]) -> MemEventLog {
    let log = MemEventLog::default();
    log.import_run_stream(run_id, &sample_manifest(feature), events)
        .unwrap();
    log
}

pub(super) fn checkpoint_facts_seq(path: &Path, run_id: &str, seq: i64) -> Option<i64> {
    Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT facts_seq FROM event_index WHERE run_id = ?1 AND seq = ?2",
            rusqlite::params![run_id, seq],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
        .flatten()
}

fn seat_head_seq(path: &Path, run_id: &str) -> Option<i64> {
    Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT folded_seq FROM seat_head WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
}

#[test]
fn decoupled_rebuild_refuses_fork_after_projection_hashes_drop() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("drop-hashes-fork.db");
    let (e1, e2, e3) = three_step_stream("r-drop-hash", "drop-hash");
    let e3_fork = sealed_event(
        "r-drop-hash",
        3,
        EventType::RunParked,
        json!({"reason": "forked"}),
        &e2.event_hash,
        "2026-11-25T10:00:13Z",
    );
    let forked = mem_log_with(
        "r-drop-hash",
        "drop-hash",
        &[e1.clone(), e2.clone(), e3_fork],
    );

    let store = Store::open(&path).unwrap();
    SqlitePorts::new(&store)
        .apply_envelopes("r-drop-hash", &[e1, e2, e3])
        .unwrap();
    drop(store);

    // (a) head_hash anchor refuses fork even when projection_hashes was dropped.
    exec_raw_sql(&path, "DROP TABLE projection_hashes;");
    let reopened = Store::open(&path).unwrap();
    assert!(matches!(
        SqlitePorts::new(&reopened).rebuild_from(&forked),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-drop-hash"
    ));
    drop(reopened);

    // (b) folded_hash anchor still refuses fork when head_seq was also reset to 0.
    exec_raw_sql(
        &path,
        "DROP TRIGGER run_summary_of_an_event; DROP TABLE projection_hashes;",
    );
    let reopened = Store::open(&path).unwrap();
    assert_eq!(summary_snapshot(&path, "r-drop-hash").0, 0);
    assert_eq!(summary_snapshot(&path, "r-drop-hash").1, Some(3));
    assert!(matches!(
        SqlitePorts::new(&reopened).rebuild_from(&forked),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-drop-hash"
    ));
}

#[test]
fn trigger_dropped_decoupled_rebuild_skips_shorter_prefix_and_refuses_unproven() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("trigger-drop-rebuild.db");
    let (e1, e2, e3) = three_step_stream("r-trig-reb", "trig-reb");
    let shorter = mem_log_with("r-trig-reb", "trig-reb", &[e1.clone(), e2.clone()]);

    let store = Store::open(&path).unwrap();
    SqlitePorts::new(&store)
        .apply_envelopes("r-trig-reb", &[e1, e2, e3])
        .unwrap();
    drop(store);

    exec_raw_sql(&path, "DROP TRIGGER run_summary_of_an_event;");
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        SqlitePorts::new(&reopened).rebuild_from(&shorter).unwrap(),
        2
    );
    assert_eq!(summary_snapshot(&path, "r-trig-reb").1, Some(3));
    assert_eq!(projection_hash_rows(&path, "r-trig-reb").len(), 3);
    drop(reopened);

    // Without projection_hashes, the shorter prefix cannot be proven against ahead folded_hash.
    exec_raw_sql(&path, "DROP TABLE projection_hashes;");
    let reopened = Store::open(&path).unwrap();
    assert!(matches!(
        SqlitePorts::new(&reopened).rebuild_from(&shorter),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-trig-reb"
    ));
}

#[test]
fn decoupled_same_head_apply_heals_event_index_checkpoint_facts_and_seat_head() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("same-head-heal.db");
    let e1 = started_event("r-heal", "heal", "2026-11-25T10:00:00Z");
    let e2 = sealed_event(
        "r-heal",
        2,
        EventType::EffectRequested,
        json!({"effect_id": "eff-1", "kind": "seat"}),
        &e1.event_hash,
        "2026-11-25T10:00:02Z",
    );
    let e3 = checkpoint_event("r-heal", 3, &e2);
    let stream = [e1, e2, e3];

    let store = Store::open(&path).unwrap();
    SqlitePorts::new(&store)
        .apply_envelopes("r-heal", &stream)
        .unwrap();
    drop(store);

    // Dropping the checkpoint trigger clears facts_seq on reopen; same-head replay heals it.
    exec_raw_sql(&path, "DROP TRIGGER event_index_checkpoint_behind;");
    let reopened = Store::open(&path).unwrap();
    assert_eq!(checkpoint_facts_seq(&path, "r-heal", 3), None);
    SqlitePorts::new(&reopened)
        .apply_envelopes("r-heal", &stream)
        .unwrap();
    assert_eq!(checkpoint_facts_seq(&path, "r-heal", 3), Some(3));
    drop(reopened);

    // Dropping event_index empties index_trail; same-head replay restores trail and facts_seq.
    exec_raw_sql(&path, "DROP TABLE event_index;");
    let reopened = Store::open(&path).unwrap();
    assert!(reopened.index_trail("r-heal").unwrap().is_empty());
    SqlitePorts::new(&reopened)
        .apply_envelopes("r-heal", &stream)
        .unwrap();
    assert_eq!(reopened.index_trail("r-heal").unwrap(), vec![1, 2]);
    assert_eq!(checkpoint_facts_seq(&path, "r-heal", 3), Some(3));
    drop(reopened);

    // Dropping seat_summary drops seat_head; same-head replay restores it.
    exec_raw_sql(&path, "DROP TABLE seat_summary;");
    let reopened = Store::open(&path).unwrap();
    assert_eq!(seat_head_seq(&path, "r-heal"), None);
    SqlitePorts::new(&reopened)
        .apply_envelopes("r-heal", &stream)
        .unwrap();
    assert_eq!(seat_head_seq(&path, "r-heal"), Some(3));
}

pub(super) fn assert_decoupled_refuses_after_sql(db_name: &str, feature: &str, sql: &str) {
    let dir = tempdir().unwrap();
    let path = dir.path().join(db_name);
    let store = Store::open(&path).unwrap();
    let manifest = sample_manifest(feature);
    let e1 = started_event("r", feature, "2026-11-25T10:00:00Z");
    let ports = SqlitePorts::new(&store);
    ports
        .apply_envelopes("r", std::slice::from_ref(&e1))
        .unwrap();
    exec_raw_sql(&path, sql);
    assert!(matches!(
        ports.create_run("r", &manifest),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r"
    ));
    assert!(matches!(
        ports.import_run_stream("r", &manifest, std::slice::from_ref(&e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r"
    ));
}

#[test]
fn decoupled_db_refuses_authoritative_writes_with_unrelated_local_run_or_secondary_only() {
    let dir = tempdir().unwrap();
    let manifest = sample_manifest("dec-mixed");
    let e1 = started_event("r", "dec-mixed", "2026-11-25T10:00:00Z");

    // Seed an unrelated local runs row: runless run_summary / projection_hashes must still refuse.
    assert_decoupled_refuses_after_sql(
        "mixed-decoupled.db",
        "dec-mixed",
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES ('other-local', 'local', 'self', '{}', '2026-11-25T10:00:00Z');",
    );

    // Empty runs/events with both run_summary and projection_hashes dropped still refuses via
    // secondary projection tables (event_index / seat_head).
    let sec_path = dir.path().join("secondary-only.db");
    let sec_store = Store::open(&sec_path).unwrap();
    SqlitePorts::new(&sec_store)
        .apply_envelopes("r", std::slice::from_ref(&e1))
        .unwrap();
    drop(sec_store);
    exec_raw_sql(
        &sec_path,
        "DROP TABLE run_summary; DROP TABLE projection_hashes;",
    );
    let reopened = Store::open(&sec_path).unwrap();
    assert!(matches!(
        SqlitePorts::new(&reopened).create_run("r", &manifest),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r"
    ));
}

fn listed_feature(ports: &SqlitePorts<'_>, run_id: &str) -> String {
    ports
        .list_runs()
        .unwrap()
        .into_iter()
        .find(|(id, _, _)| id == run_id)
        .unwrap()
        .1
}

#[test]
fn started_feature_overrides_manifest_feature_across_colocated_and_decoupled_modes() {
    let dir = tempdir().unwrap();
    let manifest = sample_manifest("manifest-default");

    // 1. Co-located via Journal + pre-existing Store::create_run with manifest-default.
    let coloc_path = dir.path().join("coloc-precedence.db");
    let mut coloc = Store::open(&coloc_path).unwrap();
    coloc
        .create_run("r-pre", "manifest-default", "self", &manifest)
        .unwrap();
    coloc
        .append_next(
            "r-pre",
            EventType::RunStarted,
            json!({"feature": "attested-goal"}),
            None,
            None,
        )
        .unwrap();
    let ports = SqlitePorts::new(&coloc);
    let direct_e1 = started_event("r-direct", "attested-goal", "2026-11-25T10:00:00Z");
    ports.create_run("r-direct", &manifest).unwrap();
    ports
        .append_envelopes("r-direct", (0, ZERO_HASH), &[direct_e1])
        .unwrap();
    assert_eq!(listed_feature(&ports, "r-direct"), "attested-goal");

    let journal = sqlite_journal(&coloc);
    journal.create_run("r-journal", &manifest).unwrap();
    journal
        .append_event(
            "r-journal",
            plain_event(
                EventType::RunStarted,
                json!({"feature": "attested-goal"}),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    assert_eq!(listed_feature(&ports, "r-journal"), "attested-goal");
    assert_eq!(journal.reindex().unwrap(), 3);
    let coloc_runs = ports.list_runs().unwrap();
    assert_eq!(coloc_runs.len(), 3);
    assert!(coloc_runs
        .iter()
        .all(|(_, feat, _)| feat == "attested-goal"));
    assert_eq!(coloc.list_runs().unwrap(), coloc_runs);
    assert!(coloc.summary_disagreements().unwrap().is_empty());

    // 2. Decoupled apply_envelopes + rebuild_from with manifest-default.
    let dec_path = dir.path().join("dec-precedence.db");
    let dec = Store::open(&dec_path).unwrap();
    let dec_ports = SqlitePorts::new(&dec);
    let e1 = started_event("r-dec", "attested-goal", "2026-11-25T10:00:00Z");
    dec_ports
        .apply_envelopes("r-dec", std::slice::from_ref(&e1))
        .unwrap();
    assert_eq!(dec_ports.list_runs().unwrap()[0].1, "attested-goal");
    let mem = mem_log_with("r-dec", "manifest-default", &[e1]);
    assert_eq!(dec_ports.rebuild_from(&mem).unwrap(), 1);
    assert_eq!(dec_ports.list_runs().unwrap()[0].1, "attested-goal");
}
