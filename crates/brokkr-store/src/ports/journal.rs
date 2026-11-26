//! Backend-agnostic [`Journal`] domain facade (decision 0126 ruling 2).
//!
//! Owns hash-chain continuity, optimistic `expected_head` fencing,
//! post-terminal refusal, `seat_record` schema validation, canonical NDJSON
//! export/import, and projection reindexing once across every storage backend.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{
    verify_chain_after, ChainError, EventEnvelope, EventType, EVENT_SCHEMA_VERSION,
};
use serde_json::Value;

use crate::{import, seat_record, ImportError, StoreError, VerifyError};

use super::{CoordinationStore, EventLog, ProjectionStore};

/// Event fields sealed by [`Journal::append_event`] and
/// [`Journal::append_if_head`].
pub struct NewEvent<'a> {
    pub event_type: EventType,
    pub payload: Value,
    pub causation_id: Option<&'a str>,
    pub attempt_id: Option<&'a str>,
    pub ts: &'a str,
}

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
    /// Admit a new run id, validate its manifest shape, and persist it in the
    /// event log.
    pub fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        import::recordable(run_id)?;
        super::validate_manifest(run_id, manifest)?;
        self.event_log.create_run(run_id, manifest)
    }

    /// Read the current head of `run_id`, seal a hash-chained event onto it,
    /// enforce post-terminal and seat-record gates, append under the head
    /// fence, and update projections.
    pub fn append_event(
        &self,
        run_id: &str,
        event: NewEvent<'_>,
    ) -> Result<EventEnvelope, StoreError> {
        let (seq, hash) = self.event_log.head(run_id)?;
        self.append_checked(run_id, (seq, &hash), event)
    }

    /// Seal and append an event onto `run_id` only while its head matches
    /// `expected_head`, refusing a moved head with [`StoreError::HeadMoved`].
    pub fn append_if_head(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        event: NewEvent<'_>,
    ) -> Result<EventEnvelope, StoreError> {
        self.append_checked(run_id, expected_head, event)
    }

    /// Load `run_id`'s full event stream from sequence 1 and verify hash-chain
    /// and run-id integrity.
    pub fn load_verified(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        let events = self.event_log.read_stream(run_id, 1)?;
        verify_chain_after(run_id, 0, ZERO_HASH, &events)?;
        Ok(events)
    }

    /// Export `run_id`'s verified stream as canonical NDJSON after validating
    /// every seat record.
    pub fn export_ndjson(&self, run_id: &str) -> Result<String, StoreError> {
        let events = self.load_verified(run_id)?;
        seat_record::validate_events(&events)?;
        super::format_canonical_ndjson(&events)
    }

    /// Verify and adopt an exported NDJSON stream under all adoption gates,
    /// persist the run and its verified envelopes atomically using the
    /// hash-attested manifest from `run/started`, and apply them to
    /// projections. Returns the number of imported events.
    ///
    /// If the [`EventLog`] commits the verified import and updating projections
    /// then fails, this returns `ImportError::Store(StoreError::ProjectionLag { .. })`.
    /// That error means the import is already durable in the [`EventLog`];
    /// recover with [`Journal::reindex`] rather than retrying `import_ndjson`
    /// (a retry would be refused with [`ImportError::Collision`]).
    pub fn import_ndjson(
        &self,
        run_id: &str,
        manifest: &Value,
        ndjson: &str,
    ) -> Result<u64, ImportError> {
        import::admit_run_id(run_id)?;
        if is_redacted(manifest) {
            return Err(ImportError::Redacted(run_id.to_string()));
        }
        let (events, _state) = import::verified_events(ndjson)?;
        if events[0].run_id != run_id {
            return Err(ImportError::Verify(VerifyError::Chain(
                ChainError::ForeignRun { seq: 1 },
            )));
        }
        let attested = events[0]
            .payload
            .get("manifest")
            .ok_or(ImportError::Unattested { field: "manifest" })?;
        if is_redacted(attested) {
            return Err(ImportError::Redacted(run_id.to_string()));
        }
        super::validate_manifest(run_id, attested)?;
        let _feature = super::started_feature(&events[0].payload)
            .ok_or(ImportError::Unattested { field: "feature" })?;
        self.event_log
            .import_run_stream(run_id, attested, &events)
            .map_err(|error| match error {
                StoreError::RunExists(id) => ImportError::Collision(id),
                other => ImportError::Store(other),
            })?;
        self.apply_projections(run_id, attested, &events)?;
        Ok(events.len() as u64)
    }

    /// Rebuild all read-model projections from the underlying [`EventLog`].
    pub fn reindex(&self) -> Result<u64, StoreError> {
        self.projections.rebuild_from(&self.event_log)
    }

    fn append_checked(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        event: NewEvent<'_>,
    ) -> Result<EventEnvelope, StoreError> {
        let seq = expected_head
            .0
            .checked_add(1)
            .ok_or(ChainError::SeqOverflow {
                after: expected_head.0,
            })?;
        let mut events = self.event_log.read_stream(run_id, 1)?;
        verify_chain_after(run_id, 0, ZERO_HASH, &events)?;
        let (found_seq, found_hash) = events
            .last()
            .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
        if found_seq != expected_head.0 || found_hash != expected_head.1 {
            return Err(StoreError::HeadMoved {
                expected_seq: expected_head.0,
                found_seq,
            });
        }
        super::refuse_after_terminal(run_id, &events, &[event.event_type])?;
        super::validate_started_manifest(run_id, event.event_type, &event.payload)?;
        let manifest = self.event_log.manifest(run_id)?;
        let envelope = EventEnvelope {
            run_id: run_id.to_string(),
            seq,
            event_id: uuid::Uuid::new_v4().to_string(),
            event_schema_version: EVENT_SCHEMA_VERSION,
            event_type: event.event_type,
            payload: event.payload,
            causation_id: event.causation_id.map(str::to_owned),
            correlation_id: run_id.to_string(),
            attempt_id: event.attempt_id.map(str::to_owned),
            recorded_at: event.ts.to_string(),
            previous_hash: expected_head.1.to_string(),
            event_hash: String::new(),
        }
        .sealed();
        super::validate_seat_records(&manifest, &events, std::slice::from_ref(&envelope))?;
        self.event_log
            .append_envelopes(run_id, expected_head, std::slice::from_ref(&envelope))?;
        events.push(envelope.clone());
        self.apply_projections(run_id, &manifest, &events)?;
        Ok(envelope)
    }

    fn apply_projections(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        self.projections
            .apply_envelopes(run_id, Some(manifest), envelopes)
            .map_err(|error| StoreError::ProjectionLag {
                run_id: run_id.to_string(),
                source: Box::new(error),
            })
    }
}

fn is_redacted(manifest: &Value) -> bool {
    manifest.get("redacted") == Some(&Value::Bool(true))
}
