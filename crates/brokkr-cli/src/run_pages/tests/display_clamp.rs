//! The participant display's narrowed page (#753 fix round 753c6): a
//! logical rank past the `u16` display clamp still keeps the stream's own
//! coordinate across four page steps and a reread, and the frame the
//! reread paints is the whole read's. Split from `console_end.rs` to keep
//! that file within its ceiling (decision 0071 ruling 4).

use super::console_end::{far_work_of, first_turn, participant_page};
use super::*;

/// L1 (rulings 5 and 9): from logical rank 65,530 of 66,000 checkpoints
/// at 80x30, four `PageDown`s reach 65,570 while the reread asks today's
/// display rank 65,535; the end is still unknown, a page follows, the
/// first painted turn is 65,536, and the frame is the whole read's at
/// 65,570.
#[test]
fn chief_review_display_clamp_keeps_frame_after_page_down() {
    use crate::tui::{page_key, settle_end};
    let (_dir, db, events) = far_work_of("chief display clamp", 66_000);
    let run = "chief display clamp";
    let now = events.last().unwrap().recorded_at.clone();
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    tui.offset = 65_530;
    let views = participant_page(&db, run, &subject, &tui);
    assert_eq!(views.start, 65_530, "the rank's own page");
    for _ in 0..4 {
        page_key(&mut tui, &views, true);
    }
    assert_eq!(tui.offset, 65_570, "four page steps keep the logical rank");
    let mut views = participant_page(&db, run, &subject, &tui);
    views.now = now;
    settle_end(&mut tui, &views);
    assert_eq!(tui.offset, 65_570, "the reread keeps the logical rank");
    assert_eq!(
        (views.start, views.more, views.end),
        (65_535, true, None),
        "the reread asks today's display rank, the end still unknown"
    );
    assert_eq!(
        first_turn(&views, "eff").as_deref(),
        Some("65536"),
        "the display starts at today's clamped rank"
    );
    let _ = participant_frame_whole_read(&db, run, &events, subject, 65_570, &tui, &views);
}
