//! PostgreSQL DDL for event log, projection, and coordination tables.

use crate::wire::PgClient;
use crate::PgError;

pub(crate) const EVENT_LOG_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS brokkr_runs (
        run_id TEXT PRIMARY KEY,
        manifest JSONB NOT NULL,
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
        envelope JSONB NOT NULL,
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
        payload JSONB NOT NULL,
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
        retry_after_seconds BIGINT,
        rounds BIGINT NOT NULL CHECK (rounds >= 1),
        recorded_at BIGINT NOT NULL,
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
