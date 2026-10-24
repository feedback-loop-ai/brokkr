//! The run view's read (decision 0089 ruling 7): at the head the run's
//! opening verified, its trail, its seat rows and its journal a page at a
//! time, every page together the run as `load` reads it; a run that moved
//! past that head, a journal whose index is not read and a run whose seat
//! rows lag are each said so; and the read is one snapshot.

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::seat::row::SeatRow;
use serde_json::{json, Value};

use crate::test_support::unindex;
use crate::tests::{plant_unfenced, store};
use crate::{Opened, RunRead, SeatRows, Store, StoreError};

/// Run `r`'s seat's turn `turn`, checkpointed.
fn turn(turn: u64) -> Value {
    let checkpoint = json!({"step": "seat-turn", "turn": turn, "model": "opus"});
    json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": checkpoint})
}

/// Run `r`: started, a phase entered, one effect requested and started,
/// and `turns` of its seat's turns checkpointed.
fn worked(store: &mut Store, turns: u64) {
    store
        .create_run("r", "a feature", "self", &json!({}))
        .unwrap();
    let begun = [
        (
            EventType::RunStarted,
            json!({"feature": "a feature", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a"}),
        ),
    ];
    let turns = (1..=turns).map(|at| (EventType::EffectCheckpointed, turn(at)));
    for (event_type, payload) in begun.into_iter().chain(turns) {
        store
            .append_next("r", event_type, payload, None, None)
            .unwrap();
    }
}

/// Events, comparable: each by its seq and its hash, which covers it.
fn keys(events: &[EventEnvelope]) -> Vec<(u64, String)> {
    let key = |event: &EventEnvelope| (event.seq, event.event_hash.clone());
    events.iter().map(key).collect()
}

/// A read, comparable: its events by their keys, or why it read none.
#[derive(Debug, PartialEq)]
enum Read {
    Current {
        trail: Vec<(u64, String)>,
        seats: Vec<SeatRow>,
        page: Vec<(u64, String)>,
    },
    Kept,
}

fn read(found: Result<RunRead, StoreError>) -> Read {
    match found.unwrap() {
        RunRead::Current { trail, seats, page } => Read::Current {
            trail: keys(&trail),
            seats,
            page: keys(&page),
        },
        RunRead::Kept => Read::Kept,
    }
}

/// `r`'s seat rows, read as current.
fn seats(store: &Store) -> Vec<SeatRow> {
    match store.seat_rows("r").unwrap() {
        SeatRows::Current(rows) => rows,
        SeatRows::Stale => panic!("r's seat rows read as stale"),
    }
}

/// `r`'s read at `opened` past `after`, at that head: its page's seqs.
fn page(store: &Store, opened: &Opened, after: u64) -> Vec<u64> {
    match read(store.run_read("r", opened, after)) {
        Read::Current { page, .. } => page.into_iter().map(|(seq, _)| seq).collect(),
        other => panic!("r read as {other:?}"),
    }
}

#[test]
fn a_run_is_read_at_the_head_its_opening_verified_a_page_at_a_time() {
    let (_dir, mut store) = store();
    worked(&mut store, 70);
    let opened = store.open_run("r").unwrap();
    assert_eq!(opened.seq, 74);
    let whole = store.load("r").unwrap();
    assert_eq!(
        read(store.run_read("r", &opened, 0)),
        Read::Current {
            trail: keys(&whole[..4]),
            seats: seats(&store),
            page: keys(&whole[..50]),
        }
    );
    // Each page is asked past the last row of the one before it, until a
    // page ends at the head; past the head, a page is empty.
    let mut pages = Vec::new();
    let mut after = 0;
    while after < opened.seq {
        let page = page(&store, &opened, after);
        after = *page.last().unwrap();
        pages.push(page);
    }
    let lengths: Vec<usize> = pages.iter().map(Vec::len).collect();
    assert_eq!(lengths, [50, 24]);
    assert_eq!(pages.concat(), (1..=74).collect::<Vec<u64>>());
    assert_eq!(page(&store, &opened, 74), Vec::<u64>::new());
}

#[test]
fn a_run_past_its_opening_or_its_projections_is_said_so() {
    let (dir, mut store) = store();
    worked(&mut store, 2);
    let opened = store.open_run("r").unwrap();
    let more = (EventType::EffectCheckpointed, turn(3));
    store.append_next("r", more.0, more.1, None, None).unwrap();
    // The run moved past its opening, its index and seat rows with it.
    assert!(matches!(store.seat_rows("r"), Ok(SeatRows::Current(_))));
    assert_eq!(read(store.run_read("r", &opened, 0)), Read::Kept);
    // An older writer's append leaves the seat rows at the head before it.
    plant_unfenced(&mut store, "r", EventType::EffectCheckpointed, turn(4));
    let lagging = store.open_run("r").unwrap();
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    assert_eq!(read(store.run_read("r", &lagging, 0)), Read::Kept);
    drop(store);
    // A writable open catches the seat rows up; a journal from before the
    // index then reads the kept way though they stand at the head.
    let path = dir.path().join("forge.db");
    drop(Store::open(&path).unwrap());
    unindex(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let reader = Store::open_read_only(&path).unwrap();
    assert!(matches!(reader.seat_rows("r"), Ok(SeatRows::Current(_))));
    let opened = reader.open_run("r").unwrap();
    assert_eq!(read(reader.run_read("r", &opened, 0)), Read::Kept);
}

/// A peer's append between the head's read and the rest is not read: the
/// seat rows, the trail and the page stand at the head the opening
/// verified.
#[test]
fn the_read_is_one_snapshot() {
    let (_dir, mut store) = store();
    worked(&mut store, 2);
    let opened = store.open_run("r").unwrap();
    let whole = store.load("r").unwrap();
    let before = seats(&store);
    let mut peer = Store::open(store.path()).unwrap();
    let found = store.run_read_with("r", &opened, 5, || {
        let more = (EventType::EffectCheckpointed, turn(3));
        peer.append_next("r", more.0, more.1, None, None).unwrap();
        peer.refold("r", &mut |_| {}).unwrap();
    });
    let at_the_head = Read::Current {
        trail: keys(&whole[..4]),
        seats: before,
        page: keys(&whole[5..]),
    };
    assert_eq!(read(found), at_the_head);
}
