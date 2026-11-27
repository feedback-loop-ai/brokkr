//! PostgreSQL DDL, schema compatibility verification, JSONB NUL filtering,
//! and lossless PostgreSQL `TEXT` NUL escaping.

use std::borrow::Cow;
use std::collections::BTreeSet;

use serde_json::Value;

use crate::wire::PgClient;
use crate::PgError;

pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 2;

const SCHEMA_META_DDL: &str = "CREATE TABLE IF NOT EXISTS brokkr_pg_schema_meta (
    component TEXT PRIMARY KEY,
    version INTEGER NOT NULL
)";
const SELECT_TABLE_COLUMNS_SQL: &str = "SELECT column_name
    FROM information_schema.columns
    WHERE table_schema = current_schema() AND table_name = $1";
const SELECT_TABLE_CHECKS_SQL: &str = "SELECT constraint_name
    FROM information_schema.table_constraints
    WHERE table_schema = current_schema()
      AND table_name = $1
      AND constraint_type = 'CHECK'";
const UPSERT_SCHEMA_META_SQL: &str = "INSERT INTO brokkr_pg_schema_meta (component, version)
    VALUES ($1, $2::integer)
    ON CONFLICT (component) DO NOTHING";
const SELECT_SCHEMA_META_SQL: &str =
    "SELECT version FROM brokkr_pg_schema_meta WHERE component = $1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaKind {
    EventLog,
    Projection,
    Coordination,
}

impl SchemaKind {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 3] = [Self::EventLog, Self::Projection, Self::Coordination];

    fn component(self) -> &'static str {
        match self {
            SchemaKind::EventLog => "event_log",
            SchemaKind::Projection => "projection",
            SchemaKind::Coordination => "coordination",
        }
    }

    fn ddl(self) -> &'static [&'static str] {
        match self {
            SchemaKind::EventLog => EVENT_LOG_DDL,
            SchemaKind::Projection => PROJECTION_DDL,
            SchemaKind::Coordination => COORDINATION_DDL,
        }
    }

    pub(crate) fn required_columns(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            SchemaKind::EventLog => &[
                (
                    "brokkr_runs",
                    &[
                        "run_id",
                        "manifest",
                        "manifest_json",
                        "feature",
                        "created_at",
                        "list_order",
                    ],
                ),
                (
                    "brokkr_events",
                    &[
                        "run_id",
                        "seq",
                        "event_type",
                        "event_hash",
                        "prev_hash",
                        "recorded_at",
                        "envelope",
                        "envelope_json",
                    ],
                ),
            ],
            SchemaKind::Projection => &[
                (
                    "brokkr_projected_runs",
                    &[
                        "run_id",
                        "feature",
                        "bundle_name",
                        "created_at",
                        "head_seq",
                        "head_hash",
                        "list_order",
                    ],
                ),
                (
                    "brokkr_projected_events",
                    &[
                        "run_id",
                        "seq",
                        "event_id",
                        "event_type",
                        "attempt_id",
                        "member",
                        "event_hash",
                        "recorded_at",
                        "payload",
                        "payload_json",
                    ],
                ),
            ],
            SchemaKind::Coordination => &[
                (
                    "brokkr_queue_entries",
                    &[
                        "id",
                        "payload",
                        "priority",
                        "status",
                        "claimed_by",
                        "claimed_at",
                        "run_id",
                        "added_at",
                    ],
                ),
                ("brokkr_queue_waits", &["entry_id", "awaits", "condition"]),
                (
                    "brokkr_provider_cooldowns",
                    &[
                        "seq",
                        "run_id",
                        "event_seq",
                        "site",
                        "provider",
                        "route",
                        "until_at",
                        "reason",
                        "retry_after_seconds",
                        "rounds",
                        "recorded_at",
                    ],
                ),
            ],
        }
    }

    pub(crate) fn required_checks(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            SchemaKind::EventLog | SchemaKind::Projection => &[],
            SchemaKind::Coordination => &[(
                "brokkr_provider_cooldowns",
                &[
                    "brokkr_cooldown_event_seq_chk",
                    "brokkr_cooldown_retry_after_chk",
                    "brokkr_cooldown_rounds_chk",
                    "brokkr_cooldown_window_chk",
                ],
            )],
        }
    }
}

pub(crate) const EVENT_LOG_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS brokkr_runs (
        run_id TEXT PRIMARY KEY,
        manifest JSONB,
        manifest_json TEXT NOT NULL,
        feature TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT '',
        list_order BIGSERIAL NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS brokkr_events (
        run_id TEXT NOT NULL REFERENCES brokkr_runs(run_id),
        seq BIGINT NOT NULL CHECK (seq >= 1),
        event_type TEXT NOT NULL,
        event_hash TEXT NOT NULL,
        prev_hash TEXT NOT NULL,
        recorded_at TEXT NOT NULL,
        envelope JSONB,
        envelope_json TEXT NOT NULL,
        PRIMARY KEY (run_id, seq)
    )",
];

pub(crate) const PROJECTION_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS brokkr_projected_runs (
        run_id TEXT PRIMARY KEY,
        feature TEXT NOT NULL,
        bundle_name TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        head_seq BIGINT NOT NULL,
        head_hash TEXT NOT NULL,
        list_order BIGSERIAL NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS brokkr_projected_runs_created_idx
        ON brokkr_projected_runs (created_at, list_order)",
    "CREATE TABLE IF NOT EXISTS brokkr_projected_events (
        run_id TEXT NOT NULL,
        seq BIGINT NOT NULL,
        event_id TEXT NOT NULL,
        event_type TEXT NOT NULL,
        attempt_id TEXT,
        member TEXT,
        event_hash TEXT NOT NULL,
        recorded_at TEXT NOT NULL,
        payload JSONB,
        payload_json TEXT NOT NULL,
        PRIMARY KEY (run_id, seq)
    )",
];

pub(crate) const COORDINATION_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS brokkr_queue_entries (
        id BIGSERIAL PRIMARY KEY,
        payload TEXT NOT NULL,
        priority BIGINT NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'pending',
        claimed_by TEXT,
        claimed_at TEXT,
        run_id TEXT,
        added_at TEXT NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS brokkr_queue_entries_dispatch_idx
        ON brokkr_queue_entries (status, priority DESC, id ASC)",
    "CREATE TABLE IF NOT EXISTS brokkr_queue_waits (
        entry_id BIGINT NOT NULL REFERENCES brokkr_queue_entries(id),
        awaits BIGINT NOT NULL REFERENCES brokkr_queue_entries(id),
        condition TEXT NOT NULL,
        PRIMARY KEY (entry_id, awaits)
    )",
    "CREATE TABLE IF NOT EXISTS brokkr_provider_cooldowns (
        seq BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL,
        event_seq BIGINT NOT NULL CONSTRAINT brokkr_cooldown_event_seq_chk CHECK (event_seq >= 1),
        site TEXT NOT NULL,
        provider TEXT NOT NULL,
        route TEXT NOT NULL,
        until_at BIGINT NOT NULL,
        reason TEXT NOT NULL,
        retry_after_seconds BIGINT CONSTRAINT brokkr_cooldown_retry_after_chk CHECK (retry_after_seconds IS NULL OR retry_after_seconds > 0),
        rounds BIGINT NOT NULL CONSTRAINT brokkr_cooldown_rounds_chk CHECK (rounds >= 1),
        recorded_at BIGINT NOT NULL,
        CONSTRAINT brokkr_cooldown_window_chk CHECK (until_at > recorded_at),
        UNIQUE (run_id, event_seq, site, rounds)
    )",
    "CREATE INDEX IF NOT EXISTS brokkr_provider_cooldowns_until_idx
        ON brokkr_provider_cooldowns (until_at, seq)",
];

pub(crate) fn init_schema(client: &mut PgClient, kind: SchemaKind) -> Result<(), PgError> {
    client.in_transaction(
        |tx| {
            tx.query(SCHEMA_META_DDL, &[])?;
            for sql in kind.ddl() {
                tx.query(sql, &[])?;
            }
            verify_catalog_names(
                tx,
                SELECT_TABLE_COLUMNS_SQL,
                kind.required_columns(),
                "column",
            )?;
            verify_catalog_names(
                tx,
                SELECT_TABLE_CHECKS_SQL,
                kind.required_checks(),
                "CHECK constraint",
            )?;
            verify_schema_version(tx, kind.component())
        },
        |err| err,
    )
}

fn verify_catalog_names(
    tx: &mut PgClient,
    sql: &str,
    required: &[(&str, &[&str])],
    item_kind: &str,
) -> Result<(), PgError> {
    for &(table, names) in required {
        let res = tx.query(sql, &[Some(table)])?;
        let mut present = BTreeSet::new();
        for row in &res.rows {
            present.insert(row.get_str(0)?);
        }
        for &name in names {
            if !present.contains(name) {
                return Err(PgError::IncompatibleSchema(format!(
                    "table '{table}' missing required {item_kind} '{name}'"
                )));
            }
        }
    }
    Ok(())
}

fn verify_schema_version(tx: &mut PgClient, component: &str) -> Result<(), PgError> {
    let version = CURRENT_SCHEMA_VERSION.to_string();
    let params = [Some(component), Some(version.as_str())];
    tx.query(UPSERT_SCHEMA_META_SQL, &params)?;
    let res = tx.query(SELECT_SCHEMA_META_SQL, &[Some(component)])?;
    let found = res.rows.first().map(|row| row.get_i64(0)).transpose()?;
    if found != Some(CURRENT_SCHEMA_VERSION) {
        return Err(PgError::IncompatibleSchema(format!(
            "component '{component}' schema version {found:?} does not match expected {CURRENT_SCHEMA_VERSION}"
        )));
    }
    Ok(())
}

/// Losslessly escape `\` and NUL (`\0`) bytes before binding arbitrary
/// JSON-derived strings to PostgreSQL `TEXT` parameters (PostgreSQL rejects
/// raw `0x00` with SQLSTATE `22P05`).
pub(crate) fn encode_pg_text(raw: &str) -> Cow<'_, str> {
    if !raw.contains(['\0', '\\']) {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len() + 4);
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            other => out.push(other),
        }
    }
    Cow::Owned(out)
}

/// Inverse of [`encode_pg_text`] for PostgreSQL `TEXT` columns read back from
/// the database.
pub(crate) fn decode_pg_text(encoded: &str) -> Result<String, PgError> {
    if !encoded.contains('\\') {
        return Ok(encoded.to_string());
    }
    let mut out = String::with_capacity(encoded.len());
    let mut chars = encoded.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('0') => out.push('\0'),
                _ => {
                    return Err(PgError::Protocol(
                        "malformed NUL-escape sequence in PostgreSQL text column".into(),
                    ));
                }
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

/// Return `Some(raw_json)` for PostgreSQL `JSONB` columns when `value` contains
/// no `\u0000` (which PostgreSQL `JSONB` refuses with SQLSTATE 22P05), or `None`
/// so the `JSONB` column stays `NULL` while the canonical `*_json TEXT` column
/// preserves the exact bytes.
pub(crate) fn jsonb_param<'a>(value: &Value, raw_json: &'a str) -> Option<&'a str> {
    (!value_contains_nul(value)).then_some(raw_json)
}

fn value_contains_nul(value: &Value) -> bool {
    match value {
        Value::String(s) => s.contains('\0'),
        Value::Array(items) => items.iter().any(value_contains_nul),
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| k.contains('\0') || value_contains_nul(v)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}
