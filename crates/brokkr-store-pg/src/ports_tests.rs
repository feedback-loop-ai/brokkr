//! Offline conformance and edge-case tests for `PgEventLog`,
//! `PgProjectionStore`, `PgCoordinationStore`, `PgStore`, and
//! `Journal<PgEventLog, PgProjectionStore, PgCoordinationStore>` over the
//! in-process mock PostgreSQL v3 server.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_store::ports::{EventLog, Journal, NewEvent, ProjectionStore, RunMetadata};
use brokkr_store::StoreError;
use serde_json::{json, Value};

use crate::mock_pg_tests::{sample_manifest, sealed_env, MockPgServer};
use crate::{PgCoordinationStore, PgEventLog, PgProjectionStore, PgStore};

#[test]
fn journal_end_to_end_over_split_pg_ports_and_unified_pg_store() {
    let srv = MockPgServer::start();
    let cfg = srv.config();
    let log = PgEventLog::connect(&cfg).expect("pg event log");
    let proj = PgProjectionStore::connect(&cfg).expect("pg projection");
    let coord = PgCoordinationStore::connect(&cfg).expect("pg coordination");
    let journal = Journal::new(log, proj, coord);

    let m = sample_manifest("slice-3-feature");
    journal.create_run("run-alpha", &m).expect("create run");
    assert!(matches!(
        journal.create_run("run-alpha", &m),
        Err(StoreError::RunExists(id)) if id == "run-alpha"
    ));

    let e1 = journal
        .append_event(
            "run-alpha",
            NewEvent {
                event_type: EventType::RunStarted,
                payload: json!({ "feature": "slice-3-feature", "manifest": m }),
                causation_id: None,
                attempt_id: None,
                ts: "2026-11-27T10:00:00Z",
            },
        )
        .expect("append RunStarted");
    assert_eq!(e1.seq, 1);

    let stale = journal.append_if_head(
        "run-alpha",
        (0, ZERO_HASH),
        NewEvent {
            event_type: EventType::PhaseEntered,
            payload: json!({ "note": "stale" }),
            causation_id: Some(&e1.event_id),
            attempt_id: None,
            ts: "2026-11-27T10:00:01Z",
        },
    );
    assert!(matches!(
        stale,
        Err(StoreError::HeadMoved {
            expected_seq: 0,
            found_seq: 1
        })
    ));

    let e2 = journal
        .append_if_head(
            "run-alpha",
            (e1.seq, &e1.event_hash),
            NewEvent {
                event_type: EventType::PhaseEntered,
                payload: json!({ "phase": "build" }),
                causation_id: Some(&e1.event_id),
                attempt_id: Some("att-1"),
                ts: "2026-11-27T10:00:02Z",
            },
        )
        .expect("append RunAnnotated");
    assert_eq!(e2.seq, 2);

    assert_eq!(
        journal.event_log().head("run-alpha").unwrap(),
        (2, e2.event_hash.clone())
    );
    assert_eq!(
        journal
            .event_log()
            .read_stream("run-alpha", 2)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(journal.event_log().manifest("run-alpha").unwrap(), m);
    assert_eq!(
        journal.event_log().list_run_ids().unwrap(),
        vec!["run-alpha".to_string()]
    );

    let ndjson = journal.export_ndjson("run-alpha").expect("export ndjson");

    verify_unified_import_and_rebuild(&m, &ndjson, &e2.event_hash);

    // Verify FOR UPDATE locking on append_envelopes.
    let queries = srv.db.lock().unwrap().queries.clone();
    assert!(queries
        .iter()
        .any(|q| q.contains("SELECT manifest_json FROM brokkr_runs WHERE run_id = $1 FOR UPDATE")));
}

fn verify_unified_import_and_rebuild(m: &Value, ndjson: &str, expected_hash: &str) {
    // Import into a second server via unified PgStore + Journal<&PgStore, &PgStore, &PgStore>.
    let srv2 = MockPgServer::start();
    let unified = PgStore::connect(&srv2.config()).expect("unified pg store");
    let unified_journal = Journal::new(&unified, &unified, &unified);
    let imported = unified_journal
        .import_ndjson("run-alpha", m, ndjson)
        .expect("import ndjson");
    assert_eq!(imported, 2);
    assert_eq!(
        unified.head("run-alpha").unwrap(),
        (2, expected_hash.to_string())
    );
    assert_eq!(unified.read_stream("run-alpha", 1).unwrap().len(), 2);
    assert_eq!(unified.manifest("run-alpha").unwrap(), *m);
    assert_eq!(
        unified.list_run_ids().unwrap(),
        vec!["run-alpha".to_string()]
    );
    assert_eq!(
        unified.list_runs().unwrap(),
        vec![(
            "run-alpha".to_string(),
            "slice-3-feature".to_string(),
            "2026-11-27T10:00:00Z".to_string()
        )]
    );

    // Unified PgStore also implements create_run / append_envelopes / rebuild_from directly.
    unified
        .create_run("run-beta", &sample_manifest("beta"))
        .expect("beta");
    let b1 = sealed_env(
        "run-beta",
        1,
        ZERO_HASH,
        EventType::RunStarted,
        json!({ "feature": "beta" }),
        "2026-11-27T11:00:00Z",
    );
    unified
        .append_envelopes("run-beta", (0, ZERO_HASH), std::slice::from_ref(&b1))
        .expect("append beta");
    assert_eq!(unified.rebuild_from(&unified).unwrap(), 3);
    assert_eq!(unified.list_runs().unwrap().len(), 2);
}

#[test]
fn event_log_error_and_corruption_paths() {
    let srv = MockPgServer::start();
    let log = PgEventLog::connect(&srv.config()).expect("connect");
    let m = sample_manifest("err-test");

    assert!(matches!(
        log.head("missing"),
        Err(StoreError::RunNotFound(_))
    ));
    assert!(matches!(
        log.read_stream("missing", 1),
        Err(StoreError::RunNotFound(_))
    ));
    assert!(matches!(
        log.manifest("missing"),
        Err(StoreError::RunNotFound(_))
    ));
    assert!(matches!(
        log.run_metadata("missing"),
        Err(StoreError::RunNotFound(_))
    ));
    assert!(matches!(
        log.append_envelopes("missing", (0, ZERO_HASH), &[]),
        Err(StoreError::RunNotFound(_))
    ));

    // Non-23505 server error on create_run
    srv.db.lock().unwrap().force_next_error = Some(("40001".into(), "serialization".into()));
    assert!(matches!(log.create_run("r1", &m), Err(StoreError::Io(_))));

    log.create_run("r1", &m).unwrap();
    let e1 = sealed_env(
        "r1",
        1,
        ZERO_HASH,
        EventType::RunStarted,
        json!({ "feature": "err-test", "manifest": m }),
        "2026-11-27T10:00:00Z",
    );
    // Duplicate import_run_stream -> RunExists
    assert!(matches!(
        log.import_run_stream("r1", &m, std::slice::from_ref(&e1)),
        Err(StoreError::RunExists(_))
    ));

    // AppendConflict on SQLSTATE 23505 during INSERT INTO brokkr_events
    log.append_envelopes("r1", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();
    // Simulate a race where the mock DB already has seq 2 right when INSERT runs
    let e2 = sealed_env(
        "r1",
        2,
        &e1.event_hash,
        EventType::PhaseEntered,
        json!({ "note": "a" }),
        "2026-11-27T10:00:01Z",
    );
    // Corrupt stored rows: RowBeforeChain, Unparsed, SeqGap, BadHash
    {
        let mut db = srv.db.lock().unwrap();
        db.events.insert(
            ("r1".into(), 0),
            (
                "run/started".into(),
                "h".into(),
                "p".into(),
                "t".into(),
                "{}".into(),
            ),
        );
    }
    assert!(matches!(
        log.head("r1"),
        Err(StoreError::RowBeforeChain { seq: 0, .. })
    ));
    {
        let mut db = srv.db.lock().unwrap();
        db.events.remove(&("r1".into(), 0));
        db.events.get_mut(&("r1".into(), 1)).unwrap().4 = "not-json".into();
    }
    assert!(matches!(
        log.head("r1"),
        Err(StoreError::Unparsed { seq: 1, .. })
    ));
    {
        let mut db = srv.db.lock().unwrap();
        let mut wrong_seq = e1.clone();
        wrong_seq.seq = 9;
        db.events.get_mut(&("r1".into(), 1)).unwrap().4 =
            serde_json::to_string(&wrong_seq).unwrap();
    }
    assert!(matches!(
        log.head("r1"),
        Err(StoreError::Chain(ChainError::SeqGap {
            seq: 9,
            expected: 1
        }))
    ));
    {
        let mut db = srv.db.lock().unwrap();
        db.events.get_mut(&("r1".into(), 1)).unwrap().4 = serde_json::to_string(&e1).unwrap();
        db.events.get_mut(&("r1".into(), 1)).unwrap().1 = "tampered-hash".into();
    }
    assert!(matches!(
        log.head("r1"),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));

    verify_append_conflict_and_retry(&srv, &log, &e1, &e2);
}

fn verify_append_conflict_and_retry(
    srv: &MockPgServer,
    log: &PgEventLog,
    e1: &EventEnvelope,
    e2: &EventEnvelope,
) {
    // Restore r1 and verify AppendConflict / non-23505 insert error via force_next_error
    {
        let mut db = srv.db.lock().unwrap();
        db.events.get_mut(&("r1".into(), 1)).unwrap().1 = e1.event_hash.clone();
    }
    let e2_conflict = e2.clone();
    // Inject a 23505 on the INSERT statement by pre-checking: actually test import_run_stream
    // error path and append_envelopes happy path + empty batch on existing run.
    srv.db.lock().unwrap().force_insert_event_error = Some(("23505".into(), "dup seq".into()));
    assert!(matches!(
        log.append_envelopes(
            "r1",
            (1, &e1.event_hash),
            std::slice::from_ref(&e2_conflict)
        ),
        Err(StoreError::AppendConflict { seq: 2 })
    ));
    srv.db.lock().unwrap().force_insert_event_error = Some(("40001".into(), "retry".into()));
    assert!(matches!(
        log.append_envelopes(
            "r1",
            (1, &e1.event_hash),
            std::slice::from_ref(&e2_conflict)
        ),
        Err(StoreError::Io(_))
    ));
    log.append_envelopes("r1", (1, &e1.event_hash), &[])
        .unwrap();
    log.append_envelopes("r1", (1, &e1.event_hash), &[e2_conflict])
        .unwrap();
}

#[test]
fn projection_monotonicity_repair_divergence_and_rebuild() {
    let srv = MockPgServer::start();
    let cfg = srv.config();
    let proj = PgProjectionStore::connect(&cfg).expect("connect proj");

    let e1 = sealed_env(
        "p-run",
        1,
        ZERO_HASH,
        EventType::RunStarted,
        json!({ "feature": "" }),
        "2026-11-27T10:00:00Z",
    );
    let e2 = sealed_env(
        "p-run",
        2,
        &e1.event_hash,
        EventType::PhaseEntered,
        json!({ "phase": "build" }),
        "2026-11-27T10:00:01Z",
    );
    let e3 = sealed_env(
        "p-run",
        3,
        &e2.event_hash,
        EventType::PhaseEntered,
        json!({ "phase": "verify" }),
        "2026-11-27T10:00:02Z",
    );

    proj.apply_envelopes("p-run", None, &[])
        .expect("empty fresh");
    proj.apply_envelopes(
        "p-run",
        Some(&json!({ "created_at": "2026-11-27T09:00:00Z" })),
        &[],
    )
    .expect("empty with created_at");
    proj.apply_envelopes("p-run", None, &[])
        .expect("empty preserves existing created_at");
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![("p-run".into(), "".into(), "2026-11-27T09:00:00Z".into())]
    );

    proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone()])
        .expect("apply 1..=2");
    proj.apply_envelopes("p-run", None, &[])
        .expect("empty on started");
    proj.apply_envelopes("p-run", Some(&sample_manifest("filled")), &[])
        .expect("fill feature on empty");
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![(
            "p-run".into(),
            "filled".into(),
            "2026-11-27T10:00:00Z".into()
        )]
    );

    proj.apply_envelopes(
        "p-run",
        Some(&sample_manifest("filled")),
        std::slice::from_ref(&e1),
    )
    .expect("monotonic shorter prefix");
    srv.db
        .lock()
        .unwrap()
        .proj_events
        .remove(&("p-run".into(), 1));
    proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone()])
        .expect("repair missing row on equal head");
    assert!(srv
        .db
        .lock()
        .unwrap()
        .proj_events
        .contains_key(&("p-run".into(), 1)));

    srv.db
        .lock()
        .unwrap()
        .proj_events
        .remove(&("p-run".into(), 2));
    proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone(), e3.clone()])
        .expect("advance to 3");
    assert_eq!(srv.db.lock().unwrap().proj_runs["p-run"].3, 3);

    verify_projection_divergence_and_rebuild(&srv, &proj, &e1, &e2, &e3);
}

fn verify_projection_divergence_and_rebuild(
    srv: &MockPgServer,
    proj: &PgProjectionStore,
    e1: &EventEnvelope,
    e2: &EventEnvelope,
    e3: &EventEnvelope,
) {
    {
        let mut db = srv.db.lock().unwrap();
        db.proj_events.insert(
            ("p-run".into(), 99),
            (
                "id".into(),
                "run/annotated".into(),
                None,
                None,
                "h".into(),
                "t".into(),
                "{}".into(),
            ),
        );
    }
    assert!(matches!(
        proj.apply_envelopes("p-run", None, std::slice::from_ref(e1)),
        Err(StoreError::DivergentEventLog { .. })
    ));
    srv.db
        .lock()
        .unwrap()
        .proj_events
        .remove(&("p-run".into(), 99));

    srv.db
        .lock()
        .unwrap()
        .proj_events
        .remove(&("p-run".into(), 1));
    assert!(matches!(
        proj.apply_envelopes("p-run", None, std::slice::from_ref(e1)),
        Err(StoreError::DivergentEventLog { .. })
    ));
    proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone(), e3.clone()])
        .unwrap();

    let alt_e3 = sealed_env(
        "p-run",
        3,
        &e2.event_hash,
        EventType::PhaseEntered,
        json!({ "phase": "alt" }),
        "2026-11-27T10:00:02Z",
    );
    assert!(matches!(
        proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone(), alt_e3]),
        Err(StoreError::DivergentEventLog { .. })
    ));
    let alt_e4 = sealed_env(
        "p-run",
        4,
        &e3.event_hash,
        EventType::PhaseEntered,
        json!({ "phase": "ship" }),
        "2026-11-27T10:00:03Z",
    );
    srv.db.lock().unwrap().proj_runs.get_mut("p-run").unwrap().4 = "wrong-hash".into();
    assert!(matches!(
        proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone(), e3.clone(), alt_e4]),
        Err(StoreError::DivergentEventLog { .. })
    ));
    assert!(matches!(
        proj.apply_envelopes("p-run", None, &[]),
        Err(StoreError::DivergentEventLog { .. })
    ));

    verify_corrupt_projection_and_rebuild(srv, proj, e1, e2, e3);
}

fn verify_corrupt_projection_and_rebuild(
    srv: &MockPgServer,
    proj: &PgProjectionStore,
    e1: &EventEnvelope,
    e2: &EventEnvelope,
    e3: &EventEnvelope,
) {
    {
        let mut db = srv.db.lock().unwrap();
        db.proj_runs.insert(
            "z-bad".into(),
            ("".into(), "".into(), "".into(), 0, "not-zero".into()),
        );
        db.proj_runs.insert(
            "neg-bad".into(),
            ("".into(), "".into(), "".into(), -1, ZERO_HASH.into()),
        );
    }
    assert!(matches!(
        proj.apply_envelopes("z-bad", None, &[]),
        Err(StoreError::DivergentEventLog { .. })
    ));
    assert!(matches!(
        proj.apply_envelopes("neg-bad", None, &[]),
        Err(StoreError::DivergentEventLog { .. })
    ));
    srv.db.lock().unwrap().proj_runs.get_mut("p-run").unwrap().4 = e3.event_hash.clone();
    // Gap detection on empty batch: deleting seq 2 while head_seq == 3 must refuse!
    let saved_e2 = srv
        .db
        .lock()
        .unwrap()
        .proj_events
        .remove(&("p-run".into(), 2))
        .expect("seq 2 present");
    assert!(matches!(
        proj.apply_envelopes("p-run", None, &[]),
        Err(StoreError::DivergentEventLog { .. })
    ));
    srv.db
        .lock()
        .unwrap()
        .proj_events
        .insert(("p-run".into(), 2), saved_e2);

    for bad_payload in ["not-json", r#"{"different":true}"#] {
        srv.db
            .lock()
            .unwrap()
            .proj_events
            .get_mut(&("p-run".into(), 1))
            .unwrap()
            .6 = bad_payload.into();
        assert!(matches!(
            proj.apply_envelopes("p-run", None, &[e1.clone(), e2.clone(), e3.clone()]),
            Err(StoreError::DivergentEventLog { .. })
        ));
    }

    verify_unstarted_metadata_and_rebuild(srv, proj, e1, e2, e3);
}

fn verify_unstarted_metadata_and_rebuild(
    srv: &MockPgServer,
    proj: &PgProjectionStore,
    e1: &EventEnvelope,
    e2: &EventEnvelope,
    e3: &EventEnvelope,
) {
    let cfg = srv.config();
    let log = PgEventLog::connect(&cfg).expect("connect log");
    log.create_run("p-run", &sample_manifest("rebuilt"))
        .unwrap();
    log.append_envelopes(
        "p-run",
        (0, ZERO_HASH),
        &[e1.clone(), e2.clone(), e3.clone()],
    )
    .unwrap();
    assert_eq!(proj.rebuild_from(&log).unwrap(), 3);
    // Unstarted run preserves feature and created_at via run_metadata across rebuild_from.
    let unstarted_manifest = json!({
        "engine": "0.15.2",
        "feature": "queued-feat",
        "bundle_name": "fast",
        "created_at": "2026-11-27T08:30:00Z"
    });
    log.create_run("unstarted-run", &unstarted_manifest)
        .unwrap();
    assert_eq!(
        log.run_metadata("unstarted-run").unwrap(),
        Some(RunMetadata {
            feature: "queued-feat".into(),
            created_at: "2026-11-27T08:30:00Z".into(),
            list_order: None,
        })
    );
    let unified = PgStore::connect(&cfg).expect("connect unified");
    assert_eq!(
        unified.run_metadata("unstarted-run").unwrap(),
        log.run_metadata("unstarted-run").unwrap()
    );
    assert_eq!(proj.rebuild_from(&log).unwrap(), 3);
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![
            (
                "unstarted-run".into(),
                "queued-feat".into(),
                "2026-11-27T08:30:00Z".into()
            ),
            (
                "p-run".into(),
                "rebuilt".into(),
                "2026-11-27T10:00:00Z".into()
            ),
        ]
    );
    let queries = srv.db.lock().unwrap().queries.clone();
    assert!(queries
        .iter()
        .any(|q| q.contains("ON CONFLICT (run_id) DO NOTHING")));
    assert!(queries.iter().any(
        |q| q == "LOCK TABLE brokkr_projected_runs, brokkr_projected_events IN EXCLUSIVE MODE"
    ));
}

#[test]
fn canonical_json_preserves_negative_zero_exponent_and_nul_alongside_jsonb() {
    let srv = MockPgServer::start();
    let cfg = srv.config();
    let journal = Journal::new(
        PgEventLog::connect(&cfg).unwrap(),
        PgProjectionStore::connect(&cfg).unwrap(),
        PgCoordinationStore::connect(&cfg).unwrap(),
    );
    let manifest = json!({
        "engine": "0.15.2",
        "feature": "canon-json",
        "bundle_name": "fast",
        "neg_zero": -0.0_f64,
        "big_exp": 1e16_f64,
        "flag": true,
        "empty": null,
        "items": ["ok"]
    });
    journal.create_run("canon-run", &manifest).unwrap();
    assert!(srv.db.lock().unwrap().runs_jsonb["canon-run"].is_some());

    let e1 = journal
        .append_event(
            "canon-run",
            NewEvent {
                event_type: EventType::RunStarted,
                payload: json!({
                    "feature": "canon-json",
                    "manifest": manifest,
                    "neg_zero": -0.0_f64,
                    "big_exp": 1e16_f64,
                }),
                causation_id: None,
                attempt_id: None,
                ts: "2026-11-27T10:00:00Z",
            },
        )
        .unwrap();
    let e2 = journal
        .append_event(
            "canon-run",
            NewEvent {
                event_type: EventType::PhaseEntered,
                payload: json!({
                    "phase": "build",
                    "nul_str": "a\u{0000}b",
                    "nul_arr": ["x\u{0000}y"],
                    "k\u{0000}ey": 1
                }),
                causation_id: Some(&e1.event_id),
                attempt_id: None,
                ts: "2026-11-27T10:00:01Z",
            },
        )
        .unwrap();
    {
        let db = srv.db.lock().unwrap();
        assert!(db.events_jsonb[&("canon-run".into(), 1)].is_some());
        assert!(db.events_jsonb[&("canon-run".into(), 2)].is_none());
        assert!(db.proj_events_jsonb[&("canon-run".into(), 1)].is_some());
        assert!(db.proj_events_jsonb[&("canon-run".into(), 2)].is_none());
    }

    let loaded = journal.load_verified("canon-run").unwrap();
    assert_eq!(
        serde_json::to_string(&loaded).unwrap(),
        serde_json::to_string(&[&e1, &e2]).unwrap()
    );
    assert_eq!(journal.reindex().unwrap(), 2);

    let ndjson = journal.export_ndjson("canon-run").unwrap();
    assert!(ndjson.contains("-0.0"));
    assert!(ndjson.contains("\\u0000"));
    let srv2 = MockPgServer::start();
    let store2 = PgStore::connect(&srv2.config()).unwrap();
    let j2 = Journal::new(&store2, &store2, &store2);
    assert_eq!(
        j2.import_ndjson("canon-run", &manifest, &ndjson).unwrap(),
        2
    );
    assert_eq!(
        serde_json::to_string(&j2.load_verified("canon-run").unwrap()).unwrap(),
        serde_json::to_string(&[&e1, &e2]).unwrap()
    );
}
