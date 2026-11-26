//! Co-located [`SqlitePorts`] conformance, tamper, divergence, projection,
//! feature-sync, coordination, and end-to-end [`Journal`] tests (decision 0126,
//! Slice 1).

use std::cell::Cell;
use std::path::Path;

use brokkr_core::fold::Status;
use rusqlite::{params, Connection};

use super::*;
use crate::damage::Damage;
use crate::{Attribution, ManifestRefusal, SeatRecordVersion};

fn summary_feature(path: &Path, run_id: &str) -> Option<String> {
    let conn = Connection::open(path).unwrap();
    conn.query_row(
        "SELECT feature FROM run_summary WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )
    .unwrap()
}

fn assert_colocated_divergent(
    ports: &SqlitePorts<'_>,
    external: &dyn EventLog,
    expected_run_id: &str,
    batch: &[EventEnvelope],
) {
    assert!(matches!(
        ports.apply_envelopes(expected_run_id, None, batch),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == expected_run_id
    ));
    assert!(matches!(
        ports.rebuild_from(external),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == expected_run_id
    ));
}

fn assert_tampered_chain(
    ports: &SqlitePorts<'_>,
    journal: &Journal<SqlitePorts<'_>, SqlitePorts<'_>, SqlitePorts<'_>>,
    from_seq: u64,
    cause_id: &str,
    expected: impl Fn(&ChainError) -> bool,
) {
    let check_err = |err: StoreError| matches!(err, StoreError::Chain(ref c) if expected(c));
    assert!(check_err(ports.read_stream("r1", 1).unwrap_err()));
    assert!(check_err(ports.read_stream("r1", from_seq).unwrap_err()));
    assert!(check_err(journal.load_verified("r1").unwrap_err()));
    assert!(check_err(journal.export_ndjson("r1").unwrap_err()));
    assert!(check_err(
        journal
            .append_event(
                "r1",
                plain_event(
                    EventType::RunCompleted,
                    json!({}),
                    Some(cause_id),
                    "2026-11-25T10:00:10Z",
                ),
            )
            .unwrap_err()
    ));
}

#[test]
fn sqlite_store_satisfies_event_log_and_projection_ports() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("sqlite-conformance");

    assert!(matches!(
        ports.head("absent"),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));
    assert!(matches!(
        ports.read_stream("absent", 1),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));
    assert!(matches!(
        ports.append_envelopes("absent", (0, ZERO_HASH), &[]),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));
    assert!(matches!(
        ports.apply_envelopes("absent", None, &[]),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));

    ports.create_run("sql-1", &manifest).unwrap();
    assert_eq!(ports.head("sql-1").unwrap(), (0, ZERO_HASH.to_string()));
    assert_eq!(ports.manifest("sql-1").unwrap(), manifest);
    assert_eq!(ports.list_run_ids().unwrap(), vec!["sql-1".to_string()]);
    assert!(matches!(
        ports.create_run("sql-1", &manifest),
        Err(StoreError::RunExists(id)) if id == "sql-1"
    ));

    let e1 = started_event("sql-1", "sqlite-conformance", "2026-11-25T13:00:00Z");
    let e2 = sealed_event(
        "sql-1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-25T13:00:05Z",
    );

    assert!(matches!(
        ports.append_envelopes("sql-1", (1, ZERO_HASH), std::slice::from_ref(&e1)),
        Err(StoreError::HeadMoved {
            expected_seq: 1,
            found_seq: 0
        })
    ));
    assert!(matches!(
        ports.append_envelopes("sql-1", (0, "bad-hash"), std::slice::from_ref(&e1)),
        Err(StoreError::HeadMoved {
            expected_seq: 0,
            found_seq: 0
        })
    ));
    assert!(matches!(
        ports.append_envelopes("sql-1", (0, ZERO_HASH), &[e1.clone(), e1.clone()]),
        Err(StoreError::Chain(ChainError::SeqGap {
            seq: 1,
            expected: 2
        }))
    ));

    ports
        .append_envelopes("sql-1", (0, ZERO_HASH), &[e1.clone(), e2.clone()])
        .unwrap();
    ports
        .apply_envelopes("sql-1", None, std::slice::from_ref(&e2))
        .unwrap();
    assert_eq!(ports.head("sql-1").unwrap(), (2, e2.event_hash.clone()));
    assert_eq!(ports.read_stream("sql-1", 1).unwrap().len(), 2);
    assert_eq!(ports.read_stream("sql-1", 2).unwrap().len(), 1);
    assert!(ports.read_stream("sql-1", u64::MAX).unwrap().is_empty());
    assert_eq!(ports.list_runs().unwrap().len(), 1);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

#[test]
fn sqlite_store_satisfies_coordination_port() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("coord.db")).unwrap();

    let entry = NewEntry {
        payload: r#"{"recipe":"self"}"#,
        priority: 5,
        waits: &[],
    };
    assert!(matches!(
        SqlitePorts::new(&store).enqueue(&entry, "not-a-timestamp"),
        Err(StoreError::Clock { ref now }) if now == "not-a-timestamp"
    ));

    let id = SqlitePorts::new(&store)
        .enqueue(&entry, "2026-11-25T14:00:00Z")
        .unwrap();
    let listed = SqlitePorts::new(&store).list_queue().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].added_at, "2026-11-25T14:00:00Z");
    assert_eq!(listed[0].payload, entry.payload);

    let manifest = sample_manifest("cool");
    Store::create_run(&mut store, "cool-run", "cool", "self", &manifest).unwrap();
    store
        .append_next(
            "cool-run",
            EventType::RunStarted,
            started_payload("cool"),
            None,
            None,
        )
        .unwrap();
    let until = OffsetDateTime::now_utc() + time::Duration::hours(2);
    let cooling = Cooling {
        site: "implement".to_string(),
        provider: "claude".to_string(),
        route: "primary".to_string(),
        until_at: until,
        reason: "429 rate limited".to_string(),
        retry_after_seconds: Some(7200),
        rounds: 1,
    };
    store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: "cool-run",
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({"effect_id": "e1", "attempt_id": "a1", "reason": "429"}),
                causation_id: None,
                attempt_id: Some("a1".to_string()),
            },
            &[cooling],
        )
        .unwrap();

    let before = (until - time::Duration::hours(1))
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let after = (until + time::Duration::hours(1))
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();

    let ports = SqlitePorts::new(&store);
    let active = ports.active_cooldowns(&before).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].provider, "claude");
    assert_eq!(active[0].route, "primary");
    assert!(ports.active_cooldowns(&after).unwrap().is_empty());
    assert!(matches!(
        ports.active_cooldowns("bad-clock"),
        Err(StoreError::Clock { ref now }) if now == "bad-clock"
    ));
}

#[test]
fn journal_end_to_end_over_sqlite_references() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("primary.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let journal = Journal::new(&ports, &ports, &ports);

    let manifest = sample_manifest("e2e-sqlite");
    journal.create_run("e2e-1", &manifest).unwrap();
    assert_eq!(
        journal.event_log().list_run_ids().unwrap(),
        vec!["e2e-1".to_string()]
    );

    let e5 = append_valid_checkpoint_flow(&journal, "e2e-1", "e2e-sqlite");
    assert_eq!(e5.seq, 5);
    assert!(matches!(
        journal.append_event(
            "e2e-1",
            plain_event(
                EventType::EffectCheckpointed,
                invalid_checkpoint_payload(),
                None,
                "2026-11-25T15:00:15Z",
            )
        ),
        Err(StoreError::SeatRecord(ref err)) if err.seq == 6
    ));
    assert!(matches!(
        journal.append_if_head(
            "e2e-1",
            (1, &e5.previous_hash),
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "verify"}),
                None,
                "2026-11-25T15:00:20Z",
            )
        ),
        Err(StoreError::HeadMoved {
            expected_seq: 1,
            found_seq: 5
        })
    ));

    assert_eq!(journal.load_verified("e2e-1").unwrap().len(), 5);
    assert_eq!(journal.projections().list_runs().unwrap().len(), 1);
    assert_eq!(journal.reindex().unwrap(), 5);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());

    let entry = NewEntry {
        payload: r#"{"recipe":"verify"}"#,
        priority: 3,
        waits: &[],
    };
    let id = journal
        .coordination()
        .enqueue(&entry, "2026-11-25T15:01:00Z")
        .unwrap();
    assert_eq!(journal.coordination().list_queue().unwrap()[0].id, id);
    assert!(journal
        .coordination()
        .active_cooldowns("2026-11-25T15:01:00Z")
        .unwrap()
        .is_empty());

    let ndjson = journal.export_ndjson("e2e-1").unwrap();
    assert_eq!(ndjson, store.export_ndjson("e2e-1").unwrap());

    let replica = Store::open(&dir.path().join("replica.db")).unwrap();
    let replica_ports = SqlitePorts::new(&replica);
    let replica_journal = Journal::new(&replica_ports, &replica_ports, &replica_ports);
    assert_eq!(
        replica_journal
            .import_ndjson("e2e-1", &manifest, &ndjson)
            .unwrap(),
        5
    );
    assert_eq!(replica_journal.export_ndjson("e2e-1").unwrap(), ndjson);
    assert!(!replica.started_here("e2e-1").unwrap());
    assert_eq!(every_run_reads_as_its_fold(&replica), Vec::<String>::new());
}

#[test]
fn sqlite_append_envelopes_enforces_chain_integrity() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chain-guard.db");
    let store = Store::open(&db_path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("chain-guard");
    ports.create_run("cg-1", &manifest).unwrap();

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

    let mut forged = started_event("cg-1", "chain-guard", "2026-11-25T10:00:00Z");
    forged.event_hash = "0".repeat(64);
    assert!(matches!(
        ports.append_envelopes("cg-1", (0, ZERO_HASH), std::slice::from_ref(&forged)),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));

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
    let manifest = sample_manifest("chain-guard");

    let mut forged_imp = started_event("imp-1", "chain-guard", "2026-11-25T10:00:00Z");
    forged_imp.event_hash = "f".repeat(64);
    assert!(matches!(
        ports.import_run_stream("imp-1", &manifest, std::slice::from_ref(&forged_imp)),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));
    let imp_e1 = started_event("imp-2", "chain-guard", "2026-11-25T10:00:00Z");
    let imp_foreign_e2 = sealed_event(
        "other-run",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &imp_e1.event_hash,
        "2026-11-25T10:00:01Z",
    );
    assert!(matches!(
        ports.import_run_stream("imp-2", &manifest, &[imp_e1, imp_foreign_e2]),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 2 }))
    ));
    assert_eq!(sqlite_table_count(&db_path, "runs"), 0);
    assert_eq!(sqlite_table_count(&db_path, "events"), 0);
}

#[test]
fn sqlite_read_head_and_append_refuse_planted_rows_before_chain() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("before-chain.db");
    let store = Store::open(&db_path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("planted");
    for run_id in ["neg-run", "zero-run", "bad-json", "valid-at-zero"] {
        ports.create_run(run_id, &manifest).unwrap();
    }
    let e1 = started_event("valid-at-zero", "planted", "2026-11-25T10:00:00Z");
    let e1_json = serde_json::to_string(&e1).unwrap();

    let raw = Connection::open(&db_path).unwrap();
    raw.execute_batch("DROP TRIGGER events_seq_is_a_position;")
        .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES ('neg-run', -1, 'h', '{}')",
        [],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES ('zero-run', 0, 'h', '{}')",
        [],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES ('bad-json', 0, 'h', '{bad')",
        [],
    )
    .unwrap();
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES ('valid-at-zero', 0, ?1, ?2)",
        params![e1.event_hash, e1_json],
    )
    .unwrap();
    drop(raw);

    for (expected_run, bad_seq) in [("neg-run", -1i64), ("zero-run", 0i64)] {
        assert!(matches!(
            ports.head(expected_run),
            Err(StoreError::RowBeforeChain { ref run_id, seq })
                if run_id == expected_run && seq == bad_seq
        ));
        let start = started_event(expected_run, "planted", "2026-11-25T10:00:00Z");
        assert!(matches!(
            ports.append_envelopes(expected_run, (0, ZERO_HASH), std::slice::from_ref(&start)),
            Err(StoreError::RowBeforeChain { ref run_id, seq })
                if run_id == expected_run && seq == bad_seq
        ));
    }
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
fn read_stream_and_journal_refuse_tampered_sql_event_columns() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("read-stream-tamper.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let journal = sqlite_journal(&store);

    journal
        .create_run("r1", &sample_manifest("sql-read-tamper"))
        .unwrap();
    let e1 = journal
        .append_event(
            "r1",
            plain_event(
                EventType::RunStarted,
                started_payload("sql-read-tamper"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    let e2 = journal
        .append_event(
            "r1",
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                Some(&e1.event_id),
                "2026-11-25T10:00:05Z",
            ),
        )
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TRIGGER events_append_only_update;")
        .unwrap();
    raw.execute(
        "UPDATE events SET event_hash = 'forged-sql-hash' WHERE run_id = 'r1' AND seq = 2",
        [],
    )
    .unwrap();
    assert_tampered_chain(&ports, &journal, 2, &e2.event_id, |c| {
        matches!(c, ChainError::BadHash { seq: 2 })
    });

    raw.execute(
        "UPDATE events SET event_hash = ?1, seq = 99 WHERE run_id = 'r1' AND seq = 2",
        params![e2.event_hash],
    )
    .unwrap();
    drop(raw);
    assert_tampered_chain(&ports, &journal, 3, &e2.event_id, |c| {
        matches!(
            c,
            ChainError::SeqGap {
                seq: 2,
                expected: 99
            }
        )
    });
}

#[test]
fn colocated_apply_envelopes_verifies_local_stream_and_heals_projections() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("coloc-apply.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("fence-apply");
    let (e1, completed, divergent_phase) = completed_vs_phase_pair("ca-1", "fence-apply");

    // Foreign run_id envelope refused with DivergentEventLog; unknown run refused with RunNotFound.
    assert!(matches!(
        ports.apply_envelopes("other-run", None, std::slice::from_ref(&e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "other-run"
    ));
    assert!(matches!(
        ports.apply_envelopes("ca-1", None, std::slice::from_ref(&e1)),
        Err(StoreError::RunNotFound(ref id)) if id == "ca-1"
    ));
    ports.create_run("target", &manifest).unwrap();
    assert!(matches!(
        ports.apply_envelopes("target", None, std::slice::from_ref(&e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "target"
    ));

    ports
        .import_run_stream("ca-1", &manifest, &[e1.clone(), completed.clone()])
        .unwrap();
    // Before fold: divergent apply_envelopes refuses.
    assert!(matches!(
        ports.apply_envelopes("ca-1", None, &[e1.clone(), divergent_phase.clone()]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-1"
    ));

    // Shorter prefix heals unfolded summary to full local head without regressing.
    ports
        .apply_envelopes("ca-1", Some(&manifest), std::slice::from_ref(&e1))
        .unwrap();
    let (head_seq, folded_seq, status, head_hash, _) = summary_snapshot(&path, "ca-1");
    assert_eq!((head_seq, folded_seq), (2, Some(2)));
    assert_eq!(status.as_deref(), Some("completed"));
    assert_eq!(head_hash.as_deref(), Some(completed.event_hash.as_str()));

    // Divergent manifest or batch still refuses after fold; terminal fence stays intact.
    assert!(matches!(
        ports.apply_envelopes("ca-1", Some(&sample_manifest("other")), &[]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-1"
    ));
    assert!(matches!(
        ports.apply_envelopes("ca-1", None, &[e1.clone(), divergent_phase]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-1"
    ));
    assert_refuses_phase_after_completed(&ports, "ca-1", &completed);

    // Corrupting summary & deleting event_index rows heals via apply_envelopes.
    exec_raw_sql(
        &path,
        "UPDATE run_summary SET head_hash = 'stale', status = 'running', folded_seq = 0 WHERE run_id = 'ca-1';
         DELETE FROM event_index WHERE run_id = 'ca-1';",
    );
    ports
        .apply_envelopes("ca-1", None, &[e1.clone(), completed.clone()])
        .unwrap();
    assert_eq!(
        summary_snapshot(&path, "ca-1").2.as_deref(),
        Some("completed")
    );
    assert_eq!(
        summary_snapshot(&path, "ca-1").3.as_deref(),
        Some(completed.event_hash.as_str())
    );
    assert_eq!(sqlite_table_count(&path, "event_index"), 2);
    assert!(store.summary_disagreements().unwrap().is_empty());

    // Corrupting an existing event_index.event_id row in place also heals via apply_envelopes.
    exec_raw_sql(
        &path,
        "UPDATE event_index SET event_id = 'corrupt-id' WHERE run_id = 'ca-1' AND seq = 1;",
    );
    assert!(!store.summary_disagreements().unwrap().is_empty());
    ports
        .apply_envelopes("ca-1", None, &[e1, completed.clone()])
        .unwrap();
    assert!(store.summary_disagreements().unwrap().is_empty());

    // Empty local run accepts empty apply_envelopes and refuses non-empty batch.
    ports.create_run("ca-empty", &manifest).unwrap();
    ports.apply_envelopes("ca-empty", None, &[]).unwrap();
    let empty_e1 = started_event("ca-empty", "fence-apply", "2026-11-25T10:00:00Z");
    assert!(matches!(
        ports.apply_envelopes("ca-empty", None, std::slice::from_ref(&empty_e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-empty"
    ));
}

fn seed_divergent_fixture(
    path: &Path,
) -> (Store, Value, EventEnvelope, EventEnvelope, EventEnvelope) {
    let store = Store::open(path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("diverge");
    let (e1, local_e2, other_e2) = completed_vs_phase_pair("div-1", "diverge");
    ports
        .import_run_stream("div-1", &manifest, &[e1.clone(), local_e2.clone()])
        .unwrap();
    ports
        .apply_envelopes("div-1", None, &[e1.clone(), local_e2.clone()])
        .unwrap();
    ports.create_run("empty-local", &manifest).unwrap();
    (store, manifest, e1, local_e2, other_e2)
}

#[test]
fn colocated_rebuild_from_refuses_foreign_missing_and_divergent_logs() {
    let dir = tempdir().unwrap();

    // Empty local store accepts empty log and refuses external log with foreign runs.
    let empty_store = Store::open(&dir.path().join("empty.db")).unwrap();
    let empty_ports = SqlitePorts::new(&empty_store);
    let empty_log = MemEventLog::default();
    assert_eq!(empty_ports.rebuild_from(&empty_log).unwrap(), 0);

    let foreign_log = MemEventLog::default();
    let ext_e1 = started_event("from-mem", "external", "2026-11-25T13:10:00Z");
    foreign_log
        .import_run_stream("from-mem", &sample_manifest("external"), &[ext_e1])
        .unwrap();
    assert!(matches!(
        empty_ports.rebuild_from(&foreign_log),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "from-mem"
    ));

    let (local_store, manifest, e1, local_e2, other_e2) =
        seed_divergent_fixture(&dir.path().join("local.db"));
    let local_ports = SqlitePorts::new(&local_store);

    // Missing local run in external log.
    assert!(matches!(
        local_ports.rebuild_from(&empty_log),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "div-1"
    ));

    // Divergent event at seq 2 + empty local vs non-empty external.
    let divergent_log = MemEventLog::default();
    divergent_log
        .import_run_stream("div-1", &manifest, &[e1.clone(), other_e2])
        .unwrap();
    divergent_log.create_run("empty-local", &manifest).unwrap();
    let err = local_ports.rebuild_from(&divergent_log).unwrap_err();
    assert!(matches!(
        err,
        StoreError::DivergentEventLog { ref run_id } if run_id == "div-1"
    ));
    assert_eq!(
        err.to_string(),
        "event log for run 'div-1' diverges from local store state"
    );

    let nonempty_for_empty = MemEventLog::default();
    nonempty_for_empty
        .import_run_stream("div-1", &manifest, &[e1.clone(), local_e2.clone()])
        .unwrap();
    let empty_ext = started_event("empty-local", "diverge", "2026-11-25T10:00:00Z");
    nonempty_for_empty
        .import_run_stream("empty-local", &manifest, &[empty_ext])
        .unwrap();
    assert!(matches!(
        local_ports.rebuild_from(&nonempty_for_empty),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "empty-local"
    ));

    // Superset external log.
    let superset = MemEventLog::default();
    superset
        .import_run_stream("div-1", &manifest, &[e1.clone(), local_e2.clone()])
        .unwrap();
    superset.create_run("empty-local", &manifest).unwrap();
    let extra_e1 = started_event("extra-2", "extra", "2026-11-25T10:01:00Z");
    superset
        .import_run_stream("extra-2", &sample_manifest("extra"), &[extra_e1])
        .unwrap();
    assert!(matches!(
        local_ports.rebuild_from(&superset),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "extra-2"
    ));

    // Tampered external log refused before touching local store.
    let tampered_ext = MemEventLog::default();
    tampered_ext
        .import_run_stream("div-1", &manifest, &[e1, local_e2])
        .unwrap();
    tampered_ext.tamper_payload("div-1", 0, json!({"feature": "evil"}));
    assert!(matches!(
        local_ports.rebuild_from(&tampered_ext),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));
}

#[test]
fn colocated_rebuild_from_refuses_tampered_sql_columns_and_orphan_events() {
    let dir = tempdir().unwrap();
    let local_path = dir.path().join("local-tamper.db");
    let (local_store, manifest, e1, local_e2, _) = seed_divergent_fixture(&local_path);
    let local_ports = SqlitePorts::new(&local_store);
    let stream = [e1, local_e2.clone()];

    let clean = MemEventLog::default();
    clean
        .import_run_stream("div-1", &manifest, &stream)
        .unwrap();
    clean.create_run("empty-local", &manifest).unwrap();

    let raw = Connection::open(&local_path).unwrap();
    raw.execute_batch("DROP TRIGGER events_append_only_update;")
        .unwrap();
    for (sql, expect_chain) in [
        (
            "UPDATE events SET event_hash = 'forged-sql-hash' WHERE run_id = 'div-1' AND seq = 2",
            Box::new(|c: &ChainError| matches!(c, ChainError::BadHash { seq: 2 }))
                as Box<dyn Fn(&ChainError) -> bool>,
        ),
        (
            "UPDATE events SET seq = 99 WHERE run_id = 'div-1' AND seq = 2",
            Box::new(|c: &ChainError| {
                matches!(
                    c,
                    ChainError::SeqGap {
                        seq: 2,
                        expected: 99
                    }
                )
            }),
        ),
    ] {
        raw.execute(
            "UPDATE events SET event_hash = ?1 WHERE run_id = 'div-1' AND seq = 2",
            params![local_e2.event_hash],
        )
        .unwrap();
        raw.execute(sql, []).unwrap();
        assert_colocated_divergent(&local_ports, &clean, "div-1", &stream);
        assert!(matches!(
            local_ports.rebuild_from(&local_ports),
            Err(StoreError::Chain(ref c)) if expect_chain(c)
        ));
    }

    // Restore div-1 and plant an orphan event row without a `runs` row -> DivergentEventLog.
    raw.execute(
        "UPDATE events SET seq = 2 WHERE run_id = 'div-1' AND seq = 99",
        [],
    )
    .unwrap();
    raw.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    let orphan_e1 = started_event("orphan-1", "orphan", "2026-11-25T10:00:00Z");
    raw.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES ('orphan-1', 1, ?1, ?2)",
        params![
            orphan_e1.event_hash,
            serde_json::to_string(&orphan_e1).unwrap()
        ],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        local_ports.rebuild_from(&clean),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "orphan-1"
    ));
}

#[test]
fn colocated_rebuild_preserves_empty_runs_created_at_ordinals_and_standing_ack() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("coloc-rebuild.db");
    let mut store = Store::open(&path).unwrap();

    {
        let ports = SqlitePorts::new(&store);
        let journal = sqlite_journal(&store);
        ports
            .create_run("empty-local", &sample_manifest("empty"))
            .unwrap();
        journal
            .create_run("skew-1", &sample_manifest("skew"))
            .unwrap();
        let e5 = append_valid_checkpoint_flow(&journal, "skew-1", "skew");
        assert_eq!(e5.seq, 5);
        journal
            .append_event(
                "skew-1",
                plain_event(
                    EventType::RunParked,
                    json!({"reason": "needs operator"}),
                    Some(&e5.event_id),
                    "2026-11-25T10:00:10Z",
                ),
            )
            .unwrap();
        exec_raw_sql(
            &path,
            "UPDATE runs SET created_at = '2025-06-01T00:00:00Z' WHERE run_id = 'skew-1';",
        );
    }

    // Record a standing quarantine acknowledgement on skew-1 and verify rebuild preserves it.
    let (head_seq, head_hash) = store.head_hash("skew-1").unwrap();
    assert_eq!(head_seq, 6);
    store
        .acknowledge_quarantine(
            "skew-1",
            (head_seq, &head_hash),
            Attribution {
                operator: "valentin",
                reason: "reviewed",
            },
        )
        .unwrap();

    {
        let ports = SqlitePorts::new(&store);
        assert_eq!(ports.rebuild_from(&ports).unwrap(), 6);
        assert!(store.summary_disagreements().unwrap().is_empty());
        assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());
        assert_eq!(summary_snapshot(&path, "skew-1").4, Some(1));
        assert_eq!(summary_snapshot(&path, "empty-local").0, 0);
        assert_eq!(summary_snapshot(&path, "empty-local").1, Some(0));
    }

    // Subsequent Store::append_next on the preserved empty run works with zero disagreements.
    store
        .append_next(
            "empty-local",
            EventType::RunStarted,
            started_payload("empty"),
            None,
            None,
        )
        .unwrap();
    assert!(store.summary_disagreements().unwrap().is_empty());
    assert_eq!(summary_snapshot(&path, "empty-local").0, 1);
}

#[test]
fn colocated_projection_propagates_token_count_out_of_range() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("tokens.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let journal = sqlite_journal(&store);
    let overflow = i64::MAX as u64 + 1;

    let e1 = started_event("tok-1", "tok", "2026-11-25T10:00:00Z");
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
    ports
        .import_run_stream("tok-1", &sample_manifest("tok"), &[e1.clone(), e2.clone()])
        .unwrap();

    assert!(matches!(
        ports.apply_envelopes("tok-1", None, &[e1, e2.clone()]),
        Err(StoreError::TokenCountOutOfRange(2, "input_tokens", count)) if count == overflow
    ));
    assert!(matches!(
        ports.rebuild_from(&ports),
        Err(StoreError::TokenCountOutOfRange(2, "input_tokens", count)) if count == overflow
    ));

    journal
        .create_run("tok-lag", &sample_manifest("tok"))
        .unwrap();
    journal
        .append_event(
            "tok-lag",
            plain_event(
                EventType::RunStarted,
                started_payload("tok"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    let lag_err = journal
        .append_event(
            "tok-lag",
            plain_event(
                EventType::EffectCheckpointed,
                e2.payload,
                None,
                "2026-11-25T10:00:05Z",
            ),
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
}

#[test]
fn colocated_feature_sync_precedence_and_late_run_started() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("feature-sync.db");
    let mut store = Store::open(&path).unwrap();
    let manifest_no_feat = json!({"engine": "0.15.0", "bundle_name": "self"});
    let manifest_default = sample_manifest("manifest-default");

    // Pre-existing runs keep runs.feature immutable across reindex so Store's
    // cached carried listing never goes stale relative to runs.feature.
    store
        .create_run("r-empty", "", "self", &manifest_no_feat)
        .unwrap();
    store
        .append_next("r-empty", EventType::RunStarted, json!({}), None, None)
        .unwrap();
    store
        .create_run("r-pre", "manifest-default", "self", &manifest_default)
        .unwrap();
    store
        .append_next(
            "r-pre",
            EventType::RunStarted,
            json!({"manifest": {"engine": "0.15.0", "feature": "from-reindex"}}),
            None,
            None,
        )
        .unwrap();

    let ports = SqlitePorts::new(&store);
    let journal = sqlite_journal(&store);

    // Journal append syncs feature from RunStarted (including legacy /manifest/feature fallback).
    journal.create_run("r-legacy", &manifest_no_feat).unwrap();
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

    // Late RunStarted past seq 1 never changes listed feature.
    let e1_phase = sealed_event(
        "r-late",
        1,
        EventType::PhaseEntered,
        json!({"phase": "plan"}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    let e2_late = sealed_event(
        "r-late",
        2,
        EventType::RunStarted,
        json!({"feature": "late-hijack"}),
        &e1_phase.event_hash,
        "2026-11-25T10:00:05Z",
    );
    assert_eq!(
        super::super::sqlite::started_feature_of(std::slice::from_ref(&e2_late)),
        None
    );
    ports
        .import_run_stream("r-late", &manifest_default, &[e1_phase, e2_late])
        .unwrap();

    assert_eq!(journal.reindex().unwrap(), 5);
    let runs: BTreeMap<String, String> = ports
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|(id, feat, _)| (id, feat))
        .collect();
    assert_eq!(
        runs.get("r-pre").map(String::as_str),
        Some("manifest-default")
    );
    assert_eq!(runs.get("r-empty").map(String::as_str), Some(""));
    assert_eq!(
        runs.get("r-legacy").map(String::as_str),
        Some("legacy-goal")
    );
    assert_eq!(
        runs.get("r-late").map(String::as_str),
        Some("manifest-default")
    );
    assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());
    assert!(store.summary_disagreements().unwrap().is_empty());

    // Continuing to append through Store's carried account stays consistent.
    store
        .append_next(
            "r-pre",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        summary_feature(&path, "r-pre").as_deref(),
        Some("manifest-default")
    );
    assert!(store.summary_disagreements().unwrap().is_empty());
}

#[test]
fn colocated_list_runs_matches_store_before_fold_tie_break_and_legacy_ro() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("list-parity.db");
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

    let expected = vec![
        ("run-z".to_string(), "feat-z".to_string(), ts.to_string()),
        ("run-a".to_string(), "feat-a".to_string(), ts.to_string()),
    ];
    assert_eq!(store.list_runs().unwrap(), expected);
    assert_eq!(ports.list_runs().unwrap(), expected);

    ports.apply_envelopes("run-z", None, &[ez]).unwrap();
    ports.apply_envelopes("run-a", None, &[ea]).unwrap();
    assert_eq!(ports.list_runs().unwrap(), expected);
    assert_eq!(ports.rebuild_from(&ports).unwrap(), 2);
    assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());

    // Legacy read-only journal without run_summary table still lists runs cleanly.
    let legacy_path = dir.path().join("legacy-ro.db");
    let raw = Connection::open(&legacy_path).unwrap();
    raw.execute_batch(crate::schema::MIGRATION_V1).unwrap();
    raw.execute(
        "INSERT INTO meta (key, value) VALUES ('database_schema', '1')",
        [],
    )
    .unwrap();
    drop(raw);

    let ro_empty = Store::open_read_only(&legacy_path).unwrap();
    assert!(SqlitePorts::new(&ro_empty).list_runs().unwrap().is_empty());
    drop(ro_empty);

    exec_raw_sql(
        &legacy_path,
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES ('r-leg', 'legacy-feat', 'self', '{}', '2026-11-25T10:00:00Z');",
    );
    let ro_populated = Store::open_read_only(&legacy_path).unwrap();
    let legacy_expected = vec![(
        "r-leg".to_string(),
        "legacy-feat".to_string(),
        "2026-11-25T10:00:00Z".to_string(),
    )];
    assert_eq!(
        SqlitePorts::new(&ro_populated).list_runs().unwrap(),
        legacy_expected
    );
    assert_eq!(ro_populated.list_runs().unwrap(), legacy_expected);
}

#[test]
fn sqlite_append_envelopes_refuses_tampered_and_damaged_local_prefix() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("append-prefix-tamper.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let e1 = started_event("r1", "prefix", "2026-11-25T10:00:00Z");
    let e2 = sealed_event(
        "r1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "plan"}),
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
        .import_run_stream("r1", &sample_manifest("prefix"), &[e1, e2.clone()])
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TRIGGER events_append_only_update;")
        .unwrap();

    // Tampered SQL event_hash is refused before head comparison or commit.
    raw.execute(
        "UPDATE events SET event_hash = 'forged-sql-hash' WHERE run_id = 'r1' AND seq = 2",
        [],
    )
    .unwrap();
    for expected_hash in ["forged-sql-hash", e2.event_hash.as_str()] {
        assert!(matches!(
            ports.append_envelopes("r1", (2, expected_hash), std::slice::from_ref(&e3)),
            Err(StoreError::Chain(ChainError::BadHash { seq: 2 }))
        ));
    }

    // Tampered envelope JSON payload with intact SQL event_hash is refused.
    let mut tampered = e2.clone();
    tampered.payload = json!({"phase": "tampered"});
    raw.execute(
        "UPDATE events SET event_hash = ?1, envelope = ?2 WHERE run_id = 'r1' AND seq = 2",
        params![e2.event_hash, serde_json::to_string(&tampered).unwrap()],
    )
    .unwrap();
    assert!(matches!(
        ports.append_envelopes("r1", (2, &e2.event_hash), std::slice::from_ref(&e3)),
        Err(StoreError::Chain(ChainError::BadHash { seq: 2 }))
    ));

    // Unparseable envelope JSON is refused as Damaged rather than committing.
    raw.execute(
        "UPDATE events SET envelope = 'not-json' WHERE run_id = 'r1' AND seq = 2",
        [],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        ports.append_envelopes("r1", (2, &e2.event_hash), std::slice::from_ref(&e3)),
        Err(StoreError::Damaged {
            damage: Damage::Envelope { ref run_id, seq: 2 },
            ..
        }) if run_id == "r1"
    ));
    assert_eq!(sqlite_table_count(&path, "events"), 2);
}

fn assert_refuses_phase_after_completed(
    ports: &SqlitePorts<'_>,
    expected_run_id: &str,
    completed: &EventEnvelope,
) {
    let after_terminal = sealed_event(
        expected_run_id,
        completed.seq + 1,
        EventType::PhaseEntered,
        json!({"phase": "illegal"}),
        &completed.event_hash,
        "2026-11-25T10:00:10Z",
    );
    assert!(matches!(
        ports.append_envelopes(
            expected_run_id,
            (completed.seq, &completed.event_hash),
            std::slice::from_ref(&after_terminal),
        ),
        Err(StoreError::AfterTerminal {
            ref run_id,
            status: Status::Completed,
            event_type: EventType::PhaseEntered,
        }) if run_id == expected_run_id
    ));
}

fn assert_write_locked(path: &Path) {
    let probe = Connection::open(path).unwrap();
    probe.busy_timeout(std::time::Duration::ZERO).unwrap();
    let err = probe.execute_batch("BEGIN IMMEDIATE").unwrap_err();
    assert_eq!(
        err.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy)
    );
}

#[test]
fn colocated_rebuild_from_refuses_duplicate_run_ids_and_holds_write_lock() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("rebuild-dup-lock.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let e1 = started_event("r1", "lock", "2026-11-25T10:00:00Z");
    ports
        .import_run_stream("r1", &sample_manifest("lock"), std::slice::from_ref(&e1))
        .unwrap();

    // Duplicate run_ids from list_run_ids() are refused with DivergentEventLog.
    let dup_log = InterceptingLog {
        inner: &ports,
        on_head: None,
        on_list_run_ids: None,
        on_read_stream: None,
        override_run_ids: Some(vec!["r1".to_string(), "r1".to_string()]),
    };
    assert!(matches!(
        ports.rebuild_from(&dup_log),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));

    // Deterministically prove rebuild_from already holds BEGIN IMMEDIATE during log scans.
    let list_checks = Cell::new(0usize);
    let read_checks = Cell::new(0usize);
    let hooked = InterceptingLog {
        inner: &ports,
        on_head: None,
        on_list_run_ids: Some(Box::new(|| {
            assert_write_locked(&path);
            list_checks.set(list_checks.get() + 1);
        })),
        on_read_stream: Some(Box::new(|| {
            assert_write_locked(&path);
            read_checks.set(read_checks.get() + 1);
        })),
        override_run_ids: None,
    };
    assert_eq!(ports.rebuild_from(&hooked).unwrap(), 1);
    assert_eq!(list_checks.get(), 1);
    assert_eq!(read_checks.get(), 1);

    // After rebuild_from commits, another connection can append and rebuild sees both events.
    let e2 = sealed_event(
        "r1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    let reader = Store::open(&path).unwrap();
    let reader_ports = SqlitePorts::new(&reader);
    reader_ports
        .append_envelopes("r1", (1, &e1.event_hash), &[e2])
        .unwrap();
    assert_eq!(ports.rebuild_from(&reader_ports).unwrap(), 2);
    assert!(store.summary_disagreements().unwrap().is_empty());
}

#[test]
fn sqlite_append_envelopes_refuses_after_terminal_despite_damaged_summary() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("damaged-summary-terminal.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let (e1, completed, _) = completed_vs_phase_pair("term-dmg", "term-dmg");
    ports
        .import_run_stream(
            "term-dmg",
            &sample_manifest("term-dmg"),
            &[e1.clone(), completed.clone()],
        )
        .unwrap();
    ports
        .apply_envelopes("term-dmg", None, &[e1, completed.clone()])
        .unwrap();
    assert_eq!(
        summary_snapshot(&path, "term-dmg").2.as_deref(),
        Some("completed")
    );

    // Damage run_summary to claim status='running' at head seq 2.
    exec_raw_sql(
        &path,
        "UPDATE run_summary SET status = 'running', folded_seq = 2 WHERE run_id = 'term-dmg';",
    );

    assert_refuses_phase_after_completed(&ports, "term-dmg", &completed);
    assert_eq!(sqlite_table_count(&path, "events"), 2);
}

#[test]
fn colocated_rebuild_from_refuses_empty_run_manifest_mismatch() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty-manifest-mismatch.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    ports
        .create_run("empty-same-id", &sample_manifest("local-feature"))
        .unwrap();

    let external = MemEventLog::default();
    external
        .create_run("empty-same-id", &sample_manifest("other-feature"))
        .unwrap();

    assert!(matches!(
        ports.rebuild_from(&external),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "empty-same-id"
    ));
}

#[test]
fn sqlite_append_and_import_refuse_invalid_embedded_run_started_manifest() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("embedded-manifest.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let ts = "2026-11-25T10:00:00Z";
    let bad_payload = json!({"manifest": {"engine": "0.15.0", "feature": 42}});

    // Direct SqlitePorts::append_envelopes refuses numeric manifest.feature without committing.
    ports
        .create_run("r-append", &sample_manifest("ok"))
        .unwrap();
    let bad_append = sealed_event(
        "r-append",
        1,
        EventType::RunStarted,
        bad_payload.clone(),
        ZERO_HASH,
        ts,
    );
    assert!(matches!(
        ports.append_envelopes("r-append", (0, ZERO_HASH), std::slice::from_ref(&bad_append)),
        Err(StoreError::InvalidManifest {
            ref run_id,
            reason: ManifestRefusal::FeatureNotString,
        }) if run_id == "r-append"
    ));
    assert_eq!(ports.head("r-append").unwrap(), (0, ZERO_HASH.to_string()));
    assert_eq!(sqlite_table_count(&path, "events"), 0);

    // Direct SqlitePorts::import_run_stream refuses numeric manifest.feature even when the
    // separate manifest argument is valid, leaving zero committed rows.
    let bad_import = sealed_event(
        "r-import",
        1,
        EventType::RunStarted,
        bad_payload,
        ZERO_HASH,
        ts,
    );
    assert!(matches!(
        ports.import_run_stream(
            "r-import",
            &sample_manifest("ok"),
            std::slice::from_ref(&bad_import),
        ),
        Err(StoreError::InvalidManifest {
            ref run_id,
            reason: ManifestRefusal::FeatureNotString,
        }) if run_id == "r-import"
    ));
    assert!(matches!(
        ports.head("r-import"),
        Err(StoreError::RunNotFound(ref id)) if id == "r-import"
    ));
    assert_eq!(sqlite_table_count(&path, "runs"), 1);
    assert_eq!(sqlite_table_count(&path, "events"), 0);
}

fn bad_seat_streams(run_id: &str) -> (EventEnvelope, EventEnvelope, EventEnvelope, EventEnvelope) {
    let start = started_event(run_id, "seat-fence", "2026-11-25T10:00:00Z");
    let bad_cp = sealed_event(
        run_id,
        2,
        EventType::EffectCheckpointed,
        invalid_checkpoint_payload(),
        &start.event_hash,
        "2026-11-25T10:00:01Z",
    );
    let panel_cp = sealed_event(
        run_id,
        2,
        EventType::EffectCheckpointed,
        panel_member_checkpoint(),
        &start.event_hash,
        "2026-11-25T10:00:02Z",
    );
    let bad_failed = sealed_event(
        run_id,
        3,
        EventType::EffectFailed,
        bad_settled_payload(),
        &panel_cp.event_hash,
        "2026-11-25T10:00:03Z",
    );
    (start, bad_cp, panel_cp, bad_failed)
}

fn v8_stream(run_id: &str) -> [EventEnvelope; 2] {
    let start = started_event(run_id, "seat-fence", "2026-11-25T10:00:00Z");
    let succ = sealed_event(
        run_id,
        2,
        EventType::EffectSucceeded,
        v8_only_succeeded_payload(),
        &start.event_hash,
        "2026-11-25T10:00:01Z",
    );
    [start, succ]
}

#[test]
fn sqlite_append_envelopes_refuses_malformed_seat_records_and_settled_members() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("seat-record-append.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let (e1, bad_cp, panel_cp, bad_failed) = bad_seat_streams("sr-append");

    ports
        .create_run("sr-append", &sample_manifest("seat-fence"))
        .unwrap();
    ports
        .append_envelopes("sr-append", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();
    assert!(matches!(
        ports.append_envelopes("sr-append", (1, &e1.event_hash), &[bad_cp]),
        Err(StoreError::SeatRecord(ref err)) if err.seq == 2
    ));
    assert!(matches!(
        ports.append_envelopes(
            "sr-append",
            (1, &e1.event_hash),
            &[panel_cp.clone(), bad_failed.clone()]
        ),
        Err(StoreError::SeatRecord(ref err))
            if err.seq == 3 && err.settled_member.as_deref().map(String::as_str) == Some("alpha")
    ));
    assert_eq!(ports.head("sr-append").unwrap(), (1, e1.event_hash.clone()));

    ports
        .append_envelopes(
            "sr-append",
            (1, &e1.event_hash),
            std::slice::from_ref(&panel_cp),
        )
        .unwrap();
    assert!(matches!(
        ports.append_envelopes("sr-append", (2, &panel_cp.event_hash), &[bad_failed]),
        Err(StoreError::SeatRecord(ref err))
            if err.seq == 3 && err.settled_member.as_deref().map(String::as_str) == Some("alpha")
    ));
    assert_eq!(ports.head("sr-append").unwrap(), (2, panel_cp.event_hash));

    // Persisted v1 manifest wins over v8 RunStarted engine on direct SQLite append.
    ports
        .create_run("sr-v1", &json!({"engine": "0.1.0", "bundle_name": "self"}))
        .unwrap();
    assert!(matches!(
        ports.append_envelopes("sr-v1", (0, ZERO_HASH), &v8_stream("sr-v1")),
        Err(StoreError::SeatRecord(ref err))
            if err.seq == 2 && err.contract == SeatRecordVersion::V1.contract()
    ));
    assert_eq!(ports.head("sr-v1").unwrap(), (0, ZERO_HASH.to_string()));
    assert_eq!(sqlite_table_count(&path, "events"), 2);
}

#[test]
fn sqlite_import_run_stream_refuses_malformed_seat_records_and_settled_members() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("seat-record-import.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let manifest = sample_manifest("seat-fence");

    let (e1, bad_cp, _, _) = bad_seat_streams("sr-imp-cp");
    assert!(matches!(
        ports.import_run_stream("sr-imp-cp", &manifest, &[e1, bad_cp]),
        Err(StoreError::SeatRecord(ref err)) if err.seq == 2
    ));

    let (e1, panel_cp, bad_failed) = {
        let (e1, _, cp, failed) = bad_seat_streams("sr-imp-set");
        (e1, cp, failed)
    };
    assert!(matches!(
        ports.import_run_stream("sr-imp-set", &manifest, &[e1, panel_cp, bad_failed]),
        Err(StoreError::SeatRecord(ref err))
            if err.seq == 3 && err.settled_member.as_deref().map(String::as_str) == Some("alpha")
    ));

    let v1_manifest = json!({"engine": "0.1.0", "bundle_name": "self"});
    assert!(matches!(
        ports.import_run_stream("sr-imp-v1", &v1_manifest, &v8_stream("sr-imp-v1")),
        Err(StoreError::SeatRecord(ref err))
            if err.seq == 2 && err.contract == SeatRecordVersion::V1.contract()
    ));

    assert_eq!(sqlite_table_count(&path, "runs"), 0);
    assert_eq!(sqlite_table_count(&path, "events"), 0);
}
