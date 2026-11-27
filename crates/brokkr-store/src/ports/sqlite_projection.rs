//! Dedicated trigger-free [`SqliteProjectionStore`] implementation of
//! [`ProjectionStore`] (decision 0126 ruling 4, Slice 2).
//!
//! Stores disposable read-model tables (`projected_runs`, `projected_events`)
//! in a standalone SQLite WAL database (such as `index.db`) with no `runs` or
//! `events` tables and zero SQLite triggers. Any projection database can be
//! deleted and rebuilt cleanly from any [`EventLog`] via
//! [`ProjectionStore::rebuild_from`].

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;

use crate::{import, patiently, StoreError, BUSY_TIMEOUT};

use super::{
    checked_seq, divergent, non_empty_str, open_standalone_store, read_verified_log,
    started_feature_of, validate_manifest, validate_started_manifests, verify_chain_checked,
    EventLog, ProjectionStore,
};

const PROJECTION_SCHEMA_VERSION: &str = "1";
const PROJECTION_USER_VERSION: u32 = 1;

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
const FOREIGN_TABLES_SQL: &str = "SELECT COUNT(*) FROM sqlite_master
     WHERE type = 'table'
       AND name NOT LIKE 'sqlite_%'
       AND name NOT IN ('projection_meta', 'projected_runs', 'projected_events')";
const ALL_USER_TABLES_SQL: &str = "SELECT COUNT(*) FROM sqlite_master
     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'";
const TRIGGERS_SQL: &str = "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger'";
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
const SELECT_PREFIX_HASHES_SQL: &str =
    "SELECT seq, event_hash FROM projected_events WHERE run_id = ?1 AND seq <= ?2 ORDER BY seq";

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
        let path = path.as_ref();
        let (conn, owned) = open_standalone_store(
            path,
            "projection_migrate",
            |_| Ok(()),
            |conn| ensure_schema(conn, path),
        )?;
        Ok(Self {
            conn,
            path: owned,
            patience: BUSY_TIMEOUT,
        })
    }

    /// Return the path to this projection database file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn foreign_projection_db(path: &Path) -> StoreError {
    StoreError::ForeignDatabase {
        path: path.to_path_buf(),
        expected: "projection index",
    }
}

fn init_projection_schema(conn: &Connection) -> Result<(), StoreError> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    tx.execute_batch(PROJECTION_DDL)?;
    const INSERT_META: &str =
        "INSERT OR IGNORE INTO projection_meta (key, value) VALUES ('projection_schema', ?1)";
    tx.execute(INSERT_META, params![PROJECTION_SCHEMA_VERSION])?;
    tx.pragma_update(None, "user_version", PROJECTION_USER_VERSION)?;
    tx.commit()?;
    Ok(())
}

fn verify_projection_schema(
    conn: &Connection,
    path: &Path,
    user_version: u32,
) -> Result<(), StoreError> {
    let ready: i64 = conn.query_row(SCHEMA_READY_SQL, [], |row| row.get(0))?;
    if ready != 3 || user_version != PROJECTION_USER_VERSION {
        return Err(foreign_projection_db(path));
    }
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM projection_meta WHERE key = 'projection_schema'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if version.as_deref() != Some(PROJECTION_SCHEMA_VERSION) {
        return Err(foreign_projection_db(path));
    }
    Ok(())
}

fn ensure_schema(conn: &Connection, path: &Path) -> Result<(), StoreError> {
    let foreign: i64 = conn.query_row(FOREIGN_TABLES_SQL, [], |row| row.get(0))?;
    let triggers: i64 = conn.query_row(TRIGGERS_SQL, [], |row| row.get(0))?;
    let user_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if foreign > 0 || triggers > 0 {
        return Err(foreign_projection_db(path));
    }
    let total_tables: i64 = conn.query_row(ALL_USER_TABLES_SQL, [], |row| row.get(0))?;
    if total_tables == 0 {
        if user_version != 0 {
            return Err(foreign_projection_db(path));
        }
        return init_projection_schema(conn);
    }
    verify_projection_schema(conn, path, user_version)
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

fn verify_stored_prefix(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<BTreeSet<u64>, StoreError> {
    let limit = checked_seq(envelopes.len() as u64)?;
    let mut stmt = tx.prepare(SELECT_PREFIX_HASHES_SQL)?;
    let rows = stmt.query_map(params![run_id, limit], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut present = BTreeSet::new();
    for row in rows {
        let (seq, hash) = row?;
        let idx = usize::try_from(seq)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(|| divergent(run_id))?;
        if envelopes[idx].event_hash != hash {
            return Err(divergent(run_id));
        }
        present.insert(seq as u64);
    }
    Ok(present)
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

fn repair_missing_events(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
    present: &BTreeSet<u64>,
) -> Result<(), StoreError> {
    let missing: Vec<EventEnvelope> = envelopes
        .iter()
        .filter(|env| !present.contains(&env.seq))
        .cloned()
        .collect();
    if !missing.is_empty() {
        insert_projected_events(tx, run_id, &missing)?;
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

fn resolved_created_at<'a>(
    envelopes: &'a [EventEnvelope],
    manifest: Option<&'a Value>,
    existing: Option<&'a StoredRun>,
) -> &'a str {
    manifest
        .and_then(|m| non_empty_str(m.get("created_at")))
        .or_else(|| envelopes.first().map(|e| e.recorded_at.as_str()))
        .or_else(|| {
            existing
                .map(|r| r.created_at.as_str())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("")
}

fn reconcile_existing_run(
    tx: &Connection,
    run_id: &str,
    existing: Option<&StoredRun>,
    envelopes: &[EventEnvelope],
    last: &EventEnvelope,
) -> Result<Option<usize>, StoreError> {
    let Some(row) = existing else {
        return Ok(Some(0));
    };
    if row.head_seq >= last.seq {
        if row.head_seq == last.seq && row.head_hash != last.event_hash {
            return Err(divergent(run_id));
        }
        let present = verify_stored_prefix(tx, run_id, envelopes)?;
        repair_missing_events(tx, run_id, envelopes, &present)?;
        return Ok(None);
    }
    if row.head_seq == 0 {
        return Ok(Some(0));
    }
    let prefix = &envelopes[..row.head_seq as usize];
    if prefix[(row.head_seq - 1) as usize].event_hash != row.head_hash {
        return Err(divergent(run_id));
    }
    let present = verify_stored_prefix(tx, run_id, prefix)?;
    repair_missing_events(tx, run_id, prefix, &present)?;
    Ok(Some(row.head_seq as usize))
}

fn apply_batch_tx(
    tx: &Connection,
    run_id: &str,
    manifest: Option<&Value>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let existing = load_stored_run(tx, run_id)?;
    let feature = resolved_feature(envelopes, manifest, existing.as_ref());
    let created_at = resolved_created_at(envelopes, manifest, existing.as_ref());

    let Some(last) = envelopes.last() else {
        if existing.as_ref().is_none_or(|row| row.head_seq == 0) {
            upsert_projected_run(tx, run_id, feature, created_at, 0, ZERO_HASH)?;
        }
        return Ok(());
    };

    let Some(append_from) = reconcile_existing_run(tx, run_id, existing.as_ref(), envelopes, last)?
    else {
        return Ok(());
    };
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
        import::recordable(run_id)?;
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
        patiently("projection_rebuild", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let verified = read_verified_log(log)?;
            tx.execute("DELETE FROM projected_events", [])?;
            tx.execute("DELETE FROM projected_runs", [])?;
            let mut total = 0u64;
            for run in &verified {
                let feature = resolved_feature(&run.events, Some(&run.manifest), None);
                let created_at = resolved_created_at(&run.events, Some(&run.manifest), None);
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
