//! Offline tests for `PgCoordinationStore` and `PgStore` queue, `FOR UPDATE SKIP LOCKED`,
//! and idempotent provider cooldown catch-up.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::ending::Ending;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::ports::{CoordinationStore, EventLog};
use brokkr_store::{EntryId, EntryState, NewEntry, QueueRefusal, StoreError, Wait, WaitOn};
use serde_json::json;

use crate::mock_pg_tests::{sample_manifest as manifest, sealed_env as env, MockPgServer};
use crate::{PgCoordinationStore, PgError, PgEventLog, PgStore};

fn connect_coord_pair() -> (MockPgServer, PgCoordinationStore, PgStore) {
    let srv = MockPgServer::start();
    let cfg = srv.config();
    let coord = PgCoordinationStore::connect(&cfg).expect("coord");
    let unified = PgStore::connect(&cfg).expect("unified");
    (srv, coord, unified)
}

#[test]
fn coordination_queue_and_skip_locked() {
    let (srv, coord, unified) = connect_coord_pair();

    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "x",
                priority: 0,
                waits: &[]
            },
            "not-a-time"
        ),
        Err(StoreError::Clock { .. })
    ));
    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "x",
                priority: 0,
                waits: &[Wait {
                    entry: EntryId(1),
                    on: WaitOn::Named(Ending::Unclassified)
                }]
            },
            "2026-11-27T12:00:00Z"
        ),
        Err(StoreError::Queue(QueueRefusal::NotACondition(_)))
    ));
    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "x",
                priority: 0,
                waits: &[
                    Wait {
                        entry: EntryId(1),
                        on: WaitOn::Completed
                    },
                    Wait {
                        entry: EntryId(1),
                        on: WaitOn::Ended
                    },
                ]
            },
            "2026-11-27T12:00:00Z"
        ),
        Err(StoreError::Queue(QueueRefusal::DuplicateWait(EntryId(1))))
    ));
    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "x",
                priority: 0,
                waits: &[Wait {
                    entry: EntryId(99),
                    on: WaitOn::Completed
                }]
            },
            "2026-11-27T12:00:00Z"
        ),
        Err(StoreError::Queue(QueueRefusal::UnknownWait(EntryId(99))))
    ));

    srv.db.lock().unwrap().force_empty_returning = true;
    assert!(matches!(
        coord.enqueue(
            &NewEntry {
                payload: "x",
                priority: 0,
                waits: &[]
            },
            "2026-11-27T12:00:00Z"
        ),
        Err(StoreError::Io(_))
    ));

    verify_queue_order_and_skip_locked(&srv, &coord, &unified);
}

fn verify_queue_order_and_skip_locked(
    srv: &MockPgServer,
    coord: &PgCoordinationStore,
    unified: &PgStore,
) {
    let id1 = coord
        .enqueue(
            &NewEntry {
                payload: "low",
                priority: 1,
                waits: &[],
            },
            "2026-11-27T12:00:00Z",
        )
        .unwrap();
    let id2 = unified
        .enqueue(
            &NewEntry {
                payload: "high",
                priority: 10,
                waits: &[Wait {
                    entry: id1,
                    on: WaitOn::Completed,
                }],
            },
            "2026-11-27T12:00:01Z",
        )
        .unwrap();

    {
        let mut db = srv.db.lock().unwrap();
        db.queue_entries.insert(
            90,
            (
                "h".into(),
                0,
                "held".into(),
                None,
                None,
                None,
                "2026-11-27T12:00:02Z".into(),
            ),
        );
        db.queue_entries.insert(
            91,
            (
                "l".into(),
                -1,
                "launching".into(),
                None,
                None,
                None,
                "2026-11-27T12:00:03Z".into(),
            ),
        );
    }
    let listed = unified.list_queue().unwrap();
    assert_eq!(listed[0].id, id2);
    assert_eq!(listed[0].position, Some(1));
    assert_eq!(
        listed[0].waits,
        vec![Wait {
            entry: id1,
            on: WaitOn::Completed
        }]
    );
    assert_eq!(listed[1].id, id1);
    assert_eq!(listed[2].state, EntryState::Held);
    assert_eq!(listed[3].state, EntryState::Launching);

    srv.db.lock().unwrap().locked_queue_ids.insert(id2.0);
    let c1 = coord
        .claim_next_entry("worker-a", "2026-11-27T12:01:00Z")
        .unwrap()
        .expect("claimed id1");
    assert_eq!(c1.id, id1);
    assert_eq!(c1.worker_id, "worker-a");
    srv.db.lock().unwrap().locked_queue_ids.remove(&id2.0);

    let c2 = unified
        .claim_next_entry("worker-b", "2026-11-27T12:01:01Z")
        .unwrap()
        .expect("claimed id2");
    assert_eq!(c2.id, id2);
    assert_eq!(
        c2.waits,
        vec![Wait {
            entry: id1,
            on: WaitOn::Completed
        }]
    );
    assert!(coord
        .claim_next_entry("worker-c", "2026-11-27T12:01:02Z")
        .unwrap()
        .is_none());

    assert!(srv
        .db
        .lock()
        .unwrap()
        .queries
        .iter()
        .any(|q| q.contains("ORDER BY priority DESC, id ASC FOR UPDATE SKIP LOCKED LIMIT 1")));

    verify_claimed_listing_and_bad_words(srv, coord, id1, id2);
}

fn verify_claimed_listing_and_bad_words(
    srv: &MockPgServer,
    coord: &PgCoordinationStore,
    id1: EntryId,
    id2: EntryId,
) {
    // Reject empty/blank worker_id with typed PgError::EmptyWorkerId.
    for blank in ["", "   "] {
        let err = coord
            .claim_next_entry(blank, "2026-11-27T12:01:05Z")
            .unwrap_err();
        let StoreError::Io(io_err) = err else {
            panic!("expected StoreError::Io, got {err:?}");
        };
        assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(matches!(
            io_err.get_ref().and_then(|e| e.downcast_ref::<PgError>()),
            Some(PgError::EmptyWorkerId)
        ));
    }

    // Worker reservation without a started run_id MUST NOT report EntryState::Claimed { run: worker_id }.
    let listed_after = coord.list_queue().unwrap();
    assert!(listed_after
        .iter()
        .any(|e| { e.id == id2 && e.position.is_some() && e.state == EntryState::Launching }));
    assert!(!listed_after
        .iter()
        .any(|e| matches!(&e.state, EntryState::Claimed { run, .. } if run == "worker-b")));

    // A row with status='claimed' and run_id=None or run_id=Some("") still reports Launching
    // AND ranks in group 0 ahead of Claimed rows (even when the Claimed row has higher priority!).
    {
        let mut db = srv.db.lock().unwrap();
        db.queue_entries.get_mut(&id1.0).unwrap().2 = "claimed".into();
        db.queue_entries.get_mut(&id1.0).unwrap().5 = Some(String::new());
        db.queue_entries.get_mut(&id2.0).unwrap().2 = "claimed".into();
        db.queue_entries.get_mut(&id2.0).unwrap().5 = Some("run-real-1".into());
    }
    let listed_runs = coord.list_queue().unwrap();
    let idx_empty_run = listed_runs
        .iter()
        .position(|e| e.id == id1 && e.position.is_some() && e.state == EntryState::Launching)
        .expect("empty run_id decoded as Launching with position");
    let idx_real_claimed = listed_runs
        .iter()
        .position(|e| {
            e.id == id2
                && e.position.is_none()
                && e.state
                    == EntryState::Claimed {
                        run: "run-real-1".into(),
                        merged: false,
                    }
        })
        .expect("real run_id decoded as Claimed");
    assert!(
        idx_empty_run < idx_real_claimed,
        "empty run_id Launching entry must rank before Claimed entries"
    );
    assert!(srv
        .db
        .lock()
        .unwrap()
        .queries
        .iter()
        .any(|q| q.contains("COALESCE(run_id, '') = ''")));

    srv.db.lock().unwrap().queue_entries.get_mut(&90).unwrap().2 = "bogus".into();
    assert!(matches!(coord.list_queue(), Err(StoreError::Io(_))));
    srv.db.lock().unwrap().queue_entries.get_mut(&90).unwrap().2 = "held".into();
    srv.db
        .lock()
        .unwrap()
        .queue_waits
        .insert((id2.0, id1.0), "bogus".into());
    assert!(matches!(coord.list_queue(), Err(StoreError::Io(_))));
    srv.db
        .lock()
        .unwrap()
        .queue_waits
        .insert((id2.0, id1.0), "completed".into());
}

#[test]
fn coordination_idempotent_cooldowns_and_conflict_refusals() {
    let (srv, coord, unified) = connect_coord_pair();
    let started = env(
        "cool-run",
        1,
        ZERO_HASH,
        EventType::RunStarted,
        json!({ "feature": "cool", "manifest": manifest("cool") }),
        "2026-11-27T12:00:00Z",
    );
    let failed = env(
        "cool-run",
        2,
        &started.event_hash,
        EventType::EffectFailed,
        json!({
            "phase": "build",
            "error": "rate_limited",
            "waits": [
                {
                    "provider": "openai",
                    "route": "primary",
                    "until": "2026-11-27T12:30:00Z",
                    "words": "slow down",
                    "retry_after_seconds": 30,
                    "seconds": 30,
                    "rounds": 1
                },
                {
                    "site": "expired-site",
                    "provider": "openai",
                    "route": "primary",
                    "until": "2026-11-27T11:59:00Z",
                    "words": "old",
                    "seconds": 60,
                    "rounds": 1
                }
            ]
        }),
        "2026-11-27T12:00:05Z",
    );
    let log = PgEventLog::connect(&srv.config()).unwrap();
    log.create_run("cool-run", &manifest("cool")).unwrap();
    log.append_envelopes(
        "cool-run",
        (0, ZERO_HASH),
        &[started.clone(), failed.clone()],
    )
    .unwrap();

    assert_eq!(
        coord
            .apply_cooldowns("cool-run", &[started.clone(), failed.clone()])
            .unwrap(),
        1
    );
    assert_eq!(
        unified
            .apply_cooldowns("cool-run", std::slice::from_ref(&failed))
            .unwrap(),
        0
    );
    assert_eq!(coord.sync_cooldowns_from(&log).unwrap(), 0);
    assert_eq!(unified.sync_cooldowns_from(&log).unwrap(), 0);

    let active = unified.active_cooldowns("2026-11-27T12:10:00Z").unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].provider, "openai");
    assert_eq!(active[0].retry_after_seconds, Some(30));
    assert!(coord
        .active_cooldowns("2026-11-27T12:40:00Z")
        .unwrap()
        .is_empty());
    // Refuses non-positive stored retry_after_seconds or rounds via checked u64::try_from.
    {
        let mut db = srv.db.lock().unwrap();
        db.cooldowns.get_mut(&1).unwrap().7 = Some(-5);
    }
    assert!(matches!(
        coord.active_cooldowns("2026-11-27T12:10:00Z"),
        Err(StoreError::Io(_))
    ));
    {
        let mut db = srv.db.lock().unwrap();
        db.cooldowns.get_mut(&1).unwrap().7 = Some(30);
        db.cooldowns.get_mut(&1).unwrap().8 = 0;
    }
    assert!(matches!(
        coord.active_cooldowns("2026-11-27T12:10:00Z"),
        Err(StoreError::Io(_))
    ));
    srv.db.lock().unwrap().cooldowns.get_mut(&1).unwrap().8 = 1;

    verify_cooldown_conflicts(&srv, &coord, &started, &failed);
}

fn verify_cooldown_conflicts(
    srv: &MockPgServer,
    coord: &PgCoordinationStore,
    started: &EventEnvelope,
    failed: &EventEnvelope,
) {
    let divergent_failed = env(
        "cool-run",
        2,
        &started.event_hash,
        EventType::EffectFailed,
        json!({
            "phase": "build",
            "error": "rate_limited",
            "waits": [{
                "provider": "openai",
                "route": "different-route",
                "until": "2026-11-27T12:30:00Z",
                "words": "slow down",
                "seconds": 60,
                    "rounds": 1
            }]
        }),
        "2026-11-27T12:00:05Z",
    );
    assert!(matches!(
        coord.apply_cooldowns("cool-run", &[divergent_failed]),
        Err(StoreError::DivergentEventLog { .. })
    ));

    let dup_batch = env(
        "cool-run",
        3,
        &failed.event_hash,
        EventType::EffectIndeterminate,
        json!({
            "waits": [
                { "provider": "p", "route": "r", "until": "2026-11-27T12:30:00Z", "words": "a", "seconds": 60,
                    "rounds": 1 },
                { "provider": "p", "route": "r", "until": "2026-11-27T12:30:00Z", "words": "a", "seconds": 60,
                    "rounds": 1 }
            ]
        }),
        "2026-11-27T12:00:10Z",
    );
    assert!(matches!(
        coord.apply_cooldowns("cool-run", &[dup_batch]),
        Err(StoreError::DivergentEventLog { .. })
    ));

    let huge_rounds = env(
        "cool-run",
        4,
        &failed.event_hash,
        EventType::EffectFailed,
        json!({
            "waits": [{
                "provider": "p",
                "route": "r",
                "until": "2026-11-27T12:30:00Z",
                "words": "a",
                "seconds": 60,
                "rounds": u64::MAX
            }]
        }),
        "2026-11-27T12:00:10Z",
    );
    assert!(matches!(
        coord.apply_cooldowns("cool-run", &[huge_rounds]),
        Err(StoreError::CooldownFieldOutOfRange {
            field: "rounds",
            ..
        })
    ));
    let empty_waits = env(
        "cool-run",
        5,
        &failed.event_hash,
        EventType::EffectFailed,
        json!({ "waits": [] }),
        "2026-11-27T12:00:10Z",
    );
    assert!(matches!(
        coord.apply_cooldowns("cool-run", &[empty_waits]),
        Err(StoreError::ProviderWait(_))
    ));

    verify_cooldown_edge_refusals(srv, coord, failed);
}

fn verify_cooldown_edge_refusals(
    srv: &MockPgServer,
    coord: &PgCoordinationStore,
    failed: &EventEnvelope,
) {
    let fresh_fail = env(
        "cool-run",
        6,
        &failed.event_hash,
        EventType::EffectFailed,
        json!({
            "waits": [{
                "provider": "p",
                "route": "r",
                "until": "2026-11-27T12:30:00Z",
                "words": "a",
                "seconds": 60,
                    "rounds": 1
            }]
        }),
        "2026-11-27T12:00:10Z",
    );
    srv.db.lock().unwrap().force_zero_cooldown_insert = true;
    assert!(matches!(
        coord.apply_cooldowns("cool-run", &[fresh_fail]),
        Err(StoreError::DivergentEventLog { .. })
    ));

    let bad_wait = env(
        "cool-run",
        7,
        &failed.event_hash,
        EventType::EffectFailed,
        json!({ "waits": [{"provider": "", "route": "r", "until": "bad", "words": "x", "seconds": 10, "rounds": 1}] }),
        "2026-11-27T12:00:10Z",
    );
    assert!(matches!(
        coord.apply_cooldowns("cool-run", &[bad_wait]),
        Err(StoreError::ProviderWait(_))
    ));
}
