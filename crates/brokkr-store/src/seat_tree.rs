//! The `seat_trees` record (decision 0104 ruling 6, slice 5b): one
//! append-only row for every run-local seat scratch tree Brokkr makes for an
//! attempt, written before the tree exists. The tree's path is `root` and
//! the attempt's id; the row is the authority the later sweep reads, and a
//! tree no row names is never removed.
//!
//! A version 1, 2, 3 or 4 queue holds no table. A queue this binary does
//! not write here — older and opened read-only, or newer than this binary
//! knows — is left as found: the append writes nothing and says so, as a
//! cool-down append leaves an older or newer queue alone.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use thiserror::Error;

use crate::queue::storage::{queue_schema, queue_stored, Stored, QUEUE_SCHEMA};
use crate::schema::objects;
use crate::{patiently, Store, StoreError};

/// The version 5 table, by name: the intact check asks for it, its unique
/// index is repaired by it, and a lost index over rows that already hold a
/// duplicate is refused by its key.
pub(crate) const SEAT_TREE_TABLE: &str = "seat_trees";

/// The unique index on `(run_id, attempt_id)`: one tree per attempt.
pub(crate) const SEAT_TREE_INDEX: &str = "seat_trees_once";

/// One `seat_trees` row as a reader holds it. `root` joins the attempt's id
/// to name the tree; `recorded_at` is Unix seconds (UTC), as the table's
/// own comment says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatTree {
    pub seq: i64,
    pub run_id: String,
    pub effect_id: String,
    pub attempt_id: String,
    pub root: PathBuf,
    pub recorded_at: i64,
}

/// What an append did: wrote the row, found the same row already standing,
/// or wrote nothing because the queue is one this binary does not write here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatTreeRecord {
    /// The row was written.
    Written,
    /// The same row stood already; nothing was written.
    Present,
    /// The queue is one this binary does not write here — older and opened
    /// read-only, or newer than this binary knows: left as found, and
    /// nothing was written.
    Unrecorded,
}

/// A seat-tree append or repair refused. A differing second append names
/// the run and attempt; a lost unique index over duplicate rows names the
/// key that is not unique.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SeatTreeRefusal {
    /// A second append for one attempt agreed on neither its effect nor
    /// its root, so it is not the same tree.
    #[error(
        "run '{run_id}' already records a seat tree for attempt '{attempt_id}' with another \
         effect or root; this one was not written"
    )]
    Mismatch { run_id: String, attempt_id: String },
    /// The lost unique index cannot be rebuilt over rows that already hold
    /// a duplicate of its key.
    #[error(
        "the journal's seat trees hold run '{run_id}' attempt '{attempt_id}' more than once; \
         the lost unique index is not rebuilt over them"
    )]
    Duplicate { run_id: String, attempt_id: String },
    /// A root path SQLite cannot hold as text.
    #[error("the seat tree root '{}' is not UTF-8, so the journal cannot name it", .0.display())]
    RootUnnameable(PathBuf),
}

impl Store {
    /// Record this run's attempt tree at `root` before the tree is made.
    /// One row per `(run_id, attempt_id)`: the same append a second time
    /// writes nothing and succeeds, while one that differs in effect or
    /// root is refused by [`SeatTreeRefusal::Mismatch`]. A queue this
    /// binary does not write here — older and opened read-only, or newer
    /// than this binary knows — is left as found and reports
    /// [`SeatTreeRecord::Unrecorded`], so an older engine's tree is made
    /// without a row, as slice 5c's doctor expects.
    pub fn record_seat_tree(
        &mut self,
        run_id: &str,
        effect_id: &str,
        attempt_id: &str,
        root: &Path,
    ) -> Result<SeatTreeRecord, StoreError> {
        let root = root
            .to_str()
            .ok_or_else(|| SeatTreeRefusal::RootUnnameable(root.to_path_buf()))?;
        let (conn, patience) = (&mut self.conn, self.patience);
        patiently("record_seat_tree", patience, || {
            record_once(conn, run_id, effect_id, attempt_id, root)
        })
    }

    /// A run's seat-tree rows, in `seq` order: the attempt trees the sweep
    /// reads. A queue older than the table holds none, and a queue this
    /// binary does not know refuses through [`StoreError::Queue`], because
    /// only its own binary can say what it holds.
    pub fn seat_trees(&self, run_id: &str) -> Result<Vec<SeatTree>, StoreError> {
        patiently("seat_trees", self.patience, || self.seat_trees_once(run_id))
    }

    fn seat_trees_once(&self, run_id: &str) -> Result<Vec<SeatTree>, StoreError> {
        match queue_stored(&self.conn)? {
            Some(Stored::V5) => read_trees(&self.conn, run_id),
            Some(Stored::V1 | Stored::V2 | Stored::V3 | Stored::V4) | None => Ok(Vec::new()),
        }
    }
}

/// One append inside the write lock: the standing row, else the insert.
fn record_once(
    conn: &mut Connection,
    run_id: &str,
    effect_id: &str,
    attempt_id: &str,
    root: &str,
) -> Result<SeatTreeRecord, StoreError> {
    if queue_schema(conn)? != Some(QUEUE_SCHEMA) {
        return Ok(SeatTreeRecord::Unrecorded);
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let standing: Option<(String, String)> = tx
        .query_row(
            "SELECT effect_id, root FROM seat_trees WHERE run_id = ?1 AND attempt_id = ?2",
            params![run_id, attempt_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let record = match standing {
        Some((effect, stands)) if effect == effect_id && stands == root => SeatTreeRecord::Present,
        Some(_) => {
            return Err(SeatTreeRefusal::Mismatch {
                run_id: run_id.to_string(),
                attempt_id: attempt_id.to_string(),
            }
            .into())
        }
        None => {
            tx.execute(
                "INSERT INTO seat_trees (run_id, effect_id, attempt_id, root, recorded_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![run_id, effect_id, attempt_id, root, now_seconds()],
            )?;
            SeatTreeRecord::Written
        }
    };
    tx.commit()?;
    Ok(record)
}

/// The rows of one run, oldest first: `seq` is the table's rising key.
fn read_trees(conn: &Connection, run_id: &str) -> Result<Vec<SeatTree>, StoreError> {
    let mut rows = conn.prepare(
        "SELECT seq, run_id, effect_id, attempt_id, root, recorded_at
         FROM seat_trees WHERE run_id = ?1 ORDER BY seq",
    )?;
    let trees = rows.query_map([run_id], |row| {
        Ok(SeatTree {
            seq: row.get(0)?,
            run_id: row.get(1)?,
            effect_id: row.get(2)?,
            attempt_id: row.get(3)?,
            root: PathBuf::from(row.get::<_, String>(4)?),
            recorded_at: row.get(5)?,
        })
    })?;
    Ok(trees.collect::<Result<_, _>>()?)
}

/// Refuse to rebuild [`SEAT_TREE_INDEX`] over rows that already hold a
/// duplicate of its key: the index can be put back only where every key is
/// still unique, and a journal that lost it and gained a second row is
/// named by that key rather than failing on SQLite's own constraint text.
/// A table that is absent is not yet a duplicate, and an index that is
/// present already refuses one, so neither is read.
pub(crate) fn refuse_duplicates(conn: &Connection) -> Result<(), StoreError> {
    if !objects(conn, "table")?
        .iter()
        .any(|name| name == SEAT_TREE_TABLE)
        || objects(conn, "index")?
            .iter()
            .any(|name| name == SEAT_TREE_INDEX)
    {
        return Ok(());
    }
    let duplicate: Option<(String, String)> = conn
        .query_row(
            "SELECT run_id, attempt_id FROM seat_trees
             GROUP BY run_id, attempt_id HAVING COUNT(*) > 1
             ORDER BY run_id, attempt_id LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((run_id, attempt_id)) = duplicate else {
        return Ok(());
    };
    Err(SeatTreeRefusal::Duplicate { run_id, attempt_id }.into())
}

/// The append's time: whole Unix seconds (UTC), the table's own type.
fn now_seconds() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

#[cfg(test)]
mod tests;
