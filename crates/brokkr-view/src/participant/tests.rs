use serde_json::{json, Value};

use crate::compare::tests::seat_journal::boxed_seat;

/// The participant as the browser receives it.
fn served(events: &[brokkr_core::envelope::EventEnvelope]) -> Value {
    serde_json::to_value(crate::run_view(events, None)).unwrap()["participants"][0].clone()
}

/// The served participant says whether it is at work, so `ui.html` paints
/// the flag rather than testing the status word itself (#351).
#[test]
fn the_served_participant_says_whether_it_is_working() {
    let mut events = boxed_seat("seat", "harness");
    let finished = served(&events);
    assert_eq!(finished["status"], "succeeded");
    assert_eq!(finished["working"], json!(false));
    events.truncate(3);
    let working = served(&events);
    assert_eq!(working["status"], "working");
    assert_eq!(working["working"], json!(true));
}

/// A participant's seat is its key read back as [`key_of`](super::key_of)
/// writes it: its effect's id, and its member where the key ends in `:`
/// and the member's name; no seat where the key does not end so.
#[test]
fn a_participant_s_seat_is_its_key_read_back() {
    let view = crate::run_view(&boxed_seat("seat", "harness"), None);
    let mut part = view.participants.into_iter().next().unwrap();
    let seat = |part: &crate::Participant| {
        let seat = part.seat();
        seat.map(|(effect, member)| (effect.to_string(), member.map(str::to_string)))
    };
    let effect = part.key.clone();
    assert_eq!(seat(&part), Some((effect.clone(), None)));
    part.member = Some("m".to_string());
    part.key = super::key_of(&effect, Some("m"));
    assert_eq!(seat(&part), Some((effect.clone(), Some("m".to_string()))));
    part.key = format!("{effect}m");
    assert_eq!(seat(&part), None);
    part.key = effect;
    assert_eq!(seat(&part), None);
}
