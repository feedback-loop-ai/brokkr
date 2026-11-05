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
use crate::{Opened, SeatRead, Store};

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

/// `effect`'s checkpoint at `attempt` naming `member`, turn `turn`.
fn checked(effect: &str, attempt: &str, member: Option<&str>, turn: u64) -> (EventType, Json) {
    (
        EventType::EffectCheckpointed,
        checkpoint_of(effect, attempt, member, turn),
    )
}

/// `n` of `effect`'s checkpoints at `attempt` naming `member`, in order.
fn turns(effect: &str, attempt: &str, member: Option<&str>, n: u64) -> Vec<(EventType, Json)> {
    (1..=n)
        .map(|turn| checked(effect, attempt, member, turn))
        .collect()
}

/// `n` result-only records of `member`'s at `attempt`.
fn results(effect: &str, attempt: &str, member: &str, n: u64) -> Vec<(EventType, Json)> {
    (0..n)
        .map(|_| {
            (
                EventType::EffectCheckpointed,
                json!({"effect_id": effect, "attempt_id": attempt,
                       "result": {"member": member}}),
            )
        })
        .collect()
}

/// `n` of `effect`'s own checkpoints at `attempt`, in order.
fn own(effect: &str, attempt: &str, n: u64) -> Vec<(EventType, Json)> {
    turns(effect, attempt, None, n)
}

/// Member `m`'s first checkpoint on attempt `a`, then attempt `b` started
/// and its first checkpoint: the two-attempt preamble the interleaved reads
/// share.
fn interleaved_member() -> Vec<(EventType, Json)> {
    let mut events = asked_and_started("fx", "a");
    events.push(checked("fx", "a", Some("m"), 1));
    events.push((
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "b"}),
    ));
    events.push(checked("fx", "b", Some("m"), 1));
    events
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
/// count takes the kept way for that participant, and another participant
/// whose ordinals stand still reads. A rank below the NULL reads, because
/// only the returned rank's reach is refused (M2); the rank at it does not.
#[test]
fn a_null_ordinal_within_reach_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let opened = journal(&mut store, planted());
    let events = store.load("r").unwrap();
    let m = whole(&events, "fx", Some("m"));
    still_to_number(&store, &[m[1]]);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", Some("m"))).unwrap(),
        None,
        "the member's whole stream cannot stand for itself"
    );
    assert_eq!(
        store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 0)
            .unwrap(),
        Some(m[0]),
        "a rank below the NULL never reaches it"
    );
    assert_eq!(
        store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 1)
            .unwrap(),
        None,
        "a rank at the NULL is refused"
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

/// A corrupt ordinal is never read as zero: the whole count takes the kept
/// way, and a rank whose reach holds it refuses. A rank below it reads,
/// because only the returned rank's reach is refused (M2).
#[test]
fn a_corrupt_ordinal_reads_the_kept_way() {
    for corrupt in [0i64, -1] {
        let (_dir, mut store) = store();
        let mut events = asked_and_started("fx", "a");
        events.extend(own("fx", "a", 2));
        let opened = journal(&mut store, events);
        let events = store.load("r").unwrap();
        let own = whole(&events, "fx", None);
        store
            .conn
            .execute(
                "UPDATE event_index SET stream_ordinal = ?1 WHERE run_id = 'r' AND seq = ?2",
                rusqlite::params![corrupt, own[1] as i64],
            )
            .unwrap();
        assert_eq!(
            store.seat_count("r", &opened, ("fx", None)).unwrap(),
            None,
            "ordinal {corrupt}: the whole count reaches it"
        );
        assert_eq!(
            store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
            Some(own[0]),
            "ordinal {corrupt}: a rank below it never reaches it"
        );
        assert_eq!(
            store.seat_seq_at("r", &opened, ("fx", None), 1).unwrap(),
            None,
            "ordinal {corrupt}: a rank at it is refused"
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

/// H1: a member's checkpoint inside the seat's own reach refuses its count
/// and its rank, so the reader reads the run the kept way rather than count
/// three where the guarded page read refuses. The page read already takes
/// the kept way.
#[test]
fn a_member_checkpoint_in_the_seat_s_reach_refuses_the_ranked_reads() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend([
        checked("fx", "a", None, 1),
        checked("fx", "a", Some("m"), 1),
        checked("fx", "a", None, 2),
        checked("fx", "a", None, 3),
    ]);
    let opened = journal(&mut store, events);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        None,
        "H1: seq 6 stands in the whole participant's reach"
    );
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 2).unwrap(),
        None,
        "H1: rank 2's reach holds seq 6"
    );
    let page = store
        .seat_from("r", &opened, ("fx", None), None, &mut |_| {
            |event: &EventEnvelope| event.seq
        })
        .unwrap();
    assert_eq!(
        page,
        SeatRead::<u64>::Kept,
        "the guarded page read refuses the same reach"
    );
}

/// M2: a NULL at the stream's end is above rank 0's reach, so rank 0 reads
/// the first row; at 300 rows the same is true of rank 1 and the sixth row.
#[test]
fn a_null_at_the_stream_s_end_above_the_rank_does_not_refuse_it() {
    for (rows, rank, expected) in [(6u64, 0u64, 5u64), (300, 1, 6)] {
        let (_dir, mut store) = store();
        let mut events = asked_and_started("fx", "a");
        events.extend(turns("fx", "a", None, rows));
        let opened = journal(&mut store, events);
        let seqs = checkpoint_seqs(&store, None);
        still_to_number(&store, &[seqs[rows as usize - 1]]);
        assert_eq!(
            store.seat_seq_at("r", &opened, ("fx", None), rank).unwrap(),
            Some(expected),
            "{rows} rows: rank {rank} reads the row below the NULL"
        );
        assert_eq!(
            store.seat_count("r", &opened, ("fx", None)).unwrap(),
            None,
            "{rows} rows: the whole count reaches the NULL"
        );
    }
}

/// M2: a NULL inside rank 0's own reach still refuses it.
#[test]
fn a_null_inside_the_rank_s_reach_refuses_it() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend(turns("fx", "a", None, 6));
    let opened = journal(&mut store, events);
    let seqs = checkpoint_seqs(&store, None);
    still_to_number(&store, &[seqs[0]]);
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
        None,
        "the NULL is the destination's own row"
    );
}

/// M3: the interleaved search settles rank 0 on the first numbered row
/// before the result-only record at seq 8; a rank whose reach holds that
/// record still refuses.
#[test]
fn the_interleaved_search_settles_before_a_not_counted_probe() {
    let (_dir, mut store) = store();
    let mut events = interleaved_member();
    events.extend([
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "b", "result": {"member": "m"}}),
        ),
        checked("fx", "b", Some("m"), 2),
        checked("fx", "b", Some("m"), 3),
        checked("fx", "a", Some("m"), 2),
    ]);
    let opened = journal(&mut store, events);
    assert_eq!(
        store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 0)
            .unwrap(),
        Some(5),
        "rank 0 settles on seq 5, before the 0 at seq 8"
    );
    assert_eq!(
        store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 2)
            .unwrap(),
        None,
        "rank 2's reach holds the 0 at seq 8"
    );
}

/// M4: a search probing a run of NULL rows places the probe by the nearest
/// numbered row below it, so rank 0 is Ok(Some(5)), never an SQL type
/// error; a NULL inside the final reach is Ok(None).
#[test]
fn a_search_over_null_numbers_reads_the_rank_below_them() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.push(checked("fx", "a", None, 1));
    events.push((
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "b"}),
    ));
    events.extend(turns("fx", "b", None, 9));
    events.push(checked("fx", "a", None, 2));
    let opened = journal(&mut store, events);
    let own = checkpoint_seqs(&store, None);
    // b's turns 2..8 are seqs 8..14; their endpoints stay numbered.
    still_to_number(&store, &own[2..=8]);
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
        Some(5),
        "rank 0 is read, never an InvalidColumnType"
    );
    still_to_number(&store, &[own[0]]);
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 0).unwrap(),
        None,
        "a NULL inside the final reach is Ok(None)"
    );
}

/// The participant's count, its last checkpoint's seq and its newest page's
/// row count, as the end read makes them.
fn end_read(
    store: &Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
) -> (Option<u64>, Option<u64>, usize) {
    let count = store.seat_count("r", opened, seat).unwrap();
    let last = count.and_then(|total| store.seat_seq_at("r", opened, seat, total - 1).unwrap());
    let page = match store
        .seat_read("r", opened, seat, None, |_| {
            |event: &EventEnvelope| event.seq
        })
        .unwrap()
    {
        SeatRead::Current(rows, _) => rows.len(),
        SeatRead::Kept => 0,
    };
    (count, last, page)
}

/// The participant's `rank`-th seq and the page above the row before it, as
/// the middle read makes them.
fn middle_read(
    store: &Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
    rank: u64,
) -> (Option<u64>, usize) {
    let seq = store.seat_seq_at("r", opened, seat, rank).unwrap();
    let after = seq.map(|seq| seq.saturating_sub(1));
    let page = match store
        .seat_from("r", opened, seat, after, &mut |_| {
            |event: &EventEnvelope| event.seq
        })
        .unwrap()
    {
        SeatRead::Current(rows, _) => rows.len(),
        SeatRead::Kept => 0,
    };
    (seq, page)
}

/// The few steps a B-tree one level deeper takes at 20,000 rows: a bound
/// read may differ by this much and still not scale with the stream.
const SLACK: i64 = 500;

/// Assert the measurements differ by no more than [`SLACK`], naming the
/// values when they do.
fn bounded(what: &str, sizes: &[i64]) {
    let low = *sizes.iter().min().unwrap();
    let high = *sizes.iter().max().unwrap();
    assert!(
        high - low <= SLACK,
        "{what}: the steps scale with the stream: {sizes:?}"
    );
}

/// The end and middle measurements at 2,000 and 20,000 checkpoints, by
/// `measure`'s journal at each size.
fn over_sizes(mut measure: impl FnMut(u64) -> (i64, i64)) -> (Vec<i64>, Vec<i64>) {
    let mut end = Vec::new();
    let mut middle = Vec::new();
    for n in [2_000u64, 20_000] {
        let (end_steps, middle_steps) = measure(n);
        end.push(end_steps);
        middle.push(middle_steps);
    }
    (end, middle)
}

/// The VM steps `sql` takes for `bound`, through a statement of its own, so
/// a seek that scans a range is named however its value stands.
fn steps(store: &Store, sql: &str, bound: impl rusqlite::Params) -> i64 {
    use rusqlite::StatementStatus;
    let mut statement = store.conn.prepare(sql).unwrap();
    statement.reset_status(StatementStatus::VmStep);
    let mut rows = statement.query(bound).unwrap();
    while rows.next().unwrap().is_some() {}
    drop(rows);
    i64::from(statement.reset_status(StatementStatus::VmStep))
}

/// A participant's end and middle reads and the steps their queries take.
struct Measured {
    end: (Option<u64>, Option<u64>, usize),
    middle: (Option<u64>, usize),
    steps: (i64, i64),
}

/// The participant `member`'s end and middle reads at `rank`, their values
/// and the steps their queries take.
fn measured(
    store: &Store,
    opened: &Opened,
    member: &str,
    rank: u64,
    last: u64,
    seq: u64,
    ordinal: u64,
) -> Measured {
    let seat = ("fx", Some(member));
    Measured {
        end: end_read(store, opened, seat),
        middle: middle_read(store, opened, seat, rank),
        steps: span_steps(store, member, last, seq, ordinal),
    }
}

/// The steps of the queries a participant's end read makes at `last` and a
/// middle read at `seq`: the rank seeks, BEFORE, GAP, RESTART, UNCOUNTED
/// and the page.
fn span_steps(store: &Store, member: &str, last: u64, seq: u64, ordinal: u64) -> (i64, i64) {
    use super::ordinal::{AT, BEFORE, BOUND, GAP, RESTART, UNCOUNTED};
    let first = 5i64;
    let last = last as i64;
    let seq = seq as i64;
    let one = |sql: &str, low: i64, high: i64| {
        steps(
            store,
            sql,
            rusqlite::params!["r", "a", Some(member), low, high],
        )
    };
    let bound = |at: i64| steps(store, &BOUND, rusqlite::params!["r", "a", Some(member), at]);
    let page = |before: i64| {
        steps(
            store,
            &crate::event_index::STREAM,
            rusqlite::params!["r", "a", Some(member), i64::MIN, before, 51i64],
        )
    };
    let end = bound(first)
        + bound(last)
        + one(&GAP, first, last)
        + one(&RESTART, first, last)
        + one(&UNCOUNTED, first, last)
        + page(last);
    let middle = steps(
        store,
        &AT,
        rusqlite::params!["r", "a", Some(member), first, last, ordinal as i64],
    ) + one(&BEFORE, first, seq)
        + one(&GAP, first, seq)
        + one(&RESTART, first, seq)
        + one(&UNCOUNTED, first, seq)
        + page(seq);
    (end, middle)
}

/// M1: a member's end and middle reads take the same steps at 2,000 and
/// 20,000 checkpoints; the not-counted seek keys the ordinal, so it never
/// scans the stream.
#[test]
fn the_member_reads_step_the_same_at_every_size() {
    let (end, middle) = over_sizes(|n| {
        let (_dir, mut store) = store();
        let mut events = asked_and_started("fx", "a");
        events.extend(turns("fx", "a", Some("m"), n));
        let opened = journal(&mut store, events);
        let measured = measured(&store, &opened, "m", n / 2, n + 4, n / 2 + 4, n / 2 + 1);
        assert_eq!(measured.end.0, Some(n), "{n}: the read-only count");
        assert_eq!(
            measured.end.1,
            Some(n + 4),
            "{n}: the last checkpoint's seq"
        );
        assert!(measured.end.2 > 0, "{n}: the end page reads");
        assert_eq!(measured.middle.0, Some(n / 2 + 5), "{n}: the middle rank");
        assert!(measured.middle.1 > 0, "{n}: the middle page reads");
        measured.steps
    });
    bounded("the member's end read", &end);
    bounded("the member's middle read", &middle);
}

/// 994o2's bound: the target's end and middle reads step the same at both
/// sizes while N rows still to number of another member, or on another
/// attempt, sit between its checkpoints. The stale index keys their stream,
/// so neither read scans them.
#[test]
fn the_stale_seek_steps_the_same_at_every_size() {
    let mut shapes = Vec::new();
    for (shape, attempt) in [("other member", "a"), ("other attempt", "b")] {
        let (end, middle) = over_sizes(|n| {
            let (_dir, mut store) = store();
            let mut events = asked_and_started("fx", "a");
            events.push(checked("fx", "a", Some("target"), 1));
            events.extend(results("fx", attempt, "m", n));
            events.push(checked("fx", "a", Some("target"), 3));
            let _opened = journal(&mut store, events);
            let stale = result_seqs(&store);
            assert_eq!(stale.len() as u64, n);
            // Model a lost trigger: the rows are still to number, which is
            // what the stale index holds. A repair would number them 0.
            still_to_number(&store, &stale);
            let opened = store.open_run("r").unwrap();
            let last = n + 6;
            let measured = measured(&store, &opened, "target", 1, last, last, 2);
            assert_eq!(measured.end.0, Some(2), "{shape} at {n}: the target's two");
            assert_eq!(
                measured.middle.0,
                Some(last),
                "{shape} at {n}: its last seq"
            );
            measured.steps
        });
        bounded(&format!("{shape}: the end read"), &end);
        bounded(&format!("{shape}: the middle read"), &middle);
        shapes.push(shape);
    }
    assert_eq!(shapes, ["other member", "other attempt"]);
}

/// Step 4's bound: a search probing a run of N not-counted rows settles on
/// the first numbered row in the same steps at both sizes, because the
/// numbered index places each probe in one seek.
#[test]
fn the_search_probe_steps_the_same_at_every_size() {
    use super::ordinal::{BEFORE, GAP, RESTART, UNCOUNTED};
    let mut sizes = Vec::new();
    for n in [2_000u64, 20_000] {
        let (_dir, mut store) = store();
        let mut events = interleaved_member();
        events.extend(results("fx", "a", "m", n));
        events.push(checked("fx", "b", Some("m"), 2));
        events.push(checked("fx", "a", Some("m"), 2));
        let opened = journal(&mut store, events);
        let found = store
            .seat_seq_at("r", &opened, ("fx", Some("m")), 0)
            .unwrap();
        assert_eq!(found, Some(5), "{n}: the probe settles on seq 5");
        let mid = 5 + n / 2;
        let probe = steps(
            &store,
            &BEFORE,
            rusqlite::params!["r", "a", Some("m"), 5i64, mid as i64],
        );
        let reach = steps(
            &store,
            &GAP,
            rusqlite::params!["r", "a", Some("m"), 5i64, 5i64],
        ) + steps(
            &store,
            &RESTART,
            rusqlite::params!["r", "a", Some("m"), 5i64, 5i64],
        ) + steps(
            &store,
            &UNCOUNTED,
            rusqlite::params!["r", "a", Some("m"), 5i64, 5i64],
        );
        sizes.push(probe + reach);
    }
    bounded("the search probe", &sizes);
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
