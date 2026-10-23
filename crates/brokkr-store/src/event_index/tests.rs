//! The event index (decision 0089, slice 2): the trigger indexes every
//! insert from its envelope as brokkr-core projects the event, an older
//! journal is backfilled at a writer's open and resumes a backfill cut
//! short, a read-only reader falls back on a journal not yet indexed, the
//! keyed reads read their indexes, and the comparison names where a run's
//! rows part from its events.

use super::*;
use crate::test_support::{plant_event, plant_run, unindex};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::walk::cut_short;
use crate::tests::{store, write_lock_on};
use serde_json::{json, Value as Json};

/// The next event of `run_id`, sealed onto its head.
fn next(store: &Store, run_id: &str, built: EnvelopeBuilder) -> EventEnvelope {
    let (seq, previous) = store.head_hash(run_id).unwrap();
    built
        .run(run_id)
        .seq(seq + 1)
        .event_id(format!("{run_id}-{}", seq + 1))
        .previous(previous)
        .sealed()
}

/// The next event of `run_id`, inserted as an older binary inserts one:
/// past every writer the store keeps, so the trigger alone indexes it.
fn planted(store: &Store, run_id: &str, built: EnvelopeBuilder) -> EventEnvelope {
    let envelope = next(store, run_id, built);
    crate::test_support::plant_envelope(&store.conn, &envelope).unwrap();
    envelope
}

fn event(event_type: EventType, payload: Json) -> EnvelopeBuilder {
    EnvelopeBuilder::new(event_type, payload)
}

/// A run `run_id` of `events` planted in `store`.
fn run_of(store: &Store, run_id: &str, events: Vec<EnvelopeBuilder>) {
    plant_run(&store.conn, run_id, "{}").unwrap();
    for built in events {
        planted(store, run_id, built);
    }
}

/// `run_id`'s index rows as SQLite holds them, in seq order.
fn rows(store: &Store, run_id: &str) -> Vec<Vec<Value>> {
    let mut statement = store.conn.prepare(ROWS).unwrap();
    let found = statement
        .query_map([run_id], |row| (0..5).map(|at| row.get(at)).collect())
        .unwrap();
    found.collect::<Result<_, _>>().unwrap()
}

fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// The stamp every built envelope carries unless it is given another.
const STAMP: &str = "2026-01-01T00:00:00Z";

fn row(seq: i64, kind: &str, attempt: Option<&str>, member: Option<&str>) -> Vec<Value> {
    let named = |name: Option<&str>| name.map_or(Value::Null, text);
    vec![
        Value::Integer(seq),
        text(kind),
        named(attempt),
        named(member),
        text(STAMP),
    ]
}

/// Where `run_id`'s rows part from its events, read whole.
fn parted_at(store: &Store, run_id: &str) -> Option<(u64, IndexFault)> {
    let events = store.load(run_id).unwrap();
    first_unindexed(&store.conn, run_id, &events).unwrap()
}

fn backfilled(store: &Store) -> bool {
    indexed(&store.conn).unwrap()
}

/// Every insert, an older binary's included, is indexed from its
/// envelope: its type, the payload's attempt (never the envelope's), the
/// checkpoint's member or, with no checkpoint, the result's, each only
/// where it is text; and the rows are what brokkr-core projects the
/// events to.
#[test]
fn every_insert_is_indexed_from_its_envelope_as_its_event_projects() {
    let (_dir, store) = store();
    run_of(
        &store,
        "r",
        vec![
            event(EventType::RunStarted, json!({"feature": "f"})),
            event(EventType::EffectStarted, json!({"attempt_id": "a1"})),
            event(
                EventType::EffectCheckpointed,
                json!({"attempt_id": "a1", "checkpoint": {"member": "m1"}, "result": {"member": "r"}}),
            )
            .attempt(Some("the envelope's")),
            event(
                EventType::EffectSucceeded,
                json!({"attempt_id": "a1", "result": {"member": "m2"}}),
            ),
            event(
                EventType::EffectCheckpointed,
                json!({"attempt_id": 7, "checkpoint": {"member": 8}, "result": {"member": "r"}}),
            ),
        ],
    );
    let checkpointed = "effect/checkpointed";
    assert_eq!(
        rows(&store, "r"),
        [
            row(1, "run/started", None, None),
            row(2, "effect/started", Some("a1"), None),
            row(3, checkpointed, Some("a1"), Some("m1")),
            row(4, "effect/succeeded", Some("a1"), Some("m2")),
            row(5, checkpointed, None, None),
        ]
    );
    assert_eq!(parted_at(&store, "r"), None);
}

/// A payload that repeats a key, which its event's hash cannot see since
/// the chain hashes the event as serde parses it, keeping a key's last
/// copy, is indexed by each key's last copy, as every reader reads it:
/// the attempt, the record and the record's member, each only where that
/// last copy is text.
#[test]
fn a_payload_that_repeats_a_key_is_indexed_by_its_last_copy() {
    let (_dir, store) = store();
    plant_run(&store.conn, "r", "{}").unwrap();
    let edited = |built, edits: [(&str, &str); 2]| {
        let envelope = next(&store, "r", built);
        let mut text = serde_json::to_string(&envelope).unwrap();
        for (from, to) in edits {
            text = text.replacen(from, to, 1);
        }
        let seq = envelope.seq as i64;
        plant_event(&store.conn, "r", &seq, &envelope.event_hash, &text).unwrap();
    };
    edited(
        event(
            EventType::EffectCheckpointed,
            json!({"attempt_id": "a", "checkpoint": {"member": "m"}}),
        ),
        [
            (
                r#""attempt_id":"a""#,
                r#""attempt_id":"stale","attempt_id":"a""#,
            ),
            (
                r#""checkpoint":{"member":"m"}"#,
                r#""checkpoint":{"member":"x"},"checkpoint":{"member":"stale","member":"m"}"#,
            ),
        ],
    );
    edited(
        event(
            EventType::EffectSucceeded,
            json!({"attempt_id": 7, "checkpoint": null, "result": {"member": "r"}}),
        ),
        [
            (
                r#""attempt_id":7"#,
                r#""attempt_id":"stale","attempt_id":7"#,
            ),
            (
                r#""checkpoint":null"#,
                r#""checkpoint":{"member":"stale"},"checkpoint":null"#,
            ),
        ],
    );
    assert_eq!(
        rows(&store, "r"),
        [
            row(1, "effect/checkpointed", Some("a"), Some("m")),
            row(2, "effect/succeeded", None, None),
        ]
    );
    assert_eq!(parted_at(&store, "r"), None);
    assert_eq!(
        store.index_checkpoints("r", "a", Some("m"), None).unwrap(),
        [1]
    );
}

/// An envelope that is not JSON is indexed with nothing read from it, and
/// its insert lands.
#[test]
fn an_envelope_no_reader_parses_is_indexed_empty_and_still_lands() {
    let (_dir, store) = store();
    plant_run(&store.conn, "raw", "{}").unwrap();
    plant_event(&store.conn, "raw", &1, "h", "not json").unwrap();
    assert_eq!(
        rows(&store, "raw"),
        [vec![
            Value::Integer(1),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null
        ]]
    );
}

/// A row's time is its envelope's stamp, cut as the run summary cuts one:
/// whole within the bound, and past it at the character boundary at or
/// below it, whether the event's trigger or the backfill wrote the row.
#[test]
fn a_row_s_time_is_its_envelope_s_stamp_cut_as_the_summary_cuts_it() {
    let (_dir, store) = store();
    let long = format!("{}é{}", "2".repeat(STAMP_BYTES - 1), "z".repeat(9));
    let stamped = |stamp: &str| event(EventType::RunStarted, json!({})).at(stamp);
    run_of(&store, "r", vec![stamped(STAMP), stamped(&long)]);
    let cut = &long[..STAMP_BYTES - 1];
    let times = |store: &Store| -> Vec<Value> {
        rows(store, "r")
            .into_iter()
            .map(|row| row[4].clone())
            .collect()
    };
    assert_eq!(times(&store), [text(STAMP), text(cut)]);
    let path = store.path().to_path_buf();
    drop(store);
    unindex(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let store = Store::open(&path).unwrap();
    assert_eq!(times(&store), [text(STAMP), text(cut)]);
    assert_eq!(parted_at(&store, "r"), None);
}

/// Three runs of a few events each, planted with no event index, as a
/// binary from before it leaves them.
fn older_journal() -> (tempfile::TempDir, std::path::PathBuf) {
    let (dir, store) = store();
    for run_id in ["a", "b", "c"] {
        let opened = event(EventType::RunStarted, json!({"feature": "f"}));
        let started = event(EventType::EffectStarted, json!({"attempt_id": run_id}));
        run_of(&store, run_id, vec![opened, started]);
    }
    let path = store.path().to_path_buf();
    drop(store);
    unindex(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    (dir, path)
}

#[test]
fn a_writable_open_indexes_an_older_journal_and_a_read_only_one_falls_back() {
    let (_dir, path) = older_journal();
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!index_intact(&reader.conn).unwrap());
    assert!(!reader.reads_index().unwrap());
    let store = Store::open(&path).unwrap();
    assert!(store.reads_index().unwrap());
    assert!(reader.reads_index().unwrap());
    each_older_run_indexed(&store);
}

/// Every run of [`older_journal`] holds its two rows, each its event's.
fn each_older_run_indexed(store: &Store) {
    for run_id in ["a", "b", "c"] {
        assert_eq!(rows(store, run_id).len(), 2, "{run_id}");
        assert_eq!(parted_at(store, run_id), None, "{run_id}");
    }
}

/// A journal that carries events but records no schema, as an edited or
/// partly restored one may, is initialized at a writable open, and its
/// events are indexed before the backfill is marked finished.
#[test]
fn a_journal_with_events_and_no_schema_is_indexed_at_its_open() {
    let (_dir, path) = older_journal();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("DELETE FROM meta WHERE key = 'database_schema'", [])
        .unwrap();
    drop(conn);
    let store = Store::open(&path).unwrap();
    assert!(store.reads_index().unwrap());
    each_older_run_indexed(&store);
}

/// A journal holding the table an unreleased build of this slice made,
/// without its time but with its indexes and trigger, is not read by its
/// index and refuses its reader nothing, and a writable open rebuilds the
/// table and indexes every event again, time included.
#[test]
fn a_journal_with_the_untimed_table_is_rebuilt_at_a_writable_open() {
    let (_dir, path) = older_journal();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE event_index (
                run_id TEXT NOT NULL, seq INTEGER NOT NULL,
                type TEXT, attempt_id TEXT, member TEXT,
                PRIMARY KEY (run_id, seq)
            ) WITHOUT ROWID;
            CREATE INDEX event_index_stream ON event_index (run_id, attempt_id, member, seq);
            CREATE INDEX event_index_trail ON event_index (run_id, seq)
                WHERE type <> 'effect/checkpointed';
            CREATE TRIGGER event_index_of_an_event AFTER INSERT ON events BEGIN
                INSERT OR IGNORE INTO event_index (run_id, seq) VALUES (NEW.run_id, NEW.seq);
            END;
            INSERT INTO event_index (run_id, seq) SELECT run_id, seq FROM events;
            INSERT INTO meta (key, value) VALUES ('event_index_backfill', 'finished');",
        )
        .unwrap();
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!index_intact(&reader.conn).unwrap());
    assert!(!reader.reads_index().unwrap());
    assert_eq!(reader.load("a").unwrap().len(), 2);
    let store = Store::open(&path).unwrap();
    assert!(timed(&store.conn).unwrap());
    assert!(backfilled(&store));
    assert!(store.reads_index().unwrap());
    for run_id in ["a", "b", "c"] {
        let opened = row(1, "run/started", None, None);
        let started = row(2, "effect/started", Some(run_id), None);
        assert_eq!(rows(&store, run_id), [opened, started], "{run_id}");
        assert_eq!(parted_at(&store, run_id), None, "{run_id}");
    }
}

/// A backfill a peer's lock cuts short writes the runs it reached, leaves
/// the journal read as unindexed, and the next writable open finishes it.
#[test]
fn a_backfill_cut_short_by_a_peer_s_lock_resumes_where_it_stopped() {
    let (_dir, path) = older_journal();
    migrate_index(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let lock = write_lock_on(&path);
    let mut store = Store::open(&path).unwrap();
    lock.execute_batch("ROLLBACK").unwrap();
    assert!(!backfilled(&store));
    assert!(rows(&store, "a").is_empty());
    let peer = cut_short(&mut store, &path, |store, stage| {
        store.index_behind_with(stage)
    });
    let indexed: Vec<usize> = ["a", "b", "c"]
        .iter()
        .map(|run_id| rows(&store, run_id).len())
        .collect();
    assert_eq!(indexed, [2, 0, 0]);
    assert!(!backfilled(&store));
    assert!(!store.reads_index().unwrap());
    drop(store);
    peer.execute_batch("ROLLBACK").unwrap();
    let store = Store::open(&path).unwrap();
    assert!(backfilled(&store));
    for run_id in ["a", "b", "c"] {
        assert_eq!(parted_at(&store, run_id), None, "{run_id}");
    }
}

/// A run `r` of one event, indexed, and then the index's `object` (an
/// index or a trigger, by its kind and name) dropped.
fn one_run_without(object: &str) -> (tempfile::TempDir, Store) {
    let (dir, store) = store();
    run_of(
        &store,
        "r",
        vec![event(EventType::RunStarted, json!({"feature": "f"}))],
    );
    store.conn.execute_batch(&format!("DROP {object}")).unwrap();
    (dir, store)
}

/// `store`'s journal opened again read-write, which repairs its index, and
/// read by it.
fn reopened(store: Store) -> Store {
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(store.reads_index().unwrap());
    store
}

/// A journal that lost its trigger is repaired and walked again, so the
/// events inserted while it was gone are indexed too.
#[test]
fn a_journal_that_lost_its_trigger_is_walked_again_once_repaired() {
    let (_dir, store) = one_run_without("TRIGGER event_index_of_an_event");
    planted(
        &store,
        "r",
        event(EventType::PhaseEntered, json!({"phase": "p"})),
    );
    assert_eq!(parted_at(&store, "r"), Some((2, IndexFault::Missing)));
    let store = reopened(store);
    assert_eq!(parted_at(&store, "r"), None);
}

/// A journal whose table holds its time but lost an index is not read by
/// its index, and a writable open restores it.
#[test]
fn a_journal_that_lost_an_index_is_read_as_before_until_restored() {
    let (_dir, store) = one_run_without("INDEX event_index_trail");
    assert!(timed(&store.conn).unwrap());
    assert!(!store.reads_index().unwrap());
    let store = reopened(store);
    assert_eq!(store.index_trail("r").unwrap(), [1]);
}

/// A run of three events, each with an attempt and a member to edit.
fn three(store: &Store) {
    let step = |seq: u64| {
        event(
            EventType::EffectCheckpointed,
            json!({"attempt_id": "a", "checkpoint": {"member": format!("m{seq}")}}),
        )
    };
    run_of(store, "r", (1..=3).map(step).collect());
}

/// Each edit of a run's rows is named at the first seq where they part
/// from its events, and how.
#[test]
fn a_run_s_rows_part_from_its_events_at_the_first_seq_and_by_the_first_column() {
    let cases = [
        (
            "DELETE FROM event_index WHERE seq = 2",
            2,
            IndexFault::Missing,
        ),
        (
            "DELETE FROM event_index WHERE seq = 3",
            3,
            IndexFault::Missing,
        ),
        (
            "INSERT INTO event_index (run_id, seq) VALUES ('r', 0)",
            0,
            IndexFault::Stray,
        ),
        (
            "INSERT INTO event_index (run_id, seq) VALUES ('r', 4)",
            4,
            IndexFault::Stray,
        ),
        (
            "UPDATE event_index SET type = 'run/started' WHERE seq = 2",
            2,
            IndexFault::Column("type"),
        ),
        (
            "UPDATE event_index SET attempt_id = NULL WHERE seq = 2",
            2,
            IndexFault::Column("attempt_id"),
        ),
        (
            "UPDATE event_index SET member = 'm1' WHERE seq = 2",
            2,
            IndexFault::Column("member"),
        ),
        (
            "UPDATE event_index SET recorded_at = '2026-01-01T00:00:01Z' WHERE seq = 2",
            2,
            IndexFault::Column("recorded_at"),
        ),
    ];
    for (edit, seq, fault) in cases {
        let (_dir, store) = store();
        three(&store);
        assert_eq!(parted_at(&store, "r"), None, "{edit}");
        store.conn.execute_batch(edit).unwrap();
        assert_eq!(parted_at(&store, "r"), Some((seq, fault)), "{edit}");
    }
}

/// Rows of a run the journal does not hold are named by the sweep, each
/// run at its first such seq, after the runs it walks: no event stands
/// behind them, and no walk of `runs` reaches them.
#[test]
fn rows_of_a_run_the_journal_does_not_hold_are_named_as_strays() {
    let (_dir, store) = store();
    three(&store);
    store
        .conn
        .execute_batch(
            "INSERT INTO event_index (run_id, seq) VALUES ('ghost', 7), ('ghost', 3), ('a', 1);
             UPDATE event_index SET member = NULL WHERE run_id = 'r' AND seq = 1;",
        )
        .unwrap();
    let named: Vec<String> = store
        .summary_disagreements()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        named,
        [
            "run 'r': the event index row of its event 1 is not the event's in its member",
            "run 'a': the event index holds a row at 1, where the run holds no event",
            "run 'ghost': the event index holds a row at 3, where the run holds no event",
        ]
    );
}

/// Every fixture journal's events, planted as an older binary plants
/// them, are indexed by the trigger as brokkr-core projects them, each
/// compared with the events read from the journal's own lines, so a chain
/// no load admits is held to it too.
#[test]
fn every_fixture_s_events_are_indexed_as_brokkr_core_projects_them() {
    let (_dir, store) = store();
    let found: Vec<_> = crate::test_support::fixture_events()
        .into_iter()
        .map(|(_, _, events)| {
            let run_id = events[0].run_id.clone();
            crate::test_support::plant_older_writer(&store, &run_id, &events);
            let parted = first_unindexed(&store.conn, &run_id, &events).unwrap();
            let indexed = rows(&store, &run_id).len();
            (run_id, indexed, parted)
        })
        .collect();
    let run = |run: &str, rows| (run.to_string(), rows, None);
    assert_eq!(
        found,
        [
            run("conclude-already-done-hand-built", 8),
            run("conclude-broken-chain-hand-built", 11),
            run("conclude-parked-hand-built", 6),
            run("conclude-stopped-mid-effect-hand-built", 11),
            run("reforging-the-road-back-hand-built", 80),
            run("tui-graph-the-selection-box-gets-80f98deb", 105),
        ]
    );
}

/// A fresh journal's open finds no event to index and records the
/// backfill finished, so its index is read from its first run.
#[test]
fn a_fresh_journal_is_read_by_its_index() {
    let (_dir, store) = store();
    assert!(backfilled(&store));
    assert!(store.reads_index().unwrap());
}

/// A run's trail is every event but its checkpoints; a seat's stream is
/// its checkpoints a page at a time, newest first, resumed below a seq;
/// the envelopes are read by key in the order asked.
#[test]
fn the_trail_skips_checkpoints_and_a_stream_pages_newest_first() {
    let (_dir, store) = store();
    let checkpoint = |member: Option<&str>| {
        event(
            EventType::EffectCheckpointed,
            json!({"attempt_id": "a", "checkpoint": {"member": member}}),
        )
    };
    let mut events = vec![
        event(EventType::RunStarted, json!({"feature": "f"})),
        event(EventType::EffectStarted, json!({"attempt_id": "a"})),
    ];
    events.extend((0..120).map(|at| checkpoint((at % 4 == 0).then_some("m"))));
    events.push(event(
        EventType::EffectSucceeded,
        json!({"attempt_id": "a", "result": {}}),
    ));
    run_of(&store, "r", events);
    assert_eq!(store.index_trail("r").unwrap(), [1, 2, 123]);
    let own: Vec<u64> = (3..=122).filter(|seq| (seq - 3) % 4 != 0).rev().collect();
    let first = store.index_checkpoints("r", "a", None, None).unwrap();
    assert_eq!(first, own[..CHECKPOINT_PAGE]);
    let resumed = Some(*first.last().unwrap());
    let second = store.index_checkpoints("r", "a", None, resumed).unwrap();
    assert_eq!(second, own[CHECKPOINT_PAGE..]);
    assert_eq!(
        store
            .index_checkpoints("r", "a", None, Some(u64::MAX))
            .unwrap(),
        first
    );
    let member: Vec<u64> = (3..=122).filter(|seq| (seq - 3) % 4 == 0).rev().collect();
    assert_eq!(
        store.index_checkpoints("r", "a", Some("m"), None).unwrap(),
        member
    );
    assert_eq!(
        store.index_checkpoints("r", "b", None, None).unwrap(),
        Vec::<u64>::new()
    );
    let read = store.envelopes_at("r", &[123, 1, 2]).unwrap();
    let seqs: Vec<u64> = read.iter().map(|envelope| envelope.seq).collect();
    assert_eq!(seqs, [123, 1, 2]);
    assert_eq!(read[0].event_type, EventType::EffectSucceeded);
    let missing = store.envelopes_at("r", &[1, 124]).unwrap_err();
    assert!(
        matches!(
            missing,
            StoreError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        ),
        "{missing:?}"
    );
}

/// `sql`'s plan, each step's detail in order.
fn plan(store: &Store, sql: &str, bound: impl Params) -> Vec<String> {
    let mut statement = store
        .conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .unwrap();
    let steps = statement.query_map(bound, |row| row.get(3)).unwrap();
    steps.collect::<Result<_, _>>().unwrap()
}

const STREAM_PLAN: &str = "SEARCH event_index USING INDEX event_index_stream \
     (run_id=? AND attempt_id=? AND member=? AND seq<?)";

/// Each keyed read searches its index, with no scan and no temporary
/// B-tree: the trail its partial index, a seat's stream its own, the
/// envelopes and the comparison their keys.
#[test]
fn each_read_searches_its_index() {
    let (_dir, store) = store();
    let none: Option<&str> = None;
    assert_eq!(
        [
            plan(&store, &TRAIL, params!["r"]),
            plan(&store, &STREAM, params!["r", "a", none, 9]),
            plan(&store, &STREAM, params!["r", "a", "m", 9]),
            plan(&store, ENVELOPE, params!["r", 1]),
            plan(&store, ROWS, params!["r"]),
        ],
        [
            ["SEARCH event_index USING INDEX event_index_trail (run_id=?)"],
            [STREAM_PLAN],
            [STREAM_PLAN],
            ["SEARCH events USING INDEX sqlite_autoindex_events_1 (run_id=? AND seq=?)"],
            ["SEARCH event_index USING PRIMARY KEY (run_id=?)"],
        ]
    );
}
