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

/// The terminal event a single seat's `report` concludes in, which is
/// indeterminate: it parks.
fn parked(report: AttemptReport) -> Value {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine
        .conclude_single(
            "effect",
            "attempt",
            DriverRun::Ran(report),
            &Selection::new(),
            None,
        )
        .unwrap();
    let mut events = engine.store.load(&engine.run_id).unwrap();
    let last = events.pop().unwrap();
    assert_eq!(last.event_type, EventType::EffectIndeterminate);
    last.payload
}

/// A single seat that reported success is journaled indeterminate — it
/// parks, never settles or retries — with the result it received.
#[test]
fn a_seat_not_proven_over_parks_with_its_received_result() {
    let result = json!({"result": "complete", "notes": "done"});
    let descendants = Unsettled::Descendants { pids: vec![41, 42] };
    assert_eq!(
        parked(unresolved(result.clone(), descendants)),
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
    let mut evidence = Unproven::Proven;
    dialect_attempt_outcome(
        DriverRun::Ran(succeeded(json!({"result": "pass"}))),
        &mut evidence,
    );
    assert!(matches!(evidence, Unproven::Proven));
    let held = unresolved(json!({"result": "pass"}), Unsettled::Stderr);
    assert!(matches!(
        dialect_attempt_outcome(DriverRun::Ran(held), &mut evidence),
        AttemptOutcome::Indeterminate { reason } if reason == "the driver reported success; the \
            attempt is not proven over: a process outside its tree still held the driver's stderr"
    ));
    assert_eq!(
        json!(evidence),
        json!({
            "received": {"status": "succeeded", "result": {"result": "pass"}},
            "cleanup": {
                "state": "unresolved",
                "reason": "a process outside its tree still held the driver's stderr",
            },
        })
    );
}

/// A checkpoint the journal refused, on an attempt not proven over: the
/// refusal replaces the outcome acted on, which still parks and names the
/// refusal, while the evidence keeps the result the driver sent as
/// `received` (#403 finding 4).
#[test]
fn a_refused_checkpoint_keeps_the_received_result_beside_an_unresolved_cleanup() {
    let result = json!({"result": "complete"});
    let refusal = brokkr_store::SeatRecordError {
        seq: 9,
        path: "/".into(),
        contract: "contracts/seat-record.v1.schema.json",
    };
    let received = succeeded(result.clone());
    let refused = AttemptReport {
        refused: Some(checkpoints::refused_outcome(
            received.outcome.clone(),
            &refusal,
        )),
        ..unresolved(result.clone(), Unsettled::Group { group: 7 })
    };
    let group = "its process group 7 still had members after the kill";
    assert_eq!(
        parked(refused),
        json!({
            "effect_id": "effect",
            "attempt_id": "attempt",
            "reason": format!(
                "{refusal}; the attempt is not proven over: {group}; stderr tail: "
            ),
            "received": {"status": "succeeded", "result": result},
            "cleanup": {"state": "unresolved", "reason": group},
        })
    );
}

/// A panel member's own marker names the outcome the panel acts on: an
/// unresolved member's is indeterminate, whatever it received (#403
/// finding 5).
#[test]
fn a_member_marker_names_the_settled_outcome() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let reports = vec![(
        "held".to_string(),
        unresolved(json!({"result": "pass"}), Unsettled::Stdout),
    )];
    engine
        .journal_panel_members("effect", "attempt", &reports, &[], "")
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let marker = &events.last().unwrap().payload["checkpoint"];
    assert_eq!(
        (&marker["member"], &marker["outcome"]),
        (&json!("held"), &json!("indeterminate"))
    );
}

/// One member not proven over parks the whole panel, and the attempt's
/// event names that member.
#[test]
fn a_panel_member_not_proven_over_parks_the_panel_with_its_evidence() {
    let result = || json!({"result": "pass", "notes": "ok"});
    let settled = vec![("ok".to_string(), succeeded(result()))];
    assert!(matches!(Unproven::panel(&settled), Unproven::Proven));
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
        json!(Unproven::panel(&reports)),
        json!({"unresolved_members": {"held": {
            "received": {"status": "succeeded", "result": result()},
            "cleanup": {
                "state": "unresolved",
                "reason": "its process group 7 still had members after the kill",
            },
        }}})
    );
}

/// Every field an attempt not proven over adds to `effect/indeterminate`
/// is published in `effect-cleanup.v1` and validates against it, and one
/// proven over adds none (decision 0016's additive rule).
#[test]
fn the_cleanup_fields_are_the_published_extension() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../contracts/effect-cleanup.v1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let v1 = ["attempt_id", "effect_id", "reason"];
    let lost = || AttemptOutcome::Indeterminate {
        reason: "lost".into(),
    };
    let proven = parked(report(lost(), ""));
    assert_eq!(
        proven.as_object().unwrap().keys().collect::<Vec<_>>(),
        v1.iter().collect::<Vec<_>>()
    );
    let refused_kill = AttemptReport {
        cleanup: Cleanup::Unresolved {
            reason: Unsettled::Reap { pid: 9 },
        },
        ..report(lost(), "")
    };
    let panel = json!(Indeterminate {
        effect_id: "effect",
        attempt_id: "attempt",
        reason: String::new(),
        unproven: Unproven::panel(&[
            ("ok".into(), succeeded(json!({}))),
            (
                "held".into(),
                unresolved(json!({"result": "pass"}), Unsettled::Stdout),
            ),
        ]),
    });
    let seat = parked(unresolved(json!({"result": "complete"}), Unsettled::Stderr));
    for payload in [seat.clone(), parked(refused_kill), panel] {
        assert!(validator.is_valid(&payload), "{payload}");
        for field in payload.as_object().unwrap().keys() {
            assert!(
                v1.contains(&field.as_str()) || schema["properties"].get(field).is_some(),
                "{field} is not published"
            );
        }
    }
    let mut halved = seat;
    halved.as_object_mut().unwrap().remove("cleanup");
    assert!(!validator.is_valid(&halved));
}
