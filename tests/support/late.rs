//! The two shapes #440 left after a run's terminal event, and the lawful
//! annotation beside them, each built once for the tests that plant or
//! fold them (decision 0108 ruling 4).
//!
//! Each test binary includes this file once through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use brokkr_core::envelope::EventType;
use serde_json::{json, Value};

/// Shape (A): a driver's settlement planted after `run/stopped`.
pub(crate) fn settlement() -> (EventType, Value) {
    (
        EventType::EffectSucceeded,
        json!({"effect_id": "effect", "attempt_id": "attempt", "result": {"result": "complete"}}),
    )
}

/// Shape (B), as 0.12.0 wrote it: a stop command and the acceptance that
/// disposes of it planted after the conclusion.
pub(crate) fn late_stop() -> Vec<(EventType, Value)> {
    vec![
        (
            EventType::OperatorCommanded,
            json!({"command_id": "c-stop", "command": "stop", "operator": "operator", "args": {}}),
        ),
        (
            EventType::OperatorAccepted,
            json!({"command_id": "c-stop", "operator": "operator", "reason": "stop it"}),
        ),
    ]
}
