//! Synchronous storage ports and the backend-agnostic [`Journal`] facade
//! (decision 0126, rulings 1–3).
//!
//! `brokkr-store` separates three persistence concerns behind narrow
//! synchronous traits — [`EventLog`], [`ProjectionStore`], and
//! [`CoordinationStore`] — with no async runtime dependency. Each trait
//! marks a persistence seam whose operations must stay mutually consistent
//! across a storage backend boundary (decision 0071 ruling 2): a co-located
//! SQLite adapter over [`crate::Store`] today (Slice 1, via [`SqlitePorts`]),
//! local split files and trigger-free disposable indexes in Slice 2, and a
//! PostgreSQL service backend in Slice 3.
//!
//! Domain invariants (run-id and manifest admission, hash-chain continuity,
//! optimistic `expected_head` fencing, post-terminal refusal, seat-record
//! schema validation, and canonical NDJSON export/import) live once in
//! [`Journal`].

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{lawful_after_terminal, terminal_event};
use serde_json::Value;

use thiserror::Error;

use crate::{seat_record, Cooldown, EntryId, NewEntry, QueueEntry, SeatRecordVersion, StoreError};

mod journal;
mod sqlite;

pub use journal::{Journal, NewEvent};
pub use sqlite::SqlitePorts;

/// Why a run manifest was refused at the storage port boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ManifestRefusal {
    #[error("manifest must be a JSON object")]
    NotAnObject,
    #[error("manifest field 'engine' must be a JSON string")]
    EngineNotString,
    #[error("manifest field 'feature' must be a JSON string")]
    FeatureNotString,
    #[error("manifest field 'bundle_name' must be a JSON string")]
    BundleNameNotString,
}

pub(crate) fn format_canonical_ndjson(events: &[EventEnvelope]) -> Result<String, StoreError> {
    let mut out = String::new();
    for event in events {
        out.push_str(&serde_json::to_string(&serde_json::to_value(event)?)?);
        out.push('\n');
    }
    Ok(out)
}

/// Validate that `manifest` is a JSON object and, if `"engine"`, `"feature"`,
/// or `"bundle_name"` is present, that each is a JSON string (refusing
/// non-string `"engine"` rather than silently treating it as absent/V1, and
/// refusing non-string `"feature"` or `"bundle_name"` rather than projecting an
/// empty metadata value).
pub(crate) fn validate_manifest(run_id: &str, manifest: &Value) -> Result<(), StoreError> {
    let Some(object) = manifest.as_object() else {
        return Err(StoreError::InvalidManifest {
            run_id: run_id.to_string(),
            reason: ManifestRefusal::NotAnObject,
        });
    };
    for (field, reason) in [
        ("engine", ManifestRefusal::EngineNotString),
        ("feature", ManifestRefusal::FeatureNotString),
        ("bundle_name", ManifestRefusal::BundleNameNotString),
    ] {
        if object.get(field).is_some_and(|value| !value.is_string()) {
            return Err(StoreError::InvalidManifest {
                run_id: run_id.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// Validate an embedded `manifest` inside a `RunStarted` payload, if present.
pub(crate) fn validate_started_manifest(
    run_id: &str,
    event_type: EventType,
    payload: &Value,
) -> Result<(), StoreError> {
    if event_type == EventType::RunStarted {
        if let Some(manifest) = payload.get("manifest") {
            validate_manifest(run_id, manifest)?;
        }
    }
    Ok(())
}

pub(crate) fn non_empty_str(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|s| !s.is_empty())
}

pub(crate) fn started_feature(payload: &Value) -> Option<&str> {
    non_empty_str(payload.get("feature"))
        .or_else(|| non_empty_str(payload.pointer("/manifest/feature")))
}

pub(crate) fn started_feature_of(events: &[EventEnvelope]) -> Option<&str> {
    events
        .first()
        .filter(|event| event.seq == 1 && event.event_type == EventType::RunStarted)
        .and_then(|event| started_feature(&event.payload))
}

/// Validate every incoming envelope's seat record (`checkpoint` / `result`)
/// and marked `settled_members` against the stricter [`SeatRecordVersion`] of
/// the persisted `manifest` and `RunStarted`'s `/manifest/engine`.
pub(crate) fn validate_seat_records(
    manifest: &Value,
    existing: &[EventEnvelope],
    incoming: &[EventEnvelope],
) -> Result<(), StoreError> {
    let manifest_version =
        SeatRecordVersion::of_manifest(manifest.get("engine").and_then(Value::as_str));
    let started_version = seat_record::version_of_events(&[existing, incoming]);
    let version = manifest_version.stricter(started_version);
    seat_record::validate_incoming(existing, incoming, version)?;
    Ok(())
}

/// Refuse any non-annotation event in `incoming` once `existing` or an earlier
/// event in `incoming` has reached a terminal event.
pub(crate) fn refuse_after_terminal(
    run_id: &str,
    existing: &[EventEnvelope],
    incoming: &[EventType],
) -> Result<(), StoreError> {
    let mut terminal = existing
        .iter()
        .rev()
        .find_map(|event| terminal_event(event.event_type));
    for &event_type in incoming {
        if let Some(status) = terminal {
            if !lawful_after_terminal(event_type) {
                return Err(StoreError::AfterTerminal {
                    run_id: run_id.to_string(),
                    status,
                    event_type,
                });
            }
        } else {
            terminal = terminal_event(event_type);
        }
    }
    Ok(())
}

/// Persistence seam for durable per-run manifests and append-only event
/// streams (decision 0126 ruling 1; decision 0071 ruling 2).
///
/// Implementations persist pre-sealed envelopes atomically under an
/// optimistic `(seq, hash)` head fence and serve streams in sequence order;
/// domain validation and hashing remain in [`Journal`].
pub trait EventLog {
    /// Persist a new run record with its immutable manifest, refusing a
    /// duplicate `run_id` with [`StoreError::RunExists`] and an invalid
    /// manifest (see [`ManifestRefusal`]) with [`StoreError::InvalidManifest`].
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError>;

    /// Atomically append pre-sealed `envelopes` to `run_id` only while the
    /// stored head matches `expected_head`, refusing a moved head with
    /// [`StoreError::HeadMoved`], a duplicate or non-contiguous sequence number
    /// in a batch with [`brokkr_core::envelope::ChainError::SeqGap`], a foreign
    /// `run_id` with [`brokkr_core::envelope::ChainError::ForeignRun`],
    /// non-annotation events after a terminal event with
    /// [`StoreError::AfterTerminal`], an invalid embedded `RunStarted`
    /// manifest with [`StoreError::InvalidManifest`], and malformed seat
    /// records or settled members with [`StoreError::SeatRecord`].
    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// Atomically persist an imported run record and its verified pre-sealed
    /// `envelopes` in one transaction or unit of work, refusing an unadoptable
    /// `run_id` or an unfoldable non-empty stream with [`StoreError::Import`],
    /// non-annotation events after a terminal event with
    /// [`StoreError::AfterTerminal`], an invalid top-level or embedded
    /// `RunStarted` manifest with [`StoreError::InvalidManifest`], and
    /// malformed seat records or settled members with
    /// [`StoreError::SeatRecord`], leaving zero partial state on refusal, and
    /// leaving local origin unset so imported runs are never mistaken for runs
    /// started on this host (decision 0030 ruling 4).
    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// Return the current `(seq, event_hash)` head of `run_id`, or
    /// `(0, ZERO_HASH)` when the run holds no events yet.
    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError>;

    /// Read all stored envelopes of `run_id` with `seq >= from_seq`, in
    /// ascending sequence order. When `from_seq <= 1`, reads the run's full
    /// stored stream and refuses rows before sequence 1.
    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError>;

    /// Read the immutable manifest recorded when `run_id` was created.
    fn manifest(&self, run_id: &str) -> Result<Value, StoreError>;

    /// List every run id held by this event log.
    fn list_run_ids(&self) -> Result<Vec<String>, StoreError>;
}

/// Persistence seam for rebuildable read-model projections derived from an
/// [`EventLog`] (decision 0126 ruling 1; decision 0071 ruling 2).
///
/// Every row managed behind this trait is disposable and can be
/// reconstructed from an [`EventLog`] via [`ProjectionStore::rebuild_from`].
pub trait ProjectionStore {
    /// Project committed `envelopes` for `run_id` into read models.
    ///
    /// `manifest` supplies the run's persisted/attested manifest when known so
    /// decoupled projections have the manifest's metadata fallback (such as
    /// `"feature"`) even when `RunStarted` omits it. Co-located adapters such
    /// as [`SqlitePorts`] require `run_id` to exist in their local event log
    /// and verify that `manifest` (when supplied) and `envelopes` match the
    /// local verified stream; decoupled projection stores require a verified
    /// prefix starting at sequence 1. Implementations must remain monotonic and
    /// never regress a run whose projection is already at or past the batch's
    /// highest sequence number.
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// List known runs as `(run_id, feature, created_at)` triples.
    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError>;

    /// Rebuild projections from `log`, returning the total number of events
    /// projected across all runs.
    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError>;
}

/// Persistence seam for mutable operational coordination state — the
/// dispatcher queue and active provider cooldowns (decision 0126 rulings 1
/// and 3; decision 0071 ruling 2).
pub trait CoordinationStore {
    /// Enqueue a new work entry at the given RFC 3339 timestamp `now`.
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError>;

    /// List queue entries in dispatch order.
    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError>;

    /// Return all provider cooldowns still in force at the RFC 3339 instant
    /// `now`.
    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError>;
}

impl<T: EventLog + ?Sized> EventLog for &T {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        (**self).create_run(run_id, manifest)
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        (**self).append_envelopes(run_id, expected_head, envelopes)
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        (**self).import_run_stream(run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        (**self).head(run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        (**self).read_stream(run_id, from_seq)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        (**self).manifest(run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        (**self).list_run_ids()
    }
}

impl<T: ProjectionStore + ?Sized> ProjectionStore for &T {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        (**self).apply_envelopes(run_id, manifest, envelopes)
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        (**self).list_runs()
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        (**self).rebuild_from(log)
    }
}

impl<T: CoordinationStore + ?Sized> CoordinationStore for &T {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        (**self).enqueue(entry, now)
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        (**self).list_queue()
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        (**self).active_cooldowns(now)
    }
}

#[cfg(test)]
mod tests;
