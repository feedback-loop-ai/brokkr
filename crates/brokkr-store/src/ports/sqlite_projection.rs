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
use std::sync::LazyLock;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::event_index::Indexed;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;

use crate::{damage, import, patiently, StoreError, BUSY_TIMEOUT};

use super::sqlite_schema::{read_user_version, ExpectedSchema, SchemaMatch};

use super::{
    checked_seq, divergent, non_empty_str, normalize_storage_path, open_standalone_store,
    read_verified_log, reject_non_regular_file, reject_symlink, sqlite_sidecar_path,
    started_feature_of, validate_manifest, validate_started_manifests, verify_chain_checked,
    EventLog, ProjectionStore, RunMetadata,
};

const PROJECTION_SCHEMA_VERSION_V1: &str = "1";
const PROJECTION_SCHEMA_VERSION: &str = "2";
const PROJECTION_USER_VERSION: i64 = 2;

const PROJECTION_COMMON_DDL: &str = r#"
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
CREATE INDEX IF NOT EXISTS projected_runs_created_idx
    ON projected_runs (created_at, run_id);
"#;

const PROJECTED_EVENTS_V2_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS projected_events (
    run_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    event_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    attempt_id TEXT,
    member TEXT,
    event_hash TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);
"#;

const PROJECTED_EVENTS_V1_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS projected_events (
    run_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_hash TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);
"#;

const DROP_LEGACY_PROJECTION_DDL: &str = r#"
DROP TABLE IF EXISTS projected_events;
DROP TABLE IF EXISTS projected_runs;
DROP TABLE IF EXISTS projection_meta;
"#;

fn init_projection_tables(conn: &Connection, events_ddl: &str) -> Result<(), StoreError> {
    conn.execute_batch(PROJECTION_COMMON_DDL)?;
    conn.execute_batch(events_ddl)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn seed_legacy_v1_schema(conn: &Connection, user_version: i64) {
    init_projection_tables(conn, PROJECTED_EVENTS_V1_DDL).expect("seed v1 tables");
    conn.execute(UPSERT_META_SQL, params![PROJECTION_SCHEMA_VERSION_V1])
        .expect("seed v1 meta");
    conn.pragma_update(None, "user_version", user_version)
        .expect("seed v1 user_version");
}

static EXPECTED_PROJECTION_SCHEMA: LazyLock<ExpectedSchema> = LazyLock::new(|| {
    ExpectedSchema::from_initializer(|conn| {
        init_projection_tables(conn, PROJECTED_EVENTS_V2_DDL).expect("canonical projection DDL");
    })
});

static LEGACY_PROJECTION_SCHEMA_V1: LazyLock<ExpectedSchema> = LazyLock::new(|| {
    ExpectedSchema::from_initializer(|conn| {
        init_projection_tables(conn, PROJECTED_EVENTS_V1_DDL)
            .expect("canonical legacy v1 projection DDL");
    })
});

const INSERT_PROJECTED_EVENT_SQL: &str = "INSERT INTO projected_events
         (run_id, seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
const UPSERT_PROJECTED_RUN_SQL: &str =
    "INSERT INTO projected_runs (run_id, feature, created_at, head_seq, head_hash)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT (run_id) DO UPDATE SET
         feature = excluded.feature,
         created_at = excluded.created_at,
         head_seq = excluded.head_seq,
         head_hash = excluded.head_hash";
const UPSERT_META_SQL: &str =
    "INSERT OR REPLACE INTO projection_meta (key, value) VALUES ('projection_schema', ?1)";
const LIST_PROJECTED_RUNS_SQL: &str =
    "SELECT run_id, feature, created_at FROM projected_runs ORDER BY created_at, rowid";
const SELECT_PREFIX_ROWS_SQL: &str =
    "SELECT seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload
     FROM projected_events
     WHERE run_id = ?1 AND seq <= ?2
     ORDER BY seq";
const SELECT_OUT_OF_RANGE_EVENTS_SQL: &str =
    "SELECT COUNT(*) FROM projected_events WHERE run_id = ?1 AND (seq < 1 OR seq > ?2)";
const SELECT_EVENT_HASH_AT_SQL: &str =
    "SELECT event_hash FROM projected_events WHERE run_id = ?1 AND seq = ?2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionSchemaState {
    Fresh,
    UpgradeMeta,
    UpgradeLegacyV1,
    Ready,
}

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
        let (conn, owned) = open_standalone_store(
            path.as_ref(),
            "projection_migrate",
            |conn, normalized| {
                let _ = inspect_projection_schema(conn, normalized)?;
                Ok(())
            },
            |_| Ok(()),
            ensure_schema,
        )?;
        Ok(Self {
            conn,
            path: owned,
            patience: BUSY_TIMEOUT,
        })
    }

    /// Inspect an existing SQLite projection database strictly untouched in
    /// read-only mode (decision 0119 ruling 3), performing no WAL switch, no
    /// `-wal`/`-shm` sidecar creation, no schema creation, and no upgrade.
    pub fn inspect_read_only(path: impl AsRef<Path>) -> Result<(), StoreError> {
        let normalized = normalize_storage_path(path.as_ref())?;
        inspect_read_only_inner(&normalized).map_err(|err| damage::classified(&normalized, err))
    }

    /// Resolve a run's `(feature, created_at)` listing metadata using the
    /// single projection precedence rule: once `envelopes` is non-empty,
    /// `RunStarted` > `manifest` `feature` and `envelopes[0].recorded_at`
    /// always win; `fallback` (`run_meta.json`) is only used when `envelopes`
    /// is empty.
    pub fn resolve_listing<'a>(
        envelopes: &'a [EventEnvelope],
        manifest: Option<&'a Value>,
        fallback: Option<&'a RunMetadata>,
    ) -> (&'a str, &'a str) {
        resolve_listing(envelopes, manifest, None, fallback)
    }

    /// Return the path to this projection database file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

struct ProjectedEventRow {
    seq: i64,
    event_id: String,
    event_type: String,
    attempt_id: Option<String>,
    member: Option<String>,
    event_hash: String,
    recorded_at: String,
    payload: String,
}

fn inspect_read_only_inner(normalized: &Path) -> Result<(), StoreError> {
    let _ = reject_non_regular_file(normalized)?;
    let _ = reject_symlink(&sqlite_sidecar_path(normalized, "-wal"))?;
    let _ = reject_symlink(&sqlite_sidecar_path(normalized, "-shm"))?;
    crate::untouched::read_connection_untouched(normalized, |conn| {
        conn.busy_timeout(BUSY_TIMEOUT)?;
        patiently("projection_inspect", BUSY_TIMEOUT, normalized, || {
            let tx = Transaction::new_unchecked(&conn, TransactionBehavior::Deferred)?;
            let _ = inspect_projection_schema(&tx, normalized)?;
            tx.commit()?;
            Ok(())
        })
    })
}

fn foreign_projection_db(path: &Path) -> StoreError {
    StoreError::ForeignDatabase {
        path: path.to_path_buf(),
        expected: "projection index",
    }
}

fn read_projection_schema_version(conn: &Connection) -> Result<Option<String>, StoreError> {
    Ok(conn
        .query_row(
            "SELECT value FROM projection_meta WHERE key = 'projection_schema'",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

fn inspect_legacy_v1_schema(
    conn: &Connection,
    path: &Path,
    user_version: i64,
) -> Result<ProjectionSchemaState, StoreError> {
    if LEGACY_PROJECTION_SCHEMA_V1.matches(conn)? != SchemaMatch::Exact {
        return Err(foreign_projection_db(path));
    }
    let version = read_projection_schema_version(conn)?;
    if version.as_deref() == Some(PROJECTION_SCHEMA_VERSION_V1) && matches!(user_version, 0 | 1) {
        Ok(ProjectionSchemaState::UpgradeLegacyV1)
    } else {
        Err(foreign_projection_db(path))
    }
}

fn inspect_projection_schema(
    conn: &Connection,
    path: &Path,
) -> Result<ProjectionSchemaState, StoreError> {
    let user_version = read_user_version(conn)?;
    match EXPECTED_PROJECTION_SCHEMA.matches(conn)? {
        SchemaMatch::Empty if user_version == 0 => Ok(ProjectionSchemaState::Fresh),
        SchemaMatch::Empty => Err(foreign_projection_db(path)),
        SchemaMatch::Foreign => inspect_legacy_v1_schema(conn, path, user_version),
        SchemaMatch::Exact => match (
            read_projection_schema_version(conn)?.as_deref(),
            user_version,
        ) {
            (Some(PROJECTION_SCHEMA_VERSION), PROJECTION_USER_VERSION) => {
                Ok(ProjectionSchemaState::Ready)
            }
            (Some(PROJECTION_SCHEMA_VERSION), 0) | (Some(PROJECTION_SCHEMA_VERSION_V1), 0 | 1) => {
                Ok(ProjectionSchemaState::UpgradeMeta)
            }
            _ => Err(foreign_projection_db(path)),
        },
    }
}

fn ensure_schema(conn: &Connection, path: &Path) -> Result<(), StoreError> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    match inspect_projection_schema(&tx, path)? {
        ProjectionSchemaState::Ready => {
            tx.commit()?;
            return Ok(());
        }
        ProjectionSchemaState::Fresh => init_projection_tables(&tx, PROJECTED_EVENTS_V2_DDL)?,
        ProjectionSchemaState::UpgradeMeta => {}
        ProjectionSchemaState::UpgradeLegacyV1 => {
            tx.execute_batch(DROP_LEGACY_PROJECTION_DDL)?;
            init_projection_tables(&tx, PROJECTED_EVENTS_V2_DDL)?;
        }
    }
    tx.execute(UPSERT_META_SQL, params![PROJECTION_SCHEMA_VERSION])?;
    tx.pragma_update(None, "user_version", PROJECTION_USER_VERSION)?;
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

fn refuse_out_of_range_events(
    tx: &Connection,
    run_id: &str,
    head_seq: u64,
) -> Result<(), StoreError> {
    let limit = checked_seq(head_seq)?;
    let count: i64 = tx.query_row(
        SELECT_OUT_OF_RANGE_EVENTS_SQL,
        params![run_id, limit],
        |row| row.get(0),
    )?;
    if count > 0 {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_stored_head_event_hash(
    tx: &Connection,
    run_id: &str,
    row: &StoredRun,
) -> Result<(), StoreError> {
    let seq = checked_seq(row.head_seq)?;
    let stored_hash: Option<String> = tx
        .query_row(SELECT_EVENT_HASH_AT_SQL, params![run_id, seq], |r| r.get(0))
        .optional()?;
    if stored_hash.as_deref() != Some(row.head_hash.as_str()) {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_stored_prefix(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<BTreeSet<u64>, StoreError> {
    let limit = checked_seq(envelopes.len() as u64)?;
    let mut stmt = tx.prepare(SELECT_PREFIX_ROWS_SQL)?;
    let rows = stmt.query_map(params![run_id, limit], |row| {
        Ok(ProjectedEventRow {
            seq: row.get(0)?,
            event_id: row.get(1)?,
            event_type: row.get(2)?,
            attempt_id: row.get(3)?,
            member: row.get(4)?,
            event_hash: row.get(5)?,
            recorded_at: row.get(6)?,
            payload: row.get(7)?,
        })
    })?;
    let mut present = BTreeSet::new();
    for row in rows {
        let stored = row?;
        let idx = usize::try_from(stored.seq)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(|| divergent(run_id))?;
        let expected = &envelopes[idx];
        let indexed = Indexed::of(expected);
        let expected_payload = serde_json::to_string(&expected.payload)?;
        let matches = stored.event_id == expected.event_id
            && stored.event_type == expected.event_type.as_str()
            && stored.attempt_id.as_deref() == indexed.attempt_id
            && stored.member.as_deref() == indexed.member
            && stored.event_hash == expected.event_hash
            && stored.recorded_at == expected.recorded_at
            && stored.payload == expected_payload;
        if !matches {
            return Err(divergent(run_id));
        }
        present.insert(stored.seq as u64);
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
        let indexed = Indexed::of(envelope);
        let payload = serde_json::to_string(&envelope.payload)?;
        let args = params![
            run_id,
            seq,
            envelope.event_id,
            envelope.event_type.as_str(),
            indexed.attempt_id,
            indexed.member,
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

fn resolve_listing<'a>(
    envelopes: &'a [EventEnvelope],
    manifest: Option<&'a Value>,
    existing: Option<&'a StoredRun>,
    fallback: Option<&'a RunMetadata>,
) -> (&'a str, &'a str) {
    let manifest_feature = manifest.and_then(|m| non_empty_str(m.get("feature")));
    if let Some(first) = envelopes.first() {
        let feature = started_feature_of(envelopes)
            .or(manifest_feature)
            .unwrap_or("");
        return (feature, first.recorded_at.as_str());
    }
    let existing_opt = existing.filter(|_| manifest.is_none());
    let feature = fallback
        .map(|m| m.feature.as_str())
        .or(manifest_feature)
        .or_else(|| {
            existing_opt
                .map(|r| r.feature.as_str())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("");
    let created_at = fallback
        .map(|m| m.created_at.as_str())
        .or_else(|| manifest.and_then(|m| non_empty_str(m.get("created_at"))))
        .or_else(|| {
            existing_opt
                .map(|r| r.created_at.as_str())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("");
    (feature, created_at)
}

fn maybe_fill_manifest_feature(
    tx: &Connection,
    run_id: &str,
    existing: &StoredRun,
    manifest: Option<&Value>,
) -> Result<(), StoreError> {
    if !existing.feature.is_empty() {
        return Ok(());
    }
    let Some(mf) = manifest.and_then(|m| non_empty_str(m.get("feature"))) else {
        return Ok(());
    };
    upsert_projected_run(
        tx,
        run_id,
        mf,
        &existing.created_at,
        existing.head_seq,
        &existing.head_hash,
    )
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
    if row.head_seq == 0 {
        return Ok(Some(0));
    }
    if row.head_seq > last.seq {
        verify_stored_head_event_hash(tx, run_id, row)?;
        let present = verify_stored_prefix(tx, run_id, envelopes)?;
        if present.len() != envelopes.len() {
            return Err(divergent(run_id));
        }
        return Ok(None);
    }
    if row.head_seq == last.seq {
        if row.head_hash != last.event_hash {
            return Err(divergent(run_id));
        }
        let present = verify_stored_prefix(tx, run_id, envelopes)?;
        repair_missing_events(tx, run_id, envelopes, &present)?;
        return Ok(None);
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
    let stored_head_seq = existing.as_ref().map_or(0, |row| row.head_seq);
    refuse_out_of_range_events(tx, run_id, stored_head_seq)?;
    if let Some(row) = &existing {
        if row.head_seq == 0 && row.head_hash != ZERO_HASH {
            return Err(divergent(run_id));
        }
    }
    let (feature, created_at) = resolve_listing(envelopes, manifest, existing.as_ref(), None);

    let Some(last) = envelopes.last() else {
        if let Some(row) = existing.as_ref().filter(|row| row.head_seq > 0) {
            verify_stored_head_event_hash(tx, run_id, row)?;
            maybe_fill_manifest_feature(tx, run_id, row, manifest)?;
            return Ok(());
        }
        upsert_projected_run(tx, run_id, feature, created_at, 0, ZERO_HASH)?;
        return Ok(());
    };

    let Some(append_from) = reconcile_existing_run(tx, run_id, existing.as_ref(), envelopes, last)?
    else {
        if let Some(row) = existing.as_ref() {
            maybe_fill_manifest_feature(tx, run_id, row, manifest)?;
        }
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
            let mut verified = match read_verified_log(log) {
                Ok(runs) => runs,
                Err(log_err) => return Ok(Err(log_err)),
            };
            verified.sort_by(|a, b| {
                let order_a = a.metadata.as_ref().and_then(|m| m.list_order);
                let order_b = b.metadata.as_ref().and_then(|m| m.list_order);
                order_a.cmp(&order_b).then_with(|| a.run_id.cmp(&b.run_id))
            });
            tx.execute("DELETE FROM projected_events", [])?;
            tx.execute("DELETE FROM projected_runs", [])?;
            let mut total = 0u64;
            for run in &verified {
                let (feature, created_at) = resolve_listing(
                    &run.events,
                    Some(&run.manifest),
                    None,
                    run.metadata.as_ref(),
                );
                let (head_seq, head_hash) = run
                    .events
                    .last()
                    .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
                upsert_projected_run(&tx, &run.run_id, feature, created_at, head_seq, head_hash)?;
                insert_projected_events(&tx, &run.run_id, &run.events)?;
                total += run.events.len() as u64;
            }
            tx.commit()?;
            Ok(Ok(total))
        })?
    }
}
