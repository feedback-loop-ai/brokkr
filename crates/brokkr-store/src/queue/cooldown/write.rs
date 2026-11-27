//! Shared INSERT and idempotent conflict-verification helpers for
//! `provider_cooldowns` (decision 0071 ruling 5; decision 0126 ruling 3).

use rusqlite::{params, Connection, Row};

use crate::StoreError;

/// One `provider_cooldowns` row as the table's one writer spells it:
/// shared by an attempt's rows and a probe's later round, so the INSERT
/// lives once (decision 0071 ruling 5).
pub(super) struct StoredRow<'a> {
    pub(super) run_id: &'a str,
    pub(super) event_seq: u64,
    pub(super) site: &'a str,
    pub(super) provider: &'a str,
    pub(super) route: &'a str,
    pub(super) until: i64,
    pub(super) reason: &'a str,
    pub(super) retry_after_seconds: Option<u64>,
    pub(super) rounds: u64,
    pub(super) recorded: i64,
}

const INSERT_COOLDOWN_SQL: &str = "INSERT INTO provider_cooldowns
            (run_id, event_seq, site, provider, route, until_at, reason,
             retry_after_seconds, rounds, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)";

const SELECT_COOLDOWN_CONFLICT_SQL: &str =
    "SELECT provider, route, until_at, reason, retry_after_seconds, recorded_at
     FROM provider_cooldowns
     WHERE run_id = ?1 AND event_seq = ?2 AND site = ?3 AND rounds = ?4";

type ConflictFields = (String, String, i64, String, Option<i64>, i64);

fn read_conflict_fields(r: &Row<'_>) -> rusqlite::Result<ConflictFields> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
    ))
}

/// Insert one row. The table's CHECK constraints and unique index judge it.
pub(super) fn insert_row(conn: &Connection, row: &StoredRow<'_>) -> Result<(), StoreError> {
    insert_row_with(conn, row, false)?;
    Ok(())
}

pub(super) fn insert_row_with(
    conn: &Connection,
    row: &StoredRow<'_>,
    idempotent: bool,
) -> Result<usize, StoreError> {
    let sql = if idempotent {
        format!("{INSERT_COOLDOWN_SQL} ON CONFLICT (run_id, event_seq, site, rounds) DO NOTHING")
    } else {
        INSERT_COOLDOWN_SQL.to_string()
    };
    let args = params![
        row.run_id,
        row.event_seq as i64,
        row.site,
        row.provider,
        row.route,
        row.until,
        row.reason,
        row.retry_after_seconds.map(|retry| retry as i64),
        row.rounds as i64,
        row.recorded,
    ];
    Ok(conn.execute(&sql, args)?)
}

pub(super) fn verify_existing_cooldown_matches(
    conn: &Connection,
    row: &StoredRow<'_>,
) -> Result<(), StoreError> {
    let args = params![
        row.run_id,
        row.event_seq as i64,
        row.site,
        row.rounds as i64
    ];
    let existing = conn.query_row(SELECT_COOLDOWN_CONFLICT_SQL, args, read_conflict_fields)?;
    let expected_retry = row.retry_after_seconds.map(|retry| retry as i64);
    let matches = existing.0 == row.provider
        && existing.1 == row.route
        && existing.2 == row.until
        && existing.3 == row.reason
        && existing.4 == expected_retry
        && existing.5 == row.recorded;
    if !matches {
        return Err(StoreError::DivergentEventLog {
            run_id: row.run_id.to_string(),
        });
    }
    Ok(())
}
