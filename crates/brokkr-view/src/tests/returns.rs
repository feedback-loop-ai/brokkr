//! #568's run view: a road back is a ruling that lands strictly earlier on
//! the rail than the phase it leaves, counted with the rules that ruled it,
//! and a seat says which visit to its phase it was asked in. Synthetic
//! journals only.
//! A child of the owning `tests.rs`, which sits at its line baseline.

use super::*;

/// One phase entered, one seat asked in it, and the ruling that leaves it.
/// `None` for `next` is the run's end: no transition after the last seat,
/// and an empty rule is a ruling that records none.
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
        let mut ruling = json!({"from": phase, "next": next});
        if !rule.is_empty() {
            ruling["rule_id"] = json!(rule);
        }
        events.push(ev(seq + 2, EventType::TransitionDecided, ruling, T1));
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

/// Each phase as `name ×visits ← from ×taken RULES, …`.
fn roads(view: &RunView) -> Vec<String> {
    let road = |road: &Road| format!("{} ×{} {}", road.from, road.taken, road.rules.join("/"));
    let line = |phase: &Phase| {
        let from: Vec<String> = phase.returns.iter().map(road).collect();
        format!("{} ×{} ← {}", phase.name, phase.visits, from.join(", "))
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
            "implement ×2 ← review ×1 REVIEW-REFORGE",
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
            "implement ×4 ← review ×2 REVIEW-REFORGE, verify ×1 VERIFY-FAIL",
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
fn returns_into_two_different_phases_are_one_road_each_on_its_own_phase() {
    // A road into `verify` is looked for among the roads already taken:
    // the one into `implement` lands elsewhere, so it is passed over and
    // the new road is its own entry on `verify`.
    let view = run_view(
        &walk(&[
            IMPL_OK,
            VERIFY_PASS,
            REFORGE,
            IMPL_OK,
            VERIFY_PASS,
            ("review", Some(("verify", "REVIEW-REVERIFY"))),
            VERIFY_PASS,
            ("review", None),
        ]),
        None,
    );
    assert_eq!(
        roads(&view),
        [
            "implement ×2 ← review ×1 REVIEW-REFORGE",
            "verify ×3 ← review ×1 REVIEW-REVERIFY",
            "review ×3 ← "
        ]
    );
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

#[test]
fn a_road_taken_under_several_rules_counts_every_taking_and_names_each_rule_once_latest_last() {
    // Review sends the run back by REVIEW-REFORGE, then by another rule,
    // then by a ruling that records none, then by REVIEW-REFORGE again.
    let back = |rule| ("review", Some(("implement", rule)));
    let view = run_view(
        &walk(&[
            IMPL_OK,
            VERIFY_PASS,
            REFORGE,
            IMPL_OK,
            VERIFY_PASS,
            back("REVIEW-SECURITY"),
            IMPL_OK,
            VERIFY_PASS,
            back(""),
            IMPL_OK,
            VERIFY_PASS,
            REFORGE,
            ("implement", None),
        ]),
        None,
    );
    assert_eq!(
        roads(&view),
        [
            "implement ×5 ← review ×4 REVIEW-SECURITY/REVIEW-REFORGE",
            "verify ×4 ← ",
            "review ×4 ← "
        ]
    );
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(
        wire["phases"][0]["returns"],
        json!([{"from": "review", "taken": 4, "rules": ["REVIEW-SECURITY", "REVIEW-REFORGE"]}])
    );
}

#[test]
fn the_recorded_table_orders_an_entered_run_and_its_first_return() {
    for first in [VERIFY_PASS, REFORGE] {
        let mut events = walk(&[first, IMPL_OK, VERIFY_PASS, REFORGE, IMPL_OK]);
        events.insert(
            0,
            ev(
                0,
                EventType::RunStarted,
                json!({
                    "phases":["intake","design","implement","verify","review","ship","done","stop"]
                }),
                T0,
            ),
        );
        let view = run_view(&events, None);
        assert_eq!(
            view.phases
                .iter()
                .map(|phase| phase.name.as_str())
                .collect::<Vec<_>>(),
            ["implement", "verify", "review"]
        );
        assert_eq!(view.phases[0].returns.len(), 1);
        assert_eq!(view.phases[0].returns[0].from, "review");
        assert_eq!(view.phases[1].returns.len(), 0);
        assert_eq!(view.phases[2].returns.len(), 0);
    }
}

#[test]
fn a_landings_first_return_uses_its_recorded_order() {
    let mut events = walk(&[
        ("classify", Some(("verify", "CLASSIFY-CODE"))),
        VERIFY_PASS,
        REFORGE,
        IMPL_OK,
        VERIFY_PASS,
        ("review", Some(("ship", "REVIEW-CLEAN"))),
        ("ship", None),
    ]);
    events.insert(
        0,
        ev(
            0,
            EventType::RunStarted,
            json!({"phases":["classify","implement","verify","review","ship","done","stop"]}),
            T0,
        ),
    );
    let view = run_view(&events, None);
    assert_eq!(
        roads(&view),
        [
            "classify ×1 ← ",
            "implement ×1 ← review ×1 REVIEW-REFORGE",
            "verify ×2 ← ",
            "review ×2 ← ",
            "ship ×1 ← ",
        ]
    );
}
