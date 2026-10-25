use brokkr_core::summary::Account;
use serde_json::json;

use super::*;
use crate::tests::ev;

/// A participant is read by its row's request, effect and member, and its
/// own model and pin, at the places `SeatRow::COLUMNS` names them.
#[test]
fn a_participant_is_read_by_its_row_s_request_effect_and_member() {
    let named = [ASKED, EFFECT, MEMBER].map(|at| SeatRow::COLUMNS[at].0);
    assert_eq!(named, ["request_seq", "effect_id", "member"]);
    let stated = [MODEL, PIN].map(|pair| pair.map(|at| SeatRow::COLUMNS[at].0));
    assert_eq!(stated, [["model_seq", "model"], ["pin_seq", "pin"]]);
}

/// Each outcome a participant's rows keep shows the status the scan shows
/// of it, and its class.
#[test]
fn each_outcome_shows_its_status() {
    let outcomes = [
        Outcome::Working,
        Outcome::Succeeded,
        Outcome::Failed,
        Outcome::Indeterminate,
        Outcome::Carried,
    ];
    assert_eq!(
        outcomes.map(status),
        [
            ("working", "running"),
            ("succeeded", "completed"),
            ("failed", "stopped"),
            ("indeterminate", "awaiting_operator"),
            ("carried", "completed"),
        ]
    );
}

/// Seat rows naming a request the trail does not ask are no writer's, so
/// no run view is composed of them, and the reader reads the run whole.
#[test]
fn rows_naming_a_request_the_trail_does_not_ask_compose_no_view() {
    let at = "2026-01-01T00:00:00Z";
    let events = [
        ev(1, EventType::RunStarted, json!({"manifest": {}}), at),
        ev(2, EventType::EffectRequested, json!({"effect_id": "e"}), at),
        ev(
            3,
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a"}),
            at,
        ),
    ];
    let (account, causes) = (Account::whole("r", &events), BTreeMap::new());
    let composed = |trail| {
        let parts = Parts {
            trail,
            seats: account.seats(),
            page: &events,
            causes: &causes,
            head_seq: 3,
            last_at: at,
        };
        composed_view(&parts, None).map(|view| view.participants.len())
    };
    assert_eq!(composed(&events), Some(1));
    assert_eq!(composed(&[events[0].clone(), events[2].clone()]), None);
}
