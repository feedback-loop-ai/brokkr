//! Fix round 0109a1r4, K2, bound: the fold carries the in-flight attempt's
//! latest engine-authored gate step observation, and nothing else.

use serde_json::{json, Value};

use super::StepObservation;
use crate::envelope::{EventEnvelope, EventType};
use crate::envelope_builder::EnvelopeBuilder;
use crate::fold::{apply, Cursor, Run, RunState};
use crate::run_state::run_state;

const A: &str = "0123456789abcdef0123456789abcdef01234567";
const B: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload)
        .run("r1")
        .seq(2)
        .event_id("e2")
        .at("2026-11-19T00:00:00Z")
        .previous("")
        .hash("hash")
        .build()
}

/// Attempt `a1` of effect `e1` in flight.
fn flying() -> RunState {
    run_state(
        "r1",
        1,
        Run::Running(Cursor::EffectInFlight {
            effect_id: "e1".into(),
            attempt_id: "a1".into(),
            seat: "review".into(),
            failed_attempts: 0,
            stop_rides: false,
        }),
    )
}

/// A step checkpoint of `attempt` whose result journals `head`, the
/// engine's own when `authored`.
fn step(attempt: &str, head: &str, authored: bool) -> EventEnvelope {
    let mut checkpoint = json!({"step": "sequence-step-finished", "step_name": "judge",
                                "result": {"result": "clean", "checked_head": head}});
    if authored {
        checkpoint["authored"] = json!("engine");
    }
    let payload = json!({"effect_id": "e1", "attempt_id": attempt, "checkpoint": checkpoint});
    event(EventType::EffectCheckpointed, payload)
}

fn checked(head: &str) -> Option<StepObservation> {
    Some(StepObservation::Checked(
        head.to_string().try_into().unwrap(),
    ))
}

#[test]
fn the_fold_carries_the_attempts_latest_authored_gate_step_observation() {
    let mut state = flying();
    apply(&mut state, &step("a1", A, true)).unwrap();
    assert_eq!(state.step_checked, checked(A));
    apply(&mut state, &step("a1", B, true)).unwrap();
    assert_eq!(state.step_checked, checked(B));
    // A driver's record, and another attempt's, are no observation.
    apply(&mut state, &step("a1", A, false)).unwrap();
    apply(&mut state, &step("a0", A, true)).unwrap();
    assert_eq!(state.step_checked, checked(B));
    apply(&mut state, &step("a1", "HEAD", true)).unwrap();
    assert_eq!(state.step_checked, Some(StepObservation::Unreadable));
    // The success carries it to decide.
    let succeeded = json!({"effect_id": "e1", "attempt_id": "a1", "result": {"result": "clean"}});
    apply(&mut state, &event(EventType::EffectSucceeded, succeeded)).unwrap();
    assert!(matches!(state.run.cursor(), Some(Cursor::Decide { .. })));
    assert_eq!(state.step_checked, Some(StepObservation::Unreadable));
}

#[test]
fn a_new_attempt_starts_with_no_step_observation() {
    let mut state = flying();
    apply(&mut state, &step("a1", A, true)).unwrap();
    let failed = json!({"effect_id": "e1", "attempt_id": "a1", "error": "lost"});
    apply(&mut state, &event(EventType::EffectFailed, failed)).unwrap();
    let started = json!({"effect_id": "e1", "attempt_id": "a2"});
    apply(&mut state, &event(EventType::EffectStarted, started)).unwrap();
    assert_eq!(state.step_checked, None);
}
