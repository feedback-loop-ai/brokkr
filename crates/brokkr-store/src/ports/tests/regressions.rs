//! Round 2 regression tests for decision 0126 Slice 1 (#1362):
//! method shadowing, import adoption gates, post-terminal & settled-member
//! invariants, whole-table & run-id integrity, decoupled SQLite projections,
//! non-contention projection lag, and atomic batch appends.

use std::cell::Cell;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::fold::{FoldError, Status};
use rusqlite::Connection;
use serde_json::{json, Value};
use tempfile::tempdir;

use super::{
    open_split_journal, sample_manifest, started_payload, valid_checkpoint_payload,
    MemCoordinationStore, MemEventLog, MemProjectionStore,
};
use crate::damage::Damage;
use crate::ports::{EventLog, Journal, ProjectionStore, SqlitePorts};
use crate::{ImportError, Store, StoreError, VerifyError};

pub(super) fn sealed_event(
    run_id: &str,
    seq: u64,
    event_type: EventType,
    payload: Value,
    previous_hash: &str,
    ts: &str,
) -> EventEnvelope {
    EventEnvelope {
        run_id: run_id.to_string(),
        seq,
        event_id: format!("ev-{run_id}-{seq}"),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: run_id.to_string(),
        attempt_id: None,
        recorded_at: ts.to_string(),
        previous_hash: previous_hash.to_string(),
        event_hash: String::new(),
    }
    .sealed()
}

pub(super) fn single_ndjson(event: &EventEnvelope) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&serde_json::to_value(event).unwrap()).unwrap()
    )
}

pub(super) fn sqlite_table_count(path: &std::path::Path, table: &str) -> i64 {
    let conn = Connection::open(path).unwrap();
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

// ---------------------------------------------------------------------------
// 1. No method-name shadowing on Store when importing ports::*
// ---------------------------------------------------------------------------

#[test]
fn wildcard_ports_import_does_not_shadow_store_methods() {
    use crate::ports::*;

    let dir = tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("shadow.db")).unwrap();
    let manifest = sample_manifest("no-shadow");
    store
        .create_run("shadow-1", "no-shadow", "self", &manifest)
        .unwrap();
    assert_eq!(store.list_runs().unwrap().len(), 1);
    assert_eq!(store.manifest("shadow-1").unwrap(), manifest);
    let ports = SqlitePorts::new(&store);
    assert!(ports.list_queue().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// 2. Journal::import_ndjson adoption gates & atomic import_run_stream
// ---------------------------------------------------------------------------

#[test]
fn import_ndjson_enforces_adoption_gates_and_attested_manifest() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("adopt.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let journal = Journal::new(&ports, &ports, &ports);
    let sidecar = sample_manifest("sidecar");

    for bad_id in ["", "a/b", "..", "with space", "bidi\u{202E}id"] {
        assert!(
            matches!(
                journal.import_ndjson(bad_id, &sidecar, ""),
                Err(ImportError::UnadoptableRunId(_))
            ),
            "expected UnadoptableRunId for {bad_id:?}"
        );
    }

    assert!(matches!(
        journal.import_ndjson("empty-run", &sidecar, ""),
        Err(ImportError::Verify(VerifyError::Fold(FoldError::Empty)))
    ));

    let bad_proto = sealed_event(
        "proto-bad",
        1,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("proto-bad", &sidecar, &single_ndjson(&bad_proto)),
        Err(ImportError::Verify(VerifyError::Fold(_)))
    ));

    let redacted_sidecar = json!({"engine": "0.15.0", "redacted": true});
    assert!(matches!(
        journal.import_ndjson("redacted-1", &redacted_sidecar, ""),
        Err(ImportError::Redacted(id)) if id == "redacted-1"
    ));

    let redacted_started = sealed_event(
        "redacted-2",
        1,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {"engine": "0.15.0", "redacted": true}}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("redacted-2", &sidecar, &single_ndjson(&redacted_started)),
        Err(ImportError::Redacted(id)) if id == "redacted-2"
    ));

    let no_feature = sealed_event(
        "no-feat",
        1,
        EventType::RunStarted,
        json!({"manifest": sample_manifest("x")}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("no-feat", &sidecar, &single_ndjson(&no_feature)),
        Err(ImportError::Unattested { field: "feature" })
    ));

    let no_manifest = sealed_event(
        "no-man",
        1,
        EventType::RunStarted,
        json!({"feature": "x"}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("no-man", &sidecar, &single_ndjson(&no_manifest)),
        Err(ImportError::Unattested { field: "manifest" })
    ));

    // Sidecar manifest attempting downgrade is ignored; attested manifest wins and origin_host is NULL.
    let attested_manifest = sample_manifest("attested-feature");
    let started = sealed_event(
        "attested-1",
        1,
        EventType::RunStarted,
        json!({"feature": "attested-feature", "manifest": attested_manifest}),
        ZERO_HASH,
        "2026-11-25T11:00:00Z",
    );
    let downgraded_sidecar = json!({"engine": "0.7.0", "feature": "evil", "bundle_name": "evil"});
    let count = journal
        .import_ndjson("attested-1", &downgraded_sidecar, &single_ndjson(&started))
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(ports.manifest("attested-1").unwrap(), attested_manifest);
    assert!(!store.started_here("attested-1").unwrap());
}

#[test]
fn import_run_stream_is_atomic_on_failure_and_allows_retry() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("atomic.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let mem = MemEventLog::default();
    let manifest = sample_manifest("atomic");
    let e1 = sealed_event(
        "atomic-1",
        1,
        EventType::RunStarted,
        started_payload("atomic"),
        ZERO_HASH,
        "2026-11-25T11:10:00Z",
    );
    let foreign = sealed_event(
        "other-run",
        1,
        EventType::RunStarted,
        started_payload("atomic"),
        ZERO_HASH,
        "2026-11-25T11:10:00Z",
    );

    assert!(matches!(
        ports.import_run_stream("atomic-1", &manifest, &[e1.clone(), e1.clone()]),
        Err(StoreError::Chain(ChainError::SeqGap {
            seq: 1,
            expected: 2
        }))
    ));
    assert!(matches!(
        mem.import_run_stream("atomic-1", &manifest, &[e1.clone(), e1.clone()]),
        Err(StoreError::AppendConflict { seq: 1 })
    ));
    for log in [&ports as &dyn EventLog, &mem as &dyn EventLog] {
        assert!(matches!(
            log.import_run_stream("atomic-1", &manifest, std::slice::from_ref(&foreign)),
            Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
        ));
        assert!(log.list_run_ids().unwrap().is_empty());
        assert!(matches!(
            log.manifest("atomic-1"),
            Err(StoreError::RunNotFound(id)) if id == "atomic-1"
        ));
        log.import_run_stream("atomic-1", &manifest, std::slice::from_ref(&e1))
            .unwrap();
        assert_eq!(log.head("atomic-1").unwrap(), (1, e1.event_hash.clone()));
    }
    assert!(!store.started_here("atomic-1").unwrap());
    // Empty stream import exercises created_at fallback when no RunStarted is present.
    ports
        .import_run_stream("empty-import", &manifest, &[])
        .unwrap();
    assert!(!store.started_here("empty-import").unwrap());
}

// ---------------------------------------------------------------------------
// 3. Post-terminal refusal, settled_members, engine resolution, seq overflow,
//    and single head() call in append_event
// ---------------------------------------------------------------------------

#[test]
fn post_terminal_refusal_on_journal() {
    let mem_journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("term");
    for (run_id, terminal_type, expected_status) in [
        ("term-done", EventType::RunCompleted, Status::Completed),
        ("term-stop", EventType::RunStopped, Status::Stopped),
    ] {
        mem_journal.create_run(run_id, &manifest).unwrap();
        mem_journal
            .append_event(
                run_id,
                EventType::RunStarted,
                started_payload("term"),
                None,
                "2026-11-25T10:00:00Z",
            )
            .unwrap();
        mem_journal
            .append_event(
                run_id,
                terminal_type,
                json!({}),
                None,
                "2026-11-25T10:00:05Z",
            )
            .unwrap();
        assert!(matches!(
            mem_journal.append_event(
                run_id,
                EventType::PhaseEntered,
                json!({"phase": "late"}),
                None,
                "2026-11-25T10:00:06Z",
            ),
            Err(StoreError::AfterTerminal {
                status,
                event_type: EventType::PhaseEntered,
                ..
            }) if status == expected_status
        ));
        // Annotations remain lawful after terminal.
        let note = mem_journal
            .append_event(
                run_id,
                EventType::OperatorRejected,
                json!({"reason": "post-mortem"}),
                None,
                "2026-11-25T10:00:07Z",
            )
            .unwrap();
        assert_eq!(note.seq, 3);
    }
}

#[test]
fn post_terminal_refusal_on_sqlite_ports() {
    let manifest = sample_manifest("term");
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("term.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    ports.create_run("sql-term", &manifest).unwrap();
    let e1 = sealed_event(
        "sql-term",
        1,
        EventType::RunStarted,
        started_payload("term"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    ports
        .append_envelopes("sql-term", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();
    let e2 = sealed_event(
        "sql-term",
        2,
        EventType::RunCompleted,
        json!({}),
        &e1.event_hash,
        "2026-11-25T10:00:01Z",
    );
    let e3_bad = sealed_event(
        "sql-term",
        3,
        EventType::PhaseEntered,
        json!({"phase": "after"}),
        &e2.event_hash,
        "2026-11-25T10:00:02Z",
    );
    assert!(matches!(
        ports.append_envelopes("sql-term", (1, &e1.event_hash), &[e2.clone(), e3_bad]),
        Err(StoreError::AfterTerminal {
            status: Status::Completed,
            event_type: EventType::PhaseEntered,
            ..
        })
    ));
    // Whole batch rolled back: head stays at seq 1.
    assert_eq!(ports.head("sql-term").unwrap().0, 1);

    ports
        .append_envelopes("sql-term", (1, &e1.event_hash), std::slice::from_ref(&e2))
        .unwrap();
    let e3_note = sealed_event(
        "sql-term",
        3,
        EventType::OperatorRejected,
        json!({"reason": "note"}),
        &e2.event_hash,
        "2026-11-25T10:00:03Z",
    );
    ports
        .append_envelopes(
            "sql-term",
            (2, &e2.event_hash),
            std::slice::from_ref(&e3_note),
        )
        .unwrap();
    assert_eq!(ports.head("sql-term").unwrap().0, 3);
}

fn panel_member_checkpoint() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "checkpoint": {
            "step": "panel-member-finished",
            "outcome": "succeeded",
            "member": "alpha",
            "authored": "engine",
        }
    })
}

#[test]
fn run_started_engine_version_overrides_sidecar_manifest() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    // Sidecar claims v8 (0.15.0), but RunStarted's attested manifest names v1 (0.7.0).
    let sidecar_v8 = sample_manifest("engine-mismatch");
    journal.create_run("v1-run", &sidecar_v8).unwrap();
    journal
        .append_event(
            "v1-run",
            EventType::RunStarted,
            json!({
                "feature": "engine-mismatch",
                "manifest": {"engine": "0.7.0", "feature": "engine-mismatch"},
            }),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();
    assert!(matches!(
        journal.append_event(
            "v1-run",
            EventType::EffectCheckpointed,
            panel_member_checkpoint(),
            None,
            "2026-11-25T10:00:01Z",
        ),
        Err(StoreError::SeatRecord(ref err))
            if err.seq == 2 && err.contract == "contracts/seat-record.v1.schema.json"
    ));
}

#[test]
fn settled_members_validation_on_effect_failed_and_indeterminate() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("v8-panel", &sample_manifest("v8-panel"))
        .unwrap();
    journal
        .append_event(
            "v8-panel",
            EventType::RunStarted,
            started_payload("v8-panel"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();
    journal
        .append_event(
            "v8-panel",
            EventType::EffectCheckpointed,
            panel_member_checkpoint(),
            None,
            "2026-11-25T10:00:01Z",
        )
        .unwrap();
    let bad_settled = json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "reason": "member failed",
        "settled_members": {
            "alpha": {"not_a_result": true}
        }
    });
    for event_type in [EventType::EffectFailed, EventType::EffectIndeterminate] {
        assert!(matches!(
            journal.append_event(
                "v8-panel",
                event_type,
                bad_settled.clone(),
                None,
                "2026-11-25T10:00:02Z",
            ),
            Err(StoreError::SeatRecord(ref err))
                if err.seq == 3 && err.settled_member.as_deref().map(String::as_str) == Some("alpha")
        ));
    }
    let good_settled = json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "reason": "peer failed",
        "settled_members": {
            "alpha": {"result": "pass", "notes": "ok"}
        }
    });
    let appended = journal
        .append_event(
            "v8-panel",
            EventType::EffectFailed,
            good_settled,
            None,
            "2026-11-25T10:00:03Z",
        )
        .unwrap();
    assert_eq!(appended.seq, 3);
}

struct CountingEventLog {
    inner: MemEventLog,
    head_calls: Cell<usize>,
}

impl EventLog for CountingEventLog {
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
        self.head_calls.set(self.head_calls.get() + 1);
        self.inner.head(run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        self.inner.read_stream(run_id, from_seq)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        self.inner.manifest(run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        self.inner.list_run_ids()
    }
}

#[test]
fn seq_overflow_and_single_head_call_in_append_event() {
    let counting = CountingEventLog {
        inner: MemEventLog::default(),
        head_calls: Cell::new(0),
    };
    let journal = Journal::new(
        counting,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("cnt-1", &sample_manifest("cnt"))
        .unwrap();

    let e1 = journal
        .append_event(
            "cnt-1",
            EventType::RunStarted,
            started_payload("cnt"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();
    assert_eq!(journal.event_log().head_calls.get(), 1);

    let e2 = journal
        .append_if_head(
            "cnt-1",
            (1, &e1.event_hash),
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            "2026-11-25T10:00:01Z",
        )
        .unwrap();
    assert_eq!(e2.seq, 2);
    assert_eq!(journal.event_log().head_calls.get(), 1);

    assert!(matches!(
        journal.append_if_head(
            "cnt-1",
            (u64::MAX, ZERO_HASH),
            EventType::PhaseEntered,
            json!({"phase": "overflow"}),
            None,
            "2026-11-25T10:00:02Z",
        ),
        Err(StoreError::Chain(ChainError::SeqOverflow { after })) if after == u64::MAX
    ));

    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("overflow.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    ports.create_run("ov-1", &sample_manifest("ov")).unwrap();
    let over_i64 = sealed_event(
        "ov-1",
        i64::MAX as u64 + 1,
        EventType::RunStarted,
        started_payload("ov"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        ports.append_envelopes("ov-1", (0, ZERO_HASH), std::slice::from_ref(&over_i64)),
        Err(StoreError::Chain(ChainError::SeqOverflow { after })) if after == i64::MAX as u64
    ));
}

// ---------------------------------------------------------------------------
// 4. Whole-table read_stream (seq <= 0) & run_id integrity
// ---------------------------------------------------------------------------

#[test]
fn sqlite_read_stream_refuses_planted_rows_before_chain() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("planted.db");
    let store = Store::open(&db_path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("planted");
    ports.create_run("bad-json", &manifest).unwrap();
    ports.create_run("valid-at-zero", &manifest).unwrap();

    let e1 = sealed_event(
        "valid-at-zero",
        1,
        EventType::RunStarted,
        started_payload("planted"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    let e1_json = serde_json::to_string(&e1).unwrap();

    // Plant rows at seq = 0 directly (bypassing the seq >= 1 trigger) as a corrupt DB would.
    let raw = Connection::open(&db_path).unwrap();
    raw.execute_batch("DROP TRIGGER events_seq_is_a_position;")
        .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, 0, 'h', '{not json')",
        ["bad-json"],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, 0, ?2, ?3)",
        rusqlite::params!["valid-at-zero", e1.event_hash, e1_json],
    )
    .unwrap();
    drop(raw);

    for from_seq in [0, 1] {
        assert!(matches!(
            ports.read_stream("bad-json", from_seq),
            Err(StoreError::Damaged {
                damage: Damage::Envelope { ref run_id, seq: 0 },
                ..
            }) if run_id == "bad-json"
        ));
        assert!(matches!(
            ports.read_stream("valid-at-zero", from_seq),
            Err(StoreError::RowBeforeChain { ref run_id, seq: 0 }) if run_id == "valid-at-zero"
        ));
    }
}

#[test]
fn foreign_run_id_refused_in_load_verified_and_append_envelopes() {
    let mem = MemEventLog::default();
    let e_a = sealed_event(
        "run-a",
        1,
        EventType::RunStarted,
        started_payload("a"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    // Plant run-a's valid stream beneath run-b.
    mem.plant_raw_stream("run-b", sample_manifest("b"), vec![e_a.clone()]);
    let journal = Journal::new(
        &mem,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    assert!(matches!(
        journal.load_verified("run-b"),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert!(matches!(
        journal.import_ndjson("run-c", &sample_manifest("c"), &single_ndjson(&e_a)),
        Err(ImportError::Verify(VerifyError::Chain(
            ChainError::ForeignRun { seq: 1 }
        )))
    ));

    mem.create_run("run-c", &sample_manifest("c")).unwrap();
    assert!(matches!(
        mem.append_envelopes("run-c", (0, ZERO_HASH), std::slice::from_ref(&e_a)),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));

    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("foreign.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    ports.create_run("run-c", &sample_manifest("c")).unwrap();
    assert!(matches!(
        ports.append_envelopes("run-c", (0, ZERO_HASH), std::slice::from_ref(&e_a)),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
}

// ---------------------------------------------------------------------------
// 5. Decoupled SQLite projections, TokenCountOutOfRange, ProjectionLag,
//    and rebuild_from verification / clearing
// ---------------------------------------------------------------------------

#[test]
fn split_journal_mem_log_sqlite_projections_end_to_end() {
    let dir = tempdir().unwrap();
    let proj_path = dir.path().join("proj.db");
    let proj_store = Store::open(&proj_path).unwrap();
    let journal = open_split_journal(&proj_store, "split-1", "split-feat");
    journal
        .append_event(
            "split-1",
            EventType::RunStarted,
            started_payload("split-feat"),
            None,
            "2026-11-25T09:00:00Z",
        )
        .unwrap();
    journal
        .append_event(
            "split-1",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            "2026-11-25T09:00:02Z",
        )
        .unwrap();
    journal
        .append_event(
            "split-1",
            EventType::EffectCheckpointed,
            valid_checkpoint_payload(),
            None,
            "2026-11-25T09:00:05Z",
        )
        .unwrap();
    journal
        .append_event(
            "split-1",
            EventType::RunCompleted,
            json!({}),
            None,
            "2026-11-25T09:00:10Z",
        )
        .unwrap();

    let expected = vec![(
        "split-1".to_string(),
        "split-feat".to_string(),
        "2026-11-25T09:00:00Z".to_string(),
    )];
    let check_decoupled = || {
        assert_eq!(journal.projections().list_runs().unwrap(), expected);
        assert_eq!(sqlite_table_count(&proj_path, "runs"), 0);
        assert_eq!(sqlite_table_count(&proj_path, "events"), 0);
    };
    check_decoupled();
    assert_eq!(sqlite_table_count(&proj_path, "event_index"), 4);

    assert_eq!(journal.reindex().unwrap(), 4);
    check_decoupled();
}

#[test]
fn split_two_file_sqlite_journal_works_without_copying_events() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("log.db");
    let proj_path = dir.path().join("index.db");
    let coord_path = dir.path().join("queue.db");
    let log_store = Store::open(&log_path).unwrap();
    let proj_store = Store::open(&proj_path).unwrap();
    let coord_store = Store::open(&coord_path).unwrap();
    let journal = Journal::new(
        SqlitePorts::new(&log_store),
        SqlitePorts::new(&proj_store),
        SqlitePorts::new(&coord_store),
    );

    journal
        .create_run("two-1", &sample_manifest("two-file"))
        .unwrap();
    journal
        .append_event(
            "two-1",
            EventType::RunStarted,
            started_payload("two-file"),
            None,
            "2026-11-25T08:00:00Z",
        )
        .unwrap();
    journal
        .append_event(
            "two-1",
            EventType::EffectCheckpointed,
            valid_checkpoint_payload(),
            None,
            "2026-11-25T08:00:05Z",
        )
        .unwrap();

    assert_eq!(sqlite_table_count(&log_path, "events"), 2);
    assert_eq!(sqlite_table_count(&proj_path, "events"), 0);
    assert_eq!(sqlite_table_count(&proj_path, "runs"), 0);
    assert_eq!(
        journal.projections().list_runs().unwrap(),
        vec![(
            "two-1".to_string(),
            "two-file".to_string(),
            "2026-11-25T08:00:00Z".to_string(),
        )]
    );
    assert_eq!(journal.reindex().unwrap(), 2);
    assert_eq!(sqlite_table_count(&proj_path, "events"), 0);
}

#[test]
fn sqlite_projection_propagates_token_count_out_of_range() {
    let dir = tempdir().unwrap();
    let proj_store = Store::open(&dir.path().join("tokens.db")).unwrap();
    let ports = SqlitePorts::new(&proj_store);
    let overflow = i64::MAX as u64 + 1;

    let e1 = sealed_event(
        "tok-1",
        1,
        EventType::RunStarted,
        started_payload("tok"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    let e2 = sealed_event(
        "tok-1",
        2,
        EventType::EffectCheckpointed,
        json!({
            "effect_id": "eff-1",
            "attempt_id": "att-1",
            "checkpoint": {
                "step": "seat-turn",
                "turn": 1,
                "input_tokens": overflow
            }
        }),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );

    // Full-stream apply_envelopes propagates TokenCountOutOfRange.
    assert!(matches!(
        ports.apply_envelopes("tok-1", &[e1.clone(), e2.clone()]),
        Err(StoreError::TokenCountOutOfRange(2, "input_tokens", count)) if count == overflow
    ));

    // Journal over decoupled SqlitePorts wraps TokenCountOutOfRange inside typed ProjectionLag.
    let split = open_split_journal(&proj_store, "tok-lag", "tok");
    split
        .append_event(
            "tok-lag",
            EventType::RunStarted,
            started_payload("tok"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();
    let lag_err = split
        .append_event(
            "tok-lag",
            EventType::EffectCheckpointed,
            e2.payload.clone(),
            None,
            "2026-11-25T10:00:05Z",
        )
        .unwrap_err();
    assert!(!lag_err.is_contention());
    assert!(matches!(
        lag_err,
        StoreError::ProjectionLag { ref run_id, ref source }
            if run_id == "tok-lag"
                && matches!(
                    source.as_ref(),
                    StoreError::TokenCountOutOfRange(2, "input_tokens", count)
                        if *count == overflow
                )
    ));

    // rebuild_from also propagates TokenCountOutOfRange.
    let mem = MemEventLog::default();
    mem.import_run_stream("tok-1", &sample_manifest("tok"), &[e1, e2])
        .unwrap();
    assert!(matches!(
        ports.rebuild_from(&mem),
        Err(StoreError::TokenCountOutOfRange(2, "input_tokens", count)) if count == overflow
    ));
}

struct ContendingProjectionStore;

impl ProjectionStore for ContendingProjectionStore {
    fn apply_envelopes(
        &self,
        _run_id: &str,
        _envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        Err(StoreError::Contended {
            operation: "apply_envelopes",
            waited_ms: 5,
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        Ok(Vec::new())
    }

    fn rebuild_from(&self, _log: &dyn EventLog) -> Result<u64, StoreError> {
        Ok(0)
    }
}

#[test]
fn post_commit_projection_failure_returns_non_contention_projection_lag() {
    let journal = Journal::new(
        MemEventLog::default(),
        ContendingProjectionStore,
        MemCoordinationStore::default(),
    );
    journal
        .create_run("lag-1", &sample_manifest("lag"))
        .unwrap();

    let err = journal
        .append_event(
            "lag-1",
            EventType::RunStarted,
            started_payload("lag"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap_err();
    assert!(!err.is_contention());
    assert!(matches!(
        err,
        StoreError::ProjectionLag { ref run_id, ref source }
            if run_id == "lag-1"
                && matches!(
                    source.as_ref(),
                    StoreError::Contended {
                        operation: "apply_envelopes",
                        waited_ms: 5,
                    }
                )
    ));
    assert!(std::error::Error::source(&err).is_some());
    assert_eq!(
        err.to_string(),
        concat!(
            "projection lag for run 'lag-1': event log committed, ",
            "but projection update failed (contended: a peer still held ",
            "the journal's write lock after 5ms of apply_envelopes; nothing was written)",
        )
    );
    // The event WAS durably committed in EventLog before projections lagged.
    assert_eq!(journal.load_verified("lag-1").unwrap().len(), 1);

    let ndjson = journal.export_ndjson("lag-1").unwrap();
    let import_journal = Journal::new(
        MemEventLog::default(),
        ContendingProjectionStore,
        MemCoordinationStore::default(),
    );
    let import_err = import_journal
        .import_ndjson("lag-1", &sample_manifest("lag"), &ndjson)
        .unwrap_err();
    assert!(matches!(
        import_err,
        ImportError::Store(
            ref store_err @ StoreError::ProjectionLag {
                ref run_id,
                ref source,
            }
        ) if run_id == "lag-1"
            && !store_err.is_contention()
            && matches!(
                source.as_ref(),
                StoreError::Contended {
                    operation: "apply_envelopes",
                    waited_ms: 5,
                }
            )
    ));
    assert_eq!(import_journal.load_verified("lag-1").unwrap().len(), 1);
}

#[test]
fn sqlite_rebuild_from_verifies_streams_and_clears_stale_runs() {
    let dir = tempdir().unwrap();
    let proj_store = Store::open(&dir.path().join("rebuild-verify.db")).unwrap();
    let ports = SqlitePorts::new(&proj_store);

    let good_log = MemEventLog::default();
    let e1 = sealed_event(
        "keep-1",
        1,
        EventType::RunStarted,
        started_payload("keep"),
        ZERO_HASH,
        "2026-11-25T07:00:00Z",
    );
    good_log
        .import_run_stream(
            "keep-1",
            &sample_manifest("keep"),
            std::slice::from_ref(&e1),
        )
        .unwrap();
    assert_eq!(ports.rebuild_from(&good_log).unwrap(), 1);
    assert_eq!(
        ports.list_runs().unwrap(),
        vec![(
            "keep-1".to_string(),
            "keep".to_string(),
            "2026-11-25T07:00:00Z".to_string(),
        )]
    );

    // Tampered envelope in source log is refused and leaves existing projections intact.
    let tampered_log = MemEventLog::default();
    tampered_log
        .import_run_stream(
            "keep-1",
            &sample_manifest("keep"),
            std::slice::from_ref(&e1),
        )
        .unwrap();
    tampered_log.tamper_payload("keep-1", 0, json!({"feature": "evil"}));
    assert!(matches!(
        ports.rebuild_from(&tampered_log),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));
    assert_eq!(ports.list_runs().unwrap().len(), 1);

    // Foreign run_id stream is refused and leaves existing projections intact.
    let foreign_log = MemEventLog::default();
    foreign_log.plant_raw_stream("wrong-id", sample_manifest("keep"), vec![e1]);
    assert!(matches!(
        ports.rebuild_from(&foreign_log),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert_eq!(ports.list_runs().unwrap().len(), 1);

    // Rebuilding from an empty log clears stale runs so list_runs() is empty.
    let empty_log = MemEventLog::default();
    assert_eq!(ports.rebuild_from(&empty_log).unwrap(), 0);
    assert!(ports.list_runs().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// 6. MemEventLog::append_envelopes batch atomicity
// ---------------------------------------------------------------------------

#[test]
fn mem_event_log_append_envelopes_batch_is_atomic() {
    let log = MemEventLog::default();
    log.create_run("batch-1", &sample_manifest("batch"))
        .unwrap();
    let e1 = sealed_event(
        "batch-1",
        1,
        EventType::RunStarted,
        started_payload("batch"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        log.append_envelopes("batch-1", (0, ZERO_HASH), &[e1.clone(), e1]),
        Err(StoreError::AppendConflict { seq: 1 })
    ));
    assert_eq!(log.head("batch-1").unwrap(), (0, ZERO_HASH.to_string()));
    assert!(log.read_stream("batch-1", 1).unwrap().is_empty());
}
