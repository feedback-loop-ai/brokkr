//! The movement held for a page read is bound to the view it was pressed
//! on (#753 fix round 753c6): preserved across a refresh, settled at the
//! absolute top after a resize, normalized against the row a resize
//! shortened, and dropped rather than applied to the fleet when the pane
//! it belongs to leaves the frame. Split from `console_end.rs` to keep
//! that file within its ceiling (decision 0071 ruling 4).

use super::console_end::{
    console_of, live_at, participant_page, retired_panel, settle_held_live,
    wrapped_first_turn_console,
};
use super::far_work::far_work_of;
use super::*;

/// The wrapped panel's console at (100, 25): twenty-five real `Down`s
/// into the row at 375x250, the continuation a widening removes.
fn wrapped_at_25(db: &Path, run: &str, now: &str) -> (Tui, Views) {
    let (mut tui, views) = live_at(db, run, 100, 375, 250, now);
    twenty_five_downs(&mut tui, &views);
    (tui, views)
}

/// The whole read at the same (100, 25), for the controls beside it. The
/// movement is proven on the paged read's walk; this control places the
/// cursor, so the proof does not lay the whole panel out twice.
fn whole_at_25(run: &str) -> Tui {
    let mut tui = live_tui(run, 100, 375, 250);
    (tui.live_offset, tui.live_line) = (100, 25);
    tui
}

/// The movement the resize proofs start from: twenty-five `Down`s from
/// the row's start land on (100, 25), one drawn line per press.
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
    let (_dir, db, _events, whole_views) = wrapped_first_turn_console(run, 51);
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
    let mut whole_tui = whole_at_25(run);
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
    assert_eq!(
        held_down_hidden_by("chief hidden and back", (80, 30)),
        (0, 0),
        "the answer moves nothing the hidden pane held"
    );
}

/// The panel's live column at (0, 0) at 375x30 holds one `Down` while its
/// read is out; the frame is resized to `hide`, which draws no live
/// column, and settles unanswered; restored to 375x30, the read answers.
/// Where the answer leaves the cursor.
fn held_down_hidden_by(run: &str, hide: (u16, u16)) -> (usize, usize) {
    use crate::tui::down_key;
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 50, 50, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    down_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 1, "the live column's Down is held");
    resize_and_settle(&mut tui, &views, hide);
    restore_and_answer(&db, run, &mut tui, &whole_views.now, (375, 30))
}

/// Narrow the frame so the live column leaves it, and settle the read
/// still out: the hidden pane's queue is dropped, never revived.
fn hide_and_settle(tui: &mut Tui, views: &Views) {
    let height = tui.height;
    resize_and_settle(tui, views, (80, height));
}

/// Resize the frame to `size`, which no longer draws the live column, and
/// settle the read still out: the hidden pane's queue is dropped.
fn resize_and_settle(tui: &mut Tui, views: &Views, (width, height): (u16, u16)) {
    use crate::tui::settle_when;
    (tui.width, tui.height) = (width, height);
    settle_when(tui, views, false);
    assert!(tui.held.is_empty(), "the hidden pane's queue is dropped");
}

/// Widen back to 375, let the read answer, and return where the answer
/// left the live column: the frame a hidden pane's cancelled queue must
/// not move.
fn widen_and_answer(db: &Path, run: &str, tui: &mut Tui, now: &str) -> (usize, usize) {
    let height = tui.height;
    restore_and_answer(db, run, tui, now, (375, height))
}

/// Restore the frame to `size`, let the read answer, and return where the
/// answer left the live column.
fn restore_and_answer(
    db: &Path,
    run: &str,
    tui: &mut Tui,
    now: &str,
    (width, height): (u16, u16),
) -> (usize, usize) {
    use crate::tui::settle_when;
    (tui.width, tui.height) = (width, height);
    let answer = live_read(db, run, tui, now);
    settle_when(tui, &answer, true);
    (tui.live_offset, tui.live_line)
}

/// 753cL3's C1 (decision 0071 rulings 5 and 9): a frame shorter than
/// `MIN_HEIGHT` draws no pane at all, only "this terminal is too small",
/// so the live column has left it as surely as a narrowed frame's. The
/// panel's live column at (0, 0) at 375x30 holds a `Down`; the frame
/// shrinks to `MIN_HEIGHT - 1` while the read is still out, and grows back
/// to 30 before the answer lands. The cursor stays at (0, 0) with nothing
/// held, as the width case above does.
#[test]
fn chief_landing_too_short_then_shown_drops_held_movement() {
    use crate::tui::MIN_HEIGHT;
    assert_eq!(
        held_down_hidden_by("chief too short and back", (375, MIN_HEIGHT - 1)),
        (0, 0),
        "the answer moves nothing the too-short frame held"
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
    let (_dir, db, _events, whole_views) = wrapped_first_turn_console(run, 51);
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
    let mut whole_tui = whole_at_25(run);
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
    let (_dir, db, _events, whole_views) = wrapped_first_turn_console(run, 51);
    let (mut tui, views) = wrapped_at_25(&db, run, &whole_views.now);
    tui.width = 750;
    up_key(&mut tui, &views);
    let _ = settle_three(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (100, 13),
        "the widened window finishes Up as the whole read does"
    );
    let mut whole_tui = whole_at_25(run);
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

/// The participant level's first page of a 1,000-checkpoint run with a
/// page beyond it: the run, the journal, its events, the seat and the
/// page, the start the participant held-end cases share.
struct FarPage {
    run: &'static str,
    db: std::path::PathBuf,
    events: Vec<EventEnvelope>,
    subject: Subject,
    views: Views,
}

fn far_page() -> FarPage {
    let far = far_work_of(1_000);
    let run = far.run;
    let db = far.db.clone();
    let events = far.events.clone();
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (_, subject) = participant_console(run, &whole);
    let views = console_rows_at(&db, run, Some(subject.clone()), false, 200, 0);
    assert_eq!(
        (views.start, views.more, views.end),
        (0, true, None),
        "the first page has an end beyond it"
    );
    FarPage {
        run,
        db,
        events,
        subject,
        views,
    }
}

/// The held-key sequences both run screens' end controls try: one `Up`,
/// two `Up`s in one coalesced run, and `Up` then `Down`.
const HELD_UP_CASES: [(&str, usize, bool); 3] = [
    ("one Up", 1, false),
    ("two Ups", 2, false),
    ("Up then Down", 1, true),
];

/// Press `ups` `Up`s and then a `Down` when asked, on a read that is
/// out: the keys both run screens' end controls hold.
fn press_ups(tui: &mut Tui, views: &Views, ups: usize, down: bool) {
    use crate::tui::{down_key, up_key};
    for _ in 0..ups {
        up_key(tui, views);
    }
    if down {
        down_key(tui, views);
    }
}

/// The whole read's own keys, each settled before the next as the
/// console's drive settles them between frames.
fn whole_press(tui: &mut Tui, views: &Views, ups: usize, down: bool) {
    use crate::tui::{down_key, settle_end, up_key};
    for _ in 0..ups {
        up_key(tui, views);
        settle_end(tui, views);
    }
    if down {
        down_key(tui, views);
        settle_end(tui, views);
    }
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
    let FarPage {
        run,
        db,
        events,
        subject,
        views,
        ..
    } = far_page();
    let mut tui = participant_tui(run);
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
    let whole_views = participant_whole_views(&db, run, &events, subject);
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

/// The review's M1 (rulings 5 and 9): an answer that does not name the
/// end neither applies a held `G` again nor forgets the cursor its
/// application replaced. A second ordinary answer arrives before the
/// end's; the marker still waits, and hiding the column returns it to
/// (0, 0), not to the marker it replaced.
#[test]
fn chief_landing_ordinary_answer_keeps_held_end_origin() {
    use crate::tui::{settle_when, Step};
    let run = "chief held end origin";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    hold_end_for_answer(&mut tui, &views);
    settle_when(&mut tui, &views, true);
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Bottom),
        "the ordinary answer leaves G waiting for the end"
    );
    assert_eq!(tui.live_offset, usize::MAX, "the marker still asks the end");
    hide_and_settle(&mut tui, &views);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the end's origin survived the ordinary answer"
    );
}

/// Hold `G` on a read that is out and settle its ordinary answer: the
/// head still owns the absolute-end request and the cursor holds the
/// unresolved marker, the start the cancelled-end controls share.
fn hold_end_for_answer(tui: &mut Tui, views: &Views) {
    use crate::tui::{bottom_key, settle_when, Step};
    tui.pending = true;
    bottom_key(tui, views);
    settle_when(tui, views, true);
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Bottom),
        "G owns the absolute-end request"
    );
    assert_eq!(tui.live_offset, usize::MAX, "G asked the absolute end");
}

/// The review's M1 (rulings 5 and 9): a held `G` that asked the absolute
/// end is cancelled when the pane it was pressed on leaves the frame,
/// never left to resolve later. The panel's live column at (0, 0) holds
/// `G` while its read is out; the ordinary answer leaves the end request
/// at the head, the frame narrows to 80 so the column disappears, and the
/// queue drops unanswered. Widened back, the answer moves nothing: (0, 0),
/// as the whole read with no key does.
#[test]
fn chief_landing_hidden_pane_drops_held_end() {
    let run = "chief hidden held end";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    hold_end_for_answer(&mut tui, &views);
    hide_and_settle(&mut tui, &views);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the dropped end puts the cursor back where it was"
    );
    assert_eq!(
        widen_and_answer(&db, run, &mut tui, &whole_views.now),
        (0, 0),
        "the answer moves nothing the dropped pane held"
    );
}

/// The review's M1 (rulings 5 and 9): the absolute-end request belongs to
/// the run that asked it. The live column's held `G` is evicted by the
/// 64-run bound before the end answers; its cursor change is cancelled,
/// so the retained runs are applied against the window their own answer
/// names and never consumed in its place. The 64 keys from (0, 0) stay
/// there, as the whole read's do.
#[test]
fn chief_landing_evicted_end_cancels_with_its_run() {
    use crate::tui::{down_key, up_key, Step, HELD_BOUND};
    let run = "chief evicted held end";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    hold_end_for_answer(&mut tui, &views);
    for _ in 0..32 {
        down_key(&mut tui, &views);
        up_key(&mut tui, &views);
    }
    assert_eq!(tui.held.len(), HELD_BOUND, "the bound keeps 64 runs");
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Down),
        "G left with the bound"
    );
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the evicted end puts the cursor back where it was"
    );
    settle_held_live(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the retained 64 keys land as the whole read's from the top"
    );
    let mut whole_tui = live_tui(run, 0, 375, 30);
    for _ in 0..32 {
        down_key(&mut whole_tui, &whole_views);
        up_key(&mut whole_tui, &whole_views);
    }
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (0, 0),
        "the whole read's 64 keys from the top"
    );
}

/// A participant-level console at the participant pane's top, ready to
/// take the same keys the whole read's control takes.
fn participant_tui(run: &str) -> Tui {
    let mut tui = Tui::new(Some(run.to_string()));
    crate::tui::participant_level(&mut tui, "eff");
    (tui.width, tui.height) = (80, 30);
    tui
}

/// The reads the shell would make for the participant level's held keys:
/// the page each waits for, answered and settled, until none is left. A
/// held key that never lands fails here, by name, rather than looping.
fn settle_participant_held(db: &Path, run: &str, subject: &Subject, tui: &mut Tui) -> Views {
    use crate::tui::settle_when;
    let mut views = participant_page(db, run, subject, tui);
    for _ in 0..1_000 {
        settle_when(tui, &views, true);
        if tui.held.is_empty() {
            return views;
        }
        views = participant_page(db, run, subject, tui);
    }
    panic!("held keys never landed: {} still held", tui.held.len())
}

/// The review's M1 (rulings 5 and 9): a held `Up` at the participant
/// level's top asks the absolute end once. The first answer does not name
/// the end, so the key waits; the end's answer resolves it, and it is not
/// applied a second time. On the first page of 1,000 checkpoints, one held
/// `Up` lands on 999, two on 998, and `Up` then `Down` on 0, as the whole
/// read does.
#[test]
fn chief_review_held_up_resolves_once_at_the_participant_level() {
    let FarPage {
        run,
        db,
        events,
        subject,
        views,
        ..
    } = far_page();
    let whole_views = participant_whole_views(&db, run, &events, subject.clone());
    for (what, ups, down) in HELD_UP_CASES {
        let mut tui = participant_tui(run);
        tui.pending = true;
        press_ups(&mut tui, &views, ups, down);
        assert!(!tui.held.is_empty(), "{what}: the keys wait for the read");
        settle_participant_held(&db, run, &subject, &mut tui);
        let mut whole_tui = participant_tui(run);
        whole_press(&mut whole_tui, &whole_views, ups, down);
        assert_eq!(
            tui.offset, whole_tui.offset,
            "{what}: the held keys land where the whole read does"
        );
    }
}

/// The review's M1 on the live column (rulings 5 and 9): the same held
/// `Up` at the column's top is resolved once. On the two-member panel's
/// 375x30 live column at (0, 0), one held `Up` lands on (999, 0), two on
/// (998, 0), and `Up` then `Down` wraps to (0, 0), as the whole read does.
#[test]
fn chief_review_held_up_resolves_once_on_the_live_column() {
    let run = "chief review held up live";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 500, 500, None));
    for (what, ups, down) in HELD_UP_CASES {
        let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
        tui.pending = true;
        press_ups(&mut tui, &views, ups, down);
        assert!(!tui.held.is_empty(), "{what}: the keys wait for the read");
        settle_held_live(&db, run, &mut tui, &whole_views.now);
        let mut whole_tui = live_tui(run, 0, 375, 30);
        whole_press(&mut whole_tui, &whole_views, ups, down);
        assert_eq!(
            (tui.live_offset, tui.live_line),
            (whole_tui.live_offset, whole_tui.live_line),
            "{what}: the held keys land where the whole read does"
        );
    }
}

/// The review's M1 at the participant level (rulings 5 and 9): a held
/// `Up` that asked the absolute end belongs to the checkpoint pane. The
/// ordinary answer leaves the marker at its head; Tab moves to the
/// transcript, cancelling it back to the rank it replaced, and Tab back
/// answers at the top the whole read would show.
#[test]
fn chief_review_participant_held_end_cancels_with_its_pane() {
    use crate::tui::{settle_when, tab_key, up_key, Step};
    let FarPage {
        run,
        db,
        subject,
        views,
        ..
    } = far_page();
    let mut tui = participant_tui(run);
    tui.pending = true;
    up_key(&mut tui, &views);
    settle_when(&mut tui, &views, true);
    assert_eq!(
        tui.held.first().map(|held| held.step),
        Some(Step::Up),
        "Up owns the absolute-end request"
    );
    assert_eq!(tui.offset, usize::MAX, "Up asked the absolute end");
    tab_key(&mut tui, &views);
    assert!(tui.held.is_empty(), "the transcript pane drops the queue");
    assert_eq!(tui.offset, 0, "the cancelled end restores the rank");
    tab_key(&mut tui, &views);
    let answer = participant_page(&db, run, &subject, &tui);
    settle_when(&mut tui, &answer, true);
    assert_eq!(tui.offset, 0, "the answer resolves nothing");
}

/// M1 (rulings 5 and 9): a resolved end consumes the held head that asked
/// for it, whichever answer resolves it. A short stream's live column at
/// (0, 0), 375x30, holds `Up` then `Down` while its read is out; the
/// first answer's whole read names the end the held `Up` asked for, so
/// that answer already consumes `Up` and leaves `Down` to land from the
/// end. Three answers leave (0, 0), as the whole read's
/// Up/settle/Down/settle does, never (19, 0) with the queue still waiting
/// for an end it has reached.
#[test]
fn a_short_stream_up_then_down_while_a_read_is_pending_matches_the_whole_read() {
    use crate::tui::{down_key, settle_when, up_key};
    let run = "short stream up then down";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 10, 10, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    assert_eq!(
        (views.start, views.more, views.end),
        (0, false, None),
        "the short stream's first window has no page beyond it"
    );
    tui.pending = true;
    up_key(&mut tui, &views);
    down_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 2, "Up and Down wait for the answer");
    for _ in 0..3 {
        let answer = live_read(&db, run, &tui, &whole_views.now);
        settle_when(&mut tui, &answer, true);
    }
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the held Up then Down matches the whole read"
    );
    assert!(tui.held.is_empty(), "both held keys have landed");

    let mut whole_tui = live_tui(run, 0, 375, 30);
    up_key(&mut whole_tui, &whole_views);
    crate::tui::settle_end(&mut whole_tui, &whole_views);
    down_key(&mut whole_tui, &whole_views);
    crate::tui::settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (tui.live_offset, tui.live_line),
        "the whole read's Up/settle/Down/settle"
    );
}

/// M1 (rulings 5 and 9): four held keys on the same short stream keep
/// their order across the answers. Up, Down, Up, Down while the read is
/// out land where the whole read's four keys settle, (0, 0), never
/// leaving the second `Up`'s end marker behind the queue.
#[test]
fn a_short_stream_four_held_keys_end_where_the_whole_read_ends() {
    use crate::tui::{down_key, settle_end, up_key};
    let run = "short stream four held";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 10, 10, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    for _ in 0..2 {
        up_key(&mut tui, &views);
        down_key(&mut tui, &views);
    }
    assert_eq!(tui.held.len(), 4, "four held runs wait in order");
    settle_held_live(&db, run, &mut tui, &whole_views.now);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (0, 0),
        "the held four keys land where the whole read does"
    );
    let mut whole_tui = live_tui(run, 0, 375, 30);
    for _ in 0..2 {
        up_key(&mut whole_tui, &whole_views);
        settle_end(&mut whole_tui, &whole_views);
        down_key(&mut whole_tui, &whole_views);
        settle_end(&mut whole_tui, &whole_views);
    }
    assert_eq!(
        (whole_tui.live_offset, whole_tui.live_line),
        (tui.live_offset, tui.live_line),
        "the whole read's Up/Down/Up/Down"
    );
}

/// M1 (rulings 5 and 9): a held `G`, then `Down`, on the same short
/// stream lands at the end and then moves down from it once, as the
/// whole read does. The end's owner is consumed before the `Down` behind
/// it is applied.
#[test]
fn a_short_stream_held_end_then_down_matches_the_whole_read() {
    use crate::tui::{bottom_key, down_key, settle_end};
    let run = "short stream held end then down";
    let (_dir, db, _events, whole_views) = console_of(run, panel_with(run, 10, 10, None));
    let (mut tui, views) = live_at(&db, run, 0, 375, 30, &whole_views.now);
    tui.pending = true;
    bottom_key(&mut tui, &views);
    down_key(&mut tui, &views);
    assert_eq!(tui.held.len(), 2, "G and Down wait for the answer");
    settle_held_live(&db, run, &mut tui, &whole_views.now);
    let mut whole_tui = live_tui(run, 0, 375, 30);
    bottom_key(&mut whole_tui, &whole_views);
    settle_end(&mut whole_tui, &whole_views);
    down_key(&mut whole_tui, &whole_views);
    settle_end(&mut whole_tui, &whole_views);
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (whole_tui.live_offset, whole_tui.live_line),
        "the held G then Down lands where the whole read does"
    );
}
