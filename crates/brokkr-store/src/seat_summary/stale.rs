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
//! ruling 8): a run joins [`STALE`] when its `run_summary` row names a
//! head no seat head holds, and leaves when a `seat_head` is written at
//! that head. A head at another seq or hash joins it again, a head that
//! moves and comes back leaves it again, and an update of a head is kept
//! as a write is. The shape half stays a read-time comparison, because
//! [`SHAPE`] is this build's constant and not the journal's:
//! [`stale_runs`] unions the set with the heads whose shape is not this
//! build's. The recommended index on `seat_head (shape)` cannot answer
//! "not this shape" without walking every head of the current shape,
//! since a range past the current value crosses every one of its
//! duplicate entries. [`SHAPE_INDEX`] is therefore a partial index over
//! `seat_head (run_id)` holding only the heads that are not this build's
//! shape, named by that constant: the shape arm scans the mismatches
//! alone. A read-only journal whose index another build's shape made
//! still answers, by the read-time `IS NOT ?` comparison on a scan, never
//! by trusting the index.
//!
//! The repair is the eighth additive repair, after the seat tables: it
//! remakes the table and the shape index, replaces the triggers' bodies
//! with this build's (a journal carrying an older body is repaired, not
//! trusted), then fills the set once under the write lock from the rule's
//! seq-and-hash half alone, so the set means exactly what its triggers
//! keep; the shape half is read at read time and never filled in. A
//! missing table, trigger or index, a trigger whose body is another
//! build's, or a table that is not keyed as this build's is, fails
//! [`stale_intact`], and the open repairs it; an index made for another
//! build's shape fails it too, so a changed [`SHAPE`] is replaced rather
//! than trusted. Nothing here moves `DATABASE_SCHEMA`:
//! the triggers are part of the file, so an older binary reads and writes
//! past an intact set exactly as before, and a summary repair an older
//! binary runs moves a head the triggers see.
//!
//! A reader that cannot write does not repair: it reads the set while the
//! set is intact, and the rule's own `run_summary` statement when it is
//! absent or damaged, as every projection fallback does (decision 0014).

use rusqlite::{Connection, OptionalExtension};

use crate::schema::all_present;
use crate::seat_summary::SHAPE;
use crate::summary::triggers_intact;
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

/// The trigger that keeps a run in the set by the head its summary row
/// names, and the one that keeps it by a head written to `seat_head`: each
/// leaves the set when the other table holds that head and joins it
/// otherwise. The summary's insert and update are separate triggers.
const SUMMARY_INSERT: &str = "seat_lag_of_a_summary";
const SUMMARY_MOVED: &str = "seat_lag_of_a_summary_head";
const SEAT_HEAD: &str = "seat_lag_of_a_seat_head";
const SEAT_HEAD_UPDATED: &str = "seat_lag_of_a_moved_head";

/// The trigger that joins a run whose seat head goes away: a run with no
/// head lags by the rule, whatever removed the head.
const SEAT_HEAD_DROPPED: &str = "seat_lag_of_a_dropped_head";

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

/// The subquery that holds when `table` carries `run`'s head at `seq` and
/// `hash`: the one home of what "the other table already holds this head"
/// means, read by every trigger body and by the fill.
fn matching(
    table: &str,
    run: &str,
    seq_column: &str,
    hash_column: &str,
    seq: &str,
    hash: &str,
) -> String {
    format!(
        "SELECT 1 FROM {table}
          WHERE run_id = {run} AND {seq_column} = {seq} AND {hash_column} = {hash}"
    )
}

/// The statements that keep `run` at the step its head names: leave when
/// `table` holds that head, join otherwise. The join reads the set first,
/// so the statement that fires the trigger cannot impose its conflict
/// policy on the body: a plain `INSERT` never meets a duplicate.
fn step(
    run: &str,
    table: &str,
    seq_column: &str,
    hash_column: &str,
    seq: &str,
    hash: &str,
) -> String {
    let matching = matching(table, run, seq_column, hash_column, seq, hash);
    format!(
        "DELETE FROM {STALE} WHERE run_id = {run} AND EXISTS ({matching});
         INSERT INTO {STALE} (run_id)
             SELECT {run}
             WHERE NOT EXISTS ({matching})
               AND NOT EXISTS (SELECT 1 FROM {STALE} WHERE run_id = {run});"
    )
}

/// [`step`] as a trigger body, for the triggers that keep one run.
fn in_step(
    run: &str,
    table: &str,
    seq_column: &str,
    hash_column: &str,
    seq: &str,
    hash: &str,
) -> String {
    format!(
        "BEGIN
            {step}
        END",
        step = step(run, table, seq_column, hash_column, seq, hash)
    )
}

/// The summary's triggers, `in_step` against the seat head the row names.
fn summary_insert() -> String {
    format!(
        "CREATE TRIGGER {SUMMARY_INSERT} AFTER INSERT ON run_summary
        {}",
        in_step(
            "NEW.run_id",
            "seat_head",
            "folded_seq",
            "folded_hash",
            "NEW.head_seq",
            "NEW.head_hash"
        )
    )
}

fn summary_moved() -> String {
    format!(
        "CREATE TRIGGER {SUMMARY_MOVED}
            AFTER UPDATE OF head_seq, head_hash ON run_summary
            WHEN OLD.head_seq IS NOT NEW.head_seq OR OLD.head_hash IS NOT NEW.head_hash
        {}",
        in_step(
            "NEW.run_id",
            "seat_head",
            "folded_seq",
            "folded_hash",
            "NEW.head_seq",
            "NEW.head_hash"
        )
    )
}

/// The seat head's insert trigger, `in_step` against the summary row it
/// names; an update goes through [`seat_head_updated`], which judges the
/// run it moved to the same way.
fn seat_head_insert() -> String {
    format!(
        "CREATE TRIGGER {SEAT_HEAD} AFTER INSERT ON seat_head
        {}",
        in_step(
            "NEW.run_id",
            "run_summary",
            "head_seq",
            "head_hash",
            "NEW.folded_seq",
            "NEW.folded_hash"
        )
    )
}

/// The seat head's update trigger, `in_step` against the summary row it
/// names. A head moved in place is kept as a head written is; a `run_id`
/// moved leaves the run it moved from with no head, so that run joins the
/// set, and the run it moved to is judged as an insert is.
fn seat_head_updated() -> String {
    format!(
        "CREATE TRIGGER {SEAT_HEAD_UPDATED}
            AFTER UPDATE OF run_id, folded_seq, folded_hash ON seat_head
            WHEN OLD.run_id IS NOT NEW.run_id
              OR OLD.folded_seq IS NOT NEW.folded_seq
              OR OLD.folded_hash IS NOT NEW.folded_hash
        BEGIN
            INSERT INTO {STALE} (run_id)
                SELECT OLD.run_id WHERE OLD.run_id IS NOT NEW.run_id
                  AND NOT EXISTS (SELECT 1 FROM {STALE} WHERE run_id = OLD.run_id);
            {step}
        END",
        step = step(
            "NEW.run_id",
            "run_summary",
            "head_seq",
            "head_hash",
            "NEW.folded_seq",
            "NEW.folded_hash"
        )
    )
}

fn seat_head_dropped() -> String {
    format!(
        "CREATE TRIGGER {SEAT_HEAD_DROPPED} AFTER DELETE ON seat_head BEGIN
            INSERT INTO {STALE} (run_id)
                SELECT OLD.run_id
                WHERE NOT EXISTS (SELECT 1 FROM {STALE} WHERE run_id = OLD.run_id);
        END"
    )
}

/// Every trigger's definition, as this build writes it and as SQLite
/// stores it (the text after `CREATE TRIGGER`, `IF NOT EXISTS` omitted):
/// the one home of each body, read by [`migration`] and [`stale_intact`].
fn definitions() -> [(&'static str, String); 5] {
    [
        (SUMMARY_INSERT, summary_insert()),
        (SUMMARY_MOVED, summary_moved()),
        (SEAT_HEAD, seat_head_insert()),
        (SEAT_HEAD_UPDATED, seat_head_updated()),
        (SEAT_HEAD_DROPPED, seat_head_dropped()),
    ]
}

/// The table, the shape index and the triggers: the table is remade, so a
/// journal carrying one of another shape takes this build's, and each
/// trigger is dropped and made anew, so a journal that lost one gets it
/// back and one carrying another build's body takes this build's. The
/// index is dropped and made anew too, so an index another build's shape
/// made is replaced, never trusted.
fn migration() -> String {
    let triggers: Vec<String> = definitions()
        .iter()
        .map(|(name, sql)| format!("DROP TRIGGER IF EXISTS {name};\n{sql};"))
        .collect();
    format!(
        "DROP TABLE IF EXISTS {STALE};
        CREATE TABLE {STALE} (
            run_id TEXT PRIMARY KEY
        ) WITHOUT ROWID;
        DROP INDEX IF EXISTS {SHAPE_INDEX};
        {index};
        {triggers}",
        index = shape_index_sql(true),
        triggers = triggers.join("\n"),
    )
}

/// The set's one fill, from the rule's seq-and-hash half alone: a run
/// joins when no seat head holds the head its summary row names. The
/// shape half is not filled, because no trigger keeps it; it is read at
/// read time, so the set means exactly what the triggers keep.
fn fill() -> String {
    format!(
        "INSERT INTO {STALE} (run_id)
         SELECT r.run_id FROM run_summary AS r
         WHERE NOT EXISTS ({matching})
           AND NOT EXISTS (SELECT 1 FROM {STALE} WHERE run_id = r.run_id)",
        matching = matching(
            "seat_head",
            "r.run_id",
            "folded_seq",
            "folded_hash",
            "r.head_seq",
            "r.head_hash"
        )
    )
}

/// Create the set, its triggers and the shape index, and fill it from the
/// rule's seq-and-hash half, holding the write lock: the eighth additive
/// repair.
pub(crate) fn migrate_stale(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(&migration())?;
    conn.execute(&fill(), [])?;
    Ok(())
}

/// Drop the set's two `run_summary` triggers when a table they name is
/// absent. SQLite compiles a trigger's body when the statement that fires
/// it is prepared, so the fifth repair's seed cannot write `run_summary`
/// while `seat_head` or `seat_lag` is gone; the eighth repair remakes and
/// refills them. A journal carrying both tables keeps its triggers, so the
/// steady-state open writes nothing.
pub(crate) fn forget_summary_triggers(conn: &Connection) -> Result<(), StoreError> {
    if all_present(conn, "table", &["seat_head", STALE])? {
        return Ok(());
    }
    conn.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {SUMMARY_INSERT};
         DROP TRIGGER IF EXISTS {SUMMARY_MOVED};"
    ))?;
    Ok(())
}

/// Does `seat_lag` declare `run_id` as its one primary key? One read of
/// its columns: a table of another shape, a column that is not the key, or
/// a composite key fails.
fn keyed(conn: &Connection) -> Result<bool, StoreError> {
    let mut statement = conn.prepare("SELECT name, pk FROM pragma_table_info(?1)")?;
    let found: Vec<(String, i64)> = statement
        .query_map([STALE], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let column = found.iter().any(|(name, _)| name == "run_id");
    let primary: Vec<&String> = found
        .iter()
        .filter(|(_, pk)| *pk > 0)
        .map(|(name, _)| name)
        .collect();
    Ok(column && primary.len() == 1 && primary[0].as_str() == "run_id")
}

/// Does every trigger this build writes carry this build's body? One read
/// of `sqlite_master`, so the set's five bodies are compared together
/// rather than one query a trigger.
fn bodies_intact(conn: &Connection) -> Result<bool, StoreError> {
    let mut statement =
        conn.prepare("SELECT name, sql FROM sqlite_master WHERE type = 'trigger'")?;
    let found: std::collections::HashMap<String, Option<String>> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(definitions()
        .iter()
        .all(|(name, sql)| found.get(*name).and_then(Option::as_deref) == Some(sql.as_str())))
}

/// Does this journal carry the set keyed as this build's, the shape index
/// with this build's definition, and every trigger with this build's body?
/// Pure reads, so the steady-state open writes nothing; a body another
/// build wrote fails the check, so it is replaced rather than trusted.
pub(crate) fn stale_intact(conn: &Connection) -> Result<bool, StoreError> {
    let names: Vec<&str> = definitions().iter().map(|(name, _)| *name).collect();
    let index: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [SHAPE_INDEX],
            |row| row.get(0),
        )
        .optional()?;
    Ok(all_present(conn, "table", &[STALE])?
        && triggers_intact(conn, &names)?
        && keyed(conn)?
        && index.as_deref() == Some(shape_index_sql(false).as_str())
        && bodies_intact(conn)?)
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
