//! The v1 event envelope (contracts/event-envelope.v1.schema.json).
//!
//! `event_hash` = SHA-256 hex over the canonical bytes of the envelope
//! with the `event_hash` member removed. `previous_hash` chains to the
//! prior event; seq 1 chains to the zero hash. `recorded_at` is evidence
//! only: nothing in fold or evaluate reads it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::canonical::{self, ZERO_HASH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "run/started")]
    RunStarted,
    #[serde(rename = "phase/entered")]
    PhaseEntered,
    #[serde(rename = "effect/requested")]
    EffectRequested,
    #[serde(rename = "effect/started")]
    EffectStarted,
    #[serde(rename = "effect/checkpointed")]
    EffectCheckpointed,
    #[serde(rename = "effect/succeeded")]
    EffectSucceeded,
    #[serde(rename = "effect/failed")]
    EffectFailed,
    #[serde(rename = "effect/indeterminate")]
    EffectIndeterminate,
    #[serde(rename = "transition/decided")]
    TransitionDecided,
    #[serde(rename = "operator/commanded")]
    OperatorCommanded,
    #[serde(rename = "operator/accepted")]
    OperatorAccepted,
    #[serde(rename = "operator/rejected")]
    OperatorRejected,
    #[serde(rename = "run/parked")]
    RunParked,
    #[serde(rename = "run/completed")]
    RunCompleted,
    #[serde(rename = "run/stopped")]
    RunStopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub run_id: String,
    pub seq: u64,
    pub event_id: String,
    pub event_schema_version: u32,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub payload: Value,
    pub causation_id: Option<String>,
    pub correlation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    pub recorded_at: String,
    pub previous_hash: String,
    pub event_hash: String,
}

#[derive(Debug, Error, PartialEq)]
pub enum ChainError {
    #[error("event {seq}: seq is not contiguous (expected {expected})")]
    SeqGap { seq: u64, expected: u64 },
    #[error("event {seq}: previous_hash does not match event {prev_seq}")]
    BrokenChain { seq: u64, prev_seq: u64 },
    #[error("event {seq}: event_hash does not match canonical content")]
    BadHash { seq: u64 },
    #[error("event {seq}: event_schema_version {found} is not supported (want 1)")]
    BadSchemaVersion { seq: u64, found: u32 },
    #[error("event {seq}: run_id differs from the journal's run")]
    ForeignRun { seq: u64 },
}

impl EventEnvelope {
    /// Compute the event hash for this envelope's content (ignoring any
    /// current `event_hash` value).
    pub fn compute_hash(&self) -> String {
        let mut value = serde_json::to_value(self).expect("envelope serializes");
        value
            .as_object_mut()
            .expect("envelope is an object")
            .remove("event_hash");
        canonical::sha256_hex(&value)
    }

    /// Seal the envelope: set `event_hash` from its canonical content.
    pub fn sealed(mut self) -> Self {
        self.event_hash = self.compute_hash();
        self
    }
}

/// Verify sequence continuity, hash chain, per-event hashes, schema
/// version, and run identity. Fails closed on the first defect.
pub fn verify_chain(events: &[EventEnvelope]) -> Result<(), ChainError> {
    let mut prev_hash = ZERO_HASH.to_string();
    let run_id = events.first().map(|e| e.run_id.clone());
    for (i, event) in events.iter().enumerate() {
        let expected_seq = (i + 1) as u64;
        if event.seq != expected_seq {
            return Err(ChainError::SeqGap {
                seq: event.seq,
                expected: expected_seq,
            });
        }
        if event.event_schema_version != 1 {
            return Err(ChainError::BadSchemaVersion {
                seq: event.seq,
                found: event.event_schema_version,
            });
        }
        if Some(&event.run_id) != run_id.as_ref() {
            return Err(ChainError::ForeignRun { seq: event.seq });
        }
        if event.previous_hash != prev_hash {
            return Err(ChainError::BrokenChain {
                seq: event.seq,
                prev_seq: event.seq.saturating_sub(1),
            });
        }
        if event.compute_hash() != event.event_hash {
            return Err(ChainError::BadHash { seq: event.seq });
        }
        prev_hash = event.event_hash.clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn envelope(seq: u64, prev: &str) -> EventEnvelope {
        EventEnvelope {
            run_id: "r1".into(),
            seq,
            event_id: format!("e{seq}"),
            event_schema_version: 1,
            event_type: EventType::PhaseEntered,
            payload: json!({"phase": "intake"}),
            causation_id: None,
            correlation_id: "r1".into(),
            attempt_id: None,
            recorded_at: "2026-08-23T00:00:00Z".into(),
            previous_hash: prev.into(),
            event_hash: String::new(),
        }
        .sealed()
    }

    #[test]
    fn chain_verifies_and_detects_tamper() {
        let e1 = envelope(1, ZERO_HASH);
        let e2 = envelope(2, &e1.event_hash);
        verify_chain(&[e1.clone(), e2.clone()]).unwrap();

        let mut tampered = e1.clone();
        tampered.payload = json!({"phase": "ship"});
        assert_eq!(
            verify_chain(&[tampered, e2.clone()]),
            Err(ChainError::BadHash { seq: 1 })
        );

        let e2_orphan = envelope(2, ZERO_HASH);
        assert_eq!(
            verify_chain(&[e1, e2_orphan]),
            Err(ChainError::BrokenChain { seq: 2, prev_seq: 1 })
        );
    }
}
