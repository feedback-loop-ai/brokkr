//! Fold behavior: protocol-shape enforcement, counters, replay
//! determinism, and the crash-recovery cursor positions.

use forge_core::envelope::{verify_chain, EventEnvelope, EventType};
use forge_core::fold::{computed_inputs, fold, Cursor, FoldError, Status};
use forge_core::canonical::ZERO_HASH;
use serde_json::{json, Value};

struct Journal {
    events: Vec<EventEnvelope>,
}

impl Journal {
    fn new() -> Self {
        let mut journal = Journal { events: Vec::new() };
        journal.append(
            EventType::RunStarted,
            json!({"feature": "t", "manifest": {}}),
        );
        journal
    }

    fn append(&mut self, event_type: EventType, payload: Value) -> &EventEnvelope {
        let seq = self.events.len() as u64 + 1;
        let previous_hash = self
            .events
            .last()
            .map(|e| e.event_hash.clone())
            .unwrap_or_else(|| ZERO_HASH.to_string());
        let envelope = EventEnvelope {
            run_id: "r1".into(),
            seq,
            event_id: format!("e{seq}"),
            event_schema_version: 1,
            event_type,
            payload,
            causation_id: None,
            correlation_id: "r1".into(),
            attempt_id: None,
            recorded_at: "2026-08-23T00:00:00Z".into(),
            previous_hash,
            event_hash: String::new(),
        }
        .sealed();
        self.events.push(envelope);
        self.events.last().unwrap()
    }
}

fn effect_cycle(journal: &mut Journal, phase: &str, result_payload: Value) {
    journal.append(EventType::PhaseEntered, json!({"phase": phase}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": format!("fx-{phase}"), "phase": phase, "seat": phase,
               "idempotency_key": format!("k-{phase}"), "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectStarted,
        json!({"effect_id": format!("fx-{phase}"), "attempt_id": "a1", "driver": "fake"}),
    );
    journal.append(
        EventType::EffectSucceeded,
        json!({"effect_id": format!("fx-{phase}"), "attempt_id": "a1", "result": result_payload}),
    );
}

#[test]
fn happy_path_reaches_decide_cursor() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "implement", json!({"result": "complete"}));
    verify_chain(&journal.events).unwrap();
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.phase.as_deref(), Some("implement"));
    assert!(matches!(state.cursor, Cursor::Decide { .. }));
}

#[test]
fn consecutive_failures_count_and_reset() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "implement", json!({"result": "broken"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "implement", "result": "broken", "rule_id": "IMPL-BROKEN-RETRY",
               "next": "implement", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.consecutive_failures.get("implement"), Some(&1));
    // The engine-side input counts the failure being decided.
    let inputs = computed_inputs(&state, "implement", "broken");
    assert_eq!(inputs.get("consecutive_failures"), Some(&Value::from(2)));

    effect_cycle(&mut journal, "implement", json!({"result": "complete"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "implement", "result": "complete", "rule_id": "IMPL-OK",
               "next": "verify", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.consecutive_failures.get("implement"), Some(&0));
}

#[test]
fn failed_effect_demands_park_and_operator_retry_reopens() {
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "implement"}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx", "phase": "implement", "seat": "implement",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a1", "driver": "fake"}),
    );
    journal.append(
        EventType::EffectFailed,
        json!({"effect_id": "fx", "attempt_id": "a1", "error": "driver crashed"}),
    );
    let state = fold(&journal.events).unwrap();
    // A determinate failure returns to the executable position with the
    // attempt counted; the engine decides retry-or-park (decision 0006).
    assert!(matches!(
        state.cursor,
        Cursor::ExecuteEffect { failed_attempts: 1, .. }
    ));

    journal.append(EventType::RunParked, json!({"reason": "effect fx failed", "evidence": {}}));
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.status, Status::AwaitingOperator);
    assert_eq!(state.cursor, Cursor::Idle);

    journal.append(
        EventType::OperatorCommanded,
        json!({"command_id": "c1", "command": "retry", "args": {}, "operator": "valentin"}),
    );
    journal.append(
        EventType::OperatorAccepted,
        json!({"command_id": "c1", "operator": "valentin", "reason": "fixed env"}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.cursor, Cursor::RequestEffect);
}

#[test]
fn requested_but_unstarted_effect_can_close_indeterminate() {
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "implement"}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx", "phase": "implement", "seat": "implement",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectIndeterminate,
        json!({"effect_id": "fx", "reason": "process died before start was durable"}),
    );
    let state = fold(&journal.events).unwrap();
    assert!(matches!(state.cursor, Cursor::Park { .. }));
}

#[test]
fn protocol_violations_fail_closed() {
    // Second effect while one is open.
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "implement"}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx1", "phase": "implement", "seat": "implement",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx2", "phase": "implement", "seat": "implement",
               "idempotency_key": "k2", "input_digest": "d"}),
    );
    assert!(matches!(
        fold(&journal.events),
        Err(FoldError::OutOfPlace { seq: 4, .. })
    ));

    // Event after terminal status.
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "done"}));
    journal.append(EventType::RunCompleted, json!({}));
    journal.append(EventType::PhaseEntered, json!({"phase": "intake"}));
    assert!(matches!(
        fold(&journal.events),
        Err(FoldError::AfterTerminal { seq: 4 })
    ));
}

#[test]
fn replay_is_deterministic() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "implement", json!({"result": "complete"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "implement", "result": "complete", "rule_id": "IMPL-OK",
               "next": "verify", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let a = format!("{:?}", fold(&journal.events).unwrap());
    let b = format!("{:?}", fold(&journal.events).unwrap());
    assert_eq!(a, b);
}
