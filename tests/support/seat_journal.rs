//! One seat's journal, as the seat-costs and resolution tests read it
//! (#351): the view's `seat_costs` tests and `brokkr compare`'s resolution
//! test share it rather than each restating it. A test module includes
//! this file through `#[path]`; it spells its envelopes through the
//! including crate's `crate::tests::envelope_builder`, the one place a
//! test spells one (#357).

use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::{json, Value};

use crate::tests::envelope_builder::EnvelopeBuilder;

pub(crate) fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload)
        .at("2026-08-28T00:00:00Z")
        .previous("")
        .build()
}

/// One seat's journal under `word`: the entry on the start, the stamp
/// beside every record that names a model (design DD19).
pub(crate) fn boxed_seat(seat: &str, word: &str) -> Vec<EventEnvelope> {
    vec![
        event(
            EventType::EffectRequested,
            json!({"effect_id": seat, "seat": seat}),
        ),
        event(
            EventType::EffectStarted,
            json!({"effect_id": seat,
                   "boundary": [{"member": null, "boundary": word, "gate": true}]}),
        ),
        event(
            EventType::EffectCheckpointed,
            json!({"effect_id": seat, "checkpoint": {
                "step": "seat-turn", "turn": 1, "model": "claude-fable-5-1",
                "boundary": word}}),
        ),
        event(
            EventType::EffectSucceeded,
            json!({"effect_id": seat, "result": {
                "result": "pass", "model": "claude-fable-5-1", "boundary": word}}),
        ),
    ]
}
