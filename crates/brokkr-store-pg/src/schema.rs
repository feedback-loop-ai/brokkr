//! PostgreSQL DDL for event log, projection, and coordination tables.

use serde_json::Value;

use crate::wire::PgClient;
use crate::PgError;

pub(crate) const EVENT_LOG_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS brokkr_runs (
        run_id TEXT PRIMARY KEY,
        manifest JSONB,
        manifest_json TEXT NOT NULL,
        feature TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT ''
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
        head_hash TEXT NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS brokkr_projected_runs_created_idx
        ON brokkr_projected_runs (created_at, run_id)",
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
        event_seq BIGINT NOT NULL CHECK (event_seq >= 1),
        site TEXT NOT NULL,
        provider TEXT NOT NULL,
        route TEXT NOT NULL,
        until_at BIGINT NOT NULL,
        reason TEXT NOT NULL,
        retry_after_seconds BIGINT CHECK (retry_after_seconds IS NULL OR retry_after_seconds > 0),
        rounds BIGINT NOT NULL CHECK (rounds >= 1),
        recorded_at BIGINT NOT NULL,
        CHECK (until_at > recorded_at),
        UNIQUE (run_id, event_seq, site, rounds)
    )",
    "CREATE INDEX IF NOT EXISTS brokkr_provider_cooldowns_until_idx
        ON brokkr_provider_cooldowns (until_at, seq)",
];

pub(crate) fn init_statements(client: &mut PgClient, stmts: &[&str]) -> Result<(), PgError> {
    client.in_transaction(
        |tx| {
            for sql in stmts {
                tx.query(sql, &[])?;
            }
            Ok(())
        },
        |err| err,
    )
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
