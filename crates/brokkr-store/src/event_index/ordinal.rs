//! The ordinal a checkpoint keeps among its stream's checkpoints (#994): a
//! nullable `stream_ordinal` column on `event_index`, set by a trigger of
//! its own from the event alone, so every writer keeps it, an older
//! binary's and an import's included (decision 0089 ruling 3). A stream is
//! the key the seat read pages by, `(run_id, attempt_id, member)`, and the
//! ordinal is the 1-based place among that stream's `effect/checkpointed`
//! rows in seq order.
//!
//! NULL means "still to number", and nothing else: a row the guarded page
//! read does not count as a checkpoint of its stream carries 0, a value of
//! its own (a member's result-only record above all, whose
//! `event_index.member` comes from its result where its checkpoint names
//! none, H1; an attemptless checkpoint; any row whose checkpoint member is
//! not the row's member). A counted row is a checkpoint's only where the
//! envelope's checkpoint member is the row's member, exactly as the page
//! read's `names` compares it, so the ordinal counts what the guarded read
//! counts and never a record it refuses. The previous-ordinal seek reads
//! counted rows alone (`> 0`).
//!
//! The ordinal restarts at 1 wherever the stream's previous numbered row
//! belongs to another effect, so two effects sharing one attempt's index
//! stream count apart by their own rows' bounds (H2). `NUMBER` resets for
//! the same change with a window over the effect, so an import, a backfill
//! and a live append write the same column row for row.
//!
//! A journal from before the column is altered in place and walked once,
//! one run per transaction, after the index's own backfill. The walk
//! numbers every checkpoint of the run in seq order, so it also repairs a
//! row a missing trigger left NULL. Its own meta key records the walk
//! finished, but the walk is gated on the stale index holding a row, never
//! on the marker: a NULL is repaired whatever the marker says (M2). The
//! stale index names the rows still to number and is keyed by the stream
//! it belongs to, so another member's or attempt's unnumbered rows are
//! never scanned (M3).

use std::sync::LazyLock;

use rusqlite::{params_from_iter, Connection, OptionalExtension};

use crate::event_index::{finished, mark_finished, CHECKPOINTED};
use crate::{patiently, Store, StoreError};

/// The column's name: a row's 1-based place among its stream's counted
/// checkpoints, 0 on every other checkpoint row, NULL only while the row is
/// still to number.
pub(super) const COLUMN: &str = "stream_ordinal";

/// The meta key that records the ordinal backfill finished (ruling 8): its
/// own, so numbering a journal from before the column sends no run's
/// summary behind. It records the first finished walk; it never skips a
/// row, which the stale index alone answers.
pub(super) const BACKFILLED: &str = "event_index_ordinal_backfill";

/// The partial index that names the checkpoint rows still to number: a
/// run's walk seeks its runs by key, and a journal whose ordinals all
/// stand reads one empty range. It is keyed by the stream a stale row
/// belongs to, so a read of one participant never scans another's.
pub(super) const STALE: &str = "event_index_ordinal_stale";

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

/// The payload's entries as a `json_each` walk, guarded so an envelope no
/// reader parses walks nothing: the one reading both the string a caller
/// holds and the `events` row a statement names share.
fn payload_of(envelope: &str) -> String {
    format!(
        "json_each(CASE WHEN json_valid({envelope}) THEN {envelope} ELSE '{{}}' END, '$.payload')"
    )
}

/// The payload's checkpoint or result object as a `json_each` walk, the
/// checkpoint preferred where both stand and its last copy read.
fn record_of(envelope: &str) -> String {
    let payload = payload_of(envelope);
    format!(
        "(SELECT value AS record, type AS kind FROM {payload}
              WHERE key IN ('checkpoint', 'result')
              ORDER BY key = 'checkpoint' DESC, id DESC LIMIT 1),
         json_each(CASE WHEN kind = 'object' THEN record ELSE '{{}}' END)"
    )
}

/// The member the payload's `checkpoint` object names, or NULL where it
/// names no checkpoint: the one field the ordinal's guard reads, and the
/// same field the seat read's `names` compares with the participant's
/// member. The payload is walked by its entries, since serde keeps the
/// last copy of a key the payload repeats.
fn checkpoint_member(envelope: &str) -> String {
    let payload = payload_of(envelope);
    let record = format!(
        "(SELECT value AS record, type AS kind FROM {payload}
              WHERE key = 'checkpoint' ORDER BY id DESC LIMIT 1),
         json_each(CASE WHEN kind = 'object' THEN record ELSE '{{}}' END)"
    );
    last_text(&record, "member")
}

/// The ordinal `kind`, `attempt`, `member` and `effect` name, read from
/// the `events` row `event` (the alias whose `run_id` and `seq` the stream
/// seek binds): its stream's previous numbered row's ordinal plus one
/// where that row belongs to the same effect, 1 where it belongs to
/// another, and 0 on every checkpoint row the guarded read does not count.
/// The previous row's effect is one `events` seek by its key. Both the
/// event trigger's insert and the index trigger's update take this one
/// derivation, so a checkpoint a writer writes and one an import's trigger
/// numbers cannot part (ruling 5).
fn numbered(
    kind: &str,
    attempt: &str,
    member: &str,
    effect: &str,
    checkpoint: &str,
    event: &str,
) -> String {
    let earlier = last_text(&payload_of("earlier_event.envelope"), "effect_id");
    format!(
        "CASE WHEN {kind} = '{CHECKPOINTED}' THEN
           CASE WHEN {attempt} IS NOT NULL AND ({checkpoint}) IS ({member}) THEN
             coalesce(
               (SELECT CASE WHEN {earlier} IS ({effect})
                            THEN earlier_row.{COLUMN} ELSE 0 END
                FROM event_index AS earlier_row INDEXED BY event_index_stream
                  JOIN events AS earlier_event
                    ON earlier_event.run_id = earlier_row.run_id
                   AND earlier_event.seq = earlier_row.seq
                WHERE earlier_row.run_id = {event}.run_id
                  AND earlier_row.attempt_id IS {attempt}
                  AND earlier_row.member IS {member}
                  AND earlier_row.seq < {event}.seq
                  AND earlier_row.type = '{CHECKPOINTED}'
                  AND earlier_row.{COLUMN} > 0
                ORDER BY earlier_row.seq DESC LIMIT 1), 0) + 1
           ELSE 0 END
         END"
    )
}

/// The ordinal the event trigger writes for `event`, a row of `events` it
/// is indexing, from the envelope it holds: the writer's own append seeks
/// the stream once and never the event again.
pub(super) fn projected(event: &str) -> String {
    let envelope = format!("{event}.envelope");
    let payload = payload_of(&envelope);
    numbered(
        &crate::summary::text_at(&envelope, "$.type"),
        &crate::event_index::last_text(&payload, "attempt_id"),
        &crate::event_index::last_text(&record_of(&envelope), "member"),
        &crate::event_index::last_text(&payload, "effect_id"),
        &checkpoint_member(&envelope),
        event,
    )
}

/// The ordinal the index trigger writes for a row inserted past the event
/// trigger, from the row it already projects and the envelope `events`
/// holds for it: one `events` seek by the row's key, never the run's
/// events.
fn projected_row(row: &str) -> String {
    let envelope =
        format!("(SELECT envelope FROM events WHERE run_id = {row}.run_id AND seq = {row}.seq)");
    let payload = payload_of(&envelope);
    numbered(
        &format!("{row}.type"),
        &format!("{row}.attempt_id"),
        &format!("{row}.member"),
        &crate::event_index::last_text(&payload, "effect_id"),
        &checkpoint_member(&envelope),
        row,
    )
}

/// The index, trigger and marker that keep the ordinals: one string placed
/// after the table, so a journal that lost any of them gets them again and
/// the backfill that follows the open numbers its rows once. The indexes
/// are dropped and made anew, since a journal from before the streamless
/// stale key carries one of the old shape under the same name. The trigger
/// numbers a row inserted past the event trigger (the index backfill's and
/// an older writer's direct insert) from [`projected`], the same derivation
/// the event trigger's insert takes: the event trigger's own rows already
/// carry theirs, so this one skips them. The stale index names the rows
/// still to number, keyed by (run, attempt, member, seq); the written index
/// names the counted rows (`> 0`).
pub(super) fn objects() -> String {
    format!(
        "DROP INDEX IF EXISTS {STALE};
         CREATE INDEX IF NOT EXISTS {STALE}
             ON event_index (run_id, attempt_id, member, seq)
             WHERE type = '{CHECKPOINTED}' AND {COLUMN} IS NULL;
         DROP INDEX IF EXISTS event_index_ordinal_written;
         CREATE INDEX IF NOT EXISTS event_index_ordinal_written
             ON event_index (run_id, attempt_id, member, {COLUMN}, seq)
             WHERE type = '{CHECKPOINTED}' AND {COLUMN} IS NOT NULL;
         DROP TRIGGER IF EXISTS event_index_ordinal;
         CREATE TRIGGER event_index_ordinal AFTER INSERT ON event_index
             WHEN NEW.type = '{CHECKPOINTED}' AND NEW.{COLUMN} IS NULL
             BEGIN
                 UPDATE event_index SET {COLUMN} = {projected}
                 WHERE run_id = NEW.run_id AND seq = NEW.seq;
             END;
         DELETE FROM meta WHERE key = '{BACKFILLED}';",
        projected = projected_row("NEW"),
    )
}

/// Does the journal carry the stale index keyed by the stream a row
/// belongs to, so a participant's read never scans another's? A journal
/// from before the re-key carries the same name with `run_id` alone, and
/// the repair must make it anew: a presence question alone would leave it.
pub(super) fn intact(conn: &Connection) -> Result<bool, StoreError> {
    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [STALE],
            |row| row.get(0),
        )
        .optional()?;
    Ok(sql.is_some_and(|sql| sql.contains("attempt_id") && sql.contains("member")))
}

/// The statement that numbers every checkpoint row of one run in seq order
/// within its stream, restarting at 1 wherever the previous counted row
/// belongs to another effect, as [`projected`] does for a live append. The
/// running sum of effect changes is the previous-row rule read as a window,
/// so an import, a backfill and a live append write the same column. A row
/// the guard does not count is 0, so a reader never reads it as a place.
pub(super) static NUMBER: LazyLock<String> = LazyLock::new(|| {
    let effect = last_text(&payload_of("e.envelope"), "effect_id");
    format!(
        "UPDATE event_index SET {COLUMN} = coalesce((
             SELECT numbered.at FROM (
                 SELECT grouped.seq AS seq,
                        row_number() OVER (
                            PARTITION BY grouped.attempt_id, grouped.member, grouped.grp
                            ORDER BY grouped.seq) AS at
                 FROM (
                     SELECT lagged.seq AS seq,
                            lagged.attempt_id AS attempt_id,
                            lagged.member AS member,
                            SUM(CASE WHEN lagged.effect IS lagged.prev_effect
                                     THEN 0 ELSE 1 END) OVER (
                                PARTITION BY lagged.attempt_id, lagged.member
                                ORDER BY lagged.seq ROWS UNBOUNDED PRECEDING) AS grp
                     FROM (
                         SELECT i.seq AS seq,
                                i.attempt_id AS attempt_id, i.member AS member,
                                {effect} AS effect,
                                lag({effect}) OVER (
                                    PARTITION BY i.attempt_id, i.member
                                    ORDER BY i.seq) AS prev_effect
                         FROM event_index AS i
                           JOIN events AS e ON e.run_id = i.run_id AND e.seq = i.seq
                         WHERE i.run_id = ?1 AND i.type = '{CHECKPOINTED}'
                           AND i.attempt_id IS NOT NULL
                           AND ({checkpoint}) IS i.member
                     ) AS lagged
                 ) AS grouped
             ) AS numbered WHERE numbered.seq = event_index.seq), 0)
         WHERE run_id = ?1 AND type = '{CHECKPOINTED}'",
        checkpoint = checkpoint_member("e.envelope"),
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
    /// backfill, whose rows must exist first. The walk is gated on the
    /// stale index holding a row, one seek, never on the marker: a NULL is
    /// repaired whatever the marker says (M2). The marker records the first
    /// finished walk, so a journal with nothing stale and its marker set
    /// writes nothing; a repair or a lost trigger clears it when it makes
    /// the objects, as the other backfills' keys are (rulings 4 and 8).
    /// `between` runs on this connection after a run is named and before
    /// its transaction, the instant the tests stage a peer's lock at. The
    /// memory is the walk's cursor.
    pub(crate) fn ordinal_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        let mut after: Option<String> = None;
        let mut numbered = false;
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
            numbered = true;
            between(&self.conn);
            let conn = &mut self.conn;
            patiently("ordinals", self.patience, || once(conn, &run_id))?;
            after = Some(run_id);
        }
        if numbered || !finished(&self.conn, BACKFILLED)? {
            let conn = &self.conn;
            patiently("ordinals", self.patience, || {
                mark_finished(conn, BACKFILLED)
            })?;
        }
        Ok(())
    }
}
