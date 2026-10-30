//! The shell settles held movement only on a successful read (#753 fix
//! round 753c6): a transient error keeps the frame and the queue, and the
//! next answer lands the key. Split from `tests.rs` to keep that file
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
