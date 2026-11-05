//! The participant's count and rank by stream ordinal (#994): they equal
//! the whole read's for every participant, at every rank and past the end;
//! two sequential effects on one attempt count apart (M1); a NULL ordinal
//! within a stream takes the kept way and one outside a participant does
//! not refuse another; and more streams than a page keeps read the kept
//! way.

use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::{json, Value as Json};

use crate::test_support::{append, checkpoint_of, opened};
use crate::tests::store;
use crate::{Opened, Store};

/// The two events that ask for `effect` and start its `attempt`.
fn asked_and_started(effect: &str, attempt: &str) -> Vec<(EventType, Json)> {
    vec![
        (EventType::EffectRequested, json!({"effect_id": effect})),
        (
            EventType::EffectStarted,
            json!({"effect_id": effect, "attempt_id": attempt}),
        ),
    ]
}

/// `n` of `effect`'s own checkpoints at `attempt`, in order.
fn own(effect: &str, attempt: &str, n: u64) -> Vec<(EventType, Json)> {
    (1..=n)
        .map(|turn| {
            (
                EventType::EffectCheckpointed,
                checkpoint_of(effect, attempt, None, turn),
            )
        })
        .collect()
}

/// A run `r` with `events` appended, opened.
fn journal(store: &mut Store, events: Vec<(EventType, Json)>) -> Opened {
    opened(store, "r", "implement", &[]);
    for (kind, payload) in events {
        append(store, "r", kind, payload);
    }
    store.open_run("r").unwrap()
}

/// The seqs the whole read keys to `effect`'s participant `member`, oldest
/// first: the checkpoints whose checkpoint names that member.
fn whole(events: &[EventEnvelope], effect: &str, member: Option<&str>) -> Vec<u64> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| event.payload.get("effect_id").and_then(Json::as_str) == Some(effect))
        .filter(|event| {
            event
                .payload
                .get("checkpoint")
                .and_then(|checkpoint| checkpoint.get("member"))
                .and_then(Json::as_str)
                == member
        })
        .map(|event| event.seq)
        .collect()
}

/// A journal of three participants: fx's own three checkpoints across two
/// attempts, fx's member `m`'s two, and gx's own two.
fn planted() -> Vec<(EventType, Json)> {
    let mut events = asked_and_started("fx", "a");
    events.extend(own("fx", "a", 2));
    events.push((
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "b"}),
    ));
    events.extend([
        (
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "b", Some("m"), 1),
        ),
        (
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "b", Some("m"), 2),
        ),
        (
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "b", None, 3),
        ),
    ]);
    events.extend(asked_and_started("gx", "c"));
    events.extend(own("gx", "c", 2));
    events
}

/// The count and rank equal the whole read's for every participant, at
/// every rank and past the end.
#[test]
fn count_and_rank_equal_the_whole_read_for_every_participant() {
    let (_dir, mut store) = store();
    let opened = journal(&mut store, planted());
    let events = store.load("r").unwrap();
    for seat in [("fx", None), ("fx", Some("m")), ("gx", None)] {
        let expected = whole(&events, seat.0, seat.1);
        assert!(!expected.is_empty(), "{seat:?} planted");
        assert_eq!(
            store.seat_count("r", &opened, seat).unwrap(),
            Some(expected.len() as u64),
            "{seat:?}: the count"
        );
        for (rank, seq) in expected.iter().enumerate() {
            assert_eq!(
                store.seat_seq_at("r", &opened, seat, rank as u64).unwrap(),
                Some(*seq),
                "{seat:?}: rank {rank}"
            );
        }
        assert_eq!(
            store
                .seat_seq_at("r", &opened, seat, expected.len() as u64)
                .unwrap(),
            None,
            "{seat:?}: past the end"
        );
    }
}

/// M1: two sequential effects reusing one attempt keep their rows' bounds,
/// so each counts its own two checkpoints and ranks from its own first.
#[test]
fn two_sequential_effects_on_one_attempt_count_apart() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend(own("fx", "a", 2));
    events.extend(asked_and_started("gx", "a"));
    events.extend(own("gx", "a", 2));
    let opened = journal(&mut store, events);
    let events = store.load("r").unwrap();
    let fx = whole(&events, "fx", None);
    let gx = whole(&events, "gx", None);
    assert_eq!((fx.len(), gx.len()), (2, 2));
    assert_eq!(
        [
            store.seat_count("r", &opened, ("fx", None)).unwrap(),
            store.seat_count("r", &opened, ("gx", None)).unwrap(),
        ],
        [Some(2), Some(2)],
        "each effect counts its own two, not the shared attempt's four"
    );
    assert_eq!(
        [
            store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
            store.seat_seq_at("r", &opened, ("gx", None), 0).unwrap(),
            store.seat_seq_at("r", &opened, ("gx", None), 1).unwrap(),
        ],
        [Some(fx[0]), Some(gx[0]), Some(gx[1])]
    );
}

/// A participant's attempts whose checkpoints interleave are ranked by
/// seq, not by one stream's order: the search among overlapping ranges is
/// exact.
#[test]
fn interleaved_attempts_rank_by_seq() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", None, 1),
    ));
    events.push((
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "b"}),
    ));
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "b", None, 1),
    ));
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", None, 2),
    ));
    let opened = journal(&mut store, events);
    let events = store.load("r").unwrap();
    let expected = whole(&events, "fx", None);
    assert_eq!(expected.len(), 3);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        Some(3)
    );
    for (rank, seq) in expected.iter().enumerate() {
        assert_eq!(
            store
                .seat_seq_at("r", &opened, ("fx", None), rank as u64)
                .unwrap(),
            Some(*seq),
            "rank {rank}"
        );
    }
}

/// A NULL ordinal inside a participant's stream is never read as zero: the
/// read takes the kept way for that participant, and another participant
/// whose ordinals stand still reads.
#[test]
fn a_null_ordinal_within_reach_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let opened = journal(&mut store, planted());
    let events = store.load("r").unwrap();
    let broken = whole(&events, "fx", Some("m"))[1];
    store
        .conn
        .execute(
            "UPDATE event_index SET stream_ordinal = NULL WHERE run_id = 'r' AND seq = ?1",
            [broken as i64],
        )
        .unwrap();
    assert_eq!(
        store.seat_count("r", &opened, ("fx", Some("m"))).unwrap(),
        None,
        "the member's stream cannot stand for itself"
    );
    assert_eq!(
        store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 0)
            .unwrap(),
        None
    );
    assert_eq!(
        store.seat_count("r", &opened, ("gx", None)).unwrap(),
        Some(2),
        "another participant's ordinals still stand"
    );
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        Some(3),
        "the seat's own stream is another participant"
    );
}

/// A participant with more streams than a page keeps reads the kept way,
/// as its page does.
#[test]
fn more_streams_than_a_page_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let mut events = vec![(EventType::EffectRequested, json!({"effect_id": "fx"}))];
    for at in 0..65 {
        let attempt = format!("a{at}");
        events.extend(asked_and_started("fx", &attempt).into_iter().skip(1));
        events.extend(own("fx", &attempt, 1));
    }
    let opened = journal(&mut store, events);
    assert_eq!(store.seat_count("r", &opened, ("fx", None)).unwrap(), None);
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
        None
    );
}

/// A stream whose bound row is a member's result-only record has no
/// ordinal to read, so the seat's own participant takes the kept way too:
/// the record is an `effect/checkpointed` row indexed under the member its
/// result names, not the seat's own.
#[test]
fn a_result_only_bound_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", Some("m"), 1),
    ));
    events.push((
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}}),
    ));
    let opened = journal(&mut store, events);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        None,
        "the seat's own row ends at a record its stream does not key"
    );
    assert_eq!(
        store.seat_count("r", &opened, ("fx", Some("m"))).unwrap(),
        Some(1),
        "the member's own checkpoint is its only one"
    );
}

/// A corrupt ordinal is never read as zero: a bound below its stream's
/// first, or one that is not a place at all, takes the kept way.
#[test]
fn a_corrupt_ordinal_reads_the_kept_way() {
    for corrupt in [0i64, -1] {
        let (_dir, mut store) = store();
        let mut events = asked_and_started("fx", "a");
        events.extend(own("fx", "a", 2));
        let opened = journal(&mut store, events);
        let events = store.load("r").unwrap();
        let last = whole(&events, "fx", None)[1];
        store
            .conn
            .execute(
                "UPDATE event_index SET stream_ordinal = ?1 WHERE run_id = 'r' AND seq = ?2",
                rusqlite::params![corrupt, last as i64],
            )
            .unwrap();
        assert_eq!(
            store.seat_count("r", &opened, ("fx", None)).unwrap(),
            None,
            "ordinal {corrupt}"
        );
        assert_eq!(
            store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
            None,
            "ordinal {corrupt}"
        );
    }
}
