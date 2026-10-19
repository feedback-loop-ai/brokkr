//! #568's run view: a road back is a ruling that lands strictly earlier on
//! the rail than the phase it leaves, and a seat says which visit to its
//! phase it was asked in. Synthetic journals only.
//! A child of the owning `tests.rs`, which sits at its line baseline.

use super::*;

/// One phase entered, one seat asked in it, and the ruling that leaves it.
/// `None` for `next` is the run's end: no transition after the last seat.
fn pass(events: &mut Vec<EventEnvelope>, phase: &str, next: Option<(&str, &str)>) {
    let seq = events.len() as u64 + 1;
    let effect = format!("eff{seq}");
    events.push(ev(
        seq,
        EventType::PhaseEntered,
        json!({"phase": phase}),
        T0,
    ));
    events.push(ev(
        seq + 1,
        EventType::EffectRequested,
        json!({"effect_id": effect, "seat": phase, "phase": phase}),
        T0,
    ));
    if let Some((next, rule)) = next {
        events.push(ev(
            seq + 2,
            EventType::TransitionDecided,
            json!({"from": phase, "next": next, "rule_id": rule}),
            T1,
        ));
    }
}

/// One phase entered and ruled on: `(phase, Some((next, rule)))`.
type Step = (&'static str, Option<(&'static str, &'static str)>);

const IMPL_OK: Step = ("implement", Some(("verify", "IMPL-OK")));
const VERIFY_PASS: Step = ("verify", Some(("review", "VERIFY-PASS")));
const REFORGE: Step = ("review", Some(("implement", "REVIEW-REFORGE")));

/// A journal walked through its steps.
fn walk(steps: &[Step]) -> Vec<EventEnvelope> {
    let mut events = Vec::new();
    for (phase, next) in steps {
        pass(&mut events, phase, *next);
    }
    events
}

/// Each phase as `name ×visits ← returns`.
fn roads(view: &RunView) -> Vec<String> {
    let line = |phase: &Phase| {
        let from = phase.returns.join(", ");
        format!("{} ×{} ← {from}", phase.name, phase.visits)
    };
    view.phases.iter().map(line).collect()
}

fn seats(view: &RunView) -> Vec<(&str, Option<u64>)> {
    view.participants
        .iter()
        .map(|seat| (seat.label.as_str(), seat.visit))
        .collect()
}

#[test]
fn one_reforge_then_a_forward_pass_and_ship_twice_is_one_road_back() {
    // implement → verify → review, sent back once, the second pass
    // forward again, and ship ruled onto itself before it is done.
    let view = run_view(
        &walk(&[
            IMPL_OK,
            VERIFY_PASS,
            REFORGE,
            IMPL_OK,
            VERIFY_PASS,
            ("review", Some(("ship", "REVIEW-PASS"))),
            ("ship", Some(("ship", "SHIP-READY"))),
            ("ship", Some(("done", "SHIP-COMPLETE"))),
        ]),
        None,
    );
    assert_eq!(
        roads(&view),
        [
            "implement ×2 ← review",
            "verify ×2 ← ",
            "review ×2 ← ",
            "ship ×2 ← "
        ]
    );
    assert_eq!(
        seats(&view),
        [
            ("implement", Some(1)),
            ("verify", Some(1)),
            ("review", Some(1)),
            ("implement", Some(2)),
            ("verify", Some(2)),
            ("review", Some(2)),
            ("ship", Some(1)),
            ("ship", Some(2)),
        ]
    );
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(wire["participants"][3]["visit"], json!(2));
}

#[test]
fn two_different_returns_are_two_roads_into_implement_in_journal_order() {
    let view = run_view(
        &walk(&[
            IMPL_OK,
            VERIFY_PASS,
            REFORGE,
            IMPL_OK,
            ("verify", Some(("implement", "VERIFY-FAIL"))),
            IMPL_OK,
            VERIFY_PASS,
            // Taken again: the same road, not a second entry.
            REFORGE,
            ("implement", None),
        ]),
        None,
    );
    assert_eq!(
        roads(&view),
        [
            "implement ×4 ← review, verify",
            "verify ×3 ← ",
            "review ×2 ← "
        ]
    );
    let implement: Vec<Option<u64>> = view
        .participants
        .iter()
        .filter(|seat| seat.phase.as_deref() == Some("implement"))
        .map(|seat| seat.visit)
        .collect();
    assert_eq!(implement, [Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn a_phase_ruled_onto_itself_is_a_retry_not_a_road_back() {
    let view = run_view(
        &walk(&[
            ("implement", Some(("implement", "IMPL-RETRY"))),
            IMPL_OK,
            ("verify", None),
        ]),
        None,
    );
    assert_eq!(roads(&view), ["implement ×2 ← ", "verify ×1 ← "]);
    assert_eq!(
        seats(&view),
        [
            ("implement", Some(1)),
            ("implement", Some(2)),
            ("verify", Some(1))
        ]
    );
}

#[test]
fn a_seat_with_no_phase_or_one_not_yet_entered_has_no_visit() {
    let events = vec![
        ev(
            1,
            EventType::EffectRequested,
            json!({"effect_id": "early", "seat": "intake", "phase": "intake"}),
            T0,
        ),
        ev(2, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "loose", "seat": "loose"}),
            T0,
        ),
        // A request with no effect id is no participant, and takes no slot.
        ev(
            4,
            EventType::EffectRequested,
            json!({"seat": "ghost", "phase": "intake"}),
            T0,
        ),
        ev(5, EventType::PhaseEntered, json!({"phase": 7}), T0),
        ev(
            6,
            EventType::EffectRequested,
            json!({"effect_id": "late", "seat": "intake", "phase": "intake"}),
            T0,
        ),
    ];
    let view = run_view(&events, None);
    assert_eq!(
        seats(&view),
        [("intake", None), ("loose", None), ("intake", Some(1))]
    );
}
