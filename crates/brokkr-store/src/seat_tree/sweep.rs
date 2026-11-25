//! The `tree_sweeps` record (decision 0104 ruling 8, slice 5c1): one
//! append-only row for every complete removal of a tree Brokkr made. A
//! scratch row closes the `seat_trees` row it removed, and a hands row
//! closes none because a hands tree belongs to a process and not to an
//! effect; every row, hands included, names the dispatch hold the pass
//! took, which R2 makes NOT NULL. A partial removal writes nothing, so the
//! next pass reads the tree still open and tries again.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{now_seconds, SeatTree, SeatTreeRefusal};
use crate::queue::storage::{queue_schema, queue_stored, Stored, QUEUE_SCHEMA};
use crate::schema::objects;
use crate::{patiently, Store, StoreError};

/// The version 6 table, by name: the intact check asks for it, its unique
/// index is repaired by it, and a lost index over rows that already hold a
/// duplicate is refused by its key.
pub(crate) const TREE_SWEEP_TABLE: &str = "tree_sweeps";

/// The unique index on `tree_seq`: one complete removal per seat tree.
pub(crate) const TREE_SWEEP_INDEX: &str = "tree_sweeps_once";

/// The tree a complete removal closed: a run-local seat scratch tree names
/// the `seat_trees` row it closed, and a hands session tree names no row.
/// The row travels with the kind, so neither an unclosed scratch nor a
/// closed hands tree is representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swept {
    Scratch(i64),
    Hands,
}

impl Swept {
    /// The table's word for the kind.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Swept::Scratch(_) => "scratch",
            Swept::Hands => "hands",
        }
    }

    /// The `seat_trees` row this removal closes, if any.
    pub(crate) const fn tree_seq(self) -> Option<i64> {
        match self {
            Swept::Scratch(seq) => Some(seq),
            Swept::Hands => None,
        }
    }

    /// The kind a table row names, or `None` for a pair the table's own
    /// `CHECK` makes impossible.
    fn of(word: &str, tree_seq: Option<i64>) -> Option<Swept> {
        match (word, tree_seq) {
            ("scratch", Some(seq)) => Some(Swept::Scratch(seq)),
            ("hands", None) => Some(Swept::Hands),
            _ => None,
        }
    }
}

/// One `tree_sweeps` row as a reader holds it. `bytes` is `None` when the
/// tree was gone before the pass; `removed_at` is Unix seconds (UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSweep {
    pub seq: i64,
    pub kind: Swept,
    pub path: PathBuf,
    pub bytes: Option<i64>,
    pub removed_at: i64,
    pub generation: i64,
}

/// What a sweep append did: wrote the row, or wrote nothing because the
/// queue is one this binary does not write here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSweepRecord {
    /// The row was written.
    Written,
    /// The queue is one this binary does not write here — older and opened
    /// read-only, or newer than this binary knows: left as found, and
    /// nothing was written.
    Unrecorded,
}

impl Store {
    /// Record one complete removal. A [`Swept::Scratch`] names the
    /// `seat_trees` row it closes; [`Swept::Hands`] names none. A second
    /// row for one `tree_seq` is refused by [`SeatTreeRefusal::SweepClosed`]
    /// with nothing written. A queue this binary does not write here is
    /// left as found and reports [`TreeSweepRecord::Unrecorded`].
    pub fn record_tree_sweep(
        &mut self,
        swept: Swept,
        path: &Path,
        bytes: Option<i64>,
        generation: i64,
    ) -> Result<TreeSweepRecord, StoreError> {
        let path = path
            .to_str()
            .ok_or_else(|| SeatTreeRefusal::SweepUnnameable(path.to_path_buf()))?;
        let (conn, patience) = (&mut self.conn, self.patience);
        patiently("record_tree_sweep", patience, &self.path, || {
            record_sweep_once(conn, swept, path, bytes, generation)
        })
    }

    /// The seat-tree rows no `tree_sweeps` row closes, in `seq` order,
    /// across every run: the work a sweep pass reads. A version 5 queue
    /// holds no sweep table, so every row it holds is open; a queue older
    /// than the table holds none. A queue this binary does not know refuses
    /// through [`StoreError::Queue`].
    pub fn open_seat_trees(&self) -> Result<Vec<SeatTree>, StoreError> {
        patiently("open_seat_trees", self.patience, &self.path, || {
            self.open_seat_trees_once()
        })
    }

    fn open_seat_trees_once(&self) -> Result<Vec<SeatTree>, StoreError> {
        match queue_stored(&self.conn)? {
            Some(Stored::V6 | Stored::V7) => read_trees_from(&self.conn, READ_OPEN_TREES),
            Some(Stored::V5) => read_trees_from(&self.conn, READ_ALL_TREES),
            Some(Stored::V1 | Stored::V2 | Stored::V3 | Stored::V4) | None => Ok(Vec::new()),
        }
    }

    /// Every `tree_sweeps` row, in `seq` order. A queue older than the
    /// table holds none, and a queue this binary does not know refuses
    /// through [`StoreError::Queue`].
    pub fn tree_sweeps(&self) -> Result<Vec<TreeSweep>, StoreError> {
        patiently("tree_sweeps", self.patience, &self.path, || {
            self.tree_sweeps_once()
        })
    }

    fn tree_sweeps_once(&self) -> Result<Vec<TreeSweep>, StoreError> {
        match queue_stored(&self.conn)? {
            Some(Stored::V6 | Stored::V7) => read_sweeps(&self.conn),
            Some(Stored::V1 | Stored::V2 | Stored::V3 | Stored::V4 | Stored::V5) | None => {
                Ok(Vec::new())
            }
        }
    }
}

/// One append inside the write lock: the standing sweep, else the insert.
fn record_sweep_once(
    conn: &mut Connection,
    swept: Swept,
    path: &str,
    bytes: Option<i64>,
    generation: i64,
) -> Result<TreeSweepRecord, StoreError> {
    if queue_schema(conn)? != Some(QUEUE_SCHEMA) {
        return Ok(TreeSweepRecord::Unrecorded);
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(tree_seq) = swept.tree_seq() {
        let closed: Option<i64> = tx
            .query_row(
                "SELECT seq FROM tree_sweeps WHERE tree_seq = ?1",
                params![tree_seq],
                |row| row.get(0),
            )
            .optional()?;
        if closed.is_some() {
            return Err(SeatTreeRefusal::SweepClosed { tree_seq }.into());
        }
    }
    tx.execute(
        "INSERT INTO tree_sweeps (kind, tree_seq, path, bytes, removed_at, generation)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            swept.word(),
            swept.tree_seq(),
            path,
            bytes,
            now_seconds(),
            generation
        ],
    )?;
    tx.commit()?;
    Ok(TreeSweepRecord::Written)
}

/// The seat trees no sweep closes, across every run, oldest first.
const READ_OPEN_TREES: &str = "SELECT seq, run_id, effect_id, attempt_id, root, recorded_at
     FROM seat_trees AS tree
     WHERE NOT EXISTS (SELECT 1 FROM tree_sweeps AS sweep WHERE sweep.tree_seq = tree.seq)
     ORDER BY seq";

/// Every seat tree, oldest first: a version 5 queue has no sweep to close
/// one.
const READ_ALL_TREES: &str = "SELECT seq, run_id, effect_id, attempt_id, root, recorded_at
     FROM seat_trees ORDER BY seq";

/// Every sweep row, oldest first.
const READ_SWEEPS: &str = "SELECT seq, kind, tree_seq, path, bytes, removed_at, generation
     FROM tree_sweeps ORDER BY seq";

/// The seat trees `query` selects, mapped through [`seat_tree_row`].
fn read_trees_from(conn: &Connection, query: &str) -> Result<Vec<SeatTree>, StoreError> {
    let mut statement = conn.prepare(query)?;
    let rows = statement.query_map([], super::seat_tree_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// One raw sweep row: the kind as the table stores it, until [`Swept`] has
/// judged it with its `tree_seq`.
type RawSweep = (i64, String, Option<i64>, String, Option<i64>, i64, i64);

/// Every sweep row, oldest first. The kind is mapped after the read so an
/// impossible word, or a pair its `CHECK` makes impossible, is refused by
/// name and the row is never guessed at.
fn read_sweeps(conn: &Connection) -> Result<Vec<TreeSweep>, StoreError> {
    let mut statement = conn.prepare(READ_SWEEPS)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })?;
    let raw: Vec<RawSweep> = rows.collect::<Result<_, _>>()?;
    raw.into_iter()
        .map(
            |(seq, kind, tree_seq, path, bytes, removed_at, generation)| {
                let kind = Swept::of(&kind, tree_seq)
                    .ok_or(SeatTreeRefusal::SweepKind { seq, word: kind })?;
                Ok(TreeSweep {
                    seq,
                    kind,
                    path: PathBuf::from(path),
                    bytes,
                    removed_at,
                    generation,
                })
            },
        )
        .collect()
}

/// Refuse to rebuild [`TREE_SWEEP_INDEX`] over rows that already hold a
/// duplicate of its key: the index can be put back only where every
/// `tree_seq` is still unique, and a journal that lost it and gained a
/// second row is named by that key rather than failing on SQLite's own
/// constraint text. A table that is absent is not yet a duplicate, and an
/// index that is present already refuses one, so neither is read. A hands
/// row names no tree, and SQLite's unique index holds many NULLs, so only
/// named trees are read.
pub(crate) fn refuse_sweep_duplicates(conn: &Connection) -> Result<(), StoreError> {
    if !objects(conn, "table")?
        .iter()
        .any(|name| name == TREE_SWEEP_TABLE)
        || objects(conn, "index")?
            .iter()
            .any(|name| name == TREE_SWEEP_INDEX)
    {
        return Ok(());
    }
    let duplicate: Option<i64> = conn
        .query_row(
            "SELECT tree_seq FROM tree_sweeps WHERE tree_seq IS NOT NULL
             GROUP BY tree_seq HAVING COUNT(*) > 1 ORDER BY tree_seq LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match duplicate {
        Some(tree_seq) => Err(SeatTreeRefusal::SweepDuplicate { tree_seq }.into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
