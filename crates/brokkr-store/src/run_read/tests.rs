//! The run view's read (decision 0089 ruling 7): at the head the run's
//! opening verified, its trail, its seat rows and its journal a page at a
//! time, every page together the run as `load` reads it, with what caused
//! each page's rows, the fold at the head and the head's stamp; a run that
//! moved past that head, a journal whose index or summary is not read, a
//! run whose seat rows or summary row lag, a run with no event and an
//! index row naming no cause are each said so; and the read is one
//! snapshot.

use std::collections::BTreeMap;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{fold, RunState};
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

/// A fold, comparable: its seq, its hash and where the run stands.
type Stands = (u64, String, String);

fn stands(state: &RunState) -> Stands {
    (state.seq, state.last_hash.clone(), state.run.to_string())
}

/// A read, comparable: its events by their keys, its fold by where it
/// stands, or why it read none.
#[derive(Debug, PartialEq)]
enum Read {
    Current {
        trail: Vec<(u64, String)>,
        seats: Vec<SeatRow>,
        page: Vec<(u64, String)>,
        causes: BTreeMap<String, u64>,
        state: Option<Stands>,
        last_at: String,
    },
    Kept,
}

fn read(found: Result<RunRead, StoreError>) -> Read {
    match found.unwrap() {
        RunRead::Current {
            trail,
            seats,
            page,
            causes,
            state,
            last_at,
        } => Read::Current {
            trail: keys(&trail),
            seats,
            page: keys(&page),
            causes,
            state: state.as_deref().map(stands),
            last_at,
        },
        RunRead::Kept => Read::Kept,
    }
}

/// `r` read whole: its events, its fold where it stands, and its last
/// event's stamp.
fn whole(store: &Store) -> (Vec<EventEnvelope>, Option<Stands>, String) {
    let whole = store.load("r").unwrap();
    let state = stands(&fold(&whole).unwrap());
    let last_at = whole.last().unwrap().recorded_at.clone();
    (whole, Some(state), last_at)
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
    let (whole, state, last_at) = whole(&store);
    // The trail folds to the run's state, the head's seq and hash its own.
    let trail = fold(&whole[..4]).unwrap();
    assert_ne!(Some(stands(&trail)), state);
    assert_eq!(
        read(store.run_read("r", &opened, 0)),
        Read::Current {
            trail: keys(&whole[..4]),
            seats: seats(&store),
            page: keys(&whole[..50]),
            causes: BTreeMap::new(),
            state,
            last_at,
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
    let (whole, state, last_at) = whole(&store);
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
        causes: BTreeMap::new(),
        state,
        last_at,
    };
    assert_eq!(read(found), at_the_head);
}

/// Each page's rows are read with what caused them, the seq of the event
/// each causation id names wherever it stands in the run, and none for an
/// id no event of it carries.
#[test]
fn a_page_is_read_with_what_caused_its_rows() {
    let (_dir, mut store) = store();
    worked(&mut store, 60);
    let started = store.load("r").unwrap()[3].event_id.clone();
    for cause in [started.as_str(), "no event's"] {
        let more = (EventType::EffectCheckpointed, turn(61));
        let cause = Some(cause.to_string());
        store.append_next("r", more.0, more.1, cause, None).unwrap();
    }
    let opened = store.open_run("r").unwrap();
    let Read::Current { causes, .. } = read(store.run_read("r", &opened, 50)) else {
        panic!("r read as kept");
    };
    assert_eq!(causes, BTreeMap::from([(started, 4)]));
}

/// A run whose fold refuses a checkpoint, which only checks the effect it
/// names is open, reads no state at its head, though its trail folds: its
/// summary row records the refusal there.
#[test]
fn a_run_whose_fold_refuses_a_checkpoint_reads_no_state() {
    let (_dir, mut store) = store();
    worked(&mut store, 1);
    let mut elsewhere = turn(2);
    elsewhere["effect_id"] = json!("elsewhere");
    let refused = (EventType::EffectCheckpointed, elsewhere);
    store
        .append_next("r", refused.0, refused.1, None, None)
        .unwrap();
    let opened = store.open_run("r").unwrap();
    let whole = store.load("r").unwrap();
    let refusal = "event 6: EffectCheckpointed is impossible at cursor EffectInFlight { \
         effect_id: \"fx\", attempt_id: \"a\", seat: \"worker\", failed_attempts: 0 }";
    assert_eq!(fold(&whole).unwrap_err().to_string(), refusal);
    assert_eq!(fold(&whole[..4]).unwrap().seq, 4);
    let Read::Current { state, .. } = read(store.run_read("r", &opened, 0)) else {
        panic!("r read as kept");
    };
    assert_eq!(state, None);
}

/// A run with no event, a journal whose summary's backfill has not
/// finished, a summary row behind the head and an index row naming no
/// cause each read the kept way.
#[test]
fn a_run_its_summary_or_its_causes_do_not_stand_for_is_read_the_kept_way() {
    let (dir, mut store) = store();
    store
        .create_run("empty", "none yet", "self", &json!({}))
        .unwrap();
    let opened = store.open_run("empty").unwrap();
    assert_eq!(read(store.run_read("empty", &opened, 0)), Read::Kept);
    worked(&mut store, 2);
    let whole = store.load("r").unwrap();
    let (first, started) = (whole[0].event_id.clone(), whole[3].event_id.clone());
    let more = (EventType::EffectCheckpointed, turn(3));
    store
        .append_next("r", more.0, more.1, Some(started), None)
        .unwrap();
    let opened = store.open_run("r").unwrap();
    assert!(matches!(
        read(store.run_read("r", &opened, 0)),
        Read::Current { .. }
    ));
    let conn = rusqlite::Connection::open(dir.path().join("forge.db")).unwrap();
    let finished: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'run_summary_backfill'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let kept = |edit: &str, undo: &str| {
        conn.execute_batch(edit).unwrap();
        let found = read(store.run_read("r", &opened, 0));
        conn.execute_batch(undo).unwrap();
        found
    };
    let unfinished = kept(
        "DELETE FROM meta WHERE key = 'run_summary_backfill'",
        &format!("INSERT INTO meta (key, value) VALUES ('run_summary_backfill', '{finished}')"),
    );
    assert_eq!(unfinished, Read::Kept);
    let behind = kept(
        "UPDATE run_summary SET folded_seq = folded_seq - 1 WHERE run_id = 'r'",
        "UPDATE run_summary SET folded_seq = folded_seq + 1 WHERE run_id = 'r'",
    );
    assert_eq!(behind, Read::Kept);
    let idless = kept(
        "UPDATE event_index SET event_id = NULL WHERE run_id = 'r' AND seq = 1",
        &format!("UPDATE event_index SET event_id = '{first}' WHERE run_id = 'r' AND seq = 1"),
    );
    assert_eq!(idless, Read::Kept);
    assert!(matches!(
        read(store.run_read("r", &opened, 0)),
        Read::Current { .. }
    ));
}
