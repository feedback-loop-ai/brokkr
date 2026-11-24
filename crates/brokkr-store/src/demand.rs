//! What admission reads of a journal to size an entry by what its recipe
//! used (decision 0125): the untouched open of another journal the realms
//! map lists, the usage its recipes' latest runs recorded under
//! `effect-usage/v1`, and the runs that have no terminal event.
//!
//! The open writes nothing to the journal's directory. A journal no
//! connection holds has no `-wal`, and is read through SQLite's
//! `immutable` URI, which takes no lock and makes no `-wal` or `-shm`;
//! one a connection holds, or a reader left its side files beside, is
//! read through [`Store::open_read_only`] over the side files already
//! there. Neither migrates, creates, leases or locks anything.
//!
//! The history is one bounded statement per journal: the latest
//! `candidates` ended runs of each recipe by `created_at`, and of their events
//! only the `phase/entered` rows and the endings carrying a `usage`
//! object, each prefiltered by its text and then read through
//! `json_valid`, so an envelope no reader parses is never extracted.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags};

use crate::{damage, literal, patiently, Store, StoreError};

/// One event of a recipe's recent run that sizes it: its run, the recipe
/// it started under and when, and the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRow {
    pub run: String,
    pub recipe: String,
    pub created_at: String,
    pub event: UsageEvent,
}

/// A `phase/entered` and the phase it names, or an ending of an attempt
/// and the `usage` object it carries, as its JSON text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageEvent {
    Entered(String),
    Ended(String),
}

/// A run with no terminal event, and the recipe it started under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unended {
    pub run: String,
    pub recipe: String,
}

/// Whether the run named by the query's `r` alias has a terminal event.
fn ended_sql() -> String {
    format!(
        "EXISTS (SELECT 1 FROM events WHERE events.run_id = r.run_id AND {})",
        crate::terminal::terminal_predicate()
    )
}

/// The latest `?1` ended runs of each recipe, and the events of theirs that size
/// it, newest run first and each run's events in order.
fn history_sql() -> String {
    let kind = crate::summary::text_at("e.envelope", "$.type");
    let phase = crate::summary::text_at("e.envelope", "$.payload.phase");
    let ended = ended_sql();
    format!(
        "SELECT r.run_id, r.bundle_name, r.created_at, {kind}, {phase},
            CASE WHEN json_valid(e.envelope)
                  AND json_type(e.envelope, '$.payload.usage') = 'object'
                THEN json_extract(e.envelope, '$.payload.usage') END
         FROM (SELECT run_id, bundle_name, created_at, ROW_NUMBER() OVER (
                   PARTITION BY bundle_name ORDER BY created_at DESC, run_id DESC) AS latest
               FROM runs AS r WHERE {ended}) AS r
         JOIN events AS e ON e.run_id = r.run_id
         WHERE r.latest <= ?1
           AND (instr(e.envelope, 'phase/entered') > 0 OR instr(e.envelope, '\"usage\"') > 0)
         ORDER BY r.created_at DESC, r.run_id DESC, e.seq"
    )
}

/// The runs no terminal event ends, in the journal's order.
fn unended_sql() -> String {
    format!(
        "SELECT r.run_id, r.bundle_name FROM runs AS r WHERE NOT {}
         ORDER BY r.created_at, r.run_id",
        ended_sql()
    )
}

/// The event a history row names, or `None` for one that sizes nothing:
/// an ending without `usage`, or a row whose text only looked like one.
fn event(kind: Option<String>, phase: Option<String>, usage: Option<String>) -> Option<UsageEvent> {
    match (kind.as_deref(), phase, usage) {
        (Some("phase/entered"), Some(phase), _) => Some(UsageEvent::Entered(phase)),
        (Some("effect/succeeded" | "effect/failed" | "effect/indeterminate"), _, Some(usage)) => {
            Some(UsageEvent::Ended(usage))
        }
        _ => None,
    }
}

impl Store {
    /// Open the EXISTING journal at `path` to read, writing nothing beside
    /// it: no file is created, no schema migrated, no lock or lease taken,
    /// and no `-wal` or `-shm` made where none was. A file its bytes say is
    /// damaged refuses as [`StoreError::Damaged`], as every open does.
    pub fn open_untouched(path: &Path) -> Result<Store, StoreError> {
        let bound = literal::journal(path).map_err(StoreError::from)?;
        if std::fs::symlink_metadata(beside(&bound, "-wal")).is_ok() {
            return Store::open_read_only(path);
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        Connection::open_with_flags(immutable(&bound), flags)
            .map_err(|error| literal::as_given(error, path, &bound).into())
            .and_then(|conn| Store::reading(conn, path))
            .map_err(|error| damage::classified(path, error))
    }

    /// Every event of the latest `candidates` ended runs of each recipe that
    /// sizes it ([`UsageEvent`]), newest run first.
    pub fn usage_history(&self, candidates: u32) -> Result<Vec<UsageRow>, StoreError> {
        let sql = history_sql();
        patiently("usage_history", self.patience, &self.path, || {
            let mut statement = self.conn.prepare(&sql)?;
            let rows = statement.query_map(params![candidates], |row| {
                let (run, recipe, created_at) = (row.get(0)?, row.get(1)?, row.get(2)?);
                let event = event(row.get(3)?, row.get(4)?, row.get(5)?);
                Ok(event.map(|event| UsageRow {
                    run,
                    recipe,
                    created_at,
                    event,
                }))
            })?;
            let rows = rows.collect::<Result<Vec<_>, _>>()?;
            Ok(rows.into_iter().flatten().collect())
        })
    }

    /// Every run no terminal event ends, and its recipe: the runs that may
    /// still be running, which only their fold says.
    pub fn unended_runs(&self) -> Result<Vec<Unended>, StoreError> {
        let sql = unended_sql();
        patiently("unended_runs", self.patience, &self.path, || {
            let mut statement = self.conn.prepare(&sql)?;
            let rows = statement.query_map([], |row| {
                Ok(Unended {
                    run: row.get(0)?,
                    recipe: row.get(1)?,
                })
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }
}

/// The side file `suffix` names beside the journal `bound`.
fn beside(bound: &Path, suffix: &str) -> PathBuf {
    let mut named = bound.as_os_str().to_owned();
    named.push(suffix);
    PathBuf::from(named)
}

/// The URI SQLite reads `bound` through as a file nothing changes: its
/// bytes percent-encoded but for the unreserved ones and `/`.
fn immutable(bound: &Path) -> String {
    let mut uri = String::from("file:");
    for byte in bound.as_os_str().as_encoded_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'.' | b'_' | b'~' => {
                uri.push(char::from(*byte));
            }
            other => uri.push_str(&format!("%{other:02X}")),
        }
    }
    uri.push_str("?mode=ro&immutable=1");
    uri
}

#[cfg(test)]
mod tests;
