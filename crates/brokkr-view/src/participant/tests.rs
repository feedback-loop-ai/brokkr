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
