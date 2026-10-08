//! A chained append lands its rows in order, each caused by the one
//! before it, and stops at the row the seat-record fence refuses (#464).

use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::{json, Value};

use crate::tests::{contended_journal, write_lock_on};
use crate::{Rows, StoreError};

fn checkpoint(record: Value) -> Value {
    json!({"effect_id": "fx", "checkpoint": record})
}

/// `rows`, each its own payload.
fn payloads(rows: &[Value]) -> Rows<'_, Value> {
    Rows {
        rows,
        payload: &Value::clone,
    }
}

#[test]
fn a_chain_lands_in_order_each_row_caused_by_the_one_before() {
    let (_dir, _db, mut store) = contended_journal();
    let rows = [json!({"step": "a"}), json!({"step": "b"})].map(checkpoint);
    let chain = store
        .append_chain(
            "r1",
            EventType::EffectCheckpointed,
            payloads(&rows),
            Some("cause".into()),
            Some("attempt".into()),
        )
        .unwrap();

    assert_eq!(chain.refused, None);
    let journal = store.load("r1").unwrap();
    let ids = |events: &[EventEnvelope]| -> Vec<String> {
        events.iter().map(|event| event.event_id.clone()).collect()
    };
    assert_eq!(ids(&chain.appended), ids(&journal[1..]));
    let shape: Vec<_> = journal[1..]
        .iter()
        .map(|event| {
            let step = event.payload["checkpoint"]["step"].clone();
            (
                event.seq,
                step,
                event.causation_id.clone(),
                event.attempt_id.clone(),
            )
        })
        .collect();
    let attempt = Some("attempt".to_string());
    assert_eq!(
        shape,
        [
            (2, json!("a"), Some("cause".into()), attempt.clone()),
            (3, json!("b"), Some(journal[1].event_id.clone()), attempt),
        ]
    );
}

/// The chain is one transaction: a later row that cannot be written
/// rolls back the rows before it, so none of them lands.
#[test]
fn a_row_that_cannot_be_written_rolls_back_the_whole_chain() {
    let (_dir, db, mut store) = contended_journal();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER poison BEFORE INSERT ON events
             WHEN json_extract(NEW.envelope, '$.payload.checkpoint.step') = 'poison'
             BEGIN SELECT RAISE(ABORT, 'poisoned'); END;",
        )
        .unwrap();
    let rows = [json!({"step": "a"}), json!({"step": "poison"})].map(checkpoint);
    let failed = store
        .append_chain(
            "r1",
            EventType::EffectCheckpointed,
            payloads(&rows),
            None,
            None,
        )
        .unwrap_err();

    assert!(matches!(failed, crate::StoreError::Sqlite(_)), "{failed:?}");
    assert_eq!(failed.to_string(), "sqlite: poisoned");
    assert_eq!(store.head_hash("r1").unwrap().0, 1);
}

/// The fence judges each row at the seq it would take: the rows before a
/// refused one land, and nothing after it is written.
#[test]
fn a_refused_row_stops_the_chain_and_the_rows_before_it_land() {
    let (_dir, _db, mut store) = contended_journal();
    let prose = json!({"step": "seat-turn", "turn": 1, "content": "private prose"});
    let rows = [json!({"step": "a"}), prose, json!({"step": "c"})].map(checkpoint);
    let chain = store
        .append_chain(
            "r1",
            EventType::EffectCheckpointed,
            payloads(&rows),
            None,
            None,
        )
        .unwrap();

    assert_eq!(chain.refused.map(|refusal| refusal.seq), Some(3));
    let seqs: Vec<_> = chain.appended.iter().map(|event| event.seq).collect();
    assert_eq!(seqs, [2]);
    assert_eq!(store.head_hash("r1").unwrap().0, 2);
}

/// A row offered again later need not wait for a peer's lock: it is
/// contended at once, and the store's patience is whole again after.
#[test]
fn an_append_without_waiting_meets_a_held_lock_at_once_and_keeps_the_patience() {
    let (_dir, db, mut store) = contended_journal();
    let patience = std::time::Duration::from_millis(400);
    store.set_patience(patience).unwrap();
    let holder = write_lock_on(&db);
    let (phase, entered) = (EventType::PhaseEntered, json!({"phase": "implement"}));
    let started = std::time::Instant::now();
    let rows = [entered.clone()];
    let refused = store.append_chain_without_waiting("r1", phase, payloads(&rows), None, None);
    assert!(started.elapsed() < patience, "{:?}", started.elapsed());
    let refused = refused.unwrap_err();
    assert!(matches!(refused, StoreError::Contended { operation, .. } if operation == "append"));
    let started = std::time::Instant::now();
    let refused = store.append_next("r1", phase, entered, None, None);
    assert!(started.elapsed() >= patience, "{:?}", started.elapsed());
    assert!(refused.unwrap_err().is_contention());
    holder.execute_batch("ROLLBACK").unwrap();
    let landed = store.append_chain_without_waiting("r1", phase, payloads(&rows), None, None);
    assert_eq!(landed.unwrap().appended[0].seq, 2);
}

/// A row becomes its payload only inside the transaction that holds the
/// lock: an attempt a peer's lock refuses builds none, and one that lands
/// builds each row's once.
#[test]
fn a_chain_the_lock_refuses_builds_no_payload() {
    let (_dir, db, mut store) = contended_journal();
    let built = std::cell::Cell::new(0);
    let payload = |step: &&str| {
        built.set(built.get() + 1);
        checkpoint(json!({"step": step}))
    };
    let rows = || Rows {
        rows: &["a", "b"],
        payload: &payload,
    };
    let holder = write_lock_on(&db);
    let refused = store
        .append_chain_without_waiting("r1", EventType::EffectCheckpointed, rows(), None, None)
        .unwrap_err();
    assert!(
        matches!(refused, StoreError::Contended { operation, .. } if operation == "append"),
        "{refused:?}"
    );
    assert_eq!(built.get(), 0);

    drop(holder);
    let chain = store
        .append_chain_without_waiting("r1", EventType::EffectCheckpointed, rows(), None, None)
        .unwrap();
    assert_eq!(chain.appended.len(), 2);
    assert_eq!(built.get(), 2);
}
