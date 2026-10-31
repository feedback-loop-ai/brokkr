//! The run summary (decision 0089, slice 1): the triggers keep what the
//! row itself states for every writer, the writer's carried account keeps
//! what the fold states, an older journal migrates and backfills, and
//! every row that stands at its head reads as its fold.

use super::*;
use crate::test_support::{
    append, every_run_reads_as_its_fold, lags, plant_envelope, plant_run, text,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::{store, write_lock_on};
use crate::{Attribution, Rows};
use brokkr_core::envelope::EventType;
use serde_json::{json, Value as Json};

const BY: Attribution<'static> = Attribution {
    operator: "op",
    reason: "looked at it",
};

/// The row's `columns` for `run_id`, as SQLite holds them.
fn row(store: &Store, run_id: &str, columns: &str) -> Vec<Value> {
    let select = format!("SELECT {columns} FROM run_summary WHERE run_id = ?1");
    let width = columns.split(',').count();
    store
        .conn
        .query_row(&select, [run_id], |row| {
            (0..width).map(|at| row.get(at)).collect()
        })
        .unwrap()
}

fn lane(store: &Store, run_id: &str) -> Value {
    row(store, run_id, "lane").remove(0)
}

/// `run_id` created and started by this store, which carries its account.
fn started(store: &mut Store, run_id: &str) {
    store
        .create_run(run_id, "a feature\nin detail", "self", &json!({}))
        .unwrap();
    let opened = json!({"feature": "a feature", "manifest": {}});
    append(store, run_id, EventType::RunStarted, opened);
}

/// The next event of `run_id`, planted the way a writer from before the
/// summary leaves it: sealed onto the head, and nothing folded written.
fn planted(store: &Store, run_id: &str, event_type: EventType, payload: Json) -> EventEnvelope {
    planted_at(store, run_id, event_type, payload, "2026-01-01T00:00:09Z")
}

fn planted_at(
    store: &Store,
    run_id: &str,
    event_type: EventType,
    payload: Json,
    recorded_at: &str,
) -> EventEnvelope {
    let (seq, previous) = store.head_hash(run_id).unwrap();
    let envelope = EnvelopeBuilder::new(event_type, payload)
        .run(run_id)
        .seq(seq + 1)
        .event_id(format!("{run_id}-{}", seq + 1))
        .at(recorded_at)
        .previous(previous)
        .sealed();
    plant_envelope(&store.conn, &envelope).unwrap();
    envelope
}

fn reopened(store: Store) -> Store {
    let path = store.path().to_path_buf();
    drop(store);
    Store::open(&path).unwrap()
}

#[test]
fn a_new_run_reads_as_an_empty_journal_and_each_append_as_its_fold() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "a feature\nin detail", "self", &json!({}))
        .unwrap();
    assert_eq!(lane(&store, "r1"), text("refused"));
    assert_eq!(
        row(
            &store,
            "r1",
            "feature_line, refusal_seq, refusal_words, folded_seq, head_seq"
        ),
        [
            text("a feature"),
            Value::Integer(0),
            text("journal is empty"),
            Value::Integer(0),
            Value::Integer(0),
        ]
    );
    let opened = json!({"feature": "a feature", "manifest": {}});
    let first = append(&mut store, "r1", EventType::RunStarted, opened);
    assert_eq!(lane(&store, "r1"), text("running"));
    assert_eq!(
        row(
            &store,
            "r1",
            "status, folded_seq, folded_hash, head_hash, refusal_words"
        ),
        [
            text("running"),
            Value::Integer(1),
            text(&first.event_hash),
            text(&first.event_hash),
            Value::Null,
        ]
    );
    let entered = json!({"phase": "implement"});
    append(&mut store, "r1", EventType::PhaseEntered, entered);
    let requested = json!({"effect_id": "fx", "seat": "implementer"});
    append(&mut store, "r1", EventType::EffectRequested, requested);
    assert_eq!(
        row(&store, "r1", "phase, hire_seat, hire_failed_attempts"),
        [text("implement"), text("implementer"), Value::Integer(0)]
    );
    append(
        &mut store,
        "r1",
        EventType::RunParked,
        json!({"reason": "a limit"}),
    );
    assert_eq!(lane(&store, "r1"), text("parked"));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

#[test]
fn a_finished_run_and_an_acknowledged_refusal_read_from_their_own_lanes() {
    let (_dir, mut store) = store();
    started(&mut store, "done");
    append(&mut store, "done", EventType::RunCompleted, json!({}));
    assert_eq!(lane(&store, "done"), text("finished"));
    store
        .create_run("empty", "nothing yet", "self", &json!({}))
        .unwrap();
    store
        .acknowledge_quarantine("empty", (0, ZERO_HASH), BY)
        .unwrap();
    assert_eq!(lane(&store, "empty"), text("acknowledged"));
    assert_eq!(
        row(&store, "empty", "acknowledged, ack_operator, ack_reason"),
        [Value::Integer(1), text("op"), text("looked at it")]
    );
}

#[test]
fn an_older_writer_keeps_presence_head_and_acknowledgement_and_falls_behind() {
    let (_dir, store) = store();
    plant_run(&store.conn, "old", "{}").unwrap();
    assert_eq!(
        row(&store, "old", "head_seq, head_hash, folded_seq, lane"),
        [
            Value::Integer(0),
            text(ZERO_HASH),
            Value::Null,
            text("behind"),
        ]
    );
    let opened = json!({"feature": "feat", "manifest": {}});
    let first = planted(&store, "old", EventType::RunStarted, opened);
    let refused = json!({"effect_id": "fx", "attempt_id": "a"});
    let second = planted(&store, "old", EventType::EffectStarted, refused);
    assert_eq!(
        row(&store, "old", "head_seq, head_hash, last_recorded_at, lane"),
        [
            Value::Integer(2),
            text(&second.event_hash),
            text("2026-01-01T00:00:09Z"),
            text("behind"),
        ]
    );
    let mut store = store;
    store
        .acknowledge_quarantine("old", (2, &second.event_hash), BY)
        .unwrap();
    assert_eq!(row(&store, "old", "acknowledged"), [Value::Integer(1)]);
    assert_ne!(first.event_hash, second.event_hash);

    let store = reopened(store);
    assert_eq!(lane(&store, "old"), text("acknowledged"));
    assert_eq!(
        row(&store, "old", "folded_seq, refusal_seq, feature"),
        [Value::Integer(2), Value::Integer(2), text("feat")]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());

    let parked = json!({"reason": "after the refusal"});
    planted(&store, "old", EventType::RunParked, parked);
    assert_eq!(
        row(
            &store,
            "old",
            "acknowledged, ack_operator, ack_reason, ack_at, lane"
        ),
        [
            Value::Integer(0),
            Value::Null,
            Value::Null,
            Value::Null,
            text("behind"),
        ]
    );
}

#[test]
fn an_account_at_another_head_writes_nothing_folded_and_is_dropped() {
    let (dir, mut store) = store();
    started(&mut store, "r1");
    started(&mut store, "r2");
    // The store carries r2's account: an append to r1 writes r1's row
    // nothing folded, and leaves r2's account carried.
    append(
        &mut store,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "a"}),
    );
    assert_eq!(lane(&store, "r1"), text("behind"));
    append(
        &mut store,
        "r2",
        EventType::PhaseEntered,
        json!({"phase": "b"}),
    );
    assert_eq!(
        row(&store, "r2", "phase, lane"),
        [text("b"), text("running")]
    );

    let mut peer = Store::open(&dir.path().join("forge.db")).unwrap();
    append(&mut peer, "r2", EventType::RunCompleted, json!({}));
    assert_eq!(lane(&store, "r2"), text("behind"));
    append(
        &mut store,
        "r2",
        EventType::OperatorCommanded,
        json!({
            "command_id": "c", "command": "note", "args": {}, "operator": "op"
        }),
    );
    assert_eq!(lane(&store, "r2"), text("behind"));
    assert!(
        store.carried.is_none(),
        "an account at another head is dropped"
    );

    let peer = reopened(peer);
    assert_eq!(lane(&peer, "r1"), text("running"));
    assert_eq!(
        row(&peer, "r2", "status, lane"),
        [text("completed"), text("finished")]
    );
}

#[test]
fn a_carried_account_written_by_an_append_reads_its_runs_row_once() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    let mut store = reopened(store);
    store.carry(Account::whole("r1", &store.load("r1").unwrap()));
    append(
        &mut store,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "a"}),
    );
    append(&mut store, "r1", EventType::RunCompleted, json!({}));
    assert_eq!(
        row(&store, "r1", "feature_line, feature, status, lane"),
        [
            text("a feature"),
            text("a feature\nin detail"),
            text("completed"),
            text("finished"),
        ]
    );
}

#[test]
fn a_chain_carries_the_account_over_the_rows_that_land() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    let entered = json!({"phase": "work"});
    append(&mut store, "r1", EventType::PhaseEntered, entered);
    let requested = json!({"effect_id": "fx", "seat": "worker"});
    append(&mut store, "r1", EventType::EffectRequested, requested);
    let opened = json!({"effect_id": "fx", "attempt_id": "a"});
    append(&mut store, "r1", EventType::EffectStarted, opened);
    let checkpoint = |step: &str| json!({"effect_id": "fx", "checkpoint": {"step": step}});
    let prose = json!({"effect_id": "fx",
        "checkpoint": {"step": "seat-turn", "turn": 1, "content": "private prose"}});
    let rows = [checkpoint("a"), checkpoint("b"), prose.clone()];
    let payloads = |rows| Rows {
        rows,
        payload: &Json::clone,
    };
    let chain = store
        .append_chain(
            "r1",
            EventType::EffectCheckpointed,
            payloads(&rows),
            None,
            None,
        )
        .unwrap();
    assert_eq!(chain.appended.len(), 2);
    assert_eq!(
        row(&store, "r1", "folded_seq, head_seq, lane"),
        [Value::Integer(6), Value::Integer(6), text("running")]
    );
    let refused = store
        .append_chain(
            "r1",
            EventType::EffectCheckpointed,
            payloads(&[prose]),
            None,
            None,
        )
        .unwrap();
    assert_eq!(refused.appended.len(), 0);
    assert_eq!(store.carried.as_ref().unwrap().account.head().0, 6);
}

#[test]
fn a_stamp_past_its_bound_is_cut_at_a_character_boundary_and_kept_whole() {
    let (_dir, store) = store();
    plant_run(&store.conn, "old", "{}").unwrap();
    let base = "2026-01-01T00:00:00Z".to_string() + &"z".repeat(16);
    // Four tails, one for each place the 40th byte can fall: before a
    // one-byte character, and on the first, second and third byte after
    // the start of a wider one.
    for (tail, kept) in [
        ("zzzzz", "zzzz"),
        ("zzz€", "zzz"),
        ("zz😀", "zz"),
        ("z😀z", "z"),
    ] {
        let stamp = format!("{base}{tail}");
        let opened = json!({"feature": "feat", "manifest": {}});
        planted_at(&store, "old", EventType::RunStarted, opened, &stamp);
        assert_eq!(
            row(&store, "old", "last_recorded_at, last_recorded_at_whole"),
            [text(&format!("{base}{kept}")), text(&stamp)],
            "{tail}"
        );
    }
    planted_at(
        &store,
        "old",
        EventType::RunStarted,
        json!({}),
        "2026-01-01T00:00:00Z",
    );
    assert_eq!(
        row(&store, "old", "last_recorded_at, last_recorded_at_whole"),
        [text("2026-01-01T00:00:00Z"), Value::Null]
    );
}

#[test]
fn an_envelope_no_reader_parses_moves_the_head_and_dates_nothing() {
    let (_dir, store) = store();
    plant_run(&store.conn, "old", "{}").unwrap();
    crate::test_support::plant_event(&store.conn, "old", &1, "h", "not json").unwrap();
    assert_eq!(
        row(&store, "old", "head_seq, head_hash, last_recorded_at"),
        [Value::Integer(1), text("h"), Value::Null]
    );
}

#[test]
fn an_acknowledgement_is_copied_cut_by_characters() {
    let (_dir, mut store) = store();
    store
        .create_run("empty", "nothing yet", "self", &json!({}))
        .unwrap();
    let operator = "ö".repeat(ACK_OPERATOR_CHARS + 3);
    let reason = "é".repeat(ACK_REASON_CHARS + 3);
    let by = Attribution {
        operator: &operator,
        reason: &reason,
    };
    let standing = store
        .acknowledge_quarantine("empty", (0, ZERO_HASH), by)
        .unwrap();
    assert_eq!(
        row(&store, "empty", "ack_operator, ack_reason, ack_at"),
        [
            text(&"ö".repeat(ACK_OPERATOR_CHARS)),
            text(&"é".repeat(ACK_REASON_CHARS)),
            text(&standing.recorded_at),
        ]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

#[test]
fn a_run_id_past_its_bound_is_refused_before_anything_is_written() {
    let (_dir, mut store) = store();
    let long = "r".repeat(crate::import::RUN_ID_MAX + 1);
    let refused = store
        .create_run(&long, "feat", "self", &json!({}))
        .unwrap_err();
    assert!(matches!(refused, StoreError::RunIdTooLong { length: 129 }));
    assert_eq!(
        refused.to_string(),
        "a run id is at most 128 bytes; this one is 129, and nothing was recorded"
    );
    assert_eq!(store.list_runs().unwrap(), Vec::new());
    let longest = "r".repeat(crate::import::RUN_ID_MAX);
    store
        .create_run(&longest, "feat", "self", &json!({}))
        .unwrap();
}

/// #820: an id no surface could name back, empty or holding a control
/// character (NUL, the rest of C0, DEL, C1), is refused by name, escaped,
/// before anything is written; one past the bound stays the bound's.
#[test]
fn a_run_id_no_surface_could_name_is_refused_before_anything_is_written() {
    let (_dir, mut store) = store();
    let past = format!("\n{}", "r".repeat(crate::import::RUN_ID_MAX));
    for (run_id, escaped) in [
        ("", ""),
        ("r\0a", "r\\u{0000}a"),
        ("line\nbreak", "line\\u{000a}break"),
        ("r\u{7f}", "r\\u{007f}"),
        ("r\u{85}", "r\\u{0085}"),
    ] {
        let refused = store
            .create_run(run_id, "feat", "self", &json!({}))
            .unwrap_err();
        assert_eq!(
            refused.to_string(),
            format!(
                "a run id is not empty and holds no control character; this one is \
                 '{escaped}', and nothing was recorded"
            )
        );
        assert!(matches!(refused, StoreError::RunIdUnnameable(said) if said == escaped));
    }
    let refused = store.create_run(&past, "feat", "self", &json!({}));
    assert!(matches!(
        refused,
        Err(StoreError::RunIdTooLong { length: 129 })
    ));
    let written = |table: &str| -> i64 {
        let count = format!("SELECT COUNT(*) FROM {table}");
        store.conn.query_row(&count, [], |row| row.get(0)).unwrap()
    };
    assert_eq!([written("runs"), written("events")], [0, 0]);
}

/// A journal as a binary from before the summary left it: three runs,
/// one acknowledged, and none of the summary's table, index or triggers.
fn older_journal() -> (tempfile::TempDir, std::path::PathBuf) {
    let (dir, mut store) = store();
    started(&mut store, "a");
    append(&mut store, "a", EventType::RunCompleted, json!({}));
    started(&mut store, "b");
    store
        .create_run("c", "never started", "self", &json!({}))
        .unwrap();
    store
        .acknowledge_quarantine("c", (0, ZERO_HASH), BY)
        .unwrap();
    let path = store.path().to_path_buf();
    drop(store);
    crate::test_support::unmigrate(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    (dir, path)
}

fn backfilled(store: &Store) -> Option<String> {
    store
        .conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [BACKFILLED],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
}

#[test]
fn a_read_only_open_of_an_older_journal_migrates_nothing() {
    let (_dir, path) = older_journal();
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!summary_intact(&reader.conn).unwrap());
    assert_eq!(backfilled(&reader), None);
}

#[test]
fn a_writable_open_migrates_an_older_journal_and_backfills_every_run() {
    let (_dir, path) = older_journal();
    let store = Store::open(&path).unwrap();
    assert!(summary_intact(&store.conn).unwrap());
    assert_eq!(backfilled(&store), Some("finished".to_string()));
    let lanes: Vec<Value> = ["a", "b", "c"]
        .iter()
        .map(|run| lane(&store, run))
        .collect();
    assert_eq!(
        lanes,
        [text("finished"), text("running"), text("acknowledged")]
    );
    assert_eq!(
        row(&store, "c", "ack_operator, ack_reason"),
        [text("op"), text("looked at it")]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

#[test]
fn a_backfill_cut_short_by_a_peer_s_lock_resumes_at_the_next_open() {
    let (_dir, path) = older_journal();
    // Migrated and seeded, every row behind, and the backfill not begun.
    let conn = rusqlite::Connection::open(&path).unwrap();
    migrate_summary(&conn).unwrap();
    crate::event_index::migrate_index(&conn).unwrap();
    crate::seat_summary::migrate_seats(&conn).unwrap();
    drop(conn);
    let lock = write_lock_on(&path);
    let store = Store::open(&path).unwrap();
    assert_eq!(lane(&store, "a"), text("behind"));
    assert_eq!(backfilled(&store), None);
    drop(store);
    lock.execute_batch("ROLLBACK").unwrap();
    let store = Store::open(&path).unwrap();
    assert_eq!(lane(&store, "a"), text("finished"));
    assert_eq!(backfilled(&store), Some("finished".to_string()));
}

#[test]
fn a_lost_index_or_trigger_is_repaired_and_its_rows_kept() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    for lost in [
        "DROP INDEX run_summary_page",
        "DROP TRIGGER run_summary_of_an_acknowledgement",
    ] {
        let path = store.path().to_path_buf();
        drop(store);
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(lost)
            .unwrap();
        store = Store::open(&path).unwrap();
        assert!(summary_intact(&store.conn).unwrap(), "{lost}");
        assert_eq!(lane(&store, "r1"), text("running"), "{lost}");
    }
}

#[test]
fn a_catch_up_passes_a_run_that_does_not_load_and_one_that_moved() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    started(&mut store, "r2");
    let (_, started_hash) = store.head_hash("r2").unwrap();
    planted(&store, "r1", EventType::PhaseEntered, json!({"phase": "a"}));
    planted(&store, "r2", EventType::PhaseEntered, json!({"phase": "b"}));
    let raw = rusqlite::Connection::open(store.path()).unwrap();
    raw.execute_batch(
        "DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = replace(envelope, '\"a feature\"', '\"spoiled\"')
             WHERE run_id = 'r1' AND seq = 1;",
    )
    .unwrap();
    let mut moved = false;
    store
        .refold_behind_with(|conn| {
            if !moved {
                let event = EnvelopeBuilder::new(EventType::RunCompleted, json!({}))
                    .run("r2")
                    .seq(3)
                    .sealed();
                plant_envelope(conn, &event).unwrap();
                moved = true;
            }
        })
        .unwrap();
    assert_eq!(
        (lane(&store, "r1"), lane(&store, "r2")),
        (text("behind"), text("behind"))
    );
    // The account r2 was read at is not its head's: nothing folded moved.
    assert_eq!(
        row(&store, "r2", "folded_seq, folded_hash, phase"),
        [Value::Integer(1), text(&started_hash), Value::Null]
    );
    lags(&mut store, &["r1", "r2"]);
}

/// A trigger that refuses each statement `refusing` names with `message`:
/// a refusal of the store itself, which no peer's lock makes.
fn refused(store: &Store, refusing: &str, message: &str) {
    let refusal =
        format!("CREATE TRIGGER refusing {refusing} BEGIN SELECT RAISE(ABORT, '{message}'); END;");
    store.conn.execute_batch(&refusal).unwrap();
}

/// A store's catch-up refused by `message`, which it returns rather than
/// passing (ruling 5: a store it cannot write fails closed).
fn assert_refused(caught: Result<(), StoreError>, message: &str) {
    let error = caught.unwrap_err();
    assert!(
        matches!(
            &error,
            StoreError::Sqlite(rusqlite::Error::SqliteFailure(failure, Some(said)))
                if failure.code == rusqlite::ErrorCode::ConstraintViolation && said == message
        ),
        "{error:?}"
    );
    assert_eq!(error.to_string(), format!("sqlite: {message}"));
}

#[test]
fn a_catch_up_returns_a_refused_refold_and_writes_nothing_folded() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    planted(&store, "r1", EventType::PhaseEntered, json!({"phase": "a"}));
    refused(
        &store,
        "BEFORE UPDATE OF folded_seq ON run_summary",
        "the summary refuses",
    );
    assert_refused(store.catch_up(), "the summary refuses");
    assert_eq!(
        row(&store, "r1", "folded_seq, lane"),
        [Value::Integer(1), text("behind")]
    );
    lags(&mut store, &["r1"]);
}

#[test]
fn a_catch_up_returns_a_backfill_it_cannot_record() {
    let (_dir, mut store) = store();
    store
        .conn
        .execute("DELETE FROM meta WHERE key = ?1", [BACKFILLED])
        .unwrap();
    refused(&store, "BEFORE INSERT ON meta", "the meta refuses");
    assert_refused(store.catch_up(), "the meta refuses");
    assert_eq!(backfilled(&store), None);
}

#[test]
fn a_listing_of_a_run_the_journal_does_not_carry_is_refused() {
    let (_dir, store) = store();
    let error = Listed::of(&store.conn, "unknown").unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        ),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "sqlite: Query returned no rows");
}

#[test]
fn a_row_that_stands_at_its_head_and_disagrees_is_named_by_its_first_column() {
    let (_dir, mut store) = store();
    started(&mut store, "agrees");
    started(&mut store, "edited");
    started(&mut store, "corrupt");
    started(&mut store, "gone");
    started(&mut store, "lagging");
    started(&mut store, "unindexed");
    planted(
        &store,
        "lagging",
        EventType::PhaseEntered,
        json!({"phase": "a"}),
    );
    let raw = rusqlite::Connection::open(store.path()).unwrap();
    raw.execute_batch(
        "UPDATE run_summary SET residual_count = 7 WHERE run_id = 'edited';
         DELETE FROM run_summary WHERE run_id = 'gone';
         DELETE FROM event_index WHERE run_id = 'unindexed';
         DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = replace(envelope, '\"a feature\"', '\"spoiled\"')
             WHERE run_id = 'corrupt' AND seq = 1;",
    )
    .unwrap();
    let found = store.summary_disagreements().unwrap();
    let named: Vec<String> = found
        .iter()
        .map(|disagreement| match disagreement {
            Disagreement::Missing { run_id } => format!("{run_id} missing"),
            Disagreement::Column { run_id, column } => format!("{run_id} {column}"),
            Disagreement::Unread { run_id, error } => format!("{run_id} unread: {error}"),
            Disagreement::Indexed { run_id, seq, fault } => format!("{run_id} {seq} {fault:?}"),
            Disagreement::Seated { run_id, row, fault } => format!("{run_id} seat {row} {fault:?}"),
            Disagreement::SeatHead { run_id } => format!("{run_id} seat head"),
        })
        .collect();
    assert_eq!(
        named,
        [
            "corrupt unread: chain: event 1: event_hash does not match canonical content",
            "edited residual_count",
            "gone missing",
            "unindexed 1 Missing",
        ]
    );
    assert_eq!(
        every_run_reads_as_its_fold(&store),
        [
            "corrupt: does not load: chain: event 1: event_hash does not match canonical content",
            "edited: residual_count",
            "gone: no summary row",
            "lagging: behind its head",
            "unindexed: event 1 indexed Missing",
        ]
    );
    lags(
        &mut store,
        &["corrupt", "edited", "gone", "lagging", "unindexed"],
    );
}

#[test]
#[should_panic(expected = "a summary row disagrees with its run's fold: [\"r1: phase\"]")]
fn a_writer_store_checks_every_run_when_it_is_dropped() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    append(
        &mut store,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "a"}),
    );
    store
        .conn
        .execute("UPDATE run_summary SET phase = 'b'", [])
        .unwrap();
}

#[test]
fn a_writer_store_dropped_while_its_test_fails_reports_that_failure() {
    let failed = std::panic::catch_unwind(|| {
        let (_dir, mut store) = store();
        started(&mut store, "r1");
        store
            .conn
            .execute("UPDATE run_summary SET phase = 'b'", [])
            .unwrap();
        panic!("the test's own failure");
    })
    .unwrap_err();
    assert_eq!(
        failed.downcast_ref::<&str>(),
        Some(&"the test's own failure")
    );
}

/// A peer's append that lands between the comparison's reads is not a
/// disagreement: the run is loaded and its row compared within one
/// snapshot, so a healthy journal with a live writer passes the doctor.
#[test]
fn a_comparison_reads_one_snapshot_while_a_peer_appends() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    let mut peer = Store::open(store.path()).unwrap();
    peer.carry(Account::whole("r1", &peer.load("r1").unwrap()));
    let compared = store
        .compared("r1", || {
            let entered = json!({"phase": "implement"});
            append(&mut peer, "r1", EventType::PhaseEntered, entered);
        })
        .unwrap();
    assert!(matches!(compared, Compared::Agrees), "{compared:?}");
    assert_eq!(row(&store, "r1", "folded_seq"), [Value::Integer(2)]);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A store that carried an account and lost it to a peer's append is
/// still held to its journal when it drops, and a row it left behind its
/// head is named.
#[test]
#[should_panic(expected = "a summary row disagrees with its run's fold: [\"r1: behind its head\"]")]
fn a_writer_store_that_lost_its_account_is_still_checked_when_it_is_dropped() {
    let (_dir, mut store) = store();
    started(&mut store, "r1");
    let mut peer = Store::open(store.path()).unwrap();
    append(
        &mut peer,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "a"}),
    );
    append(
        &mut store,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "b"}),
    );
    assert!(store.carried.is_none(), "the account was lost");
}

/// `r1` started, a store that reopened it carrying its account, a peer's
/// event planted past that account, then `rows` appended by the store as
/// one chain: the store, and how many rows the chain landed.
fn chained_past_a_peer(rows: &[Json]) -> (tempfile::TempDir, Store, usize) {
    let (dir, mut creator) = store();
    started(&mut creator, "r1");
    let mut store = reopened(creator);
    store.carry(Account::whole("r1", &store.load("r1").unwrap()));
    planted(&store, "r1", EventType::PhaseEntered, json!({"phase": "a"}));
    let chain = store
        .append_chain(
            "r1",
            EventType::PhaseEntered,
            Rows {
                rows,
                payload: &Json::clone,
            },
            None,
            None,
        )
        .unwrap();
    let landed = chain.appended.len();
    (dir, store, landed)
}

/// A run a store wrote only through a chain is held to its fold too: the
/// chain that lost the account leaves its run behind, and is named.
#[test]
#[should_panic(expected = "a summary row disagrees with its run's fold: [\"r1: behind its head\"]")]
fn a_writer_store_that_lost_its_account_in_a_chain_is_checked_when_it_is_dropped() {
    let (_dir, store, landed) = chained_past_a_peer(&[json!({"phase": "b"})]);
    assert_eq!(landed, 1);
    assert!(store.carried.is_none(), "the account was lost");
}

/// A chain that lands nothing leaves its run as it found it, and the
/// store is not held to it when it drops: a run behind its head that no
/// row of the chain reached is not named.
#[test]
fn a_chain_that_lands_nothing_holds_its_store_to_no_run() {
    let (_dir, store, landed) = chained_past_a_peer(&[]);
    assert_eq!(landed, 0);
    assert_eq!(lane(&store, "r1"), text("behind"));
}

#[test]
#[should_panic(expected = "the summary comparison reads: sqlite: no such table: run_summary")]
fn a_comparison_that_cannot_read_panics_rather_than_passing() {
    let (_dir, store) = store();
    plant_run(&store.conn, "r1", "{}").unwrap();
    store.conn.execute_batch("DROP TABLE run_summary").unwrap();
    every_run_reads_as_its_fold(&store);
}

/// A run that enters `verify`, asks effect `e1`, starts it, and has a
/// `stop` accepted at seq 6 with no ending: the stopping fold the two
/// migration tests below read.
fn stopping_without_ending(store: &mut Store) {
    started(store, "r");
    append(
        store,
        "r",
        EventType::PhaseEntered,
        json!({"phase": "verify"}),
    );
    append(
        store,
        "r",
        EventType::EffectRequested,
        json!({"effect_id": "e1", "seat": "verify"}),
    );
    append(
        store,
        "r",
        EventType::EffectStarted,
        json!({"effect_id": "e1", "attempt_id": "a1"}),
    );
    append(
        store,
        "r",
        EventType::OperatorCommanded,
        json!({"command_id": "c1", "command": "stop"}),
    );
    append(
        store,
        "r",
        EventType::OperatorAccepted,
        json!({"command_id": "c1"}),
    );
}

/// The refolded row reads as the stopping run it is, at seq 6, with both
/// of the accepted stop's columns written.
fn stopping_row(store: &Store) {
    assert_eq!(lane(store, "r"), text("stopping"));
    assert_eq!(row(store, "r", "stop_accepted_seq"), [Value::Integer(6)]);
    assert_eq!(row(store, "r", "stop_folded_seq"), [Value::Integer(6)]);
    assert_eq!(every_run_reads_as_its_fold(store), Vec::<String>::new());
}

/// A journal whose summary a build from before the accepted-stop column
/// made is made anew at the next writable open, and the stopping run is
/// refolded into the stopping lane: the column is additive, and
/// `DATABASE_SCHEMA` does not move for it.
#[test]
fn a_summary_from_before_the_accepted_stop_column_is_made_anew_and_refolded() {
    let (_dir, mut store) = store();
    stopping_without_ending(&mut store);
    assert_eq!(lane(&store, "r"), text("stopping"));
    assert_eq!(row(&store, "r", "stop_accepted_seq"), [Value::Integer(6)]);
    let path = store.path().to_path_buf();
    drop(store);
    let older = rusqlite::Connection::open(&path).unwrap();
    older.execute_batch("DROP TABLE run_summary;").unwrap();
    assert!(!summary_intact(&older).unwrap(), "the table is gone");
    migrate_summary(&older).unwrap();
    drop(older);
    let store = Store::open(&path).unwrap();
    assert!(summary_intact(&store.conn).unwrap());
    assert!(store.pages().unwrap());
    stopping_row(&store);
}

/// A journal a build from before the accepted-stop columns wrote carries
/// the aggregates but not the stop's two columns: the summary reads as not
/// intact, and the repair makes it anew and refolds the stopping run.
#[test]
fn a_summary_with_its_aggregates_but_not_the_stop_columns_is_made_anew() {
    let (_dir, mut store) = store();
    stopping_without_ending(&mut store);
    let path = store.path().to_path_buf();
    drop(store);
    let older = rusqlite::Connection::open(&path).unwrap();
    // The older build's table: every column but the stop's two, and none of
    // the triggers or indexes this build adds.
    let kept: Vec<String> = older
        .prepare(
            "SELECT name FROM pragma_table_info('run_summary')
             WHERE name NOT IN ('stop_accepted_seq', 'stop_folded_seq')",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    older
        .execute_batch(&format!(
            "CREATE TEMP TABLE older AS SELECT {} FROM run_summary;
             DROP TABLE run_summary;
             CREATE TABLE run_summary AS SELECT * FROM older;
             DROP TABLE older;",
            kept.join(", ")
        ))
        .unwrap();
    assert!(
        !summary_intact(&older).unwrap(),
        "the stop's columns are gone"
    );
    migrate_summary(&older).unwrap();
    drop(older);
    let store = Store::open(&path).unwrap();
    assert!(summary_intact(&store.conn).unwrap());
    stopping_row(&store);
}

/// A writer from before the accepted-stop columns may fold while their
/// trigger is gone: its row stands at its head with the stop's columns
/// unwritten, so the open's repair sends it behind on the stop's own stamp
/// and the backfill refolds it. The stop's stamp proves the columns were
/// written, not merely that the table carries them.
#[test]
fn a_row_an_older_writer_folds_without_the_stop_stamp_is_sent_behind_and_refolded() {
    let (_dir, mut store) = store();
    stopping_without_ending(&mut store);
    assert_eq!(lane(&store, "r"), text("stopping"));
    let path = store.path().to_path_buf();
    // The older binary's trigger is gone, and it writes the folded head and
    // the aggregates it knows while leaving the stop's columns alone.
    store
        .conn
        .execute_batch("DROP TRIGGER run_summary_stopping")
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash,
                 aggregated_seq = head_seq, stop_accepted_seq = NULL,
                 stop_folded_seq = NULL
             WHERE run_id = 'r'",
            [],
        )
        .unwrap();
    assert_eq!(
        row(&store, "r", "stop_accepted_seq, stop_folded_seq"),
        [Value::Null, Value::Null]
    );
    // The row lags on purpose until the next writable open refolds it.
    lags(&mut store, &["r"]);
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(summary_intact(&store.conn).unwrap(), "the trigger is back");
    stopping_row(&store);
}
