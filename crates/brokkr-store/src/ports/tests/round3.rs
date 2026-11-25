//! Round 3 regression tests for decision 0126 Slice 1 (#1362):
//! prefix verification on `Journal::append_checked`, full chain and
//! `RowBeforeChain` guards in `SqlitePorts`, divergent-log protection on
//! co-located `rebuild_from`, monotonic full-fold decoupled projections and
//! standing acknowledgements, empty-run and `created_at` consistency on
//! `rebuild_from`, and `import_ndjson` recovery via `reindex()`.

use std::cell::Cell;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::fold::Status;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tempfile::tempdir;

use super::regressions::{sealed_event, sqlite_table_count};
use super::{
    sample_manifest, started_payload, valid_checkpoint_payload, MemCoordinationStore, MemEventLog,
    MemProjectionStore,
};
use crate::ports::{EventLog, Journal, ProjectionStore, SqlitePorts};
use crate::{ImportError, Store, StoreError};

fn session_finished_checkpoint(attempt_id: &str, output_tokens: u64) -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": attempt_id,
        "checkpoint": {
            "step": "claude-session-finished",
            "output_tokens": output_tokens
        }
    })
}

fn started_event(run_id: &str, feature: &str, ts: &str) -> EventEnvelope {
    sealed_event(
        run_id,
        1,
        EventType::RunStarted,
        started_payload(feature),
        ZERO_HASH,
        ts,
    )
}

fn summary_snapshot(
    path: &std::path::Path,
    run_id: &str,
) -> (i64, Option<i64>, Option<String>, String, Option<i64>) {
    let conn = Connection::open(path).unwrap();
    conn.query_row(
        "SELECT head_seq, folded_seq, status, lane, last_at_ms
         FROM run_summary WHERE run_id = ?1",
        [run_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// HIGH #1: Verify existing prefix in Journal::append_checked
// ---------------------------------------------------------------------------

#[test]
fn journal_append_refuses_tampered_prefix_and_foreign_run_stream() {
    let mem = MemEventLog::default();
    let journal = Journal::new(
        &mem,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("run-a", &sample_manifest("alpha"))
        .unwrap();
    let e1 = journal
        .append_event(
            "run-a",
            EventType::RunStarted,
            started_payload("alpha"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();

    // Tamper existing seq-1 payload: both append_event and append_if_head must refuse.
    mem.tamper_payload("run-a", 0, json!({"feature": "evil"}));
    assert!(matches!(
        journal.append_event(
            "run-a",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            "2026-11-25T10:00:01Z",
        ),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));
    assert!(matches!(
        journal.append_if_head(
            "run-a",
            (1, &e1.event_hash),
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            "2026-11-25T10:00:02Z",
        ),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));
    assert_eq!(mem.read_stream("run-a", 1).unwrap().len(), 1);

    // Plant a foreign run_id envelope under run-b: both append_event and append_if_head refuse.
    let foreign_e1 = started_event("run-a", "alpha", "2026-11-25T10:00:00Z");
    mem.plant_raw_stream("run-b", sample_manifest("beta"), vec![foreign_e1.clone()]);
    assert!(matches!(
        journal.append_event(
            "run-b",
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            "2026-11-25T10:00:03Z",
        ),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert!(matches!(
        journal.append_if_head(
            "run-b",
            (1, &foreign_e1.event_hash),
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            "2026-11-25T10:00:04Z",
        ),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert_eq!(mem.read_stream("run-b", 1).unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// HIGH #2: Verify chain & RowBeforeChain inside SqlitePorts
// ---------------------------------------------------------------------------

#[test]
fn sqlite_append_envelopes_enforces_chain_integrity() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chain-guard.db");
    let store = Store::open(&db_path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("chain-guard");
    ports.create_run("cg-1", &manifest).unwrap();

    // Non-contiguous seq = 5 on empty run.
    let seq5 = sealed_event(
        "cg-1",
        5,
        EventType::RunStarted,
        started_payload("chain-guard"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        ports.append_envelopes("cg-1", (0, ZERO_HASH), std::slice::from_ref(&seq5)),
        Err(StoreError::Chain(ChainError::SeqGap {
            seq: 5,
            expected: 1
        }))
    ));

    // Forged event_hash.
    let mut forged = started_event("cg-1", "chain-guard", "2026-11-25T10:00:00Z");
    forged.event_hash = "0".repeat(64);
    assert!(matches!(
        ports.append_envelopes("cg-1", (0, ZERO_HASH), std::slice::from_ref(&forged)),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));

    // Wrong previous_hash.
    let wrong_prev = sealed_event(
        "cg-1",
        1,
        EventType::RunStarted,
        started_payload("chain-guard"),
        &"1".repeat(64),
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        ports.append_envelopes("cg-1", (0, ZERO_HASH), std::slice::from_ref(&wrong_prev)),
        Err(StoreError::Chain(ChainError::BrokenChain {
            seq: 1,
            prev_seq: 0
        }))
    ));

    // Wrong event_schema_version.
    let mut bad_ver = started_event("cg-1", "chain-guard", "2026-11-25T10:00:00Z");
    bad_ver.event_schema_version = 99;
    let bad_ver = bad_ver.sealed();
    assert!(matches!(
        ports.append_envelopes("cg-1", (0, ZERO_HASH), std::slice::from_ref(&bad_ver)),
        Err(StoreError::Chain(ChainError::BadSchemaVersion {
            seq: 1,
            found: 99
        }))
    ));

    // 2-event batch: seq 1 local, seq 2 sealed for foreign run_id -> refuse ForeignRun at seq 2.
    let local_e1 = started_event("cg-1", "chain-guard", "2026-11-25T10:00:00Z");
    let foreign_e2 = sealed_event(
        "other-run",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &local_e1.event_hash,
        "2026-11-25T10:00:01Z",
    );
    assert!(matches!(
        ports.append_envelopes("cg-1", (0, ZERO_HASH), &[local_e1, foreign_e2]),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 2 }))
    ));
    assert_eq!(ports.head("cg-1").unwrap(), (0, ZERO_HASH.to_string()));
    assert_eq!(sqlite_table_count(&db_path, "events"), 0);
}

#[test]
fn sqlite_import_run_stream_enforces_chain_integrity() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("import-guard.db");
    let store = Store::open(&db_path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("imp-guard");

    let mut forged = started_event("imp-1", "imp-guard", "2026-11-25T10:00:00Z");
    forged.event_hash = "f".repeat(64);
    assert!(matches!(
        ports.import_run_stream("imp-1", &manifest, std::slice::from_ref(&forged)),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));

    let local_e1 = started_event("imp-2", "imp-guard", "2026-11-25T10:00:00Z");
    let foreign_e2 = sealed_event(
        "other-run",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &local_e1.event_hash,
        "2026-11-25T10:00:01Z",
    );
    assert!(matches!(
        ports.import_run_stream("imp-2", &manifest, &[local_e1, foreign_e2]),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 2 }))
    ));
    assert_eq!(sqlite_table_count(&db_path, "runs"), 0);
    assert_eq!(sqlite_table_count(&db_path, "events"), 0);
}

#[test]
fn sqlite_head_and_append_refuse_planted_rows_before_chain() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("before-chain.db");
    let store = Store::open(&db_path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("planted");
    ports.create_run("neg-run", &manifest).unwrap();
    ports.create_run("zero-run", &manifest).unwrap();

    let raw = Connection::open(&db_path).unwrap();
    raw.execute_batch("DROP TRIGGER events_seq_is_a_position;")
        .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, -1, 'h', '{}')",
        ["neg-run"],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, 0, 'h', '{}')",
        ["zero-run"],
    )
    .unwrap();
    drop(raw);

    for (expected_run, bad_seq) in [("neg-run", -1i64), ("zero-run", 0i64)] {
        assert!(matches!(
            ports.head(expected_run),
            Err(StoreError::RowBeforeChain { ref run_id, seq })
                if run_id == expected_run && seq == bad_seq
        ));
        let e1 = started_event(expected_run, "planted", "2026-11-25T10:00:00Z");
        assert!(matches!(
            ports.append_envelopes(expected_run, (0, ZERO_HASH), std::slice::from_ref(&e1)),
            Err(StoreError::RowBeforeChain { ref run_id, seq })
                if run_id == expected_run && seq == bad_seq
        ));
    }
}

// ---------------------------------------------------------------------------
// HIGH #3: Protect co-located rebuild_from against divergent local events
// ---------------------------------------------------------------------------

#[test]
fn colocated_rebuild_from_refuses_divergent_logs_and_preserves_terminal_fence() {
    let dir = tempdir().unwrap();
    let local_store = Store::open(&dir.path().join("local.db")).unwrap();
    let other_store = Store::open(&dir.path().join("other.db")).unwrap();
    let local_ports = SqlitePorts::new(&local_store);
    let other_ports = SqlitePorts::new(&other_store);
    let manifest = sample_manifest("diverge");

    let e1 = started_event("div-1", "diverge", "2026-11-25T10:00:00Z");
    let local_e2 = sealed_event(
        "div-1",
        2,
        EventType::RunCompleted,
        json!({}),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    let other_e2 = sealed_event(
        "div-1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );

    local_ports
        .import_run_stream("div-1", &manifest, &[e1.clone(), local_e2.clone()])
        .unwrap();
    local_ports
        .apply_envelopes("div-1", &[e1.clone(), local_e2.clone()])
        .unwrap();
    other_ports
        .import_run_stream("div-1", &manifest, &[e1.clone(), other_e2])
        .unwrap();

    let err = local_ports.rebuild_from(&other_ports).unwrap_err();
    assert!(matches!(
        err,
        StoreError::DivergentEventLog { ref run_id } if run_id == "div-1"
    ));
    assert!(err
        .to_string()
        .contains("divergent event log for run 'div-1'"));

    let late_e3 = sealed_event(
        "div-1",
        3,
        EventType::PhaseEntered,
        json!({"phase": "after-complete"}),
        &local_e2.event_hash,
        "2026-11-25T10:00:10Z",
    );
    assert!(matches!(
        local_ports.append_envelopes(
            "div-1",
            (2, &local_e2.event_hash),
            std::slice::from_ref(&late_e3)
        ),
        Err(StoreError::AfterTerminal {
            ref run_id,
            status: Status::Completed,
            event_type: EventType::PhaseEntered,
        }) if run_id == "div-1"
    ));

    let empty_log = MemEventLog::default();
    assert!(matches!(
        local_ports.rebuild_from(&empty_log),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "div-1"
    ));

    let superset = MemEventLog::default();
    superset
        .import_run_stream("div-1", &manifest, &[e1, local_e2])
        .unwrap();
    let extra_e1 = started_event("extra-2", "extra", "2026-11-25T10:01:00Z");
    superset
        .import_run_stream("extra-2", &sample_manifest("extra"), &[extra_e1])
        .unwrap();
    assert!(matches!(
        local_ports.rebuild_from(&superset),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "extra-2"
    ));
}

// ---------------------------------------------------------------------------
// MEDIUM #1: Full-fold monotonic decoupled projection & standing ack
// ---------------------------------------------------------------------------

fn append_parked_session_flow<
    E: EventLog,
    P: ProjectionStore,
    C: crate::ports::CoordinationStore,
>(
    journal: &Journal<E, P, C>,
    run_id: &str,
) -> Vec<EventEnvelope> {
    let steps = [
        (EventType::RunStarted, started_payload("dec-full"), "00"),
        (EventType::PhaseEntered, json!({"phase": "implement"}), "01"),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff-1", "seat": "implement"}),
            "02",
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff-1", "attempt_id": "att-1"}),
            "03",
        ),
        (
            EventType::EffectCheckpointed,
            session_finished_checkpoint("att-1", 100),
            "05",
        ),
        (
            EventType::EffectCheckpointed,
            session_finished_checkpoint("att-1", 200),
            "10",
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "eff-1", "attempt_id": "att-1", "reason": "exhausted"}),
            "12",
        ),
        (
            EventType::RunParked,
            json!({"reason": "needs operator"}),
            "15",
        ),
    ];
    steps
        .into_iter()
        .map(|(ty, payload, sec)| {
            journal
                .append_event(
                    run_id,
                    ty,
                    payload,
                    None,
                    &format!("2026-11-25T09:00:{sec}Z"),
                )
                .unwrap()
        })
        .collect()
}

#[test]
fn decoupled_journal_full_fold_monotonic_and_session_dedup() {
    let dir = tempdir().unwrap();
    let proj_path = dir.path().join("proj-full.db");
    let proj_store = Store::open(&proj_path).unwrap();
    let journal = Journal::new(
        MemEventLog::default(),
        SqlitePorts::new(&proj_store),
        MemCoordinationStore::default(),
    );

    journal
        .create_run("dec-1", &sample_manifest("dec-full"))
        .unwrap();
    let sealed = append_parked_session_flow(&journal, "dec-1");

    let (head_seq, folded_seq, status, lane, last_at_ms) = summary_snapshot(&proj_path, "dec-1");
    assert_eq!(head_seq, 8);
    assert_eq!(folded_seq, Some(8));
    assert_eq!(status.as_deref(), Some("awaiting_operator"));
    assert_ne!(lane, "behind");
    assert_eq!(lane, "parked");
    assert!(last_at_ms.is_some());
    assert!(sqlite_table_count(&proj_path, "seat_summary") >= 1);
    assert_eq!(sqlite_table_count(&proj_path, "seat_head"), 1);

    let conn = Connection::open(&proj_path).unwrap();
    let session_seqs: Vec<i64> = conn
        .prepare("SELECT seq FROM event_index WHERE run_id = 'dec-1' AND session = 1 ORDER BY seq")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(session_seqs, vec![6]);
    let park_facts_seq: Option<i64> = conn
        .query_row(
            "SELECT facts_seq FROM event_index WHERE run_id = 'dec-1' AND seq = 8",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(park_facts_seq, Some(8));
    drop(conn);

    journal
        .projections()
        .apply_envelopes("dec-1", std::slice::from_ref(&sealed[0]))
        .unwrap();
    let after_stale = summary_snapshot(&proj_path, "dec-1");
    assert_eq!((after_stale.0, after_stale.1), (8, Some(8)));
    assert_eq!(after_stale.2.as_deref(), Some("awaiting_operator"));
    assert_eq!(after_stale.3, "parked");

    journal
        .projections()
        .apply_envelopes("dec-1", &sealed)
        .unwrap();
    assert_eq!(summary_snapshot(&proj_path, "dec-1"), after_stale);
}

#[test]
fn decoupled_projection_standing_acknowledgement_uses_summary_head() {
    let dir = tempdir().unwrap();
    let proj_path = dir.path().join("proj-ack.db");
    let proj_store = Store::open(&proj_path).unwrap();
    let ports = SqlitePorts::new(&proj_store);

    let e1 = started_event("ack-dec", "ack-feat", "2026-11-25T09:00:00Z");
    let e2 = sealed_event(
        "ack-dec",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-25T09:00:05Z",
    );

    let raw = Connection::open(&proj_path).unwrap();
    raw.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    raw.execute(
        "INSERT INTO quarantine_acknowledgements
            (run_id, head_seq, head_hash, operator, reason, recorded_at)
         VALUES (?1, 0, ?2, 'stale-op', 'empty head', '2026-11-25T08:59:00Z')",
        params!["ack-dec", ZERO_HASH],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO quarantine_acknowledgements
            (run_id, head_seq, head_hash, operator, reason, recorded_at)
         VALUES (?1, 2, ?2, 'valentin', 'reviewed at seq 2', '2026-11-25T09:05:00Z')",
        params!["ack-dec", e2.event_hash],
    )
    .unwrap();
    drop(raw);

    let read_ack_cols = || -> (i64, Option<String>, Option<String>) {
        let conn = Connection::open(&proj_path).unwrap();
        conn.query_row(
            "SELECT acknowledged, ack_operator, ack_reason FROM run_summary WHERE run_id = 'ack-dec'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
    };

    ports
        .apply_envelopes("ack-dec", std::slice::from_ref(&e1))
        .unwrap();
    assert_eq!(read_ack_cols(), (0, None, None));

    ports
        .apply_envelopes("ack-dec", &[e1.clone(), e2.clone()])
        .unwrap();
    let expected = (
        1,
        Some("valentin".to_string()),
        Some("reviewed at seq 2".to_string()),
    );
    assert_eq!(read_ack_cols(), expected);
    assert_eq!(sqlite_table_count(&proj_path, "events"), 0);

    let mem = MemEventLog::default();
    mem.import_run_stream("ack-dec", &sample_manifest("ack-feat"), &[e1, e2])
        .unwrap();
    assert_eq!(ports.rebuild_from(&mem).unwrap(), 2);
    assert_eq!(read_ack_cols(), expected);
}

#[test]
fn sqlite_direct_suffix_apply_envelopes_covers_colocated_and_decoupled() {
    let dir = tempdir().unwrap();
    let coloc_path = dir.path().join("coloc-suffix.db");
    let coloc = Store::open(&coloc_path).unwrap();
    let coloc_ports = SqlitePorts::new(&coloc);
    coloc_ports
        .create_run("cs-1", &sample_manifest("coloc"))
        .unwrap();

    let e1 = started_event("cs-1", "coloc", "2026-11-25T10:00:00Z");
    let e2 = sealed_event(
        "cs-1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "verify"}),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    coloc_ports
        .append_envelopes("cs-1", (0, ZERO_HASH), &[e1.clone(), e2.clone()])
        .unwrap();
    coloc_ports
        .apply_envelopes("cs-1", std::slice::from_ref(&e2))
        .unwrap();
    let coloc_snap = summary_snapshot(&coloc_path, "cs-1");
    assert_eq!((coloc_snap.0, coloc_snap.1), (2, Some(2)));

    let dec_path = dir.path().join("dec-suffix.db");
    let dec = Store::open(&dec_path).unwrap();
    let dec_ports = SqlitePorts::new(&dec);
    let e3 = sealed_event(
        "cs-1",
        3,
        EventType::EffectCheckpointed,
        valid_checkpoint_payload(),
        &e2.event_hash,
        "2026-11-25T10:00:10Z",
    );
    let e4 = sealed_event(
        "cs-1",
        4,
        EventType::RunCompleted,
        json!({}),
        &e3.event_hash,
        "2026-11-25T10:00:15Z",
    );
    assert!(matches!(
        dec_ports.apply_envelopes("other-run", std::slice::from_ref(&e1)),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    let manifest_only = sealed_event(
        "mf-1",
        1,
        EventType::RunStarted,
        json!({"manifest": {"engine": "0.15.0", "feature": "from-manifest"}}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    dec_ports.apply_envelopes("mf-1", &[manifest_only]).unwrap();
    assert_eq!(dec_ports.list_runs().unwrap()[0].1, "from-manifest");

    dec_ports
        .apply_envelopes("cs-1", std::slice::from_ref(&e1))
        .unwrap();
    for event in [e2, e3, e4] {
        dec_ports
            .apply_envelopes("cs-1", std::slice::from_ref(&event))
            .unwrap();
    }
    let conn = Connection::open(&dec_path).unwrap();
    let (head_seq, phase, status): (i64, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT head_seq, phase, status FROM run_summary WHERE run_id = 'cs-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(head_seq, 4);
    assert_eq!(phase.as_deref(), Some("verify"));
    assert_eq!(status.as_deref(), Some("completed"));
}

// ---------------------------------------------------------------------------
// MEDIUM #2: Empty-run consistency & created_at from events[0].recorded_at
// ---------------------------------------------------------------------------

#[test]
fn rebuild_from_skips_empty_runs_and_derives_created_at_from_first_event() {
    let dir = tempdir().unwrap();

    let mem = MemEventLog::default();
    mem.create_run("empty-1", &sample_manifest("empty"))
        .unwrap();
    let e_2020 = started_event("live-2020", "live", "2020-05-01T12:00:00Z");
    mem.import_run_stream("live-2020", &sample_manifest("live"), &[e_2020])
        .unwrap();
    let dec_store = Store::open(&dir.path().join("dec-empty.db")).unwrap();
    let dec_ports = SqlitePorts::new(&dec_store);
    assert_eq!(dec_ports.rebuild_from(&mem).unwrap(), 1);
    assert_eq!(
        dec_ports.list_runs().unwrap(),
        vec![(
            "live-2020".to_string(),
            "live".to_string(),
            "2020-05-01T12:00:00Z".to_string(),
        )]
    );

    let coloc_path = dir.path().join("coloc-2020.db");
    let coloc_store = Store::open(&coloc_path).unwrap();
    let coloc_ports = SqlitePorts::new(&coloc_store);
    coloc_ports
        .create_run("empty-local", &sample_manifest("empty"))
        .unwrap();
    coloc_ports
        .create_run("retro-1", &sample_manifest("retro"))
        .unwrap();
    let retro_e1 = started_event("retro-1", "retro", "2020-01-01T00:00:00Z");
    coloc_ports
        .append_envelopes("retro-1", (0, ZERO_HASH), &[retro_e1])
        .unwrap();

    let raw = Connection::open(&coloc_path).unwrap();
    raw.execute(
        "UPDATE runs SET created_at = '2026-11-25T00:00:00Z' WHERE run_id = 'retro-1'",
        [],
    )
    .unwrap();
    drop(raw);

    assert_eq!(coloc_ports.rebuild_from(&coloc_ports).unwrap(), 1);
    assert_eq!(
        coloc_ports.list_runs().unwrap(),
        vec![(
            "retro-1".to_string(),
            "retro".to_string(),
            "2020-01-01T00:00:00Z".to_string(),
        )]
    );
}

// ---------------------------------------------------------------------------
// MEDIUM #3: import_ndjson ProjectionLag recovery via journal.reindex()
// ---------------------------------------------------------------------------

struct FailingImportLog;

impl EventLog for FailingImportLog {
    fn create_run(&self, _: &str, _: &Value) -> Result<(), StoreError> {
        unreachable!()
    }
    fn append_envelopes(
        &self,
        _: &str,
        _: (u64, &str),
        _: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        unreachable!()
    }
    fn import_run_stream(&self, _: &str, _: &Value, _: &[EventEnvelope]) -> Result<(), StoreError> {
        Err(StoreError::Contended {
            operation: "import_run",
            waited_ms: 1,
        })
    }
    fn head(&self, _: &str) -> Result<(u64, String), StoreError> {
        unreachable!()
    }
    fn read_stream(&self, _: &str, _: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        unreachable!()
    }
    fn manifest(&self, _: &str) -> Result<Value, StoreError> {
        unreachable!()
    }
    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        unreachable!()
    }
}

struct FailOnceProjectionStore<P> {
    inner: P,
    fail_next: Cell<bool>,
}

impl<P: ProjectionStore> ProjectionStore for FailOnceProjectionStore<P> {
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        if self.fail_next.replace(false) {
            return Err(StoreError::Contended {
                operation: "apply_envelopes",
                waited_ms: 3,
            });
        }
        self.inner.apply_envelopes(run_id, envelopes)
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        self.inner.list_runs()
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        self.inner.rebuild_from(log)
    }
}

#[test]
fn import_ndjson_projection_lag_heals_via_reindex() {
    let dir = tempdir().unwrap();
    let src_journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("heal-feat");
    src_journal.create_run("heal-1", &manifest).unwrap();
    src_journal
        .append_event(
            "heal-1",
            EventType::RunStarted,
            started_payload("heal-feat"),
            None,
            "2026-11-25T11:00:00Z",
        )
        .unwrap();
    src_journal
        .append_event(
            "heal-1",
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            "2026-11-25T11:00:05Z",
        )
        .unwrap();
    let ndjson = src_journal.export_ndjson("heal-1").unwrap();

    let proj_store = Store::open(&dir.path().join("heal-proj.db")).unwrap();
    let flaky = FailOnceProjectionStore {
        inner: SqlitePorts::new(&proj_store),
        fail_next: Cell::new(true),
    };
    let dst_journal = Journal::new(
        MemEventLog::default(),
        flaky,
        MemCoordinationStore::default(),
    );

    let err = dst_journal
        .import_ndjson("heal-1", &manifest, &ndjson)
        .unwrap_err();
    assert!(matches!(
        err,
        ImportError::Store(ref store_err @ StoreError::ProjectionLag { ref run_id, .. })
            if run_id == "heal-1" && !store_err.is_contention()
    ));
    assert!(matches!(
        dst_journal.import_ndjson("heal-1", &manifest, &ndjson),
        Err(ImportError::Collision(ref id)) if id == "heal-1"
    ));
    assert!(dst_journal.projections().list_runs().unwrap().is_empty());

    let contended_journal = Journal::new(
        FailingImportLog,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    assert!(matches!(
        contended_journal.import_ndjson("heal-1", &manifest, &ndjson),
        Err(ImportError::Store(StoreError::Contended {
            operation: "import_run",
            ..
        }))
    ));

    assert_eq!(dst_journal.reindex().unwrap(), 2);
    assert_eq!(
        dst_journal.projections().list_runs().unwrap(),
        vec![(
            "heal-1".to_string(),
            "heal-feat".to_string(),
            "2026-11-25T11:00:00Z".to_string(),
        )]
    );
}
