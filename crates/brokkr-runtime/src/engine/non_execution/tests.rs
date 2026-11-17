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

/// `event` moved to the effect `effect_id`, its attempt kept.
fn of_effect(mut event: EventEnvelope, effect_id: &str) -> EventEnvelope {
    event.payload["effect_id"] = json!(effect_id);
    event
}

/// A start failure reads `FailedToStart` unless a checkpoint of that very
/// attempt, of this effect, landed before its terminal: each conjunct of
/// the scan alone keeps a checkpoint from proving the attempt ran.
#[test]
fn only_this_attempts_checkpoint_before_its_terminal_proves_it_ran() {
    let read = |events: &[EventEnvelope]| last_attempt(events, "effect-1");
    let start = || event(1, EventType::EffectStarted, "a1", json!({}));
    let failed = |seq| event(seq, EventType::EffectFailed, "a1", start_failure());
    let checkpoint = |seq, attempt| event(seq, EventType::EffectCheckpointed, attempt, json!({}));
    // The control: this attempt's checkpoint, before its terminal, ran.
    let ran = [start(), checkpoint(2, "a1"), failed(3)];
    assert_eq!(read(&ran), Some(LastAttempt::Failed));
    // Another attempt's checkpoint proves nothing of this one.
    let other_attempt = [start(), checkpoint(2, "a0"), failed(3)];
    assert_eq!(read(&other_attempt), Some(LastAttempt::FailedToStart));
    // Another effect's checkpoint proves nothing of this one.
    let other_effect = [
        start(),
        of_effect(checkpoint(2, "a1"), "effect-2"),
        failed(3),
    ];
    assert_eq!(read(&other_effect), Some(LastAttempt::FailedToStart));
    // A checkpoint after the terminal is not the attempt having run.
    let late = [start(), failed(2), checkpoint(3, "a1")];
    assert_eq!(read(&late), Some(LastAttempt::FailedToStart));
}

/// An attempt is settled only by a terminal event (not a checkpoint) after
/// its start, of this effect and of that attempt: each conjunct alone
/// leaves a started attempt unsettled.
#[test]
fn only_this_attempts_own_terminal_after_its_start_settles_it() {
    let settled = |events: &[EventEnvelope]| every_attempt_settled(events, "effect-1");
    let start = |seq| event(seq, EventType::EffectStarted, "a1", json!({}));
    let ended = |seq, attempt| event(seq, EventType::EffectFailed, attempt, json!({}));
    // The control: the attempt's own terminal, after its start, settles it,
    // and so does each terminal kind.
    for kind in [
        EventType::EffectSucceeded,
        EventType::EffectFailed,
        EventType::EffectIndeterminate,
    ] {
        let own = [start(1), event(2, kind, "a1", json!({}))];
        assert!(settled(&own), "{kind:?}");
    }
    // A checkpoint is no terminal.
    let checkpointed = [
        start(1),
        event(2, EventType::EffectCheckpointed, "a1", json!({})),
    ];
    assert!(!settled(&checkpointed));
    // A terminal before the start ended an earlier opening, not this one.
    let before = [ended(1, "a1"), start(2)];
    assert!(!settled(&before));
    // Another effect's terminal of the same attempt id does not settle it.
    let elsewhere = [start(1), of_effect(ended(2, "a1"), "effect-2")];
    assert!(!settled(&elsewhere));
    // Another attempt's terminal does not settle it.
    let other_attempt = [start(1), ended(2, "a0")];
    assert!(!settled(&other_attempt));
}
