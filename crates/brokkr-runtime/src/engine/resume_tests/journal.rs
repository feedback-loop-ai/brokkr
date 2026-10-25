//! What a failing offers assertion prints of the run beside the shim's
//! log (#835). A site whose log holds fewer `start`s than it was due was
//! never handed one: its spawn was refused, or its attempt ended before
//! the handshake. The log cannot say which; the journal can, so the
//! assertion carries it.

use brokkr_core::envelope::{EventEnvelope, EventType};

/// One line per event that says how an effect attempt went: its start,
/// its end with the error or reason it ended on, every panel member's
/// finishing marker, and a park.
pub(super) fn effects(events: &[EventEnvelope]) -> String {
    events
        .iter()
        .filter(|event| tells(event))
        .map(|event| format!("{} {}", event.event_type.as_str(), event.payload))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Is `event` one [`effects`] prints?
fn tells(event: &EventEnvelope) -> bool {
    let marker = event.payload["checkpoint"]["step"] == "panel-member-finished";
    matches!(
        event.event_type,
        EventType::EffectStarted
            | EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate
            | EventType::RunParked
    ) || (event.event_type == EventType::EffectCheckpointed && marker)
}
