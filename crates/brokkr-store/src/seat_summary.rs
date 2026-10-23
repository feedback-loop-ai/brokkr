//! The seat summary (decision 0089, slice 2): one row per participant of
//! an attempt, a seat, a panel member or a sequence step (ruling 1), the
//! rows brokkr-core's projection carries in the writer's [`Account`],
//! each field in a typed column of its own ([`SeatRow::COLUMNS`]).
//!
//! What the fold states, the writer keeps (ruling 3): an append that
//! writes the run's summary row writes its seat rows from the same
//! carried account in the same transaction, only the rows the append
//! moved where the table stands at the head the account was carried
//! from, and every row where it does not. A row is keyed by the run and
//! its place among the run's rows in the order the projection opened
//! them, `opened`: the projection's own key, its request, attempt and
//! member, held in the row beside it, opens one place and never moves
//! it, so the place is that key's one image and the order the rows are
//! grouped in. No text stands in for an absent attempt or member, as a
//! sentinel would collide with an attempt or a member a journal names by
//! that text.
//!
//! The rows carry their own head (#753 ruling 2a), `seat_head`'s
//! `(folded_seq, folded_hash)`, the last event they were folded through:
//! a binary from before this table moves a run's head and its summary
//! row and leaves its seat rows behind, so they are read only while their
//! head is the run's, and every writable open refolds a run whose seat
//! head lags, one run per transaction, with a backfill key of its own
//! (ruling 8), so no run's summary falls behind for it.
//!
//! The rows are derived, never exported and never hashed: the journal's
//! events stay the record (decisions 0001 and 0003), and no
//! `brokkr-runtime` decision reads a row (ruling 1).

use std::sync::LazyLock;

use brokkr_core::seat::row::{Datum, SeatRow};
use brokkr_core::summary::Account;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use crate::event_index::{finished, first_parted, mark_finished, IndexFault};
use crate::schema::all_present;
use crate::summary::{refold_once, Compared};
use crate::{head_of, patiently, Disagreement, Store, StoreError};

/// The meta key that records the backfill finished (ruling 8): its own,
/// so keeping seat rows sends no run's summary behind (#753 ruling 4a).
const BACKFILLED: &str = "seat_summary_backfill";

const TABLES: [&str; 2] = ["seat_summary", "seat_head"];

/// The two tables, made anew: a journal that lost one loses the other
/// with it, so no seat head outlives the rows it vouches for and every
/// run is walked again; and the backfill marked unfinished.
fn migration() -> String {
    let columns: Vec<String> = SeatRow::COLUMNS
        .iter()
        .map(|(name, kind)| format!("{name} {kind}"))
        .collect();
    format!(
        "DROP TABLE IF EXISTS seat_summary;
        DROP TABLE IF EXISTS seat_head;
        CREATE TABLE seat_summary (
            run_id TEXT NOT NULL, opened INTEGER NOT NULL, {},
            PRIMARY KEY (run_id, opened)
        ) WITHOUT ROWID;
        CREATE TABLE seat_head (
            run_id TEXT PRIMARY KEY, folded_seq INTEGER NOT NULL, folded_hash TEXT NOT NULL
        ) WITHOUT ROWID;
        DELETE FROM meta WHERE key = '{BACKFILLED}';",
        columns.join(", ")
    )
}

/// Does this journal carry both tables? A pure read of `sqlite_master`.
pub(crate) fn seats_intact(conn: &Connection) -> Result<bool, StoreError> {
    all_present(conn, "table", &TABLES)
}

/// Create the tables, on a connection holding the write lock: the seventh
/// additive repair. Their presence is asked again under the lock, so an
/// opener that raced a peer to it drops none of the rows the peer's walk
/// has written since. Their rows come from the backfill that follows the
/// open, outside this lock.
pub(crate) fn migrate_seats(conn: &Connection) -> Result<(), StoreError> {
    if !seats_intact(conn)? {
        conn.execute_batch(&migration())?;
    }
    Ok(())
}

/// Record that the backfill finished; a journal created with the tables
/// has nothing to backfill, and a run it lacks a seat head for is read as
/// stale until the open's walk folds it.
pub(crate) fn mark_seated(conn: &Connection) -> Result<(), StoreError> {
    mark_finished(conn, BACKFILLED)
}

/// The names of a row's columns after its key, comma-joined.
fn names() -> String {
    let names: Vec<&str> = SeatRow::COLUMNS.iter().map(|(name, _)| *name).collect();
    names.join(", ")
}

static INSERT: LazyLock<String> = LazyLock::new(|| {
    let bound: Vec<String> = (1..=SeatRow::COLUMNS.len() + 2)
        .map(|at| format!("?{at}"))
        .collect();
    format!(
        "INSERT OR REPLACE INTO seat_summary (run_id, opened, {}) VALUES ({})",
        names(),
        bound.join(", ")
    )
});

/// A run's rows by their key, each its place and then its columns.
static KEPT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT opened, {} FROM seat_summary WHERE run_id = ?1 ORDER BY opened",
        names()
    )
});

/// A run's rows read only where the journal holds the run and its seat
/// head is the head named: none for a head that lags or a run the journal
/// does not hold, and one empty row for a run with no rows, so one
/// statement reads the head and the rows as one snapshot.
static CURRENT: LazyLock<String> = LazyLock::new(|| {
    let columns: Vec<String> = SeatRow::COLUMNS
        .iter()
        .map(|(name, _)| format!("s.{name}"))
        .collect();
    format!(
        "SELECT s.opened, {} FROM runs AS r
             JOIN seat_head AS h ON h.run_id = r.run_id
             LEFT JOIN seat_summary AS s ON s.run_id = h.run_id
         WHERE r.run_id = ?1 AND h.folded_seq = ?2 AND h.folded_hash = ?3
         ORDER BY s.opened",
        columns.join(", ")
    )
});

const HEAD: &str =
    "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash) VALUES (?1, ?2, ?3)";

const FOLDED: &str = "SELECT folded_seq, folded_hash FROM seat_head WHERE run_id = ?1";

/// The first run past `after`, or the first of all, whose seat head is
/// not its summary row's head, which the summary's trigger keeps at the
/// run's last event: one search of each key a run.
const LAGGING: &str = "SELECT s.run_id FROM run_summary AS s
     LEFT JOIN seat_head AS h ON h.run_id = s.run_id
     WHERE s.run_id > ?1 AND (h.run_id IS NULL OR h.folded_seq <> s.head_seq
           OR h.folded_hash <> s.head_hash)
     ORDER BY s.run_id LIMIT 1";

const FIRST_LAGGING: &str = "SELECT s.run_id FROM run_summary AS s
     LEFT JOIN seat_head AS h ON h.run_id = s.run_id
     WHERE h.run_id IS NULL OR h.folded_seq <> s.head_seq OR h.folded_hash <> s.head_hash
     ORDER BY s.run_id LIMIT 1";

fn value(datum: Datum) -> Value {
    match datum {
        Datum::Null => Value::Null,
        Datum::Integer(value) => Value::Integer(value),
        Datum::Text(value) => Value::Text(value),
    }
}

/// The value a column holds as a row's datum, `None` for a real or a
/// blob, which no row writes.
fn datum(value: Value) -> Option<Datum> {
    match value {
        Value::Null => Some(Datum::Null),
        Value::Integer(value) => Some(Datum::Integer(value)),
        Value::Text(value) => Some(Datum::Text(value)),
        Value::Real(_) | Value::Blob(_) => None,
    }
}

/// The head `run_id`'s seat rows were folded through, if any.
fn folded(conn: &Connection, run_id: &str) -> Result<Option<(u64, String)>, StoreError> {
    let found = conn
        .prepare_cached(FOLDED)?
        .query_row([run_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional();
    Ok(found?)
}

/// Does `run_id`'s seat head stand at `head`?
fn folded_at(conn: &Connection, run_id: &str, head: (u64, &str)) -> Result<bool, StoreError> {
    let folded = folded(conn, run_id)?;
    Ok(folded.as_ref().map(|(seq, hash)| (*seq, hash.as_str())) == Some(head))
}

/// Record that `run_id`'s seat rows are folded through `head`.
pub(crate) fn write_head(
    tx: &Connection,
    run_id: &str,
    (seq, hash): (u64, &str),
) -> Result<(), StoreError> {
    tx.prepare_cached(HEAD)?
        .execute(params![run_id, seq as i64, hash])?;
    Ok(())
}

/// The rows the table already holds of `run_id`, as `before` carries
/// them, where its seat head stands at `before`'s head; otherwise none,
/// and the table holds none of the run's once this returns.
fn held<'a>(
    tx: &Connection,
    run_id: &str,
    before: Option<&'a Account>,
) -> Result<&'a [SeatRow], StoreError> {
    if let Some(before) = before {
        if folded_at(tx, run_id, before.head())? {
            return Ok(before.seats());
        }
    }
    let cleared = tx.prepare_cached("DELETE FROM seat_summary WHERE run_id = ?1");
    cleared?.execute([run_id])?;
    Ok(&[])
}

/// Write `account`'s seat rows and head, inside the caller's transaction:
/// only the rows that are not `before`'s where the table stands at the
/// head `before` was carried to, so an append writes the rows its events
/// moved, a carried member whose source they named again among them; and
/// every row where it does not, or where no `before` is given.
pub(crate) fn write(
    tx: &Connection,
    account: &Account,
    before: Option<&Account>,
) -> Result<(), StoreError> {
    let run_id = account.run_id();
    let held = held(tx, run_id, before)?;
    let mut insert = tx.prepare_cached(&INSERT)?;
    for (opened, row) in account.seats().iter().enumerate() {
        if held.get(opened) != Some(row) {
            let key = [
                Value::Text(run_id.to_string()),
                Value::Integer(opened as i64),
            ];
            let values = row.data().into_iter().map(value);
            insert.execute(params_from_iter(key.into_iter().chain(values)))?;
        }
    }
    write_head(tx, run_id, account.head())
}

/// What a run's seat rows read as: current, at the run's last event, or
/// stale, where nothing is read from them (ruling 8's fallback).
#[derive(Debug, Clone, PartialEq)]
pub enum SeatRows {
    Current(Vec<SeatRow>),
    Stale,
}

/// `run_id`'s rows, where its seat head is `head`; stale where it is
/// not, or where a row holds what no writer writes.
fn current(conn: &Connection, run_id: &str, head: (u64, &str)) -> Result<SeatRows, StoreError> {
    let mut statement = conn.prepare_cached(&CURRENT)?;
    let mut found = statement.query(params![run_id, head.0 as i64, head.1])?;
    let mut rows = Vec::new();
    let mut at_head = false;
    while let Some(row) = found.next()? {
        at_head = true;
        let opened: Option<i64> = row.get(0)?;
        if opened.is_none() {
            continue;
        }
        let read: Vec<Value> = (1..=SeatRow::COLUMNS.len())
            .map(|at| row.get(at))
            .collect::<Result<_, _>>()?;
        let data: Option<Vec<Datum>> = read.into_iter().map(datum).collect();
        match data.and_then(SeatRow::of_data) {
            Some(seat) => rows.push(seat),
            None => return Ok(SeatRows::Stale),
        }
    }
    Ok(match at_head {
        true => SeatRows::Current(rows),
        false => SeatRows::Stale,
    })
}

/// The columns `row` is written as, by name.
fn columns(row: &SeatRow) -> Vec<(&'static str, Value)> {
    let names = SeatRow::COLUMNS.iter().map(|(name, _)| *name);
    names.zip(row.data().into_iter().map(value)).collect()
}

/// How `account`'s seat rows stand in the table (ruling 4): behind where
/// their seat head is not the account's head, a lag the next writer
/// refolds; else the first row that parts from the account's, by its
/// place and how, the rows streamed beside the account's.
pub(crate) fn compared(conn: &Connection, account: &Account) -> Result<Compared, StoreError> {
    let run_id = account.run_id();
    if !folded_at(conn, run_id, account.head())? {
        return Ok(Compared::SeatsBehind);
    }
    let mut statement = conn.prepare_cached(&KEPT)?;
    let rows = statement.query([run_id])?;
    let seats = account.seats().iter().enumerate();
    let expected = seats.map(|(opened, row)| (opened as u64, columns(row)));
    Ok(match first_parted(rows, expected)? {
        Some((row, fault)) => Compared::Disagrees(Disagreement::Seated {
            run_id: run_id.to_string(),
            row,
            fault,
        }),
        None => Compared::Agrees,
    })
}

/// The words `brokkr doctor` names a seat row that is not its fold's by.
pub(crate) fn named(
    f: &mut std::fmt::Formatter<'_>,
    run_id: &str,
    row: u64,
    fault: IndexFault,
) -> std::fmt::Result {
    match fault {
        IndexFault::Missing => write!(
            f,
            "run '{run_id}': its fold's seat row {row} has no seat summary row"
        ),
        IndexFault::Column(column) => write!(
            f,
            "run '{run_id}': its seat rows stand at its head, and the {column} of its seat row \
             {row} is not its fold's"
        ),
        IndexFault::Stray => write!(
            f,
            "run '{run_id}': the seat summary holds a row at {row}, where its fold holds none"
        ),
    }
}

/// The words `brokkr doctor` names a seat head of no run by.
pub(crate) fn headless(f: &mut std::fmt::Formatter<'_>, run_id: &str) -> std::fmt::Result {
    write!(
        f,
        "run '{run_id}': the seat summary holds a seat head of it, where the journal holds no \
         such run"
    )
}

/// Each run id the seat summary holds rows of and `runs` does not, with
/// its first such place, in run id order: the rows a sweep that walks
/// `runs` never reaches, read in one statement. A run a peer creates
/// meanwhile is never named, since its `runs` row commits before its
/// first seat row.
const RUNLESS_ROWS: &str = "SELECT run_id, min(opened) FROM seat_summary
     WHERE run_id NOT IN (SELECT run_id FROM runs) GROUP BY run_id ORDER BY run_id";

/// Each run id the seat summary holds a seat head of and `runs` does not,
/// in run id order; a run's head commits with its `runs` row.
const RUNLESS_HEADS: &str = "SELECT run_id FROM seat_head
     WHERE run_id NOT IN (SELECT run_id FROM runs) ORDER BY run_id";

/// Every seat row of a run the journal does not hold, named as a stray at
/// its run's first such place, then every seat head of one (ruling 4): no
/// fold stands behind either, since every fold is a `runs` row's.
pub(crate) fn runless(conn: &Connection) -> Result<Vec<Disagreement>, StoreError> {
    let mut rows = conn.prepare_cached(RUNLESS_ROWS)?;
    let strays = rows.query_map([], |row| {
        Ok(Disagreement::Seated {
            run_id: row.get(0)?,
            row: row.get(1)?,
            fault: IndexFault::Stray,
        })
    })?;
    let mut found: Vec<Disagreement> = strays.collect::<Result<_, _>>()?;
    let mut heads = conn.prepare_cached(RUNLESS_HEADS)?;
    let headless = heads.query_map([], |row| {
        Ok(Disagreement::SeatHead {
            run_id: row.get(0)?,
        })
    })?;
    for head in headless {
        found.push(head?);
    }
    Ok(found)
}

impl Store {
    /// Refold every run whose seat head lags its head, one run per
    /// transaction, and record the backfill finished (ruling 8). Every
    /// writable open makes it after the summary's catch-up, spending no
    /// patience: a peer's lock ends it, and the next open carries on where
    /// it stopped, since a run it refolded no longer lags. A run that does
    /// not load is passed over, as the summary's catch-up passes it.
    /// `between` runs on this connection after a run is read and before
    /// its transaction, the instant the tests stage a peer's lock at. The
    /// runs are read by their key one at a time, so the memory is the
    /// largest run's.
    pub(crate) fn seat_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        let mut after: Option<String> = None;
        while let Some(run_id) = self.lagging_after(after.as_deref())? {
            self.refold(&run_id, &mut between)?;
            after = Some(run_id);
        }
        if !finished(&self.conn, BACKFILLED)? {
            let conn = &self.conn;
            patiently("seat", self.patience, || mark_seated(conn))?;
        }
        Ok(())
    }

    /// Refold `run_id` read whole, its summary row and its seat rows, in a
    /// transaction of its own, `between` run after it is read: one run of
    /// either catch-up. A run that does not load is passed over.
    pub(crate) fn refold(
        &mut self,
        run_id: &str,
        between: &mut impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        if let Ok(events) = self.load(run_id) {
            let account = Account::whole(run_id, &events);
            between(&self.conn);
            let conn = &mut self.conn;
            patiently("catch_up", self.patience, || refold_once(conn, &account))?;
        }
        Ok(())
    }

    /// The first run past `after` whose seat head lags, or the first of
    /// all.
    fn lagging_after(&self, after: Option<&str>) -> Result<Option<String>, StoreError> {
        let next = match after {
            None => self.conn.query_row(FIRST_LAGGING, [], |row| row.get(0)),
            Some(after) => self.conn.query_row(LAGGING, [after], |row| row.get(0)),
        };
        Ok(next.optional()?)
    }

    /// A run's seat rows, what brokkr-core's `facts` groups into the run
    /// view's participants (ruling 7), while the journal holds the run,
    /// carries the tables, their backfill finished and the run's seat head
    /// is its last event; [`SeatRows::Stale`] otherwise, for a reader to read the run
    /// whole instead (ruling 8). The head and the rows are read in one
    /// statement, so they are one snapshot.
    pub fn seat_rows(&self, run_id: &str) -> Result<SeatRows, StoreError> {
        patiently("seat_rows", self.patience, || {
            if !(seats_intact(&self.conn)? && finished(&self.conn, BACKFILLED)?) {
                return Ok(SeatRows::Stale);
            }
            let (seq, hash) = head_of(&self.conn, run_id)?;
            current(&self.conn, run_id, (seq, &hash))
        })
    }
}

#[cfg(test)]
mod tests;
