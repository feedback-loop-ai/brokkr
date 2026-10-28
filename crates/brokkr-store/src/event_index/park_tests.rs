//! The parks' dated facts (decision 0089 ruling 7): each append keeps the
//! phase, start, answer, end and span class of every park its fold states,
//! ruling 4's comparison holds each column to the run read whole, a
//! writer from before them leaves its park off its head and the next
//! writable open refolds it, and a table from before them is made anew.

use super::*;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::stamp::parse_millis;
use brokkr_core::summary::{parks, Bounded, NAME_BYTES};
use rusqlite::params;
use serde_json::{json, Value as Json};

use super::park_walk::PARK_STALE;
use super::{index_intact, parked, IndexFault, PARK_COLUMNS};
use crate::summary::Compared;
use crate::test_support::{
    append, every_run_reads_as_its_fold, fixture_events, lags, plant_envelope, plant_older_writer,
    text,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::store;
use crate::{Disagreement, Store, StoreError};

/// A park's dated facts, by their columns' order: `phase`, `at_ms`,
/// `answered_ms`, `ended_ms`, `span_class`, and the head they cover.
fn kept(store: &Store, run_id: &str, seq: u64) -> Vec<Value> {
    let select = format!(
        "SELECT {} FROM event_index WHERE run_id = ?1 AND seq = ?2",
        PARK_COLUMNS.join(", ")
    );
    store
        .conn
        .query_row(&select, params![run_id, seq as i64], |row| {
            (0..PARK_COLUMNS.len()).map(|at| row.get(at)).collect()
        })
        .unwrap()
}

fn at(event: &EventEnvelope) -> i64 {
    parse_millis(&event.recorded_at).unwrap()
}

/// A run created and carried by this store, started under a feature,
/// with `phase` entered, and then `steps`.
fn opened(store: &mut Store, run_id: &str, phase: &str, steps: &[(EventType, Json)]) {
    store
        .create_run(run_id, "a feature", "self", &json!({}))
        .unwrap();
    append(
        store,
        run_id,
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    );
    append(
        store,
        run_id,
        EventType::PhaseEntered,
        json!({"phase": phase}),
    );
    for (event_type, payload) in steps {
        append(store, run_id, *event_type, payload.clone());
    }
}

/// A run with `phase` entered and one park opened by this store, which
/// carries it, and that park's row.
fn parked_row(store: &mut Store) -> Vec<Value> {
    opened(
        store,
        "r",
        "implement",
        &[(EventType::RunParked, json!({"reason": "waiting"}))],
    );
    kept(store, "r", 3)
}

/// The class of a positive reach, or 0, as the projection reads it.
fn class(reach: i64) -> Value {
    if reach <= 0 {
        return Value::Integer(0);
    }
    Value::Integer(i64::from(u64::BITS - (reach as u64).leading_zeros()))
}

/// The next event of `run_id`, a phase entered at nine seconds past the
/// journal's epoch, planted as a peer writes it.
fn planted_move(store: &Store, run_id: &str, event_id: &str) {
    let (seq, previous) = store.head_hash(run_id).unwrap();
    let moved = EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "review"}))
        .run(run_id)
        .seq(seq + 1)
        .event_id(event_id)
        .at("2026-01-01T00:00:09Z")
        .previous(previous)
        .sealed();
    plant_envelope(&store.conn, &moved).unwrap();
}

/// `run_id`'s summary row and seat rows written at its current head, over
/// events already planted: what a writer from before the parks leaves, so
/// only the parks' own backfill can refold them.
fn kept_before_parks(store: &Store, run_id: &str) {
    let whole = store.load(run_id).unwrap();
    let account = Account::whole(run_id, &whole);
    let listed = crate::summary::Listed::of(&store.conn, run_id).unwrap();
    crate::summary::write(
        &store.conn,
        run_id,
        &brokkr_core::summary::Summary::of(&account, listed.listing()),
    )
    .unwrap();
    crate::seat_summary::write(&store.conn, &account, None).unwrap();
}

/// `events` planted and the projections a writer from before the parks
/// keeps written at their head: its summary row and its seat rows, so only
/// the parks' own backfill can refold them.
fn plant_before_parks(store: &Store, run_id: &str, events: &[EventEnvelope]) {
    for event in events {
        plant_envelope(&store.conn, event).unwrap();
    }
    kept_before_parks(store, run_id);
}

/// `store`'s journal opened again read-write, which repairs its parks.
fn refolded(store: Store) -> Store {
    let path = store.path().to_path_buf();
    drop(store);
    Store::open(&path).unwrap()
}

/// A run parked, answered and then moved on by this store: its park is
/// closed at the move.
fn parked_and_moved(store: &mut Store) {
    opened(
        store,
        "r",
        "implement",
        &[
            (EventType::RunParked, json!({"reason": "waiting"})),
            (EventType::OperatorCommanded, json!({"command": "go"})),
            (EventType::PhaseEntered, json!({"phase": "review"})),
        ],
    );
}

/// What the doctor's sweep names, as words: empty where every run reads as
/// its fold.
fn disagreements(store: &Store) -> Vec<String> {
    store
        .summary_disagreements()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// `run_id` is a lag the doctor passes, never a disagreement, and the next
/// writable open refolds it.
fn refolded_lag(mut store: Store, run_id: &str) -> Store {
    assert_eq!(disagreements(&store), Vec::<String>::new());
    assert!(matches!(
        store.compared(run_id, || {}).unwrap(),
        Compared::Behind
    ));
    lags(&mut store, &[run_id]);
    refolded(store)
}

/// `store`'s journal opened again with its park row at the head it now
/// has: no row is left for the sweep, the park reads `held`, and every
/// run reads as its fold.
fn refolded_at_head(store: Store, held: &[Value]) {
    let store = refolded(store);
    assert_eq!(store.park_after().unwrap(), None);
    assert_eq!(kept(&store, "r", 3), held);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

#[test]
fn each_append_keeps_the_parks_its_fold_states() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    let park = append(
        &mut store,
        "r",
        EventType::RunParked,
        json!({"reason": "waiting"}),
    );
    let parked_at = at(&park);
    assert_eq!(
        kept(&store, "r", park.seq),
        [
            text("implement"),
            Value::Integer(parked_at),
            Value::Null,
            Value::Null,
            Value::Integer(64),
            Value::Integer(park.seq as i64),
        ]
    );
    // A command answers the park and the run stays parked.
    let commanded = append(
        &mut store,
        "r",
        EventType::OperatorCommanded,
        json!({"command": "go"}),
    );
    assert_eq!(
        kept(&store, "r", park.seq),
        [
            text("implement"),
            Value::Integer(parked_at),
            Value::Integer(at(&commanded)),
            Value::Null,
            Value::Integer(64),
            Value::Integer(commanded.seq as i64),
        ]
    );
    // The run's next move ends the park, at that move's own time.
    let moved = append(
        &mut store,
        "r",
        EventType::PhaseEntered,
        json!({"phase": "review"}),
    );
    let reach = at(&commanded).max(at(&moved)) - parked_at;
    assert_eq!(
        kept(&store, "r", park.seq),
        [
            text("implement"),
            Value::Integer(parked_at),
            Value::Integer(at(&commanded)),
            Value::Integer(at(&moved)),
            class(reach),
            Value::Integer(moved.seq as i64),
        ]
    );
    // A second park opens with the phase the run now has.
    let again = append(
        &mut store,
        "r",
        EventType::RunParked,
        json!({"reason": "again"}),
    );
    assert_eq!(
        kept(&store, "r", again.seq),
        [
            text("review"),
            Value::Integer(at(&again)),
            Value::Null,
            Value::Null,
            Value::Integer(64),
            Value::Integer(again.seq as i64),
        ]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Ruling 4's comparison holds every one of a park's columns to the run
/// read whole: an edit of any one is named by that column.
#[test]
fn each_park_column_that_is_not_its_fold_s_is_named() {
    let (_dir, mut store) = store();
    parked_and_moved(&mut store);
    let held = kept(&store, "r", 3);
    let mut named = Vec::new();
    for (at, column) in PARK_COLUMNS.iter().enumerate() {
        let edit = format!("UPDATE event_index SET {column} = ?1 WHERE run_id = 'r' AND seq = 3");
        let wrong = match &held[at] {
            Value::Text(_) => text("edited"),
            Value::Integer(1) => Value::Integer(2),
            _ => Value::Integer(1),
        };
        store.conn.execute(&edit, [&wrong]).unwrap();
        let read = store.compared("r", || {}).unwrap();
        if *column == "facts_seq" {
            // A stamp off the head is a lag, not an edit: the next writable
            // open refolds the park, so the comparison names it as behind.
            assert!(matches!(read, Compared::Behind), "{column}: {read:?}");
        } else {
            match read {
                Compared::Disagrees(Disagreement::Indexed {
                    fault: IndexFault::Column(found),
                    ..
                }) => assert_eq!(found, *column),
                other => panic!("{column}: {other:?}"),
            }
        }
        named.push(column.to_string());
        store.conn.execute(&edit, [&held[at]]).unwrap();
    }
    assert_eq!(
        named,
        PARK_COLUMNS
            .iter()
            .map(|column| column.to_string())
            .collect::<Vec<_>>()
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET phase = 'edited' WHERE run_id = 'r' AND seq = 3",
            [],
        )
        .unwrap();
    assert_eq!(
        store.summary_disagreements().unwrap()[0].to_string(),
        "run 'r': the event index row of its event 3 is not the event's in its phase"
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET phase = ?1 WHERE run_id = 'r' AND seq = 3",
            [&held[0]],
        )
        .unwrap();
}

/// A phase past the name bound is written cut and reads as its fold's:
/// the writer's row and the comparison's named row take the cut from one
/// derivation, so a whole phase can stand on neither side of the check.
#[test]
fn a_phase_past_its_bound_is_cut_where_it_is_stored() {
    let (_dir, mut store) = store();
    let phase = "p".repeat(NAME_BYTES + 20);
    opened(
        &mut store,
        "r",
        &phase,
        &[(EventType::RunParked, json!({"reason": "waiting"}))],
    );
    assert_eq!(kept(&store, "r", 3)[0], text(&"p".repeat(NAME_BYTES)));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    assert_eq!(disagreements(&store), Vec::<String>::new());
}

/// A writer from before the parks writes the next event and no park's
/// dated facts, and the trigger sends the open park off its run's head, so
/// no reader trusts it: the next writable open refolds the run and its
/// park.
#[test]
fn a_writer_from_before_the_parks_leaves_them_off_their_head() {
    let (_dir, mut store) = store();
    let parked_at = parked_row(&mut store)[1].clone();
    assert_eq!(kept(&store, "r", 3)[5], Value::Integer(3));
    planted_move(&store, "r", "older");
    assert_eq!(kept(&store, "r", 3)[5], Value::Null);
    // Its facts are not its head's, which is a lag the next writable open
    // refolds, never a disagreement the doctor names.
    let store = refolded_lag(store, "r");
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    let ended = 1_767_225_609_000;
    let Value::Integer(parked_at) = parked_at else {
        panic!("a park holds its start");
    };
    let reach = ended - parked_at;
    assert_eq!(
        kept(&store, "r", 3),
        [
            text("implement"),
            Value::Integer(parked_at),
            Value::Integer(ended),
            Value::Integer(ended),
            class(reach),
            Value::Integer(4),
        ]
    );
}

/// A park's facts that nothing wrote are refolded at the next writable
/// open even when the run's summary stands at its head, so the parks'
/// backfill is the journal's own walk and not the summary's.
#[test]
fn an_unwritten_park_is_refolded_at_the_next_open() {
    let (_dir, mut store) = store();
    let held = parked_row(&mut store);
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE type = 'run/parked'",
            [],
        )
        .unwrap();
    assert!(store.park_after().unwrap().is_some());
    lags(&mut store, &["r"]);
    refolded_at_head(store, &held);
}

/// A park row whose stamp is non-NULL but names no head its run has is a
/// lag too, not only the NULL a writer from before the parks leaves: the
/// next writable open refolds it, so no reader trusts the facts behind a
/// stamp the run has left.
#[test]
fn a_park_stamp_that_is_not_its_head_is_refolded_at_the_next_open() {
    let (_dir, mut store) = store();
    let held = parked_row(&mut store);
    assert_eq!(held[5], Value::Integer(3));
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = 1 WHERE run_id = 'r' AND seq = 3",
            [],
        )
        .unwrap();
    // An intact table leaves the row for the next writable open, which
    // refolds it at the head the run now has.
    assert_eq!(store.park_after().unwrap().as_deref(), Some("r"));
    assert_eq!(disagreements(&store), Vec::<String>::new());
    assert!(matches!(
        store.compared("r", || {}).unwrap(),
        Compared::Behind
    ));
    lags(&mut store, &["r"]);
    refolded_at_head(store, &held);
}

/// The sweep's two reads name different runs, and the least id is taken:
/// an off-head park in `a` and an unwritten park in `b` are both refolded,
/// in one walk, each at its own head.
#[test]
fn the_sweep_takes_the_least_run_of_its_two_reads() {
    let (_dir, mut store) = store();
    for run_id in ["a", "b"] {
        opened(
            &mut store,
            run_id,
            "implement",
            &[(EventType::RunParked, json!({"reason": "waiting"}))],
        );
    }
    let a = kept(&store, "a", 3);
    let b = kept(&store, "b", 3);
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = 1 WHERE run_id = 'a' AND seq = 3",
            [],
        )
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = 'b' AND seq = 3",
            [],
        )
        .unwrap();
    assert_eq!(store.park_after().unwrap().as_deref(), Some("a"));
    lags(&mut store, &["a", "b"]);
    let mut visited = 0;
    store.park_behind_with(|_| visited += 1).unwrap();
    assert_eq!(visited, 2);
    assert_eq!(kept(&store, "a", 3), a);
    assert_eq!(kept(&store, "b", 3), b);
    assert_eq!(store.park_after().unwrap(), None);
}

/// A journal of `prefix` one-event runs, the first `stale` of them parked
/// with their facts sent behind, then `current` parked runs whose facts
/// stand at head: the last sweep's selection steps, its off-head scan and
/// its stale seeks apart. The prefix runs past `stale` carry no park row,
/// so both journals have the same facts index and the same events.
fn mixed_park_sweep(stale: usize, prefix: usize, current: usize) -> (i64, i64) {
    let (_dir, mut store) = store();
    for index in 0..prefix + current {
        let run_id = format!("r{index:04}");
        let no_park = index < prefix && index >= stale;
        let event = if no_park {
            EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "implement"}))
        } else {
            EnvelopeBuilder::new(EventType::RunParked, json!({"reason": "waiting"}))
        }
        .run(&run_id)
        .seq(1)
        .event_id(format!("{run_id}-1"))
        .at("2026-01-01T00:00:01Z")
        .sealed();
        plant_older_writer(&store, &run_id, std::slice::from_ref(&event));
        if !no_park {
            kept_before_parks(&store, &run_id);
        }
    }
    store.park_behind_with(|_| {}).unwrap();
    for index in 0..stale {
        store
            .conn
            .execute(
                "UPDATE event_index SET facts_seq = NULL
                 WHERE run_id = ?1 AND type = 'run/parked'",
                params![format!("r{index:04}")],
            )
            .unwrap();
    }
    store.park_behind_with(|_| {}).unwrap();
    store.park_steps.get()
}

/// A mixed journal's current suffix is read once, however many stale runs
/// precede it: the off-head scan steps are the same for one stale run as
/// for `prefix`, the stale seeks grow with the runs they visit, and each
/// is a range search rather than a scan. Before the walk, every refold
/// read that suffix again, so 64 stale runs cost 64 scans of it, and the
/// cursor's optional-or predicate made the seek a scan too (decision 0071
/// ruling 9).
#[test]
fn a_mixed_park_sweep_reads_the_current_suffix_once() {
    let (_dir, empty) = store();
    assert_eq!(
        crate::test_support::plan(&empty, &PARK_STALE, [""]).unwrap(),
        ["SEARCH event_index USING INDEX event_index_park_stale (run_id>?)"]
    );
    let one = mixed_park_sweep(1, 64, 64);
    let many = mixed_park_sweep(64, 64, 64);
    assert!(one.0 > 0, "the walk scans the facts index: {one:?}");
    assert_eq!(
        one.0, many.0,
        "the current suffix is read once: {one:?} vs {many:?}"
    );
    assert!(
        many.1 > one.1,
        "the stale seeks grow with the stale runs: {one:?} vs {many:?}"
    );
}

/// Two runs whose park facts nothing wrote are both refolded at the next
/// writable open: the parks' backfill walks past the run it has just
/// written to the next one, and ends when no run is left.
#[test]
fn two_unwritten_park_runs_are_both_refolded_at_the_next_open() {
    let (_dir, mut store) = store();
    let mut held = Vec::new();
    for run_id in ["a", "b"] {
        opened(
            &mut store,
            run_id,
            "implement",
            &[(EventType::RunParked, json!({"reason": "waiting"}))],
        );
        held.push((run_id, kept(&store, run_id, 3)));
    }
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE type = 'run/parked'",
            [],
        )
        .unwrap();
    assert_eq!(store.park_after().unwrap().as_deref(), Some("a"));
    lags(&mut store, &["a", "b"]);
    let store = refolded(store);
    assert_eq!(store.park_after().unwrap(), None);
    for (run_id, row) in held {
        assert_eq!(kept(&store, run_id, 3), row, "run {run_id}");
    }
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A `run/parked` whose stamp does not parse opens no park, so its row
/// keeps no dated facts; the writer still records it processed at the
/// head, so the parks' backfill does not read and refold the run at every
/// later open, and ruling 4's comparison reads the row as its fold's.
#[test]
fn an_ignored_park_is_processed_once_and_reads_as_its_fold() {
    let (_dir, mut store) = store();
    let run_id = "r";
    let stamped = |seq: u64, event_type, payload, at: &str, previous: &str| {
        EnvelopeBuilder::new(event_type, payload)
            .run(run_id)
            .seq(seq)
            .event_id(format!("r-{seq}"))
            .at(at)
            .previous(previous)
            .sealed()
    };
    let started = stamped(
        1,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {}}),
        "2026-01-01T00:00:01Z",
        brokkr_core::canonical::ZERO_HASH,
    );
    let entered = stamped(
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        "2026-01-01T00:00:02Z",
        &started.event_hash,
    );
    let ignored = stamped(
        3,
        EventType::RunParked,
        json!({"reason": "waiting"}),
        "not a stamp",
        &entered.event_hash,
    );
    let moved = stamped(
        4,
        EventType::PhaseEntered,
        json!({"phase": "review"}),
        "2026-01-01T00:00:09Z",
        &ignored.event_hash,
    );
    plant_older_writer(&store, run_id, &[started, entered, ignored, moved]);
    kept_before_parks(&store, run_id);
    // An append carried by the writer records the ignored row processed at
    // the head the append covers, as it does every park row.
    store.carry(Account::whole(run_id, &store.load(run_id).unwrap()));
    let next = append(
        &mut store,
        run_id,
        EventType::PhaseEntered,
        json!({"phase": "review"}),
    );
    let processed_at = Value::Integer(next.seq as i64);
    assert_eq!(
        kept(&store, run_id, 3),
        [
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            processed_at.clone(),
        ]
    );
    // A writer from before the parks leaves the row unwritten; the parks'
    // backfill records it processed at the head, once.
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = ?1 AND seq = 3",
            params![run_id],
        )
        .unwrap();
    assert_eq!(store.park_after().unwrap().as_deref(), Some(run_id));
    let mut visited = 0;
    store.park_behind_with(|_| visited += 1).unwrap();
    assert_eq!(visited, 1);
    assert_eq!(
        kept(&store, run_id, 3),
        [
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            processed_at,
        ]
    );
    assert_eq!(store.park_after().unwrap(), None);
    assert!(matches!(
        store.compared(run_id, || {}).unwrap(),
        Compared::Agrees
    ));
    assert_eq!(disagreements(&store), Vec::<String>::new());
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    // A later open finds the row processed: it reads and refolds no run.
    let mut store = refolded(store);
    let mut visited = 0;
    store.park_behind_with(|_| visited += 1).unwrap();
    assert_eq!(visited, 0);
    assert_eq!(store.park_after().unwrap(), None);
}

/// A table from before the parks is not read by its index and refuses its
/// reader nothing; a writable open makes it anew and refolds its parks.
#[test]
fn an_index_from_before_the_parks_is_made_anew_and_refolded() {
    let (_dir, mut store) = store();
    let held = parked_row(&mut store);
    let path = store.path().to_path_buf();
    drop(store);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("ALTER TABLE event_index DROP COLUMN phase")
        .unwrap();
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!index_intact(&reader.conn).unwrap());
    assert!(!parked(&reader.conn).unwrap());
    assert!(!reader.reads_index().unwrap());
    drop(reader);
    let store = Store::open(&path).unwrap();
    assert!(parked(&store.conn).unwrap());
    assert!(store.reads_index().unwrap());
    assert_eq!(kept(&store, "r", 3), held);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Every fixture journal keeps each park column for column what
/// brokkr-core's projection reads of its run whole (decision 0089 ruling
/// 7): the phase cut where it is stored, the moments as the events give
/// them, the class at the head the facts cover.
#[test]
fn every_fixture_keeps_what_the_parks_read() {
    let (_dir, mut store) = store();
    let mut wrong = Vec::new();
    for (_, _, events) in fixture_events() {
        let run_id = events[0].run_id.clone();
        plant_older_writer(&store, &run_id, &events);
        store.park_behind_with(|_| {}).unwrap();
        let expected = parks(&events);
        let found: Vec<u64> = {
            let mut statement = store
                .conn
                .prepare(
                    "SELECT seq FROM event_index WHERE run_id = ?1 AND type = 'run/parked'
                     ORDER BY seq",
                )
                .unwrap();
            let seqs = statement.query_map([&run_id], |row| row.get(0)).unwrap();
            seqs.map(|seq| seq.unwrap()).collect()
        };
        let want_seqs: Vec<u64> = expected.iter().map(|park| park.seq).collect();
        if found != want_seqs {
            wrong.push(format!("{run_id}: parks {found:?}, read {want_seqs:?}"));
            continue;
        }
        for park in expected {
            let row = kept(&store, &run_id, park.seq);
            let cut = park.phase.as_deref().map_or(Value::Null, |phase| {
                text(&Bounded::within(phase, NAME_BYTES).cut)
            });
            let want = vec![
                cut,
                Value::Integer(park.at),
                park.answered.map_or(Value::Null, Value::Integer),
                park.ended.map_or(Value::Null, Value::Integer),
                Value::Integer(i64::from(park.span_class())),
                Value::Integer(events.last().unwrap().seq as i64),
            ];
            if row != want {
                wrong.push(format!(
                    "{run_id}: park {} {row:?}, read {want:?}",
                    park.seq
                ));
            }
        }
    }
    assert_eq!(wrong, Vec::<String>::new());
}

/// The drop binding a writer's store is held to names a park column too:
/// a row at its head whose park is not its fold's refuses on drop.
#[test]
#[should_panic(
    expected = "a summary row disagrees with its run's fold: [\"r: event 3 indexed Column(\\\"phase\\\")\"]"
)]
fn a_writer_store_dropped_with_an_edited_park_is_named() {
    let (_dir, mut store) = store();
    opened(
        &mut store,
        "r",
        "implement",
        &[(EventType::RunParked, json!({"reason": "waiting"}))],
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET phase = 'edited' WHERE run_id = 'r' AND seq = 3",
            [],
        )
        .unwrap();
}

/// A park's written span class is the bit length of its reach: 0 at a
/// zero or negative reach, c + 1 at the boundary 2^c, c one below it, and
/// 64 while the park is open.
#[test]
fn a_park_s_written_span_class_is_its_reach_s_bit_length() {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    let stamp_at = |ms: i64| {
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
            .unwrap()
            .format(&Rfc3339)
            .unwrap()
    };
    let at = 1_767_225_600_000;
    let mut cases: Vec<(i64, Option<i64>)> = vec![(0, Some(at)), (-5, Some(at - 1))];
    for c in 0..=10 {
        let reach = 1_i64 << c;
        cases.push((reach, Some(at + reach)));
        cases.push((reach - 1, Some(at + reach - 1)));
    }
    cases.push((0, None));
    let (_dir, mut store) = store();
    for (index, (reach, ended)) in cases.into_iter().enumerate() {
        let run_id = format!("r{index}");
        let mut steps = vec![
            (
                EventType::RunStarted,
                json!({"feature": "f", "manifest": {}}),
            ),
            (EventType::PhaseEntered, json!({"phase": "implement"})),
            (EventType::RunParked, json!({"reason": "waiting"})),
        ];
        if let Some(ended) = ended {
            steps.push((EventType::PhaseEntered, json!({"phase": "review"})));
            let _ = ended;
        }
        let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
        let mut events = Vec::new();
        for (step, (event_type, payload)) in steps.into_iter().enumerate() {
            let seq = step as u64 + 1;
            let when = match step {
                0 => at - 2,
                1 => at - 1,
                2 => at,
                _ => ended.unwrap(),
            };
            let event = EnvelopeBuilder::new(event_type, payload)
                .run(&run_id)
                .seq(seq)
                .event_id(format!("{run_id}-{seq}"))
                .at(stamp_at(when))
                .previous(previous.clone())
                .sealed();
            previous.clone_from(&event.event_hash);
            events.push(event);
        }
        plant_older_writer(&store, &run_id, &events);
        store.park_behind_with(|_| {}).unwrap();
        let written = kept(&store, &run_id, 3);
        let want = match ended {
            None => Value::Integer(64),
            Some(_) => class(reach),
        };
        assert_eq!(written[4], want, "reach {reach}: {written:?}");
    }
}

/// A run whose chain does not load is left for a reader to read whole and
/// the doctor to name: the parks' backfill writes nothing of it and walks
/// past it rather than stopping.
#[test]
fn a_run_that_does_not_load_keeps_its_parks_for_the_doctor() {
    let (_dir, mut store) = store();
    let run_id = "broken";
    let first = EnvelopeBuilder::new(
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {}}),
    )
    .run(run_id)
    .seq(1)
    .event_id("broken-1")
    .at("2026-01-01T00:00:01Z")
    .sealed();
    // The link from the second event to the first names no hash of it.
    let second = EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "implement"}))
        .run(run_id)
        .seq(2)
        .event_id("broken-2")
        .at("2026-01-01T00:00:02Z")
        .previous("not the first's hash")
        .sealed();
    let third = EnvelopeBuilder::new(EventType::RunParked, json!({"reason": "waiting"}))
        .run(run_id)
        .seq(3)
        .event_id("broken-3")
        .at("2026-01-01T00:00:03Z")
        .previous(second.event_hash.clone())
        .sealed();
    plant_older_writer(&store, run_id, &[first, second, third]);
    let refused = store.load(run_id).unwrap_err();
    assert!(
        matches!(
            &refused,
            StoreError::Chain(ChainError::BrokenChain {
                seq: 2,
                prev_seq: 1
            })
        ),
        "{refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        "chain: event 2: previous_hash does not match event 1"
    );
    assert_eq!(store.park_after().unwrap().as_deref(), Some(run_id));
    store.park_behind_with(|_| {}).unwrap();
    assert_eq!(store.park_after().unwrap().as_deref(), Some(run_id));
    assert_eq!(kept(&store, run_id, 3), vec![Value::Null; 6]);
}

/// The parks' backfill writes a run only while its head is the one it read:
/// a run a peer moved while it was read is left for the next open.
#[test]
fn a_park_read_at_a_head_that_moved_is_not_written() {
    let (_dir, mut store) = store();
    let _ = parked_row(&mut store);
    let account = Account::whole("r", &store.load("r").unwrap());
    planted_move(&store, "r", "peer");
    lags(&mut store, &["r"]);
    park_once(&mut store.conn, &account).unwrap();
    assert_eq!(kept(&store, "r", 3)[5], Value::Null);
    let store = refolded(store);
    assert_eq!(kept(&store, "r", 3)[5], Value::Integer(4));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A park from a run that named no phase keeps none: the phase is absent,
/// not an empty key, and the row reads as its fold.
#[test]
fn a_park_from_a_phase_the_run_never_named_keeps_no_phase() {
    let (_dir, mut store) = store();
    store
        .create_run("r", "a feature", "self", &json!({}))
        .unwrap();
    append(
        &mut store,
        "r",
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    );
    append(&mut store, "r", EventType::PhaseEntered, json!({}));
    let park = append(
        &mut store,
        "r",
        EventType::RunParked,
        json!({"reason": "waiting"}),
    );
    assert_eq!(
        kept(&store, "r", park.seq),
        [
            Value::Null,
            Value::Integer(at(&park)),
            Value::Null,
            Value::Null,
            Value::Integer(64),
            Value::Integer(park.seq as i64),
        ]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// The trigger that sends a park behind searches the park rows whose facts
/// are written, never the run's every event: its plan names the partial
/// index, and an append to a run with no parks costs the same at every
/// seq, so a journal of checkpoints does not make every append read them.
#[test]
fn sending_a_park_behind_visits_only_the_park_rows() {
    use crate::import::INSERT_EVENT;
    use rusqlite::StatementStatus;

    let (_dir, store) = store();
    let trigger: String = store
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'trigger' AND name = 'event_index_park_moved'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let update = trigger
        .split_once("BEGIN")
        .and_then(|(_, rest)| rest.split_once(';'))
        .map(|(update, _)| update)
        .unwrap()
        .replace("NEW.run_id", "'r'")
        .replace("NEW.seq", "1000");
    assert_eq!(
        crate::test_support::plan(&store, &update, []).unwrap(),
        [
            "SEARCH event_index USING PRIMARY KEY (run_id=? AND seq=?)",
            "LIST SUBQUERY 1",
            "SEARCH event_index USING INDEX event_index_park_facts (run_id=? AND seq<?)",
        ]
    );

    crate::test_support::plant_run(&store.conn, "no-parks", "{}").unwrap();
    let mut statement = store.conn.prepare(INSERT_EVENT).unwrap();
    let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
    let mut steps = Vec::new();
    for seq in 1..=200u64 {
        let event = EnvelopeBuilder::new(EventType::EffectCheckpointed, json!({}))
            .run("no-parks")
            .seq(seq)
            .event_id(format!("no-parks-{seq}"))
            .at("2026-01-01T00:00:00Z")
            .previous(previous.clone())
            .sealed();
        previous.clone_from(&event.event_hash);
        let text = serde_json::to_string(&event).unwrap();
        statement.reset_status(StatementStatus::VmStep);
        statement
            .execute(params!["no-parks", seq as i64, &event.event_hash, &text])
            .unwrap();
        steps.push(statement.reset_status(StatementStatus::VmStep));
    }
    assert!(
        steps.windows(2).all(|pair| pair[0] == pair[1]),
        "an append with no parks costs the same at every seq: {steps:?}"
    );
}

/// A trigger restored after it was gone reconciles the facts a writer
/// advanced while it was missing: the park's stamp is behind its head, so
/// the repair sends it behind and the open's backfill refolds it.
#[test]
fn restoring_the_park_trigger_sends_the_facts_it_missed_behind() {
    let (_dir, mut store) = store();
    let held = parked_row(&mut store);
    let parked_at = held[1].clone();
    store
        .conn
        .execute_batch("DROP TRIGGER event_index_park_moved")
        .unwrap();
    let (seq, previous) = store.head_hash("r").unwrap();
    let answered = EnvelopeBuilder::new(EventType::OperatorCommanded, json!({"command": "go"}))
        .run("r")
        .seq(seq + 1)
        .event_id("r-4")
        .at("2026-01-01T00:00:09Z")
        .previous(previous)
        .sealed();
    // The trigger was gone, so the row still holds the open park's facts
    // at the head it was written at; a binary from before the parks keeps
    // the summary and seat rows current at the new head.
    plant_before_parks(&store, "r", std::slice::from_ref(&answered));
    assert_eq!(kept(&store, "r", 3), held);
    // The sweep finds the stamp the missing trigger left off the head, so
    // the next writable open refolds it whether or not the repair runs.
    assert_eq!(store.park_after().unwrap().as_deref(), Some("r"));
    let store = refolded_lag(store, "r");
    assert_eq!(store.park_after().unwrap(), None);
    assert_eq!(
        kept(&store, "r", 3),
        [
            text("implement"),
            parked_at,
            Value::Integer(at(&answered)),
            Value::Null,
            Value::Integer(64),
            Value::Integer(answered.seq as i64),
        ]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// An older writer's append after a park ended leaves the closed park at an
/// older head; the trigger sends it behind on that append and the next
/// writable open refolds it at the new head, so no later reader trusts a
/// stamp the run has left.
#[test]
fn a_writer_from_before_the_parks_leaves_a_closed_park_behind_its_head() {
    let (_dir, mut store) = store();
    parked_and_moved(&mut store);
    let closed = kept(&store, "r", 3);
    assert_eq!(closed[5], Value::Integer(5));
    assert_ne!(closed[3], Value::Null);
    planted_move(&store, "r", "older");
    assert_eq!(kept(&store, "r", 3)[5], Value::Null);
    let store = refolded_lag(store, "r");
    let refolded_row = kept(&store, "r", 3);
    assert_eq!(refolded_row[..5], closed[..5]);
    assert_eq!(refolded_row[5], Value::Integer(6));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}
