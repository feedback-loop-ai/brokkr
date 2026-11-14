//! A repaired run's late tail in the view's own scans (decision 0108
//! ruling 4, F7): a late supersede is kept and shown, never applied.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::fold;
use serde_json::{json, Value};

use crate::tests::ev;
use crate::{residual_findings, residual_findings_at, resolutions_at, run_view};

const T0: &str = "2026-01-01T00:00:00Z";
const T1: &str = "2026-01-01T00:00:05Z";

/// A review hold carrying a security residual, at `seq`.
fn security_hold(seq: u64) -> brokkr_core::EventEnvelope {
    ev(
        seq,
        EventType::TransitionDecided,
        json!({"from": "review", "rule_id": "REVIEW-SECURITY-HOLD", "next": "done",
               "result": "residual",
               "inputs": {"has_security_residual": true}}),
        T0,
    )
}

/// The late supersede both view tests plant: it closes finding `8`, and
/// no readout applies it.
fn late_supersede() -> (EventType, serde_json::Value) {
    (
        EventType::OperatorCommanded,
        json!({"command_id": "c-supersede", "command": "supersede", "operator": "operator",
               "args": {"findings": [8],
                        "by": {"realm": null, "run_id": "later", "seq": 21},
                        "reason": "late"}}),
    )
}

/// F7's view control: a late supersede inside a repaired tail is kept and
/// shown, never applied, so the finding stays open. The same scan without
/// the state still reads the annotation.
#[test]
fn a_late_supersede_closes_no_finding_in_a_repaired_tail() {
    let mut events = crate::tests::seat_journal();
    events.push(security_hold(8));
    events.push(ev(9, EventType::PhaseEntered, json!({"phase": "done"}), T1));
    events.push(ev(
        10,
        EventType::RunStopped,
        json!({"reason": "concluded"}),
        T1,
    ));
    let (supersede, supersede_args) = late_supersede();
    events.push(ev(11, supersede, supersede_args, T1));
    events.push(ev(
        12,
        EventType::OperatorCommanded,
        json!({"command_id": "c-repair", "command": "repair", "operator": "operator",
               "args": {"first_seq": 11, "last_seq": 11, "terminal_seq": 10,
                        "event_types": ["operator/commanded"], "reason": "kept"}}),
        T1,
    ));
    let state = fold(&events).expect("the repaired run folds");
    assert_eq!(state.late_tail.as_ref().map(|tail| tail.last_seq), Some(11));
    let shown = residual_findings_at("r1", &events, Some(&state));
    assert_eq!(shown.len(), 1);
    assert!(
        shown[0].superseded.is_none(),
        "the late supersede is never applied"
    );
    let unguarded = residual_findings("r1", &events);
    assert!(
        unguarded[0].superseded.is_some(),
        "without the state the same scan reads the annotation"
    );
}

/// M3: the whole run view of a repaired journal equals the view of the
/// journal cut at its terminal event, in every derived field. The raw
/// journal list and the count are how a late event is SHOWN, and the
/// late-tail sentence names what they withhold, so the test blanks those
/// and the moved head's seq.
#[test]
fn a_repaired_runs_view_equals_the_terminal_cuts() {
    let mut events = crate::tests::seat_journal();
    events.push(security_hold(8));
    events.push(ev(
        9,
        EventType::RunStopped,
        json!({"reason": "concluded"}),
        T1,
    ));
    // The late tail: a classify, a supersede, an effect event, a cost and
    // a phase move — every figure a raw scan would move.
    events.push(ev(
        10,
        EventType::OperatorCommanded,
        json!({"command_id": "c-classify", "command": "classify", "operator": "operator",
               "args": {"cause": "withdrawn", "reason": "late"}}),
        T1,
    ));
    let (supersede, supersede_args) = late_supersede();
    events.push(ev(11, supersede, supersede_args, T1));
    events.push(ev(
        12,
        EventType::EffectSucceeded,
        json!({"effect_id": "late", "attempt_id": "late",
               "result": {"result": "complete", "notes": "late settlement"}}),
        T1,
    ));
    events.push(ev(
        13,
        EventType::EffectCheckpointed,
        json!({"effect_id": "late", "attempt_id": "late",
               "checkpoint": {"step": "turn", "total_cost_usd": 100.0}}),
        T1,
    ));
    events.push(ev(
        14,
        EventType::PhaseEntered,
        json!({"phase": "done"}),
        T1,
    ));
    events.push(ev(
        15,
        EventType::OperatorCommanded,
        json!({"command_id": "c-repair", "command": "repair", "operator": "operator",
               "args": {"first_seq": 10, "last_seq": 14, "terminal_seq": 9,
                        "event_types": ["operator/commanded", "operator/commanded",
                                        "effect/succeeded", "effect/checkpointed",
                                        "phase/entered"],
                        "reason": "kept"}}),
        T1,
    ));
    let state = fold(&events).expect("the repaired run folds");
    let cut = fold(&events[..9]).expect("the terminal cut folds");

    let view = |events: &[brokkr_core::EventEnvelope], state| {
        let mut value = serde_json::to_value(run_view(events, Some(state))).unwrap();
        for (path, blank) in [
            ("/summary/late_tail", Value::Null),
            ("/summary/seq", Value::Null),
            ("/journal", Value::Null),
            ("/event_count", Value::Null),
        ] {
            if let Some(cell) = value.pointer_mut(path) {
                *cell = blank;
            }
        }
        value
    };
    assert_eq!(
        view(&events, &state),
        view(&events[..9], &cut),
        "every derived field is the terminal cut's"
    );
}

/// M3's compare scan: a late invocation site resolves nothing, so the
/// repaired run's resolutions are the terminal cut's.
#[test]
fn a_repaired_runs_resolutions_equal_the_terminal_cuts() {
    let mut events = crate::tests::seat_journal();
    events.push(ev(
        8,
        EventType::RunStopped,
        json!({"reason": "concluded"}),
        T1,
    ));
    events.push(ev(
        9,
        EventType::EffectStarted,
        json!({"effect_id": "late", "attempt_id": "late",
               "provenance": [{"agent": "a", "model": "late-model", "provider": "p"}]}),
        T1,
    ));
    events.push(ev(
        10,
        EventType::OperatorCommanded,
        json!({"command_id": "c-repair", "command": "repair", "operator": "operator",
               "args": {"first_seq": 9, "last_seq": 9, "terminal_seq": 8,
                        "event_types": ["effect/started"], "reason": "kept"}}),
        T1,
    ));
    let state = fold(&events).expect("the repaired run folds");
    let cut = fold(&events[..8]).expect("the terminal cut folds");
    assert_eq!(
        serde_json::to_value(resolutions_at(&events, Some(&state))).unwrap(),
        serde_json::to_value(resolutions_at(&events[..8], Some(&cut))).unwrap(),
        "the late invocation site resolves nothing"
    );
}
