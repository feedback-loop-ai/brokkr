//! The stale-seat set (decision 0089 slice 4, U3d): the runs whose seat
//! head lags the summary head they name, kept by the journal's own
//! triggers so the whole-run fallback visits the stale runs alone.
//!
//! The seat-currentness rule's one home stays [`lags`]: a run lags when it
//! has no seat head, when its seat head is at another seq or hash than the
//! summary row names, or when the head was written by a build of another
//! shape. The first two are facts the journal's own tables state, so a
//! table of their runs is kept by triggers and an older writer keeps it
//! exactly, its appends being rows of the same journal (decision 0089
//! ruling 8): a run joins [`STALE`] when its `run_summary` row is inserted
//! or its head moves, and leaves when a `seat_head` is written at that
//! head. A head at another seq or hash joins it again. The shape half stays
//! a read-time comparison, because [`SHAPE`] is this build's constant and
//! not the journal's: [`stale_runs`] unions the set with the heads whose
//! shape is not this build's. The recommended index on `seat_head (shape)`
//! cannot answer "not this shape" without walking every head of the
//! current shape, since a range past the current value crosses every one
//! of its duplicate entries. [`SHAPE_INDEX`] is therefore a partial index
//! over `seat_head (run_id)` holding only the heads that are not this
//! build's shape, named by that constant: the shape arm scans the
//! mismatches alone. A read-only journal whose index another build's
//! shape made still answers, by the read-time `IS NOT ?` comparison on a
//! scan, never by trusting the index.
//!
//! The repair is the eighth additive repair, beside [`migrate_seats`]: it
//! creates the table, its triggers and the shape index `IF NOT EXISTS`,
//! then fills the set once under the write lock from [`lags`], so a
//! journal migrated mid-life names every run that lags now. A missing
//! table, trigger or index fails [`stale_intact`], and the open repairs
//! it; an index made for another build's shape fails it too, so a changed
//! [`SHAPE`] is replaced rather than trusted. Nothing here moves
//! `DATABASE_SCHEMA`: the triggers are part of the file, so an older
//! binary reads and writes past an intact set exactly as before.
//!
//! A reader that cannot write does not repair and reads the set as it
//! finds it, as every projection fallback does (decision 0014).

use rusqlite::{Connection, OptionalExtension};

use crate::schema::{all_present, holds};
use crate::seat_summary::{lags, SHAPE};
use crate::StoreError;

/// The set's table: the runs whose seat head lags by seq or hash, each
/// once. The key is the run's own, so the set costs one row a lagging run
/// and the fallback reads it by that key.
pub(crate) const STALE: &str = "seat_lag";

/// The shape index: a partial index over the heads that are not this
/// build's shape, so the shape arm visits the mismatches alone and never
/// the journal's current heads. Its predicate names [`SHAPE`], so
/// [`stale_intact`] replaces it when the constant moves.
pub(crate) const SHAPE_INDEX: &str = "seat_head_shape";

/// The trigger that joins a run when its summary row is inserted, and the
/// one that joins it when the row's head moves: the two writes the
/// journal's own triggers and writers make.
const SUMMARY_INSERT: &str = "seat_lag_of_a_summary";
const SUMMARY_MOVED: &str = "seat_lag_of_a_summary_head";

/// The trigger that keeps a `seat_head` write and the summary row it names
/// in step: it leaves the set at the row's head, and joins it otherwise.
const SEAT_HEAD: &str = "seat_lag_of_a_seat_head";

/// The trigger that joins a run whose seat head goes away: a run with no
/// head lags by the rule, whatever removed the head.
const SEAT_HEAD_DROPPED: &str = "seat_lag_of_a_dropped_head";

const TRIGGERS: [&str; 4] = [SUMMARY_INSERT, SUMMARY_MOVED, SEAT_HEAD, SEAT_HEAD_DROPPED];

/// The shape index's statement: `if_not_exists` for the repair's own
/// `CREATE`, false for the text SQLite stores and [`stale_intact`] reads
/// back (SQLite drops the clause it stores).
fn shape_index_sql(if_not_exists: bool) -> String {
    let clause = if if_not_exists { "IF NOT EXISTS " } else { "" };
    format!(
        "CREATE INDEX {clause}{SHAPE_INDEX} ON seat_head (run_id)
         WHERE shape IS NOT '{}'",
        SHAPE.as_str()
    )
}

/// The table, the shape index and the four triggers, `IF NOT EXISTS`
/// throughout: a journal that lost one gets it back, and a journal that
/// already carries them takes no write lock for this repair. The index is
/// dropped and made anew when this repair runs, so an index another
/// build's shape made is replaced, never trusted.
fn migration() -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {STALE} (
            run_id TEXT PRIMARY KEY
        ) WITHOUT ROWID;
        DROP INDEX IF EXISTS {SHAPE_INDEX};
        {index};
        CREATE TRIGGER IF NOT EXISTS {SUMMARY_INSERT} AFTER INSERT ON run_summary BEGIN
            INSERT OR IGNORE INTO {STALE} (run_id) VALUES (NEW.run_id);
        END;
        CREATE TRIGGER IF NOT EXISTS {SUMMARY_MOVED}
            AFTER UPDATE OF head_seq, head_hash ON run_summary BEGIN
            INSERT OR IGNORE INTO {STALE} (run_id) VALUES (NEW.run_id);
        END;
        CREATE TRIGGER IF NOT EXISTS {SEAT_HEAD} AFTER INSERT ON seat_head BEGIN
            DELETE FROM {STALE}
                WHERE run_id = NEW.run_id AND EXISTS (
                    SELECT 1 FROM run_summary
                     WHERE run_id = NEW.run_id
                       AND head_seq = NEW.folded_seq
                       AND head_hash = NEW.folded_hash);
            INSERT OR IGNORE INTO {STALE} (run_id)
                SELECT NEW.run_id WHERE NOT EXISTS (
                    SELECT 1 FROM run_summary
                     WHERE run_id = NEW.run_id
                       AND head_seq = NEW.folded_seq
                       AND head_hash = NEW.folded_hash);
        END;
        CREATE TRIGGER IF NOT EXISTS {SEAT_HEAD_DROPPED} AFTER DELETE ON seat_head BEGIN
            INSERT OR IGNORE INTO {STALE} (run_id) VALUES (OLD.run_id);
        END;",
        index = shape_index_sql(true),
    )
}

/// The set's one fill, from [`lags`] (`?1` this build's shape): every run
/// that lags now, whether no trigger kept it or a lost trigger kept it
/// wrongly. The table is cleared first, so an entry a lost trigger failed
/// to remove is not left naming a current run.
fn fill() -> String {
    format!(
        "INSERT OR IGNORE INTO {STALE} (run_id)
         SELECT r.run_id FROM run_summary AS r
         LEFT JOIN seat_head AS h ON h.run_id = r.run_id
         WHERE {}",
        lags("r", "h", "?1")
    )
}

/// Create the set, its triggers and the shape index, and fill it from
/// [`lags`], holding the write lock: the eighth additive repair.
pub(crate) fn migrate_stale(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(&migration())?;
    conn.execute_batch(&format!("DELETE FROM {STALE};"))?;
    conn.execute(&fill(), [SHAPE.as_str()])?;
    Ok(())
}

/// Does this journal carry the set, the shape index with this build's
/// definition, and every trigger that keeps it? Pure reads, so the
/// steady-state open writes nothing.
pub(crate) fn stale_intact(conn: &Connection) -> Result<bool, StoreError> {
    let index: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [SHAPE_INDEX],
            |row| row.get(0),
        )
        .optional()?;
    Ok(all_present(conn, "table", &[STALE])?
        && holds(conn, STALE, ["run_id"])?
        && index.as_deref() == Some(shape_index_sql(false).as_str())
        && all_present(conn, "trigger", &TRIGGERS)?)
}

/// The runs whose seat rows are stale, as one statement's fragment: the
/// set by its key, unioned with the seat heads whose shape is not this
/// build's (`shape` names that bound). The predicate names [`SHAPE`] as a
/// literal so the planner finds [`SHAPE_INDEX`]; the read-time bound is
/// kept beside it, so a journal carrying another build's index scans and
/// answers this build's shape rather than trusting the index. The one home
/// of what the write-side set and the read-time shape comparison together
/// mean.
pub(crate) fn stale_runs(shape: &str) -> String {
    format!(
        "SELECT run_id FROM {STALE}
         UNION
         SELECT run_id FROM seat_head
          WHERE shape IS NOT '{}' AND shape IS NOT {shape}",
        SHAPE.as_str()
    )
}

#[cfg(test)]
mod tests;
