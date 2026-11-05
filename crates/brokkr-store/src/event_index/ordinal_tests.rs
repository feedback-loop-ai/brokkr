//! The ordinals a writer keeps (#994): a run's checkpoints carry 1..n per
//! stream in seq order, a member's result-only record carries none (H1),
//! two sequential effects on one attempt continue one stream but count
//! apart by their rows' bounds, an import and an older writer's append
//! keep them under the trigger, and a journal from before the column is
//! altered and walked once at a writer's open.

use brokkr_core::envelope::{EventEnvelope, EventType};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value as Json};

use super::ordinal::{add_column, BACKFILLED, COLUMN};
use super::*;
use crate::test_support::{append, checkpoint_of, opened};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::store;

/// The `stream_ordinal` SQLite holds at `seq`, or none where the row is
/// not there.
fn ordinal(store: &Store, run_id: &str, seq: u64) -> Option<i64> {
    store
        .conn
        .query_row(
            &format!("SELECT {COLUMN} FROM event_index WHERE run_id = ?1 AND seq = ?2"),
            params![run_id, seq as i64],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .unwrap()
        .flatten()
}

/// `n` of `fx`'s own turns at attempt `a`, in order.
fn own_turns(n: u64) -> Vec<(EventType, Json)> {
    (1..=n)
        .map(|turn| {
            (
                EventType::EffectCheckpointed,
                checkpoint_of("fx", "a", None, turn),
            )
        })
        .collect()
}

/// A checkpointed event naming its member by its result alone, as a
/// member's result-only record does.
fn resulted(attempt: &str, member: &str) -> Json {
    json!({"effect_id": "fx", "attempt_id": attempt, "result": {"member": member}})
}

/// The next event of `run_id`, sealed onto its head, and planted as an
/// older writer or an import plants one: past the store's writer, so the
/// trigger alone indexes and numbers it.
fn planted(store: &Store, run_id: &str, kind: EventType, payload: Json) -> EventEnvelope {
    let (seq, previous) = store.head_hash(run_id).unwrap();
    let envelope = EnvelopeBuilder::new(kind, payload)
        .run(run_id)
        .seq(seq + 1)
        .event_id(format!("{run_id}-{}", seq + 1))
        .previous(previous)
        .sealed();
    crate::test_support::plant_envelope(&store.conn, &envelope).unwrap();
    envelope
}

/// A run `r` asked and started at attempt `a`, then `events`.
fn begun(store: &mut Store, events: Vec<(EventType, Json)>) -> Vec<EventEnvelope> {
    opened(store, "r", "implement", &[]);
    let mut made = vec![append(
        store,
        "r",
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker"}),
    )];
    made.push(append(
        store,
        "r",
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a"}),
    ));
    for (kind, payload) in events {
        made.push(append(store, "r", kind, payload));
    }
    made
}

/// `events` with one checkpoint of `fx`'s member `m` after them.
fn with_member(mut events: Vec<(EventType, Json)>, turn: u64) -> Vec<(EventType, Json)> {
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", Some("m"), turn),
    ));
    events
}

/// The seqs of `events`.
fn seqs(events: &[EventEnvelope]) -> Vec<u64> {
    events.iter().map(|event| event.seq).collect()
}

#[test]
fn a_run_s_checkpoints_carry_their_stream_ordinals() {
    let (_dir, mut store) = store();
    let made = begun(&mut store, with_member(own_turns(2), 1));
    let seqs = seqs(&made);
    assert_eq!(
        [ordinal(&store, "r", seqs[2]), ordinal(&store, "r", seqs[3])],
        [Some(1), Some(2)],
        "the seat's own checkpoints are 1 and 2 in seq order"
    );
    assert_eq!(
        ordinal(&store, "r", seqs[4]),
        Some(1),
        "the member's own stream begins at 1"
    );
    // Every row that is not a checkpoint of a stream carries none.
    for seq in 1..=seqs[2] {
        if seqs.contains(&seq) {
            continue;
        }
        assert_eq!(ordinal(&store, "r", seq), None, "row {seq} carries none");
    }
}

/// H1: a member's result-only record is an `effect/checkpointed` row whose
/// `event_index.member` comes from its result, where its checkpoint names
/// none. The ordinal never counts it, and a stream whose bound is such a
/// row cannot stand for the participant.
#[test]
fn a_member_s_result_only_record_carries_none() {
    let (_dir, mut store) = store();
    let made = begun(
        &mut store,
        vec![
            (
                EventType::EffectCheckpointed,
                checkpoint_of("fx", "a", Some("m"), 1),
            ),
            (EventType::EffectCheckpointed, resulted("a", "m")),
            (
                EventType::EffectCheckpointed,
                checkpoint_of("fx", "a", Some("m"), 2),
            ),
        ],
    );
    let seqs = seqs(&made);
    let (first, result, second) = (seqs[2], seqs[3], seqs[4]);
    assert_eq!(
        ordinal(&store, "r", result),
        None,
        "the result-only record carries none"
    );
    assert_eq!(
        [ordinal(&store, "r", first), ordinal(&store, "r", second)],
        [Some(1), Some(2)],
        "the checkpoints either side are 1 and 2"
    );
}

/// M1: sequential effects reusing one attempt continue the one stream's
/// ordinals, while each effect's row bounds stay its own.
#[test]
fn sequential_effects_on_one_attempt_continue_their_stream() {
    let (_dir, mut store) = store();
    let made = begun(&mut store, own_turns(2));
    let first = made[2].seq;
    // A second effect asked under the same attempt, with two more.
    append(
        &mut store,
        "r",
        EventType::EffectRequested,
        json!({"effect_id": "gx", "seat": "worker"}),
    );
    append(
        &mut store,
        "r",
        EventType::EffectStarted,
        json!({"effect_id": "gx", "attempt_id": "a"}),
    );
    let third = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint_of("gx", "a", None, 1),
    )
    .seq;
    let fourth = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint_of("gx", "a", None, 2),
    )
    .seq;
    assert_eq!(
        [
            ordinal(&store, "r", first),
            ordinal(&store, "r", first + 1),
            ordinal(&store, "r", third),
            ordinal(&store, "r", fourth),
        ],
        [Some(1), Some(2), Some(3), Some(4)],
        "one stream continues across the two effects"
    );
}

/// An older writer's append, which inserts the event past the store's
/// writer, and an import, which adopts a verified chain, both keep the
/// ordinals: the trigger lives in the journal, not in either writer.
#[test]
fn an_older_writer_and_an_import_keep_the_ordinals() {
    let (_dir, mut source) = store();
    let made = begun(&mut source, own_turns(2));
    let older = planted(
        &source,
        "r",
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", None, 3),
    );
    assert_eq!(
        ordinal(&source, "r", older.seq),
        Some(3),
        "an older writer's append is numbered"
    );
    assert_eq!(
        [
            ordinal(&source, "r", made[2].seq),
            ordinal(&source, "r", made[3].seq)
        ],
        [Some(1), Some(2)]
    );
    // The planted append leaves the summary behind, as an older writer's
    // does; the test's store drop is told not to hold the run to it.
    crate::test_support::lags(&mut source, &["r"]);
    // The import: a verified chain adopted into a fresh journal keeps the
    // same row ordinals, inserted by the trigger alone.
    let ndjson = source.export_ndjson("r").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut dest = Store::open(&dir.path().join("dest.db")).unwrap();
    dest.import_run(
        &ndjson,
        &json!({}),
        std::path::Path::new("/exports/r.ndjson"),
    )
    .unwrap();
    assert_eq!(
        [
            ordinal(&dest, "r", made[2].seq),
            ordinal(&dest, "r", made[3].seq),
            ordinal(&dest, "r", older.seq),
        ],
        [Some(1), Some(2), Some(3)]
    );
}

/// Drop the ordinals' objects and column, as a journal from before them
/// carries none, one statement at a time: the shape a writable open must
/// alter in place rather than drop.
fn unordinal(conn: &rusqlite::Connection) {
    for sql in [
        "DROP TRIGGER event_index_ordinal",
        "DROP TRIGGER event_index_of_an_event",
        "DROP INDEX event_index_ordinal_stale",
        "DROP INDEX event_index_ordinal_written",
        "ALTER TABLE event_index DROP COLUMN stream_ordinal",
        "DELETE FROM meta WHERE key = 'event_index_ordinal_backfill'",
    ] {
        conn.execute(sql, []).unwrap();
    }
}

/// The rows a journal from before the column keeps, numbered at a
/// writer's open: the column is added in place, and the walk fills 1..n
/// per stream without dropping a row. A read-only reader before it takes
/// the kept way rather than read a NULL as zero.
#[test]
fn a_journal_from_before_the_column_is_altered_and_numbered() {
    let (_dir, mut store) = store();
    let made = begun(&mut store, with_member(own_turns(2), 1));
    let path = store.path().to_path_buf();
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    unordinal(&conn);
    drop(conn);
    let reader = Store::open_read_only(&path).unwrap();
    let rows: i64 = reader
        .conn
        .query_row("SELECT count(*) FROM event_index", [], |row| row.get(0))
        .unwrap();
    assert!(!index_intact(&reader.conn).unwrap(), "the column is gone");
    assert!(!reader.reads_index().unwrap(), "a reader cannot number it");
    assert_eq!(
        reader
            .seat_count("r", &reader.open_run("r").unwrap(), ("fx", None))
            .unwrap(),
        None,
        "a reader without the column takes the kept way"
    );
    drop(reader);
    let store = Store::open(&path).unwrap();
    assert!(index_intact(&store.conn).unwrap());
    assert!(finished(&store.conn, BACKFILLED).unwrap());
    let after: i64 = store
        .conn
        .query_row("SELECT count(*) FROM event_index", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after, rows, "the altered table keeps its rows");
    let seqs = seqs(&made);
    assert_eq!(
        [
            ordinal(&store, "r", seqs[2]),
            ordinal(&store, "r", seqs[3]),
            ordinal(&store, "r", seqs[4]),
        ],
        [Some(1), Some(2), Some(1)],
        "the backfill numbers each stream in seq order"
    );
}

/// A NULL an older writer or a lost trigger left inside a stream is
/// repaired by the next writable open, under the column's own meta key.
#[test]
fn a_null_ordinal_is_repaired_at_the_next_writable_open() {
    let (_dir, mut store) = store();
    let made = begun(&mut store, own_turns(2));
    let path = store.path().to_path_buf();
    let seq = made[2].seq;
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    // The trigger is gone and the first row is unnumbered: the shape a
    // writer from before the column leaves, with the column already here.
    conn.execute_batch(&format!(
        "DROP TRIGGER event_index_ordinal;
         UPDATE event_index SET stream_ordinal = NULL WHERE run_id = 'r' AND seq = {seq};"
    ))
    .unwrap();
    drop(conn);
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!index_intact(&reader.conn).unwrap());
    assert_eq!(
        reader
            .seat_count("r", &reader.open_run("r").unwrap(), ("fx", None))
            .unwrap(),
        None
    );
    drop(reader);
    let store = Store::open(&path).unwrap();
    assert!(index_intact(&store.conn).unwrap());
    assert_eq!(ordinal(&store, "r", seq), Some(1), "the walk repairs it");
}

/// `add_column` is the one statement an altered table takes, and a table
/// that already holds the column asks and writes nothing.
#[test]
fn the_column_is_added_once() {
    let (_dir, store) = store();
    add_column(&store.conn).unwrap();
    assert!(crate::schema::holds(&store.conn, "event_index", [COLUMN]).unwrap());
    add_column(&store.conn).unwrap();
}
