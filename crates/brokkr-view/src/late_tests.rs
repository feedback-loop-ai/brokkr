//! A repaired run's late tail in the view's own scans (decision 0108
//! ruling 4, F7): a late supersede is kept and shown, never applied.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::fold;
use serde_json::json;

use crate::tests::ev;
use crate::{residual_findings, residual_findings_at};

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
    events.push(ev(
        11,
        EventType::OperatorCommanded,
        json!({"command_id": "c-supersede", "command": "supersede", "operator": "operator",
               "args": {"findings": [8],
                        "by": {"realm": null, "run_id": "later", "seq": 21},
                        "reason": "late"}}),
        T1,
    ));
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
