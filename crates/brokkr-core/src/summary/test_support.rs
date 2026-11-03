//! The summary projections' tests share one journal builder, so a step's
//! stamp and seq are built one way (decision 0071 ruling 9).

use crate::envelope::{EventEnvelope, EventType};
use crate::envelope_builder::EnvelopeBuilder;

/// The run's events at seq 1 onward, each stamped as its step names,
/// sealed from their content.
pub(crate) fn journal(steps: &[(EventType, serde_json::Value, &str)]) -> Vec<EventEnvelope> {
    steps
        .iter()
        .enumerate()
        .map(|(index, (event_type, payload, at))| {
            let seq = index as u64 + 1;
            EnvelopeBuilder::new(*event_type, payload.clone())
                .seq(seq)
                .event_id(format!("e{seq}"))
                .at(*at)
                .sealed()
        })
        .collect()
}
