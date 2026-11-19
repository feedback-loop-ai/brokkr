//! Fix round 0109a1r4, K2, bound: the fold carries the in-flight attempt's
//! latest engine-authored gate step observation, and nothing else; V1
//! (r5): with the record's `covered`, which must agree with its head.

use serde_json::{json, Value};

use super::StepObservation;
use crate::canonical::sha256_bytes;
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

/// A step checkpoint of `attempt` whose result journals `head` and its
/// `covered`, the engine's own when `authored`.
fn step(attempt: &str, head: &str, authored: bool) -> EventEnvelope {
    let mut checkpoint = json!({"step": "sequence-step-finished", "step_name": "judge",
                                "result": {"result": "clean", "checked_head": head,
                                           "covered": digest_of(head)}});
    if authored {
        checkpoint["authored"] = json!("engine");
    }
    let payload = json!({"effect_id": "e1", "attempt_id": attempt, "checkpoint": checkpoint});
    event(EventType::EffectCheckpointed, payload)
}

fn checked(head: &str) -> Option<StepObservation> {
    Some(StepObservation::Checked {
        head: head.to_string().try_into().unwrap(),
        covered: serde_json::from_value(digest_of(head)).unwrap(),
        step: Some("judge".into()),
    })
}

/// Fix round 0109a1r6, W1: an authored step record is whole or partial,
/// never dropped; a work step's record, carrying neither field, is no
/// observation; and the success names the step that produced it.
#[test]
fn a_partial_step_record_is_kept_as_partial_and_the_success_names_its_step() {
    let mut state = flying();
    apply(&mut state, &step("a1", A, true)).unwrap();
    let mut work = step("a1", B, true);
    let result = &mut work.payload["checkpoint"]["result"];
    result
        .as_object_mut()
        .unwrap()
        .retain(|field, _| field == "result");
    apply(&mut state, &work).unwrap();
    assert_eq!(state.step_checked, checked(A));
    for dropped in ["checked_head", "covered"] {
        let mut partial = step("a1", A, true);
        let result = partial.payload["checkpoint"]["result"]
            .as_object_mut()
            .unwrap();
        result.remove(dropped);
        let mut state = flying();
        apply(&mut state, &partial).unwrap();
        assert_eq!(
            state.step_checked,
            Some(StepObservation::Partial),
            "{dropped}"
        );
    }
    let succeeded = json!({"effect_id": "e1", "attempt_id": "a1", "step": "judge",
                           "result": {"result": "clean"}});
    apply(&mut state, &event(EventType::EffectSucceeded, succeeded)).unwrap();
    assert_eq!(state.ending_step.as_deref(), Some("judge"));
    assert_eq!(state.step_checked, checked(A));
}

/// The protected phase's `covered` of `head`.
fn digest_of(head: &str) -> Value {
    json!({"kind": "head", "head_digest": sha256_bytes(head.as_bytes()), "dirty": true})
}

/// Fix round 0109a1r5, V1: the fold keeps each record's `covered`, and a
/// record whose own `checked_head` and `covered` disagree, or whose
/// `covered` does not read, is no observation decide can agree with.
#[test]
fn the_fold_keeps_a_records_covered_and_refuses_one_that_disagrees_with_its_head() {
    let covered = |head: &str, covered: Value| {
        let mut record = step("a1", head, true);
        record.payload["checkpoint"]["result"]["covered"] = covered;
        record
    };
    let mut state = flying();
    apply(&mut state, &covered(A, digest_of(A))).unwrap();
    assert_eq!(state.step_checked, checked(A));
    apply(&mut state, &covered(A, digest_of(B))).unwrap();
    assert_eq!(state.step_checked, Some(StepObservation::Unreadable));
    apply(&mut state, &covered(B, json!({"kind": "head"}))).unwrap();
    assert_eq!(state.step_checked, Some(StepObservation::Unreadable));
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
