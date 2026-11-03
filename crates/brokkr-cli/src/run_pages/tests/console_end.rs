//! The console's ends and carried windows (#753; decision 0089 slice 3):
//! the absolute-end key, a carried skewed merge, an empty screen's first
//! arrival, the focused and unfocused participant pane, a tall carried
//! window's requested rank, a wrapped window's `PageUp`, a large end's
//! logical rank and the arrival after it, overlapping statement
//! collectors and a kept page's whole fallback. Split from `tests.rs` to
//! keep each test file within its ceiling.

use super::far_work::{far_work, far_work_of};
use super::*;

/// The participant level over `whole`, its first seat selected.
fn participant_of(run: &str, whole: &RunView) -> (Tui, Subject) {
    participant_console(run, whole)
}

/// The whole read's five Downs and `PageUp` at `start`, settled: the
/// control a paged crossing must reach.
fn whole_page_up(run: &str, start: usize, whole: &Views) -> Tui {
    use crate::tui::{down_key, page_key, settle_end};
    let mut tui = live_tui(run, start, 375, 250);
    for _ in 0..5 {
        down_key(&mut tui, whole);
    }
    page_key(&mut tui, whole, false);
    settle_end(&mut tui, whole);
    tui
}

/// The participant level's page for `tui`'s own scroll, read as the shell
/// asks it.
pub(super) fn participant_page(db: &Path, run: &str, subject: &Subject, tui: &Tui) -> Views {
    use crate::tui::{ask_offset, asked_rows};
    let rows = asked_rows(tui);
    console_rows_at(db, run, Some(subject.clone()), false, rows, ask_offset(tui))
}

/// The participant level at `run`'s first page with `G` asked, the start
/// the end's controls share.
fn participant_end(db: &Path, run: &str, whole: &RunView) -> (Tui, Subject, Views) {
    use crate::tui::bottom_key;
    let (mut tui, subject) = participant_console(run, whole);
    let views = participant_page(db, run, &subject, &tui);
    bottom_key(&mut tui, &views);
    (tui, subject, views)
}

/// The participant level's held page at rows 0..200 with a page beyond it,
/// the window a `PageDown` from 195 crosses.
fn held_lower_page(db: &Path, run: &str, subject: &Subject) -> Views {
    let lower = console_rows_at(db, run, Some(subject.clone()), false, 200, 0);
    assert_eq!(
        (lower.start, lower.more),
        (0, true),
        "the held window and the page beyond it"
    );
    lower
}

/// The live column's absolute-end request: the read at the end and the
/// cursor it resolves to, at `now`.
fn settle_live_end(db: &Path, run: &str, tui: &mut Tui, now: &str) -> Views {
    use crate::tui::{ask_offset, asked_rows, settle_end};
    let rows = asked_rows(tui);
    let mut views = console_rows_at(db, run, None, false, rows, ask_offset(tui));
    views.now = now.to_string();
    settle_end(tui, &views);
    views
}

/// The participant level's absolute-end key resolved from `start`, the
/// level's first page: the real `G`, the read it asks, and the rank the
/// read answers at `now`.
fn settle_participant_end(
    db: &Path,
    run: &str,
    subject: &Subject,
    tui: &mut Tui,
    start: &Views,
    now: &str,
) -> Views {
    use crate::tui::{bottom_key, settle_end};
    bottom_key(tui, start);
    let mut views = participant_page(db, run, subject, tui);
    views.now = now.to_string();
    settle_end(tui, &views);
    views
}

/// The reads the shell would make for the live column's held keys: the
/// page each waits for, answered and settled, until none is left. A held
/// key that never lands fails here, by name, rather than looping.
pub(super) fn settle_held_live(db: &Path, run: &str, tui: &mut Tui, now: &str) -> Views {
    use crate::tui::settle_when;
    let mut views = live_read(db, run, tui, now);
    // Far more reads than any held run of keys needs: each read lands at
    // least one window's worth.
    for _ in 0..1_000 {
        settle_when(tui, &views, true);
        if tui.held.is_empty() {
            return views;
        }
        views = live_read(db, run, tui, now);
    }
    panic!("held keys never landed: {} still held", tui.held.len())
}

/// The live column's real read at `rank` on the wrapped panel, and the
/// terminal it is drawn at.
pub(super) fn live_at(
    db: &Path,
    run: &str,
    rank: usize,
    width: u16,
    height: u16,
    now: &str,
) -> (Tui, Views) {
    let tui = live_tui(run, rank, width, height);
    let views = live_read(db, run, &tui, now);
    (tui, views)
}

/// One run's journal planted at a fresh `db`, its events, and the whole
/// console's read of it.
type ConsoleJournal = (
    tempfile::TempDir,
    std::path::PathBuf,
    Vec<EventEnvelope>,
    Views,
);

pub(super) fn console_of(run: &str, journal: Vec<EventEnvelope>) -> ConsoleJournal {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    planted(&db, vec![(run.to_string(), journal)]);
    let (_, events, _) = console_journal(&db, run);
    let views = whole_console(&db, run, &events);
    (dir, db, events, views)
}

/// The journal at `db` holding one run's two-member panel of 500
/// interleaved 3,000-character checkpoints each, and its whole console
/// read.
pub(super) fn wrapped_panel_console(run: &str) -> ConsoleJournal {
    wrapped_panel_console_of(run, 500)
}

/// [`wrapped_panel_console`] with `turns` of each member: the rows a
/// resize proof reads, not the whole 1,000-row stream.
pub(super) fn wrapped_panel_console_of(run: &str, turns: u64) -> ConsoleJournal {
    let target = "x".repeat(3_000);
    console_of(run, panel_with(run, turns, turns, Some(&target)))
}

/// [`wrapped_panel_console_of`] with only the first turn's two rows
/// wrapped: the newest-first stream's rows 100 and 101, the last rows a
/// resize proof starts on. Wrapping the rows above them too would make
/// each of the proof's twenty-five `Down`s lay out rows no assertion
/// reads.
pub(super) fn wrapped_first_turn_console(run: &str, turns: u64) -> ConsoleJournal {
    let target = "x".repeat(3_000);
    console_of(run, panel_from(run, turns, turns, 1..=1, Some(&target)))
}

/// The first checkpoint turn a participant-level read paints.
pub(super) fn first_turn(views: &Views, key: &str) -> Option<String> {
    views
        .run
        .as_ref()?
        .participants
        .iter()
        .find(|part| part.key == key)?
        .checkpoints
        .first()
        .map(|row| row.turn.text.clone())
}

/// A carried detail window over a skewed panel keeps its absolute rank and
/// its rows through each deeper ask (M1, F1): member `a` records 400 turns
/// and `b` 60, so `b`'s rows pass above the window as it deepens; the same
/// `Shown` carried through asks of 224, 274 and 324 rows at one head keeps
/// the whole read's rank (24, 74, 124), never counting `b`'s dropped rows
/// again, and the live draw at row 300 paints the whole read's row over
/// wrapped lines.
#[test]
fn a_carried_skewed_panel_keeps_its_rank_and_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "carried skew";
    let target = format!("crates/{}", "wide/".repeat(16));
    planted(
        &db,
        vec![(run.to_string(), panel_with(run, 400, 60, Some(&target)))],
    );
    let store = Store::open_read_only(&db).unwrap();
    let head = store.head_hash(run).unwrap();
    let mut shown = None;
    let mut starts = Vec::new();
    for rows in [224, 274, 324] {
        let (next, _) =
            super::console(&store, run, Console::Detail, shown, Some(&head), rows, 0).unwrap();
        starts.push(next.start(&Console::Detail));
        shown = Some(next);
    }
    assert_eq!(starts, [24, 74, 124], "the carried window's absolute ranks");
    let shown = shown.unwrap();
    let (_, events, _) = console_journal(&db, run);
    let whole = whole_console(&db, run, &events);
    let views = detail_shown_views(&db, run, &shown, &whole.now);
    let (width, height) = (375, 20);
    let tui = live_tui(run, 300, width, height);
    let paged = live_frame_for_test(&tui, &views, run, width, height);
    let expected = live_frame_for_test(&tui, &whole, run, width, height);
    assert_eq!(paged, expected, "the live draw at row 300");
}

/// The first checkpoint to land on a screen that had none is painted once
/// (M2): a participant with zero checkpoints read at 30 rows and offset 0
/// keeps its empty page, then a correctly chained turn 1 lands and the
/// moved head is polled; the detail pane and the participant level each
/// hold [1], never [1, 1].
#[test]
fn the_first_arrival_is_shown_once_on_an_empty_screen() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let runs = ["empty detail", "empty participant"];
    let journals: Vec<(String, Vec<EventEnvelope>)> = runs
        .iter()
        .map(|run| (run.to_string(), at_work(run, 0, true)))
        .collect();
    planted(&db, journals);
    let store = Store::open_read_only(&db).unwrap();
    let asked = [
        (runs[0], Console::Detail),
        (runs[1], Console::Participant("eff".to_string())),
    ];
    for (run, screen) in asked {
        let (shown, _) = super::console(&store, run, screen.clone(), None, None, 30, 0).unwrap();
        assert_eq!(turns(&shown.view()), Vec::<String>::new(), "{run}: empty");
        let mut head = store.head_hash(run).unwrap();
        land(&db, run, &mut head, 1, "2026-01-01T00:10:00Z");
        drop(Store::open(&db).unwrap());
        let moved = store.head_hash(run).unwrap();
        let (shown, _) =
            super::console(&store, run, screen, Some(shown), Some(&moved), 30, 0).unwrap();
        assert_eq!(turns(&shown.view()), ["1"], "{run}: the first arrival");
    }
}

/// The real `G` key reaches the stream's absolute end (M3): over a
/// forward-clock run of 1,000 checkpoints the live column asks the end,
/// resolves to offset 999 and paints the whole read's last screen from
/// turn 1; the participant level resolves to offset 999 and paints turn
/// 1,000 first. Both read a bounded window, not the stream.
#[test]
fn the_console_s_bottom_reaches_the_stream_s_absolute_end() {
    use crate::tui::{bottom_key, checkpoint_offset, settle_end};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "bottom";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let whole = whole_views.run.as_ref().unwrap();
    // The live column: `G` asks for the end, the read answers it, and the
    // shell resolves the request into the rank the pane scrolls at.
    let (width, height) = (375, 20);
    let mut tui = live_tui(run, 0, width, height);
    let views = live_read(&db, run, &tui, &whole_views.now);
    bottom_key(&mut tui, &views);
    let rows = crate::tui::asked_rows(&tui);
    let mut views = console_rows_at(&db, run, None, false, rows, checkpoint_offset(&tui));
    views.now = whole_views.now.clone();
    settle_end(&mut tui, &views);
    assert_eq!(tui.live_offset, 999, "the live column's end");
    let paged = live_frame_for_test(&tui, &views, run, width, height);
    let expected = live_frame_for_test(&tui, &whole_views, run, width, height);
    assert_eq!(paged, expected, "the live column's last screen");
    let live = brokkr_view::working_checkpoints(views.run.as_ref().unwrap(), "work");
    assert_eq!(
        live.last().map(|(_, row)| row.turn.text.as_str()),
        Some("1"),
        "the live column's last screen holds the oldest row"
    );
    // The participant level: `G` asks for the end; its pane paints from
    // the oldest row's rank and shows the newest row first.
    let (mut tui, subject) = participant_console(run, whole);
    tui.height = 30;
    let mut participant_whole = console_rows(&db, run, Some(subject.clone()), false, usize::MAX);
    participant_whole.now = whole_views.now.clone();
    participant_whole.run = Some(brokkr_view::run_view(&events, fold(&events).as_ref().ok()));
    participant_whole.start = 0;
    participant_whole.more = false;
    let rows = crate::tui::asked_rows(&tui);
    let mut views = console_rows_at(&db, run, Some(subject.clone()), false, rows, tui.offset);
    views.now = whole_views.now.clone();
    bottom_key(&mut tui, &views);
    let rows = crate::tui::asked_rows(&tui);
    let views = console_rows_at(
        &db,
        run,
        Some(subject),
        false,
        rows,
        checkpoint_offset(&tui),
    );
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 999, "the participant level's end");
    let paged = frame_for_test(&tui, &views, width, height);
    let expected = frame_for_test(&tui, &participant_whole, width, height);
    assert_eq!(paged, expected, "the participant level's last screen");
    let lines = rows_of_text(&paged);
    assert!(
        lines.iter().any(|line| line.contains("│1000  turn")),
        "the newest row is the first: {lines:?}"
    );
}

/// Tabbing away from a scrolled participant paints the unfocused pane
/// from its top, and Tab back restores the focused offset (M4): at
/// checkpoint offset 112 of 120 turns, the focused frame is the whole
/// read's at 112; after the real Tab the checkpoint pane is the whole
/// read's at its top, turn 1 first; Tab back paints 112 again.
#[test]
fn tabbing_from_a_scrolled_participant_paints_todays_unfocused_top() {
    use crate::tui::{checkpoint_offset, tab_key};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "tabbed";
    at_work_in(&db, &[(run, 120, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    tui.height = 32;
    let whole_views = participant_whole_views(&db, run, &events, subject.clone());
    let frame = |tui: &Tui, views: &Views| tui::frame_for_test(tui, views, 80, 32);
    tui.offset = 112;
    let rows = crate::tui::asked_rows(&tui);
    let mut views = console_rows_at(
        &db,
        run,
        Some(subject.clone()),
        false,
        rows,
        checkpoint_offset(&tui),
    );
    views.now = whole_views.now.clone();
    assert_eq!(
        frame(&tui, &views),
        frame(&tui, &whole_views),
        "the focused checkpoint pane at 112"
    );
    tab_key(&mut tui, &views);
    assert_eq!(tui.pane, 1, "Tab focuses the transcript");
    assert_eq!(checkpoint_offset(&tui), 0, "the hidden pane shows its top");
    let rows = crate::tui::asked_rows(&tui);
    let mut views = console_rows_at(&db, run, Some(subject.clone()), false, rows, 0);
    views.now = whole_views.now.clone();
    let unfocused = frame(&tui, &views);
    assert_eq!(
        unfocused,
        frame(&tui, &whole_views),
        "the unfocused checkpoint pane paints from its top"
    );
    assert!(
        rows_of_text(&unfocused)
            .iter()
            .any(|line| line.contains("│1  turn")),
        "the top begins at turn 1"
    );
    tab_key(&mut tui, &views);
    assert_eq!(tui.pane, 0, "Tab focuses the checkpoint pane again");
    assert_eq!(tui.offset, 112, "the focused offset is kept");
    let rows = crate::tui::asked_rows(&tui);
    let mut views = console_rows_at(
        &db,
        run,
        Some(subject),
        false,
        rows,
        checkpoint_offset(&tui),
    );
    views.now = whole_views.now.clone();
    assert_eq!(
        frame(&tui, &views),
        frame(&tui, &whole_views),
        "the focused checkpoint pane at 112 after Tab back"
    );
}

/// Overlapping statement collectors keep their own statements (M5): two
/// connections on two threads enter `statements_traced`, A queries and
/// waits while B installs its own trace, queries and returns; each sees
/// exactly its own one statement, never (0, 1).
#[test]
fn overlapping_statement_collectors_keep_their_own_statements() {
    use std::sync::mpsc;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "traced";
    at_work_in(&db, &[(run, 120, true)]);
    let path = db.clone();
    let (a_done, b_wait) = mpsc::channel();
    let (b_done, a_wait) = mpsc::channel();
    let a = std::thread::spawn(move || {
        let mut store = Store::open_read_only(&path).unwrap();
        let (_, begun) = statements_traced(&mut store, |store| {
            let head = store.head_hash(run).unwrap();
            a_done.send(()).unwrap();
            a_wait.recv().unwrap();
            head
        });
        begun.len()
    });
    let path = db.clone();
    let b = std::thread::spawn(move || {
        b_wait.recv().unwrap();
        let mut store = Store::open_read_only(&path).unwrap();
        let (_, begun) = statements_traced(&mut store, |store| store.head_hash(run).unwrap());
        b_done.send(()).unwrap();
        begun.len()
    });
    let counts = [a.join().unwrap(), b.join().unwrap()];
    assert_eq!(counts, [1, 1], "each collector keeps its own statement");
}

/// A page the store says is kept after a successful first page takes the
/// whole read (M6, ruling 8): `eff`'s own turn 1, member `m`'s turn 1 and
/// `eff`'s turns 2..120 make the third seat page cross the member's
/// checkpoint, which the store keeps; asking the detail pane for 200 rows
/// paints the whole run's 120 own rows and one member row, never a
/// truncated page.
#[test]
fn a_kept_page_after_a_successful_one_takes_the_whole_read() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "kept after a page";
    let mut steps = in_flight_as(run, "eff", "att");
    steps.push(journals::checkpoint(
        "eff",
        "att",
        json!({"step": "seat-turn", "turn": 1}),
    ));
    steps.push(journals::checkpoint(
        "eff",
        "att",
        json!({"member": "m", "step": "seat-turn", "turn": 1}),
    ));
    for turn in 2..=120 {
        steps.push(journals::checkpoint(
            "eff",
            "att",
            json!({"step": "seat-turn", "turn": turn}),
        ));
    }
    let events = chained(run, journals::journal(steps));
    planted(&db, vec![(run.to_string(), events)]);
    let store = Store::open_read_only(&db).unwrap();
    let (shown, _) = super::console(&store, run, Console::Detail, None, None, 200, 0).unwrap();
    assert!(
        matches!(shown.read, Held::Whole(..)),
        "a kept page takes ruling 8's whole read"
    );
    let events = store.load(run).unwrap();
    let whole = brokkr_view::run_view(&events, fold(&events).ok().as_ref());
    let counts: Vec<usize> = shown
        .view()
        .participants
        .iter()
        .map(|part| part.checkpoints.len())
        .collect();
    assert_eq!(
        counts,
        [120, 1],
        "the whole run's own rows and its member's"
    );
    assert_eq!(
        serde_json::to_string(&shown.view().with_scopes()).unwrap(),
        serde_json::to_string(&whole.with_scopes()).unwrap(),
        "the whole read's bytes"
    );
}

/// The participant level's keys wrap at the stream's absolute ends, not at
/// the held window's: `Up` at rank zero asks the stream's end and resolves
/// to 999 of 1,000, and `Down` from there returns to rank 0 (M1).
#[test]
fn the_participant_level_s_keys_wrap_at_the_stream_s_absolute_ends() {
    use crate::tui::{down_key, settle_end, up_key};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "participant wrap";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    assert_eq!(tui.offset, 0);
    let views = participant_page(&db, run, &subject, &tui);
    up_key(&mut tui, &views);
    assert_eq!(tui.offset, usize::MAX, "Up asks the stream's absolute end");
    let mut views = participant_page(&db, run, &subject, &tui);
    views.now = events.last().unwrap().recorded_at.clone();
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 999, "the stream's last row");
    down_key(&mut tui, &views);
    assert_eq!(tui.offset, 0, "Down from the stream's end is its top");
}

/// The live column's `Up` at rank zero asks the stream's absolute end and
/// resolves to its last row, not the held window's last row (M1).
#[test]
fn the_live_column_s_up_wraps_at_the_stream_s_absolute_end() {
    use crate::tui::up_key;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "live wrap";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let mut tui = live_tui(run, 0, 375, 20);
    let views = live_read(&db, run, &tui, &whole_views.now);
    up_key(&mut tui, &views);
    assert_eq!(
        tui.live_offset,
        usize::MAX,
        "Up asks the stream's absolute end"
    );
    settle_live_end(&db, run, &mut tui, &whole_views.now);
    assert_eq!((tui.live_offset, tui.live_line), (999, 0));
}

/// `G` reaches the final drawn line of a wrapped final row on both a paged
/// and a whole read: two members' 1,000 interleaved 3,000-character rows
/// at 375x20 resolve to (999, 28), and the paged frame is the whole
/// read's (M1's end, M2).
#[test]
fn the_end_key_reaches_the_last_drawn_line_of_a_wrapped_row() {
    use crate::tui::{asked_rows, bottom_key, checkpoint_offset, settle_end};
    let (_dir, db, _events, whole_views) = wrapped_panel_console("wrapped end");
    let run = "wrapped end";
    let mut paged_tui = live_tui(run, 0, 375, 20);
    let views = live_read(&db, run, &paged_tui, &whole_views.now);
    bottom_key(&mut paged_tui, &views);
    let rows = asked_rows(&paged_tui);
    let mut views = console_rows_at(&db, run, None, false, rows, checkpoint_offset(&paged_tui));
    views.now = whole_views.now.clone();
    settle_end(&mut paged_tui, &views);
    let mut whole_tui = live_tui(run, 0, 375, 20);
    bottom_key(&mut whole_tui, &whole_views);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (
            paged_tui.live_offset,
            paged_tui.live_line,
            whole_tui.live_offset,
            whole_tui.live_line
        ),
        (999, 28, 999, 28),
        "both reads reach the final row's last drawn line"
    );
    assert_eq!(
        live_frame_for_test(&paged_tui, &views, run, 375, 20),
        live_frame_for_test(&whole_tui, &whole_views, run, 375, 20),
        "the paged frame is the whole read's"
    );
}

/// A tall detail pane keeps every row it draws: at 250 rows the merge's
/// window starts at rank 0 and its newest turn is 1,000, where a fixed
/// 200-row buffer evicted the first 50 (M3).
#[test]
fn a_tall_detail_pane_keeps_every_row_it_draws() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "tall detail";
    at_work_in(&db, &[(run, 1_000, true)]);
    let store = Store::open_read_only(&db).unwrap();
    let (shown, _) = super::console_with(
        &store,
        run,
        Console::Detail,
        None,
        None,
        crate::run_pages::Reading {
            rows: 250,
            offset: 0,
            visible: 250,
        },
    )
    .unwrap();
    assert_eq!(
        shown.start(&Console::Detail),
        0,
        "the window starts at rank 0"
    );
    let view = shown.view();
    let live = brokkr_view::working_checkpoints(&view, "work");
    assert_eq!(live.len(), 250, "every drawable row is held");
    assert_eq!(
        live.first().map(|(_, row)| row.turn.text.as_str()),
        Some("1000"),
        "the newest turn is first"
    );
}

/// The participant's `G` keeps today's frame at a rank past `u16::MAX`:
/// 65,600 checkpoints resolve to logical rank 65,599, display rank 65,535,
/// the page that rank names, and the pane starts at turn 65,536, never the
/// newest page's first row (M4, R2).
#[test]
fn the_participant_s_end_keeps_today_s_narrowed_display_rank() {
    use crate::tui::checkpoint_offset;
    let far = far_work();
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let (mut tui, subject) = participant_console(run, whole);
    let (mut today, _) = participant_console(run, whole);
    today.offset = u16::MAX as usize;
    let start = participant_page(db, run, &subject, &tui);
    let views = settle_participant_end(
        db,
        run,
        &subject,
        &mut tui,
        &start,
        &events.last().unwrap().recorded_at,
    );
    assert_eq!(tui.offset, 65_599, "the stream's logical end");
    assert_eq!(
        checkpoint_offset(&tui),
        u16::MAX as usize,
        "today's renderer's narrowed display rank"
    );
    let paged = frame_for_test(&tui, &views, 80, 30);
    let whole_views = far.whole_views();
    let expected = frame_for_test(&today, whole_views, 80, 30);
    assert_eq!(paged, expected, "today's frame at the narrowed rank");
    assert!(
        rows_of_text(&paged)
            .iter()
            .any(|line| line.contains("│65536  turn")),
        "the frame starts at turn 65,536"
    );
}

/// A carried member page that holds no rows still takes a new arrival:
/// after a 50-row detail read of a 700/110 panel, `b:111` is merged into
/// b's empty carried page and is the first row, as a fresh read shows
/// (M5).
#[test]
fn a_carried_empty_member_page_takes_its_new_arrival() {
    use brokkr_core::EventType;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "carried arrival";
    planted(
        &db,
        vec![(run.to_string(), panel_with(run, 700, 110, None))],
    );
    let store = Store::open_read_only(&db).unwrap();
    let head = store.head_hash(run).unwrap();
    let (first, _) = super::console(&store, run, Console::Detail, None, None, 50, 0).unwrap();
    let landed = envelope_builder::EnvelopeBuilder::new(
        EventType::EffectCheckpointed,
        json!({
            "effect_id": "eff",
            "attempt_id": "att",
            "checkpoint": {"member": "b", "step": "seat-turn", "turn": 111},
        }),
    )
    .run(run)
    .seq(head.0 + 1)
    .event_id("carried-arrival-111")
    .at("2026-01-02T00:00:00Z")
    .previous(head.1)
    .sealed();
    let journal = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_envelope(&journal, &landed).unwrap();
    drop(Store::open(&db).unwrap());
    let moved = store.head_hash(run).unwrap();
    let reading = crate::run_pages::Reading {
        rows: 50,
        offset: 0,
        visible: 20,
    };
    let (next, _) = super::console_with(
        &store,
        run,
        Console::Detail,
        Some(first),
        Some(&moved),
        reading,
    )
    .unwrap();
    let (fresh, _) =
        super::console_with(&store, run, Console::Detail, None, Some(&moved), reading).unwrap();
    let first_row = |shown: &Shown| {
        brokkr_view::working_checkpoints(&shown.view(), "work")
            .first()
            .map(|(label, row)| (label.to_string(), row.turn.text.clone()))
    };
    assert_eq!(
        first_row(&next),
        first_row(&fresh),
        "the fresh read's first row"
    );
    assert_eq!(
        first_row(&next).map(|(_, turn)| turn).as_deref(),
        Some("111"),
        "b's arrival is first"
    );
}

/// `Up` at a stream held whole wraps to its foot, as today's renderer did
/// (R1): ten checkpoints at 375x20 are one page with `more == false`, so
/// the absolute end is asked, the read answers it, and the cursor lands on
/// the last row's last drawn line, which is the whole read's frame.
#[test]
fn the_live_column_s_up_wraps_a_fully_held_stream() {
    use crate::tui::{settle_end, up_key};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "held whole";
    at_work_in(&db, &[(run, 10, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let (width, height) = (375, 20);
    let mut tui = live_tui(run, 0, width, height);
    let views = live_read(&db, run, &tui, &whole_views.now);
    assert!(!views.more, "the whole stream is one page here");
    up_key(&mut tui, &views);
    assert_eq!(
        tui.live_offset,
        usize::MAX,
        "Up asks the stream's absolute end"
    );
    let views = settle_live_end(&db, run, &mut tui, &whole_views.now);
    let mut whole_tui = live_tui(run, 0, width, height);
    up_key(&mut whole_tui, &whole_views);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (
            tui.live_offset,
            tui.live_line,
            whole_tui.live_offset,
            whole_tui.live_line
        ),
        (9, 0, 9, 0),
        "both reads wrap to the last row's last drawn line"
    );
    assert_eq!(
        live_frame_for_test(&tui, &views, run, width, height),
        live_frame_for_test(&whole_tui, &whole_views, run, width, height),
        "the paged frame is the whole read's"
    );
}

/// `Down` at the participant's held end with a page beyond it asks the next
/// row instead of wrapping inside the window (R2): holding the oldest 200
/// rows at start 0 with `more == true`, the 200th real `Down` leaves offset
/// 200, whose page begins at turn 201.
#[test]
fn the_participant_level_s_down_crosses_a_held_window() {
    use crate::tui::down_key;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "pending window";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let views = console_rows_at(&db, run, Some(subject.clone()), false, 200, 0);
    assert_eq!(views.start, 0, "the held window starts at the top");
    assert!(views.more, "a page follows the held window");
    for _ in 0..200 {
        down_key(&mut tui, &views);
    }
    assert_eq!(tui.offset, 200, "the 200th Down asks the next row");
    let views = participant_page(&db, run, &subject, &tui);
    assert_eq!(
        first_turn(&views, "eff").as_deref(),
        Some("201"),
        "the next page's first turn"
    );
}

/// The participant's `G` fills a tall frame (R3): at 65,600 checkpoints the
/// end read starts at today's clamped rank and carries the drawable suffix,
/// so an 80-row pane paints through turn 65,568 and a 100-row one through
/// 65,578, exactly the whole read's frame at that rank.
#[test]
fn the_participant_s_end_fills_a_tall_frame() {
    let far = far_work();
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let now = events.last().unwrap().recorded_at.clone();
    let whole_views = far.whole_views();
    for (height, last) in [(80, "65568"), (100, "65578")] {
        let (mut tui, subject) = participant_console(run, whole);
        tui.height = height;
        let start = participant_page(db, run, &subject, &tui);
        let views = settle_participant_end(db, run, &subject, &mut tui, &start, &now);
        let (mut today, _) = participant_console(run, whole);
        (today.width, today.height) = (80, height);
        today.offset = u16::MAX as usize;
        let paged = frame_for_test(&tui, &views, 80, height);
        let expected = frame_for_test(&today, whole_views, 80, height);
        assert_eq!(paged, expected, "height {height}: the whole read's frame");
        let lines = rows_of_text(&paged);
        assert!(
            lines
                .iter()
                .any(|line| line.contains(&format!("│{last}  turn"))),
            "height {height}: every drawable row is painted: {lines:?}"
        );
    }
}

/// `Down` after a large participant `G` wraps at the stream's logical end
/// (R4): the read reports the counted endpoint 65,600, the pane keeps the
/// logical 65,599 and displays today's 65,535 rank, so `Down` is 0 as the
/// whole read's is.
#[test]
fn the_participant_s_down_after_a_large_end_wraps_to_the_top() {
    use crate::tui::{bottom_key, checkpoint_offset, down_key, settle_end};
    let far = far_work();
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let (mut tui, subject, _) = participant_end(db, run, whole);
    let mut views = participant_page(db, run, &subject, &tui);
    views.now = events.last().unwrap().recorded_at.clone();
    assert_eq!(views.end, Some(65_600), "the counted stream's end");
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 65_599, "the stream's logical end");
    assert_eq!(
        checkpoint_offset(&tui),
        u16::MAX as usize,
        "today's display rank"
    );
    down_key(&mut tui, &views);
    assert_eq!(tui.offset, 0, "Down is the stream's top");
    let (mut whole_tui, _) = participant_console(run, whole);
    let whole_views = far.whole_views();
    bottom_key(&mut whole_tui, whole_views);
    settle_end(&mut whole_tui, whole_views);
    down_key(&mut whole_tui, whole_views);
    assert_eq!(whole_tui.offset, 0, "the whole read's Down");
    let views = participant_page(db, run, &subject, &tui);
    assert_eq!(
        first_turn(&views, "eff").as_deref(),
        Some("1"),
        "the top is turn 1"
    );
}

/// `Up` across a tall window lands on the preceding wrapped row's last
/// drawn line (R5): two members' 500 interleaved 3,000-character rows at
/// 375x250, read at rank 100 and stepped up, agree with the whole read at
/// (99, 28) rather than dropping the continuations at (99, 0).
#[test]
fn the_live_column_s_up_crosses_to_the_preceding_wrapped_rows_last_line() {
    use crate::tui::{settle_end, up_key};
    let (_dir, db, _events, whole_views) = wrapped_panel_console("tall cross");
    let run = "tall cross";
    let (width, height) = (375, 250);
    let (mut tui, views) = live_at(&db, run, 100, width, height, &whole_views.now);
    assert_eq!(
        views.start, 100,
        "the held window starts at the read's rank"
    );
    up_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "Up is held for the row above");
    let views = settle_held_live(&db, run, &mut tui, &whole_views.now);
    let mut whole_tui = live_tui(run, 100, width, height);
    up_key(&mut whole_tui, &whole_views);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (99, 28),
        "the paged cursor is the whole read's"
    );
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (99, 28),
        "the whole read's cursor"
    );
    assert_eq!(
        live_frame_for_test(&tui, &views, run, width, height),
        live_frame_for_test(&whole_tui, &whole_views, run, width, height),
        "the paged frame is the whole read's"
    );
}

/// A carried tall detail window includes the rank its new scroll asks for
/// first (R1): at one head, 350 rows from rank 100 hold 100..350, and the
/// next ask, 349 rows from rank 99, must start at 99 and paint turn 901,
/// exactly as a fresh read of the same ask does. A carried window that
/// starts below the requested rank omits a visible row.
#[test]
fn chief_carried_tall_window_includes_requested_rank() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "carried tall rank";
    at_work_in(&db, &[(run, 1_000, true)]);
    let store = Store::open_read_only(&db).unwrap();
    let head = store.head_hash(run).unwrap();
    let first = crate::run_pages::Reading {
        rows: 350,
        offset: 100,
        visible: 250,
    };
    let (held, _) = super::console_with(&store, run, Console::Detail, None, None, first).unwrap();
    assert_eq!(held.start(&Console::Detail), 100, "the first read's rank");
    let reading = crate::run_pages::Reading {
        rows: 349,
        offset: 99,
        visible: 250,
    };
    let (carried, _) = super::console_with(
        &store,
        run,
        Console::Detail,
        Some(held),
        Some(&head),
        reading,
    )
    .unwrap();
    let (fresh, _) =
        super::console_with(&store, run, Console::Detail, None, Some(&head), reading).unwrap();
    let first_turn = |shown: &Shown| {
        brokkr_view::working_checkpoints(&shown.view(), "work")
            .first()
            .map(|(_, row)| row.turn.text.clone())
    };
    assert_eq!(
        carried.start(&Console::Detail),
        99,
        "the carried window starts at the requested rank"
    );
    assert_eq!(
        first_turn(&carried).as_deref(),
        Some("901"),
        "the requested rank's turn is painted first"
    );
    assert_eq!(
        first_turn(&carried),
        first_turn(&fresh),
        "the carried read is a fresh read's first row"
    );
}

/// A `PageUp` across a held window moves drawn lines, not checkpoint rows
/// (R3): two members' 500 interleaved 3,000-character rows at 375x250, read
/// at rank 100, cross to the whole read's (99, 19), not to rank 90's first
/// line, and paint the whole read's frame.
#[test]
fn chief_page_up_crosses_wrapped_window_in_drawn_lines() {
    use crate::tui::{page_key, settle_end};
    let (_dir, db, _events, whole_views) = wrapped_panel_console("wrapped page up");
    let run = "wrapped page up";
    let (width, height) = (375, 250);
    let (mut tui, views) = live_at(&db, run, 100, width, height, &whole_views.now);
    assert_eq!(views.start, 100, "the held window starts at the read");
    page_key(&mut tui, &views, false);
    let views = live_read(&db, run, &tui, &whole_views.now);
    settle_end(&mut tui, &views);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (99, 19),
        "PageUp crosses to the preceding row's line, not ten rows"
    );
    let mut whole_tui = live_tui(run, 100, width, height);
    page_key(&mut whole_tui, &whole_views, false);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (99, 19),
        "the whole read's PageUp"
    );
    assert_eq!(
        live_frame_for_test(&tui, &views, run, width, height),
        live_frame_for_test(&whole_tui, &whole_views, run, width, height),
        "the paged frame is the whole read's"
    );
}

/// A large participant `G` keeps the stream's logical rank for movement
/// while the pane displays today's narrowed frame (R2): at 65,600
/// checkpoints the end settles to logical 65,599 and display 65,535, `Up`
/// moves one logical row to 65,598, `PageUp` ten to 65,589, and both paint
/// the whole read's frame, whose first turn is 65,536.
#[test]
fn chief_large_end_keeps_logical_movement_for_display() {
    use crate::tui::{checkpoint_offset, page_key, settle_end, up_key};
    let far = far_work();
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let now = events.last().unwrap().recorded_at.clone();
    let (mut tui, subject) = participant_console(run, whole);
    let start = participant_page(db, run, &subject, &tui);
    let views = settle_participant_end(db, run, &subject, &mut tui, &start, &now);
    assert_eq!(tui.offset, 65_599, "the stream's logical end");
    assert_eq!(
        checkpoint_offset(&tui),
        u16::MAX as usize,
        "today's display rank"
    );
    up_key(&mut tui, &views);
    assert_eq!(tui.offset, 65_598, "Up keeps the logical rank");
    let mut views = participant_page(db, run, &subject, &tui);
    views.now = now.clone();
    settle_end(&mut tui, &views);
    let (mut whole_tui, _) = participant_console(run, whole);
    whole_tui.offset = 65_598;
    let whole_views = far.whole_views();
    let paged = frame_for_test(&tui, &views, 80, 30);
    assert_eq!(
        paged,
        frame_for_test(&whole_tui, whole_views, 80, 30),
        "Up paints the whole read's frame"
    );
    assert!(
        rows_of_text(&paged)
            .iter()
            .any(|line| line.contains("│65536  turn")),
        "the display still starts at turn 65,536"
    );
    let (mut paged_tui, _) = participant_console(run, whole);
    let views = settle_participant_end(db, run, &subject, &mut paged_tui, &start, &now);
    page_key(&mut paged_tui, &views, false);
    assert_eq!(paged_tui.offset, 65_589, "PageUp keeps the logical rank");
    let mut views = participant_page(db, run, &subject, &paged_tui);
    views.now = now.clone();
    settle_end(&mut paged_tui, &views);
    let (mut page_today, _) = participant_console(run, whole);
    page_today.offset = 65_589;
    let page_frame = frame_for_test(&paged_tui, &views, 80, 30);
    assert_eq!(
        page_frame,
        frame_for_test(&page_today, whole_views, 80, 30),
        "PageUp paints the whole read's frame"
    );
    assert!(
        rows_of_text(&page_frame)
            .iter()
            .any(|line| line.contains("│65536  turn")),
        "PageUp's display still starts at turn 65,536"
    );
}

/// `Down` after a large participant end wraps at the current stream's end,
/// not a remembered one (R4): `G` settles to 999 of 1,000, a correctly
/// chained turn 1001 lands and the moved head is read without moving, so
/// `Down` reaches the new arrival at 1,000 instead of wrapping to 0.
#[test]
fn chief_down_after_end_refresh_reaches_new_arrival() {
    use crate::tui::{down_key, settle_end};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "end refresh arrival";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_, events, _) = console_journal(&db, run);
    let now = events.last().unwrap().recorded_at.clone();
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, _subject) = participant_of(run, &whole);
    let key = Console::Participant("eff".to_string());
    let end = crate::run_pages::Reading {
        rows: 30,
        offset: usize::MAX,
        visible: 30,
    };
    let stored = Store::open_read_only(&db).unwrap();
    let (first, _) = super::console_with(&stored, run, key.clone(), None, None, end).unwrap();
    assert_eq!(first.end(&key), Some(1_000), "the counted stream's end");
    let mut views = Views::empty();
    views.now = now.clone();
    views.run = Some(first.view());
    views.start = first.start(&key);
    views.more = first.more(&key);
    views.end = first.end(&key);
    tui.offset = usize::MAX;
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 999, "G's logical end");
    let mut head = stored.head_hash(run).unwrap();
    land(&db, run, &mut head, 1_001, "2026-01-02T00:00:00Z");
    drop(Store::open(&db).unwrap());
    let moved = stored.head_hash(run).unwrap();
    let reading = crate::run_pages::Reading {
        rows: 999 + 30,
        offset: 999,
        visible: 30,
    };
    let (next, _) = super::console_with(
        &stored,
        run,
        key.clone(),
        Some(first),
        Some(&moved),
        reading,
    )
    .unwrap();
    let mut refreshed = Views::empty();
    refreshed.now = now;
    refreshed.run = Some(next.view());
    refreshed.start = next.start(&key);
    refreshed.more = next.more(&key);
    let held: Vec<String> = refreshed
        .run
        .as_ref()
        .unwrap()
        .participants
        .iter()
        .find(|part| part.key == "eff")
        .unwrap()
        .checkpoints
        .iter()
        .map(|row| row.turn.text.clone())
        .collect();
    assert_eq!(
        held.last().map(String::as_str),
        Some("1001"),
        "the refreshed read holds the arrival below the requested rank"
    );
    down_key(&mut tui, &refreshed);
    assert_eq!(tui.offset, 1_000, "Down reaches the arrival, not the top");
}

/// A wrapped `PageUp` that begins inside the held window keeps the whole
/// read's remaining drawn-line distance (R1): two members' 500 interleaved
/// 3,000-character rows at 375x250, read at rank 100 and stepped Down five
/// drawn lines, crosses to the whole read's (99,24), not to the window's
/// first line at (100,0).
#[test]
fn gate_review_wrapped_page_up_preserves_remaining_distance() {
    use crate::tui::{down_key, page_key, settle_end};
    let run = "gate wrapped page up";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = live_at(&db, run, 100, 375, 250, &whole_views.now);
    assert_eq!(views.start, 100, "the window starts at the read");
    for _ in 0..5 {
        down_key(&mut tui, &views);
    }
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (100, 5),
        "five drawn lines inside the window"
    );
    page_key(&mut tui, &views, false);
    let paged_views = live_read(&db, run, &tui, &whole_views.now);
    settle_end(&mut tui, &paged_views);
    let reached = (tui.live_offset, tui.live_line);
    assert_eq!(
        reached,
        (99, 24),
        "PageUp keeps the five lines still owed above the window"
    );
    let whole_tui = whole_page_up(run, 100, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (99, 24),
        "the whole read's PageUp"
    );
    assert_eq!(
        live_frame_for_test(&tui, &paged_views, run, 375, 250),
        live_frame_for_test(&whole_tui, &whole_views, run, 375, 250),
        "the paged frame is the whole read's"
    );
}

/// The participant level's page jump moves the ten rows it names from the
/// absolute offset, not to the edge of the held window (R2): holding rows
/// 0..200 with the scroll at 195, `PageDown` leaves 205, and holding rows
/// 100..300 with it at 105, `PageUp` leaves 95, both the whole read's
/// offsets.
#[test]
fn gate_review_participant_page_jump_keeps_ten_rows() {
    use crate::tui::page_key;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "gate participant page jump";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let whole_views = participant_whole_views(&db, run, &events, subject.clone());
    let lower = held_lower_page(&db, run, &subject);
    tui.offset = 195;
    page_key(&mut tui, &lower, true);
    assert_eq!(tui.offset, 205, "PageDown keeps its ten rows");
    let (mut whole_tui, _) = participant_console(run, &whole);
    whole_tui.offset = 195;
    page_key(&mut whole_tui, &whole_views, true);
    assert_eq!(whole_tui.offset, 205, "the whole read's PageDown");
    let upper = console_rows_at(&db, run, Some(subject.clone()), false, 200, 100);
    assert_eq!(
        (upper.start, upper.more),
        (100, true),
        "a window that starts below the top"
    );
    tui.offset = 105;
    page_key(&mut tui, &upper, false);
    assert_eq!(tui.offset, 95, "PageUp keeps its ten rows");
    let (mut whole_up, _) = participant_console(run, &whole);
    whole_up.offset = 105;
    page_key(&mut whole_up, &whole_views, false);
    assert_eq!(whole_up.offset, 95, "the whole read's PageUp");
}

/// After a large participant `G`, movement keeps the stream's logical
/// position and its current end, not the narrowed display window's bounds
/// (R3): 66,000 checkpoints settle to 65,999 over the window
/// [65,500,65,600), where `Down` is the top and `Up` is 65,998, and the
/// frame after the reread is the whole read's clamped frame, starting at
/// turn 65,536.
#[test]
fn gate_review_large_logical_end_is_independent_of_display_page() {
    use crate::tui::{checkpoint_offset, down_key, settle_end, up_key};
    let far = far_work_of(66_000);
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let now = events.last().unwrap().recorded_at.clone();
    let (mut down_tui, down_subject) = participant_console(run, whole);
    let start = participant_page(db, run, &down_subject, &down_tui);
    let views = settle_participant_end(db, run, &down_subject, &mut down_tui, &start, &now);
    assert_eq!(down_tui.offset, 65_999, "G's logical end");
    assert_eq!(
        (views.start, views.more, views.end),
        (65_500, true, Some(66_000)),
        "the narrowed display window under the stream's current end"
    );
    down_key(&mut down_tui, &views);
    assert_eq!(down_tui.offset, 0, "Down wraps at the stream's end");
    let (mut tui, subject) = participant_console(run, whole);
    let views = settle_participant_end(db, run, &subject, &mut tui, &start, &now);
    assert_eq!(
        checkpoint_offset(&tui),
        u16::MAX as usize,
        "today's display rank"
    );
    up_key(&mut tui, &views);
    assert_eq!(tui.offset, 65_998, "Up keeps the logical rank");
    let mut views = participant_page(db, run, &subject, &tui);
    views.now = now.clone();
    settle_end(&mut tui, &views);
    let paged = participant_frame_whole_read(run, whole, far.whole_views(), 65_998, &tui, &views);
    let starts_at_today = rows_of_text(&paged)
        .into_iter()
        .any(|line| line.contains("│65536  turn"));
    assert!(
        starts_at_today,
        "the display starts at today's clamped rank"
    );
}

/// After `G` and an ordinary reread, the participant level keeps the
/// stream's current end across the narrowed display page (R3, F1): 66,000
/// checkpoints settle to 65,998 after `Up`, the reread asks the display
/// rank 65,535 without losing the rank, and `Down` twice wraps at 66,000
/// to 0 as the whole read's does, never to 66,000.
#[test]
fn chief_confirm_large_end_after_reread_wraps() {
    use crate::tui::{bottom_key, checkpoint_offset, down_key, settle_end, up_key};
    use brokkr_runtime::realms::Hearth;
    let far = far_work_of(66_000);
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let now = events.last().unwrap().recorded_at.clone();
    let (mut tui, subject) = participant_console(run, whole);
    // The console's source keeps one Kept across refreshes; a fresh helper
    // per key could not carry the rank the end read answered (F1).
    let mut source = crate::tui_read::tui_source(vec![Hearth {
        realms: Vec::new(),
        journal: db.clone(),
    }]);
    let mut read = |tui: &Tui, force: bool| -> Option<Views> {
        let ask = tui::Ask {
            run: Some(run),
            subject: Some(subject.clone()),
            run_level: false,
            force,
            fleet: true,
            tab: 0,
            window: Default::default(),
            live_rows: crate::tui::asked_rows(tui),
            live_offset: checkpoint_offset(tui),
        };
        source(ask).unwrap().map(|mut views| {
            views.now = now.clone();
            views
        })
    };
    let views = read(&tui, true).expect("the first frame is read");
    assert_eq!(views.start, 0, "the level opens on its oldest page");
    bottom_key(&mut tui, &views);
    let views = read(&tui, false).expect("the end is a new read");
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 65_999, "G's logical end");
    assert_eq!(views.end, Some(66_000), "the read's whole rank");
    up_key(&mut tui, &views);
    let mut views = read(&tui, false).expect("the narrowed page is a new read");
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 65_998, "Up keeps the logical rank");
    assert_eq!(views.end, Some(66_000), "the rank survives the reread");
    down_key(&mut tui, &views);
    if let Some(next) = read(&tui, false) {
        views = next;
    }
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 65_999, "Down keeps the logical rank");
    down_key(&mut tui, &views);
    if let Some(next) = read(&tui, false) {
        views = next;
    }
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 0, "Down wraps at the stream's current end");
    let (mut whole_tui, _) = participant_console(run, whole);
    let whole_views = far.whole_views();
    whole_tui.offset = 65_998;
    down_key(&mut whole_tui, whole_views);
    down_key(&mut whole_tui, whole_views);
    assert_eq!(whole_tui.offset, 0, "the whole read's end wraps");
}

/// A live `PageDown` that crosses the held window keeps the drawn lines
/// it still owes (F2): holding rows [100,350) with the cursor at
/// (349,24), the read reaches (350,5) as the whole read does, not the
/// window's first line at (350,0).
#[test]
fn chief_confirm_wrapped_page_down_keeps_remaining_distance() {
    use crate::tui::{page_key, settle_end};
    let run = "chief wrapped page down";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = live_at(&db, run, 100, 375, 250, &whole_views.now);
    assert_eq!(views.start, 100, "the window starts at the read");
    tui.live_offset = 349;
    tui.live_line = 24;
    page_key(&mut tui, &views, true);
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_end(&mut tui, &next);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (350, 5),
        "PageDown keeps the five lines past the window"
    );
    let mut whole_tui = live_tui(run, 100, 375, 250);
    whole_tui.live_offset = 349;
    whole_tui.live_line = 24;
    page_key(&mut whole_tui, &whole_views, true);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (350, 5),
        "the whole read's PageDown"
    );
    assert_eq!(
        live_frame_for_test(&tui, &next, run, 375, 250),
        live_frame_for_test(&whole_tui, &whole_views, run, 375, 250),
        "the paged frame is the whole read's"
    );
}

/// A participant `PageDown` whose absolute destination lies past the
/// stream's unknown end resolves to the discovered end's last row (F3):
/// holding [0,200) at offset 195 with a page beyond, the read settles at
/// 200 and still paints turn 201, as the whole read does, rather than
/// leaving 205 and an empty pane.
#[test]
fn chief_confirm_page_down_over_unknown_end_retains_last_row() {
    use crate::tui::{page_key, settle_end};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "chief page down end";
    at_work_in(&db, &[(run, 201, true)]);
    let (_, events, _) = console_journal(&db, run);
    let now = events.last().unwrap().recorded_at.clone();
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let lower = held_lower_page(&db, run, &subject);
    tui.offset = 195;
    page_key(&mut tui, &lower, true);
    let mut views = participant_page(&db, run, &subject, &tui);
    views.now = now.clone();
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 200, "the destination resolves to the last row");
    assert_eq!(views.end, Some(201), "the read discovered the whole rank");
    assert_eq!(
        first_turn(&views, "eff").as_deref(),
        Some("201"),
        "the last row is still painted"
    );
    let whole_views = participant_whole_views(&db, run, &events, subject);
    let (mut whole_tui, _) = participant_console(run, whole_views.run.as_ref().unwrap());
    whole_tui.offset = 195;
    page_key(&mut whole_tui, &whole_views, true);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(whole_tui.offset, 200, "the whole read's PageDown");
    assert_eq!(
        frame_for_test(&tui, &views, 80, 30),
        frame_for_test(&whole_tui, &whole_views, 80, 30),
        "the paged frame keeps the whole read's last row"
    );
}

/// A previous seat's pinned working member extends no live stream (F4):
/// after `eff:a`'s 300 turns and its seat succeeds into the next phase,
/// `p2:c` records 30 turns and the live column paints only the current
/// hire's seat. Down 29 times agrees with the whole read, and one more
/// wraps to the top as the whole read does.
#[test]
fn chief_confirm_live_end_ignores_a_previous_seats_pinned_member() {
    use crate::tui::{down_key, settle_end};
    let run = "chief previous seat";
    let (_dir, db, _events, whole_views) = console_of(run, retired_panel(run));
    let whole = whole_views.run.as_ref().unwrap();
    assert!(
        whole
            .participants
            .iter()
            .any(|part| part.key == "eff:a" && part.working),
        "the earlier seat's member is still pinned working"
    );
    assert_eq!(
        brokkr_view::working_checkpoints(whole, "p2").len(),
        30,
        "the current hire paints only its own rows"
    );
    let (mut tui, mut views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    let mut whole_tui = live_tui(run, 0, 375, 30);
    let press_down = |tui: &mut Tui, views: &mut Views, whole_tui: &mut Tui| {
        down_key(tui, views);
        *views = live_read(&db, run, tui, &whole_views.now);
        settle_end(tui, views);
        down_key(whole_tui, &whole_views);
        settle_end(whole_tui, &whole_views);
    };
    for _ in 0..29 {
        press_down(&mut tui, &mut views, &mut whole_tui);
    }
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (29, 0),
        "the paged cursor reaches the current seat's last row"
    );
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (29, 0),
        "the whole cursor reaches it too"
    );
    assert_eq!(
        live_frame_for_test(&tui, &views, run, 375, 30),
        live_frame_for_test(&whole_tui, &whole_views, run, 375, 30),
        "the paged frame is the whole read's"
    );
    press_down(&mut tui, &mut views, &mut whole_tui);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the paged cursor wraps at the current seat's end"
    );
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (0, 0),
        "the whole cursor wraps there"
    );
    assert_eq!(
        live_frame_for_test(&tui, &views, run, 375, 30),
        live_frame_for_test(&whole_tui, &whole_views, run, 375, 30),
        "the wrapped frame is the whole read's"
    );
}

/// The fold-legal journal `run` in which `eff:a` recorded 300 turns and
/// its panel member finished working, `eff` succeeded into `next`, and
/// the new hire `p2:c` recorded 30: the earlier seat's pinned member
/// still folds as working, but the live column paints only `p2`'s seat.
pub(super) fn retired_panel(run: &str) -> Vec<EventEnvelope> {
    let mut steps = in_flight_as(run, "eff", "a");
    steps[2].1["seat"] = json!("eff");
    steps[3].1["boundary"] = json!([
        {"member": null, "boundary": "harness"},
        {"member": "a", "boundary": "open"},
    ]);
    for turn in 1..=300 {
        steps.push(journals::checkpoint(
            "eff",
            "a",
            json!({"member": "a", "step": "seat-turn", "turn": turn}),
        ));
    }
    steps.push(journals::checkpoint(
        "eff",
        "a",
        json!({"member": "a", "step": "panel-member-finished", "outcome": "working"}),
    ));
    steps.push((
        EventType::EffectSucceeded,
        json!({"effect_id": "eff", "attempt_id": "a", "result": {"result": "done"}}),
    ));
    steps.push((
        EventType::TransitionDecided,
        json!({"from": "work", "result": "done", "next": "next", "rule_id": "R6"}),
    ));
    steps.push((EventType::PhaseEntered, json!({"phase": "next"})));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "p2", "seat": "p2", "phase": "next"}),
    ));
    steps.push((
        EventType::EffectStarted,
        json!({"effect_id": "p2", "attempt_id": "c", "boundary": [
            {"member": null, "boundary": "harness"},
            {"member": "c", "boundary": "open"},
        ]}),
    ));
    for turn in 1..=30 {
        steps.push(journals::checkpoint(
            "p2",
            "c",
            json!({"member": "c", "step": "seat-turn", "turn": turn}),
        ));
    }
    chained(run, journals::journal(steps))
}

/// A page request held by the keys is resolved only by the frame that
/// answers it (R4): with the next read still out, the held window does
/// not consume a PageUp across it or a `G`, so the answer reaches
/// (99, 24) and (999, 28) as the whole read does, never (100, 0) or the
/// old window's foot.
#[test]
fn gate_review_pending_page_request_waits_for_its_answer() {
    use crate::tui::{bottom_key, down_key, page_key, settle_when};
    let run = "gate pending page request";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = live_at(&db, run, 100, 375, 250, &whole_views.now);
    assert_eq!(views.start, 100, "the held window starts at the read");
    for _ in 0..5 {
        down_key(&mut tui, &views);
    }
    page_key(&mut tui, &views, false);
    settle_when(&mut tui, &views, false);
    assert_eq!(tui.held.len(), 1, "a read still out holds the crossing");
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &next, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (99, 24),
        "the answer resolves the whole distance"
    );
    let whole_tui = whole_page_up(run, 100, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (99, 24),
        "the whole read's PageUp"
    );
    let (mut bottom, held) = live_at(&db, run, 0, 375, 250, &whole_views.now);
    bottom_key(&mut bottom, &held);
    settle_when(&mut bottom, &held, false);
    assert_eq!(bottom.live_offset, usize::MAX, "G waits for its read");
    let end = live_read(&db, run, &bottom, &whole_views.now);
    settle_when(&mut bottom, &end, true);
    assert_eq!(
        (bottom.live_offset, bottom.live_line),
        (999, 28),
        "G resolves at the stream's end"
    );
}

/// Two Downs pressed before the page they ask for is read keep both moves
/// (R5): the unwrapped panel at 375x30 holds [0, 50) with the cursor at
/// (49, 0), both moves are held, and the answer applies them in order to
/// 51, as the whole read's does, rather than clamping back to 50.
#[test]
fn gate_review_two_downs_before_answer_accumulate() {
    use crate::tui::{down_key, settle_end};
    let run = "gate two downs";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    assert_eq!(views.start, 0, "the window starts at the top");
    assert!(views.more, "the stream continues past the window");
    tui.live_offset = 49;
    tui.live_line = 0;
    down_key(&mut tui, &views);
    down_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "both moves are one held run");
    assert_eq!(tui.held[0].count, 2, "the run counts both moves");
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_end(&mut tui, &next);
    assert_eq!((tui.live_offset, tui.live_line), (51, 0), "both moves land");
    let mut whole_tui = live_tui(run, 49, 375, 30);
    down_key(&mut whole_tui, &whole_views);
    down_key(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (51, 0),
        "the whole read's two Downs"
    );
}

/// A forward crossing past the stream's discovered end stops at the last
/// drawn line (R6): the 201-checkpoint panel at 375x200, held [0, 200)
/// with the cursor at (195, 0), settles PageDown at (200, 0), the whole
/// read's cursor, rather than (200, 5); at width 300 the frames agree too.
#[test]
fn gate_review_end_crossing_stops_at_the_last_drawn_line() {
    use crate::tui::{page_key, settle_end, settle_when};
    let run = "gate end crossing";
    let target = "x".repeat(40);
    let (_dir, db, _events, whole_views) =
        console_of(run, panel_with(run, 101, 100, Some(&target)));
    let mut tui = live_tui(run, 195, 375, 200);
    let mut views = console_rows_at(&db, run, None, false, 200, 195);
    views.now = whole_views.now.clone();
    assert_eq!(
        (views.start, views.more),
        (0, true),
        "the held window has a page beyond it"
    );
    page_key(&mut tui, &views, true);
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &next, true);
    let reached = (tui.live_offset, tui.live_line);
    assert_eq!(
        reached,
        (200, 0),
        "the crossing stops at the last drawn line"
    );
    let mut whole_tui = live_tui(run, 195, 375, 200);
    page_key(&mut whole_tui, &whole_views, true);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (200, 0),
        "the whole read's PageDown"
    );
    assert_eq!(
        live_frame_for_test(&tui, &next, run, 375, 200),
        live_frame_for_test(&whole_tui, &whole_views, run, 375, 200),
        "the paged frame is the whole read's"
    );
    assert_eq!(
        live_frame_for_test(&tui, &next, run, 300, 200),
        live_frame_for_test(&whole_tui, &whole_views, run, 300, 200),
        "the resized frame keeps the last row's first line"
    );
}

/// A hearth switch resets the live column's row's line with its row (R7):
/// at the first hearth's top, five Downs leave (0, 5); the real `]` moves
/// to the second hearth, whose live column paints from (0, 0), the fresh
/// selection's frame, not from inside its first checkpoint.
#[test]
fn gate_review_hearth_switch_resets_the_live_line() {
    use crate::tui::{char_key, down_key};
    let run1 = "gate hearth one";
    let run2 = "gate hearth two";
    let (_dir1, db1, _events1, whole1) = wrapped_panel_console(run1);
    let (_dir2, db2, events2, _whole2) = wrapped_panel_console(run2);
    let tabs = vec!["one".to_string(), "two".to_string()];
    let mut tui = Tui::over(None, tabs.clone(), 0);
    tui.pane = 2;
    (tui.width, tui.height) = (375, 250);
    tui.cursor[0] = Some(run1.to_string());
    let views1 = live_read(&db1, run1, &tui, &whole1.now);
    for _ in 0..5 {
        down_key(&mut tui, &views1);
    }
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 5),
        "five drawn lines into the first hearth"
    );
    char_key(&mut tui, &views1, ']');
    assert_eq!(tui.tab, 1, "the real hearth key moved");
    assert_eq!(tui.live_line, 0, "the new hearth starts at its first line");
    let views2 = whole_console(&db2, run2, &events2);
    tui.cursor[0] = Some(run2.to_string());
    let mut fresh = Tui::over(None, tabs, 1);
    fresh.pane = 2;
    (fresh.width, fresh.height) = (375, 250);
    fresh.cursor[0] = Some(run2.to_string());
    assert_eq!(
        live_frame_for_test(&tui, &views2, run2, 375, 250),
        live_frame_for_test(&fresh, &views2, run2, 375, 250),
        "the switched-to hearth paints the fresh selection's frame"
    );
}

/// The carried participant rank advances by the checkpoints that landed
/// while the held page still has a newer cursor (R8): after `G` knows the
/// 66,000-row end, turn 66,001 lands, the level reads its oldest page
/// again, and the read still reports 66,001, so `Down` from the end wraps
/// to 0 as the whole read does rather than stepping past it.
#[test]
fn gate_review_carried_rank_advances_with_arrivals() {
    use crate::tui::down_key;
    let far = far_work_of(66_000);
    let (run, events) = (far.run, &far.events);
    let (_dir, db) = far.copy();
    let now = events.last().unwrap().recorded_at.clone();
    let (mut tui, _subject) = participant_of(run, &far.whole);
    let key = Console::Participant("eff".to_string());
    let stored = Store::open_read_only(&db).unwrap();
    let end = crate::run_pages::Reading {
        rows: 30,
        offset: usize::MAX,
        visible: 30,
    };
    let (first, _) = super::console_with(&stored, run, key.clone(), None, None, end).unwrap();
    assert_eq!(first.end(&key), Some(66_000), "the known end");
    let mut head = stored.head_hash(run).unwrap();
    land(&db, run, &mut head, 66_001, "2026-01-02T00:00:00Z");
    drop(Store::open(&db).unwrap());
    let moved = stored.head_hash(run).unwrap();
    tui.offset = 0;
    let reading = crate::run_pages::Reading {
        rows: 30,
        offset: 0,
        visible: 30,
    };
    let (next, _) = super::console_with(
        &stored,
        run,
        key.clone(),
        Some(first),
        Some(&moved),
        reading,
    )
    .unwrap();
    assert_eq!(
        next.end(&key),
        Some(66_001),
        "the carried rank advances by the landed checkpoint"
    );
    let mut views = Views::empty();
    views.now = now;
    views.run = Some(next.view());
    views.start = next.start(&key);
    views.more = next.more(&key);
    views.end = next.end(&key);
    tui.offset = 66_000;
    down_key(&mut tui, &views);
    assert_eq!(tui.offset, 0, "Down wraps at the advanced end");
}

/// M1 (rulings 5 and 9): a `G` whose end read is out, then `Up`, settles
/// both against the end's window, as the whole read does. Two working
/// members' 1,000 unwrapped checkpoints at 375x30 reach (998, 0), never
/// the old window's crossing at (48, 0).
#[test]
fn chief_review_live_pending_end_then_up() {
    use crate::tui::{bottom_key, settle_when, up_key};
    let run = "chief live end then up";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    bottom_key(&mut tui, &views);
    assert_eq!(tui.live_offset, usize::MAX, "G asks the absolute end");
    tui.pending = true;
    up_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "Up waits behind G's read");
    let end = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &end, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (998, 0),
        "the held Up lands at the end's window"
    );
    let mut whole_tui = live_tui(run, 0, 375, 30);
    bottom_key(&mut whole_tui, &whole_views);
    up_key(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (998, 0),
        "the whole read's G then Up"
    );
}

/// M1 (rulings 5 and 9): the participant level's `G` then `Up` with the
/// end read out reaches 998 of 1,000, as the whole read does, never the
/// `G`'s own end at 999.
#[test]
fn chief_review_participant_pending_end_then_up() {
    use crate::tui::{bottom_key, settle_when, up_key};
    let far = far_work_of(1_000);
    let (run, db, events, whole) = (far.run, &far.db, &far.events, &far.whole);
    let (mut tui, subject, views) = participant_end(db, run, whole);
    assert_eq!(tui.offset, usize::MAX, "G asks the absolute end");
    tui.pending = true;
    up_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "Up waits behind G's read");
    let end = participant_page(db, run, &subject, &tui);
    settle_when(&mut tui, &end, true);
    assert_eq!(tui.offset, 998, "the held Up lands at the end's window");
    let (mut whole_tui, _) = participant_console(run, whole);
    let whole_views = participant_whole_views(db, run, events, subject);
    bottom_key(&mut whole_tui, &whole_views);
    up_key(&mut whole_tui, &whole_views);
    assert_eq!(whole_tui.offset, 998, "the whole read's G then Up");
}

/// M2 (rulings 5 and 9): four `PageDown`s before an answer are held and
/// applied one window at a time, reaching (89, 0) on the 1,000-row panel
/// at 375x30, as the whole read does, never the old window's (79, 0).
#[test]
fn chief_review_four_pending_page_downs() {
    use crate::tui::page_key;
    let run = "chief four page downs";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.live_offset = 49;
    tui.live_line = 0;
    page_key(&mut tui, &views, true);
    tui.pending = true;
    for _ in 0..3 {
        page_key(&mut tui, &views, true);
    }
    assert_eq!(tui.held.len(), 1, "the four moves are one held run");
    assert_eq!(tui.held[0].count, 4, "the run counts four moves");
    settle_held_live(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (89, 0),
        "the four held PageDowns land"
    );
    let mut whole_tui = live_tui(run, 0, 375, 30);
    whole_tui.live_offset = 49;
    whole_tui.live_line = 0;
    for _ in 0..4 {
        page_key(&mut whole_tui, &whole_views, true);
    }
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (89, 0),
        "the whole read's four PageDowns"
    );
}

/// M2 (rulings 5 and 9): eleven `Up`s before an answer, from (100, 0) at
/// 375x250, are held and applied against the page above, reaching
/// (89, 0) as the whole read does, never the old window's (90, 0).
#[test]
fn chief_review_eleven_pending_ups() {
    use crate::tui::up_key;
    let run = "chief eleven ups";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 100, 375, 250, &whole_views.now);
    assert_eq!(views.start, 100, "the window starts at the cursor");
    up_key(&mut tui, &views);
    tui.pending = true;
    for _ in 0..10 {
        up_key(&mut tui, &views);
    }
    assert_eq!(tui.held.len(), 1, "the eleven moves are one held run");
    assert_eq!(tui.held[0].count, 11, "the run counts eleven moves");
    settle_held_live(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (89, 0),
        "the eleven held Ups land"
    );
    let mut whole_tui = live_tui(run, 100, 375, 250);
    for _ in 0..11 {
        up_key(&mut whole_tui, &whole_views);
    }
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (89, 0),
        "the whole read's eleven Ups"
    );
}

/// A run of one held key is one entry with a count, and past the stated
/// bound the oldest run is dropped with a status-line word (decision 0071
/// ruling 5).
#[test]
fn chief_review_held_runs_coalesce_and_the_bound_drops_the_oldest() {
    use crate::tui::{hold, Held, Step, HELD_BOUND};
    let mut tui = Tui::new(None);
    for _ in 0..3 {
        hold(&mut tui, Step::PageDown);
    }
    assert_eq!(
        tui.held,
        [Held {
            step: Step::PageDown,
            count: 3,
            end: None,
        }],
        "a run of one key is one entry"
    );
    for run in 0..HELD_BOUND {
        let step = match run % 2 {
            0 => Step::Up,
            _ => Step::Down,
        };
        hold(&mut tui, step);
    }
    assert_eq!(tui.held.len(), HELD_BOUND, "the queue keeps its bound");
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Up),
        "the oldest run was dropped"
    );
    assert!(
        tui.status
            .as_deref()
            .is_some_and(|status| status.contains("dropped")),
        "the status line says so"
    );
}

/// A `G` held behind another read asks the absolute end when that answer
/// lands, never the held window's last rank (decision 0071 ruling 5).
#[test]
fn chief_review_held_end_asks_the_absolute_end() {
    use crate::tui::{bottom_key, settle_when};
    let run = "chief held end";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    bottom_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "G waits behind the read");
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &next, true);
    assert_eq!(tui.live_offset, usize::MAX, "G asks the absolute end");
    let end = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &end, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (999, 0),
        "G reaches the stream's end"
    );
    let mut whole_tui = live_tui(run, 0, 375, 30);
    bottom_key(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (999, 0),
        "the whole read's G"
    );
}

/// A held page move belongs to the view it was pressed on: a `PageDown`
/// at the participant level, then `Esc` to the fleet before its answer,
/// leaves the fleet's cursor where it was (decision 0071 ruling 5).
#[test]
fn chief_review_level_change_drops_held_page_down() {
    use crate::tui::{escape_key, page_key, settle_when};
    let run = "chief level change";
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    at_work_in(&db, &[(run, 120, true), ("second", 10, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let views = participant_page(&db, run, &subject, &tui);
    tui.pending = true;
    page_key(&mut tui, &views, true);
    assert_eq!(tui.held.len(), 1, "the participant's PageDown is held");
    escape_key(&mut tui, &views);
    escape_key(&mut tui, &views);
    assert_eq!(
        (tui.level, tui.held.len()),
        (tui::Level::Runs, 0),
        "the fleet drops the movement of the view it left"
    );
    let fleet = console_rows(&db, run, None, false, 0);
    let before = tui.cursor[0].clone();
    settle_when(&mut tui, &fleet, true);
    assert_eq!(tui.cursor[0], before, "the fleet's cursor did not move");
}
