//! Live PostgreSQL integration suite gated on `BROKKR_TEST_PG_URL`.
//!
//! When `BROKKR_TEST_PG_URL` is unset, every test returns cleanly so offline
//! `cargo test --workspace` runs without an external PostgreSQL daemon.

use std::sync::{Arc, Barrier};
use std::thread;

use brokkr_core::envelope::EventType;
use brokkr_store::ports::{CoordinationStore, EventLog, Journal, NewEvent, ProjectionStore};
use brokkr_store::{EntryState, NewEntry, StoreError};
use brokkr_store_pg::{
    PgClient, PgConfig, PgCoordinationStore, PgEventLog, PgProjectionStore, PgStore,
};
use serde_json::{json, Value};

fn live_config() -> Option<PgConfig> {
    let url = std::env::var("BROKKR_TEST_PG_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    Some(PgConfig::parse(&url).expect("valid BROKKR_TEST_PG_URL"))
}

fn reset_tables(cfg: &PgConfig) {
    let mut client = PgClient::connect(cfg).expect("connect to live postgres");
    for stmt in [
        "DROP TABLE IF EXISTS brokkr_queue_waits",
        "DROP TABLE IF EXISTS brokkr_queue_entries",
        "DROP TABLE IF EXISTS brokkr_provider_cooldowns",
        "DROP TABLE IF EXISTS brokkr_projected_events",
        "DROP TABLE IF EXISTS brokkr_projected_runs",
        "DROP TABLE IF EXISTS brokkr_events",
        "DROP TABLE IF EXISTS brokkr_runs",
    ] {
        client.query(stmt, &[]).expect("drop test table");
    }
}

#[test]
fn live_pg_journal_round_trip_concurrency_rebuild_and_skip_locked() {
    let Some(cfg) = live_config() else {
        return;
    };
    reset_tables(&cfg);

    let event_log = PgEventLog::connect(&cfg).expect("init event log");
    let projections = PgProjectionStore::connect(&cfg).expect("init projections");
    let coordination = PgCoordinationStore::connect(&cfg).expect("init coordination");
    let journal = Journal::new(event_log, projections, coordination);

    // 1. Journal round-trip: create_run -> append_event -> HeadMoved -> canonical JSONB
    let manifest = json!({
        "engine": "0.15.2",
        "feature": "pg-slice-3",
        "bundle_name": "fast",
        "neg_zero": -0.0_f64,
        "big_exp": 1e16_f64
    });
    journal
        .create_run("pg-live-run-1", &manifest)
        .expect("create run");
    assert!(matches!(
        journal.create_run("pg-live-run-1", &manifest),
        Err(StoreError::RunExists(_))
    ));

    let started = journal
        .append_event(
            "pg-live-run-1",
            NewEvent {
                event_type: EventType::RunStarted,
                payload: json!({
                    "feature": "pg-slice-3",
                    "manifest": manifest,
                    "neg_zero": -0.0_f64,
                    "big_exp": 1e16_f64
                }),
                causation_id: None,
                attempt_id: None,
                ts: "2026-11-27T12:00:00Z",
            },
        )
        .expect("append RunStarted");
    assert_eq!(started.seq, 1);

    let failed = journal
        .append_event(
            "pg-live-run-1",
            NewEvent {
                event_type: EventType::EffectFailed,
                payload: json!({
                    "phase": "build",
                    "error": "rate_limited",
                    "nul_note": "ok\u{0000}nul",
                    "waits": [{
                        "provider": "anthropic",
                        "route": "default",
                        "until": "2026-11-27T12:30:00Z",
                        "words": "429 too many requests",
                        "retry_after_seconds": 60,
                        "seconds": 60,
                        "rounds": 1
                    }]
                }),
                causation_id: Some(&started.event_id),
                attempt_id: Some("att-1"),
                ts: "2026-11-27T12:01:00Z",
            },
        )
        .expect("append EffectFailed");
    assert_eq!(failed.seq, 2);

    // 2. Genuine concurrent two-connection HeadMoved contention on seq 3.
    verify_concurrent_head_moved(&cfg, "pg-live-run-1", failed.seq, &failed.event_hash);

    // 3. Rebuild projections, cooldown catch-up, and canonical export/import round-trip.
    verify_live_rebuild_and_cooldowns(&journal);
    verify_live_export_import_canonical(&cfg, &journal, &manifest);
    verify_live_skip_locked_and_unified(&cfg, journal.coordination());
}

fn verify_concurrent_head_moved(cfg: &PgConfig, run_id: &str, head_seq: u64, head_hash: &str) {
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for worker in ["w1", "w2"] {
        let c = cfg.clone();
        let b = Arc::clone(&barrier);
        let rid = run_id.to_string();
        let prev = head_hash.to_string();
        handles.push(thread::spawn(move || {
            let log = PgEventLog::connect(&c).expect("connect racing log");
            let env = NewEvent {
                event_type: EventType::PhaseEntered,
                payload: json!({ "phase": worker }),
                causation_id: None,
                attempt_id: None,
                ts: "2026-11-27T12:02:00Z",
            }
            .seal(&rid, head_seq + 1, format!("race-{worker}"), &prev);
            b.wait();
            log.append_envelopes(&rid, (head_seq, &prev), &[env])
        }));
    }
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let ok_count = results.iter().filter(|r| r.is_ok()).count();
    let moved_count = results
        .iter()
        .filter(|r| matches!(r, Err(StoreError::HeadMoved { .. })))
        .count();
    assert_eq!(ok_count, 1, "expected 1 winner, got {results:?}");
    assert_eq!(moved_count, 1, "expected 1 HeadMoved, got {results:?}");
}

fn verify_live_rebuild_and_cooldowns(
    journal: &Journal<PgEventLog, PgProjectionStore, PgCoordinationStore>,
) {
    let loaded = journal
        .load_verified("pg-live-run-1")
        .expect("load verified");
    assert_eq!(loaded.len(), 3);

    let rebuilt = journal.reindex().expect("rebuild projections");
    assert_eq!(rebuilt, 3);
    let listed = journal.projections().list_runs().expect("list runs");
    assert_eq!(
        listed,
        vec![(
            "pg-live-run-1".to_string(),
            "pg-slice-3".to_string(),
            "2026-11-27T12:00:00Z".to_string()
        )]
    );

    let inserted = journal
        .coordination()
        .sync_cooldowns_from(journal.event_log())
        .expect("sync cooldowns");
    assert_eq!(inserted, 1);
    let replay = journal
        .coordination()
        .sync_cooldowns_from(journal.event_log())
        .expect("idempotent replay");
    assert_eq!(replay, 0);
    let active = journal
        .coordination()
        .active_cooldowns("2026-11-27T12:15:00Z")
        .expect("active cooldowns");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].provider, "anthropic");
}

fn verify_live_export_import_canonical(
    cfg: &PgConfig,
    journal: &Journal<PgEventLog, PgProjectionStore, PgCoordinationStore>,
    manifest: &Value,
) {
    let ndjson = journal
        .export_ndjson("pg-live-run-1")
        .expect("export canonical ndjson");
    assert!(ndjson.contains("-0.0"));
    assert!(ndjson.contains("\\u0000"));
    let expected = journal.load_verified("pg-live-run-1").expect("load");

    reset_tables(cfg);
    let store = PgStore::connect(cfg).expect("reconnect unified store");
    let imported_journal = Journal::new(&store, &store, &store);
    let count = imported_journal
        .import_ndjson("pg-live-run-1", manifest, &ndjson)
        .expect("import canonical ndjson");
    assert_eq!(count, 3);
    assert_eq!(
        serde_json::to_string(
            &imported_journal
                .load_verified("pg-live-run-1")
                .expect("verify imported")
        )
        .unwrap(),
        serde_json::to_string(&expected).unwrap()
    );
    assert_eq!(imported_journal.reindex().expect("reindex imported"), 3);
}

fn verify_live_skip_locked_and_unified(cfg: &PgConfig, _coord: &PgCoordinationStore) {
    let coord = PgCoordinationStore::connect(cfg).expect("reconnect coord");
    let e1 = coord
        .enqueue(
            &NewEntry {
                payload: r#"{"job":"high"}"#,
                priority: 10,
                waits: &[],
            },
            "2026-11-27T12:05:00Z",
        )
        .expect("enqueue high");
    let e2 = coord
        .enqueue(
            &NewEntry {
                payload: r#"{"job":"low"}"#,
                priority: 1,
                waits: &[],
            },
            "2026-11-27T12:05:01Z",
        )
        .expect("enqueue low");

    let mut locker = PgClient::connect(cfg).expect("connect locker");
    locker.query("BEGIN", &[]).expect("begin locker tx");
    let locked = locker
        .query(
            "SELECT id FROM brokkr_queue_entries WHERE status = 'pending' ORDER BY priority DESC, id ASC FOR UPDATE SKIP LOCKED LIMIT 1",
            &[],
        )
        .expect("lock highest priority");
    assert_eq!(locked.rows[0].get_i64(0).unwrap(), e1.0);

    let worker2 = PgCoordinationStore::connect(cfg).expect("worker 2");
    let claimed_by_w2 = worker2
        .claim_next_entry("worker-2", "2026-11-27T12:06:00Z")
        .expect("worker 2 skip locked claim")
        .expect("worker 2 got e2");
    assert_eq!(claimed_by_w2.id, e2);
    assert_eq!(claimed_by_w2.worker_id, "worker-2");

    let listed = worker2.list_queue().expect("list queue");
    assert!(listed
        .iter()
        .any(|e| e.id == e2 && e.state == EntryState::Launching));

    locker.query("ROLLBACK", &[]).expect("release lock on e1");

    let unified = PgStore::connect(cfg).expect("connect unified PgStore");
    let claimed_by_w1 = unified
        .claim_next_entry("worker-1", "2026-11-27T12:06:05Z")
        .expect("worker 1 claim")
        .expect("worker 1 got e1");
    assert_eq!(claimed_by_w1.id, e1);
    assert!(unified
        .claim_next_entry("worker-3", "2026-11-27T12:06:10Z")
        .expect("empty claim")
        .is_none());

    assert_eq!(
        unified.list_run_ids().expect("list ids"),
        vec!["pg-live-run-1".to_string()]
    );
}
