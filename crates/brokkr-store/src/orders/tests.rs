use std::path::{Path, PathBuf};

use rusqlite::Error as SqlError;
use tempfile::TempDir;

use super::*;

const DRAFT: &str = "fixtures/work-order/draft.md";
const COMPLETE: &str = "fixtures/work-order/complete.md";

const BY: Attribution<'_> = Attribution {
    operator: "operator",
    reason: "the operator's reasons stay with the work",
};

fn scratch() -> (TempDir, PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.db");
    let store = Store::open(&path).unwrap();
    (dir, path, store)
}

fn fixture(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(path).unwrap()
}

fn trackers() -> Trackers {
    Trackers::from_definitions([fixture("crates/brokkr-cli/trackers/github.json").as_str()])
        .unwrap()
}

fn id(number: u32) -> WorkOrderId {
    WorkOrderId::from_number(number).unwrap()
}

fn rule_current(store: &mut Store, id: &WorkOrderId) -> Result<(), StoreError> {
    let latest = current(&store.conn, id)?;
    store.order_rule(id, &latest.digest, BY)
}

fn draft() -> WorkOrder {
    WorkOrder::parse(&fixture(DRAFT), &trackers()).unwrap()
}

fn refusal<T: std::fmt::Debug>(result: Result<T, StoreError>, text: &str) -> OrderRefusal {
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), text);
    match error {
        StoreError::Orders(refusal) => refusal,
        other => panic!("expected a work order refusal, got {other:?}"),
    }
}

#[test]
fn fresh_and_older_journals_gain_orders_schema_one() {
    let (_dir, path, store) = scratch();
    assert_eq!(orders_schema(&store.conn).unwrap(), Some(1));
    assert!(orders_intact(&store.conn).unwrap());
    store
        .conn
        .execute_batch(
            "DELETE FROM meta WHERE key = 'orders_schema'; DROP TABLE work_order_events;",
        )
        .unwrap();
    drop(store);

    let reader = Store::open_read_only(&path).unwrap();
    assert!(matches!(
        refusal(
            reader.orders(&trackers()),
            "this build does not read work order storage version 0"
        ),
        OrderRefusal::SchemaMismatch(0)
    ));
    drop(reader);
    let store = Store::open(&path).unwrap();
    assert_eq!(orders_schema(&store.conn).unwrap(), Some(1));
    assert_eq!(store.orders(&trackers()).unwrap(), []);
}

#[test]
fn missing_guards_are_repaired_but_lost_history_refuses_open() {
    let (_dir, path, mut store) = scratch();
    let work = store.order_add(draft(), BY).unwrap();
    for (kind, name) in [("TRIGGER", UPDATE_GUARD), ("INDEX", PROPOSED_INDEX)] {
        store
            .conn
            .execute_batch(&format!("DROP {kind} {name}"))
            .unwrap();
        drop(store);
        store = Store::open(&path).unwrap();
        assert!(orders_intact(&store.conn).unwrap());
        assert_eq!(
            store.order(&work, &trackers()).unwrap(),
            draft().with_id(work.clone())
        );
    }
    store
        .conn
        .execute_batch("DROP TABLE work_order_events")
        .unwrap();
    drop(store);
    assert!(matches!(
        Store::open(&path),
        Err(StoreError::Damaged { journal, damage: Damage::Orders }) if journal == path
    ));
}

#[test]
fn damaged_work_order_storage_refuses_every_read_and_write() {
    for sql in [
        "UPDATE meta SET value = 'damaged' WHERE key = 'orders_schema'",
        "UPDATE meta SET value = '0' WHERE key = 'orders_schema'",
        "UPDATE meta SET value = X'FF' WHERE key = 'orders_schema'",
        "DROP TABLE work_order_events",
    ] {
        let (_dir, path, mut writer) = scratch();
        let work = writer.order_add(draft(), BY).unwrap();
        writer.conn.execute_batch(sql).unwrap();
        let reader = Store::open_read_only(&path).unwrap();
        let trackers = trackers();
        for error in [
            reader.orders(&trackers).unwrap_err(),
            reader.order(&work, &trackers).unwrap_err(),
            writer.order_add(draft(), BY).unwrap_err(),
            writer.order_edit(&work, draft(), BY).unwrap_err(),
            writer.order_rule(&work, "", BY).unwrap_err(),
            Store::open(&path)
                .err()
                .expect("writable open refuses damage"),
        ] {
            assert!(
                matches!(
                    error,
                    StoreError::Damaged { ref journal, damage: Damage::Orders } if *journal == path
                ),
                "{sql}: {error:?}"
            );
        }
    }
}

#[test]
fn an_unknown_schema_is_left_untouched_and_every_method_refuses_it() {
    let (_dir, path, store) = scratch();
    store
        .conn
        .execute_batch("UPDATE meta SET value = '9' WHERE key = 'orders_schema'; DROP TABLE work_order_events;")
        .unwrap();
    drop(store);
    let mut store = Store::open(&path).unwrap();
    // A repair racing a version change also leaves the newer storage alone.
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    migrate_orders(&tx).unwrap();
    tx.commit().unwrap();
    assert_eq!(orders_schema(&store.conn).unwrap(), Some(9));
    assert!(!all_present(&store.conn, "table", &["work_order_events"]).unwrap());
    let work = id(1);
    let trackers = trackers();
    let errors = [
        store.order_add(draft(), BY).unwrap_err(),
        store.order_edit(&work, draft(), BY).unwrap_err(),
        store.order_rule(&work, "", BY).unwrap_err(),
        store.orders(&trackers).unwrap_err(),
        store.order(&work, &trackers).unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(
            refusal(
                Err::<(), _>(error),
                "this build does not read work order storage version 9"
            ),
            OrderRefusal::SchemaMismatch(9)
        ));
    }
}

#[test]
fn add_assigns_after_the_highest_number_and_refuses_a_second_proposal() {
    let (_dir, _, mut store) = scratch();
    for number in [1, 2] {
        assert_eq!(store.order_add(draft(), BY).unwrap(), id(number));
    }
    for number in [12, 4] {
        assert_eq!(
            store.order_add(draft().with_id(id(number)), BY).unwrap(),
            id(number)
        );
    }
    assert_eq!(store.order_add(draft(), BY).unwrap(), id(13));
    assert!(matches!(
        refusal(store.order_add(draft().with_id(id(12)), BY), "WO-12 was already proposed; edit it to change its text"),
        OrderRefusal::Exists(work) if work == id(12)
    ));
    let listed = store.orders(&trackers()).unwrap();
    assert_eq!(
        listed,
        [1, 2, 4, 12, 13].map(|number| draft().with_id(id(number)))
    );
}

#[test]
fn a_zero_sql_number_cannot_be_read_as_an_order_id() {
    let (_dir, _, store) = scratch();
    assert!(matches!(
        store.conn.query_row("SELECT 0", [], read_id),
        Err(SqlError::IntegralValueOutOfRange(0, 0))
    ));
}

#[test]
fn assigning_past_the_last_number_refuses_without_wrapping() {
    let (_dir, _, mut store) = scratch();
    let last = id(u32::MAX);
    assert_eq!(
        store.order_add(draft().with_id(last.clone()), BY).unwrap(),
        last
    );
    assert!(matches!(
        store.order_add(draft(), BY),
        Err(StoreError::Sqlite(SqlError::IntegralValueOutOfRange(
            0,
            4_294_967_296
        )))
    ));
    assert_eq!(store.orders(&trackers()).unwrap(), [draft().with_id(last)]);
}

#[test]
fn a_ruled_header_is_journaled_and_read_as_proposed() {
    let (_dir, _, mut store) = scratch();
    let trackers = trackers();
    let order = WorkOrder::parse(&fixture(COMPLETE), &trackers).unwrap();
    let id = store.order_add(order.clone(), BY).unwrap();
    let expected = order.with_state(WorkOrderState::Proposed);
    assert_eq!(current(&store.conn, &id).unwrap().text, expected.render());
    assert_eq!(store.order(&id, &trackers).unwrap(), expected);
}

#[test]
fn commands_and_reads_refuse_unknown_orders_and_edits_refuse_other_ids() {
    let (_dir, _, mut store) = scratch();
    let unknown = id(2);
    for result in [
        store.order_edit(&unknown, draft(), BY),
        store.order_rule(&unknown, "", BY),
        store.order(&unknown, &trackers()).map(drop),
    ] {
        assert!(matches!(
            refusal(result, "no work order was proposed as WO-2"),
            OrderRefusal::Unknown(work) if work == unknown
        ));
    }
    let known = store.order_add(draft(), BY).unwrap();
    assert!(matches!(
        refusal(store.order_edit(&known, draft().with_id(unknown.clone()), BY),
            "the edit names WO-2; it must name the work order being edited or omit its id"),
        OrderRefusal::OtherId(work) if work == unknown
    ));
}

#[test]
fn editing_returns_to_proposed_and_a_later_ruling_of_old_text_does_not_stand() {
    let (_dir, _, mut store) = scratch();
    let trackers = trackers();
    let first = draft();
    let id = store.order_add(first.clone(), BY).unwrap();
    let first_read = store.order(&id, &trackers).unwrap();
    let digest = first_read.digest();
    store.order_rule(&id, digest, BY).unwrap();
    assert_eq!(
        store.order(&id, &trackers).unwrap(),
        first
            .clone()
            .with_id(id.clone())
            .with_state(WorkOrderState::Ruled)
    );

    // Even byte-for-byte identical text needs a new ruling after an edit.
    store
        .order_edit(&id, first.clone().with_id(id.clone()), BY)
        .unwrap();
    assert_eq!(
        store.order(&id, &trackers).unwrap(),
        first.with_id(id.clone())
    );

    let text = fixture(DRAFT).replace("Keep the work clear.", "Describe the edited work.");
    let edited = WorkOrder::parse(&text, &trackers)
        .unwrap()
        .with_id(id.clone());
    store
        .order_edit(&id, edited.clone().with_state(WorkOrderState::Ruled), BY)
        .unwrap();
    assert_eq!(current(&store.conn, &id).unwrap().text, edited.render());
    assert_eq!(store.order(&id, &trackers).unwrap(), edited);
    let events = history(&store).len();
    assert!(matches!(
        refusal(
            store.order_rule(&id, digest, BY),
            "the text of WO-1 changed since that digest was read; show it again and rule the digest it prints",
        ),
        OrderRefusal::StaleDigest(work) if work == id
    ));
    assert_eq!(history(&store).len(), events);

    // Plant a stale ruling after the edit to exercise the digest comparison
    // independently of the sequence comparison.
    record(&store.conn, &id, Event::Ruled(digest), BY).unwrap();
    assert_eq!(store.order(&id, &trackers).unwrap(), edited);
    rule_current(&mut store, &id).unwrap();
    assert_eq!(
        store.order(&id, &trackers).unwrap(),
        edited.with_state(WorkOrderState::Ruled)
    );
}

#[test]
fn ruling_waits_for_the_open_ask_and_can_only_be_recorded_once_per_text() {
    let (_dir, _, mut store) = scratch();
    let trackers = trackers();
    let text = fixture(COMPLETE).replacen(",\n      \"ruling\": 1", "", 1);
    let open = WorkOrder::parse(&text, &trackers).unwrap();
    let id = store.order_add(open, BY).unwrap();
    assert!(matches!(
        refusal(rule_current(&mut store, &id), "WO-12 has an open ask; close its asks before ruling it"),
        OrderRefusal::OpenAsk(work) if work == id
    ));
    assert_eq!(
        store.order(&id, &trackers).unwrap().state(),
        WorkOrderState::Proposed
    );
    let closed = WorkOrder::parse(&fixture(COMPLETE), &trackers).unwrap();
    store.order_edit(&id, closed.clone(), BY).unwrap();
    rule_current(&mut store, &id).unwrap();
    assert_eq!(store.order(&id, &trackers).unwrap(), closed);
    assert!(matches!(
        refusal(rule_current(&mut store, &id), "WO-12 is already ruled at its current text"),
        OrderRefusal::AlreadyRuled(work) if work == id
    ));
}

#[derive(Debug, PartialEq, Eq)]
struct Recorded {
    event: String,
    text: Option<String>,
    digest: String,
    operator: String,
    reason: String,
    recorded_at: String,
}

fn history(store: &Store) -> Vec<Recorded> {
    let mut statement = store.conn.prepare(
        "SELECT event, text, digest, operator, reason, recorded_at FROM work_order_events ORDER BY seq",
    ).unwrap();
    statement
        .query_map([], |row| {
            Ok(Recorded {
                event: row.get(0)?,
                text: row.get(1)?,
                digest: row.get(2)?,
                operator: row.get(3)?,
                reason: row.get(4)?,
                recorded_at: row.get(5)?,
            })
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn assert_constraint(result: rusqlite::Result<usize>, extended: i32, message: Option<&str>) {
    assert!(matches!(result, Err(SqlError::SqliteFailure(failure, text))
        if failure.extended_code == extended && message.is_none_or(|message| text.as_deref() == Some(message))));
}

#[test]
fn every_version_is_replayable_and_events_are_append_only() {
    let (_dir, _, mut store) = scratch();
    let order = draft();
    let id = store.order_add(order.clone(), BY).unwrap();
    let changed = fixture(DRAFT).replace("Keep the work clear.", "Record each version.");
    let edited = WorkOrder::parse(&changed, &trackers()).unwrap();
    store.order_edit(&id, edited.clone(), BY).unwrap();
    rule_current(&mut store, &id).unwrap();

    let rows = history(&store);
    let proposed = order.with_id(id.clone()).render();
    let edited = edited.with_id(id).render();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .map(|row| (row.event.as_str(), row.text.as_deref(), row.digest.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                "proposed",
                Some(proposed.as_str()),
                "167afdc611b69fc43f54e9c13f4e5aba53d2b99e52f7114125217225b32659ae"
            ),
            (
                "edited",
                Some(edited.as_str()),
                "bc2995047bed7afa02e20b918d680327fcd8148f6bf81fa6636dba0ce819d4c6"
            ),
            (
                "ruled",
                None,
                "bc2995047bed7afa02e20b918d680327fcd8148f6bf81fa6636dba0ce819d4c6"
            ),
        ]
    );
    for row in &rows {
        assert_eq!((&*row.operator, &*row.reason), (BY.operator, BY.reason));
        time::OffsetDateTime::parse(
            &row.recorded_at,
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
    }
    for sql in [
        "UPDATE work_order_events SET reason = 'changed'",
        "DELETE FROM work_order_events",
    ] {
        assert_constraint(
            store.conn.execute(sql, []),
            rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER,
            Some("work order events are append-only"),
        );
    }
    assert_eq!(history(&store), rows);
}

#[test]
fn the_table_refuses_duplicate_proposals_and_wrong_event_payloads() {
    let (_dir, _, mut store) = scratch();
    let draft_text = fixture(DRAFT);
    store.order_add(draft(), BY).unwrap();
    let insert =
        "INSERT INTO work_order_events (number, event, text, digest, operator, reason, recorded_at)
        VALUES (?1, ?2, ?3, '', '', '', '')";
    for (number, event, text) in [
        (2, "proposed", None),
        (1, "edited", None),
        (1, "ruled", Some(draft_text.as_str())),
        (1, "unknown", None),
        (0, "proposed", Some(draft_text.as_str())),
    ] {
        assert_constraint(
            store.conn.execute(insert, params![number, event, text]),
            rusqlite::ffi::SQLITE_CONSTRAINT_CHECK,
            None,
        );
    }
    assert_constraint(
        store
            .conn
            .execute(insert, params![1, "proposed", draft_text]),
        rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE,
        None,
    );
    assert_eq!(history(&store).len(), 1);
}

#[test]
fn unreadable_text_is_reported_by_reads_and_cannot_be_ruled_if_malformed() {
    let (_dir, _, mut store) = scratch();
    let order = WorkOrder::parse(&fixture(COMPLETE), &trackers()).unwrap();
    let id = store.order_add(order, BY).unwrap();
    let empty = Trackers::from_definitions([]).unwrap();
    for result in [
        store.order(&id, &empty).map(drop),
        store.orders(&empty).map(drop),
    ] {
        assert!(matches!(
            refusal(result, "the journal's text for WO-12 cannot be read: the work order cites unknown tracker 'github'"),
            OrderRefusal::Unreadable(work, _) if work == id
        ));
    }
    record(&store.conn, &id, Event::Edited("unreadable"), BY).unwrap();
    assert!(matches!(
        refusal(rule_current(&mut store, &id), "the journal's text for WO-12 cannot be read: a work order must start with a fenced JSON header"),
        OrderRefusal::Unreadable(work, _) if work == id
    ));
}
