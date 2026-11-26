//! Isolated [`SqliteCoordinationStore`] implementation of [`CoordinationStore`]
//! with idempotent provider cooldown catch-up (decision 0126 rulings 3–4,
//! Slice 2).
//!
//! Stores dispatcher queue and provider cooldown state in a standalone SQLite
//! WAL database (such as `queue.db`), completely isolated from event logs and
//! read-model projections, reusing [`crate::queue`] schema, validation, and
//! SQL helpers directly.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::{provider_waits, ProviderWait};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    patiently, queue, Attribution, Cooldown, Cooling, EntryId, NewEntry, QueueEntry, StoreError,
    BUSY_TIMEOUT,
};

use super::{
    checked_seq, open_standalone_sqlite, parse_clock, read_verified_log, CoordinationStore,
    EventLog,
};

const META_DDL: &str = "CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";
const META_PRESENT_SQL: &str =
    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'meta'";

/// Standalone SQLite [`CoordinationStore`] (e.g. `queue.db`), isolated from
/// event logs and projections.
pub struct SqliteCoordinationStore {
    conn: Connection,
    path: PathBuf,
    patience: std::time::Duration,
}

impl SqliteCoordinationStore {
    /// Open (creating if absent) an isolated SQLite coordination database at
    /// `path` in WAL mode with `busy_timeout`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::open_at(path.as_ref())
    }

    fn open_at(path: &Path) -> Result<Self, StoreError> {
        let path = path.to_path_buf();
        let conn = open_standalone_sqlite(&path)?;
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        patiently("coordination_migrate", BUSY_TIMEOUT, &path, || {
            ensure_schema(&conn)
        })?;
        Ok(Self {
            conn,
            path,
            patience: BUSY_TIMEOUT,
        })
    }

    /// Return the path to this coordination database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Derive and idempotently persist provider cooldowns from committed
    /// `envelopes` belonging to `run_id`, returning the count of newly
    /// recorded cooldown rows (decision 0126 ruling 3).
    ///
    /// Re-applying the same envelopes is a no-op (`Ok(0)`) because cooldown
    /// rows are deduplicated on `(run_id, event_seq, site, rounds)` without
    /// requiring two-phase commit across storage engines.
    pub fn apply_cooldowns(
        &self,
        run_id: &str,
        envelopes: &[EventEnvelope],
    ) -> Result<u64, StoreError> {
        validate_cooldown_envelopes(run_id, envelopes)?;
        let derived = extract_cooldowns(envelopes);
        patiently("apply_cooldowns", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let inserted = apply_derived(&tx, run_id, &derived)?;
            tx.commit()?;
            Ok(inserted)
        })
    }

    /// Scan every verified run in `log` and idempotently catch up provider
    /// cooldowns across the entire event log, returning the count of newly
    /// recorded cooldown rows (decision 0126 ruling 3).
    pub fn sync_cooldowns_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let verified = read_verified_log(log)?;
        patiently("sync_cooldowns", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let mut inserted = 0u64;
            for run in &verified {
                let derived = extract_cooldowns(&run.events);
                inserted += apply_derived(&tx, &run.run_id, &derived)?;
            }
            tx.commit()?;
            Ok(inserted)
        })
    }
}

fn apply_derived(
    tx: &Connection,
    run_id: &str,
    derived: &[(u64, &str, Vec<Cooling>)],
) -> Result<u64, StoreError> {
    let mut inserted = 0u64;
    for (seq, recorded_at, rows) in derived {
        inserted += queue::insert_cooldowns_idempotent(tx, run_id, *seq, recorded_at, rows)?;
    }
    Ok(inserted)
}

fn meta_present(conn: &Connection) -> Result<bool, StoreError> {
    let found: i64 = conn.query_row(META_PRESENT_SQL, [], |row| row.get(0))?;
    Ok(found == 1)
}

fn ensure_schema(conn: &Connection) -> Result<(), StoreError> {
    if meta_present(conn)? && queue::queue_intact(conn)? {
        return Ok(());
    }
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tx.execute_batch(META_DDL)?;
    queue::migrate_queue(&tx)?;
    tx.commit()?;
    Ok(())
}

fn validate_cooldown_envelopes(
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        if envelope.run_id != run_id {
            return Err(StoreError::Chain(ChainError::ForeignRun {
                seq: envelope.seq,
            }));
        }
        if envelope.seq < 1 {
            return Err(StoreError::RowBeforeChain {
                run_id: run_id.to_string(),
                seq: 0,
            });
        }
        let _ = checked_seq(envelope.seq)?;
        if envelope.event_hash != envelope.compute_hash() {
            return Err(StoreError::Chain(ChainError::BadHash { seq: envelope.seq }));
        }
    }
    Ok(())
}

fn cooling_of_wait(wait: &ProviderWait) -> Cooling {
    Cooling {
        site: wait.site.clone().unwrap_or_default(),
        provider: wait.provider.clone(),
        route: wait.route.clone(),
        until_at: wait.until,
        reason: wait.words.clone(),
        retry_after_seconds: wait.retry_after_seconds,
        rounds: wait.rounds,
    }
}

fn extract_cooldowns(envelopes: &[EventEnvelope]) -> Vec<(u64, &str, Vec<Cooling>)> {
    let mut batches = Vec::new();
    for envelope in envelopes {
        if !matches!(
            envelope.event_type,
            EventType::EffectFailed | EventType::EffectIndeterminate
        ) {
            continue;
        }
        if let Ok(Some(waits)) = provider_waits(envelope) {
            let rows: Vec<Cooling> = waits.iter().map(cooling_of_wait).collect();
            batches.push((envelope.seq, envelope.recorded_at.as_str(), rows));
        }
    }
    batches
}

impl CoordinationStore for SqliteCoordinationStore {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        let _ = parse_clock(now)?;
        let by = Attribution {
            operator: "port",
            reason: "enqueue",
        };
        patiently("queue_add", self.patience, &self.path, || {
            queue::add_at(&self.conn, *entry, by, now)
        })
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        patiently("queue_list", self.patience, &self.path, || {
            queue::list_once(&self.conn, &mut || {})
        })
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        let parsed = parse_clock(now)?;
        patiently("cooldowns_in_force", self.patience, &self.path, || {
            queue::cooldowns_in_force_conn(&self.conn, parsed)
        })
    }
}
