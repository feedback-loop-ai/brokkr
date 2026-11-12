//! A provider's cool-downs in the queue (decision 0103 ruling 2(a), slice
//! 4d1): one append-only `provider_cooldowns` row per refused site and
//! round of an attempt's episode, written in the terminal event's own
//! `Immediate` transaction, read as the aggregate that holds a provider
//! and route, and read again inside a claim's transaction.
//!
//! The table's two times are Unix seconds (UTC), never text: a text time
//! compared as a string is wrong across precisions and offsets, and a
//! `unixepoch(text)` CHECK passes on malformed text because the function
//! returns NULL and a NULL CHECK passes. Only the readers here convert
//! them, into the `time` type the engine already carries.

use rusqlite::{params, Connection, OptionalExtension};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::QueueRefusal;
use crate::{patiently, Store, StoreError};

/// One cool-down a terminal event records, as its writer derives it. The
/// store fills `event_seq` and `recorded_at` from the event it writes the
/// rows with — the seq the event takes inside that transaction and the
/// instant the journal recorded it — so a caller never guesses either, and
/// a row whose end has already passed writes no row at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cooling {
    /// The invocation-site tag, `''` for an inline seat.
    pub site: String,
    pub provider: String,
    pub route: String,
    /// When the wait ends; a row is in force until this instant passes.
    pub until_at: OffsetDateTime,
    /// The provider's bounded words, as the terminal event recorded them.
    pub reason: String,
    /// The provider's own retry-after, when it gave one.
    pub retry_after_seconds: Option<u64>,
    /// Which round of this site's waits within the effect's episode.
    pub rounds: u64,
}

/// One `provider_cooldowns` row as a reader holds it: the cool-down the
/// writer recorded, with the instant the journal recorded it. The run and
/// the event seq are the row's own identity and are not carried here; a
/// caller that needs them reads the table by its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cooldown {
    pub site: String,
    pub provider: String,
    pub route: String,
    pub until_at: OffsetDateTime,
    pub reason: String,
    pub retry_after_seconds: Option<u64>,
    pub rounds: u64,
    pub recorded_at: OffsetDateTime,
}

/// The aggregate one provider and route stands at, at an instant: the
/// latest `until_at` across every unexpired row — a monotonic MAX, so a
/// later, shorter refusal never shortens a wait — and the words of the row
/// that ends last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownAggregate {
    pub until: OffsetDateTime,
    pub reason: String,
}

/// Write `rows` as the cool-downs of `run_id`'s event `event_seq`, inside
/// the transaction that wrote that event, which the journal recorded at
/// `recorded_at`. A row whose end has already passed is history and is
/// written not at all; a row that violates the table's CHECK constraints
/// or its unique index fails the whole transaction, so the event and every
/// row land together or not at all.
pub(crate) fn insert(
    tx: &Connection,
    run_id: &str,
    event_seq: u64,
    recorded_at: &str,
    rows: &[Cooling],
) -> Result<(), StoreError> {
    let recorded = OffsetDateTime::parse(recorded_at, &Rfc3339)
        .expect("the journal's own recorded_at is RFC 3339")
        .unix_timestamp();
    let mut insert = tx.prepare(
        "INSERT INTO provider_cooldowns
            (run_id, event_seq, site, provider, route, until_at, reason,
             retry_after_seconds, rounds, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for row in rows {
        let until = row.until_at.unix_timestamp();
        if until <= recorded {
            continue;
        }
        insert.execute(params![
            run_id,
            event_seq as i64,
            row.site,
            row.provider,
            row.route,
            until,
            row.reason,
            row.retry_after_seconds.map(|retry| retry as i64),
            row.rounds as i64,
            recorded,
        ])?;
    }
    Ok(())
}

/// Every row in force at `now`, in the order it was written: the claim's
/// judge and the aggregate share this reading. A row whose stored instant
/// this reader cannot spell refuses by name, carrying the row's `seq`.
pub(super) fn in_force(conn: &Connection, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    let mut statement = conn.prepare(
        "SELECT seq, site, provider, route, until_at, reason, retry_after_seconds, rounds,
                recorded_at
         FROM provider_cooldowns WHERE until_at > ?1 ORDER BY seq",
    )?;
    let raw = statement.query_map([now], Raw::read)?;
    let rows = raw.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(Raw::typed).collect()
}

/// A row's columns as SQLite hands them over, before its instants are
/// turned into the `time` type: kept apart so a row this reader cannot
/// name refuses by its `seq` ([`time_of`]) instead of being guessed at.
struct Raw {
    seq: i64,
    site: String,
    provider: String,
    route: String,
    until: i64,
    reason: String,
    retry_after: Option<i64>,
    rounds: i64,
    recorded: i64,
}

impl Raw {
    fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Raw> {
        Ok(Raw {
            seq: row.get(0)?,
            site: row.get(1)?,
            provider: row.get(2)?,
            route: row.get(3)?,
            until: row.get(4)?,
            reason: row.get(5)?,
            retry_after: row.get(6)?,
            rounds: row.get(7)?,
            recorded: row.get(8)?,
        })
    }

    fn typed(self) -> Result<Cooldown, StoreError> {
        Ok(Cooldown {
            site: self.site,
            provider: self.provider,
            route: self.route,
            until_at: time_of(self.until, self.seq)?,
            reason: self.reason,
            retry_after_seconds: self.retry_after.map(|retry| retry as u64),
            rounds: self.rounds as u64,
            recorded_at: time_of(self.recorded, self.seq)?,
        })
    }
}

/// The aggregate `provider` and `route` stand at, at `now`, or `None` when
/// no row is in force: every unexpired row holds the lane, and the lane
/// ends at the latest `until_at`.
fn aggregate(
    conn: &Connection,
    provider: &str,
    route: &str,
    now: i64,
) -> Result<Option<CooldownAggregate>, StoreError> {
    let found: Option<(i64, i64, String)> = conn
        .query_row(
            "SELECT until_at, seq, reason FROM provider_cooldowns
             WHERE provider = ?1 AND route = ?2 AND until_at > ?3
             ORDER BY until_at DESC, seq DESC LIMIT 1",
            params![provider, route, now],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((until, seq, reason)) = found else {
        return Ok(None);
    };
    Ok(Some(CooldownAggregate {
        until: time_of(until, seq)?,
        reason,
    }))
}

/// A stored instant this reader can name, or the row's refusal: a whole
/// number of seconds outside `time`'s range is never guessed at.
fn time_of(seconds: i64, seq: i64) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| QueueRefusal::CooldownTime { seq }.into())
}

impl Store {
    /// The aggregate `provider` and `route` stand at, at `now`: every
    /// unexpired row is in force and the wait ends at the latest
    /// `until_at` across them (decision 0103 ruling 2(a)). An expired row
    /// is history and is never deleted; `None` is a route no row holds.
    pub fn cooldown(
        &self,
        provider: &str,
        route: &str,
        now: OffsetDateTime,
    ) -> Result<Option<CooldownAggregate>, StoreError> {
        patiently("cooldown", self.patience, || {
            aggregate(&self.conn, provider, route, now.unix_timestamp())
        })
    }

    /// Every cool-down row in force at `now`, in the order it was written:
    /// the rows the claim's judge reads inside its transaction.
    pub fn cooldowns_in_force(&self, now: OffsetDateTime) -> Result<Vec<Cooldown>, StoreError> {
        patiently("cooldowns_in_force", self.patience, || {
            in_force(&self.conn, now.unix_timestamp())
        })
    }
}

#[cfg(test)]
mod tests;
