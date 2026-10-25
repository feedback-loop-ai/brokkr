//! The seat view's read (decision 0089 ruling 7): at the head the run's
//! opening verified, a participant's checkpoints a page of 50 at a time,
//! newest first, across its attempts, each page below the cursor the one
//! before handed over, with the start each attempt on the page last
//! names; and each shape its streams cannot stand for, said so.

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::seat::row::SeatRow;
use rusqlite::params;
use serde_json::{json, Value};

use super::{reaches, Stream, ATTEMPT, FIRST, LAST, LEAST, MEMBER, NEXT, SINCE};
use crate::event_index::STREAM;
use crate::test_support::plan;
use crate::tests::store;
use crate::{Opened, SeatRead, Store};

/// An event of run `r`: its type and its payload.
type Event = (EventType, Value);

fn asked(effect: &str) -> Event {
    let payload = json!({"effect_id": effect, "seat": "worker", "phase": "work"});
    (EventType::EffectRequested, payload)
}

fn started(effect: &str, attempt: &str) -> Event {
    let payload = json!({"effect_id": effect, "attempt_id": attempt});
    (EventType::EffectStarted, payload)
}

/// Turn `turn` of `effect`'s attempt `attempt`, its member's where it
/// names one.
fn turn(effect: &str, attempt: &str, member: Option<&str>, turn: u64) -> Event {
    let mut checkpoint = json!({"step": "seat-turn", "turn": turn});
    if let Some(member) = member {
        checkpoint["member"] = member.into();
    }
    let payload = json!({"effect_id": effect, "attempt_id": attempt, "checkpoint": checkpoint});
    (EventType::EffectCheckpointed, payload)
}

/// Run `r`, started and its phase entered, then `events`, opened.
fn opened(store: &mut Store, events: Vec<Event>) -> Opened {
    let manifest = json!({});
    store.create_run("r", "a run", "self", &manifest).unwrap();
    let mut begun = vec![
        (EventType::RunStarted, json!({"manifest": manifest})),
        (EventType::PhaseEntered, json!({"phase": "work"})),
    ];
    begun.extend(events);
    let appended = begun.into_iter().try_for_each(|(kind, body)| {
        let appended = store.append_next("r", kind, body, None, None);
        appended.map(drop)
    });
    appended.unwrap();
    store.open_run("r").unwrap()
}

/// A page of `seat`'s checkpoints below `before`, each by its seq.
fn page(
    store: &Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
    before: Option<u64>,
) -> SeatRead<u64> {
    let read = store.seat_read("r", opened, seat, before, |_| {
        |event: &EventEnvelope| event.seq
    });
    read.unwrap()
}

/// A seat retried, its first attempt's 60 turns, then its retry's member
/// `m`'s 10 and its own 50: seqs 5 to 64, then 66 to 75 and 76 on.
fn retried() -> Vec<Event> {
    let mut events = vec![asked("fx"), started("fx", "a")];
    events.extend((1..=60).map(|at| turn("fx", "a", None, at)));
    events.push(started("fx", "b"));
    events.extend((1..=10).map(|at| turn("fx", "b", Some("m"), at)));
    events.extend((1..=50).map(|at| turn("fx", "b", None, at)));
    events
}

#[test]
fn a_participant_s_checkpoints_are_read_a_page_at_a_time_newest_first() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, retried());
    assert_eq!(opened.seq, 125);
    let whole = store.load("r").unwrap();
    let theirs = |member: Option<&str>| -> Vec<u64> {
        let named = whole.iter().filter(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["checkpoint"]["member"].as_str() == member
        });
        named.map(|event| event.seq).rev().collect()
    };
    let (own, members) = (theirs(None), theirs(Some("m")));
    assert_eq!((own.len(), members.len()), (110, 10));
    // Each page is asked below the cursor the one before handed over,
    // until a page hands none; the seat's own pages hold both attempts'.
    let mut pages = Vec::new();
    let mut before = None;
    loop {
        let SeatRead::Current(seqs, next) = page(&store, &opened, ("fx", None), before) else {
            panic!("r's seat read as kept");
        };
        assert_eq!(next, (seqs.len() == 50).then(|| *seqs.last().unwrap()));
        pages.push(seqs);
        if next.is_none() {
            break;
        }
        before = next;
    }
    let lengths: Vec<usize> = pages.iter().map(Vec::len).collect();
    assert_eq!(lengths, [50, 50, 10]);
    assert_eq!(pages.concat(), own);
    let member = page(&store, &opened, ("fx", Some("m")), None);
    assert_eq!(member, SeatRead::Current(members, None));
    // Below its oldest checkpoint, and for a participant no row keeps, a
    // page is empty.
    let empty = SeatRead::Current(Vec::new(), None);
    assert_eq!(page(&store, &opened, ("fx", None), Some(5)), empty);
    assert_eq!(page(&store, &opened, ("gx", None), None), empty);
}

/// The starts `seat`'s page below `before` is handed over, by seq, beside
/// each checkpoint on it.
fn starts_of(
    store: &Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
    before: Option<u64>,
) -> SeatRead<(u64, Vec<u64>)> {
    let read = store.seat_read("r", opened, seat, before, |starts| {
        let starts: Vec<u64> = starts.iter().map(|start| start.seq).collect();
        move |event: &EventEnvelope| (event.seq, starts.clone())
    });
    read.unwrap()
}

#[test]
fn each_page_is_handed_over_the_start_each_of_its_attempts_last_names() {
    let (_dir, mut store) = store();
    let mut events = retried();
    // The effect asked again and attempt `a` started again under the new
    // request, whose rows hold no checkpoint, and another effect's start
    // naming it: the page of `a` is handed the later start of its own
    // effect alone, a row's boundary read by its effect and attempt.
    events.extend([
        asked("fx"),
        started("fx", "a"),
        asked("gx"),
        started("gx", "a"),
    ]);
    let opened = opened(&mut store, events);
    let SeatRead::Current(page, None) = starts_of(&store, &opened, ("fx", Some("m")), None) else {
        panic!("r's member read as kept");
    };
    assert_eq!(page[0], (75, vec![65]));
    let below = starts_of(&store, &opened, ("fx", None), Some(6));
    assert_eq!(below, SeatRead::Current(vec![(5, vec![127])], None));
    // A page across both attempts is handed both starts, each once.
    let across = starts_of(&store, &opened, ("fx", None), Some(86));
    let SeatRead::Current(across, Some(_)) = across else {
        panic!("r's seat read as kept");
    };
    assert_eq!(across[0], (85, vec![127, 65]));
    // A page of an attempt whose rows keep no start reads the kept way.
    let unstarted = vec![asked("fx"), turn("fx", "a", None, 1)];
    assert_eq!(kept(unstarted, ("fx", None)), SeatRead::Kept);
}

/// The read is the run view's at the head its opening verified: a run that
/// moved past it reads the kept way.
#[test]
fn a_run_past_its_opening_is_said_so() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, retried());
    let (event_type, payload) = turn("fx", "b", None, 51);
    store
        .append_next("r", event_type, payload, None, None)
        .unwrap();
    assert_eq!(page(&store, &opened, ("fx", None), None), SeatRead::Kept);
    let reopened = store.open_run("r").unwrap();
    let SeatRead::Current(seqs, _) = page(&store, &reopened, ("fx", None), None) else {
        panic!("r's seat read as kept");
    };
    assert_eq!(seqs[0], 126);
}

/// `events`' seat `seat` read at the head: what no writer's streams stand
/// for reads the kept way.
fn kept(events: Vec<Event>, seat: (&str, Option<&str>)) -> SeatRead<u64> {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, events);
    page(&store, &opened, seat, None)
}

#[test]
fn what_a_stream_cannot_stand_for_is_read_the_kept_way() {
    let begun = || vec![asked("fx"), started("fx", "a"), turn("fx", "a", None, 1)];
    // A checkpoint naming no attempt, which no stream keys.
    let mut attemptless = begun();
    let (event_type, mut payload) = turn("fx", "a", None, 2);
    payload.as_object_mut().unwrap().remove("attempt_id");
    attemptless.push((event_type, payload));
    // Two of the seat's rows, its effect asked again and its attempt
    // started again, whose checkpoints are keyed by one attempt.
    let mut shared = begun();
    shared.extend([asked("fx"), started("fx", "a"), turn("fx", "a", None, 2)]);
    // Another effect's checkpoint keyed by the seat's attempt, within its
    // stream; and one of an effect never asked.
    let mut another = begun();
    another.extend([
        asked("gx"),
        started("gx", "a"),
        turn("gx", "a", None, 1),
        turn("fx", "a", None, 2),
    ]);
    let mut unasked = begun();
    unasked.extend([turn("hx", "a", None, 1), turn("fx", "a", None, 2)]);
    // A record naming its member by its result alone: the seat's rows read
    // it as the seat's, its index row as the member's.
    let resulted = (
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}}),
    );
    let mut member_s = begun();
    member_s.extend([
        turn("fx", "a", Some("m"), 1),
        resulted.clone(),
        turn("fx", "a", Some("m"), 2),
    ]);
    let mut seat_s = begun();
    seat_s.push(resulted.clone());
    // The same record between two of the seat's own checkpoints, where
    // neither end of its stream tells; and a member's own checkpoint there,
    // which only the bodies tell from it.
    let mut interior = begun();
    interior.extend([
        turn("fx", "a", Some("m"), 1),
        resulted,
        turn("fx", "a", None, 2),
    ]);
    let mut membered = begun();
    membered.extend([turn("fx", "a", Some("m"), 1), turn("fx", "a", None, 2)]);
    let shapes = [
        ("attemptless", attemptless, None),
        ("shared", shared, None),
        ("another's", another, None),
        ("unasked", unasked, None),
        ("member's", member_s, Some("m")),
        ("seat's", seat_s, None),
        ("interior", interior, None),
        ("membered", membered, None),
    ];
    for (name, events, member) in shapes {
        assert_eq!(kept(events, ("fx", member)), SeatRead::Kept, "{name}");
    }
    assert_eq!(
        kept(begun(), ("fx", None)),
        SeatRead::Current(vec![5], None)
    );
}

/// A member's checkpoint of the seat's attempt is looked for within the
/// reach of the page it would stand on, by its index row alone, whether it
/// names its member by its result, as the seat's rows read the seat's own,
/// or by its checkpoint: a page above it reads at its head, and the page
/// whose reach holds it reads the kept way.
#[test]
fn a_member_s_checkpoint_is_looked_for_within_the_page_s_reach() {
    let resulted = json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}});
    let membered = turn("fx", "a", Some("m"), 1);
    for (name, record) in [
        ("resulted", (EventType::EffectCheckpointed, resulted)),
        ("membered", membered),
    ] {
        let (_dir, mut store) = store();
        let mut events = vec![asked("fx"), started("fx", "a")];
        events.extend((1..=60).map(|at| turn("fx", "a", None, at)));
        // Seq 7, between the seat's second and third turns.
        events.insert(4, record);
        let opened = opened(&mut store, events);
        let SeatRead::Current(first, Some(16)) = page(&store, &opened, ("fx", None), None) else {
            panic!("{name}: r's first page read as kept");
        };
        assert_eq!((first[0], first.len()), (65, 50), "{name}");
        let second = page(&store, &opened, ("fx", None), Some(16));
        assert_eq!(second, SeatRead::Kept, "{name}");
    }
}

/// The read holds its participant's effect's seat rows alone, read by
/// key: a row of another effect edited past its digest leaves the page
/// read at its head, and the edited effect's reads the kept way; a row
/// gone from its run's places, or the seat tables gone, reads every
/// participant the kept way.
#[test]
fn a_page_reads_its_own_effect_s_seat_rows_alone() {
    let (_dir, mut store) = store();
    let mut events = vec![asked("fx"), started("fx", "a"), turn("fx", "a", None, 1)];
    events.extend([asked("gx"), started("gx", "b"), turn("gx", "b", None, 1)]);
    let opened = opened(&mut store, events);
    let edit = |sql: &str| store.conn.execute_batch(sql).unwrap();
    edit("UPDATE seat_summary SET digest = upper(digest) WHERE effect_id = 'gx'");
    let read = |effect| page(&store, &opened, (effect, None), None);
    assert_eq!(
        [read("fx"), read("gx")],
        [SeatRead::Current(vec![5], None), SeatRead::Kept]
    );
    edit("DELETE FROM seat_summary WHERE effect_id = 'fx' AND attempt_id IS NULL");
    assert_eq!([read("fx"), read("gx")], [SeatRead::Kept, SeatRead::Kept]);
    crate::test_support::unseat(&store.conn).unwrap();
    assert_eq!(read("fx"), SeatRead::Kept);
    crate::test_support::lags(&mut store, &["r"]);
}

/// A stream is its row's where it begins at the row's last checkpoint,
/// when the read starts there, and ends at its first, when no page cuts
/// it.
#[test]
fn a_stream_is_its_row_s_where_it_begins_and_ends_at_the_row_s_checkpoints() {
    let stream = |last| Stream {
        attempt: "a".to_string(),
        first: 3,
        last,
        since: None,
    };
    // A page and one more, cut above the row's first checkpoint.
    let cut: Vec<u64> = (10..=60).rev().collect();
    let cases: [(&[u64], u64, u64, bool); 7] = [
        (&[9, 5, 3], 9, 9, true),
        (&[5, 3], 9, 9, false),
        (&[5, 3], 9, 6, true),
        (&[9, 5], 9, 9, false),
        (&[], 9, 9, false),
        (&cut, 60, 60, true),
        (&cut, 61, 60, true),
    ];
    for (found, last, upper, reached) in cases {
        let read = reaches(found, &stream(last), upper);
        assert_eq!(
            read, reached,
            "{found:?} of a row ending at {last}, from {upper}"
        );
    }
}

/// A row's stream is read by the columns `SeatRow::COLUMNS` names at
/// these places.
#[test]
fn a_row_s_stream_is_read_by_its_effect_attempt_member_and_checkpoints() {
    let named = [ATTEMPT, MEMBER, SINCE, FIRST, LAST].map(|at| SeatRow::COLUMNS[at].0);
    assert_eq!(
        named,
        ["attempt_id", "member", "since", "first_seq", "last_seq"]
    );
}

/// A stream, an attempt's members and a member's checkpoint within a
/// page's reach each search the stream index between their bounds, with no
/// scan, no temporary B-tree and no envelope read.
#[test]
fn a_stream_and_its_members_search_the_stream_index() {
    let (_dir, store) = store();
    let none: Option<&str> = None;
    let stream = "SEARCH event_index USING INDEX event_index_stream \
         (run_id=? AND attempt_id=? AND member=? AND seq>? AND seq<?)";
    let members = "SEARCH event_index USING COVERING INDEX event_index_stream \
         (run_id=? AND attempt_id=? AND member>?)";
    let plan = |sql: &str, bound: &[&dyn rusqlite::ToSql]| plan(&store, sql, bound).unwrap();
    assert_eq!(
        [
            plan(&STREAM, params!["r", "a", none, 1, 9, 51]),
            plan(&STREAM, params!["r", "a", "m", 1, 9, 1]),
        ],
        [[stream], [stream]]
    );
    assert_eq!(
        [
            plan(LEAST, params!["r", "a", ""]),
            plan(NEXT, params!["r", "a", "m"])
        ],
        [[members], [members]]
    );
}
