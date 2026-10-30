//! The movement held for a page read is bound to the view it was pressed
//! on (#753 fix round 753c6): preserved across a refresh, settled at the
//! absolute top after a resize, and dropped rather than applied to the
//! fleet when the pane it belongs to leaves the frame. Split from
//! `console_end.rs` to keep that file within its ceiling (decision 0071
//! ruling 4).

use super::console_end::{console_of, live_at, settle_held_live, wrapped_panel_console};
use super::*;

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
    use crate::tui::{down_key, settle_when, top_key};
    let run = "chief resize top";
    let (_dir, db, _events, whole_views) = wrapped_panel_console(run);
    let (mut tui, views) = live_at(&db, run, 100, 375, 250, &whole_views.now);
    for _ in 0..25 {
        down_key(&mut tui, &views);
    }
    assert_eq!(
        (tui.live_offset, tui.live_line),
        (100, 25),
        "twenty-five drawn lines into the row"
    );
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
    let mut whole_tui = live_tui(run, 100, 375, 250);
    for _ in 0..25 {
        down_key(&mut whole_tui, &whole_views);
    }
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
