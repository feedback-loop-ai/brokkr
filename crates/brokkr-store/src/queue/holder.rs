//! The journal's dispatchers and its dispatch settings (decision 0095
//! rulings 2 and 7). A dispatcher that takes the journal's lock appends a
//! holder row, its generation the next, so the latest row names the one
//! dispatcher whose steps land: each step of a start from the queue is
//! journaled only under that generation, read in the step's own
//! transaction, and a dispatcher a newer one fenced out writes nothing.
//! The lock, and so liveness, is the runtime's; nothing here reads a clock
//! for it, and the time a holder records is for the operator.
//!
//! The bound on the refusals counted against an entry is a setting of the
//! journal: one row per command that set it, the latest in force, and 3
//! where none was ever set. A row that does not read as a bound of at
//! least 1 refuses its reader, since no default stands in for a bound
//! that is set.

use std::num::NonZeroU32;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use thiserror::Error;

use super::claim::{in_step, Generation};
use super::storage::queue_stored;
use super::{Attribution, QueueRefusal};
use crate::{now_rfc3339, patiently, Store, StoreError};

/// The bound of a journal whose operator never set one (the operator's
/// 2026-10-23 ruling of ask 8: "Configurable, default 3").
const DEFAULT_BOUND: NonZeroU32 = NonZeroU32::new(3).unwrap();

/// A dispatcher as its holder row records it: its generation, its process
/// and when it took the journal.
type Holder = (Generation, u32, String);

/// A bound as `dispatch_settings` holds it: a whole number of at least 1
/// that a `u32` holds, or refused.
struct Bound(NonZeroU32);

impl FromSql for Bound {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let whole = value.as_i64().ok().and_then(|n| u32::try_from(n).ok());
        let bound = whole.and_then(NonZeroU32::new).map(Bound);
        bound.ok_or_else(|| FromSqlError::Other(Box::new(UnreadBound)))
    }
}

/// The journal's latest dispatch setting does not read as a bound.
#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "the journal's latest dispatch setting does not hold a whole number of at least 1, so its \
     bound on counted refusals is unreadable, and no default stands in for it (decision 0095)"
)]
pub(crate) struct UnreadBound;

/// Refuse a step taken under `generation` unless it is the journal's
/// latest holder, as `tx` reads it.
pub(super) fn latest(tx: &Transaction<'_>, generation: Generation) -> Result<(), StoreError> {
    let query = "SELECT MAX(generation) FROM dispatch_holders";
    let latest: Option<i64> = tx.query_row(query, [], |row| row.get(0))?;
    match latest == Some(generation.0) {
        true => Ok(()),
        false => Err(QueueRefusal::Superseded(generation).into()),
    }
}

fn hold_in(tx: &Transaction<'_>, host: Option<&str>, pid: u32) -> Result<Generation, StoreError> {
    tx.execute(
        "INSERT INTO dispatch_holders (host, pid, started_at) VALUES (?1, ?2, ?3)",
        params![host, pid, now_rfc3339()],
    )?;
    Ok(Generation(tx.last_insert_rowid()))
}

fn holder_now(conn: &Connection) -> Result<Option<Holder>, StoreError> {
    queue_stored(conn)?;
    let query = "SELECT generation, pid, started_at FROM dispatch_holders
        ORDER BY generation DESC LIMIT 1";
    let holder = conn.query_row(query, [], |row| {
        Ok((Generation(row.get(0)?), row.get(1)?, row.get(2)?))
    });
    Ok(holder.optional()?)
}

fn bound_now(conn: &Connection) -> Result<NonZeroU32, StoreError> {
    queue_stored(conn)?;
    let query = "SELECT bound FROM dispatch_settings ORDER BY seq DESC LIMIT 1";
    let set: Option<Bound> = conn.query_row(query, [], |row| row.get(0)).optional()?;
    Ok(set.map_or(DEFAULT_BOUND, |Bound(bound)| bound))
}

fn set_bound_in(
    tx: &Transaction<'_>,
    bound: NonZeroU32,
    by: Attribution<'_>,
) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO dispatch_settings (bound, operator, reason, recorded_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![bound.get(), by.operator, by.reason, now_rfc3339()],
    )?;
    Ok(())
}

impl Store {
    /// Record process `pid`, on this host, as the journal's dispatcher, by
    /// a holder row of the next generation, which this returns: the one
    /// every step it takes is journaled under. Called holding the
    /// journal's dispatch lock (decision 0095 ruling 2); a dispatcher whose
    /// generation is no longer the latest writes nothing after.
    pub fn dispatch_hold(&mut self, pid: u32) -> Result<Generation, StoreError> {
        // Asked before the write lock is taken, never under it.
        let host = crate::origin::local_host();
        let conn = &mut self.conn;
        patiently("dispatch_hold", self.patience, || {
            in_step(conn, |tx| hold_in(tx, host.as_deref(), pid))
        })
    }

    /// The journal's latest dispatcher, as its holder row records it: its
    /// generation, its pid and when it took the journal; `None` where no
    /// dispatcher ever took it.
    pub fn dispatch_holder(&self) -> Result<Option<Holder>, StoreError> {
        patiently("dispatch_holder", self.patience, || holder_now(&self.conn))
    }

    /// The journal's bound on the refusals counted against an entry: the
    /// one its latest setting holds, or 3 where none was ever set. A
    /// setting that does not read as a bound of at least 1 is refused.
    pub fn dispatch_bound(&self) -> Result<NonZeroU32, StoreError> {
        patiently("dispatch_bound", self.patience, || bound_now(&self.conn))
    }

    /// Set the journal's bound on counted refusals to `bound`, journaling
    /// the setting with its operator and reason; it governs from the next
    /// pass that reads it.
    pub fn set_dispatch_bound(
        &mut self,
        bound: NonZeroU32,
        by: Attribution<'_>,
    ) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("set_dispatch_bound", self.patience, || {
            in_step(conn, |tx| set_bound_in(tx, bound, by))
        })
    }
}

#[cfg(test)]
mod tests;
