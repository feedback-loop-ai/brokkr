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

/// A checkpoint of attempt `a`, naming `member`, its record `record`, at
/// the second second of the shared journal: what the checkpoint and site
/// projections' tests read.
pub(crate) fn checkpoint(
    member: Option<&str>,
    record: serde_json::Value,
) -> (EventType, serde_json::Value, &'static str) {
    let mut checkpoint = record;
    if let (Some(member), Some(object)) = (member, checkpoint.as_object_mut()) {
        object.insert("member".into(), serde_json::json!(member));
    }
    (
        EventType::EffectCheckpointed,
        serde_json::json!({"attempt_id": "a", "checkpoint": checkpoint}),
        "2026-01-01T00:00:02Z",
    )
}
