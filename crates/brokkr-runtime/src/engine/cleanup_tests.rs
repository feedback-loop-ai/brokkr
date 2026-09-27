//! #403: an attempt whose process tree is not proven over parks, whatever
//! its driver reached, and its terminal event carries the outcome it
//! received, typed, beside the unresolved cleanup. A settled attempt's
//! event carries nothing more.

use super::tests::{engine, report, single_body};
use super::*;
use brokkr_protocol::process::Unsettled;

fn succeeded(result: Value) -> AttemptReport {
    report(AttemptOutcome::Succeeded { result }, "")
}

fn unresolved(result: Value, reason: Unsettled) -> AttemptReport {
    AttemptReport {
        cleanup: Cleanup::Unresolved { reason },
        ..succeeded(result)
    }
}

/// A single seat that reported success is journaled indeterminate — it
/// parks, never settles or retries — with the result it received.
#[test]
fn a_seat_not_proven_over_parks_with_its_received_result() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let result = json!({"result": "complete", "notes": "done"});
    let descendants = Unsettled::Descendants { pids: vec![41, 42] };
    engine
        .conclude_single(
            "effect",
            "attempt",
            DriverRun::Ran(unresolved(result.clone(), descendants)),
            &Selection::new(),
            None,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let last = events.last().unwrap();
    assert_eq!(last.event_type, EventType::EffectIndeterminate);
    assert_eq!(
        last.payload,
        json!({
            "effect_id": "effect",
            "attempt_id": "attempt",
            "reason": "the driver reported success; the attempt is not proven over: \
                       its descendants 41, 42 were still running after the kill; stderr tail: ",
            "received": {"status": "succeeded", "result": result},
            "cleanup": {
                "state": "unresolved",
                "reason": "its descendants 41, 42 were still running after the kill",
            },
        })
    );
}

#[test]
fn a_dialect_step_not_proven_over_parks_with_its_evidence() {
    let mut evidence = Map::new();
    dialect_attempt_outcome(
        DriverRun::Ran(succeeded(json!({"result": "pass"}))),
        &mut evidence,
    );
    assert_eq!(evidence, Map::new());
    let held = unresolved(json!({"result": "pass"}), Unsettled::Stderr);
    assert!(matches!(
        dialect_attempt_outcome(DriverRun::Ran(held), &mut evidence),
        AttemptOutcome::Indeterminate { reason } if reason == "the driver reported success; the \
            attempt is not proven over: a process outside its tree still held the driver's stderr"
    ));
    assert_eq!(
        Value::Object(evidence),
        json!({
            "received": {"status": "succeeded", "result": {"result": "pass"}},
            "cleanup": {
                "state": "unresolved",
                "reason": "a process outside its tree still held the driver's stderr",
            },
        })
    );
}

/// One member not proven over parks the whole panel, and the attempt's
/// event names that member.
#[test]
fn a_panel_member_not_proven_over_parks_the_panel_with_its_evidence() {
    let result = || json!({"result": "pass", "notes": "ok"});
    let settled = vec![("ok".to_string(), succeeded(result()))];
    assert_eq!(panel_evidence(&settled), Map::new());
    let reports = vec![
        ("ok".to_string(), succeeded(result())),
        (
            "held".to_string(),
            unresolved(result(), Unsettled::Group { group: 7 }),
        ),
    ];
    assert!(matches!(
        panel_outcome(Aggregate::UnanimousPass, reports.clone()),
        AttemptOutcome::Indeterminate { reason }
            if reason == "panel members [\"held\"] could not establish completion"
    ));
    assert_eq!(
        Value::Object(panel_evidence(&reports)),
        json!({"unresolved_members": {"held": {
            "received": {"status": "succeeded", "result": result()},
            "cleanup": {
                "state": "unresolved",
                "reason": "its process group 7 still had members after the kill",
            },
        }}})
    );
}
