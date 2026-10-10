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

/// Every type's wire name, in declaration order, so [`EventType::as_str`]
/// indexes it by discriminant. Its length is typed by the last variant, so
/// a missing or extra entry does not compile, and each entry is held to
/// serde's name by `every_event_type_names_itself_as_serde_does`.
const WIRE_NAMES: [&str; EventType::RunStopped as usize + 1] = [
    "run/started",
    "phase/entered",
    "effect/requested",
    "effect/started",
    "effect/checkpointed",
    "effect/succeeded",
    "effect/failed",
    "effect/indeterminate",
    "transition/decided",
    "operator/commanded",
    "operator/accepted",
    "operator/rejected",
    "run/parked",
    "run/completed",
    "run/stopped",
];

impl EventType {
    /// The wire name serde writes for this type, for readers that name an
    /// event without serializing it.
    pub const fn as_str(self) -> &'static str {
        WIRE_NAMES[self as usize]
    }
}

/// The one `event_schema_version` this envelope is, and the only one a
/// chain verifies.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

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
    #[error(
        "event {seq}: event_schema_version {found} is not supported (want {EVENT_SCHEMA_VERSION})"
    )]
    BadSchemaVersion { seq: u64, found: u32 },
    #[error("event {seq}: run_id differs from the journal's run")]
    ForeignRun { seq: u64 },
    #[error("event after {after}: seq has no successor")]
    SeqOverflow { after: u64 },
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
    let Some(first) = events.first() else {
        return Ok(());
    };
    verify_chain_after(&first.run_id, 0, ZERO_HASH, events)
}

/// [`verify_chain`] for what landed after a head the caller already
/// verified: `events` must continue `run_id`'s chain from seq `seq`, whose
/// hash is `hash`, with every check [`verify_chain`] makes. A whole
/// journal is the suffix of the empty head, `(0, ZERO_HASH)`. A seq with
/// no successor in `u64` refuses rather than wrapping.
pub fn verify_chain_after(
    run_id: &str,
    seq: u64,
    hash: &str,
    events: &[EventEnvelope],
) -> Result<(), ChainError> {
    let (mut prev_seq, mut prev_hash) = (seq, hash);
    for event in events {
        let expected_seq = prev_seq
            .checked_add(1)
            .ok_or(ChainError::SeqOverflow { after: prev_seq })?;
        if event.seq != expected_seq {
            return Err(ChainError::SeqGap {
                seq: event.seq,
                expected: expected_seq,
            });
        }
        if event.event_schema_version != EVENT_SCHEMA_VERSION {
            return Err(ChainError::BadSchemaVersion {
                seq: event.seq,
                found: event.event_schema_version,
            });
        }
        if event.run_id != run_id {
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
        (prev_seq, prev_hash) = (expected_seq, &event.event_hash);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
