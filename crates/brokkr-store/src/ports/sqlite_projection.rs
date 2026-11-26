//! Dedicated trigger-free [`SqliteProjectionStore`] implementation of
//! [`ProjectionStore`] (decision 0126 ruling 4, Slice 2).
//!
//! Stores disposable read-model tables (`projected_runs`, `projected_events`)
//! in a standalone SQLite WAL database (such as `index.db`) with no `runs` or
//! `events` tables and zero SQLite triggers. Any projection database can be
//! deleted and rebuilt cleanly from any [`EventLog`] via
//! [`ProjectionStore::rebuild_from`].

use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;

use crate::{patiently, StoreError, BUSY_TIMEOUT};

use super::{
    checked_seq, divergent, non_empty_str, open_standalone_sqlite, read_verified_log,
    started_feature_of, validate_manifest, validate_started_manifests, verify_chain_checked,
    EventLog, ProjectionStore,
};

const PROJECTION_SCHEMA_VERSION: &str = "1";

const PROJECTION_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS projection_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS projected_runs (
    run_id TEXT PRIMARY KEY,
    feature TEXT NOT NULL,
    created_at TEXT NOT NULL,
    head_seq INTEGER NOT NULL,
    head_hash TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS projected_events (
    run_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_hash TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);
CREATE INDEX IF NOT EXISTS projected_runs_created_idx
    ON projected_runs (created_at, run_id);
"#;

const SCHEMA_READY_SQL: &str = "SELECT COUNT(*) FROM sqlite_master
     WHERE type = 'table'
       AND name IN ('projection_meta', 'projected_runs', 'projected_events')";
const INSERT_PROJECTED_EVENT_SQL: &str =
    "INSERT INTO projected_events (run_id, seq, event_type, event_hash, recorded_at, payload)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
const UPSERT_PROJECTED_RUN_SQL: &str =
    "INSERT INTO projected_runs (run_id, feature, created_at, head_seq, head_hash)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT (run_id) DO UPDATE SET
         feature = excluded.feature,
         created_at = excluded.created_at,
         head_seq = excluded.head_seq,
         head_hash = excluded.head_hash";
const LIST_PROJECTED_RUNS_SQL: &str =
    "SELECT run_id, feature, created_at FROM projected_runs ORDER BY created_at, run_id";

/// Standalone trigger-free SQLite [`ProjectionStore`] (e.g. `index.db`).
pub struct SqliteProjectionStore {
    conn: Connection,
    path: PathBuf,
    patience: std::time::Duration,
}

struct StoredRun {
    feature: String,
    created_at: String,
    head_seq: u64,
    head_hash: String,
}

impl SqliteProjectionStore {
    /// Open (creating if absent) a disposable trigger-free SQLite projection
    /// database at `path` in WAL mode with `busy_timeout`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::open_at(path.as_ref())
    }

    fn open_at(path: &Path) -> Result<Self, StoreError> {
        let path = path.to_path_buf();
        let conn = open_standalone_sqlite(&path)?;
        patiently("projection_migrate", BUSY_TIMEOUT, &path, || {
            ensure_schema(&conn)
        })?;
        Ok(Self {
            conn,
            path,
            patience: BUSY_TIMEOUT,
        })
    }

    /// Return the path to this projection database file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn schema_ready(conn: &Connection) -> Result<bool, StoreError> {
    let tables: i64 = conn.query_row(SCHEMA_READY_SQL, [], |row| row.get(0))?;
    Ok(tables == 3)
}

fn ensure_schema(conn: &Connection) -> Result<(), StoreError> {
    if schema_ready(conn)? {
        return Ok(());
    }
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tx.execute_batch(PROJECTION_DDL)?;
    const INSERT_META: &str =
        "INSERT OR IGNORE INTO projection_meta (key, value) VALUES ('projection_schema', ?1)";
    tx.execute(INSERT_META, params![PROJECTION_SCHEMA_VERSION])?;
    tx.commit()?;
    Ok(())
}

fn load_stored_run(tx: &Connection, run_id: &str) -> Result<Option<StoredRun>, StoreError> {
    let row: Option<(String, String, i64, String)> = tx
        .query_row(
            "SELECT feature, created_at, head_seq, head_hash
             FROM projected_runs WHERE run_id = ?1",
            params![run_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((feature, created_at, head_seq, head_hash)) = row else {
        return Ok(None);
    };
    let head_seq = u64::try_from(head_seq).map_err(|_| divergent(run_id))?;
    Ok(Some(StoredRun {
        feature,
        created_at,
        head_seq,
        head_hash,
    }))
}

fn stored_event_hash(
    tx: &Connection,
    run_id: &str,
    seq: u64,
) -> Result<Option<String>, StoreError> {
    let seq = checked_seq(seq)?;
    let hash = tx
        .query_row(
            "SELECT event_hash FROM projected_events WHERE run_id = ?1 AND seq = ?2",
            params![run_id, seq],
            |row| row.get(0),
        )
        .optional()?;
    Ok(hash)
}

fn insert_projected_events(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut stmt = tx.prepare_cached(INSERT_PROJECTED_EVENT_SQL)?;
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?;
        let payload = serde_json::to_string(&envelope.payload)?;
        let args = params![
            run_id,
            seq,
            envelope.event_type.as_str(),
            envelope.event_hash,
            envelope.recorded_at,
            payload,
        ];
        stmt.execute(args)?;
    }
    Ok(())
}

fn upsert_projected_run(
    tx: &Connection,
    run_id: &str,
    feature: &str,
    created_at: &str,
    head_seq: u64,
    head_hash: &str,
) -> Result<(), StoreError> {
    let seq = checked_seq(head_seq)?;
    let args = params![run_id, feature, created_at, seq, head_hash];
    tx.execute(UPSERT_PROJECTED_RUN_SQL, args)?;
    Ok(())
}

fn resolved_feature<'a>(
    envelopes: &'a [EventEnvelope],
    manifest: Option<&'a Value>,
    existing: Option<&'a StoredRun>,
) -> &'a str {
    started_feature_of(envelopes)
        .or_else(|| manifest.and_then(|m| non_empty_str(m.get("feature"))))
        .or_else(|| {
            existing
                .map(|r| r.feature.as_str())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("")
}

fn apply_batch_tx(
    tx: &Connection,
    run_id: &str,
    manifest: Option<&Value>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let existing = load_stored_run(tx, run_id)?;
    let feature = resolved_feature(envelopes, manifest, existing.as_ref());
    let created_at = envelopes
        .first()
        .map(|e| e.recorded_at.as_str())
        .or_else(|| existing.as_ref().map(|r| r.created_at.as_str()))
        .unwrap_or("");

    let Some(last) = envelopes.last() else {
        if existing.as_ref().is_none_or(|row| row.head_seq == 0) {
            upsert_projected_run(tx, run_id, feature, created_at, 0, ZERO_HASH)?;
        }
        return Ok(());
    };

    let mut append_from = 0usize;
    if let Some(row) = &existing {
        if row.head_seq >= last.seq {
            if stored_event_hash(tx, run_id, last.seq)?.as_deref() != Some(last.event_hash.as_str())
            {
                return Err(divergent(run_id));
            }
            return Ok(());
        }
        if row.head_seq > 0 {
            let prefix = &envelopes[(row.head_seq - 1) as usize];
            if prefix.event_hash != row.head_hash {
                return Err(divergent(run_id));
            }
            append_from = row.head_seq as usize;
        }
    }

    insert_projected_events(tx, run_id, &envelopes[append_from..])?;
    upsert_projected_run(tx, run_id, feature, created_at, last.seq, &last.event_hash)?;
    Ok(())
}

impl ProjectionStore for SqliteProjectionStore {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        if let Some(manifest) = manifest {
            validate_manifest(run_id, manifest)?;
        }
        verify_chain_checked(run_id, (0, ZERO_HASH), envelopes)?;
        validate_started_manifests(run_id, envelopes)?;
        patiently("projection_apply", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            apply_batch_tx(&tx, run_id, manifest, envelopes)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        patiently("projection_list_runs", self.patience, &self.path, || {
            let mut stmt = self.conn.prepare(LIST_PROJECTED_RUNS_SQL)?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let verified = read_verified_log(log)?;
        patiently("projection_rebuild", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            tx.execute("DELETE FROM projected_events", [])?;
            tx.execute("DELETE FROM projected_runs", [])?;
            let mut total = 0u64;
            for run in &verified {
                let feature = resolved_feature(&run.events, Some(&run.manifest), None);
                let created_at = run
                    .events
                    .first()
                    .map_or("", |first| first.recorded_at.as_str());
                let (head_seq, head_hash) = run
                    .events
                    .last()
                    .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
                upsert_projected_run(&tx, &run.run_id, feature, created_at, head_seq, head_hash)?;
                insert_projected_events(&tx, &run.run_id, &run.events)?;
                total += run.events.len() as u64;
            }
            tx.commit()?;
            Ok(total)
        })
    }
}
