//! Backend-agnostic [`Journal`] domain facade (decision 0126 ruling 2).
//!
//! Owns hash-chain continuity, optimistic `expected_head` fencing,
//! `seat_record` schema validation, canonical NDJSON export/import, and
//! projection reindexing once across every storage backend.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{
    verify_chain, verify_chain_after, EventEnvelope, EventType, EVENT_SCHEMA_VERSION,
};
use serde_json::Value;

use crate::{format_canonical_ndjson, import, seat_record, SeatRecordVersion, StoreError};

use super::{CoordinationStore, EventLog, ProjectionStore};

/// Backend-agnostic journal facade composing an [`EventLog`], a
/// [`ProjectionStore`], and a [`CoordinationStore`] (decision 0126 ruling 2).
pub struct Journal<E, P, C> {
    event_log: E,
    projections: P,
    coordination: C,
}

impl<E, P, C> Journal<E, P, C> {
    /// Compose a journal facade over concrete storage port implementations.
    pub fn new(event_log: E, projections: P, coordination: C) -> Self {
        Self {
            event_log,
            projections,
            coordination,
        }
    }

    /// Borrow the underlying event-log port.
    pub fn event_log(&self) -> &E {
        &self.event_log
    }

    /// Borrow the underlying projection-store port.
    pub fn projections(&self) -> &P {
        &self.projections
    }

    /// Borrow the underlying coordination-store port.
    pub fn coordination(&self) -> &C {
        &self.coordination
    }
}

impl<E: EventLog, P: ProjectionStore, C: CoordinationStore> Journal<E, P, C> {
    /// Admit a new run id and persist its manifest in the event log.
    pub fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        import::recordable(run_id)?;
        self.event_log.create_run(run_id, manifest)
    }

    /// Read the current head of `run_id`, seal a hash-chained event onto it,
    /// validate any seat record, append under the head fence, and update
    /// projections.
    pub fn append_event(
        &self,
        run_id: &str,
        event_type: EventType,
        payload: Value,
        causation_id: Option<&str>,
        ts: &str,
    ) -> Result<EventEnvelope, StoreError> {
        let (seq, hash) = self.event_log.head(run_id)?;
        self.append_if_head(run_id, (seq, &hash), event_type, payload, causation_id, ts)
    }

    /// Seal and append an event onto `run_id` only while its head matches
    /// `expected_head`, refusing a moved head with [`StoreError::HeadMoved`].
    pub fn append_if_head(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        event_type: EventType,
        payload: Value,
        causation_id: Option<&str>,
        ts: &str,
    ) -> Result<EventEnvelope, StoreError> {
        let (found_seq, found_hash) = self.event_log.head(run_id)?;
        if found_seq != expected_head.0 || found_hash != expected_head.1 {
            return Err(StoreError::HeadMoved {
                expected_seq: expected_head.0,
                found_seq,
            });
        }
        let seq = expected_head.0 + 1;
        self.validate_record(run_id, seq, event_type, &payload)?;
        let envelope = EventEnvelope {
            run_id: run_id.to_string(),
            seq,
            event_id: uuid::Uuid::new_v4().to_string(),
            event_schema_version: EVENT_SCHEMA_VERSION,
            event_type,
            payload,
            causation_id: causation_id.map(str::to_owned),
            correlation_id: run_id.to_string(),
            attempt_id: None,
            recorded_at: ts.to_string(),
            previous_hash: expected_head.1.to_string(),
            event_hash: String::new(),
        }
        .sealed();
        self.event_log
            .append_envelopes(run_id, expected_head, std::slice::from_ref(&envelope))?;
        self.projections
            .apply_envelopes(run_id, std::slice::from_ref(&envelope))?;
        Ok(envelope)
    }

    /// Load `run_id`'s full event stream from sequence 1 and verify hash-chain
    /// integrity.
    pub fn load_verified(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        let events = self.event_log.read_stream(run_id, 1)?;
        verify_chain(&events)?;
        Ok(events)
    }

    /// Export `run_id`'s verified stream as canonical NDJSON after validating
    /// every seat record.
    pub fn export_ndjson(&self, run_id: &str) -> Result<String, StoreError> {
        let events = self.load_verified(run_id)?;
        seat_record::validate_events(&events)?;
        format_canonical_ndjson(&events)
    }

    /// Verify an exported NDJSON stream, create `run_id`, append the verified
    /// envelopes verbatim, and apply them to projections. Returns the number
    /// of imported events.
    pub fn import_ndjson(
        &self,
        run_id: &str,
        manifest: &Value,
        ndjson: &str,
    ) -> Result<u64, StoreError> {
        let events = ndjson
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str::<EventEnvelope>)
            .collect::<Result<Vec<_>, _>>()?;
        verify_chain_after(run_id, 0, ZERO_HASH, &events)?;
        seat_record::validate_events(&events)?;
        self.create_run(run_id, manifest)?;
        self.event_log
            .append_envelopes(run_id, (0, ZERO_HASH), &events)?;
        self.projections.apply_envelopes(run_id, &events)?;
        Ok(events.len() as u64)
    }

    /// Rebuild all read-model projections from the underlying [`EventLog`].
    pub fn reindex(&self) -> Result<u64, StoreError> {
        self.projections.rebuild_from(&self.event_log)
    }

    fn validate_record(
        &self,
        run_id: &str,
        seq: u64,
        event_type: EventType,
        payload: &Value,
    ) -> Result<(), StoreError> {
        let Some(record) = seat_record::record_of(event_type, payload) else {
            return Ok(());
        };
        let manifest = self.event_log.manifest(run_id)?;
        let version =
            SeatRecordVersion::of_manifest(manifest.get("engine").and_then(Value::as_str));
        seat_record::validate_seat_record(record, seq, version)?;
        Ok(())
    }
}
