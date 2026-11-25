//! Round 5 regression tests for decision 0126 Slice 1 (#1362):
//! decoupled forked-chain refusal, store-wide co-located vs. decoupled
//! exclusivity, `folded_seq` + `head_hash` + SQL column verification,
//! concurrent decoupled `rebuild_from` preservation, decoupled feature
//! preservation, and co-located `list_runs` tie-breaking parity.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tempfile::tempdir;

use super::regressions::{sealed_event, sqlite_table_count};
use super::round3::{started_event, summary_snapshot};
use super::round4::completed_vs_phase_pair;
use super::{sample_manifest, MemEventLog};
use crate::ports::{EventLog, ProjectionStore, SqlitePorts};
use crate::{Store, StoreError};

fn summary_hash(path: &std::path::Path, run_id: &str) -> String {
    let conn = Connection::open(path).unwrap();
    conn.query_row(
        "SELECT head_hash FROM run_summary WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )
    .unwrap()
}

fn event_index_types(path: &std::path::Path, run_id: &str) -> Vec<(i64, String)> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT seq, type FROM event_index WHERE run_id = ?1 ORDER BY seq")
        .unwrap();
    let rows = stmt
        .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    rows
}

#[test]
fn decoupled_apply_envelopes_refuses_forked_chains_and_divergent_prefixes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("dec-fork.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let (e1, e2_completed, e2_phase) = completed_vs_phase_pair("r1", "fork-guard");
    let e3_phase = sealed_event(
        "r1",
        3,
        EventType::PhaseEntered,
        json!({"phase": "verify"}),
        &e2_phase.event_hash,
        "2026-11-25T10:00:10Z",
    );

    ports
        .apply_envelopes("r1", &[e1.clone(), e2_completed.clone()])
        .unwrap();

    assert!(matches!(
        ports.apply_envelopes("r1", &[e1.clone(), e2_phase.clone(), e3_phase]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));
    assert!(matches!(
        ports.apply_envelopes("r1", &[e1.clone(), e2_phase]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));
    assert_eq!(
        summary_snapshot(&path, "r1").2.as_deref(),
        Some("completed")
    );
    assert_eq!(
        event_index_types(&path, "r1"),
        vec![
            (1, "run/started".to_string()),
            (2, "run/completed".to_string()),
        ]
    );

    ports
        .apply_envelopes("r1", std::slice::from_ref(&e1))
        .unwrap();
    assert_eq!(summary_snapshot(&path, "r1").0, 2);
    assert_eq!(
        summary_snapshot(&path, "r1").2.as_deref(),
        Some("completed")
    );

    let (r2_e1, r2_e2_done, r2_e2_phase) = completed_vs_phase_pair("r2", "short-fork");
    let r2_e3 = sealed_event(
        "r2",
        3,
        EventType::RunCompleted,
        json!({}),
        &r2_e2_phase.event_hash,
        "2026-11-25T10:00:10Z",
    );
    ports
        .apply_envelopes("r2", &[r2_e1.clone(), r2_e2_phase, r2_e3])
        .unwrap();
    assert!(matches!(
        ports.apply_envelopes("r2", &[r2_e1, r2_e2_done]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r2"
    ));
    assert_eq!(summary_snapshot(&path, "r2").0, 3);
    assert_eq!(
        event_index_types(&path, "r2"),
        vec![
            (1, "run/started".to_string()),
            (2, "phase/entered".to_string()),
            (3, "run/completed".to_string()),
        ]
    );
}

#[test]
fn store_wide_colocated_and_decoupled_modes_are_mutually_exclusive() {
    let dir = tempdir().unwrap();

    let coloc_path = dir.path().join("coloc-only.db");
    let coloc_store = Store::open(&coloc_path).unwrap();
    let coloc_ports = SqlitePorts::new(&coloc_store);
    let manifest = sample_manifest("exclusive");
    let local_e1 = started_event("local-1", "exclusive", "2026-11-25T10:00:00Z");
    coloc_ports
        .import_run_stream("local-1", &manifest, &[local_e1])
        .unwrap();
    let foreign_e1 = started_event("foreign-1", "foreign", "2026-11-25T10:01:00Z");
    assert!(matches!(
        coloc_ports.apply_envelopes("foreign-1", &[foreign_e1]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "foreign-1"
    ));
    assert_eq!(coloc_ports.list_runs().unwrap().len(), 1);
    assert_eq!(coloc_ports.list_runs().unwrap()[0].0, "local-1");

    let dec_path = dir.path().join("dec-only.db");
    let dec_store = Store::open(&dec_path).unwrap();
    let dec_ports = SqlitePorts::new(&dec_store);
    let (dec_e1, dec_e2_done, dec_e2_phase) = completed_vs_phase_pair("dec-1", "exclusive");
    dec_ports
        .apply_envelopes("dec-1", &[dec_e1.clone(), dec_e2_phase])
        .unwrap();
    assert!(matches!(
        dec_ports.create_run("dec-2", &manifest),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "dec-2"
    ));
    assert!(matches!(
        dec_ports.import_run_stream("dec-1", &manifest, &[dec_e1, dec_e2_done]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "dec-1"
    ));
    assert_eq!(sqlite_table_count(&dec_path, "runs"), 0);
    assert_eq!(sqlite_table_count(&dec_path, "events"), 0);
    assert_eq!(
        summary_snapshot(&dec_path, "dec-1").2.as_deref(),
        Some("running")
    );
}

#[test]
fn colocated_apply_envelopes_folds_unfolded_summary_and_repairs_stale_head_hash() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unfolded.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("unfolded");
    let (e1, e2_completed, _) = completed_vs_phase_pair("r1", "unfolded");

    ports
        .import_run_stream("r1", &manifest, &[e1.clone(), e2_completed.clone()])
        .unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE run_summary SET folded_seq = 0 WHERE run_id = 'r1'",
        [],
    )
    .unwrap();
    assert_eq!(summary_snapshot(&path, "r1").1, Some(0));

    ports
        .apply_envelopes("r1", std::slice::from_ref(&e1))
        .unwrap();
    let (head_seq, folded_seq, status, _, _) = summary_snapshot(&path, "r1");
    assert_eq!((head_seq, folded_seq), (2, Some(2)));
    assert_eq!(status.as_deref(), Some("completed"));
    assert_eq!(summary_hash(&path, "r1"), e2_completed.event_hash);

    raw.execute(
        "UPDATE run_summary SET head_hash = 'stale-hash', status = 'running' WHERE run_id = 'r1'",
        [],
    )
    .unwrap();
    drop(raw);
    ports
        .apply_envelopes("r1", &[e1, e2_completed.clone()])
        .unwrap();
    assert_eq!(
        summary_snapshot(&path, "r1").2.as_deref(),
        Some("completed")
    );
    assert_eq!(summary_hash(&path, "r1"), e2_completed.event_hash);

    let dec_path = dir.path().join("dec-unfolded.db");
    let dec_store = Store::open(&dec_path).unwrap();
    let dec_ports = SqlitePorts::new(&dec_store);
    let (d1, d2, _) = completed_vs_phase_pair("d1", "dec-unfolded");
    dec_ports.apply_envelopes("d1", &[d1.clone(), d2]).unwrap();
    let dec_raw = Connection::open(&dec_path).unwrap();
    dec_raw
        .execute(
            "UPDATE run_summary SET folded_seq = 0 WHERE run_id = 'd1'",
            [],
        )
        .unwrap();
    drop(dec_raw);
    let mem = MemEventLog::default();
    mem.import_run_stream("d1", &manifest, &[d1]).unwrap();
    assert_eq!(dec_ports.rebuild_from(&mem).unwrap(), 1);
    assert_eq!(summary_snapshot(&dec_path, "d1").0, 1);
    assert_eq!(summary_snapshot(&dec_path, "d1").1, Some(1));
}

#[test]
fn colocated_apply_envelopes_and_rebuild_refuse_tampered_sql_event_columns() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sql-tamper.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("sql-tamper");
    let (e1, e2_completed, _) = completed_vs_phase_pair("t1", "sql-tamper");

    ports
        .import_run_stream("t1", &manifest, &[e1.clone(), e2_completed.clone()])
        .unwrap();
    ports
        .apply_envelopes("t1", &[e1.clone(), e2_completed.clone()])
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TRIGGER events_append_only_update;")
        .unwrap();
    raw.execute(
        "UPDATE events SET event_hash = 'forged-sql-hash' WHERE run_id = 't1' AND seq = 2",
        [],
    )
    .unwrap();
    assert!(matches!(
        ports.apply_envelopes("t1", &[e1.clone(), e2_completed.clone()]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "t1"
    ));
    assert!(matches!(
        ports.rebuild_from(&ports),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "t1"
    ));

    raw.execute(
        "UPDATE events SET event_hash = ?1, seq = 99 WHERE run_id = 't1' AND seq = 2",
        params![e2_completed.event_hash],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        ports.apply_envelopes("t1", &[e1, e2_completed]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "t1"
    ));
    assert!(matches!(
        ports.rebuild_from(&ports),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "t1"
    ));
}

struct ConcurrentWriterLog<'a> {
    inner: MemEventLog,
    target: &'a Store,
    injected_run: (&'static str, EventEnvelope),
}

impl EventLog for ConcurrentWriterLog<'_> {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        self.inner.create_run(run_id, manifest)
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        self.inner
            .append_envelopes(run_id, expected_head, envelopes)
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        self.inner.import_run_stream(run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        self.inner.head(run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        self.inner.read_stream(run_id, from_seq)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        self.inner.manifest(run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        let ports = SqlitePorts::new(self.target);
        ports.apply_envelopes(
            self.injected_run.0,
            std::slice::from_ref(&self.injected_run.1),
        )?;
        self.inner.list_run_ids()
    }
}

#[test]
fn concurrent_decoupled_rebuild_preserves_run_created_during_scan() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("concurrent-rebuild.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let stale_e1 = started_event("stale-1", "stale", "2026-11-25T09:00:00Z");
    let kept_e1 = started_event("kept-1", "kept", "2026-11-25T10:00:00Z");
    let new_e1 = started_event("new-run", "fresh", "2026-11-25T11:00:00Z");
    ports.apply_envelopes("stale-1", &[stale_e1]).unwrap();
    ports
        .apply_envelopes("kept-1", std::slice::from_ref(&kept_e1))
        .unwrap();

    let inner = MemEventLog::default();
    inner
        .import_run_stream("kept-1", &sample_manifest("kept"), &[kept_e1])
        .unwrap();
    let scanning_log = ConcurrentWriterLog {
        inner,
        target: &store,
        injected_run: ("new-run", new_e1),
    };

    assert_eq!(ports.rebuild_from(&scanning_log).unwrap(), 1);
    assert_eq!(
        ports.list_runs().unwrap(),
        vec![
            (
                "kept-1".to_string(),
                "kept".to_string(),
                "2026-11-25T10:00:00Z".to_string(),
            ),
            (
                "new-run".to_string(),
                "fresh".to_string(),
                "2026-11-25T11:00:00Z".to_string(),
            ),
        ]
    );
}

#[test]
fn decoupled_apply_envelopes_preserves_existing_feature_when_started_omits_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("feature-preserve.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let e1_no_feature = sealed_event(
        "f-1",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    let e2_phase = sealed_event(
        "f-1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1_no_feature.event_hash,
        "2026-11-25T10:00:05Z",
    );

    let mem = MemEventLog::default();
    mem.import_run_stream(
        "f-1",
        &sample_manifest("from-manifest"),
        std::slice::from_ref(&e1_no_feature),
    )
    .unwrap();
    assert_eq!(ports.rebuild_from(&mem).unwrap(), 1);
    assert_eq!(ports.list_runs().unwrap()[0].1, "from-manifest");

    ports
        .apply_envelopes("f-1", &[e1_no_feature, e2_phase])
        .unwrap();
    assert_eq!(ports.list_runs().unwrap()[0].1, "from-manifest");
    assert_eq!(summary_snapshot(&path, "f-1").0, 2);

    let blank_e1 = sealed_event(
        "f-blank",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-25T10:01:00Z",
    );
    ports.apply_envelopes("f-blank", &[blank_e1]).unwrap();
    assert_eq!(ports.list_runs().unwrap()[1].1, "");
}

#[test]
fn colocated_list_runs_matches_store_tie_breaking_on_equal_created_at() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tie-break.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let ts = "2026-11-25T10:00:00Z";

    let ez = started_event("run-z", "first", ts);
    let ea = started_event("run-a", "second", ts);
    ports
        .import_run_stream(
            "run-z",
            &sample_manifest("first"),
            std::slice::from_ref(&ez),
        )
        .unwrap();
    ports
        .import_run_stream(
            "run-a",
            &sample_manifest("second"),
            std::slice::from_ref(&ea),
        )
        .unwrap();
    ports.apply_envelopes("run-z", &[ez]).unwrap();
    ports.apply_envelopes("run-a", &[ea]).unwrap();

    let expected = vec![
        ("run-z".to_string(), "first".to_string(), ts.to_string()),
        ("run-a".to_string(), "second".to_string(), ts.to_string()),
    ];
    assert_eq!(store.list_runs().unwrap(), expected);
    assert_eq!(ports.list_runs().unwrap(), expected);

    assert_eq!(ports.rebuild_from(&ports).unwrap(), 2);
    assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());
}
