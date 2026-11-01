//! The console's coverage edges (#753 coverage round 753c8): the keys a
//! paged live column binds to `j`/`k`, a column held whole moving over its
//! window, an empty or cursor-less laid window, and the settlement guards
//! a key path cannot leave. Each real entry drives `apply`; the guards a
//! caller's invariant makes unreachable in a frame are called at their own
//! seam with the laid window the caller would pass.

use super::columns_tests::{
    fleet_reading, held_view as parked_view, reviewing_view, selecting, REVIEWING,
};
use super::fleet_tests::HELD;
use super::state::held_view as held_view_of;
use super::tests::views as fleet_views;
use super::*;

/// A laid window whose rows start where `rows` says and which is a stream
/// (or the findings) as `streamed` says.
fn laid(rows: Vec<usize>, streamed: bool) -> Laid {
    Laid {
        title: String::new(),
        lines: Vec::new(),
        rows,
        streamed,
    }
}

/// The fleet at `run` with its third column focused: the one key walk a
/// live or findings proof shares.
fn live_column(run: &str, view: brokkr_view::RunView) -> (Views, Tui) {
    let views = fleet_reading(view);
    let mut tui = selecting(run);
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(focused(&tui, &views), FleetColumn::Live);
    (views, tui)
}

/// The fleet at its findings column, focused on the live column.
fn findings_column() -> (Views, Tui) {
    let (views, tui) = live_column(HELD, parked_view());
    assert!(
        !laid_live(&tui, &views).unwrap().streamed,
        "a parked run's third column is its findings"
    );
    (views, tui)
}

/// The console's live-frame helper paints nothing when the frame has no
/// live column and when it has no row for the run: both absences are an
/// empty string, never a panic or another run's frame.
#[test]
fn the_live_frame_helper_paints_no_absent_column_or_run() {
    let views = fleet_views();
    let tui = Tui::new(None);
    assert_eq!(
        live_frame_for_test(&tui, &views, "run-7", 10, 10),
        "",
        "no live column at ten columns"
    );
    assert_eq!(
        live_frame_for_test(&tui, &views, "not-a-run", 375, 30),
        "",
        "no row for the run"
    );
}

/// `j` and `k` move a paged live column as the arrows do, and while the
/// operator is typing they are the search text, not movement (ruling 5).
#[test]
fn a_paged_live_column_moves_on_j_and_k() {
    let (views, mut tui) = live_column(REVIEWING, reviewing_view());
    assert!(
        paged_window(&tui, &views),
        "the live column is a paged stream"
    );
    tui.live_offset = 0;
    apply(&mut tui, &views, Key::Char('j'));
    assert_eq!((tui.live_offset, tui.live_line), (1, 0), "j moves down");
    apply(&mut tui, &views, Key::Char('k'));
    assert_eq!((tui.live_offset, tui.live_line), (0, 0), "k moves up");
    tui.pending = true;
    apply(&mut tui, &views, Key::Char('j'));
    assert_eq!(
        tui.held.last().map(|held| held.step),
        Some(Step::Down),
        "a pending j is a Down"
    );
    tui.held.clear();
    tui.held_for = None;
    tui.pending = false;
    apply(&mut tui, &views, Key::Char('g'));
    assert_eq!(tui.live_offset, 0, "g is the top");
    apply(&mut tui, &views, Key::Char('G'));
    assert!(tui.live_offset > 0, "G is a rank");
    tui.live_offset = 0;
    tui.typing = true;
    apply(&mut tui, &views, Key::Char('j'));
    assert_eq!(tui.filter, "j", "a typed j is the filter's");
    assert_eq!(tui.live_offset, 0, "a typed j moves nothing");
    tui.filter.clear();
    for key in [Key::Char('k'), Key::Char('g'), Key::Char('G')] {
        apply(&mut tui, &views, key);
    }
    assert_eq!(tui.filter, "kgG", "a typed key is the filter's");
    assert_eq!(tui.live_offset, 0, "a typed key moves nothing");
}

/// A column held whole moves over its window: `Up` at the top asks the
/// window's end, `Down` at the last line asks the page beyond it or wraps
/// to the top, and `PageUp`/`PageDown` stop at the window's ends (R2).
#[test]
fn a_held_whole_column_wraps_and_pages_at_its_window() {
    let (views, mut tui) = findings_column();
    let laid = laid_live(&tui, &views).unwrap();
    let end = laid.lines.len();
    assert!(end > PAGE, "the findings run longer than a page");
    let mut more = views;
    more.more = true;
    tui.live_offset = 0;
    apply(&mut tui, &more, Key::Up);
    assert_eq!(tui.live_offset, usize::MAX, "Up at the top asks the end");
    more.more = false;
    tui.live_offset = 0;
    apply(&mut tui, &more, Key::Up);
    assert_eq!(tui.live_offset, end - 1, "Up at the top wraps to the end");
    tui.live_offset = 5;
    apply(&mut tui, &more, Key::Up);
    assert_eq!(tui.live_offset, 4, "Up moves one line");
    tui.live_offset = 0;
    apply(&mut tui, &more, Key::Down);
    assert_eq!(tui.live_offset, 1, "Down moves one line");
    more.more = true;
    tui.live_offset = end - 1;
    apply(&mut tui, &more, Key::Down);
    assert_eq!(tui.live_offset, end, "Down asks the page beyond the window");
    more.more = false;
    tui.live_offset = end - 1;
    apply(&mut tui, &more, Key::Down);
    assert_eq!(tui.live_offset, 0, "Down at the end wraps to the top");
    tui.live_offset = end;
    apply(&mut tui, &more, Key::PageUp);
    assert_eq!(tui.live_offset, end - 1, "PageUp returns to the last line");
    tui.live_offset = 5;
    apply(&mut tui, &more, Key::PageUp);
    assert_eq!(tui.live_offset, 0, "PageUp moves a page from the top");
    tui.live_offset = 0;
    apply(&mut tui, &more, Key::PageDown);
    assert_eq!(tui.live_offset, PAGE.min(end - 1), "PageDown moves a page");
    apply(&mut tui, &more, Key::Char('g'));
    assert_eq!(tui.live_offset, 0, "g is the window's top");
    more.more = true;
    apply(&mut tui, &more, Key::Char('G'));
    assert_eq!(tui.live_offset, usize::MAX, "G asks the absolute end");
    more.more = false;
    tui.live_offset = 0;
    apply(&mut tui, &more, Key::Char('G'));
    assert_eq!(tui.live_offset, end - 1, "G is the window's end");
}

/// A step whose cursor the laid window no longer holds moves nothing:
/// the continuation a resize removed is not a row to step from (M1).
#[test]
fn a_live_step_without_a_laid_cursor_moves_nothing() {
    let views = fleet_views();
    let mut tui = Tui::new(Some("run-7".to_string()));
    tui.live_offset = 999;
    tui.live_line = 3;
    scroll_live(&mut tui, &views, &laid(vec![0, 5], true), Step::Down, false);
    scroll_live(&mut tui, &views, &laid(vec![0, 5], true), Step::Up, false);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (999, 3),
        "the cursor is outside the window"
    );
}

/// A laid window with no row is the top: a step from it lands at the
/// first line rather than underflowing the row table.
#[test]
fn an_empty_laid_window_stops_at_the_top() {
    let views = fleet_views();
    let mut tui = Tui::new(Some("run-7".to_string()));
    tui.live_offset = 5;
    tui.live_line = 2;
    scroll_live(&mut tui, &views, &laid(vec![0], true), Step::Down, false);
    assert_eq!((tui.live_offset, tui.live_line), (0, 0));
}

/// A stream of no row is exact to move in, and a cursor the window does
/// not hold is not: `live_can` is the guard the shell holds keys behind.
#[test]
fn a_live_step_is_exact_without_a_window_and_held_without_a_cursor() {
    let views = fleet_views();
    let mut tui = Tui::new(Some("run-7".to_string()));
    tui.live_offset = 0;
    assert!(
        live_can(&tui, &views, &laid(vec![0], true), Step::Down),
        "no row to leave"
    );
    assert!(
        live_can(&tui, &views, &laid(vec![0, 5], true), Step::Top),
        "the top needs no continuation"
    );
    tui.live_offset = 9;
    assert!(
        !live_can(&tui, &views, &laid(vec![0, 5], true), Step::Down),
        "a lost continuation holds the step"
    );
}

/// The last drawn line of a row: none for the findings, and none for a
/// laid table with no row, never an underflow.
#[test]
fn the_last_live_line_is_none_for_findings_and_an_empty_window() {
    let views = fleet_views();
    let mut tui = Tui::new(Some("run-7".to_string()));
    assert_eq!(
        last_line_of(&laid(vec![0, 3], false), &tui, &views),
        0,
        "the findings are not a stream"
    );
    assert_eq!(
        last_line_of(&laid(vec![0], true), &tui, &views),
        0,
        "an empty window has no line"
    );
    tui.live_offset = 1;
    assert_eq!(
        last_line_of(&laid(vec![0, 2, 5], true), &tui, &views),
        2,
        "the row's foot is its last drawn line"
    );
}

/// A settled end on a column that is now the findings resolves to
/// nothing: the page it would end is not a stream, so the cursor stays
/// where the operator left it (M3).
#[test]
fn a_finished_live_column_resolves_no_end() {
    let (views, mut tui) = findings_column();
    tui.live_offset = usize::MAX;
    tui.live_line = 0;
    settle_end(&mut tui, &views);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (usize::MAX, 0),
        "the findings have no stream end to resolve"
    );
    let mut ended = views;
    ended.end = Some(3);
    settle_end(&mut tui, &ended);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (2, 0),
        "the end lands on the first line of a findings row"
    );
}

/// A settled end with no detail row for the selection lands at the first
/// line: there is no laid row to put the cursor on, and no panic.
#[test]
fn a_settled_end_without_a_detail_row_lands_at_the_first_line() {
    let (views, mut tui) = findings_column();
    let mut ended = views;
    ended.end = Some(3);
    tui.cursor[0] = Some("not-a-run".to_string());
    tui.live_offset = usize::MAX;
    tui.live_line = 7;
    settle_end(&mut tui, &ended);
    assert_eq!((tui.live_offset, tui.live_line), (2, 0));
}

/// A run-level step with no laid window, and one on the findings, is
/// exact: only a streamed live column can be paged.
#[test]
fn a_run_level_step_without_a_stream_is_exact() {
    let views = fleet_views();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some("not-a-run".to_string());
    assert!(can_move(&tui, &views, Step::Down), "no laid window");
    let (views, tui) = findings_column();
    assert!(can_move(&tui, &views, Step::Down), "the findings are whole");
    let streamed = fleet_reading(reviewing_view());
    let listed = selecting(REVIEWING);
    assert!(
        can_move(&listed, &streamed, Step::Down),
        "a streamed run's list is exact"
    );
}

/// A participant pane with no seat in the answer is inside whatever
/// window it has, and a run level with no laid window is inside it too:
/// the shell drops neither queue over a missing model.
#[test]
fn a_window_with_no_model_holds_its_cursor() {
    let views = fleet_views();
    let mut tui = Tui::new(None);
    tui.level = Level::Participant;
    tui.pane = 0;
    tui.seat = Some("gone".to_string());
    assert!(in_window(&tui, &views), "no seat is no window to leave");
    tui.level = Level::Runs;
    tui.cursor[0] = Some("not-a-run".to_string());
    assert!(in_window(&tui, &views), "no laid window is no window");
}

/// Taking an application off an empty queue is nothing, not a panic: the
/// guard every resolution relies on when its owner has already left.
#[test]
fn taking_an_application_from_an_empty_queue_is_nothing() {
    let mut tui = Tui::new(None);
    assert!(tui.held.is_empty());
    consumed(&mut tui);
    assert!(tui.held.is_empty());
}

/// Flushing a queue pressed on another view drops it rather than moving
/// the view that replaced it (M4), and cancels any end it still owns.
#[test]
fn flushing_another_view_s_queue_drops_it() {
    use super::state::HeldOrigin;
    let (views, mut tui) = findings_column();
    tui.live_offset = 5;
    tui.live_line = 1;
    tui.held_for = Some(held_view_of(&tui, &views));
    tui.held.push(Held {
        step: Step::Down,
        count: 1,
        end: Some(HeldOrigin::Live { offset: 0, line: 0 }),
    });
    tui.want = Some(0);
    tui.level = Level::Run;
    flush_held(&mut tui, &views);
    assert!(tui.held.is_empty(), "the queue is dropped");
    assert_eq!(tui.want, None, "its page request is cancelled");
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "its movement is not applied and its end is cancelled"
    );
}

/// A held column's step is the same arithmetic over a held window: the
/// page step keeps the ten rows it names and stops at the window's end.
#[test]
fn a_held_page_step_keeps_its_distance() {
    let mut offset = 0;
    scroll_paged(&mut offset, 0, 25, Step::PageDown, true, None, Paging::Held);
    assert_eq!(offset, PAGE, "one page from the top");
    offset = 15;
    scroll_paged(&mut offset, 0, 25, Step::PageDown, true, None, Paging::Held);
    assert_eq!(offset, 25, "one past the window asks the next");
}

/// A held end step asks the rank it needs: the top asks zero and the end
/// asks the absolute marker, never a row the window happens to hold.
#[test]
fn a_held_end_step_asks_its_rank() {
    let tui = Tui::new(None);
    assert_eq!(ask_want(&tui, Step::Top), 0);
    assert_eq!(ask_want(&tui, Step::Bottom), usize::MAX);
}

/// A participant pane on the transcript is always inside its window: the
/// held-movement window is the checkpoint pane's alone.
#[test]
fn a_transcript_pane_holds_its_cursor() {
    let views = fleet_views();
    let mut tui = Tui::new(None);
    tui.level = Level::Participant;
    tui.pane = 1;
    assert!(in_window(&tui, &views));
}

/// A run level reading the findings holds its cursor: the column is not
/// a stream, so every drawn line is inside the window it has.
#[test]
fn a_findings_column_holds_its_cursor() {
    let (views, tui) = findings_column();
    assert!(in_window(&tui, &views));
}

/// A queue pressed on the view the answer belongs to is applied: its head
/// leaves the queue after its one application.
#[test]
fn a_queue_on_its_own_view_is_applied() {
    let (views, mut tui) = findings_column();
    tui.held_for = Some(held_view_of(&tui, &views));
    tui.held.push(Held {
        step: Step::Down,
        count: 1,
        end: None,
    });
    flush_held(&mut tui, &views);
    assert!(tui.held.is_empty(), "the head's application consumed it");
}

/// A typing operator on a paged live column binds neither `g` nor `G` to
/// movement: the keys are the filter's while the prompt is open.
#[test]
fn a_typing_paged_column_binds_no_end_key() {
    let (views, mut tui) = live_column(REVIEWING, reviewing_view());
    tui.typing = true;
    assert!(page_step(&tui, &views, Key::Char('g')).is_none());
    assert!(page_step(&tui, &views, Key::Char('G')).is_none());
}
