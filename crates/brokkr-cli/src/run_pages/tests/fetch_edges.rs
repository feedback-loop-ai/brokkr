//! The paged reader's own edges (#753 coverage round 753c8): the pages a
//! store keeps, the empty page a cursor above the oldest row hands back,
//! the checkpoints that landed without an attempt, and the participant
//! asks that name no seat row. Each drives `Fetch`, the reader the shell's
//! `console` builds from its store read.

use super::*;
use crate::run_pages::fetch::{landed_count, Fetch};
use crate::run_pages::page::{Checkpoints, Page, WINDOW};

/// The store, its opening and events for `run`, with a reader at `rows`,
/// `offset`, the frame's `visible` rows, and what landed since its page.
struct Journal {
    _dir: tempfile::TempDir,
    store: Store,
    at: Opened,
    events: Vec<EventEnvelope>,
}

impl Journal {
    fn of(events: Vec<EventEnvelope>, run: &str) -> Journal {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("forge.db");
        planted(&db, vec![(run.to_string(), events)]);
        let store = Store::open_read_only(&db).unwrap();
        let at = store.open_run(run).unwrap();
        let events = store.load(run).unwrap();
        Journal {
            _dir: dir,
            store,
            at,
            events,
        }
    }

    fn fetch<'a>(
        &'a self,
        run: &'a str,
        rows: usize,
        offset: usize,
        landed: &'a [EventEnvelope],
    ) -> Fetch<'a> {
        Fetch {
            store: &self.store,
            run,
            at: &self.at,
            rows,
            offset,
            visible: WINDOW,
            landed,
            trail: &self.events,
        }
    }
}

/// The checkpoint events of `events`.
fn checkpoints(events: &[EventEnvelope]) -> Vec<EventEnvelope> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .cloned()
        .collect()
}

/// A checkpoint of `seat`'s effect with no attempt, which no stream keys:
/// `page_rows` makes nothing of it.
fn attemptless(seat: &str) -> EventEnvelope {
    let mut event = checkpoints(&at_work("landed", 1, true)).pop().unwrap();
    event.payload["effect_id"] = json!(seat);
    event.payload.as_object_mut().unwrap().remove("attempt_id");
    event
}

/// `run`'s own checkpoints with `member`'s checkpoint at `member_at`, so
/// the seat's forward pages cross a member's stream where the fixture says.
fn own_and_member(run: &str, turns: u64, member_at: u64) -> Vec<EventEnvelope> {
    let mut steps = in_flight_as(run, "eff", "att");
    for turn in 1..=turns {
        steps.push(journals::checkpoint(
            "eff",
            "att",
            json!({"step": "seat-turn", "turn": turn}),
        ));
        if turn == member_at {
            steps.push(journals::checkpoint(
                "eff",
                "att",
                json!({"member": "m", "step": "seat-turn", "turn": turn}),
            ));
        }
    }
    chained(run, journals::journal(steps))
}

/// A carried page at `start` with `older` and no newer cursor.
fn carried(start: usize, older: Option<u64>) -> Page {
    Page {
        rows: Vec::new(),
        next: None,
        older,
        start,
    }
}

/// The first checkpoint's seq of `events`.
fn first_seq(events: &[EventEnvelope]) -> u64 {
    checkpoints(events).first().unwrap().seq
}

/// The last checkpoint's seq of `events`.
fn last_seq(events: &[EventEnvelope]) -> u64 {
    checkpoints(events).last().unwrap().seq
}

/// [`Fetch::participant`]'s page for `fetched`: its rows' seqs and cursor.
fn seqs(fetched: Option<(Page, Option<usize>)>) -> (Vec<u64>, Option<u64>, usize) {
    let (page, total) = fetched.expect("the page is read");
    (
        page.rows.iter().map(|(seq, _)| *seq).collect(),
        page.next,
        total.unwrap_or(0),
    )
}

/// M6 (ruling 8): a page the store keeps while a held page is extended
/// back is no page, so the participant level reads the run whole.
#[test]
fn a_kept_page_while_extending_back_is_no_page() {
    let run = "kept back";
    let journal = Journal::of(own_and_member(run, 120, 2), run);
    let fetch = journal.fetch(run, 0, 50, &[]);
    // A cursor just past the member's own checkpoint keeps the backward
    // page that would hold it, as the store's interleave check says.
    let cursor = checkpoints(&journal.events)[3].seq;
    let page = carried(100, Some(cursor));
    assert!(
        fetch
            .participant(("eff", None), Some(page))
            .unwrap()
            .is_none(),
        "the kept older page takes the whole read"
    );
}

/// A page step into a carried window's older rows reads them and widens
/// the window, then stops at the stream's oldest row.
#[test]
fn a_page_step_back_reads_the_older_rows() {
    let run = "older rows";
    let journal = Journal::of(at_work(run, 250, true), run);
    let fetch = journal.fetch(run, 0, 0, &[]);
    let cursor = checkpoints(&journal.events)[150].seq;
    let page = carried(60, Some(cursor));
    let (rows, _, _) = seqs(fetch.participant(("eff", None), Some(page)).unwrap());
    assert!(!rows.is_empty(), "the older page's rows are held");
}

/// A page step above a carried window whose older cursor is gone stops
/// at the row it holds, rather than reading a page it does not have.
#[test]
fn a_carried_window_without_an_older_cursor_stops_at_its_start() {
    let run = "no older";
    let journal = Journal::of(at_work(run, 120, true), run);
    let fetch = journal.fetch(run, 0, 0, &[]);
    let page = carried(5, None);
    let (rows, next, total) = seqs(fetch.participant(("eff", None), Some(page)).unwrap());
    assert!(rows.is_empty(), "the held window's start stands");
    assert_eq!(next, None, "no page beyond the held window is read");
    assert_eq!(total, 5, "the rank the window already holds");
}

/// A run at work whose one participant holds a thousand checkpoints, and
/// their seqs in order: the stream both backward-read cases scroll.
fn thousand_checkpoints(run: &str) -> (Journal, Vec<u64>) {
    let journal = Journal::of(at_work(run, 1_000, true), run);
    let seqs = checkpoints(&journal.events)
        .iter()
        .map(|event| event.seq)
        .collect();
    (journal, seqs)
}

/// The backward read's first older page already crosses the window's end,
/// with nothing cut before it (753cL4's host coverage: `extend_back`'s
/// `cut || page.seen() > end` second arm). The carried page is the stream's
/// last, [950, 1000), and the scroll asks for row 740, so the window ends at
/// 940, inside the rows the first prepended page brings in. The suffix past
/// 940 is cut there, and the forward cursor names row 939, the window's last.
#[test]
fn a_backward_read_whose_first_page_crosses_the_window_end_cuts_there() {
    let run = "backward crossing";
    let (journal, seqs) = thousand_checkpoints(run);
    let (mut page, _) = journal
        .fetch(run, 24, 950, &[])
        .participant(("eff", None), None)
        .unwrap()
        .expect("the deep page reads");
    assert_eq!(
        (page.start, page.seen()),
        (950, 1_000),
        "the carried page is the stream's last"
    );
    let back = journal.fetch(run, 24, 740, &[]);
    assert!(back
        .extend_back(("eff", None), &mut page)
        .unwrap()
        .is_some());
    assert_eq!(
        page.seen(),
        740 + WINDOW,
        "the rows end at the window's end"
    );
    assert_eq!(
        page.next,
        Some(seqs[740 + WINDOW - 1]),
        "the forward cursor is the window's last row"
    );
}

/// M1 (ruling 9): a backward jump from the stream's last window to its
/// head holds one window, not every page between. The carried page is the
/// one a deep step read ([950, 1000)); `g` asks for row 0, the backward
/// read retains only the window the caller will hold, and its forward
/// cursor names that window's last row. The page is still the whole
/// read's, and the stream's known end survives the cut.
#[test]
fn a_backward_jump_holds_the_window_not_the_stream() {
    let run = "backward window";
    let (journal, seqs) = thousand_checkpoints(run);
    assert_eq!(seqs.len(), 1_000, "the journal holds the stream");
    let deep = journal.fetch(run, 24, 950, &[]);
    let head = journal.fetch(run, 24, 0, &[]);
    let carried_page = || {
        let (page, total) = deep
            .participant(("eff", None), None)
            .unwrap()
            .expect("the deep page reads");
        assert_eq!(page.start, 950, "the carried page is the deep one");
        assert_eq!(total, Some(1_000), "the deep read reached the end");
        page
    };
    // The bound itself: after the backward read the retained rows are the
    // caller's window, never the thousand rows between it and the end.
    let mut page = carried_page();
    assert!(head
        .extend_back(("eff", None), &mut page)
        .unwrap()
        .is_some());
    assert_eq!(page.rows.len(), WINDOW, "the retained rows are one window");
    assert_eq!(
        page.next,
        Some(seqs[WINDOW - 1]),
        "the forward cursor is the window's last row"
    );
    // The whole read's window, through the participant read the shell
    // makes: the same rows, and the end the carried page already knew.
    let (window, total) = head
        .participant(("eff", None), Some(carried_page()))
        .unwrap()
        .expect("the head page reads");
    let held: Vec<u64> = window.rows.iter().map(|(seq, _)| *seq).collect();
    assert_eq!(
        held,
        seqs[..WINDOW],
        "the jump's rows are the whole read's first window"
    );
    assert_eq!(
        total,
        Some(1_000),
        "the known end survives the backward cut"
    );
}

/// An older page that holds no checkpoint is the stream's oldest row: the
/// window drops the cursor rather than asking again.
#[test]
fn an_empty_older_page_is_the_stream_s_oldest() {
    let run = "empty older";
    let journal = Journal::of(at_work(run, 120, true), run);
    let fetch = journal.fetch(run, 0, 0, &[]);
    let page = carried(5, Some(first_seq(&journal.events)));
    let (rows, _, _) = seqs(fetch.participant(("eff", None), Some(page)).unwrap());
    assert!(rows.is_empty(), "the oldest page has no rows");
}

/// A page the store keeps while a window is extended forward is no page:
/// the participant level reads the run whole (M6).
#[test]
fn a_kept_page_while_extending_forward_is_no_page() {
    let run = "kept forward";
    let journal = Journal::of(own_and_member(run, 120, 60), run);
    let fetch = journal.fetch(run, 0, 0, &[]);
    assert!(
        fetch.participant(("eff", None), None).unwrap().is_none(),
        "the kept newer page takes the whole read"
    );
}

/// A whole participant read (a tall pane's `rows == MAX`) takes the kept
/// way when the store will not hand its next page (M6).
#[test]
fn a_kept_whole_participant_read_is_no_page() {
    let run = "kept whole";
    whole_participant_is_no_page(run, own_and_member(run, 120, 2));
}

/// A whole participant read extended past its first page takes the kept
/// way when a later page is kept (M6).
#[test]
fn a_kept_page_in_a_whole_participant_read_is_no_page() {
    let run = "kept whole forward";
    whole_participant_is_no_page(run, own_and_member(run, 120, 60));
}

/// A whole participant read of `events` takes the kept way when the store
/// will not hand a page: the one assertion both whole-read shapes share.
fn whole_participant_is_no_page(run: &str, events: Vec<EventEnvelope>) {
    let journal = Journal::of(events, run);
    let fetch = journal.fetch(run, usize::MAX, 0, &[]);
    assert!(
        fetch
            .participant(("eff", None), Some(Page::default()))
            .unwrap()
            .is_none(),
        "the kept page takes the whole read"
    );
}

/// A carried page whose landing names no attempt is no page: the whole
/// read is the only read that can make its row.
#[test]
fn a_landing_without_an_attempt_is_no_page() {
    let run = "no attempt";
    let journal = Journal::of(at_work(run, 120, true), run);
    let started = journal
        .events
        .iter()
        .find(|event| event.event_type == EventType::EffectStarted)
        .cloned()
        .unwrap();
    let landed = [started, attemptless("eff")];
    let fetch = journal.fetch(run, 0, 0, &landed);
    let page = Page::default();
    assert!(
        fetch
            .participant(("eff", None), Some(page))
            .unwrap()
            .is_none(),
        "a landing the row maker refuses takes the whole read"
    );
}

/// A carried detail page whose landing names no attempt is no page: the
/// merged window cannot make the arrived row (M6).
#[test]
fn a_detail_landing_without_an_attempt_is_no_page() {
    let run = "detail no attempt";
    let journal = Journal::of(at_work(run, 120, true), run);
    let landed = [attemptless("eff")];
    let fetch = journal.fetch(run, 0, 0, &landed);
    let mut held = std::collections::BTreeMap::new();
    held.insert("eff".to_string(), Page::default());
    assert!(
        fetch
            .window(
                &["eff".to_string()],
                &std::collections::BTreeMap::from([("eff".to_string(), ("eff".to_string(), None))]),
                held,
                0,
                false,
            )
            .unwrap()
            .is_none(),
        "a landing the row maker refuses takes the whole read"
    );
}

/// A detail stream whose key names no seat row is no page: the screen
/// reads the run whole rather than inventing a stream.
#[test]
fn a_detail_key_without_a_seat_row_is_no_page() {
    let run = "no seat";
    let journal = Journal::of(at_work(run, 120, true), run);
    let fetch = journal.fetch(run, 0, 0, &[]);
    assert!(
        fetch
            .window(
                &["missing".to_string()],
                &std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                0,
                true,
            )
            .unwrap()
            .is_none(),
        "a key with no seat row reads the run whole"
    );
}

/// The merge reads the empty page a cursor above the newest row hands
/// back as the stream's end, not as a page: a participant window at the
/// end stops there.
#[test]
fn an_empty_newer_page_ends_the_window() {
    let run = "empty newer";
    let journal = Journal::of(at_work(run, 120, true), run);
    let fetch = journal.fetch(run, 0, 0, &[]);
    let page = Page {
        rows: Vec::new(),
        next: Some(last_seq(&journal.events)),
        older: None,
        start: 0,
    };
    let (rows, next, _) = seqs(fetch.participant(("eff", None), Some(page)).unwrap());
    assert!(rows.is_empty(), "no row stands after the last checkpoint");
    assert_eq!(next, None, "the stream's end has no cursor");
}

/// A participant's first page kept by the store's own interleave check is
/// no page for the absolute end either (M6).
#[test]
fn a_kept_first_page_at_the_absolute_end_is_no_page() {
    let run = "kept end first";
    let journal = Journal::of(own_and_member(run, 120, 2), run);
    let fetch = journal.fetch(run, 24, usize::MAX, &[]);
    assert!(
        fetch.participant(("eff", None), None).unwrap().is_none(),
        "the kept first page takes the whole read"
    );
}

/// A participant's absolute end reads its pages until the display rank's
/// edge; a kept second page stops it there (M6).
#[test]
fn a_kept_second_page_at_the_absolute_end_is_no_page() {
    let run = "kept end second";
    let journal = Journal::of(own_and_member(run, 120, 60), run);
    let fetch = journal.fetch(run, 24, usize::MAX, &[]);
    assert!(
        fetch.participant(("eff", None), None).unwrap().is_none(),
        "the kept second page takes the whole read"
    );
}

/// The landing counter names only the seat's own checkpoints: another
/// member's or another effect's is not its rank.
#[test]
fn the_landing_count_names_only_the_seat_s_checkpoints() {
    let run = "landing count";
    let events = at_work(run, 3, true);
    let own = checkpoints(&events)[0].clone();
    let mut member = own.clone();
    member.payload["checkpoint"]["member"] = json!("m");
    let mut other = own.clone();
    other.payload["effect_id"] = json!("other");
    let started = events
        .iter()
        .find(|event| event.event_type == EventType::EffectStarted)
        .cloned()
        .unwrap();
    let landed = [started, own.clone(), member, other];
    assert_eq!(landed_count(("eff", None), &landed), 1, "the seat's own");
    assert_eq!(
        landed_count(("eff", Some("m")), &landed),
        1,
        "the member's own"
    );
    assert_eq!(landed_count(("none", None), &landed), 0, "no effect");
}

/// A carried detail window that dropped its newer prefix is refilled when
/// a checkpoint lands: its kept start is no longer the head's (F1, F4).
#[test]
fn a_scrolled_detail_window_refills_on_a_landing() {
    let run = "detail scrolled";
    let journal = Journal::of(at_work(run, 120, true), run);
    let landed = [checkpoints(&journal.events)[0].clone()];
    let fetch = journal.fetch(run, 0, 0, &landed);
    let mut held = Checkpoints::new();
    held.start = 100;
    held.pages.insert("eff".to_string(), Page::default());
    let seats = std::collections::BTreeMap::from([("eff".to_string(), ("eff".to_string(), None))]);
    let page =
        super::super::detail_checkpoints(&fetch, &["eff".to_string()], &seats, held, 0).unwrap();
    let page = page.expect("the landing refills the scrolled window");
    assert!(
        page.pages["eff"].rows.len() > 1,
        "the refilled page is the newest page, not one arrival"
    );
}
