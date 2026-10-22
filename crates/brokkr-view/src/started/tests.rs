use super::*;
use crate::tests::ev;
use serde_json::json;

/// The carried member `a` of the effect `carrying`, as `events` read it.
fn carrying_a(events: &[EventEnvelope]) -> Participant {
    participants(events, &scan_participants(events))
        .into_iter()
        .find(|part| part.key == "carrying:a")
        .unwrap()
}

#[test]
fn a_carried_part_is_completed_and_names_its_original_attempt() {
    let at = "2026-01-01T00:00:00Z";
    let mut events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id":"source", "phase":"review", "seat":"review"}),
            at,
        ),
        ev(
            2,
            EventType::EffectStarted,
            json!({"effect_id":"source", "attempt_id":"one", "driver":"panel", "provenance":[{"member":"a", "agent":"reviewer", "model":"m", "provider":"test"}]}),
            at,
        ),
        ev(
            3,
            EventType::EffectIndeterminate,
            json!({"effect_id":"source", "attempt_id":"one", "reason":"b vanished"}),
            at,
        ),
        ev(
            4,
            EventType::EffectRequested,
            json!({"effect_id":"carrying", "phase":"review", "seat":"review"}),
            at,
        ),
        ev(
            5,
            EventType::EffectStarted,
            json!({"effect_id":"carrying", "attempt_id":"two", "driver":"panel", "carried":[{"member":"a", "attempt_id":"one"}, {"member":"bad"}]}),
            at,
        ),
    ];
    let scan = scan_participants(&events);
    let parts = participants(&events, &scan);
    let carried = parts.iter().find(|part| part.key == "carrying:a").unwrap();
    assert_eq!(
        (&*carried.status, &*carried.status_class),
        ("carried", "completed")
    );
    assert_eq!(
        carried.carried_from,
        Some(CarriedFrom {
            attempt_id: "one".into(),
            ordinal: 1
        })
    );
    assert_eq!(carried.activity.text, "carried from review attempt 1");
    assert_eq!(carried.activity.note.as_deref(), Some("one"));
    assert!(!carried.working);
    assert_eq!(carried.cost, None);
    assert_eq!(carried.served.model.text, "—");
    assert_eq!(carried.transcript, None);
    assert!(!parts.iter().any(|part| part.key == "carrying:bad"));
    let expected = serde_json::to_value(compare::resolutions(&events[..4])).unwrap();
    let actual = serde_json::to_value(compare::resolutions(&events)).unwrap();
    assert_eq!(actual["review:a"], expected["review:a"]);
    events.push(ev(
        6,
        EventType::EffectStarted,
        json!({"effect_id":"carrying", "attempt_id":"three", "driver":"panel"}),
        at,
    ));
    let cleared = carrying_a(&events);
    assert_eq!(cleared.carried_from, None);
    assert_eq!(cleared.status, "working");
    // An entry naming an attempt the journal never started has no source to name.
    events.push(ev(
        7,
        EventType::EffectStarted,
        json!({"effect_id":"carrying", "attempt_id":"four", "driver":"panel", "carried":[{"member":"a", "attempt_id":"ghost"}]}),
        at,
    ));
    let unsourced = carrying_a(&events);
    assert_eq!(unsourced.carried_from, None);
    assert_eq!(unsourced.status, "carried");
}

#[test]
fn a_later_start_replaces_the_carried_source_and_counts_every_phase_attempt() {
    let at = "2026-01-01T00:00:00Z";
    let mut events = Vec::new();
    for (kind, payload) in [
        (
            EventType::EffectRequested,
            json!({"effect_id":"source","phase":"review","seat":"review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"source","attempt_id":"one","driver":"panel"}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id":"source","attempt_id":"one"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"source","attempt_id":"two","driver":"panel"}),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id":"source","attempt_id":"two"}),
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id":"carrying","phase":"review","seat":"review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"carrying","attempt_id":"three","driver":"panel","carried":[{"member":"a","attempt_id":"one"}]}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id":"carrying","attempt_id":"three"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"carrying","attempt_id":"four","driver":"panel","carried":[{"member":"a","attempt_id":"two"}]}),
        ),
    ] {
        events.push(ev(events.len() as u64 + 1, kind, payload, at));
    }
    let part = carrying_a(&events);
    assert_eq!(
        part.carried_from,
        Some(CarriedFrom {
            attempt_id: "two".into(),
            ordinal: 2
        })
    );
    assert_eq!(part.activity.text, "carried from review attempt 2");
    assert_eq!(part.activity.note.as_deref(), Some("two"));
    assert_eq!(
        (
            part.status.as_str(),
            part.status_class.as_str(),
            part.working
        ),
        ("carried", "completed", false)
    );
    // A source in the carrying effect itself: the ordinal counts every start of the phase.
    events.push(ev(
        10,
        EventType::EffectStarted,
        json!({"effect_id":"carrying","attempt_id":"five","driver":"panel","carried":[{"member":"a","attempt_id":"three"}]}),
        at,
    ));
    let part = carrying_a(&events);
    assert_eq!(
        part.carried_from,
        Some(CarriedFrom {
            attempt_id: "three".into(),
            ordinal: 3
        })
    );
    assert_eq!(part.activity.text, "carried from review attempt 3");
}

#[test]
fn a_refused_carry_parks_with_its_stated_reason_as_a_gate_move_does() {
    for reason in [
        "effect e1 indeterminate: carrying attempt 'two' changed its reviewed subject: ...",
        "effect e1 indeterminate: GATE-MOVED-HEAD: the gate's head moved",
    ] {
        let state = crate::tests::parked_state("review", reason);
        assert_eq!(summary(&state).park_reason.as_deref(), Some(reason));
    }
}
