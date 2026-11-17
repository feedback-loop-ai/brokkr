//! The predicate's pure conjuncts, over envelopes built directly: the ones
//! the contention tests cannot plant through a store the fold would refuse.

use serde_json::json;

use super::*;
use crate::envelope_builder::EnvelopeBuilder;

fn event(seq: u64, event_type: EventType, attempt: &str, extra: Value) -> EventEnvelope {
    let mut payload = json!({"effect_id": "effect-1", "attempt_id": attempt});
    for (field, value) in extra.as_object().into_iter().flatten() {
        payload[field] = value.clone();
    }
    EnvelopeBuilder::new(event_type, payload)
        .seq(seq)
        .attempt(Some(attempt))
        .build()
}

fn start_failure() -> Value {
    json!({"start_failure": true, "start_failure_sites": [null]})
}

const UNDER: Standing = Standing {
    failed_attempts: 1,
    max_attempts: 2,
    reservation: false,
};

#[test]
fn an_attempt_that_never_settled_is_not_proof_of_non_execution() {
    let settled = [
        event(1, EventType::EffectStarted, "a1", json!({})),
        event(2, EventType::EffectFailed, "a1", start_failure()),
    ];
    assert!(never_executed(&settled, "effect-1", UNDER));

    let unsettled = [
        event(1, EventType::EffectStarted, "a0", json!({})),
        event(2, EventType::EffectStarted, "a1", json!({})),
        event(3, EventType::EffectFailed, "a1", start_failure()),
    ];
    assert!(!never_executed(&unsettled, "effect-1", UNDER));
}

#[test]
fn a_terminal_that_names_no_attempt_proves_nothing() {
    let mut nameless = event(2, EventType::EffectFailed, "a1", start_failure());
    nameless.attempt_id = None;
    nameless
        .payload
        .as_object_mut()
        .unwrap()
        .remove("attempt_id");
    let events = [
        event(1, EventType::EffectStarted, "a1", json!({})),
        nameless,
    ];
    assert_eq!(last_attempt(&events, "effect-1"), None);
}

#[test]
fn the_last_terminal_names_what_the_last_attempt_was() {
    let kinds = |terminal: EventType, extra: Value| {
        let events = [
            event(1, EventType::EffectStarted, "a1", json!({})),
            event(2, terminal, "a1", extra),
        ];
        last_attempt(&events, "effect-1")
    };
    assert_eq!(
        kinds(EventType::EffectIndeterminate, json!({})),
        Some(LastAttempt::Indeterminate)
    );
    assert_eq!(
        kinds(EventType::EffectFailed, json!({"waits": []})),
        Some(LastAttempt::Refused)
    );
    assert_eq!(
        kinds(EventType::EffectFailed, start_failure()),
        Some(LastAttempt::FailedToStart)
    );
    assert_eq!(
        kinds(EventType::EffectFailed, json!({})),
        Some(LastAttempt::Failed)
    );
}
