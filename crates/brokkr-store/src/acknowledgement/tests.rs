//! A quarantined run's acknowledgement (#511): beside the chain, keyed by
//! the head it was made at, and lapsing when the journal moves.

use super::*;
use crate::tests::store;
use crate::DATABASE_SCHEMA;
use brokkr_core::envelope::EventType;
use serde_json::json;

const BY: Attribution<'static> = Attribution {
    operator: "valentin",
    reason: "history from an engine since changed",
};

/// A run with two events, as a quarantined one's journal stands: the store
/// neither folds nor cares whether it would. Its head.
fn run_of_two(store: &mut Store, run_id: &str) -> (u64, String) {
    let feature = "a quarantined run";
    store
        .create_run(run_id, feature, "self", &json!({}))
        .unwrap();
    let started = json!({"feature": feature, "manifest": {}});
    store
        .append_next(run_id, EventType::RunStarted, started, None, None)
        .unwrap();
    move_journal(store, run_id);
    store.head_hash(run_id).unwrap()
}

/// A journal holding run `r1`, acknowledged at its head.
fn acknowledged_run() -> (tempfile::TempDir, Store, Acknowledgement) {
    let (dir, mut store) = store();
    let (seq, hash) = run_of_two(&mut store, "r1");
    let made = store
        .acknowledge_quarantine("r1", (seq, &hash), BY)
        .unwrap();
    (dir, store, made)
}

/// Move a run's journal by one event.
fn move_journal(store: &mut Store, run_id: &str) {
    store
        .append_next(
            run_id,
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            None,
        )
        .unwrap();
}

fn rows(store: &Store) -> i64 {
    store
        .conn
        .query_row(
            "SELECT count(*) FROM quarantine_acknowledgements",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn an_acknowledgement_stands_at_the_head_it_names_beside_an_untouched_chain() {
    let (_dir, mut store) = store();
    let (seq, hash) = run_of_two(&mut store, "r1");
    run_of_two(&mut store, "r2");
    let chain = store.export_ndjson("r1").unwrap();
    assert_eq!(store.acknowledgement("r1").unwrap(), None);

    let made = store
        .acknowledge_quarantine("r1", (seq, &hash), BY)
        .unwrap();
    assert_eq!(
        (made.seq, made.event_hash.as_str(), made.operator.as_str()),
        (2, hash.as_str(), "valentin")
    );
    assert_eq!(made.reason, "history from an engine since changed");
    assert_eq!(store.acknowledgement("r1").unwrap(), Some(made));
    assert_eq!(store.acknowledgement("r2").unwrap(), None, "per run");
    assert_eq!(store.export_ndjson("r1").unwrap(), chain, "no event moved");
    assert_eq!(store.head_hash("r1").unwrap(), (seq, hash));
}

#[test]
fn an_acknowledgement_lapses_when_the_journal_moves() {
    let (_dir, mut store, _) = acknowledged_run();
    move_journal(&mut store, "r1");
    assert_eq!(store.acknowledgement("r1").unwrap(), None);
    assert_eq!(rows(&store), 1, "the lapsed one stays as history");

    // The new head may be acknowledged in its turn.
    let (seq, hash) = store.head_hash("r1").unwrap();
    let again = store
        .acknowledge_quarantine("r1", (seq, &hash), BY)
        .unwrap();
    assert_eq!(again.seq, 3);
    assert_eq!(store.acknowledgement("r1").unwrap(), Some(again));
}

#[test]
fn an_acknowledgement_on_a_moved_head_writes_nothing() {
    let (_dir, mut store) = store();
    let (seq, hash) = run_of_two(&mut store, "r1");
    move_journal(&mut store, "r1");
    let refused = store.acknowledge_quarantine("r1", (seq, &hash), BY);
    assert!(
        matches!(
            refused,
            Err(StoreError::HeadMoved {
                expected_seq: 2,
                found_seq: 3
            })
        ),
        "{refused:?}"
    );
    assert_eq!(rows(&store), 0);
    assert_eq!(store.acknowledgement("r1").unwrap(), None);
}

#[test]
fn a_second_acknowledgement_at_one_head_is_refused() {
    let (_dir, mut store, first) = acknowledged_run();
    let second = Attribution {
        operator: "someone-else",
        reason: "again",
    };
    let head = (first.seq, first.event_hash.as_str());
    let refused = store
        .acknowledge_quarantine("r1", head, second)
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "run 'r1' is already acknowledged at event 2; nothing was recorded"
    );
    assert!(matches!(
        refused,
        StoreError::AlreadyAcknowledged { ref run_id, seq: 2 } if run_id == "r1"
    ));
    assert_eq!(store.acknowledgement("r1").unwrap(), Some(first));
    assert_eq!(rows(&store), 1);
}

#[test]
fn acknowledgements_are_append_only() {
    let (_dir, store, _) = acknowledged_run();
    for statement in [
        "UPDATE quarantine_acknowledgements SET reason = 'rewritten'",
        "DELETE FROM quarantine_acknowledgements",
    ] {
        let refused = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(
            refused.to_string(),
            "quarantine acknowledgements are append-only",
            "{statement}"
        );
    }
    assert_eq!(rows(&store), 1);
}

#[test]
fn an_old_journal_reads_none_read_only_and_gains_the_table_on_a_writable_open() {
    // A journal as an older binary left it: everything but the table,
    // whose guards go with it.
    let (dir, mut store) = store();
    let db = dir.path().join("forge.db");
    store.create_run("old", "feat", "self", &json!({})).unwrap();
    let old = &store.conn;
    old.execute_batch("DROP TABLE quarantine_acknowledgements")
        .unwrap();
    assert!(!acknowledgements_intact(old).unwrap());

    // A read-only open never migrates, and answers none.
    let reader = Store::open_read_only(&db).unwrap();
    assert_eq!(reader.acknowledgement("old").unwrap(), None);
    drop(reader);
    assert!(
        !acknowledgements_intact(old).unwrap(),
        "the read wrote nothing"
    );

    // The first writable open adds the table and its guards, under the
    // same schema version.
    let mut store = Store::open(&db).unwrap();
    assert!(acknowledgements_intact(&store.conn).unwrap());
    let recorded: String = store
        .conn
        .query_row(crate::schema::RECORDED_SCHEMA, [], |row| row.get(0))
        .unwrap();
    assert_eq!((recorded.as_str(), DATABASE_SCHEMA), ("1", 1));
    let empty = store.acknowledge_quarantine("old", (0, ZERO_HASH), BY);
    assert_eq!(empty.unwrap().event_hash, ZERO_HASH, "an empty run's head");
    assert_eq!(store.acknowledgement("old").unwrap().unwrap().seq, 0);
}

/// A journal whose table lost a guard gets it back on the next writable
/// open.
#[test]
fn a_journal_that_lost_a_guard_gets_it_back_on_open() {
    let (dir, store) = store();
    store
        .conn
        .execute_batch("DROP TRIGGER quarantine_acknowledgements_append_only_delete")
        .unwrap();
    assert!(!acknowledgements_intact(&store.conn).unwrap());
    let reopened = Store::open(&dir.path().join("forge.db")).unwrap();
    assert!(acknowledgements_intact(&reopened.conn).unwrap());
}
