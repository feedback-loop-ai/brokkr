//! The movement held for a page read is bound to the view it was pressed
//! on (#753 fix round 753c6): preserved across a refresh, settled at the
//! absolute top after a resize, normalized against the row a resize
//! shortened, and dropped rather than applied to the fleet when the pane
//! it belongs to leaves the frame. Split from `console_end.rs` to keep
//! that file within its ceiling (decision 0071 ruling 4).

use super::console_end::{
    console_of, far_work_of, live_at, retired_panel, settle_held_live, wrapped_panel_console,
};
use super::*;

/// The wrapped panel's console at (100, 25): twenty-five drawn lines into
/// the row at 375x250, the continuation a widening removes.
fn wrapped_at_25(db: &Path, run: &str, now: &str) -> (Tui, Views) {
    let (mut tui, views) = live_at(db, run, 100, 375, 250, now);
    twenty_five_downs(&mut tui, &views);
    (tui, views)
}

/// The whole read at the same (100, 25), for the controls beside it.
fn whole_at_25(run: &str, whole: &Views) -> Tui {
    let mut tui = live_tui(run, 100, 375, 250);
    twenty_five_downs(&mut tui, whole);
    tui
}

fn twenty_five_downs(tui: &mut Tui, views: &Views) {
    use crate::tui::down_key;
    for _ in 0..25 {
        down_key(tui, views);
    }
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (100, 25),
        "twenty-five drawn lines into the row"
    );
}

/// Read the page `tui` waits for and settle it until its queue is empty,
/// at most three reads, so an unnormalized continuation fails instead of
/// hanging.
fn settle_three(db: &Path, run: &str, tui: &mut Tui, now: &str) -> Views {
    use crate::tui::settle_when;
    let mut answer = live_read(db, run, tui, now);
    for _ in 0..3 {
        settle_when(tui, &answer, true);
        if tui.held.is_empty() {
            break;
        }
        answer = live_read(db, run, tui, now);
    }
    answer
}

/// M1 (rulings 5 and 9): a refresh, help or an unbound character does not
/// drop the movement held for a read still out. At ranks 0 and 49 of the
/// 1,000-checkpoint panel at 375x30, a `PageDown` is held, `?`, `z` and
/// `r` are pressed, and the answer lands the step at 10 and 59 as the
/// whole read does, never leaving the cursors at 0 and 49.
#[test]
fn chief_753c6_refresh_preserves_held_movement() {
    use crate::tui::{char_key, page_key};
    let run = "chief refresh held";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    for (rank, want) in [(0, 10), (49, 59)] {
        let (mut tui, views) = live_at(&db, run, rank, 375, 30, &whole_views.now);
        tui.pending = true;
        page_key(&mut tui, &views, true);
        assert_eq!(tui.held.len(), 1, "{rank}: the PageDown waits");
        char_key(&mut tui, &views, '?');
        assert_eq!(tui.held.len(), 1, "{rank}: help keeps the held key");
        char_key(&mut tui, &views, 'z');
        assert_eq!(
            tui.held.len(),
            1,
            "{rank}: an unbound character keeps the held key"
        );
        char_key(&mut tui, &views, 'r');
        assert_eq!(tui.held.len(), 1, "{rank}: the refresh keeps the held key");
        settle_held_live(&db, run, &mut tui, &whole_views.now);
        assert_eq!(
            (tui.live_offset, tui.live_line),
            (want, 0),
            "{rank}: the held PageDown lands"
        );
        let mut whole_tui = live_tui(run, rank, 375, 30);
        page_key(&mut whole_tui, &whole_views, true);
        assert_eq!(
            (whole_tui.live_offset, whole_tui.live_line),
            (want, 0),
            "{rank}: the whole read's PageDown"
        );
    }
}

/// M3 (rulings 5 and 9): a widened frame removes the saved continuation
/// the old width had, and the top does not need it. At (100, 25) of the
/// wrapped panel, widening to 750 then pressing `g` reaches (0, 0) as the
/// whole read does, never staying at (100, 25) with `g` held.
#[test]
fn chief_753c6_resize_top_settles() {
    use crate::tui::{settle_when, top_key};
    let run = "chief resize top";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = wrapped_at_25(&db, run, &whole_views.now);
    tui.width = 750;
    top_key(&mut tui, &views);
    assert!(tui.held.is_empty(), "the top needs no continuation");
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the top is the stream's, whatever the width"
    );
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &next, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the answer leaves the top there"
    );
    let mut whole_tui = whole_at_25(run, &whole_views);
    whole_tui.width = 750;
    top_key(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (0, 0),
        "the whole read's top after the same resize"
    );
}

/// M4 (rulings 5 and 9): a held movement pressed on the live column is
/// not applied to the fleet when a narrowed frame removes that column.
/// The panel's live column at (0, 0) holds an `Up` (the fleet lists the
/// panel last, so `Up` is the step that would leave it), the frame
/// narrows to 80 so the column disappears, and the answer leaves the
/// fleet's selection on `chief resize hide`, never moving it to `second`.
#[test]
fn chief_753c6_hidden_pane_does_not_move_fleet() {
    use crate::tui::{settle_when, up_key};
    let run = "chief resize hide";
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    planted(
        &db,
        vec![
            ("second".to_string(), panel_with("second", 10, 10, None)),
            (run.to_string(), panel_with(run, 500, 500, None)),
        ],
    );
    let (_, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    up_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "the live column's Up is held");
    tui.width = 80;
    let next = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &next, true);
    assert_eq!(
        tui.cursor[0].as_deref(),
        Some(run),
        "the fleet keeps the run it selected"
    );
    assert!(tui.held.is_empty(), "the queue is dropped, not applied");
    let mut whole_tui = live_tui(run, 0, 375, 30);
    up_key(&mut whole_tui, &whole_views);
    whole_tui.width = 80;
    assert_eq!(
        whole_tui.cursor[0].as_deref(),
        Some(run),
        "the whole read keeps the same selection after the resize"
    );
}

/// The landing's M1 (rulings 5 and 9): a pane that leaves the frame while
/// its read is out takes its held movement with it, and coming back
/// before the answer does not revive it. The panel's live column at
/// (0, 0) at 375x30 holds a `Down`; the frame narrows to 80, hiding the
/// column, while the read is still out, and widens back to 375 before the
/// answer lands. The cursor stays at (0, 0) with nothing held.
#[test]
fn chief_landing_hidden_then_shown_drops_held_movement() {
    use crate::tui::{down_key, settle_when};
    let run = "chief hidden and back";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 50, 50, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    down_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "the live column's Down is held");
    tui.width = 80;
    settle_when(&mut tui, &views, false);
    assert!(tui.held.is_empty(), "the hidden pane's queue is dropped");
    tui.width = 375;
    let answer = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &answer, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the answer moves nothing the hidden pane held"
    );
}

/// M1 (rulings 5 and 9): a resize that removes the saved continuation
/// does not strand ordinary movement. At (100, 25) of the wrapped panel,
/// widening to 750 and pressing `Down` lands on (101, 0) as the whole
/// read does, and a second `Down` follows to (101, 1), whether the window
/// is read at once or the second key waits behind the first's answer.
#[test]
fn chief_gate_resize_down_must_finish() {
    use crate::tui::down_key;
    let run = "chief gate resize down";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = wrapped_at_25(&db, run, &whole_views.now);
    tui.width = 750;
    down_key(&mut tui, &views);
    let answer = settle_three(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (101, 0),
        "the widened window finishes Down as the whole read does"
    );
    down_key(&mut tui, &answer);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (101, 1),
        "the next movement follows the landed one"
    );
    let mut whole_tui = whole_at_25(run, &whole_views);
    whole_tui.width = 750;
    down_key(&mut whole_tui, &whole_views);
    down_key(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (101, 1),
        "the whole read's two Downs after the resize"
    );

    // The keys behind a held move keep their order: two Downs pressed
    // while a read is out land one after the other, as the whole read's.
    let (mut queued, views) = wrapped_at_25(&db, run, &whole_views.now);
    queued.width = 750;
    queued.pending = true;
    down_key(&mut queued, &views);
    down_key(&mut queued, &views);
    assert_eq!(queued.held[0].count, 2, "the two Downs are one held run");
    let _ = settle_three(&db, run, &mut queued, &whole_views.now);
    assert_eq!(
        (queued.live_offset, queued.live_line),
        (101, 1),
        "the queued second Down lands after the first"
    );
}

/// M1 (rulings 5 and 9): the same resize with `Up`: at (100, 25), widening
/// to 750 and pressing `Up` lands on the row's last drawn line, (100, 13),
/// as the whole read does, never holding forever at (100, 25).
#[test]
fn chief_gate_resize_up_must_finish() {
    use crate::tui::up_key;
    let run = "chief gate resize up";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = wrapped_at_25(&db, run, &whole_views.now);
    tui.width = 750;
    up_key(&mut tui, &views);
    let _ = settle_three(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (100, 13),
        "the widened window finishes Up as the whole read does"
    );
    let mut whole_tui = whole_at_25(run, &whole_views);
    whole_tui.width = 750;
    up_key(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (100, 13),
        "the whole read's Up after the resize"
    );
}

/// 753c6's last review, M1 (decision 0089 rulings 5 and 8): on an
/// unindexed journal the console falls back to the whole read, and that
/// read ends the live column at the hired seat's last row, as the paged
/// read does. `eff:a`'s pinned 300 turns extend nothing, so Up from the
/// top with the read out wraps to (29, 0), as the whole read does, never
/// (330, 0).
#[test]
fn chief_review_fallback_end_ignores_retired_seat() {
    use crate::tui::{settle_when, up_key};
    let run = "chief fallback end";
    let (_dir, db, _events, whole_views) = console_of(run, retired_panel(run));
    unindex(&rusqlite::Connection::open(&db).unwrap()).unwrap();
    let store = Store::open_read_only(&db).unwrap();
    let (shown, _) = super::super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert!(
        matches!(shown.read, Held::Whole(..)),
        "an unindexed journal is read whole"
    );
    assert_eq!(
        shown.end(&Console::Detail),
        Some(30),
        "the hired seat's 30 rows end the column"
    );
    drop(store);
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    up_key(&mut tui, &views);
    let answer = live_read(&db, run, &tui, &whole_views.now);
    settle_when(&mut tui, &answer, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (29, 0),
        "Up from the top wraps at the hired seat's end"
    );
    let mut whole_tui = live_tui(run, 0, 375, 30);
    up_key(&mut whole_tui, &whole_views);
    crate::tui::settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (29, 0),
        "the whole read's Up"
    );
}

/// Hold `G` on the read that is out, settle its first answer, and press
/// `Up` on that answer's tick before the absolute end answers: the held
/// end and the movement behind it.
fn hold_end_then_up(tui: &mut Tui, views: &Views) {
    use crate::tui::{bottom_key, settle_when, up_key, Step};
    tui.pending = true;
    bottom_key(tui, views);
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Bottom),
        "G waits behind the read"
    );
    settle_when(tui, views, true);
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Bottom),
        "G still owns the movement until the absolute end answers"
    );
    up_key(tui, views);
    assert_eq!(tui.held.len(), 2, "Up waits behind the unresolved end");
}

/// M2 (rulings 5 and 9): a held `G` stays at the head of the queue until
/// the answer names the stream's absolute end, so a movement pressed on
/// that answer's tick waits behind it. The participant level starts on
/// the first 200 of 1,000 checkpoints; `G` is held while the read is out,
/// its first answer lands, and `Up` is pressed before the end answers.
/// The end's window settles both: 998, as the whole read's `G` then `Up`.
#[test]
fn chief_landing_held_end_owns_the_movement_after_its_first_answer() {
    use crate::tui::{asked_rows, bottom_key, up_key};
    let run = "chief landing held end participant";
    let (_dir, db, events) = far_work_of(run, 1_000);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let views = console_rows_at(&db, run, Some(subject.clone()), false, 200, 0);
    assert_eq!(
        (views.start, views.more, views.end),
        (0, true, None),
        "the first page has an end beyond it"
    );
    hold_end_then_up(&mut tui, &views);
    assert_eq!(tui.offset, usize::MAX, "G asks the absolute end");
    let end = console_rows_at(
        &db,
        run,
        Some(subject.clone()),
        false,
        asked_rows(&tui),
        crate::tui::ask_offset(&tui),
    );
    assert_eq!(end.end, Some(1_000), "the end's window names the stream");
    crate::tui::settle_when(&mut tui, &end, true);
    assert_eq!(tui.offset, 998, "G then Up lands as the whole read does");
    let whole_views = participant_whole_views(&db, run, &events, whole, subject);
    let (mut whole_tui, _) = participant_console(run, whole_views.run.as_ref().unwrap());
    bottom_key(&mut whole_tui, &whole_views);
    up_key(&mut whole_tui, &whole_views);
    assert_eq!(whole_tui.offset, 998, "the whole read's G then Up");
}

/// M2 (rulings 5 and 9; decision 0089 ruling 9's bounded window): the
/// same sequence on the live column keeps the end request `usize::MAX`
/// while `Up` waits, so the read asks the viewport and not `MAX` plus a
/// viewport. The held sequence lands at (998, 0), and the end's window
/// holds one window of checkpoints, never the 1,000-row stream.
#[test]
fn chief_landing_held_end_keeps_the_live_window_bounded() {
    use crate::tui::{ask_offset, asked_rows, settle_when};
    let run = "chief landing held end live";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    hold_end_then_up(&mut tui, &views);
    assert_eq!(tui.live_offset, usize::MAX, "G asks the absolute end");
    assert_eq!((ask_offset(&tui), asked_rows(&tui)), (usize::MAX, 30));
    let end = live_read(&db, run, &tui, &whole_views.now);
    let retained: usize = end.run.as_ref().map_or(0, |view| {
        view.participants
            .iter()
            .map(|part| part.checkpoints.len())
            .sum()
    });
    assert_eq!(retained, 200, "the end read keeps one window");
    settle_when(&mut tui, &end, true);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (998, 0),
        "G then Up lands as the whole read does"
    );
}
