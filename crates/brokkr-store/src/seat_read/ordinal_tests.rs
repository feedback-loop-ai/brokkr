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
#[cfg(feature = "test-support")]
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

/// The event that starts `effect`'s `attempt`.
fn started(effect: &str, attempt: &str) -> (EventType, Json) {
    (
        EventType::EffectStarted,
        json!({"effect_id": effect, "attempt_id": attempt}),
    )
}

/// `fx`'s two own checkpoints on attempt `a`, then `gx` asked and started
/// on it: the shared start of the sequential-effects cases.
fn two_effects_on_one_attempt() -> Vec<(EventType, Json)> {
    let mut events = asked_and_started("fx", "a");
    events.extend(own("fx", "a", 2));
    events.extend(asked_and_started("gx", "a"));
    events
}

/// Member `m`'s first checkpoint on attempt `a`, then attempt `b` started
/// and its first checkpoint: the two-attempt preamble the interleaved reads
/// share.
fn interleaved_member() -> Vec<(EventType, Json)> {
    let mut events = asked_and_started("fx", "a");
    events.push(checked("fx", "a", Some("m"), 1));
    events.push(started("fx", "b"));
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

/// The participant `seat`'s count and every rank on `events`, whose whole
/// read holds `rows` checkpoints: the kept way.
fn assert_kept(
    store: &mut Store,
    events: Vec<(EventType, Json)>,
    seat: (&str, Option<&str>),
    rows: usize,
) {
    let opened = journal(store, events);
    let loaded = store.load("r").unwrap();
    assert_eq!(
        whole(&loaded, seat.0, seat.1).len(),
        rows,
        "{seat:?}: the whole rows"
    );
    kept_way(store, &opened, seat, rows);
}

/// H1's journal events: the member's two attempts interleave.
fn h1_events() -> Vec<(EventType, Json)> {
    let mut events = asked_and_started("fx", "a");
    events.extend([
        checked("fx", "a", Some("m"), 1),
        started("fx", "b"),
        checked("fx", "b", Some("m"), 1),
        checked("fx", "b", Some("m"), 2),
        checked("fx", "a", Some("m"), 2),
    ]);
    events
}

/// A disjoint participant of three spans: one checkpoint on attempt `a`,
/// N on `b`, and one on `c`.
fn three_spans(n: u64) -> Vec<(EventType, Json)> {
    let mut events = asked_and_started("fx", "a");
    events.extend(own("fx", "a", 1));
    events.push(started("fx", "b"));
    events.extend(own("fx", "b", n));
    events.push(started("fx", "c"));
    events.extend(own("fx", "c", 1));
    events
}

/// `store`'s run marked lagging for its stated projection fault, the
/// writer closed, and the journal reopened read-only and opened.
fn reopen_read_only(mut store: Store) -> (Store, Opened) {
    crate::test_support::lags(&mut store, &["r"]);
    let path = store.path().to_path_buf();
    drop(store);
    let reader = Store::open_read_only(&path).unwrap();
    let opened = reader.open_run("r").unwrap();
    (reader, opened)
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
    events.push(started("fx", "b"));
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
    let mut events = two_effects_on_one_attempt();
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

/// A participant's attempts whose checkpoints interleave take the kept
/// way: `seat_count` and every rank are none before any rank work, so the
/// reader reads the run whole. The ruling supersedes ranking them by seq.
#[test]
fn interleaved_attempts_take_the_kept_way() {
    let (_dir, mut store) = store();
    let opened = interleaved(&mut store, 1);
    let events = store.load("r").unwrap();
    let expected = whole(&events, "fx", None);
    assert_eq!(expected.len(), 3);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        None,
        "the two spans overlap, so the count is kept"
    );
    for rank in 0..=expected.len() as u64 {
        assert_eq!(
            store.seat_seq_at("r", &opened, ("fx", None), rank).unwrap(),
            None,
            "rank {rank}"
        );
    }
}

/// The participant `seat`'s count and every rank through `rows` are none:
/// the kept way an interleaved participant takes.
fn kept_way(store: &Store, opened: &Opened, seat: (&str, Option<&str>), rows: usize) {
    assert_eq!(
        store.seat_count("r", opened, seat).unwrap(),
        None,
        "{seat:?}: the count"
    );
    for rank in 0..=rows as u64 {
        assert_eq!(
            store.seat_seq_at("r", opened, seat, rank).unwrap(),
            None,
            "{seat:?}: rank {rank}"
        );
    }
}

/// H1's journal with no fault: the member's two attempts interleave, so
/// the count and every rank take the kept way.
#[test]
fn h1s_interleaved_journal_takes_the_kept_way() {
    let (_dir, mut store) = store();
    assert_kept(&mut store, h1_events(), ("fx", Some("m")), 4);
}

/// M1's own journal: attempt `a`'s span reaches past attempt `b`'s and
/// across another effect's start, so the spans interleave and take the kept
/// way.
#[test]
fn m1s_own_interleaved_journal_takes_the_kept_way() {
    let (_dir, mut store) = store();
    let mut events = two_effects_on_one_attempt();
    events.push(checked("gx", "a", None, 1));
    events.push(checked("fx", "a", None, 3));
    events.push(started("fx", "b"));
    events.push(checked("fx", "b", None, 1));
    events.push(checked("fx", "a", None, 4));
    assert_kept(&mut store, events, ("fx", None), 5);
}

/// M1's member journal: the member's attempt `a` span reaches past attempt
/// `b`'s and across another effect's eight rows, so the spans interleave
/// and take the kept way.
#[test]
fn m1s_member_interleaved_journal_takes_the_kept_way() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend(turns("fx", "a", Some("m"), 3));
    events.push(started("fx", "b"));
    events.push(checked("fx", "b", Some("m"), 1));
    events.extend(asked_and_started("gx", "a"));
    events.extend(turns("gx", "a", Some("m"), 8));
    events.push(checked("fx", "a", Some("m"), 4));
    assert_kept(&mut store, events, ("fx", Some("m")), 5);
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

/// M3: the journal's spans interleave, so the final reach takes the kept
/// way: the count and every rank are none, and the interleaved search is no
/// longer run.
#[test]
fn the_interleaved_final_reach_takes_the_kept_way() {
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
    assert_kept(&mut store, events, ("fx", Some("m")), 5);
}

/// M4: a journal whose spans interleave takes the kept way, so a run of
/// NULL ordinals inside it is never read as a place or as an SQL type
/// error: the count and every rank are none.
#[test]
fn a_search_over_null_numbers_takes_the_kept_way() {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.push(checked("fx", "a", None, 1));
    events.push(started("fx", "b"));
    events.extend(turns("fx", "b", None, 9));
    events.push(checked("fx", "a", None, 2));
    let opened = journal(&mut store, events);
    let events = store.load("r").unwrap();
    let rows = whole(&events, "fx", None);
    assert_eq!(rows.len(), 11);
    kept_way(&store, &opened, ("fx", None), rows.len());
    // A NULL inside the interleaved spans is still none, never an
    // InvalidColumnType: the kept way reads no ordinal at all.
    let own = checkpoint_seqs(&store, None);
    still_to_number(&store, &own[2..=8]);
    kept_way(&store, &opened, ("fx", None), rows.len());
}

/// The few steps a B-tree one level deeper takes at 20,000 rows: a
/// complete read may differ by this much and still not scale with the
/// stream. The chief's honest trace moved 4,958 to 5,294 steps, so 500
/// names that one level and no scan.
#[cfg(feature = "test-support")]
const SLACK: i64 = 500;

/// Assert the measurements differ by no more than [`SLACK`], naming the
/// values when they do.
#[cfg(feature = "test-support")]
fn bounded(what: &str, sizes: &[i64]) {
    let low = *sizes.iter().min().unwrap();
    let high = *sizes.iter().max().unwrap();
    assert!(
        high - low <= SLACK,
        "{what}: the steps scale with the stream: {sizes:?}"
    );
}

/// One complete production read's value and the VM steps every statement
/// it executes takes, by the test-only tally.
#[cfg(feature = "test-support")]
fn counted<T>(store: &mut Store, read: impl FnOnce(&Store) -> T) -> (T, i64) {
    crate::test_support::read_steps(store, read)
}

/// The participant `seat`'s five complete reads: its count, its last and
/// middle ranks, and the fetch's end and middle pages, each one whole call
/// with its transaction, refusal seeks and page included. `rows` is the
/// participant's whole count, `last` and `middle` the ranks read.
#[cfg(feature = "test-support")]
fn complete_reads(
    store: &mut Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
    rows: u64,
    last: u64,
    middle: u64,
) -> [i64; 5] {
    let (count, count_steps) = counted(store, |store| store.seat_count("r", opened, seat).unwrap());
    assert_eq!(count, Some(rows), "{seat:?}: the count");
    let (last_seq, last_steps) = counted(store, |store| {
        store.seat_seq_at("r", opened, seat, last).unwrap()
    });
    assert!(last_seq.is_some(), "{seat:?}: the last rank reads");
    let (middle_seq, middle_steps) = counted(store, |store| {
        store.seat_seq_at("r", opened, seat, middle).unwrap()
    });
    let middle_seq = middle_seq.unwrap_or_else(|| panic!("{seat:?}: the middle rank reads"));
    let (end_page, end_page_steps) = counted(store, |store| {
        store
            .seat_read("r", opened, seat, None, |starts| {
                // The production page consumes every start its page names
                // before it makes a row (brokkr_view::page_rows); counting
                // them here makes the lazy read part of the measured call
                // (M2).
                assert!(starts.count() > 0, "{seat:?}: the end page reads a start");
                |event: &EventEnvelope| event.seq
            })
            .unwrap()
    });
    assert!(
        matches!(end_page, SeatRead::Current(..)),
        "{seat:?}: the end page reads"
    );
    let (middle_page, middle_page_steps) = counted(store, |store| {
        store
            .seat_from(
                "r",
                opened,
                seat,
                Some(middle_seq.saturating_sub(1)),
                &mut |starts| {
                    // The page's starts are read lazily; consume them so
                    // the measured call is the production fetch's (M2).
                    assert!(
                        starts.count() > 0,
                        "{seat:?}: the middle page reads a start"
                    );
                    |event: &EventEnvelope| event.seq
                },
            )
            .unwrap()
    });
    assert!(
        matches!(middle_page, SeatRead::Current(..)),
        "{seat:?}: the middle page reads"
    );
    [
        count_steps,
        last_steps,
        middle_steps,
        end_page_steps,
        middle_page_steps,
    ]
}

/// Assert each of the five complete reads steps the same at both sizes.
#[cfg(feature = "test-support")]
fn bounded_reads(shape: &str, sizes: &[[i64; 5]; 2]) {
    for (at, what) in [
        "count",
        "last rank",
        "middle rank",
        "end page",
        "middle page",
    ]
    .iter()
    .enumerate()
    {
        bounded(&format!("{shape}: {what}"), &[sizes[0][at], sizes[1][at]]);
    }
}

/// M1-of-round-three's N positive member checkpoints, read as the member's
/// participant: its five complete reads at `n`.
#[cfg(feature = "test-support")]
fn member_shape(n: u64) -> [i64; 5] {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend(turns("fx", "a", Some("m"), n));
    let opened = journal(&mut store, events);
    complete_reads(&mut store, &opened, ("fx", Some("m")), n, n - 1, n / 2)
}

/// 994o2's shape: the target's two checkpoints with N result-only records
/// of another member between them, on the target's attempt or another,
/// numbered as the writer leaves them (0) or with their numbering lost
/// (`null`): the NULL rows stand in the stale index, so the GAP seek's
/// participant key is what keeps them from the target's span (M2).
#[cfg(feature = "test-support")]
fn stale_shape(n: u64, attempt: &str, null: bool) -> [i64; 5] {
    let (_dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.push(checked("fx", "a", Some("target"), 1));
    events.extend(results("fx", attempt, "m", n));
    events.push(checked("fx", "a", Some("target"), 3));
    let opened = journal(&mut store, events);
    let (mut store, opened) = if null {
        let updated = store
            .conn
            .execute(
                "UPDATE event_index SET stream_ordinal = NULL
                 WHERE run_id = 'r' AND attempt_id = ?1 AND member = 'm'
                   AND stream_ordinal = 0",
                rusqlite::params![attempt],
            )
            .unwrap();
        assert_eq!(updated as u64, n, "the N result rows are the fixture");
        reopen_read_only(store)
    } else {
        (store, opened)
    };
    complete_reads(&mut store, &opened, ("fx", Some("target")), 2, 1, 0)
}

/// A disjoint participant of three spans: one checkpoint on attempt `a`,
/// N on `b`, and one on `c`.
#[cfg(feature = "test-support")]
fn multi_span_shape(n: u64) -> [i64; 5] {
    let (_dir, mut store) = store();
    let opened = journal(&mut store, three_spans(n));
    let rows = n + 2;
    complete_reads(&mut store, &opened, ("fx", None), rows, rows - 1, 1 + n / 2)
}

/// The cost bound counts complete production reads: the member's N
/// checkpoints; 994o2's N rows of another member on the target's attempt
/// and on another, both as the writer leaves them and with their ordinals
/// lost to the stale index; and a disjoint three-span participant with N
/// rows in the middle span, all step the same at 2,000 and 20,000. A
/// statement measured on its own would miss the work a full read spends,
/// and a stale seek only its own empty plan would miss the rows another
/// participant's NULLs put in its index (M2).
#[test]
#[cfg(feature = "test-support")]
fn the_complete_reads_step_the_same_at_every_size() {
    bounded_reads(
        "the member's N checkpoints",
        &[member_shape(2_000), member_shape(20_000)],
    );
    for (shape, attempt, null) in [
        ("994o2's other member", "a", false),
        ("994o2's other member, NULL", "a", true),
        ("994o2's other attempt", "b", false),
        ("994o2's other attempt, NULL", "b", true),
    ] {
        bounded_reads(
            shape,
            &[
                stale_shape(2_000, attempt, null),
                stale_shape(20_000, attempt, null),
            ],
        );
    }
    bounded_reads(
        "the disjoint three-span participant",
        &[multi_span_shape(2_000), multi_span_shape(20_000)],
    );
}

/// M1: the tally counts the instructions of the statement the read runs,
/// at the values it binds. A member key holding a NUL is truncated by
/// SQLite's C trace to `m`, which finds no row; a tally that measured that
/// text again would report the missing lookup's work for both keys. The
/// real statement finds the NUL row, so the tally's two counts differ by
/// exactly the two statements' own `VmStep` counts.
#[test]
#[cfg(feature = "test-support")]
fn a_nul_key_is_tallied_at_its_actual_statement() {
    use rusqlite::{params, OptionalExtension, StatementStatus};

    const NUL: &str = "m\u{0}suffix";
    let (_dir, mut store) = store();
    let _opened = journal(&mut store, {
        let mut events = asked_and_started("fx", "a");
        events.extend(turns("fx", "a", Some(NUL), 3));
        events
    });
    let seq = checkpoint_seqs(&store, Some(NUL))[0];
    let mut direct = store.conn.prepare(&super::ordinal::BOUND).unwrap();
    let mut run = |member: &str| {
        direct.reset_status(StatementStatus::VmStep);
        let read: Option<Option<i64>> = direct
            .query_row(params!["r", "a", member, seq as i64], |row| row.get(0))
            .optional()
            .unwrap();
        (
            read,
            i64::from(direct.reset_status(StatementStatus::VmStep)),
        )
    };
    let (nul_direct, nul_vm) = run(NUL);
    let (missing_direct, missing_vm) = run("m");
    assert_eq!(nul_direct, Some(Some(1)), "the NUL key's row reads");
    assert_eq!(missing_direct, None, "the trace's truncated key reads none");
    drop(direct);
    let bound = |store: &Store, member: &str| {
        let mut statement = store.conn.prepare(&super::ordinal::BOUND).unwrap();
        let read: Option<Option<i64>> = statement
            .query_row(params!["r", "a", member, seq as i64], |row| row.get(0))
            .optional()
            .unwrap();
        read
    };
    let (_, nul_steps) = counted(&mut store, |store| bound(store, NUL));
    let (_, missing_steps) = counted(&mut store, |store| bound(store, "m"));
    assert_eq!(
        nul_steps - missing_steps,
        nul_vm - missing_vm,
        "the tally counts the NUL statement's instructions, not a lost trace"
    );
}

/// The journal `r` of one attempt's `turns` member checkpoints with `fault`
/// applied to its first index row, the writer closed and the journal
/// reopened read-only: the stated projection fault is the only one.
fn faulted_first(rows: u64, fault: &str) -> (tempfile::TempDir, Store, Opened) {
    let (dir, mut store) = store();
    let mut events = asked_and_started("fx", "a");
    events.extend(turns("fx", "a", Some("m"), rows));
    let _opened = journal(&mut store, events);
    let first = checkpoint_seqs(&store, Some("m"))[0];
    let sql = match fault {
        "missing" => format!("DELETE FROM event_index WHERE run_id = 'r' AND seq = {first}"),
        ordinal => format!(
            "UPDATE event_index SET stream_ordinal = {ordinal}
             WHERE run_id = 'r' AND seq = {first}"
        ),
    };
    store.conn.execute_batch(&sql).unwrap();
    let (reader, opened) = reopen_read_only(store);
    (dir, reader, opened)
}

/// H1: a span's first row is read by key, so a missing row, a NULL, a
/// negative and a positive-but-not-1 first all refuse the count and every
/// rank. The read-only open plants only the stated projection fault.
#[test]
fn a_missing_or_corrupt_first_bound_refuses_the_disjoint_walk() {
    for fault in ["missing", "NULL", "-1", "2"] {
        let (_dir, store, opened) = faulted_first(3, fault);
        for rank in [0u64, 1, 2] {
            assert_eq!(
                store
                    .seat_seq_at("r", &opened, ("fx", Some("m")), rank)
                    .unwrap(),
                None,
                "{fault}: rank {rank}"
            );
        }
        assert_eq!(
            store.seat_count("r", &opened, ("fx", Some("m"))).unwrap(),
            None,
            "{fault}: the count"
        );
    }
}

/// H1's interleaved journal refuses through the kept way whatever row its
/// first span loses: the fault never reaches the deleted search.
#[test]
fn h1s_faulted_interleaved_journal_refuses_through_the_kept_way() {
    for fault in ["missing", "-1"] {
        let (_dir, mut store) = store();
        let _opened = journal(&mut store, h1_events());
        let sql = match fault {
            "missing" => "DELETE FROM event_index WHERE run_id = 'r' AND seq = 5".to_string(),
            ordinal => format!(
                "UPDATE event_index SET stream_ordinal = {ordinal}
                 WHERE run_id = 'r' AND seq = 5"
            ),
        };
        store.conn.execute_batch(&sql).unwrap();
        let (reader, opened) = reopen_read_only(store);
        kept_way(&reader, &opened, ("fx", Some("m")), 4);
    }
}

/// A disjoint multi-span participant with no fault still ranks every row,
/// and a passed span's last row is read by key: deleting it refuses a rank
/// that passes that span.
#[test]
fn a_disjoint_multi_span_participant_ranks_every_row() {
    let (_dir, mut store) = store();
    let opened = journal(&mut store, three_spans(9));
    let events = store.load("r").unwrap();
    let expected = whole(&events, "fx", None);
    assert_eq!(expected.len(), 11);
    assert_eq!(
        store.seat_count("r", &opened, ("fx", None)).unwrap(),
        Some(11)
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
    // The middle span's last row is a bound a rank in the last span
    // passes: deleting it refuses that rank, never reading it as a count.
    let last_of_middle = expected[9];
    store
        .conn
        .execute(
            "DELETE FROM event_index WHERE run_id = 'r' AND seq = ?1",
            [last_of_middle as i64],
        )
        .unwrap();
    crate::test_support::lags(&mut store, &["r"]);
    assert_eq!(
        store.seat_seq_at("r", &opened, ("fx", None), 10).unwrap(),
        None,
        "the last rank passes the deleted middle bound"
    );
}
