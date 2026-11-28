//! Isolated [`SqliteCoordinationStore`] implementation of [`CoordinationStore`]
//! with idempotent provider cooldown catch-up (decision 0126 rulings 3–4,
//! Slice 2).
//!
//! Stores dispatcher queue and provider cooldown state in a standalone SQLite
//! WAL database (such as `queue.db`), completely isolated from event logs and
//! read-model projections, reusing [`crate::queue`] schema, validation, and
//! SQL helpers directly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::provider_waits;
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    patiently, queue, Attribution, Cooldown, Cooling, EntryId, NewEntry, QueueEntry, StoreError,
    BUSY_TIMEOUT,
};

use super::sqlite_schema::{read_user_version, ExpectedSchema, SchemaMatch};
use super::{
    checked_seq, divergent, open_standalone_store, parse_clock, read_verified_log,
    CoordinationStore, EventLog,
};

const META_DDL: &str = "CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";
const MONOLITH_META_SQL: &str = "SELECT COUNT(*) FROM meta WHERE key = 'database_schema'";

static EXPECTED_COORDINATION_SCHEMA: LazyLock<ExpectedSchema> = LazyLock::new(|| {
    ExpectedSchema::from_initializer(|conn| {
        conn.execute_batch(META_DDL)
            .expect("canonical coordination meta DDL");
        queue::migrate_queue(conn).expect("canonical coordination queue migration");
    })
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoordinationSchemaState {
    Fresh,
    Ready,
}

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
        let (conn, owned) = open_standalone_store(
            path.as_ref(),
            "coordination_migrate",
            |conn, normalized| {
                let _ = inspect_coordination_schema(conn, normalized)?;
                Ok(())
            },
            |conn| {
                conn.pragma_update(None, "foreign_keys", "OFF")?;
                Ok(())
            },
            ensure_schema,
        )?;
        Ok(Self {
            conn,
            path: owned,
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
    /// requiring two-phase commit across storage engines; conflicting fields
    /// for the same key fail closed with [`StoreError::DivergentEventLog`].
    pub fn apply_cooldowns(
        &self,
        run_id: &str,
        envelopes: &[EventEnvelope],
    ) -> Result<u64, StoreError> {
        crate::import::recordable(run_id)?;
        validate_cooldown_envelopes(run_id, envelopes)?;
        let derived = extract_cooldowns(envelopes)?;
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
                let derived = extract_cooldowns(&run.events)?;
                inserted += apply_derived(&tx, &run.run_id, &derived)?;
            }
            tx.commit()?;
            Ok(inserted)
        })
    }
}

type CooldownBatch<'a> = (u64, &'a str, Vec<Cooling>);

fn apply_derived(
    tx: &Connection,
    run_id: &str,
    derived: &[CooldownBatch<'_>],
) -> Result<u64, StoreError> {
    let mut inserted = 0u64;
    for (seq, recorded_at, rows) in derived {
        inserted += queue::insert_cooldowns_idempotent(tx, run_id, *seq, recorded_at, rows)?;
    }
    Ok(inserted)
}

fn foreign_coordination_db(path: &Path) -> StoreError {
    StoreError::ForeignDatabase {
        path: path.to_path_buf(),
        expected: "coordination store",
    }
}

fn inspect_coordination_schema(
    conn: &Connection,
    path: &Path,
) -> Result<CoordinationSchemaState, StoreError> {
    let user_version = read_user_version(conn)?;
    if user_version != 0 || queue::has_foreign_queue_triggers(conn)? {
        return Err(foreign_coordination_db(path));
    }
    match EXPECTED_COORDINATION_SCHEMA.matches(conn)? {
        SchemaMatch::Empty => Ok(CoordinationSchemaState::Fresh),
        SchemaMatch::Foreign => Err(foreign_coordination_db(path)),
        SchemaMatch::Exact => {
            let monolith_meta: i64 = conn.query_row(MONOLITH_META_SQL, [], |row| row.get(0))?;
            if monolith_meta > 0 || queue::queue_schema(conn)? != Some(queue::QUEUE_SCHEMA) {
                return Err(foreign_coordination_db(path));
            }
            Ok(CoordinationSchemaState::Ready)
        }
    }
}

fn ensure_schema(conn: &Connection, path: &Path) -> Result<(), StoreError> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    if inspect_coordination_schema(&tx, path)? == CoordinationSchemaState::Fresh {
        tx.execute_batch(META_DDL)?;
        queue::migrate_queue(&tx)?;
    }
    tx.commit()?;
    Ok(())
}

fn validate_cooldown_envelopes(
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut seen: BTreeMap<u64, &str> = BTreeMap::new();
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
        if let Some(prev_hash) = seen.insert(envelope.seq, envelope.event_hash.as_str()) {
            if prev_hash != envelope.event_hash {
                return Err(divergent(run_id));
            }
        }
    }
    Ok(())
}

fn extract_cooldowns(envelopes: &[EventEnvelope]) -> Result<Vec<CooldownBatch<'_>>, StoreError> {
    let mut batches = Vec::new();
    for envelope in envelopes {
        if !matches!(
            envelope.event_type,
            EventType::EffectFailed | EventType::EffectIndeterminate
        ) {
            continue;
        }
        if let Some(waits) = provider_waits(envelope)? {
            let rows: Vec<Cooling> = waits.iter().map(Cooling::from).collect();
            batches.push((envelope.seq, envelope.recorded_at.as_str(), rows));
        }
    }
    Ok(batches)
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
