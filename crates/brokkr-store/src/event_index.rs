//! The event index (decision 0089, slice 2): one row per event, keyed
//! `(run_id, seq)`, holding its type and the attempt and member it names,
//! so a run's trail and a seat's checkpoints are read by key, never by
//! reading the run whole (ruling 7).
//!
//! What the row states, the event alone states, so SQLite keeps it
//! (ruling 3): a trigger after each insert into `events` adds the row,
//! reading the envelope's text through the one text-only guard
//! ([`text_at`]), so every writer keeps it, an older binary's and an
//! import's included. No body is held (ruling 10), and no time: a page
//! reads its envelopes by key, stamps included. Every reader parses the
//! journal's schema when it opens it, so the index adds as few objects
//! and as short a trigger as it can to every reader's memory, the
//! fleet's page included (ruling 9). A journal from before the index is
//! backfilled at a writer's open, one run per transaction
//! ([`Store::index_behind_with`]), and every row is held to brokkr-core's
//! projection of its event by ruling 4's comparison ([`first_unindexed`]).
//!
//! The row is derived, never exported and never hashed: the journal's
//! events stay the record (decisions 0001 and 0003), and no
//! `brokkr-runtime` decision reads a row (ruling 1).

use std::sync::LazyLock;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Params};

use crate::schema::all_present;
use crate::summary::text_at;
use crate::{patiently, Store, StoreError};

/// The meta key that records the backfill finished (ruling 8): its own,
/// so indexing an older journal sends no run's summary behind.
const BACKFILLED: &str = "event_index_backfill";

const TABLE: [&str; 1] = ["event_index"];
const INDEXES: [&str; 2] = ["event_index_stream", "event_index_trail"];
const TRIGGERS: [&str; 1] = ["event_index_of_an_event"];

/// The checkpoints one page of a seat's stream holds (ruling 7).
pub(crate) const CHECKPOINT_PAGE: usize = 50;

/// The one type a run's trail skips and a seat's stream reads.
const CHECKPOINTED: &str = EventType::EffectCheckpointed.as_str();

/// A row's columns, in the order [`projected`] gives them.
const COLUMNS: &str = "run_id, seq, type, attempt_id, member";

/// The row an `events` row named `event` projects to, in [`COLUMNS`]'
/// order, read from its envelope's text as brokkr-core's [`Indexed::of`]
/// reads the envelope: the type, the payload's attempt, and the
/// checkpoint's member or, where the payload carries no checkpoint, the
/// result's.
fn projected(event: &str) -> String {
    let envelope = format!("{event}.envelope");
    let text = |path: &str| text_at(&envelope, path);
    format!(
        "{event}.run_id, {event}.seq, {kind}, {attempt},
         CASE WHEN json_valid({envelope})
                   AND json_type({envelope}, '$.payload.checkpoint') IS NOT NULL
             THEN {checkpoint} ELSE {result} END",
        kind = text("$.type"),
        attempt = text("$.payload.attempt_id"),
        checkpoint = text("$.payload.checkpoint.member"),
        result = text("$.payload.result.member"),
    )
}

/// The table, its two indexes and its trigger, `IF NOT EXISTS` throughout
/// so racing openers agree, and the backfill marked unfinished, so a
/// journal that lost an object is walked again once it is restored. The
/// stream index keys a seat's checkpoints, `(run_id, attempt_id, member,
/// seq)`; the trail index holds every event but a checkpoint.
fn migration() -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS event_index (
            run_id TEXT NOT NULL, seq INTEGER NOT NULL,
            type TEXT, attempt_id TEXT, member TEXT,
            PRIMARY KEY (run_id, seq)
        ) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS event_index_stream
            ON event_index (run_id, attempt_id, member, seq);
        CREATE INDEX IF NOT EXISTS event_index_trail
            ON event_index (run_id, seq) WHERE type <> '{CHECKPOINTED}';
        CREATE TRIGGER IF NOT EXISTS event_index_of_an_event AFTER INSERT ON events BEGIN
            INSERT OR IGNORE INTO event_index ({COLUMNS}) SELECT {projected};
        END;
        DELETE FROM meta WHERE key = '{BACKFILLED}';",
        projected = projected("NEW"),
    )
}

/// Does this journal carry the table, both indexes and its trigger?
/// Pure reads of `sqlite_master`, so the steady-state open writes nothing.
pub(crate) fn index_intact(conn: &Connection) -> Result<bool, StoreError> {
    Ok(all_present(conn, "table", &TABLE)?
        && all_present(conn, "index", &INDEXES)?
        && all_present(conn, "trigger", &TRIGGERS)?)
}

/// Create the index, on a connection holding the write lock: the sixth
/// additive repair. Its rows come from the backfill that follows the
/// open, outside this lock.
pub(crate) fn migrate_index(conn: &Connection) -> Result<(), StoreError> {
    Ok(conn.execute_batch(&migration())?)
}

/// Record that the backfill finished; a journal created with the index
/// has nothing to backfill.
pub(crate) fn mark_indexed(conn: &Connection) -> Result<(), StoreError> {
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, 'finished')",
        [BACKFILLED],
    )?;
    Ok(())
}

/// Has the backfill finished?
fn indexed(conn: &Connection) -> Result<bool, StoreError> {
    let finished: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [BACKFILLED],
            |row| row.get(0),
        )
        .optional()?;
    Ok(finished.is_some())
}

/// The rows a run's events that have none yet project to, as the trigger
/// would have written each, read by the run's key and in seq order.
static UNINDEXED: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT {} FROM events AS e WHERE e.run_id = ?1 AND NOT EXISTS
             (SELECT 1 FROM event_index AS i WHERE i.run_id = e.run_id AND i.seq = e.seq)
         ORDER BY e.seq",
        projected("e")
    )
});

static INSERT_ROW: LazyLock<String> = LazyLock::new(|| {
    format!("INSERT OR IGNORE INTO event_index ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5)")
});

/// `run_id`'s rows yet to write, read without the write lock.
fn unindexed(conn: &Connection, run_id: &str) -> Result<Vec<Vec<Value>>, StoreError> {
    let mut statement = conn.prepare_cached(&UNINDEXED)?;
    let rows = statement.query_map([run_id], |row| (0..5).map(|at| row.get(at)).collect())?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// One run of the backfill, in its own transaction. An event's row is a
/// function of the event, which never changes, so rows read before the
/// lock are written as read, and a row a peer's append wrote meanwhile is
/// kept as it stands.
fn index_once(conn: &mut Connection, rows: &[Vec<Value>]) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    {
        let mut insert = tx.prepare_cached(&INSERT_ROW)?;
        for row in rows {
            insert.execute(params_from_iter(row))?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// A run's trail: every event but its checkpoints, in seq order, read
/// through the trail index alone. Each read names its index, since the
/// planner would take the primary key's range and filter the checkpoints
/// out one by one, and a journal that lost the index refuses the read
/// rather than reading it so.
static TRAIL: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_trail
         WHERE run_id = ?1 AND type <> '{CHECKPOINTED}' ORDER BY seq"
    )
});

/// A page of a seat's checkpoints, newest first, below a seq, read
/// through the stream index.
static STREAM: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id = ?2 AND member IS ?3 AND seq < ?4
           AND type = '{CHECKPOINTED}'
         ORDER BY seq DESC LIMIT {CHECKPOINT_PAGE}"
    )
});

/// One event's envelope, by its key.
const ENVELOPE: &str = "SELECT envelope FROM events WHERE run_id = ?1 AND seq = ?2";

/// A run's index rows in seq order, by its key: what ruling 4 compares.
const ROWS: &str = "SELECT seq, type, attempt_id, member FROM event_index
     WHERE run_id = ?1 ORDER BY seq";

/// The seqs `sql` selects with `bound`.
fn seqs(conn: &Connection, sql: &str, bound: impl Params) -> Result<Vec<u64>, StoreError> {
    let mut statement = conn.prepare_cached(sql)?;
    let found = statement.query_map(bound, |row| row.get(0))?;
    Ok(found.collect::<Result<_, _>>()?)
}

/// How a run's index stands against its events (ruling 4), at the first
/// seq where they part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFault {
    /// The event has no row.
    Missing,
    /// The event's row holds, in this column, the first, what the event
    /// does not project to.
    Column(&'static str),
    /// A row stands where the run holds no event.
    Stray,
}

/// The first seq where `run_id`'s index rows are not what `events`, the
/// run read whole, project to, and how: the rows streamed beside the
/// events, so the comparison holds the run and one row.
pub(crate) fn first_unindexed(
    conn: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
) -> Result<Option<(u64, IndexFault)>, StoreError> {
    let mut statement = conn.prepare_cached(ROWS)?;
    let mut rows = statement.query([run_id])?;
    let mut events = events.iter();
    loop {
        let found = match (events.next(), rows.next()?) {
            (None, None) => return Ok(None),
            (Some(event), None) => Some((event.seq, IndexFault::Missing)),
            (None, Some(row)) => Some((row.get(0)?, IndexFault::Stray)),
            (Some(event), Some(row)) => parted(event, row)?,
        };
        if found.is_some() {
            return Ok(found);
        }
    }
}

/// Where `row` parts from `event`, each the next of its kind in seq order.
fn parted(
    event: &EventEnvelope,
    row: &rusqlite::Row<'_>,
) -> Result<Option<(u64, IndexFault)>, StoreError> {
    let seq: u64 = row.get(0)?;
    if seq != event.seq {
        return Ok(Some(if seq > event.seq {
            (event.seq, IndexFault::Missing)
        } else {
            (seq, IndexFault::Stray)
        }));
    }
    let indexed = Indexed::of(event);
    let text = |text: Option<&str>| text.map_or(Value::Null, |text| Value::Text(text.into()));
    let expected = [
        ("type", text(Some(indexed.event_type.as_str()))),
        ("attempt_id", text(indexed.attempt_id)),
        ("member", text(indexed.member)),
    ];
    for (at, (column, value)) in expected.into_iter().enumerate() {
        if row.get::<_, Value>(at + 1)? != value {
            return Ok(Some((seq, IndexFault::Column(column))));
        }
    }
    Ok(None)
}

impl Store {
    /// Index every event that has no row, one run per transaction, and
    /// record the backfill finished (ruling 8). Every writable open makes
    /// it after the summary's catch-up, spending no patience: a peer's lock
    /// ends it, and the next open carries on where it stopped, since a run
    /// it indexed has no row left to write. A finished backfill reads one
    /// meta row. `between` runs on this connection after a run's rows are
    /// read and before its transaction, the instant the tests stage a
    /// peer's lock at. The runs are read by their key one at a time, so
    /// the memory is the largest run's rows.
    pub(crate) fn index_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        if indexed(&self.conn)? {
            return Ok(());
        }
        let mut after: Option<String> = None;
        while let Some(run_id) = self.run_after(after.as_deref())? {
            let rows = unindexed(&self.conn, &run_id)?;
            if !rows.is_empty() {
                between(&self.conn);
                let conn = &mut self.conn;
                patiently("index", self.patience, || index_once(conn, &rows))?;
            }
            after = Some(run_id);
        }
        let conn = &self.conn;
        patiently("index", self.patience, || mark_indexed(conn))
    }

    /// Whether this journal's event index is read (ruling 8): it carries
    /// the index and its backfill finished. A journal that does not is
    /// read as before, by a reader that cannot write to migrate it.
    pub fn reads_index(&self) -> Result<bool, StoreError> {
        Ok(index_intact(&self.conn)? && indexed(&self.conn)?)
    }

    /// A run's trail (ruling 7): the seq of every event but its
    /// checkpoints, in order.
    pub fn index_trail(&self, run_id: &str) -> Result<Vec<u64>, StoreError> {
        patiently("index", self.patience, || {
            seqs(&self.conn, &TRAIL, params![run_id])
        })
    }

    /// A page of a seat's checkpoints (ruling 7): the seqs of at most 50
    /// (`CHECKPOINT_PAGE`) of `run_id`'s checkpoints under `attempt_id`
    /// that name `member`, `None` for the seat's own, newest first, each
    /// before `before` where a page resumes.
    pub fn index_checkpoints(
        &self,
        run_id: &str,
        attempt_id: &str,
        member: Option<&str>,
        before: Option<u64>,
    ) -> Result<Vec<u64>, StoreError> {
        let before = before.map_or(i64::MAX, |seq| seq.min(i64::MAX as u64) as i64);
        patiently("index", self.patience, || {
            seqs(
                &self.conn,
                &STREAM,
                params![run_id, attempt_id, member, before],
            )
        })
    }

    /// `run_id`'s envelopes at `seqs`, in `seqs`' order, each read by its
    /// key, parsed and not verified: the run's opening verifies its chain
    /// first (ruling 5). A seq the run holds no event at is refused as no
    /// row, never skipped.
    pub fn envelopes_at(
        &self,
        run_id: &str,
        seqs: &[u64],
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        patiently("load", self.patience, || {
            let mut statement = self.conn.prepare_cached(ENVELOPE)?;
            seqs.iter()
                .map(|seq| {
                    let text: String =
                        statement.query_row(params![run_id, *seq as i64], |row| row.get(0))?;
                    Ok(serde_json::from_str(&text)?)
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests;
