//! Shared INSERT and idempotent conflict-verification helpers for
//! `provider_cooldowns` (decision 0071 ruling 5; decision 0126 ruling 3).

use std::collections::BTreeSet;

use rusqlite::{params, Connection, OptionalExtension, Row};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::super::storage::cool_downs_writable;
use super::{until_second, Cooling};
use crate::ports::checked_seq;
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

fn checked_cooldown_i64(field: &'static str, value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::CooldownFieldOutOfRange { field, value })
}

struct CheckedIds {
    event_seq: i64,
    retry_after_seconds: Option<i64>,
    rounds: i64,
}

fn check_row_bounds(row: &StoredRow<'_>) -> Result<CheckedIds, StoreError> {
    let event_seq = checked_seq(row.event_seq)?;
    let retry_after_seconds = row
        .retry_after_seconds
        .map(|retry| checked_cooldown_i64("retry_after_seconds", retry))
        .transpose()?;
    let rounds = checked_cooldown_i64("rounds", row.rounds)?;
    Ok(CheckedIds {
        event_seq,
        retry_after_seconds,
        rounds,
    })
}

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

fn verify_conflict_fields(
    row: &StoredRow<'_>,
    expected_retry: Option<i64>,
    existing: &ConflictFields,
) -> Result<(), StoreError> {
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

pub(super) fn insert_row_with(
    conn: &Connection,
    row: &StoredRow<'_>,
    idempotent: bool,
) -> Result<usize, StoreError> {
    let checked = check_row_bounds(row)?;
    let sql = if idempotent {
        format!("{INSERT_COOLDOWN_SQL} ON CONFLICT (run_id, event_seq, site, rounds) DO NOTHING")
    } else {
        INSERT_COOLDOWN_SQL.to_string()
    };
    let args = params![
        row.run_id,
        checked.event_seq,
        row.site,
        row.provider,
        row.route,
        row.until,
        row.reason,
        checked.retry_after_seconds,
        checked.rounds,
        row.recorded,
    ];
    Ok(conn.execute(&sql, args)?)
}

pub(super) fn verify_existing_cooldown(
    conn: &Connection,
    row: &StoredRow<'_>,
    require_present: bool,
) -> Result<(), StoreError> {
    let checked = check_row_bounds(row)?;
    let args = params![row.run_id, checked.event_seq, row.site, checked.rounds];
    let existing = if require_present {
        Some(conn.query_row(SELECT_COOLDOWN_CONFLICT_SQL, args, read_conflict_fields)?)
    } else {
        conn.query_row(SELECT_COOLDOWN_CONFLICT_SQL, args, read_conflict_fields)
            .optional()?
    };
    if let Some(existing) = existing {
        verify_conflict_fields(row, checked.retry_after_seconds, &existing)?;
    }
    Ok(())
}

/// Idempotently write `rows` as the cool-downs of `(run_id, event_seq)` at
/// `recorded_at`, returning the count of newly inserted rows (decision 0126
/// ruling 3). Already-expired rows (`until <= recorded`) are skipped, matching
/// [`super::insert`], and duplicate `(run_id, event_seq, site, rounds)` rows
/// are ignored without error while CHECK constraints remain enforced.
pub(crate) fn insert_idempotent(
    tx: &Connection,
    run_id: &str,
    event_seq: u64,
    recorded_at: &str,
    rows: &[Cooling],
) -> Result<u64, StoreError> {
    if rows.is_empty() || !cool_downs_writable(tx)? {
        return Ok(0);
    }
    let recorded = OffsetDateTime::parse(recorded_at, &Rfc3339)
        .map_err(|_| StoreError::Clock {
            now: recorded_at.to_string(),
        })?
        .unix_timestamp();
    let mut inserted = 0u64;
    let mut seen_in_batch = BTreeSet::new();
    for row in rows {
        let until = until_second(row.until_at);
        let stored = StoredRow {
            run_id,
            event_seq,
            site: &row.site,
            provider: &row.provider,
            route: &row.route,
            until,
            reason: &row.reason,
            retry_after_seconds: row.retry_after_seconds,
            rounds: row.rounds,
            recorded,
        };
        if !seen_in_batch.insert((row.site.as_str(), row.rounds)) {
            return Err(StoreError::DivergentEventLog {
                run_id: run_id.to_string(),
            });
        }
        if until <= recorded {
            verify_existing_cooldown(tx, &stored, false)?;
            continue;
        }
        let changed = insert_row_with(tx, &stored, true)?;
        if changed == 0 {
            verify_existing_cooldown(tx, &stored, true)?;
        }
        inserted += changed as u64;
    }
    Ok(inserted)
}
