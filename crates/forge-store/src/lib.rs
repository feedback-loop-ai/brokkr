//! The runtime store: bundled SQLite holding append-only facts.
//!
//! Application triggers reject UPDATE and DELETE on event rows — this
//! protects against ordinary defects, not a hostile database owner
//! (target-architecture). The journal is runtime truth; canonical NDJSON
//! export is the portable, human-auditable form. Single-host,
//! single-logical-writer: a concurrent writer loses on the (run_id, seq)
//! primary key inside its append transaction — optimistic fencing
//! instead of a lease service.

use std::path::Path;

use forge_core::canonical::ZERO_HASH;
use forge_core::envelope::{verify_chain, ChainError, EventEnvelope, EventType};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const DATABASE_SCHEMA: u32 = 1;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("run '{0}' not found")]
    RunNotFound(String),
    #[error("run '{0}' already exists")]
    RunExists(String),
    #[error("chain: {0}")]
    Chain(#[from] ChainError),
    #[error("database schema {found} unsupported (want {DATABASE_SCHEMA})")]
    SchemaMismatch { found: u32 },
    #[error("append conflict: seq {seq} already written by another writer")]
    AppendConflict { seq: u64 },
}

pub struct Store {
    conn: Connection,
}

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    feature TEXT NOT NULL,
    bundle_name TEXT NOT NULL,
    manifest TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    seq INTEGER NOT NULL,
    event_hash TEXT NOT NULL,
    envelope TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);
CREATE TRIGGER IF NOT EXISTS events_append_only_update
    BEFORE UPDATE ON events
    BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TRIGGER IF NOT EXISTS events_append_only_delete
    BEFORE DELETE ON events
    BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TRIGGER IF NOT EXISTS runs_manifest_immutable
    BEFORE UPDATE OF manifest, run_id ON runs
    BEGIN SELECT RAISE(ABORT, 'run manifests are immutable'); END;
"#;

impl Store {
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        conn.execute_batch(MIGRATION_V1)?;
        let schema = DATABASE_SCHEMA.to_string();
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('database_schema', ?1)",
            [&schema],
        )?;
        let found: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'database_schema'",
            [],
            |row| row.get(0),
        )?;
        let found: u32 = found.parse().unwrap_or(0);
        if found != DATABASE_SCHEMA {
            return Err(StoreError::SchemaMismatch { found });
        }
        Ok(Store { conn })
    }

    /// Open an EXISTING journal for reading only. The connection carries
    /// `SQLITE_OPEN_READ_ONLY`, so every write refuses at the database
    /// rather than at a reviewer's memory: a read surface that opens
    /// this way is *unable* to append, which is what decision 0020 asks
    /// of Muninn. No file is created and no migration runs — a missing
    /// database is an error, never an empty fleet.
    pub fn open_read_only(path: &Path) -> Result<Store, StoreError> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        let found: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'database_schema'",
            [],
            |row| row.get(0),
        )?;
        let found: u32 = found.parse().unwrap_or(0);
        if found != DATABASE_SCHEMA {
            return Err(StoreError::SchemaMismatch { found });
        }
        Ok(Store { conn })
    }

    pub fn create_run(
        &mut self,
        run_id: &str,
        feature: &str,
        bundle_name: &str,
        manifest: &Value,
    ) -> Result<(), StoreError> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT run_id FROM runs WHERE run_id = ?1",
                params![run_id],
                |r| r.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Err(StoreError::RunExists(run_id.to_string()));
        }
        let manifest = serde_json::to_string(manifest)?;
        let created_at = now_rfc3339();
        self.conn.execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, feature, bundle_name, manifest, created_at],
        )?;
        Ok(())
    }

    pub fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT manifest FROM runs WHERE run_id = ?1",
                params![run_id],
                |r| r.get(0),
            )
            .optional()?;
        let raw = raw.ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT run_id, feature, created_at FROM runs ORDER BY created_at")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        let head: Option<(i64, String)> = self
            .conn
            .query_row(
                "SELECT seq, event_hash FROM events WHERE run_id = ?1
                 ORDER BY seq DESC LIMIT 1",
                params![run_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(match head {
            Some((seq, hash)) => (seq as u64, hash),
            None => (0, ZERO_HASH.to_string()),
        })
    }

    /// Build, seal, and durably append the next event in one transaction.
    /// Envelope identity (seq, previous_hash) comes from the journal head
    /// inside the transaction; a concurrent writer conflicts instead of
    /// forking the chain.
    pub fn append_next(
        &mut self,
        run_id: &str,
        event_type: EventType,
        payload: Value,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let head: Option<(i64, String)> = tx
            .query_row(
                "SELECT seq, event_hash FROM events WHERE run_id = ?1
                 ORDER BY seq DESC LIMIT 1",
                params![run_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (last_seq, previous_hash) = match head {
            Some((seq, hash)) => (seq as u64, hash),
            None => (0, ZERO_HASH.to_string()),
        };
        let envelope = EventEnvelope {
            run_id: run_id.to_string(),
            seq: last_seq + 1,
            event_id: uuid::Uuid::new_v4().to_string(),
            event_schema_version: 1,
            event_type,
            payload,
            causation_id,
            correlation_id: run_id.to_string(),
            attempt_id,
            recorded_at: now_rfc3339(),
            previous_hash,
            event_hash: String::new(),
        }
        .sealed();
        let serialized = serde_json::to_string(&envelope)?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO events (run_id, seq, event_hash, envelope)
             VALUES (?1, ?2, ?3, ?4)",
            params![run_id, envelope.seq as i64, envelope.event_hash, serialized],
        )?;
        if inserted == 0 {
            return Err(StoreError::AppendConflict { seq: envelope.seq });
        }
        tx.commit()?;
        Ok(envelope)
    }

    /// Load and verify the full journal of a run. A journal that fails
    /// chain verification is corrupt and is never partially returned.
    pub fn load(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT envelope FROM events WHERE run_id = ?1 ORDER BY seq")?;
        let events = stmt
            .query_map(params![run_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|raw| serde_json::from_str::<EventEnvelope>(&raw))
            .collect::<Result<Vec<_>, _>>()?;
        if events.is_empty() {
            // Distinguish "no such run" from "run without events".
            let _ = self.manifest(run_id)?;
        }
        verify_chain(&events)?;
        Ok(events)
    }

    /// Canonical NDJSON export: one sealed envelope per line, in order.
    pub fn export_ndjson(&self, run_id: &str) -> Result<String, StoreError> {
        let events = self.load(run_id)?;
        let mut out = String::new();
        for event in &events {
            out.push_str(&serde_json::to_string(&serde_json::to_value(event)?)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// The journal head hash — cheap identity for fencing and anchors.
    pub fn head_hash(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        self.head(run_id)
    }
}

/// Verify an exported NDJSON journal offline: parse, chain, fold.
pub fn verify_export(ndjson: &str) -> Result<forge_core::RunState, VerifyError> {
    let events = ndjson
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<EventEnvelope>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(VerifyError::Parse)?;
    verify_chain(&events).map_err(VerifyError::Chain)?;
    forge_core::fold(&events).map_err(VerifyError::Fold)
}

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("parse: {0}")]
    Parse(serde_json::Error),
    #[error("chain: {0}")]
    Chain(ChainError),
    #[error("fold: {0}")]
    Fold(forge_core::FoldError),
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("valid")
        .format(&Rfc3339)
        .expect("rfc3339 formats")
}

#[cfg(test)]
mod tests;
