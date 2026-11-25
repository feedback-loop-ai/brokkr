//! Synchronous storage ports and the backend-agnostic [`Journal`] facade
//! (decision 0126, rulings 1–3).
//!
//! `brokkr-store` separates three persistence concerns behind narrow
//! synchronous traits — [`EventLog`], [`ProjectionStore`], and
//! [`CoordinationStore`] — with no async runtime dependency. Each trait
//! marks a persistence seam whose operations must stay mutually consistent
//! across a storage backend boundary (decision 0071 ruling 2): SQLite today
//! (Slice 1, via [`SqlitePorts`]), local split files and disposable indexes
//! in Slice 2, and a PostgreSQL service backend in Slice 3.
//!
//! Domain invariants (run-id admission, hash-chain continuity, optimistic
//! `expected_head` fencing, post-terminal refusal, seat-record schema
//! validation, and canonical NDJSON export/import) live once in [`Journal`].

use brokkr_core::envelope::EventEnvelope;
use serde_json::Value;

use crate::{Cooldown, EntryId, NewEntry, QueueEntry, StoreError};

mod journal;
mod sqlite;

pub use journal::Journal;
pub use sqlite::SqlitePorts;

/// Persistence seam for durable per-run manifests and append-only event
/// streams (decision 0126 ruling 1; decision 0071 ruling 2).
///
/// Implementations persist pre-sealed envelopes atomically under an
/// optimistic `(seq, hash)` head fence and serve streams in sequence order;
/// domain validation and hashing remain in [`Journal`].
pub trait EventLog {
    /// Persist a new run record with its immutable manifest, refusing a
    /// duplicate `run_id` with [`StoreError::RunExists`].
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError>;

    /// Atomically append pre-sealed `envelopes` to `run_id` only while the
    /// stored head matches `expected_head`, refusing a moved head with
    /// [`StoreError::HeadMoved`], a duplicate sequence number with
    /// [`StoreError::AppendConflict`], a foreign `run_id` with
    /// [`brokkr_core::envelope::ChainError::ForeignRun`], and non-annotation
    /// events after a terminal event with [`StoreError::AfterTerminal`].
    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError>;

    /// Atomically persist an imported run record and its verified pre-sealed
    /// `envelopes` in one transaction or unit of work, leaving zero partial
    /// state on refusal and leaving local origin unset so imported runs are
    /// never mistaken for runs started on this host (decision 0030 ruling 4).
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
/// reconstructed from any [`EventLog`] via [`ProjectionStore::rebuild_from`].
pub trait ProjectionStore {
    /// Project newly committed `envelopes` for `run_id` into read models.
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError>;

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
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        (**self).apply_envelopes(run_id, envelopes)
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
