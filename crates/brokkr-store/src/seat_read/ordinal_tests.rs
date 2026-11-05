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

/// The seqs of `run`'s checkpoints naming `member` in their checkpoint,
/// oldest first.
fn checkpoint_seqs(store: &Store, member: Option<&str>) -> Vec<u64> {
    store
        .load("r")
        .unwrap()
        .iter()
        .filter(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["checkpoint"]["member"].as_str() == member
        })
        .map(|event| event.seq)
        .collect()
}

/// The seqs of `run`'s result-only records, oldest first.
fn result_seqs(store: &Store) -> Vec<u64> {
    store
        .load("r")
        .unwrap()
        .iter()
        .filter(|event| event.payload.get("result").is_some())
        .map(|event| event.seq)
        .collect()
}

/// Leave the rows at `seqs` still to number, as a lost trigger does.
fn still_to_number(store: &Store, seqs: &[u64]) {
    let mut update = store
        .conn
        .prepare("UPDATE event_index SET stream_ordinal = NULL WHERE run_id = 'r' AND seq = ?1")
        .unwrap();
    for seq in seqs {
        update.execute([*seq as i64]).unwrap();
    }
}

/// A run whose member `m` records `turns` checkpoints, with a result-only
/// record of `m` after turn `result_at`.
fn member_with_record(store: &mut Store, turns: u64, result_at: u64) -> Opened {
    let mut events = asked_and_started("fx", "a");
    for turn in 1..=turns {
        events.push((
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "a", Some("m"), turn),
        ));
        if turn == result_at {
            events.push((
                EventType::EffectCheckpointed,
                json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}}),
            ));
        }
    }
    if result_at > turns {
        events.push((
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}}),
        ));
    }
    journal(store, events)
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

/// `fx`'s attempt `a` records turn 1, attempt `b` records `b_turns`, then
/// `a` records turn 2, so the two attempts' ranges overlap.
fn interleaved(store: &mut Store, b_turns: u64) -> Opened {
    let mut events = asked_and_started("fx", "a");
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", None, 1),
    ));
    events.extend(asked_and_started("fx", "b").into_iter().skip(1));
    for turn in 1..=b_turns {
        events.push((
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "b", None, turn),
        ));
    }
    events.push((
        EventType::EffectCheckpointed,
        checkpoint_of("fx", "a", None, 2),
    ));
    journal(store, events)
}

/// A participant's attempts whose checkpoints interleave are ranked by
/// seq, not by one stream's order: the search among overlapping ranges is
/// exact.
#[test]
fn interleaved_attempts_rank_by_seq() {
    let (_dir, mut store) = store();
    let opened = interleaved(&mut store, 1);
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
    still_to_number(&store, &[broken]);
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
    let opened = member_with_record(&mut store, 1, 2);
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

/// A result-only record of a member's inside the seat's own span is the
/// kept refusal the guarded page read takes (H1): the ordinal count and
/// rank refuse, so a reader reads the run whole rather than count 120
/// where the whole read counts 121.
#[test]
fn a_member_s_record_inside_the_seat_s_span_refuses_the_ordinal_reads() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    for turn in 1..=120 {
        events.push((
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "a", None, turn),
        ));
        if turn == 2 {
            events.push((
                EventType::EffectCheckpointed,
                json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}}),
            ));
        }
    }
    let opened = journal(&mut store, events);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        None,
        "the seat's own participant cannot stand for its 121 whole rows"
    );
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 100).unwrap(),
        None,
        "a rank past the member's record reaches the refusal"
    );
}

/// Another effect's checkpoints on the same attempt break the span (H2):
/// the restart at each effect's first row means the span cannot be counted
/// by difference, so the count and the rank take the kept way.
#[test]
fn another_effect_s_rows_on_the_same_attempt_refuse_the_span() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend(asked_and_started("gx", "a"));
    for (effect, turn) in [("fx", 1), ("gx", 1), ("fx", 2), ("gx", 2), ("fx", 3)] {
        events.push((
            EventType::EffectCheckpointed,
            checkpoint_of(effect, "a", None, turn),
        ));
    }
    let opened = journal(&mut store, events);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        None,
        "the foreign effect's rows break fx's span"
    );
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 1).unwrap(),
        None,
        "a rank whose reach holds the restart is kept"
    );
}

/// A member's own participant refuses a result-only record inside its own
/// stream's reach (the mirror of H1), and reads when that record is below
/// its span.
#[test]
fn a_member_s_not_counted_row_refuses_only_inside_its_span() {
    {
        let (_dir, mut store) = store();
        let opened = member_with_record(&mut store, 2, 1);
        assert_eq!(
            store.seat_count("r", &opened, ("fx", Some("m"))).unwrap(),
            None,
            "the record inside m's span is not counted"
        );
        assert_eq!(
            store
                .seat_seq_at("r", &opened, ("fx", Some("m")), 1)
                .unwrap(),
            None
        );
    }

    let (_dir, mut store) = store();
    let opened = member_with_record(&mut store, 2, 3);
    let own = checkpoint_seqs(&store, Some("m"));
    assert_eq!(
        store.seat_count("r", &opened, ("fx", Some("m"))).unwrap(),
        Some(2),
        "the record below m's span is outside its reach"
    );
    assert_eq!(
        store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 1)
            .unwrap(),
        Some(own[1]),
        "the rank reads m's own second checkpoint"
    );
}

/// M1: a span's whole check refuses a window only where that window
/// reaches it. A NULL above the rank is not in the rank's reach, so a low
/// rank reads; a rank at it does not.
#[test]
fn a_null_above_a_rank_does_not_refuse_that_rank() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    for turn in 1..=6 {
        events.push((
            EventType::EffectCheckpointed,
            checkpoint_of("fx", "a", None, turn),
        ));
    }
    let opened = journal(&mut store, events);
    let seqs = checkpoint_seqs(&store, None);
    still_to_number(&store, &[seqs[3]]);
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 2).unwrap(),
        Some(seqs[2]),
        "a rank below the NULL reads"
    );
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 3).unwrap(),
        None,
        "a rank at the NULL is refused"
    );
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        None,
        "the whole count reaches the NULL"
    );
}

/// M3: the stale index's seek steps the same at every size and for both
/// shapes. N rows still to number of another member, and N on a separate
/// attempt, never lie in the target's range; the old run-only key scanned
/// both. The end's whole-span seek and a middle seek take the same steps.
#[test]
fn the_stale_index_seek_steps_the_same_at_every_size() {
    use rusqlite::StatementStatus;

    let mut measured = Vec::new();
    for (shape, attempt) in [("other member", "a"), ("other attempt", "b")] {
        let mut sizes = Vec::new();
        for n in [2_000u64, 20_000] {
            let (_dir, mut store) = store();
            let mut events = asked_and_started("fx", "a");
            events.push((
                EventType::EffectCheckpointed,
                checkpoint_of("fx", "a", Some("target"), 1),
            ));
            for _ in 0..n {
                events.push((
                    EventType::EffectCheckpointed,
                    json!({"effect_id": "fx", "attempt_id": attempt,
                           "result": {"member": "m"}}),
                ));
            }
            events.push((
                EventType::EffectCheckpointed,
                checkpoint_of("fx", "a", Some("target"), 3),
            ));
            let _opened = journal(&mut store, events);
            let target = checkpoint_seqs(&store, Some("target"));
            let stale = result_seqs(&store);
            assert_eq!(stale.len() as u64, n);
            // Model a lost trigger: the rows are still to number, which is
            // what the stale index holds. A repair would number them 0.
            still_to_number(&store, &stale);
            let opened = store.open_run("r").unwrap();
            assert_eq!(
                store
                    .seat_count("r", &opened, ("fx", Some("target")))
                    .unwrap(),
                Some(2),
                "{shape} at {n}: the target's own two count"
            );
            let steps = |low: u64, high: u64| {
                let mut statement = store.conn.prepare(&super::ordinal::GAP).unwrap();
                statement.reset_status(StatementStatus::VmStep);
                let found: bool = statement
                    .query_row(
                        rusqlite::params!["r", "a", "target", low as i64, high as i64],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert!(!found, "{shape} at {n}: nothing still to number in range");
                i64::from(statement.reset_status(StatementStatus::VmStep))
            };
            let end = steps(target[0], target[1]);
            let middle = steps(target[0], target[1] - 1);
            sizes.push((end, middle));
        }
        assert_eq!(
            sizes[0], sizes[1],
            "{shape}: the seek steps the same at both sizes: {sizes:?}"
        );
        assert_eq!(
            sizes[0].0, sizes[0].1,
            "{shape}: the end and middle seeks take the same steps: {sizes:?}"
        );
        measured.push(shape);
    }
    assert_eq!(measured, ["other member", "other attempt"]);
}

/// A not-counted row in the middle of an interleaved rank search is never
/// read as a place: the candidate ordinal falls below the span's first, so
/// the search takes the kept way.
#[test]
fn a_not_counted_row_in_an_interleaved_search_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let opened = interleaved(&mut store, 3);
    let own = checkpoint_seqs(&store, None);
    // The middle row of attempt b, not its bound: the span's bounds still
    // carry places, and only the search's candidate reads the 0.
    store
        .conn
        .execute(
            "UPDATE event_index SET stream_ordinal = 0 WHERE run_id = 'r' AND seq = ?1",
            [own[2] as i64],
        )
        .unwrap();
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 2).unwrap(),
        None,
        "the interleaved search never reads the not-counted row as a place"
    );
}
