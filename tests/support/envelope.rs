//! The one place a test spells an `EventEnvelope` (#357). A field added
//! to the envelope is added here and in the tests that assert it, and
//! nowhere else. Each test binary includes this file once through
//! `#[path]`; it is test code by its path, so it stays outside the
//! coverage denominator.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::Value;

/// A hand-built event: the first of run `run`, chained to the zero hash
/// and unsealed. A test names only the fields its assertion reads.
pub struct EnvelopeBuilder {
    envelope: EventEnvelope,
}

impl EnvelopeBuilder {
    pub fn new(event_type: EventType, payload: Value) -> Self {
        Self {
            envelope: EventEnvelope {
                run_id: "run".into(),
                seq: 1,
                event_id: "event".into(),
                event_schema_version: 1,
                event_type,
                payload,
                causation_id: None,
                correlation_id: "run".into(),
                attempt_id: None,
                recorded_at: "2026-01-01T00:00:00Z".into(),
                previous_hash: ZERO_HASH.into(),
                event_hash: String::new(),
            },
        }
    }

    /// The run the event belongs to; its correlation follows it.
    pub fn run(mut self, run_id: &str) -> Self {
        self.envelope.run_id = run_id.into();
        self.envelope.correlation_id = run_id.into();
        self
    }

    pub fn correlation(mut self, correlation_id: &str) -> Self {
        self.envelope.correlation_id = correlation_id.into();
        self
    }

    pub fn seq(mut self, seq: u64) -> Self {
        self.envelope.seq = seq;
        self
    }

    pub fn event_id(mut self, event_id: impl Into<String>) -> Self {
        self.envelope.event_id = event_id.into();
        self
    }

    pub fn caused_by(mut self, causation_id: &str) -> Self {
        self.envelope.causation_id = Some(causation_id.into());
        self
    }

    pub fn attempt(mut self, attempt_id: Option<&str>) -> Self {
        self.envelope.attempt_id = attempt_id.map(str::to_string);
        self
    }

    pub fn at(mut self, recorded_at: impl Into<String>) -> Self {
        self.envelope.recorded_at = recorded_at.into();
        self
    }

    pub fn previous(mut self, previous_hash: impl Into<String>) -> Self {
        self.envelope.previous_hash = previous_hash.into();
        self
    }

    /// An event hash the test states instead of one sealed from content.
    pub fn hash(mut self, event_hash: impl Into<String>) -> Self {
        self.envelope.event_hash = event_hash.into();
        self
    }

    pub fn build(self) -> EventEnvelope {
        self.envelope
    }

    /// The event with its hash computed from its content.
    pub fn sealed(self) -> EventEnvelope {
        self.envelope.sealed()
    }
}

/// The builder's defaults and every setter, pinned once.
#[test]
fn the_builder_states_every_envelope_field_once() {
    let plain = EnvelopeBuilder::new(EventType::RunStarted, serde_json::json!({})).build();
    assert_eq!(
        serde_json::to_value(&plain).unwrap(),
        serde_json::json!({
            "run_id": "run", "seq": 1, "event_id": "event", "event_schema_version": 1,
            "type": "run/started", "payload": {}, "causation_id": null,
            "correlation_id": "run", "recorded_at": "2026-01-01T00:00:00Z",
            "previous_hash": ZERO_HASH, "event_hash": "",
        })
    );
    let named = EnvelopeBuilder::new(EventType::PhaseEntered, serde_json::json!({"phase": "a"}))
        .run("r1")
        .correlation("corr")
        .seq(2)
        .event_id("e2")
        .caused_by("e1")
        .attempt(Some("attempt"))
        .at("2026-09-26T00:00:00Z")
        .previous("p")
        .hash("h")
        .build();
    assert_eq!(
        serde_json::to_value(&named).unwrap(),
        serde_json::json!({
            "run_id": "r1", "seq": 2, "event_id": "e2", "event_schema_version": 1,
            "type": "phase/entered", "payload": {"phase": "a"}, "causation_id": "e1",
            "correlation_id": "corr", "attempt_id": "attempt",
            "recorded_at": "2026-09-26T00:00:00Z", "previous_hash": "p", "event_hash": "h",
        })
    );
    let sealed = EnvelopeBuilder::new(EventType::RunStarted, serde_json::json!({}))
        .run("r2")
        .sealed();
    assert_eq!(
        (sealed.run_id.as_str(), sealed.correlation_id.as_str()),
        ("r2", "r2")
    );
    assert_eq!(sealed.event_hash, sealed.compute_hash());
    assert_eq!(sealed.event_hash.len(), 64);
}
