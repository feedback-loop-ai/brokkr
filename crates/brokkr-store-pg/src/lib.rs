//! PostgreSQL storage backend implementing [`EventLog`], [`ProjectionStore`],
//! and [`CoordinationStore`] over the synchronous PostgreSQL v3 wire protocol
//! (decision 0126 ruling 5).
//!
//! Zero async runtime and zero new third-party crates: uses `std::net::TcpStream`
//! with `SCRAM-SHA-256` (RFC 5802 / RFC 7677), cleartext, and trust
//! authentication, Extended Query parameterization, and `FOR UPDATE SKIP LOCKED`
//! queue claims.
//!
//! Callers may compose either separate PostgreSQL databases via [`PgEventLog`],
//! [`PgProjectionStore`], and [`PgCoordinationStore`], or a single shared
//! PostgreSQL database via [`PgStore`], directly inside
//! [`brokkr_store::ports::Journal`].

use std::sync::{Mutex, MutexGuard, PoisonError};

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::ports::{read_verified_log, CoordinationStore, EventLog, ProjectionStore};
use brokkr_store::{Cooldown, EntryId, NewEntry, QueueEntry, StoreError};
use serde_json::Value;

mod config;
mod coordination;
mod error;
mod event_log;
mod projection;
mod schema;
mod scram;
mod wire;

#[cfg(test)]
mod coord_tests;
#[cfg(test)]
mod mock_pg_tests;
#[cfg(test)]
mod ports_tests;
#[cfg(test)]
mod wire_tests;

pub use config::{PgConfig, DEFAULT_CONNECT_TIMEOUT, DEFAULT_IO_TIMEOUT, DEFAULT_PORT};
pub use coordination::ClaimedEntry;
pub use error::PgError;
pub use wire::{PgClient, PgRow, QueryResult};

fn lock_client(mutex: &Mutex<PgClient>) -> MutexGuard<'_, PgClient> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// PostgreSQL [`EventLog`] backend over `brokkr_runs` and `brokkr_events`.
pub struct PgEventLog {
    client: Mutex<PgClient>,
}

impl PgEventLog {
    /// Connect to PostgreSQL and initialize the event-log tables idempotently.
    pub fn connect(config: &PgConfig) -> Result<Self, PgError> {
        let mut client = PgClient::connect(config)?;
        schema::init_statements(&mut client, schema::EVENT_LOG_DDL)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
}

impl EventLog for PgEventLog {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        event_log::create_run(&mut lock_client(&self.client), run_id, manifest)
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        event_log::append_envelopes(
            &mut lock_client(&self.client),
            run_id,
            expected_head,
            envelopes,
        )
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        event_log::import_run_stream(&mut lock_client(&self.client), run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        event_log::head(&mut lock_client(&self.client), run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        event_log::read_stream(&mut lock_client(&self.client), run_id, from_seq)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        event_log::manifest(&mut lock_client(&self.client), run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        event_log::list_run_ids(&mut lock_client(&self.client))
    }
}

/// PostgreSQL [`ProjectionStore`] backend over `brokkr_projected_runs` and
/// `brokkr_projected_events`.
pub struct PgProjectionStore {
    client: Mutex<PgClient>,
}

impl PgProjectionStore {
    /// Connect to PostgreSQL and initialize the projection tables idempotently.
    pub fn connect(config: &PgConfig) -> Result<Self, PgError> {
        let mut client = PgClient::connect(config)?;
        schema::init_statements(&mut client, schema::PROJECTION_DDL)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
}

impl ProjectionStore for PgProjectionStore {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        projection::apply_envelopes(&mut lock_client(&self.client), run_id, manifest, envelopes)
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        projection::list_runs(&mut lock_client(&self.client))
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let verified = read_verified_log(log)?;
        projection::rebuild_from_verified(&mut lock_client(&self.client), verified)
    }
}

/// PostgreSQL [`CoordinationStore`] backend over `brokkr_queue_entries`,
/// `brokkr_queue_waits`, and `brokkr_provider_cooldowns`.
pub struct PgCoordinationStore {
    client: Mutex<PgClient>,
}

impl PgCoordinationStore {
    /// Connect to PostgreSQL and initialize the coordination tables idempotently.
    pub fn connect(config: &PgConfig) -> Result<Self, PgError> {
        let mut client = PgClient::connect(config)?;
        schema::init_statements(&mut client, schema::COORDINATION_DDL)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    /// Atomically claim the next pending queue entry in dispatch order using
    /// `SELECT ... FOR UPDATE SKIP LOCKED LIMIT 1`.
    pub fn claim_next_entry(
        &self,
        worker_id: &str,
        now: &str,
    ) -> Result<Option<ClaimedEntry>, StoreError> {
        coordination::claim_next_entry(&mut lock_client(&self.client), worker_id, now)
    }

    /// Derive and idempotently persist provider cooldowns from committed
    /// `envelopes` belonging to `run_id`, returning newly inserted row count.
    pub fn apply_cooldowns(
        &self,
        run_id: &str,
        envelopes: &[EventEnvelope],
    ) -> Result<u64, StoreError> {
        coordination::apply_cooldowns(&mut lock_client(&self.client), run_id, envelopes)
    }

    /// Scan every verified run in `log` and idempotently catch up provider
    /// cooldowns, returning newly inserted row count.
    pub fn sync_cooldowns_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let verified = read_verified_log(log)?;
        coordination::sync_verified_cooldowns(&mut lock_client(&self.client), &verified)
    }
}

impl CoordinationStore for PgCoordinationStore {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        coordination::enqueue(&mut lock_client(&self.client), entry, now)
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        coordination::list_queue(&mut lock_client(&self.client))
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        coordination::active_cooldowns(&mut lock_client(&self.client), now)
    }
}

/// Unified PostgreSQL backend implementing [`EventLog`], [`ProjectionStore`],
/// and [`CoordinationStore`] over a single PostgreSQL database.
pub struct PgStore {
    client: Mutex<PgClient>,
}

impl PgStore {
    /// Connect to PostgreSQL and initialize all event-log, projection, and
    /// coordination tables idempotently.
    pub fn connect(config: &PgConfig) -> Result<Self, PgError> {
        let mut client = PgClient::connect(config)?;
        schema::init_statements(&mut client, schema::EVENT_LOG_DDL)?;
        schema::init_statements(&mut client, schema::PROJECTION_DDL)?;
        schema::init_statements(&mut client, schema::COORDINATION_DDL)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    /// Atomically claim the next pending queue entry in dispatch order using
    /// `SELECT ... FOR UPDATE SKIP LOCKED LIMIT 1`.
    pub fn claim_next_entry(
        &self,
        worker_id: &str,
        now: &str,
    ) -> Result<Option<ClaimedEntry>, StoreError> {
        coordination::claim_next_entry(&mut lock_client(&self.client), worker_id, now)
    }

    /// Derive and idempotently persist provider cooldowns from committed
    /// `envelopes` belonging to `run_id`, returning newly inserted row count.
    pub fn apply_cooldowns(
        &self,
        run_id: &str,
        envelopes: &[EventEnvelope],
    ) -> Result<u64, StoreError> {
        coordination::apply_cooldowns(&mut lock_client(&self.client), run_id, envelopes)
    }

    /// Scan every verified run in `log` and idempotently catch up provider
    /// cooldowns, returning newly inserted row count.
    pub fn sync_cooldowns_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let verified = read_verified_log(log)?;
        coordination::sync_verified_cooldowns(&mut lock_client(&self.client), &verified)
    }
}

impl EventLog for PgStore {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        event_log::create_run(&mut lock_client(&self.client), run_id, manifest)
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        event_log::append_envelopes(
            &mut lock_client(&self.client),
            run_id,
            expected_head,
            envelopes,
        )
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        event_log::import_run_stream(&mut lock_client(&self.client), run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        event_log::head(&mut lock_client(&self.client), run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        event_log::read_stream(&mut lock_client(&self.client), run_id, from_seq)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        event_log::manifest(&mut lock_client(&self.client), run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        event_log::list_run_ids(&mut lock_client(&self.client))
    }
}

impl ProjectionStore for PgStore {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        projection::apply_envelopes(&mut lock_client(&self.client), run_id, manifest, envelopes)
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        projection::list_runs(&mut lock_client(&self.client))
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let verified = read_verified_log(log)?;
        projection::rebuild_from_verified(&mut lock_client(&self.client), verified)
    }
}

impl CoordinationStore for PgStore {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        coordination::enqueue(&mut lock_client(&self.client), entry, now)
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        coordination::list_queue(&mut lock_client(&self.client))
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        coordination::active_cooldowns(&mut lock_client(&self.client), now)
    }
}
