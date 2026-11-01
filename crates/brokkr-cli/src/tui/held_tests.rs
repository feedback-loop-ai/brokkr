//! The shell settles held movement only on a successful read (#753 fix
//! round 753c6): a transient error keeps the frame and the queue, and the
//! next answer lands the key. A view that leaves the frame takes the
//! movement pressed on it, with its unresolved end, before the level
//! moves (#753 landings' M1). Split from `tests.rs` to keep that file
//! within its baseline (decision 0071 ruling 4).

use super::tests::{at_seats, journal, over_fleet, script, test_ops, TERMINAL};
use super::*;
use brokkr_core::{EventEnvelope, EventType};
use ratatui::backend::TestBackend;
use serde_json::json;

/// The participant level's views over `events` with `end` checkpoints
/// answered, the window from its top.
fn participant_views(events: &[EventEnvelope], end: usize) -> Views {
    let folded = brokkr_core::fold::fold(events);
    let mut views = over_fleet(Some(brokkr_view::run_view(events, folded.as_ref().ok())));
    views.start = 0;
    views.end = Some(end);
    views
}

/// The fixture participant level at rank 1 on the 80x30 frame both
/// overflow tests press their keys on.
fn overflow_tui() -> Tui {
    let mut tui = Tui::new(Some("run-7".to_string()));
    participant_level(&mut tui, "eff-i");
    (tui.width, tui.height) = (80, 30);
    tui.offset = 1;
    tui
}

/// The sixty-five alternating runs, starting and ending Up, that overflow
/// the 64-run bound by one: no two adjacent keys share a run.
fn overflow_keys() -> Vec<Key> {
    (0..=HELD_BOUND)
        .map(|index| match index % 2 {
            0 => Key::Up,
            _ => Key::Down,
        })
        .collect()
}

/// A source that leaves the reads in `pending` out and answers every other
/// call with `views` at its second checkpoint, so a test presses keys while
/// the read is out and sees the frame that answers them.
fn answering(
    events: &[EventEnvelope],
    pending: std::ops::RangeInclusive<usize>,
    views: fn(&[EventEnvelope], usize) -> Views,
) -> impl FnMut(Ask) -> Polled {
    let answers = events.to_vec();
    let mut call = 0usize;
    move |_ask: Ask| -> Polled {
        call += 1;
        if pending.contains(&call) {
            Polled::Pending
        } else {
            Ok(Some(views(&answers, 2))).into()
        }
    }
}

/// Drive `tui` against `source` on the 80x30 frame the overflow tests draw,
/// returning the terminal whose buffer holds the last frame.
fn overflow_drive(
    tui: &mut Tui,
    source: &mut impl FnMut(Ask) -> Polled,
    max: usize,
) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
    drive(&mut terminal, &test_ops(), source, tui, max).unwrap();
    terminal
}

/// Whether the overflow notice is on `terminal`'s last drawn frame.
fn overflow_notice_drawn(terminal: &Terminal<TestBackend>) -> bool {
    let drawn: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    drawn.contains("held movement keys dropped: the queue is full")
}

/// M2 (rulings 5 and 9): a failed read is not an answer. The participant
/// pane holds two checkpoints and a `Down` waits while a read is out; the
/// error arrives and then a three-checkpoint frame. The cursor reaches 2
/// as the whole read does, never wrapping to 0 against the old frame.
#[test]
fn chief_753c6_transient_error_preserves_held_down() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let old_events = journal("eff-i");
    let old = participant_views(&old_events, 2);
    assert_eq!(
        participant(&old, "eff-i").map(|part| part.checkpoints.len()),
        Some(2),
        "the old frame holds two checkpoints"
    );
    let mut third = old_events
        .iter()
        .find(|event| event.event_type == EventType::EffectCheckpointed)
        .cloned()
        .expect("the fixture holds a checkpoint");
    third.seq = 18;
    third.event_id = "ev18".to_string();
    third.payload = json!({"effect_id": "eff-i", "attempt_id": "att1",
                           "checkpoint": {"step": "seat-turn", "turn": 4}});
    let mut fresh_events = old_events.clone();
    fresh_events.push(third);
    let fresh = participant_views(&fresh_events, 3);
    assert_eq!(
        participant(&fresh, "eff-i").map(|part| part.checkpoints.len()),
        Some(3),
        "the fresh frame holds three checkpoints"
    );
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &old, Key::Enter);
    apply(&mut tui, &old, Key::Enter);
    assert_eq!(tui.level, Level::Participant, "the seat is open");
    tui.offset = 1;
    // A harmless character carries the shell past the first two answers;
    // `Down` is read only while the read is out.
    script(&[Key::Char('z'), Key::Down, Key::Char('z'), Key::Quit]);
    let mut call = 0usize;
    let mut source = move |_ask: Ask| -> Polled {
        let answer = match call {
            0 => Ok(Some(participant_views(&old_events, 2))).into(),
            1 => Polled::Pending,
            2 => Err(anyhow::anyhow!("database is locked")).into(),
            _ => Ok(Some(participant_views(&fresh_events, 3))).into(),
        };
        call += 1;
        answer
    };
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 6).unwrap();
    assert_eq!(
        tui.offset, 2,
        "the Down waits for a successful answer, then lands"
    );
}

/// The participant level's first page over `events`, the end still beyond
/// it: the window a held `G` asks its unresolved marker against.
fn first_page(events: &[EventEnvelope]) -> Views {
    let folded = brokkr_core::fold::fold(events);
    let mut views = over_fleet(Some(brokkr_view::run_view(events, folded.as_ref().ok())));
    views.start = 0;
    views.more = true;
    views.end = None;
    views
}

/// The console's participant level on `seat`, its cursor at `rank`.
fn participant_at(seat: &str, rank: usize) -> Tui {
    let mut tui = Tui::new(Some("run-7".to_string()));
    participant_level(&mut tui, seat);
    (tui.width, tui.height) = (80, 30);
    tui.offset = rank;
    tui
}

/// The landings' M1 (rulings 5 and 9): a `G` held on a seat that the
/// answered frame no longer carries is dropped with its view. Its
/// unresolved end is cancelled against the participant rank it replaced
/// before the level leaves that seat, so the `Esc` that follows walks to
/// the fleet's top, never restoring the old rank onto its dashboard. The
/// chief's rank is the first checkpoint's; the review's is a real
/// journal's hundredth.
#[test]
fn chief_and_review_vanished_seat_cancels_held_end_before_escape() {
    use crate::tui::{bottom_key, escape_key, settle_when};
    let old_events = journal("eff-i");
    let old = first_page(&old_events);
    let replacement = first_page(&old_events[..2]);
    assert!(
        replacement
            .run
            .as_ref()
            .is_some_and(|run| run.participants.is_empty()),
        "the replacement frame keeps the run and loses the seat"
    );
    for origin in [1, 100] {
        let mut tui = participant_at("eff-i", origin);
        tui.pending = true;
        bottom_key(&mut tui, &old);
        settle_when(&mut tui, &old, true);
        assert_eq!(tui.offset, usize::MAX, "{origin}: G asks the absolute end");
        assert!(
            tui.held.first().is_some_and(|held| held.end.is_some()),
            "{origin}: G owns the unresolved end"
        );
        settle_when(&mut tui, &replacement, true);
        assert_eq!(tui.level, Level::Run, "{origin}: the seat vanished");
        assert!(
            tui.held.is_empty(),
            "{origin}: the held end leaves with its seat"
        );
        assert_eq!(
            tui.offset, origin,
            "{origin}: cancelled against the view it was pressed on"
        );
        escape_key(&mut tui, &replacement);
        assert_eq!(
            (tui.level, tui.offset),
            (Level::Runs, 0),
            "{origin}: the dashboard is not scrolled by the cancelled end"
        );
    }
}

/// The landings' M1 in the run-vanish shape (rulings 5 and 9): the open
/// run leaves the fleet's frame while a held `G` still owns its marker.
/// The queue is dropped, and its end cancelled against the participant
/// rank, before the level moves, so the fleet draws at its top instead of
/// the stale marker's row on the next tick.
#[test]
fn chief_landing_vanished_run_cancels_held_end_before_the_fleet_draws() {
    use crate::tui::{bottom_key, settle_when};
    let old_events = journal("eff-i");
    let old = first_page(&old_events);
    let mut gone = over_fleet(None);
    gone.runs.runs.retain(|row| key_of(row) != "run-7");
    gone.runs.count = gone.runs.runs.len();
    let mut tui = participant_at("eff-i", 0);
    tui.pending = true;
    bottom_key(&mut tui, &old);
    settle_when(&mut tui, &old, true);
    assert_eq!(tui.offset, usize::MAX, "G asks the absolute end");
    settle_when(&mut tui, &gone, true);
    assert_eq!(tui.run, None, "the run vanished");
    assert_eq!(tui.level, Level::Runs, "the fleet is drawn");
    assert!(
        tui.held.is_empty(),
        "the held end leaves with the run that vanished"
    );
    assert_eq!(
        tui.offset, 0,
        "the fleet is not scrolled by the cancelled end"
    );
}

/// M2 (rulings 5 and 9): the queue's overflow notice survives the frame
/// that answers before the next draw. At the participant level, 65
/// alternating keys overflow the 64-run bound while the read is out; the
/// answering frame carries no note of its own, and the frame drawn after
/// it still says the oldest run was dropped. The retained 64 keys end at
/// 1, where the whole read's 65 end at 0: the dropped run is the oldest.
#[test]
fn chief_753c9_overflow_notice_survives_the_answering_frame() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let events = journal("eff-i");
    let mut tui = overflow_tui();
    // A leading unbound character lets the first frame arrive before the
    // keys, as the shell's first read does, so the seat is held.
    let mut keys = vec![Key::Char('z')];
    keys.extend(overflow_keys());
    script(&keys);
    let mut source = answering(&events, 2..=HELD_BOUND + 2, participant_views);
    let terminal = overflow_drive(&mut tui, &mut source, HELD_BOUND + 3);
    assert_eq!(
        tui.offset, 1,
        "the oldest run was dropped: the retained 64 end at 1, the whole read's 65 at 0"
    );
    assert!(tui.notice.is_none(), "the draw consumed the notice");
    assert_eq!(tui.status, None, "the answering frame said nothing");
    assert!(
        overflow_notice_drawn(&terminal),
        "the overflow notice is on the frame"
    );
}

/// The review's A1 (rulings 5 and 9): a frame whose status line the help
/// overlay covers does not consume the overflow notice. Help is open when
/// the sixty-fifth alternating key overflows the 64-run bound while the
/// read is out; the answering frame carries no note, and the frame drawn
/// after help closes still paints the notice. The retained 64 keys end at
/// 1, where the whole read's 65 end at 0.
#[test]
fn review_753c9_overflow_notice_waits_for_the_uncovered_status_line() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let events = journal("eff-i");
    let mut tui = overflow_tui();
    // `?` opens help before the keys; the second `?` closes it after the
    // answering frame, so the frame after it paints the notice help
    // covered. The unbound `z` lets the first frame arrive, as the shell's
    // first read does, so the seat is held.
    let mut keys = vec![Key::Char('?'), Key::Char('z')];
    keys.extend(overflow_keys());
    keys.extend([Key::Char('?'), Key::Quit]);
    script(&keys);
    let mut source = answering(&events, 3..=HELD_BOUND + 3, participant_views);
    let terminal = overflow_drive(&mut tui, &mut source, HELD_BOUND + 5);
    assert_eq!(
        tui.offset, 1,
        "the oldest run was dropped: the retained 64 end at 1, the whole read's 65 at 0"
    );
    assert!(
        tui.notice.is_none(),
        "the uncovered frame consumed the notice"
    );
    assert!(
        overflow_notice_drawn(&terminal),
        "the overflow notice is on the frame"
    );
}

/// The review's A1 extension (rulings 5 and 9): the shell consumes a
/// notice only on a frame that paints the status line uncovered. A frame
/// too narrow or too short for the line, and one whose help or reader
/// overlay clears the whole frame after painting it, all leave the notice
/// pending; only the final case consumes it.
#[test]
fn review_753c9_only_an_uncovered_frame_consumes_the_notice() {
    let mut tui = Tui::new(None);
    let mut frame = |size: (u16, u16), help: bool, reading: bool| {
        tui.notice = Some("held movement keys dropped: the queue is full".to_string());
        tui.help = help;
        tui.reading = match reading {
            true => Some("a row".to_string()),
            false => None,
        };
        drawn(&mut tui, size);
        tui.notice.is_none()
    };
    assert!(
        !frame((MIN_WIDTH - 1, MIN_HEIGHT), false, false),
        "a narrow frame paints no status line"
    );
    assert!(
        !frame((MIN_WIDTH, MIN_HEIGHT - 1), false, false),
        "a short frame paints no status line"
    );
    assert!(
        !frame((MIN_WIDTH, MIN_HEIGHT), true, false),
        "help covers the status line"
    );
    assert!(
        !frame((MIN_WIDTH, MIN_HEIGHT), false, true),
        "the reader covers the status line"
    );
    assert!(
        frame((MIN_WIDTH, MIN_HEIGHT), false, false),
        "an uncovered frame consumes the notice"
    );
}
