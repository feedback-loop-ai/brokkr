//! The ordinal a checkpoint keeps among its stream's checkpoints (#994): a
//! nullable `stream_ordinal` column on `event_index`, set by a trigger of
//! its own from the event alone, so every writer keeps it, an older
//! binary's and an import's included (decision 0089 ruling 3). A stream is
//! the key the seat read pages by, `(run_id, attempt_id, member)`, and the
//! ordinal is the 1-based place among that stream's `effect/checkpointed`
//! rows in seq order.
//!
//! The column is NULL on every row the guarded page read does not count as
//! a checkpoint of its stream: a member's result-only record above all,
//! whose `event_index.member` comes from its result where its checkpoint
//! names none (H1). A started row is a checkpoint's only where the
//! envelope's checkpoint member is the row's member, exactly as the page
//! read's `names` compares it, so the ordinal counts what the guarded read
//! counts and never a record it refuses.
//!
//! A journal from before the column is altered in place and walked once,
//! one run per transaction, after the index's own backfill. The walk
//! numbers every checkpoint of the run in seq order, so it also repairs a
//! row a missing trigger left NULL. Its own meta key records the walk
//! finished, so a steady-state open reads one meta row and never the
//! journal; a repair or a lost trigger clears it, as the other backfills'
//! keys are cleared (rulings 4 and 8).

use std::sync::LazyLock;

use rusqlite::{params_from_iter, Connection, OptionalExtension};

use crate::event_index::{finished, mark_finished, CHECKPOINTED};
use crate::{patiently, Store, StoreError};

/// The column's name: a row's 1-based place among its stream's
/// checkpoints, NULL on every other row.
pub(super) const COLUMN: &str = "stream_ordinal";

/// The meta key that records the ordinal backfill finished (ruling 8): its
/// own, so numbering a journal from before the column sends no run's
/// summary behind.
pub(super) const BACKFILLED: &str = "event_index_ordinal_backfill";

/// The partial index that names the checkpoint rows still to number: a
/// run's walk seeks its runs by key, and a journal whose ordinals all
/// stand reads one empty range.
const STALE: &str = "event_index_ordinal_stale";

/// The column a table made anew declares, after the checkpoint facts.
pub(super) fn declared() -> String {
    format!("{COLUMN} INTEGER")
}

/// Add the column a journal from before it lacks, one `ALTER` as SQLite has
/// no `ADD COLUMN IF NOT EXISTS`, presence asked rather than an error
/// swallowed.
pub(super) fn add_column(conn: &Connection) -> Result<(), StoreError> {
    if crate::schema::holds(conn, "event_index", [COLUMN])? {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE event_index ADD COLUMN {COLUMN} INTEGER"),
        [],
    )?;
    Ok(())
}

/// The last entry named `key` among `entries`, a `json_each` walk, where
/// that entry is text, else NULL: the last copy of a repeated key, as
/// serde reads it.
fn last_text(entries: &str, key: &str) -> String {
    format!(
        "(SELECT CASE WHEN type = 'text' THEN value END FROM {entries}
              WHERE key = '{key}' ORDER BY id DESC LIMIT 1)"
    )
}

/// The member the payload's `checkpoint` object names, or NULL where it
/// names no checkpoint: the one field the ordinal's guard reads, and the
/// same field the seat read's `names` compares with the participant's
/// member. The payload is walked by its entries, since serde keeps the
/// last copy of a key the payload repeats.
fn checkpoint_member(event: &str) -> String {
    let payload = crate::event_index::payload(event);
    let record = format!(
        "(SELECT value AS record, type AS kind FROM {payload}
              WHERE key = 'checkpoint' ORDER BY id DESC LIMIT 1),
         json_each(CASE WHEN kind = 'object' THEN record ELSE '{{}}' END)"
    );
    last_text(&record, "member")
}

/// The ordinal the event trigger writes for `event`, a row of `events` it
/// is indexing: its stream's previous ordinal plus one where the event's
/// checkpoint names the row's member, NULL on every other row. The guard
/// reads the envelope the trigger already holds, so the writer's own
/// append seeks the stream once and never the event again.
pub(super) fn projected(event: &str) -> String {
    let envelope = format!("{event}.envelope");
    let payload = crate::event_index::payload(event);
    let record = crate::event_index::record(event);
    let kind = crate::summary::text_at(&envelope, "$.type");
    let attempt = crate::event_index::last_text(&payload, "attempt_id");
    let member = crate::event_index::last_text(&record, "member");
    format!(
        "CASE WHEN {kind} = '{CHECKPOINTED}' AND {attempt} IS NOT NULL
               AND ({checkpoint}) IS ({member})
          THEN coalesce(
              (SELECT {COLUMN} FROM event_index INDEXED BY event_index_stream
                WHERE run_id = {event}.run_id AND attempt_id IS {attempt}
                  AND member IS {member} AND seq < {event}.seq
                  AND type = '{CHECKPOINTED}' AND {COLUMN} IS NOT NULL
                ORDER BY seq DESC LIMIT 1), 0) + 1
          END",
        checkpoint = checkpoint_member(event),
    )
}

/// The index, trigger and marker that keep the ordinals: one string placed
/// after the table, so a journal that lost any of them gets them again and
/// the backfill that follows the open numbers its rows once. The trigger
/// numbers a row inserted past the event trigger (the index backfill's and
/// an older writer's direct insert): the event trigger's own rows already
/// carry theirs, so this one skips them. The stale index names the rows
/// still to number.
pub(super) fn objects() -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {STALE} ON event_index (run_id)
             WHERE type = '{CHECKPOINTED}' AND {COLUMN} IS NULL;
         CREATE INDEX IF NOT EXISTS event_index_ordinal_written
             ON event_index (run_id, attempt_id, member, {COLUMN})
             WHERE type = '{CHECKPOINTED}' AND {COLUMN} IS NOT NULL;
         DROP TRIGGER IF EXISTS event_index_ordinal;
         CREATE TRIGGER event_index_ordinal AFTER INSERT ON event_index
             WHEN NEW.type = '{CHECKPOINTED}' AND NEW.attempt_id IS NOT NULL
              AND NEW.{COLUMN} IS NULL
              AND (SELECT {member} FROM events AS e
                    WHERE e.run_id = NEW.run_id AND e.seq = NEW.seq) IS NEW.member
             BEGIN
                 UPDATE event_index SET {COLUMN} = coalesce(
                     (SELECT {COLUMN} FROM event_index INDEXED BY event_index_stream
                       WHERE run_id = NEW.run_id AND attempt_id IS NEW.attempt_id
                         AND member IS NEW.member AND seq < NEW.seq
                         AND type = '{CHECKPOINTED}' AND {COLUMN} IS NOT NULL
                       ORDER BY seq DESC LIMIT 1), 0) + 1
                 WHERE run_id = NEW.run_id AND seq = NEW.seq;
             END;
         DELETE FROM meta WHERE key = '{BACKFILLED}';",
        member = checkpoint_member("e"),
    )
}

/// The statement that numbers every checkpoint row of one run in seq order,
/// partitioned by its stream. A row the guard does not count (a
/// result-only record, an attemptless checkpoint) is left NULL, so a
/// reader never reads it as zero.
static NUMBER: LazyLock<String> = LazyLock::new(|| {
    format!(
        "UPDATE event_index SET {COLUMN} = (
             SELECT numbered.at FROM (
                 SELECT i.seq AS seq,
                        row_number() OVER (PARTITION BY i.attempt_id, i.member
                                           ORDER BY i.seq) AS at
                 FROM event_index AS i
                 WHERE i.run_id = ?1 AND i.type = '{CHECKPOINTED}'
                   AND i.attempt_id IS NOT NULL
                   AND (SELECT {member} FROM events AS e
                         WHERE e.run_id = i.run_id AND e.seq = i.seq) IS i.member
             ) AS numbered WHERE numbered.seq = event_index.seq)
         WHERE run_id = ?1 AND type = '{CHECKPOINTED}'",
        member = checkpoint_member("e"),
    )
});

/// The first run, at or after `after`, with a checkpoint row still to
/// number, sought through the stale partial index alone.
fn stale(after: bool) -> String {
    let cursor = if after { "AND run_id > ?1" } else { "" };
    format!(
        "SELECT run_id FROM event_index INDEXED BY {STALE}
         WHERE type = '{CHECKPOINTED}' AND {COLUMN} IS NULL {cursor}
         ORDER BY run_id LIMIT 1"
    )
}

static STALE_FIRST: LazyLock<String> = LazyLock::new(|| stale(false));
static STALE_AFTER: LazyLock<String> = LazyLock::new(|| stale(true));

/// One run of the ordinal backfill, in its own transaction.
fn once(conn: &mut Connection, run_id: &str) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(&NUMBER, [run_id])?;
    tx.commit()?;
    Ok(())
}

impl Store {
    /// Number every checkpoint row a writer from before the column, or a
    /// missing trigger, left unnumbered, one run per transaction (decision
    /// 0089 ruling 8). Every writable open makes it after the index's own
    /// backfill, whose rows must exist first. A finished walk reads one
    /// meta row; a repair or a lost trigger clears the mark. `between` runs
    /// on this connection after a run is named and before its transaction,
    /// the instant the tests stage a peer's lock at. The memory is the
    /// walk's cursor.
    pub(crate) fn ordinal_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        if finished(&self.conn, BACKFILLED)? {
            return Ok(());
        }
        let mut after: Option<String> = None;
        loop {
            let (sql, bound) = match &after {
                Some(after) => (&*STALE_AFTER, Some(after.as_str())),
                None => (&*STALE_FIRST, None),
            };
            let run_id: Option<String> = {
                let mut statement = self.conn.prepare_cached(sql)?;
                statement
                    .query_row(params_from_iter(bound), |row| row.get(0))
                    .optional()?
            };
            let Some(run_id) = run_id else {
                break;
            };
            between(&self.conn);
            let conn = &mut self.conn;
            patiently("ordinals", self.patience, || once(conn, &run_id))?;
            after = Some(run_id);
        }
        let conn = &self.conn;
        patiently("ordinals", self.patience, || {
            mark_finished(conn, BACKFILLED)
        })
    }
}
