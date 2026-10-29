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

/// A participant's facts pair with what its own rows keep by its effect
/// and member, never by place: facts listed in another order pair as they
/// are listed, and facts naming a participant no row keeps pair with none.
#[test]
fn facts_pair_with_their_own_rows_by_effect_and_member() {
    let at = "2026-01-01T00:00:00Z";
    let asked = |seq, effect| {
        ev(
            seq,
            EventType::EffectRequested,
            json!({"effect_id": effect}),
            at,
        )
    };
    let events = [
        ev(1, EventType::RunStarted, json!({"manifest": {}}), at),
        asked(2, "e"),
        asked(3, "f"),
    ];
    let account = Account::whole("r", &events);
    let homes = |listed: Vec<SeatFacts>| {
        let pairs = paired(listed, account.seats())?;
        let home = |(facts, own): (SeatFacts, Own)| (facts.effect_id, own.home);
        Some(pairs.into_iter().map(home).collect::<Vec<_>>())
    };
    let mut listed = facts(account.seats());
    let (e, f) = (
        ("e".to_string(), Datum::Integer(2)),
        ("f".to_string(), Datum::Integer(3)),
    );
    assert_eq!(homes(listed.clone()), Some(vec![e.clone(), f.clone()]));
    listed.reverse();
    assert_eq!(homes(listed.clone()), Some(vec![f, e]));
    listed[0].effect_id = "g".to_string();
    assert_eq!(homes(listed), None);
}

/// The stamp every event of a run built here is recorded at.
const AT: &str = "2026-01-01T00:00:00Z";

/// A run started, asking for effect `e` and starting its attempt `a`, the
/// start's payload `started`.
fn begun(started: serde_json::Value) -> Vec<EventEnvelope> {
    vec![
        ev(1, EventType::RunStarted, json!({"manifest": {}}), AT),
        ev(2, EventType::EffectRequested, json!({"effect_id": "e"}), AT),
        ev(3, EventType::EffectStarted, started, AT),
    ]
}

/// Seat rows naming a request the trail does not ask are no writer's, so
/// no run view is composed of them, and the reader reads the run whole.
#[test]
fn rows_naming_a_request_the_trail_does_not_ask_compose_no_view() {
    let at = AT;
    let events = begun(json!({"effect_id": "e", "attempt_id": "a"}));
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

/// A run composed whole from its trail, its seat rows and every event its
/// opening kept is its run view read whole, byte for byte: each journal
/// row names its cause, and each participant, the seat's own and its
/// member's, shows its own checkpoints within its attempt's boundary, and
/// no other's; a checkpoint naming no effect, or effect `e:x`, which the
/// run never asks for, is no participant's. No view is composed, so the
/// reader reads the run whole, where the trail and the seat rows compose
/// none, among them a run that asks for effect `e:x`, whose seat's key is
/// `e`'s member's, or where a checkpoint its participant is keyed by names
/// no attempt, as the seat view's page makes no row of it.
#[test]
fn a_run_composed_whole_is_its_run_view() {
    let at = AT;
    let checkpoint = |seq, effect: &str, member: Option<&str>| {
        let mut checkpoint = json!({"step": format!("s{seq}"), "model": "m"});
        if let Some(member) = member {
            checkpoint["member"] = json!(member);
        }
        let payload = json!({"effect_id": effect, "attempt_id": "a", "checkpoint": checkpoint});
        ev(seq, EventType::EffectCheckpointed, payload, at)
    };
    let entries = json!([{"member": null, "boundary": "harness"},
                         {"member": "x", "boundary": "open"}]);
    let started = json!({"effect_id": "e", "attempt_id": "a", "boundary": entries});
    let mut events = begun(started);
    events.extend([
        checkpoint(4, "e", None),
        checkpoint(5, "e", Some("x")),
        checkpoint(6, "e", None),
        ev(
            7,
            EventType::EffectCheckpointed,
            json!({"checkpoint": {}}),
            at,
        ),
    ]);
    events[3].causation_id = Some("ev3".to_string());
    let trail_of = |events: &[EventEnvelope]| -> Vec<EventEnvelope> {
        let trail = events.iter();
        let trail = trail.filter(|event| event.event_type != EventType::EffectCheckpointed);
        trail.cloned().collect()
    };
    let whole = |events: &[EventEnvelope], trail: &[EventEnvelope]| {
        let account = Account::whole("r", events);
        let view = composed_whole(trail, account.seats(), events, at, None);
        view.map(|view| serde_json::to_string(&view).unwrap())
    };
    events.push(checkpoint(8, "e:x", None));
    let today = serde_json::to_string(&crate::run_view(&events, None)).unwrap();
    assert_eq!(whole(&events, &trail_of(&events)), Some(today));
    let unasked = [events[0].clone(), events[2].clone()];
    assert_eq!(whole(&events, &unasked), None);
    let mut keyed_twice = events.clone();
    let asked = json!({"effect_id": "e:x"});
    keyed_twice.insert(2, ev(9, EventType::EffectRequested, asked, at));
    assert_eq!(whole(&keyed_twice, &trail_of(&keyed_twice)), None);
    let view = crate::run_view(&events, None);
    let shown = view.participants.iter();
    let shown: Vec<(&str, Vec<&str>)> = shown
        .map(|part| {
            let rows = part.checkpoints.iter();
            let words = rows.map(|row| row.served.boundary.text.as_str());
            (part.key.as_str(), words.collect())
        })
        .collect();
    assert_eq!(
        shown,
        [("e", vec!["harness", "harness"]), ("e:x", vec!["open"])]
    );
    assert_eq!(view.journal[3].causation_seq, Some(3));
    events[5].payload["attempt_id"] = json!(null);
    assert_eq!(whole(&events, &trail_of(&events)), None);
}

/// A page holding only the acceptance still names the command the trail
/// holds on another page: the command is read from the trail as from the
/// page, so the composed view prints the words the whole run view prints,
/// never a `?` for a command it had in hand.
#[test]
fn a_page_holding_the_acceptance_keeps_the_command_its_trail_holds() {
    let at = AT;
    let events = [
        ev(1, EventType::RunStarted, json!({"manifest": {}}), at),
        ev(2, EventType::PhaseEntered, json!({"phase": "verify"}), at),
        ev(
            3,
            EventType::OperatorCommanded,
            json!({"command_id": "c1", "command": "stop"}),
            at,
        ),
        ev(
            4,
            EventType::OperatorAccepted,
            json!({"command_id": "c1"}),
            at,
        ),
    ];
    let (account, causes) = (Account::whole("r", &events), BTreeMap::new());
    let page = [events[3].clone()];
    let parts = Parts {
        trail: &events,
        seats: account.seats(),
        page: &page,
        causes: &causes,
        head_seq: 4,
        last_at: at,
    };
    let composed = composed_view(&parts, None).unwrap();
    let whole = crate::run_view(&events, None);
    let accepted = |view: &RunView| {
        view.journal
            .iter()
            .find(|row| row.seq == 4)
            .unwrap()
            .what
            .text
            .clone()
    };
    assert_eq!(
        accepted(&composed),
        "stop · accepted at seq 4 (command at seq 3)"
    );
    assert_eq!(accepted(&composed), accepted(&whole));
}

/// A command id reused by a later command never re-labels an earlier
/// disposition: the trail row says the command that stood when the event
/// answered, in a whole read and in a read composed from a page, and a
/// rejection is worded as a rejection.
#[test]
fn a_reused_command_id_keeps_each_disposition_at_its_own_time() {
    let events = [
        ev(1, EventType::RunStarted, json!({"manifest": {}}), AT),
        ev(2, EventType::PhaseEntered, json!({"phase": "verify"}), AT),
        ev(
            3,
            EventType::OperatorCommanded,
            json!({"command_id": "c", "command": "stop"}),
            AT,
        ),
        ev(
            4,
            EventType::OperatorAccepted,
            json!({"command_id": "c"}),
            AT,
        ),
        ev(
            5,
            EventType::OperatorCommanded,
            json!({"command_id": "c", "command": "release"}),
            AT,
        ),
        ev(
            6,
            EventType::OperatorAccepted,
            json!({"command_id": "c"}),
            AT,
        ),
        ev(
            7,
            EventType::OperatorCommanded,
            json!({"command_id": "d", "command": "retry"}),
            AT,
        ),
        ev(
            8,
            EventType::OperatorRejected,
            json!({"command_id": "d", "reason": "the mock is not ready"}),
            AT,
        ),
    ];
    let (account, causes) = (Account::whole("r", &events), BTreeMap::new());
    let page = &events[2..];
    let parts = Parts {
        trail: &events,
        seats: account.seats(),
        page,
        causes: &causes,
        head_seq: 8,
        last_at: AT,
    };
    let composed = composed_view(&parts, None).unwrap();
    let whole = crate::run_view(&events, None);
    let text = |view: &RunView, seq: u64| {
        view.journal
            .iter()
            .find(|row| row.seq == seq)
            .unwrap()
            .what
            .text
            .clone()
    };
    for view in [&composed, &whole] {
        assert_eq!(text(view, 4), "stop · accepted at seq 4 (command at seq 3)");
        assert_eq!(
            text(view, 6),
            "release · accepted at seq 6 (command at seq 5)"
        );
        assert_eq!(
            text(view, 8),
            "retry · rejected at seq 8: the mock is not ready (command at seq 7)"
        );
    }
}
