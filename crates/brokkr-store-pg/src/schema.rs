//! PostgreSQL DDL, schema compatibility verification, JSONB NUL filtering,
//! and lossless PostgreSQL `TEXT` NUL escaping.

use std::borrow::Cow;
use std::collections::BTreeMap;

use serde_json::Value;

use crate::wire::PgClient;
use crate::PgError;

pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 3;

/// Fixed 64-bit PostgreSQL transaction advisory lock key (`"brokschm"`)
/// serializing concurrent first-open schema initialization.
const SCHEMA_INIT_LOCK_KEY: i64 = 0x6272_6f6b_7363_686d;
const ACQUIRE_SCHEMA_LOCK_SQL: &str = "SELECT pg_advisory_xact_lock($1::bigint)";
const SCHEMA_META_DDL: &str = "CREATE TABLE IF NOT EXISTS brokkr_pg_schema_meta (
    component TEXT PRIMARY KEY,
    version INTEGER NOT NULL
)";
const SELECT_TABLE_COLUMNS_SQL: &str = "SELECT column_name, data_type, is_nullable
    FROM information_schema.columns
    WHERE table_schema = current_schema() AND table_name = $1";
const SELECT_TABLE_CHECKS_SQL: &str = "SELECT c.conname, pg_get_constraintdef(c.oid)
    FROM pg_constraint c
    JOIN pg_class t ON t.oid = c.conrelid
    JOIN pg_namespace n ON n.oid = t.relnamespace
    WHERE n.nspname = current_schema()
      AND t.relname = $1
      AND c.contype = 'c'";
const UPSERT_SCHEMA_META_SQL: &str = "INSERT INTO brokkr_pg_schema_meta (component, version)
    VALUES ($1, $2::integer)
    ON CONFLICT (component) DO NOTHING";
const SELECT_SCHEMA_META_SQL: &str =
    "SELECT version FROM brokkr_pg_schema_meta WHERE component = $1";
const SELECT_CURRENT_SCHEMA_SQL: &str = "SELECT current_schema()";
const SELECT_LEGACY_INDEX_SQL: &str = "SELECT 1
    FROM pg_index ix
    JOIN pg_class i ON i.oid = ix.indexrelid
    JOIN pg_class t ON t.oid = ix.indrelid
    JOIN pg_namespace n ON n.oid = i.relnamespace
    WHERE n.nspname = $1
      AND t.relnamespace = n.oid
      AND t.relname = $2
      AND i.relname = $3";

#[derive(Clone, Copy)]
pub(crate) enum PgColType {
    Text,
    BigInt,
    BigSerial,
    Jsonb,
}

impl PgColType {
    fn ddl_type(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::BigInt => "BIGINT",
            Self::BigSerial => "BIGSERIAL",
            Self::Jsonb => "JSONB",
        }
    }

    pub(crate) fn information_schema_type(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::BigInt | Self::BigSerial => "bigint",
            Self::Jsonb => "jsonb",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ColumnSpec {
    pub(crate) name: &'static str,
    pub(crate) col_type: PgColType,
    pub(crate) nullable: bool,
    pub(crate) extras: &'static str,
}

macro_rules! col {
    ($name:expr, $ty:ident, $nullable:expr, $extras:expr) => {
        ColumnSpec {
            name: $name,
            col_type: PgColType::$ty,
            nullable: $nullable,
            extras: $extras,
        }
    };
}

impl ColumnSpec {
    pub(crate) fn is_nullable_word(self) -> &'static str {
        if self.nullable {
            "YES"
        } else {
            "NO"
        }
    }

    fn ddl_fragment(self) -> String {
        let mut out = format!("{} {}", self.name, self.col_type.ddl_type());
        if !self.nullable {
            out.push_str(" NOT NULL");
        }
        if !self.extras.is_empty() {
            out.push(' ');
            out.push_str(self.extras);
        }
        out
    }
}

#[derive(Clone, Copy)]
struct LegacyIndexSpec {
    table: &'static str,
    index: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct CheckSpec {
    pub(crate) name: &'static str,
    pub(crate) expr: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct TableSpec {
    pub(crate) name: &'static str,
    pub(crate) columns: &'static [ColumnSpec],
    pub(crate) checks: &'static [CheckSpec],
    pub(crate) table_constraints: &'static [&'static str],
    pub(crate) indexes: &'static [&'static str],
}

impl TableSpec {
    fn create_table_sql(&self) -> String {
        let mut defs: Vec<String> = self.columns.iter().map(|c| c.ddl_fragment()).collect();
        for chk in self.checks {
            defs.push(format!("CONSTRAINT {} CHECK ({})", chk.name, chk.expr));
        }
        for tc in self.table_constraints {
            defs.push((*tc).to_string());
        }
        format!(
            "CREATE TABLE IF NOT EXISTS {} (
    {}
)",
            self.name,
            defs.join(
                ",
    "
            )
        )
    }
}

#[derive(Clone, Copy)]
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

    pub(crate) fn tables(self) -> &'static [TableSpec] {
        match self {
            SchemaKind::EventLog => EVENT_LOG_TABLES,
            SchemaKind::Projection => PROJECTION_TABLES,
            SchemaKind::Coordination => COORDINATION_TABLES,
        }
    }

    fn legacy_indexes(self) -> &'static [LegacyIndexSpec] {
        match self {
            SchemaKind::Projection => &[LegacyIndexSpec {
                table: "brokkr_projected_runs",
                index: "brokkr_projected_runs_created_idx",
            }],
            SchemaKind::EventLog | SchemaKind::Coordination => &[],
        }
    }
}

const EVENT_LOG_TABLES: &[TableSpec] = &[
    TableSpec {
        name: "brokkr_runs",
        columns: &[
            col!("run_id", Text, false, "PRIMARY KEY"),
            col!("manifest", Jsonb, true, ""),
            col!("manifest_json", Text, false, ""),
            col!("feature", Text, false, "DEFAULT ''"),
            col!("created_at", Text, false, "DEFAULT ''"),
            col!("list_order", BigSerial, false, ""),
        ],
        checks: &[],
        table_constraints: &[],
        indexes: &[],
    },
    TableSpec {
        name: "brokkr_events",
        columns: &[
            col!("run_id", Text, false, "REFERENCES brokkr_runs(run_id)"),
            col!("seq", BigInt, false, "CHECK (seq >= 1)"),
            col!("event_type", Text, false, ""),
            col!("event_hash", Text, false, ""),
            col!("prev_hash", Text, false, ""),
            col!("recorded_at", Text, false, ""),
            col!("envelope", Jsonb, true, ""),
            col!("envelope_json", Text, false, ""),
        ],
        checks: &[],
        table_constraints: &["PRIMARY KEY (run_id, seq)"],
        indexes: &[],
    },
];

const PROJECTION_TABLES: &[TableSpec] = &[
    TableSpec {
        name: "brokkr_projected_runs",
        columns: &[
            col!("run_id", Text, false, "PRIMARY KEY"),
            col!("feature", Text, false, ""),
            col!("bundle_name", Text, false, "DEFAULT ''"),
            col!("created_at", Text, false, ""),
            col!("head_seq", BigInt, false, ""),
            col!("head_hash", Text, false, ""),
        ],
        checks: &[],
        table_constraints: &[],
        indexes: &[],
    },
    TableSpec {
        name: "brokkr_projected_events",
        columns: &[
            col!("run_id", Text, false, ""),
            col!("seq", BigInt, false, ""),
            col!("event_id", Text, false, ""),
            col!("event_type", Text, false, ""),
            col!("attempt_id", Text, true, ""),
            col!("member", Text, true, ""),
            col!("event_hash", Text, false, ""),
            col!("recorded_at", Text, false, ""),
            col!("payload", Jsonb, true, ""),
            col!("payload_json", Text, false, ""),
        ],
        checks: &[],
        table_constraints: &["PRIMARY KEY (run_id, seq)"],
        indexes: &[],
    },
];

const COORDINATION_TABLES: &[TableSpec] = &[
    TableSpec {
        name: "brokkr_queue_entries",
        columns: &[
            col!("id", BigSerial, false, "PRIMARY KEY"),
            col!("payload", Text, false, ""),
            col!("priority", BigInt, false, "DEFAULT 0"),
            col!("status", Text, false, "DEFAULT 'pending'"),
            col!("claimed_by", Text, true, ""),
            col!("claimed_at", Text, true, ""),
            col!("run_id", Text, true, ""),
            col!("added_at", Text, false, ""),
        ],
        checks: &[],
        table_constraints: &[],
        indexes: &[
            "CREATE INDEX IF NOT EXISTS brokkr_queue_entries_dispatch_idx
            ON brokkr_queue_entries (status, priority DESC, id ASC)",
        ],
    },
    TableSpec {
        name: "brokkr_queue_waits",
        columns: &[
            col!(
                "entry_id",
                BigInt,
                false,
                "REFERENCES brokkr_queue_entries(id)"
            ),
            col!(
                "awaits",
                BigInt,
                false,
                "REFERENCES brokkr_queue_entries(id)"
            ),
            col!("condition", Text, false, ""),
        ],
        checks: &[],
        table_constraints: &["PRIMARY KEY (entry_id, awaits)"],
        indexes: &[],
    },
    TableSpec {
        name: "brokkr_provider_cooldowns",
        columns: &[
            col!("seq", BigSerial, false, "PRIMARY KEY"),
            col!("run_id", Text, false, ""),
            col!("event_seq", BigInt, false, ""),
            col!("site", Text, false, ""),
            col!("provider", Text, false, ""),
            col!("route", Text, false, ""),
            col!("until_at", BigInt, false, ""),
            col!("reason", Text, false, ""),
            col!("retry_after_seconds", BigInt, true, ""),
            col!("rounds", BigInt, false, ""),
            col!("recorded_at", BigInt, false, ""),
        ],
        checks: &[
            CheckSpec {
                name: "brokkr_cooldown_event_seq_chk",
                expr: "event_seq >= 1",
            },
            CheckSpec {
                name: "brokkr_cooldown_retry_after_chk",
                expr: "retry_after_seconds IS NULL OR retry_after_seconds > 0",
            },
            CheckSpec {
                name: "brokkr_cooldown_rounds_chk",
                expr: "rounds >= 1",
            },
            CheckSpec {
                name: "brokkr_cooldown_window_chk",
                expr: "until_at > recorded_at",
            },
        ],
        table_constraints: &["UNIQUE (run_id, event_seq, site, rounds)"],
        indexes: &[
            "CREATE INDEX IF NOT EXISTS brokkr_provider_cooldowns_until_idx
            ON brokkr_provider_cooldowns (until_at, seq)",
        ],
    },
];

pub(crate) fn init_schema(client: &mut PgClient, kind: SchemaKind) -> Result<(), PgError> {
    client.in_transaction(
        |tx| {
            let lock_key = SCHEMA_INIT_LOCK_KEY.to_string();
            tx.query(ACQUIRE_SCHEMA_LOCK_SQL, &[Some(lock_key.as_str())])?;
            tx.query(SCHEMA_META_DDL, &[])?;
            let tables = kind.tables();
            for table in tables {
                tx.query(&table.create_table_sql(), &[])?;
            }
            for table in tables {
                verify_table_columns(tx, table)?;
                verify_table_checks(tx, table)?;
            }
            for table in tables {
                for index_sql in table.indexes {
                    tx.query(index_sql, &[])?;
                }
            }
            drop_legacy_indexes(tx, kind.legacy_indexes())?;
            verify_schema_version(tx, kind.component())
        },
        |err| err,
    )
}

fn verify_table_columns(tx: &mut PgClient, table: &TableSpec) -> Result<(), PgError> {
    let res = tx.query(SELECT_TABLE_COLUMNS_SQL, &[Some(table.name)])?;
    let mut present = BTreeMap::new();
    for row in &res.rows {
        present.insert(row.get_str(0)?, (row.get_str(1)?, row.get_str(2)?));
    }
    for col in table.columns {
        let Some(&(actual_type, actual_nullable)) = present.get(col.name) else {
            return Err(PgError::IncompatibleSchema(format!(
                "table '{}' missing required column '{}'",
                table.name, col.name
            )));
        };
        let want_type = col.col_type.information_schema_type();
        let want_nullable = col.is_nullable_word();
        if actual_type != want_type || actual_nullable != want_nullable {
            return Err(PgError::IncompatibleSchema(format!(
                "table '{}' column '{}' has ({actual_type}, nullable={actual_nullable}), expected ({want_type}, nullable={want_nullable})",
                table.name, col.name
            )));
        }
    }
    Ok(())
}

fn verify_table_checks(tx: &mut PgClient, table: &TableSpec) -> Result<(), PgError> {
    if table.checks.is_empty() {
        return Ok(());
    }
    let res = tx.query(SELECT_TABLE_CHECKS_SQL, &[Some(table.name)])?;
    let mut present = BTreeMap::new();
    for row in &res.rows {
        present.insert(row.get_str(0)?, row.get_str(1)?);
    }
    for chk in table.checks {
        let Some(&actual_def) = present.get(chk.name) else {
            return Err(PgError::IncompatibleSchema(format!(
                "table '{}' missing required CHECK constraint '{}'",
                table.name, chk.name
            )));
        };
        let actual_norm = normalize_check_def(actual_def);
        let expected_norm = normalize_check_expr(chk.expr);
        if actual_norm.as_deref() != Some(expected_norm.as_str()) {
            return Err(PgError::IncompatibleSchema(format!(
                "table '{}' CHECK constraint '{}' has definition '{actual_def}', expected 'CHECK ({})'",
                table.name, chk.name, chk.expr
            )));
        }
    }
    Ok(())
}

fn normalize_check_def(def: &str) -> Option<String> {
    let trimmed = def.trim();
    let rest = trimmed
        .get(..5)
        .filter(|prefix| prefix.eq_ignore_ascii_case("check"))
        .map(|_| &trimmed[5..])?;
    Some(normalize_check_expr(rest))
}

fn normalize_check_expr(expr: &str) -> String {
    expr.replace(['(', ')'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn drop_legacy_indexes(
    tx: &mut PgClient,
    legacy_indexes: &[LegacyIndexSpec],
) -> Result<(), PgError> {
    if legacy_indexes.is_empty() {
        return Ok(());
    }
    let schema = current_schema(tx)?;
    let quoted_schema = quote_pg_ident(&schema);
    for legacy in legacy_indexes {
        let params = [
            Some(schema.as_str()),
            Some(legacy.table),
            Some(legacy.index),
        ];
        let found = tx.query(SELECT_LEGACY_INDEX_SQL, &params)?;
        if !found.rows.is_empty() {
            let sql = format!(
                "DROP INDEX IF EXISTS {quoted_schema}.{}",
                quote_pg_ident(legacy.index)
            );
            tx.query(&sql, &[])?;
        }
    }
    Ok(())
}

fn current_schema(tx: &mut PgClient) -> Result<String, PgError> {
    let res = tx.query(SELECT_CURRENT_SCHEMA_SQL, &[])?;
    let schema = res
        .rows
        .first()
        .map(|row| row.get_opt_str(0))
        .transpose()?
        .flatten()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            PgError::IncompatibleSchema("no current PostgreSQL schema selected".into())
        })?;
    Ok(schema.to_string())
}

fn quote_pg_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
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
