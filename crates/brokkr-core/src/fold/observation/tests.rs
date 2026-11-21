//! Fix round 0109a1r4, K2, bound: the fold carries the in-flight attempt's
//! engine-authored step records, and nothing else; V1 (r5): with each
//! record's `covered`, which must agree with its head; X1 (r7): every
//! record by its step, and the first malformed one latched.

use serde_json::{json, Value};

use super::{StepObservation, StepRecords};
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

/// Step `name`'s checkpoint of `attempt` whose result journals `head` and
/// its `covered`, the engine's own when `authored`.
fn named(name: &str, attempt: &str, head: &str, authored: bool) -> EventEnvelope {
    let mut checkpoint = json!({"step": "sequence-step-finished", "step_name": name,
                                "result": {"result": "clean", "checked_head": head,
                                           "covered": digest_of(head)}});
    if authored {
        checkpoint["authored"] = json!("engine");
    }
    let payload = json!({"effect_id": "e1", "attempt_id": attempt, "checkpoint": checkpoint});
    event(EventType::EffectCheckpointed, payload)
}

/// `judge`'s checkpoint, as [`named`].
fn step(attempt: &str, head: &str, authored: bool) -> EventEnvelope {
    named("judge", attempt, head, authored)
}

fn checked(head: &str) -> StepObservation {
    StepObservation::Checked {
        head: head.to_string().try_into().unwrap(),
        covered: serde_json::from_value(digest_of(head)).unwrap(),
    }
}

/// `step`'s record in `state`.
fn judged<'s>(state: &'s RunState, step: &str) -> Option<&'s StepObservation> {
    state.step_records.by_step.get(step)
}

/// Fix round 0109a1r6, W1: an authored step record is whole or partial,
/// never dropped; a work step's record, carrying neither field, is
/// unobserved (X1: kept by its step, never read as no record); and the
/// success names the step that produced it.
#[test]
fn a_partial_step_record_is_kept_as_partial_and_the_success_names_its_step() {
    let mut state = flying();
    apply(&mut state, &step("a1", A, true)).unwrap();
    let mut work = named("tidy", "a1", B, true);
    let result = &mut work.payload["checkpoint"]["result"];
    result
        .as_object_mut()
        .unwrap()
        .retain(|field, _| field == "result");
    apply(&mut state, &work).unwrap();
    assert_eq!(judged(&state, "judge"), Some(&checked(A)));
    assert_eq!(judged(&state, "tidy"), Some(&StepObservation::Unobserved));
    assert_eq!(state.step_records.malformed, None);
    for dropped in ["checked_head", "covered"] {
        let mut partial = step("a1", A, true);
        let result = partial.payload["checkpoint"]["result"]
            .as_object_mut()
            .unwrap();
        result.remove(dropped);
        let mut state = flying();
        apply(&mut state, &partial).unwrap();
        let partial = Some(StepObservation::Partial);
        assert_eq!(judged(&state, "judge"), partial.as_ref(), "{dropped}");
        assert_eq!(state.step_records.malformed, partial, "{dropped}");
    }
    let succeeded = json!({"effect_id": "e1", "attempt_id": "a1", "step": "judge",
                           "result": {"result": "clean"}});
    apply(&mut state, &event(EventType::EffectSucceeded, succeeded)).unwrap();
    assert_eq!(state.ending_step.as_deref(), Some("judge"));
    assert_eq!(judged(&state, "judge"), Some(&checked(A)));
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
    assert_eq!(judged(&state, "judge"), Some(&checked(A)));
    for (head, covered_by) in [(A, digest_of(B)), (B, json!({"kind": "head"}))] {
        let mut state = flying();
        apply(&mut state, &covered(head, covered_by)).unwrap();
        let unreadable = Some(StepObservation::Unreadable);
        assert_eq!(judged(&state, "judge"), unreadable.as_ref());
        assert_eq!(state.step_records.malformed, unreadable);
    }
}

#[test]
fn the_fold_carries_the_attempts_authored_step_records_by_step() {
    let mut state = flying();
    apply(&mut state, &step("a1", A, true)).unwrap();
    assert_eq!(judged(&state, "judge"), Some(&checked(A)));
    apply(&mut state, &named("chief", "a1", B, true)).unwrap();
    assert_eq!(judged(&state, "judge"), Some(&checked(A)));
    assert_eq!(judged(&state, "chief"), Some(&checked(B)));
    // Each step is kept in the order its record was journaled (fix round
    // 0109a1r8, M3), whatever the map's own order.
    apply(&mut state, &named("aide", "a1", B, true)).unwrap();
    assert_eq!(state.step_records.order, ["judge", "chief", "aide"]);
    // A driver's record, and another attempt's, are no record.
    apply(&mut state, &named("chief", "a1", A, false)).unwrap();
    apply(&mut state, &named("chief", "a0", A, true)).unwrap();
    assert_eq!(judged(&state, "chief"), Some(&checked(B)));
    assert_eq!(state.step_records.malformed, None);
    assert_eq!(state.step_records.order, ["judge", "chief", "aide"]);
    // The success carries them to decide.
    let succeeded = json!({"effect_id": "e1", "attempt_id": "a1", "result": {"result": "clean"}});
    apply(&mut state, &event(EventType::EffectSucceeded, succeeded)).unwrap();
    assert!(matches!(state.run.cursor(), Some(Cursor::Decide { .. })));
    assert_eq!(judged(&state, "chief"), Some(&checked(B)));
}

/// Fix round 0109a1r7, X1: a step runs once in an attempt, so a second
/// whole record of it, a copy or a replacement, is latched as unreadable.
#[test]
fn a_second_record_of_one_step_in_an_attempt_is_latched() {
    let mut state = flying();
    apply(&mut state, &step("a1", A, true)).unwrap();
    apply(&mut state, &step("a1", B, true)).unwrap();
    assert_eq!(judged(&state, "judge"), Some(&checked(B)));
    let unreadable = Some(StepObservation::Unreadable);
    assert_eq!(state.step_records.malformed, unreadable);
    assert_eq!(state.step_records.order, ["judge"]);
}

/// Fix round 0109a1r7, X1: a partial or unreadable record is latched for
/// the attempt, so a later complete record of the same step heals
/// nothing; an authored record naming no step, or carrying no result, is
/// unreadable and latched too.
#[test]
fn a_malformed_step_record_stays_latched_whatever_record_follows_it() {
    let mut state = flying();
    let mut partial = step("a1", A, true);
    let result = partial.payload["checkpoint"]["result"]
        .as_object_mut()
        .unwrap();
    result.remove("covered");
    apply(&mut state, &partial).unwrap();
    apply(&mut state, &step("a1", A, true)).unwrap();
    assert_eq!(judged(&state, "judge"), Some(&checked(A)));
    let latched = Some(StepObservation::Partial);
    assert_eq!(state.step_records.malformed, latched);
    apply(&mut state, &step("a1", "HEAD", true)).unwrap();
    assert_eq!(state.step_records.malformed, latched, "the first is kept");

    for strip in ["step_name", "result"] {
        let mut unnamed = step("a1", A, true);
        let checkpoint = unnamed.payload["checkpoint"].as_object_mut().unwrap();
        checkpoint.remove(strip);
        let mut state = flying();
        apply(&mut state, &unnamed).unwrap();
        let unreadable = Some(StepObservation::Unreadable);
        assert_eq!(state.step_records.malformed, unreadable, "{strip}");
    }
}

#[test]
fn a_new_attempt_starts_with_no_step_records() {
    let mut state = flying();
    let mut partial = step("a1", A, true);
    partial.payload["checkpoint"]["result"]["covered"] = json!(null);
    apply(&mut state, &partial).unwrap();
    assert!(state.step_records.malformed.is_some());
    let failed = json!({"effect_id": "e1", "attempt_id": "a1", "error": "lost"});
    apply(&mut state, &event(EventType::EffectFailed, failed)).unwrap();
    let started = json!({"effect_id": "e1", "attempt_id": "a2"});
    apply(&mut state, &event(EventType::EffectStarted, started)).unwrap();
    assert_eq!(state.step_records, StepRecords::default());
}
