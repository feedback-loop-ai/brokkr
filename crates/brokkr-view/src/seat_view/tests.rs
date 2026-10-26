use serde_json::json;

use super::*;
use crate::tests::ev;
use crate::{run_view, Participant};

/// A page's checkpoint, made into its row beside the starts naming its
/// attempt, is the row the participant scan makes of it: its boundary its
/// member's entry of the attempt's last start, which names the seat's own
/// and no member's, so the member's shows none.
#[test]
fn a_page_s_checkpoint_is_the_row_the_scan_makes_of_it() {
    let at = "2026-01-01T00:00:00Z";
    let start = |seq, member: Value, word: &str| {
        let boundary = json!([{"member": member, "boundary": word, "gate": true}]);
        let payload = json!({"effect_id": "e", "attempt_id": "a", "boundary": boundary});
        ev(seq, EventType::EffectStarted, payload, at)
    };
    let checkpoint = |seq, checkpoint: Value| {
        let payload = json!({"effect_id": "e", "attempt_id": "a", "checkpoint": checkpoint});
        ev(seq, EventType::EffectCheckpointed, payload, at)
    };
    let asked = json!({"effect_id": "e", "seat": "panel"});
    let turn = json!({"step": "seat-turn", "tool": "Edit", "turn": 1, "model": "opus"});
    let events = vec![
        ev(1, EventType::RunStarted, json!({"manifest": {}}), at),
        ev(2, EventType::EffectRequested, asked, at),
        start(3, json!("m"), "namespace"),
        start(4, Value::Null, "harness"),
        checkpoint(5, turn),
        checkpoint(6, json!({"member": "m", "step": "panel-member-finished"})),
    ];
    let rows = |member| page_rows(member, &mut events[2..4].iter().cloned());
    let (seat_rows, member_rows) = (rows(None), rows(Some("m")));
    let row = |at: usize| match at {
        4 => seat_rows(&events[at]).unwrap(),
        _ => member_rows(&events[at]).unwrap(),
    };
    let (seat, member) = (row(4), row(5));
    // A page's rows are its participant's: the seat's own makes no row of
    // its member's checkpoint, nor the member's of the seat's.
    let other = [seat_rows(&events[5]), member_rows(&events[4])];
    assert_eq!(other.map(|row| row.map(|row| row.step)), [None, None]);
    assert_eq!(seat.served.boundary.text, "harness");
    assert_eq!(
        (member.served.boundary.absent, member.served.boundary.note),
        (true, Some(NO_BOUNDARY.to_string()))
    );
    let view = run_view(&events, None);
    let shown = |participant: &Participant| serde_json::to_string(&participant.checkpoints);
    let paged = |at: usize| serde_json::to_string(&[row(at)]);
    let keys: Vec<&str> = view.participants.iter().map(|part| &*part.key).collect();
    assert_eq!(keys, ["e", "e:m"]);
    assert_eq!(shown(&view.participants[0]).unwrap(), paged(4).unwrap());
    assert_eq!(shown(&view.participants[1]).unwrap(), paged(5).unwrap());
    // A record naming no attempt, or no effect, makes no row of a page.
    for key in ["attempt_id", "effect_id"] {
        let mut unnamed = events[4].clone();
        unnamed.payload.as_object_mut().unwrap().remove(key);
        let made = seat_rows(&unnamed).map(|row| row.step);
        assert_eq!(made, None, "{key}");
    }
}

/// What a page keeps of a start is the words its rows read, the labels of
/// other members let go of: a start naming two members gives the member's
/// row its own entry's word and the seat's own row both words joined, as
/// the participant scan gives them.
#[test]
fn a_page_keeps_the_words_its_rows_read_of_a_start() {
    let at = "2026-01-01T00:00:00Z";
    let boundary = json!([
        {"member": "another", "boundary": "namespace", "gate": true},
        {"member": "m", "boundary": "harness", "gate": true},
    ]);
    let started = json!({"effect_id": "e", "attempt_id": "a", "boundary": boundary});
    let checkpoint = |seq, checkpoint: Value| {
        let payload = json!({"effect_id": "e", "attempt_id": "a", "checkpoint": checkpoint});
        ev(seq, EventType::EffectCheckpointed, payload, at)
    };
    let events = vec![
        ev(1, EventType::RunStarted, json!({"manifest": {}}), at),
        ev(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "panel"}),
            at,
        ),
        ev(3, EventType::EffectStarted, started, at),
        checkpoint(4, json!({"step": "seat-turn", "tool": "Edit", "turn": 1})),
        checkpoint(5, json!({"member": "m", "step": "panel-member-finished"})),
    ];
    let boundary = |member, at: usize| {
        let rows = page_rows(member, &mut events[2..3].iter().cloned());
        rows(&events[at]).unwrap().served.boundary.text
    };
    assert_eq!(
        [boundary(None, 3), boundary(Some("m"), 4)],
        ["harness, namespace", "harness"]
    );
    // Handed no start of its attempt, a row shows no boundary.
    let unstarted = page_rows(None, &mut events[..2].iter().cloned());
    let shown = unstarted(&events[3]).unwrap().served.boundary;
    assert_eq!(
        (shown.absent, shown.note),
        (true, Some(NO_BOUNDARY.to_string()))
    );
    let view = run_view(&events, None);
    let shown: Vec<(&str, &str)> = view
        .participants
        .iter()
        .filter_map(|part| Some((&*part.key, &*part.checkpoints.first()?.served.boundary.text)))
        .collect();
    assert_eq!(shown, [("e", "harness, namespace"), ("e:m", "harness")]);
}
