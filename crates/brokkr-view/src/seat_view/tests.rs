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
    let rows = page_rows(&mut events[2..4].iter().cloned());
    let row = |at: usize| rows(&events[at]).unwrap();
    let (seat, member) = (row(4), row(5));
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
        let made = rows(&unnamed).map(|row| row.step);
        assert_eq!(made, None, "{key}");
    }
}
