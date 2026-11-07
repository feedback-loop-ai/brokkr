//! The seat view's read (decision 0089 ruling 7): at the head the run's
//! opening verified, a participant's checkpoints a page of 50 at a time,
//! newest first, across its attempts, each page below the cursor the one
//! before handed over, with the start each attempt on the page last
//! names; and each shape its streams cannot stand for, said so.

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::seat::row::SeatRow;
use rusqlite::params;
use serde_json::{json, Value};

use super::{reaches, Stream, ATTEMPT, FIRST, LAST, LEAST, MEMBER, NEXT, SINCE, STREAMS};
use crate::event_index::STREAM;
use crate::test_support::{plan, unindex};
use crate::tests::{plant_unfenced, store};
use crate::{Opened, SeatRead, Store, StoreError};

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

/// A seat asked and started, then ten of its own turns: seqs 5 to 14.
fn ten() -> Vec<Event> {
    let mut events = vec![asked("fx"), started("fx", "a")];
    events.extend((1..=10).map(|at| turn("fx", "a", None, at)));
    events
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

/// A page of `seat`'s checkpoints after `after`, oldest first, each by its
/// seq.
fn from_page(
    store: &Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
    after: Option<u64>,
) -> SeatRead<u64> {
    let read = store.seat_from("r", opened, seat, after, &mut |_| {
        |event: &EventEnvelope| event.seq
    });
    read.unwrap()
}

/// The first two forward pages of `seat`, after 0 and after its first
/// page's cursor: a full page and the page behind it, both read at
/// `opened`.
fn two_forward_pages(
    store: &Store,
    opened: &Opened,
    seat: (&str, Option<&str>),
) -> [SeatRead<u64>; 2] {
    [
        from_page(store, opened, seat, None),
        from_page(store, opened, seat, Some(54)),
    ]
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

/// `retried` opened, then moved past its opening by one checkpoint of
/// its retry: the head the read no longer stands at.
fn past_opening(store: &mut Store) -> Opened {
    let opened = opened(store, retried());
    let (event_type, payload) = turn("fx", "b", None, 51);
    store
        .append_next("r", event_type, payload, None, None)
        .unwrap();
    opened
}

/// The forward page `after` of the run at `opened` once the index row at
/// `seq` is deleted: the read the index can no longer stand behind.
fn kept_after_index_delete(dir: &tempfile::TempDir, opened: &Opened, seq: u64) -> SeatRead<u64> {
    let path = dir.path().join("forge.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    let gone = format!("DELETE FROM event_index WHERE run_id = 'r' AND seq = {seq}");
    assert_eq!(conn.execute(&gone, []).unwrap(), 1);
    drop(conn);
    let store = Store::open_read_only(&path).unwrap();
    from_page(&store, opened, ("fx", None), Some(64))
}

/// The seqs of `whole`'s checkpoints `member` recorded, oldest first.
fn checkpoint_seqs(whole: &[EventEnvelope], member: Option<&str>) -> Vec<u64> {
    let named = whole.iter().filter(|event| {
        event.event_type == EventType::EffectCheckpointed
            && event.payload["checkpoint"]["member"].as_str() == member
    });
    named.map(|event| event.seq).collect()
}

/// `seat`'s pages from `store`, newest first below the cursor until a page
/// hands none: the pages and their cursors, each cursor the page's
/// outermost seq where it is full.
fn pages(store: &Store, opened: &Opened, seat: (&str, Option<&str>)) -> Vec<Vec<u64>> {
    let mut pages = Vec::new();
    let mut cursor = None;
    loop {
        let read = page(store, opened, seat, cursor);
        let SeatRead::Current(seqs, next) = read else {
            panic!("r's seat read as kept");
        };
        assert_eq!(next, (seqs.len() == 50).then(|| *seqs.last().unwrap()));
        pages.push(seqs);
        if next.is_none() {
            break;
        }
        cursor = next;
    }
    pages
}

#[test]
fn a_participant_s_checkpoints_are_read_a_page_at_a_time_newest_first() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, retried());
    assert_eq!(opened.seq, 125);
    let whole = store.load("r").unwrap();
    let mut own = checkpoint_seqs(&whole, None);
    let mut members = checkpoint_seqs(&whole, Some("m"));
    assert_eq!((own.len(), members.len()), (110, 10));
    own.reverse();
    members.reverse();
    // Each page is asked below the cursor the one before handed over,
    // until a page hands none; the seat's own pages hold both attempts'.
    let pages = pages(&store, &opened, ("fx", None));
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
        let starts: Vec<u64> = starts.map(|start| start.seq).collect();
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
    // A page across both attempts is handed both starts, each once, in
    // seq order.
    let across = starts_of(&store, &opened, ("fx", None), Some(86));
    let SeatRead::Current(across, Some(_)) = across else {
        panic!("r's seat read as kept");
    };
    assert_eq!(across[0], (85, vec![65, 127]));
    // A page of an attempt whose rows keep no start reads the kept way.
    let unstarted = vec![asked("fx"), turn("fx", "a", None, 1)];
    assert_eq!(kept(unstarted, ("fx", None)), SeatRead::Kept);
}

/// `attempts` attempts of a seat, each started and its first turn taken,
/// then each one's second turn, so every attempt's stream reaches the
/// newest checkpoints and none can be dropped.
fn interleaved(attempts: usize) -> Vec<Event> {
    let named: Vec<String> = (0..attempts).map(|at| format!("a{at}")).collect();
    let mut events = vec![asked("fx")];
    for attempt in &named {
        events.extend([started("fx", attempt), turn("fx", attempt, None, 1)]);
    }
    events.extend(named.iter().map(|attempt| turn("fx", attempt, None, 2)));
    events
}

/// A page keeps at most `STREAMS` streams that can reach it: one more
/// reads the kept way, and as many reads the page at its head.
#[test]
fn a_page_reached_by_more_than_its_streams_reads_the_kept_way() {
    let read = [STREAMS, STREAMS + 1].map(|attempts| {
        let (_dir, mut store) = store();
        let opened = opened(&mut store, interleaved(attempts));
        page(&store, &opened, ("fx", None), None)
    });
    // Seq 3 asks the seat; each attempt's start and first turn follow, then
    // each one's second turn, the newest at 3 + 3 a stream.
    let newest: Vec<u64> = (0..50).map(|at| 3 + 3 * STREAMS as u64 - at).collect();
    let next = Some(newest[49]);
    assert_eq!(read, [SeatRead::Current(newest, next), SeatRead::Kept]);
}

/// A start the page cannot read is the read's refusal, never a page
/// without it.
#[test]
fn a_start_the_page_cannot_read_is_refused() {
    let (dir, mut store) = store();
    let opened = opened(&mut store, retried());
    let conn = rusqlite::Connection::open(dir.path().join("forge.db")).unwrap();
    conn.execute_batch("DROP TRIGGER events_append_only_delete")
        .unwrap();
    let gone = "DELETE FROM events WHERE run_id = 'r' AND seq = 65";
    assert_eq!(conn.execute(gone, []).unwrap(), 1);
    let read = store.seat_read("r", &opened, ("fx", None), None, |starts| {
        let starts: Vec<u64> = starts.map(|start| start.seq).collect();
        move |_: &EventEnvelope| starts.clone()
    });
    assert!(
        matches!(
            read,
            Err(StoreError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
        ),
        "{read:?}"
    );
    crate::test_support::lags(&mut store, &["r"]);
}

/// The read is the run view's at the head its opening verified: a run that
/// moved past it reads the kept way.
#[test]
fn a_run_past_its_opening_is_said_so() {
    let (_dir, mut store) = store();
    let opened = past_opening(&mut store);
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
        sited: true,
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

/// The forward page is a page and the lookahead before it is cut: a
/// member's record at the boundary is examined, so the page that would
/// step over it reads the kept way (H1).
#[test]
fn the_forward_page_checks_the_record_at_its_boundary() {
    let (_dir, mut store) = store();
    let mut events = vec![asked("fx"), started("fx", "a")];
    events.extend((1..=50).map(|at| turn("fx", "a", None, at)));
    events.push((
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m"}}),
    ));
    events.extend((51..=100).map(|at| turn("fx", "a", None, at)));
    let opened = opened(&mut store, events);
    assert_eq!(opened.seq, 105, "the record stands at seq 55");
    assert_eq!(
        [
            from_page(&store, &opened, ("fx", None), None),
            from_page(&store, &opened, ("fx", None), Some(54)),
        ],
        [SeatRead::Kept, SeatRead::Kept],
        "the page and the page resumed past its cursor"
    );
    assert_eq!(
        page(&store, &opened, ("fx", None), None),
        SeatRead::Kept,
        "the backward reader refuses the same journal"
    );
}

/// A re-opened attempt is one stream, its two site rows' checkpoints read
/// together: the first page is a full page with another behind it, never
/// the kept way, whatever the sites' rows hold (decision 0089 ruling 7).
#[test]
fn a_reopened_attempt_s_sites_are_one_stream_that_pages() {
    let (_dir, mut store) = store();
    let sited = |attempt: &str| {
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": attempt,
                   "provenance": [{"member": null, "provider": "anthropic"}]}),
        )
    };
    let mut events = vec![asked("fx"), sited("a")];
    events.extend((1..=50).map(|at| turn("fx", "a", None, at)));
    events.push(sited("a"));
    events.push(turn("fx", "a", None, 51));
    let opened = opened(&mut store, events);
    let pages = two_forward_pages(&store, &opened, ("fx", None));
    assert_eq!(pages[0], SeatRead::Current((5..=54).collect(), Some(54)));
    assert_eq!(pages[1], SeatRead::Current(vec![56], None));
}

/// A plain forward run is still paged, not refused, at 100 checkpoints:
/// two full pages and the cursor between them (H1's control).
#[test]
fn a_hundred_checkpoints_page_forward_without_a_refusal() {
    let (_dir, mut store) = store();
    let mut events = vec![asked("fx"), started("fx", "a")];
    events.extend((1..=100).map(|at| turn("fx", "a", None, at)));
    let opened = opened(&mut store, events);
    assert_eq!(
        two_forward_pages(&store, &opened, ("fx", None)),
        [
            SeatRead::Current((5..=54).collect(), Some(54)),
            SeatRead::Current((55..=104).collect(), None)
        ]
    );
}

/// The forward page stands at the head its opening verified: an older
/// writer's append past it leaves the seat rows where they stood, and the
/// page reads the kept way rather than the stale rows (H2).
#[test]
fn a_forward_page_reads_at_the_head_its_opening_verified() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, ten());
    assert_eq!(opened.seq, 14);
    let (event_type, payload) = turn("fx", "a", None, 11);
    plant_unfenced(&mut store, "r", event_type, payload);
    assert_eq!(
        from_page(&store, &opened, ("fx", None), None),
        SeatRead::Kept
    );
}

/// A forward page falls back when the event index is gone, as the
/// backward page does, rather than failing on the missing table (H2).
#[test]
fn a_forward_page_falls_back_when_the_index_is_gone() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, ten());
    let path = store.path().to_path_buf();
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    unindex(&conn).unwrap();
    drop(conn);
    let store = Store::open_read_only(&path).unwrap();
    assert_eq!(
        from_page(&store, &opened, ("fx", None), None),
        SeatRead::Kept
    );
}

/// The forward page is read inside one read transaction, as the backward
/// page is: the row-maker sees the connection out of autocommit (H2).
#[test]
fn a_forward_page_is_one_read_transaction() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, ten());
    let mut saw = None;
    let read = store.seat_from("r", &opened, ("fx", None), None, &mut |_| {
        saw = Some(!store.conn.is_autocommit());
        |event: &EventEnvelope| event.seq
    });
    assert!(matches!(read, Ok(SeatRead::Current(..))), "{read:?}");
    assert_eq!(
        saw,
        Some(true),
        "the row-maker stands in a read transaction"
    );
}

/// A forward page skips an attempt wholly below its cursor and reads the
/// one that reaches past it: the streams are filtered above the cursor,
/// never read below it.
#[test]
fn a_forward_page_skips_an_attempt_below_its_cursor() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, retried());
    assert_eq!(opened.seq, 125);
    assert_eq!(
        from_page(&store, &opened, ("fx", None), Some(64)),
        SeatRead::Current((76..=125).collect(), None),
        "the retry's own 50 rows, oldest first from its first checkpoint"
    );
}

/// A cursor at the stream's newest checkpoint still reads a page: the
/// streams reach past it, and their rows stand past it, so the page is
/// empty rather than no page at all.
#[test]
fn a_forward_page_after_the_last_row_is_empty() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, retried());
    assert_eq!(
        from_page(&store, &opened, ("fx", None), Some(125)),
        SeatRead::Current(Vec::new(), None)
    );
}

/// A member's forward page does not ask the seat's own interleave check:
/// its stream is already the member's, and a member's page is exact.
#[test]
fn a_member_s_forward_page_is_not_interleaved() {
    let (_dir, mut store) = store();
    let opened = opened(&mut store, retried());
    assert_eq!(
        from_page(&store, &opened, ("fx", Some("m")), None),
        SeatRead::Current((66..=75).collect(), None)
    );
}

/// A run that moved past its opening is read the kept way forward too:
/// the seat rows and its head no longer stand at the opening (H2).
#[test]
fn a_forward_page_past_its_opening_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let opened = past_opening(&mut store);
    assert_eq!(
        from_page(&store, &opened, ("fx", None), None),
        SeatRead::Kept
    );
}

/// A forward page whose index no longer holds its stream's last row does
/// not stand for its row: the read takes the kept way, never a short page
/// (H2, M6).
#[test]
fn a_forward_index_missing_its_last_row_reads_the_kept_way() {
    let (dir, mut store) = store();
    let opened = opened(&mut store, retried());
    assert_eq!(opened.seq, 125);
    drop(store);
    assert_eq!(kept_after_index_delete(&dir, &opened, 125), SeatRead::Kept);
}

/// A checkpoint whose attempt never started is not a stream the rows
/// stand for: the forward read takes the kept way rather than a page
/// whose start it cannot name.
#[test]
fn a_checkpoint_without_its_start_reads_the_kept_way() {
    let (_dir, mut store) = store();
    let mut events = vec![asked("fx"), started("fx", "a")];
    events.extend((1..=10).map(|at| turn("fx", "a", None, at)));
    events.push(turn("fx", "b", None, 1));
    let opened = opened(&mut store, events);
    assert_eq!(
        from_page(&store, &opened, ("fx", None), None),
        SeatRead::Kept
    );
}

/// A seat row whose digest is not its values does not stand for its
/// stream: the forward read takes the kept way before it pages (M6).
#[test]
fn a_forward_row_whose_digest_is_not_its_reads_the_kept_way() {
    let (dir, mut store) = store();
    let opened = opened(&mut store, ten());
    drop(store);
    let conn = rusqlite::Connection::open(dir.path().join("forge.db")).unwrap();
    let corrupt = "UPDATE seat_summary SET digest = 'not its values' WHERE run_id = 'r'";
    assert!(conn.execute(corrupt, []).unwrap() > 0);
    drop(conn);
    let store = Store::open_read_only(&dir.path().join("forge.db")).unwrap();
    assert_eq!(
        from_page(&store, &opened, ("fx", None), None),
        SeatRead::Kept
    );
}

/// A forward index missing the stream's first row begins below its row's
/// first checkpoint: no page of it stands for the row, so the read is the
/// kept one (H2, M6).
#[test]
fn a_forward_index_missing_its_first_row_reads_the_kept_way() {
    let (dir, mut store) = store();
    let opened = opened(&mut store, retried());
    drop(store);
    assert_eq!(kept_after_index_delete(&dir, &opened, 76), SeatRead::Kept);
}

/// A checkpoint of attempt `a` naming no member, appended to `run_id`, and
/// its seq.
fn checkpointed(store: &mut Store, run_id: &str) -> u64 {
    let payload = json!({"effect_id": "fx", "attempt_id": "a",
                         "checkpoint": {"step": "seat-turn"}});
    crate::test_support::append(store, run_id, EventType::EffectCheckpointed, payload).seq
}

/// A re-opened attempt's checkpoints each find their own site row: the one
/// before the second start finds the earlier site, the one after it the
/// later, and a checkpoint before any start finds none (decision 0089
/// ruling 7).
#[test]
fn a_reopened_attempts_checkpoint_finds_its_later_site() {
    let (_dir, mut store) = store();
    crate::test_support::opened(&mut store, "r", "work", &[]);
    crate::test_support::append(
        &mut store,
        "r",
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
    );
    crate::test_support::started(&mut store, "r", "a", None, "anthropic");
    let first = checkpointed(&mut store, "r");
    crate::test_support::failed(&mut store, "r", "a");
    crate::test_support::started(&mut store, "r", "a", None, "anthropic");
    let second = checkpointed(&mut store, "r");
    let site_at = |started_seq: i64| {
        store
            .conn
            .query_row(
                "SELECT opened FROM seat_summary
                 WHERE run_id = 'r' AND attempt_id = 'a' AND started_seq = ?1
                   AND site IS NOT NULL",
                [started_seq],
                |row| row.get::<_, u64>(0),
            )
            .expect("the attempt's site row")
    };
    let (earlier, later) = (site_at(4), site_at(7));
    assert_ne!(earlier, later);
    assert_eq!(
        store.site_of_checkpoint("r", "a", None, first).unwrap(),
        Some(earlier)
    );
    assert_eq!(
        store.site_of_checkpoint("r", "a", None, second).unwrap(),
        Some(later)
    );
    // A checkpoint appended before its attempt opens any site finds none.
    crate::test_support::opened(&mut store, "q", "work", &[]);
    let early = checkpointed(&mut store, "q");
    assert_eq!(
        store.site_of_checkpoint("q", "a", None, early).unwrap(),
        None
    );
}

/// The site read's plan is the site index's range, with no `SCAN`
/// (decision 0089 ruling 7): a window's checkpoint finds its site row
/// through the key alone.
#[test]
fn a_checkpoints_site_is_found_through_the_site_index() {
    let (_dir, store) = store();
    let plan = plan(
        &store,
        super::SITE_OF,
        params!["r", "a", Option::<&str>::None, 8_i64],
    )
    .unwrap();
    assert_eq!(
        plan,
        ["SEARCH seat_summary USING INDEX seat_summary_site \
             (run_id=? AND attempt_id=? AND member=? AND started_seq<?)"]
    );
}
