//! Live PostgreSQL integration suite gated on `BROKKR_TEST_PG_URL`.
//!
//! When `BROKKR_TEST_PG_URL` is unset, every test returns cleanly so offline
//! `cargo test --workspace` runs without an external PostgreSQL daemon.

use brokkr_core::envelope::EventType;
use brokkr_store::ports::{CoordinationStore, EventLog, Journal, NewEvent, ProjectionStore};
use brokkr_store::{NewEntry, StoreError};
use brokkr_store_pg::{
    PgClient, PgConfig, PgCoordinationStore, PgEventLog, PgProjectionStore, PgStore,
};
use serde_json::json;

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

    // 1. Journal round-trip: create_run -> append_event -> HeadMoved -> export/import
    let manifest = json!({
        "engine": "0.15.2",
        "feature": "pg-slice-3",
        "bundle_name": "fast"
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
                payload: json!({ "feature": "pg-slice-3", "manifest": manifest }),
                causation_id: None,
                attempt_id: None,
                ts: "2026-11-27T12:00:00Z",
            },
        )
        .expect("append RunStarted");
    assert_eq!(started.seq, 1);

    // Optimistic concurrency check: stale head (0, ZERO_HASH) fails with HeadMoved.
    let stale = journal.append_if_head(
        "pg-live-run-1",
        (0, brokkr_core::canonical::ZERO_HASH),
        NewEvent {
            event_type: EventType::PhaseEntered,
            payload: json!({ "phase": "build" }),
            causation_id: Some(&started.event_id),
            attempt_id: None,
            ts: "2026-11-27T12:00:01Z",
        },
    );
    assert!(matches!(
        stale,
        Err(StoreError::HeadMoved {
            expected_seq: 0,
            found_seq: 1
        })
    ));

    let failed = journal
        .append_event(
            "pg-live-run-1",
            NewEvent {
                event_type: EventType::EffectFailed,
                payload: json!({
                    "phase": "build",
                    "error": "rate_limited",
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

    verify_live_rebuild_and_cooldowns(&journal);
    verify_live_skip_locked_and_unified(&cfg, journal.coordination());
}

fn verify_live_rebuild_and_cooldowns(
    journal: &Journal<PgEventLog, PgProjectionStore, PgCoordinationStore>,
) {
    // 2. Rebuild projections from EventLog.
    let rebuilt = journal.reindex().expect("rebuild projections");
    assert_eq!(rebuilt, 2);
    let listed = journal.projections().list_runs().expect("list runs");
    assert_eq!(
        listed,
        vec![(
            "pg-live-run-1".to_string(),
            "pg-slice-3".to_string(),
            "2026-11-27T12:00:00Z".to_string()
        )]
    );

    // 3. Idempotent cooldown catch-up from EventLog.
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

fn verify_live_skip_locked_and_unified(cfg: &PgConfig, coord: &PgCoordinationStore) {
    // 4. FOR UPDATE SKIP LOCKED multi-worker claim contention.
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

    // Hold a row lock on e1 in an open transaction, then claim from another connection:
    // SKIP LOCKED must skip e1 and immediately claim e2 without blocking!
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

    // Verify unified PgStore reads the same event log.
    assert_eq!(
        unified.list_run_ids().expect("list ids"),
        vec!["pg-live-run-1".to_string()]
    );
}
