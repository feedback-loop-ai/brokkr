//! Round 7 regression tests for decision 0126 Slice 1 (#1362):
//! SQL `(seq, event_hash)` column verification in `SqlitePorts::read_stream`,
//! decoupled `projection_hashes` checks after `head_seq == 0` reset and
//! same-head hash restoration, persisted-manifest `SeatRecordVersion`
//! resolution in `Journal`, `attempt_id` sealing/round-trip on `Journal`,
//! co-located `runs.feature` sync from `RunStarted`, and `(run_id, head_seq)`
//! guarded stale-run cleanup in decoupled `rebuild_from`.

use std::path::Path;

use rusqlite::{params, Connection};

use super::regressions::sealed_event;
use super::round3::{started_event, summary_snapshot};
use super::round4::completed_vs_phase_pair;
use super::*;
use crate::SeatRecordVersion;

fn v8_only_succeeded_payload() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "result": {
            "step": "seat-turn",
            "model": "m",
            "root_session": {
                "kind": "custom-harness",
                "id": "019c4b7e",
                "harness_version": "1.2.3",
                "persistent": true
            }
        }
    })
}

pub(super) fn v8_succeeded_event() -> NewEvent<'static> {
    NewEvent {
        event_type: EventType::EffectSucceeded,
        payload: v8_only_succeeded_payload(),
        causation_id: None,
        attempt_id: Some("att-1"),
        ts: "2026-11-25T10:00:05Z",
    }
}

pub(super) fn projection_hash_rows(path: &Path, run_id: &str) -> Vec<(i64, String)> {
    query_run_pairs(
        path,
        "SELECT seq, event_hash FROM projection_hashes WHERE run_id = ?1 ORDER BY seq",
        run_id,
    )
}

pub(super) fn sqlite_journal(
    store: &Store,
) -> Journal<SqlitePorts<'_>, SqlitePorts<'_>, SqlitePorts<'_>> {
    Journal::new(
        SqlitePorts::new(store),
        SqlitePorts::new(store),
        SqlitePorts::new(store),
    )
}

pub(super) fn exec_raw_sql(path: &Path, sql: &str) {
    Connection::open(path).unwrap().execute_batch(sql).unwrap();
}

pub(super) fn assert_v8_refused_under_v1(
    journal: &Journal<SqlitePorts<'_>, SqlitePorts<'_>, SqlitePorts<'_>>,
    run_id: &str,
) {
    let err = journal
        .append_event(run_id, v8_succeeded_event())
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::SeatRecord(ref e) if e.contract == SeatRecordVersion::V1.contract()
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
fn decoupled_apply_refuses_fork_after_trigger_drop_reopen_resets_head_seq() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("trigger-reset.db");
    let store = Store::open(&path).unwrap();
    let (e1, e2_completed, e2_phase) = completed_vs_phase_pair("r-trig", "trigger-reset");
    SqlitePorts::new(&store)
        .apply_envelopes("r-trig", &[e1.clone(), e2_phase.clone()])
        .unwrap();
    drop(store);

    exec_raw_sql(&path, "DROP TRIGGER run_summary_of_an_event;");

    let reopened = Store::open(&path).unwrap();
    assert_eq!(summary_snapshot(&path, "r-trig").0, 0);

    let ports = SqlitePorts::new(&reopened);
    assert!(matches!(
        ports.apply_envelopes("r-trig", &[e1.clone(), e2_completed]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-trig"
    ));

    ports.apply_envelopes("r-trig", &[e1, e2_phase]).unwrap();
    assert_eq!(summary_snapshot(&path, "r-trig").0, 2);
}

#[test]
fn decoupled_same_head_replay_restores_empty_projection_hashes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("restore-hashes.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let (e1, e2_forked, e2_phase) = completed_vs_phase_pair("r-restore", "restore-hashes");
    let e3 = sealed_event(
        "r-restore",
        3,
        EventType::RunCompleted,
        json!({"outcome": "restored"}),
        &e2_phase.event_hash,
        "2026-11-25T10:00:12Z",
    );
    ports
        .apply_envelopes("r-restore", &[e1.clone(), e2_phase.clone(), e3.clone()])
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "DELETE FROM projection_hashes WHERE run_id = 'r-restore'",
        [],
    )
    .unwrap();
    drop(raw);
    assert!(projection_hash_rows(&path, "r-restore").is_empty());

    assert!(matches!(
        ports.apply_envelopes("r-restore", &[e1.clone(), e2_phase.clone()]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-restore"
    ));
    let e3_forked = sealed_event(
        "r-restore",
        3,
        EventType::RunCompleted,
        json!({"outcome": "forked"}),
        &e2_forked.event_hash,
        "2026-11-25T10:00:12Z",
    );
    assert!(matches!(
        ports.apply_envelopes("r-restore", &[e1.clone(), e2_forked.clone(), e3_forked]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-restore"
    ));

    ports
        .apply_envelopes("r-restore", &[e1.clone(), e2_phase.clone(), e3.clone()])
        .unwrap();
    assert_eq!(
        projection_hash_rows(&path, "r-restore"),
        vec![
            (1, e1.event_hash.clone()),
            (2, e2_phase.event_hash.clone()),
            (3, e3.event_hash.clone()),
        ]
    );

    ports
        .apply_envelopes("r-restore", &[e1.clone(), e2_phase])
        .unwrap();
    assert_eq!(summary_snapshot(&path, "r-restore").0, 3);
    assert!(matches!(
        ports.apply_envelopes("r-restore", &[e1, e2_forked]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r-restore"
    ));
}

#[test]
fn journal_resolves_seat_record_version_from_persisted_manifest() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("manifest-version.db");
    let mut store = Store::open(&path).unwrap();

    let v1_manifest = json!({"engine": "0.1.0", "bundle_name": "self"});
    let v8_manifest = json!({"engine": "0.15.0", "bundle_name": "self"});

    // Parity reference: Store::append_next refuses V8-only record under persisted 0.1.0 manifest
    // even when RunStarted claims 0.15.0.
    store
        .create_run("store-v1", "parity", "self", &v1_manifest)
        .unwrap();
    store
        .append_next(
            "store-v1",
            EventType::RunStarted,
            json!({"feature": "parity", "manifest": v8_manifest}),
            None,
            None,
        )
        .unwrap();
    let store_err = store
        .append_next(
            "store-v1",
            EventType::EffectSucceeded,
            v8_only_succeeded_payload(),
            None,
            Some("att-1".to_string()),
        )
        .unwrap_err();
    assert!(matches!(
        store_err,
        StoreError::SeatRecord(ref err) if err.contract == SeatRecordVersion::V1.contract()
    ));

    let journal = sqlite_journal(&store);

    // 1) Spoofed upgrade: persisted 0.1.0, RunStarted claims 0.15.0 -> refused under V1.
    // 2) Reverse mismatch: persisted 0.15.0, RunStarted names 0.1.0 -> stricter V1 wins.
    for (run_id, persisted, started_manifest) in [
        ("port-v1", &v1_manifest, &v8_manifest),
        ("port-v8-down", &v8_manifest, &v1_manifest),
    ] {
        journal.create_run(run_id, persisted).unwrap();
        journal
            .append_event(
                run_id,
                plain_event(
                    EventType::RunStarted,
                    json!({"feature": run_id, "manifest": started_manifest}),
                    None,
                    "2026-11-25T10:00:00Z",
                ),
            )
            .unwrap();
        assert_v8_refused_under_v1(&journal, run_id);
    }

    // 3) Persisted 0.15.0 manifest with RunStarted naming 0.15.0 -> admits V8 and exports cleanly.
    journal.create_run("port-v8-ok", &v8_manifest).unwrap();
    journal
        .append_event(
            "port-v8-ok",
            plain_event(
                EventType::RunStarted,
                json!({"feature": "v8-ok", "manifest": v8_manifest}),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    let ok = journal
        .append_event("port-v8-ok", v8_succeeded_event())
        .unwrap();
    assert_eq!(ok.seq, 2);
    assert!(journal.export_ndjson("port-v8-ok").is_ok());
}

#[test]
fn journal_seals_and_preserves_attempt_id_through_export_and_import() {
    let dir = tempdir().unwrap();
    let src_path = dir.path().join("attempt-src.db");
    let dst_path = dir.path().join("attempt-dst.db");
    let src = Store::open(&src_path).unwrap();
    let journal = sqlite_journal(&src);
    let manifest = sample_manifest("attempt-roundtrip");
    journal.create_run("r-att", &manifest).unwrap();

    let append = |et, payload, cause: Option<&str>, attempt: Option<&str>, ts| {
        journal
            .append_event(
                "r-att",
                NewEvent {
                    event_type: et,
                    payload,
                    causation_id: cause,
                    attempt_id: attempt,
                    ts,
                },
            )
            .unwrap()
    };
    let e1 = append(
        EventType::RunStarted,
        started_payload("attempt-roundtrip"),
        None,
        None,
        "2026-11-25T10:00:00Z",
    );
    let e2 = append(
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        Some(&e1.event_id),
        None,
        "2026-11-25T10:00:01Z",
    );
    let e3 = append(
        EventType::EffectRequested,
        json!({"effect_id": "eff-1", "seat": "implement"}),
        Some(&e2.event_id),
        None,
        "2026-11-25T10:00:02Z",
    );
    let e4 = append(
        EventType::EffectStarted,
        json!({"effect_id": "eff-1", "attempt_id": "att-1"}),
        Some(&e3.event_id),
        Some("att-1"),
        "2026-11-25T10:00:03Z",
    );
    let e5 = journal
        .append_if_head(
            "r-att",
            (e4.seq, &e4.event_hash),
            NewEvent {
                event_type: EventType::EffectCheckpointed,
                payload: valid_checkpoint_payload(),
                causation_id: Some(&e4.event_id),
                attempt_id: Some("att-1"),
                ts: "2026-11-25T10:00:04Z",
            },
        )
        .unwrap();
    let e6 = append(
        EventType::EffectSucceeded,
        json!({
            "effect_id": "eff-1",
            "attempt_id": "att-1",
            "result": {"result": "complete", "notes": "done"}
        }),
        Some(&e5.event_id),
        Some("att-1"),
        "2026-11-25T10:00:05Z",
    );
    assert_eq!(e1.attempt_id, None);
    assert_eq!(e4.attempt_id.as_deref(), Some("att-1"));
    assert_eq!(e5.attempt_id.as_deref(), Some("att-1"));
    assert_eq!(e6.attempt_id.as_deref(), Some("att-1"));

    let ndjson = journal.export_ndjson("r-att").unwrap();
    let mut dst = Store::open(&dst_path).unwrap();
    dst.import_run(&ndjson, &manifest, Path::new("r-att.ndjson"))
        .unwrap();
    let imported = dst.load("r-att").unwrap();
    let ids: Vec<Option<&str>> = imported.iter().map(|e| e.attempt_id.as_deref()).collect();
    assert_eq!(
        ids,
        vec![
            None,
            None,
            None,
            Some("att-1"),
            Some("att-1"),
            Some("att-1")
        ]
    );
}

#[test]
fn colocated_sqlite_syncs_runs_feature_from_run_started_payload() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("feature-sync.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);
    let journal = sqlite_journal(&store);

    let manifest = json!({"engine": "0.15.0", "bundle_name": "self"});
    journal.create_run("r-v14", &manifest).unwrap();
    let ts = "2026-11-25T10:00:00Z";
    let started = journal
        .append_event(
            "r-v14",
            plain_event(
                EventType::RunStarted,
                json!({"feature": "real-goal", "manifest": manifest}),
                None,
                ts,
            ),
        )
        .unwrap();
    ports
        .apply_envelopes("r-v14", std::slice::from_ref(&started))
        .unwrap();

    let summary_feature = |db: &Path| -> Option<String> {
        let conn = Connection::open(db).unwrap();
        conn.query_row(
            "SELECT feature FROM run_summary WHERE run_id = 'r-v14'",
            [],
            |row| row.get(0),
        )
        .unwrap()
    };
    let assert_feature = || {
        let expected = vec![("r-v14".to_string(), "real-goal".to_string(), ts.to_string())];
        assert_eq!(ports.list_runs().unwrap(), expected);
        assert_eq!(store.list_runs().unwrap(), expected);
        assert_eq!(summary_feature(&path).as_deref(), Some("real-goal"));
    };
    assert_feature();
    assert_eq!(journal.reindex().unwrap(), 1);
    assert_feature();
}

#[test]
fn decoupled_rebuild_preserves_run_advanced_from_empty_scan() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("concurrent-empty-advance.db");
    let store = Store::open(&path).unwrap();
    let ports = SqlitePorts::new(&store);

    let (e1, _, e2_phase) = completed_vs_phase_pair("r-adv", "adv");
    let stale_e1 = started_event("r-stale", "stale", "2026-11-25T09:00:00Z");
    ports
        .apply_envelopes("r-adv", std::slice::from_ref(&e1))
        .unwrap();
    ports
        .apply_envelopes("r-stale", std::slice::from_ref(&stale_e1))
        .unwrap();

    let inner = MemEventLog::default();

    let store_ref = &store;
    let e1_conc = e1.clone();
    let e2_conc = e2_phase.clone();
    let scanning_log = InterceptingMemLog {
        inner,
        on_head: None,
        on_list_run_ids: Some(Box::new(move || {
            SqlitePorts::new(store_ref)
                .apply_envelopes("r-adv", &[e1_conc.clone(), e2_conc.clone()])
        })),
    };

    assert_eq!(ports.rebuild_from(&scanning_log).unwrap(), 0);
    assert_eq!(summary_snapshot(&path, "r-adv").0, 2);
    assert_eq!(summary_snapshot(&path, "r-adv").1, Some(2));
    assert_eq!(
        ports.list_runs().unwrap(),
        vec![(
            "r-adv".to_string(),
            "adv".to_string(),
            "2026-11-25T10:00:00Z".to_string(),
        )]
    );
}
