//! The small journal builders the seat journals and the heap tests share
//! (#753; decision 0089 ruling 4): one run's events at their seqs, a
//! checkpoint's payload, and the stamp at a seq. A test module includes
//! this file through `#[path]` beside an `envelope_builder` module, the
//! one place a test spells an envelope (#357). The seat journals build on
//! these; a test that reads only a working run includes this file alone,
//! rather than every hand-built journal beside it.

use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::{json, Value};

use super::envelope_builder::EnvelopeBuilder;

/// The stamp of the event at `seq`: one second apart.
pub(crate) fn at(seq: u64) -> String {
    format!("2026-01-01T00:00:{seq:02}Z")
}

/// A journal of `events`, each at its seq and stamp from 1.
pub(crate) fn journal(events: Vec<(EventType, Value)>) -> Vec<EventEnvelope> {
    let seqs = 1u64..;
    let events = seqs.zip(events);
    events
        .map(|(seq, (event_type, payload))| {
            EnvelopeBuilder::new(event_type, payload)
                .seq(seq)
                .at(at(seq))
                .build()
        })
        .collect()
}

/// A checkpoint of effect `effect`'s attempt `attempt`.
pub(crate) fn checkpoint(effect: &str, attempt: &str, checkpoint: Value) -> (EventType, Value) {
    let payload = json!({"effect_id": effect, "attempt_id": attempt, "checkpoint": checkpoint});
    (EventType::EffectCheckpointed, payload)
}
