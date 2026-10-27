//! The event index (decision 0089, slice 2): one row per event, keyed
//! `(run_id, seq)`, holding its type, the attempt and member it names, its
//! time and its id, so a run's trail, a seat's checkpoints and what caused
//! each row of a journal page are read by key, never by reading the run
//! whole (ruling 7).
//!
//! What the row states, the event alone states, so SQLite keeps it
//! (ruling 3): a trigger after each insert into `events` adds the row,
//! reading each name from the envelope's text only where it is text and
//! a repeated key by its last copy, as serde reads it ([`projected`]), so
//! every writer keeps it, an older binary's and an import's included. No body is held (ruling 10). The time is the
//! envelope's `recorded_at`, which a trigger of its own cuts to the run
//! summary's bound only past it, whichever insert wrote it. Every reader
//! parses the journal's schema when it opens it, so the index adds as few
//! objects and as short a trigger as it can to every reader's memory, the
//! fleet's page included (ruling 9). A journal from before the index is
//! backfilled at a writer's open, one run per transaction
//! ([`Store::index_behind_with`]), and every row is held to brokkr-core's
//! projection of its event by ruling 4's comparison ([`first_unindexed`]).
//!
//! The row is derived, never exported and never hashed: the journal's
//! events stay the record (decisions 0001 and 0003), and no
//! `brokkr-runtime` decision reads a row (ruling 1).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::summary::{parks, Account, Bounded, Park, NAME_BYTES};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Params};

use crate::schema::all_present;
use brokkr_core::summary::STAMP_BYTES;

use crate::summary::{cut_stamp, stamp_of, text_at};
use crate::{head_of, patiently, Disagreement, Store, StoreError};

/// The meta key that records the backfill finished (ruling 8): its own,
/// so indexing an older journal sends no run's summary behind.
const BACKFILLED: &str = "event_index_backfill";

const TABLE: [&str; 1] = ["event_index"];
const INDEXES: [&str; 6] = [
    "event_index_stream",
    "event_index_trail",
    "event_index_cause",
    "event_index_park",
    "event_index_park_stale",
    "event_index_park_facts",
];
const TRIGGERS: [&str; 3] = [
    "event_index_of_an_event",
    "event_index_stamp_bounded",
    "event_index_park_moved",
];

mod compare;

pub(crate) use compare::{first_parted, place};

/// The checkpoints one page of a seat's stream holds (ruling 7).
pub(crate) const CHECKPOINT_PAGE: usize = 50;

/// The one type a run's trail skips and a seat's stream reads.
pub(crate) const CHECKPOINTED: &str = EventType::EffectCheckpointed.as_str();

/// The type a park's row is (decision 0089 ruling 7).
const PARKED: &str = EventType::RunParked.as_str();

/// A row's columns, in the order [`projected`] gives them.
const COLUMNS: &str = "run_id, seq, type, attempt_id, member, recorded_at, event_id";

/// The dated facts a park's row keeps (decision 0089 ruling 7), and the
/// stamp that says they were computed through its run's head: a writer
/// from before them, or one that lost them, leaves `facts_seq` NULL and
/// the row is read whole until the next writable open refolds it.
const PARK_COLUMNS: [&str; 6] = [
    "phase",
    "at_ms",
    "answered_ms",
    "ended_ms",
    "span_class",
    "facts_seq",
];

/// The row an `events` row named `event` projects to, in [`COLUMNS`]'
/// order, read from its envelope's text as brokkr-core's [`Indexed::of`]
/// reads the envelope: the type, the payload's attempt, the
/// checkpoint's member or, where the payload carries no checkpoint, the
/// result's, the stamp whole, which the bounding trigger cuts, and the
/// envelope's own id, where it is text. The
/// payload is walked by its entries, since serde keeps the last copy of a
/// key the payload repeats where `json_extract` reads the first, and its
/// event's hash, taken over the payload as serde parses it, cannot see
/// the first.
fn projected(event: &str) -> String {
    let envelope = format!("{event}.envelope");
    let payload = format!(
        "json_each(CASE WHEN json_valid({envelope}) THEN {envelope} ELSE '{{}}' END, '$.payload')"
    );
    let record = format!(
        "(SELECT value AS record, type AS kind FROM {payload}
              WHERE key IN ('checkpoint', 'result')
              ORDER BY key = 'checkpoint' DESC, id DESC LIMIT 1),
         json_each(CASE WHEN kind = 'object' THEN record ELSE '{{}}' END)"
    );
    format!(
        "{event}.run_id, {event}.seq, {kind}, {attempt}, {member}, {stamp}, {id}",
        kind = text_at(&envelope, "$.type"),
        attempt = last_text(&payload, "attempt_id"),
        member = last_text(&record, "member"),
        stamp = stamp_of(&envelope),
        id = text_at(&envelope, "$.event_id"),
    )
}

/// The last entry named `key` among `entries`, a `json_each` walk, where
/// that entry is text, else NULL, as serde reads an object that repeats a
/// key.
fn last_text(entries: &str, key: &str) -> String {
    format!(
        "(SELECT CASE WHEN type = 'text' THEN value END FROM {entries}
              WHERE key = '{key}' ORDER BY id DESC LIMIT 1)"
    )
}

/// The table and its six indexes, `IF NOT EXISTS` throughout so racing
/// openers agree; the park trigger is dropped and made anew, so a copy
/// from before the facts index is replaced, and the backfill is marked
/// unfinished, so a journal that lost an object is walked again once it
/// is restored. The stream index keys a seat's checkpoints, `(run_id,
/// attempt_id, member, seq)`; the trail index holds every event but a
/// checkpoint; the cause index keys an event by its id, `(run_id,
/// event_id)`; the park index keys a window's parks, `(span_class,
/// at_ms)`; and the facts index holds the park rows whose dated facts
/// are written, so a later event sends those facts behind by searching
/// the park rows alone ([`park_moved`]), never the run's every event. A
/// stamp past its bound is cut by a trigger of its own, which fires only
/// then, after the event's trigger or the backfill inserted the row, so
/// the event's trigger reads the stamp once.
fn migration() -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS event_index (
            run_id TEXT NOT NULL, seq INTEGER NOT NULL,
            type TEXT, attempt_id TEXT, member TEXT, recorded_at TEXT, event_id TEXT,
            phase TEXT, at_ms INTEGER, answered_ms INTEGER, ended_ms INTEGER,
            span_class INTEGER, facts_seq INTEGER,
            PRIMARY KEY (run_id, seq)
        ) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS event_index_stream
            ON event_index (run_id, attempt_id, member, seq);
        CREATE INDEX IF NOT EXISTS event_index_trail
            ON event_index (run_id, seq) WHERE type <> '{CHECKPOINTED}';
        CREATE INDEX IF NOT EXISTS event_index_cause ON event_index (run_id, event_id);
        CREATE INDEX IF NOT EXISTS event_index_park
            ON event_index (span_class, at_ms) WHERE type = '{PARKED}';
        CREATE INDEX IF NOT EXISTS event_index_park_stale
            ON event_index (run_id) WHERE type = '{PARKED}' AND facts_seq IS NULL;
        CREATE INDEX IF NOT EXISTS event_index_park_facts
            ON event_index (run_id, seq) WHERE type = '{PARKED}' AND facts_seq IS NOT NULL;
        DROP TRIGGER IF EXISTS event_index_park_moved;
        CREATE TRIGGER event_index_park_moved AFTER INSERT ON events BEGIN
            {moved};
        END;
        CREATE TRIGGER IF NOT EXISTS event_index_of_an_event AFTER INSERT ON events BEGIN
            INSERT OR IGNORE INTO event_index ({COLUMNS}) SELECT {projected};
        END;
        CREATE TRIGGER IF NOT EXISTS event_index_stamp_bounded AFTER INSERT ON event_index
            WHEN length(CAST(NEW.recorded_at AS BLOB)) > {STAMP_BYTES} BEGIN
            UPDATE event_index SET recorded_at = {cut}
                WHERE run_id = NEW.run_id AND seq = NEW.seq;
        END;
        DELETE FROM meta WHERE key = '{BACKFILLED}';",
        moved = park_moved("NEW"),
        projected = projected("NEW"),
        cut = cut_stamp("NEW.recorded_at"),
    )
}

/// The statement that sends every park of the event's run behind its new
/// head, inside the event's own transaction (decision 0089 ruling 7):
/// `event` names the trigger's new row, `NEW`. Its `IN` selects the seqs
/// of the park rows whose facts are written, through
/// `event_index_park_facts`, so a checkpoint costs the park rows before
/// it, never the run's every event; the outer update then names each row
/// by the primary key. The facts are recomputed at the head the append
/// covers, which [`write_parks`] writes; a park the statement leaves NULL
/// is a lag the next writable open refolds.
fn park_moved(event: &str) -> String {
    format!(
        "UPDATE event_index SET facts_seq = NULL
                WHERE run_id = {event}.run_id AND seq IN (
                    SELECT seq FROM event_index INDEXED BY event_index_park_facts
                    WHERE run_id = {event}.run_id AND seq < {event}.seq
                      AND type = '{PARKED}' AND facts_seq IS NOT NULL)"
    )
}

/// Does the table hold `column`?
fn holds(conn: &Connection, column: &str) -> Result<bool, StoreError> {
    let found: rusqlite::Result<i64> = conn.query_row(
        "SELECT count(*) FROM pragma_table_info('event_index') WHERE name = ?1",
        [column],
        |row| row.get(0),
    );
    Ok(found? == 1)
}

/// Does the table hold its time? A table without it is the shape an
/// unreleased build of this slice made, which [`migrate_index`] rebuilds.
fn timed(conn: &Connection) -> Result<bool, StoreError> {
    holds(conn, "recorded_at")
}

/// Does the table hold each event's id? A table without it is the shape a
/// build before slice 2b made, which [`migrate_index`] rebuilds; a build
/// that made it reads this shape as its own, and its trigger, which lives
/// in the journal, writes the id for it.
fn keyed(conn: &Connection) -> Result<bool, StoreError> {
    holds(conn, "event_id")
}

/// Does the table hold a park's dated facts? A table without them is the
/// shape a build before decision 0089 slice 4 made, which
/// [`migrate_index`] rebuilds.
fn parked(conn: &Connection) -> Result<bool, StoreError> {
    for column in PARK_COLUMNS {
        if !holds(conn, column)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Does this journal carry the table, with its time and its ids, its three
/// indexes and both triggers? Pure reads, so the steady-state open writes
/// nothing.
pub(crate) fn index_intact(conn: &Connection) -> Result<bool, StoreError> {
    Ok(all_present(conn, "table", &TABLE)?
        && timed(conn)?
        && keyed(conn)?
        && parked(conn)?
        && all_present(conn, "index", &INDEXES)?
        && all_present(conn, "trigger", &TRIGGERS)?)
}

/// Send behind every park whose facts are not its run's head's, as the
/// trigger sends one: the repair recreates the trigger, and a writer
/// whose copy was missing may have moved a park while it was gone, so a
/// stamp the trigger did not write must not stand at the head the run now
/// has. The walk that follows the open refolds the rows this sends behind
/// (rulings 4 and 8).
fn send_parks_behind() -> String {
    format!(
        "UPDATE event_index SET facts_seq = NULL
         WHERE type = '{PARKED}' AND facts_seq IS NOT NULL
           AND facts_seq IS NOT (SELECT max(seq) FROM events
                                  WHERE events.run_id = event_index.run_id)"
    )
}

/// Create the index, on a connection holding the write lock: the sixth
/// additive repair. A table without its time or its ids, and the trigger
/// that wrote it, are dropped first, since every row is derived. Its rows
/// come from the backfill that follows the open, outside this lock.
pub(crate) fn migrate_index(conn: &Connection) -> Result<(), StoreError> {
    if !(timed(conn)? && keyed(conn)? && parked(conn)?) {
        let dropped = conn.execute_batch(
            "DROP TRIGGER IF EXISTS event_index_of_an_event;
             DROP TRIGGER IF EXISTS event_index_park_moved;
             DROP TABLE IF EXISTS event_index;",
        );
        dropped?;
    }
    conn.execute_batch(&migration())?;
    Ok(conn.execute_batch(&send_parks_behind())?)
}

/// Record that the backfill `key` names finished.
pub(crate) fn mark_finished(conn: &Connection, key: &str) -> Result<(), StoreError> {
    let marked = conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, 'finished')",
        [key],
    );
    marked?;
    Ok(())
}

/// Has the backfill `key` names finished?
pub(crate) fn finished(conn: &Connection, key: &str) -> Result<bool, StoreError> {
    let finished: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(finished.is_some())
}

/// Record that the backfill finished, once its walk found every event's
/// row: the one place the key is written, a new journal's included.
fn mark_indexed(conn: &Connection) -> Result<(), StoreError> {
    mark_finished(conn, BACKFILLED)
}

/// Has the backfill finished?
fn indexed(conn: &Connection) -> Result<bool, StoreError> {
    finished(conn, BACKFILLED)
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
    format!("INSERT OR IGNORE INTO event_index ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
});

/// `run_id`'s rows yet to write, read without the write lock.
fn unindexed(conn: &Connection, run_id: &str) -> Result<Vec<Vec<Value>>, StoreError> {
    let mut statement = conn.prepare_cached(&UNINDEXED)?;
    let rows = statement.query_map([run_id], |row| (0..7).map(|at| row.get(at)).collect())?;
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
pub(crate) static TRAIL: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_trail
         WHERE run_id = ?1 AND type <> '{CHECKPOINTED}' ORDER BY seq"
    )
});

/// A seat's checkpoints under an attempt and a member, newest first,
/// between two seqs, both held, read through the stream index, as many as
/// the last bound: a page, or the seat view's page and one more
/// (`seat_read.rs`).
pub(crate) static STREAM: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id = ?2 AND member IS ?3 AND seq >= ?4 AND seq <= ?5
           AND type = '{CHECKPOINTED}'
         ORDER BY seq DESC LIMIT ?6"
    )
});

/// One event's envelope, by its key.
const ENVELOPE: &str = "SELECT envelope FROM events WHERE run_id = ?1 AND seq = ?2";

/// A run's index rows in seq order, by its key: what ruling 4 compares.
const ROWS: &str = "SELECT seq, type, attempt_id, member, recorded_at, event_id,
     phase, at_ms, answered_ms, ended_ms, span_class, facts_seq
     FROM event_index WHERE run_id = ?1 ORDER BY seq";

/// The last seq of a run's events that `event_id` names, NULL where none
/// does, and whether a row of the run holds no id, each read through the
/// cause index, in one statement so both are of one snapshot.
const CAUSE: &str = "SELECT
     (SELECT max(seq) FROM event_index INDEXED BY event_index_cause
          WHERE run_id = ?1 AND event_id = ?2),
     EXISTS (SELECT 1 FROM event_index INDEXED BY event_index_cause
          WHERE run_id = ?1 AND event_id IS NULL)";

/// The seqs `sql` selects with `bound`.
pub(crate) fn seqs(
    conn: &Connection,
    sql: &str,
    bound: impl Params,
) -> Result<Vec<u64>, StoreError> {
    let mut statement = conn.prepare_cached(sql)?;
    let found = statement.query_map(bound, |row| row.get(0))?;
    Ok(found.collect::<Result<_, _>>()?)
}

/// How a run's index stands against its events (ruling 4), at the first
/// seq where they part, or its seat rows against its fold, at the first
/// row's place where they part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFault {
    /// The event, or the fold's row, has no row.
    Missing,
    /// The row holds, in this column, the first, what the event projects
    /// or the fold's row holds otherwise.
    Column(&'static str),
    /// A row stands where the run holds no event, or the fold no row.
    Stray,
}

/// Each run id the index holds rows of and `runs` does not, with its
/// first such seq, in run id order: the rows a sweep that walks `runs`
/// never reaches, read in one statement, so a run a peer creates meanwhile
/// is never named (its `runs` row commits before its first event).
const RUNLESS: &str = "SELECT run_id, min(seq) FROM event_index
     WHERE run_id NOT IN (SELECT run_id FROM runs) GROUP BY run_id ORDER BY run_id";

/// Every index row of a run the journal does not hold, named as a stray
/// at its run's first such seq (ruling 4): no event stands behind it,
/// since every event's run is a `runs` row.
pub(crate) fn runless(conn: &Connection) -> Result<Vec<Disagreement>, StoreError> {
    let mut statement = conn.prepare_cached(RUNLESS)?;
    let found = statement.query_map([], |row| {
        Ok(Disagreement::Indexed {
            run_id: row.get(0)?,
            seq: row.get(1)?,
            fault: IndexFault::Stray,
        })
    })?;
    Ok(found.collect::<Result<_, _>>()?)
}

/// How a run's rows stand against its events (ruling 4): the first seq
/// where a row is not its event's, a park whose dated facts are not its
/// head's (a lag the next writable open refolds, never a disagreement),
/// or every row its event's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexReading {
    Agrees,
    Behind,
    Parted(u64, IndexFault),
}

/// The first seq where `run_id`'s index rows are not what `events`, the
/// run read whole, project to, and how: the rows streamed beside the
/// events, so the comparison holds the run and one row. A park whose
/// facts are not written at the run's head is a lag: its immutable
/// columns are still compared, and its dated facts are left for the
/// refold that writes them.
pub(crate) fn first_unindexed(
    conn: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
) -> Result<IndexReading, StoreError> {
    let mut statement = conn.prepare_cached(ROWS)?;
    let rows = statement.query([run_id])?;
    let parks: BTreeMap<u64, Park> = parks(events)
        .into_iter()
        .map(|park| (park.seq, park))
        .collect();
    let head = events.last().map_or(0, |event| event.seq);
    let mut stale = false;
    let parted = compare::first_parted_at(
        rows,
        events.iter().map(|event| {
            (
                event.seq,
                compare::projects(event, parks.get(&event.seq), head),
            )
        }),
        &mut stale,
    )?;
    Ok(match parted {
        Some((seq, fault)) => IndexReading::Parted(seq, fault),
        None if stale => IndexReading::Behind,
        None => IndexReading::Agrees,
    })
}

/// The statement that writes one park's dated facts, by the seq its run
/// head covers.
const WRITE_PARK: &str = "UPDATE event_index SET phase = ?3, at_ms = ?4, answered_ms = ?5,
         ended_ms = ?6, span_class = ?7, facts_seq = ?8
     WHERE run_id = ?1 AND seq = ?2";

/// Write every park row of `account`'s run, at the head the account covers
/// (decision 0089 ruling 7), inside the caller's transaction. A phase is
/// a group key, cut where it is stored; the moments stay absent while
/// they have not happened, and the class is written with the end, or 64
/// while the park is open. A `run/parked` row core read no park for is one
/// whose stamp did not parse: it keeps no dated facts, but its `facts_seq`
/// is still the head, so the partial index of rows still to write records
/// that it was processed and no later open reads the run for it again.
/// `facts_seq` otherwise says the facts cover that head, so a row a writer
/// from before them leaves is off its run's head and read whole until the
/// next writable open refolds it.
pub(crate) fn write_parks(tx: &Connection, account: &Account) -> Result<(), StoreError> {
    let head = account.head().0 as i64;
    let parks: BTreeMap<u64, Park> = account
        .parks()
        .into_iter()
        .map(|park| (park.seq, park))
        .collect();
    let mut statement = tx.prepare_cached(WRITE_PARK)?;
    for seq in account.park_rows() {
        let park = parks.get(seq);
        let phase = park
            .and_then(|park| park.phase.as_deref())
            .map(|phase| Bounded::within(phase, NAME_BYTES).cut);
        statement.execute(params![
            account.run_id(),
            *seq as i64,
            phase,
            park.map(|park| park.at),
            park.and_then(|park| park.answered),
            park.and_then(|park| park.ended),
            park.map(|park| i64::from(park.span_class())),
            head,
        ])?;
    }
    Ok(())
}

/// One run of the parks' backfill, in its own transaction, written only
/// while the head is still the one `account` was read at.
fn park_once(conn: &mut Connection, account: &Account) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (seq, hash) = head_of(&tx, account.run_id())?;
    if account.head() == (seq, hash.as_str()) {
        write_parks(&tx, account)?;
    }
    tx.commit()?;
    Ok(())
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

    /// The first run past `after` whose park rows do not have their dated
    /// facts, read through the partial index of the rows still to write,
    /// or the first such run.
    fn park_after(&self, after: Option<&str>) -> Result<Option<String>, StoreError> {
        let next = match after {
            None => self.conn.query_row(
                &format!(
                    "SELECT run_id FROM event_index INDEXED BY event_index_park_stale
                     WHERE type = '{PARKED}' AND facts_seq IS NULL ORDER BY run_id LIMIT 1"
                ),
                [],
                |row| row.get(0),
            ),
            Some(after) => self.conn.query_row(
                &format!(
                    "SELECT run_id FROM event_index INDEXED BY event_index_park_stale
                     WHERE type = '{PARKED}' AND facts_seq IS NULL AND run_id > ?1
                     ORDER BY run_id LIMIT 1"
                ),
                [after],
                |row| row.get(0),
            ),
        };
        Ok(next.optional()?)
    }

    /// Write every park's dated facts that a writer from before them left
    /// unwritten, one run per transaction (decision 0089 ruling 8), and
    /// let a run that does not load stay for a reader to read whole and
    /// the doctor to name. Every writable open makes it; the partial
    /// index holds only the rows still to write, so a journal whose parks
    /// are all kept reads it as empty. `between` runs on this connection
    /// after a run is read and before its transaction, the instant the
    /// tests stage a peer's lock at. The runs are read by their key one
    /// at a time, so the memory is the largest run's.
    pub(crate) fn park_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        let mut after: Option<String> = None;
        while let Some(run_id) = self.park_after(after.as_deref())? {
            if let Ok(events) = self.load(&run_id) {
                let account = Account::whole(&run_id, &events);
                between(&self.conn);
                let conn = &mut self.conn;
                patiently("parks", self.patience, || park_once(conn, &account))?;
            }
            after = Some(run_id);
        }
        Ok(())
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
        let below = before.map_or(i64::MAX, |seq| seq.min(i64::MAX as u64) as i64) - 1;
        let page = CHECKPOINT_PAGE as i64;
        patiently("index", self.patience, || {
            seqs(
                &self.conn,
                &STREAM,
                params![run_id, attempt_id, member, i64::MIN, below, page],
            )
        })
    }

    /// What caused a journal page's rows (ruling 7): the seq of `run_id`'s
    /// event that each of `event_ids` names, the last where an id repeats,
    /// as the run view maps them, and no seq for an id no event of the run
    /// carries. `None` where a row of the run holds no id: a build from
    /// before the ids reads this table as its own and, opening the journal
    /// while a current writer's walk is under way, backfills rows without
    /// them, and such a row names no cause, so the reader reads the run
    /// whole rather than miss one.
    pub fn index_causes(
        &self,
        run_id: &str,
        event_ids: &[&str],
    ) -> Result<Option<BTreeMap<String, u64>>, StoreError> {
        patiently("index", self.patience, || {
            let mut statement = self.conn.prepare_cached(CAUSE)?;
            let mut causes = BTreeMap::new();
            for id in event_ids {
                let read = |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?));
                let (seq, unkeyed): (Option<i64>, bool) =
                    statement.query_row(params![run_id, id], read)?;
                if unkeyed {
                    return Ok(None);
                }
                if let Some(seq) = seq {
                    causes.insert(id.to_string(), seq as u64);
                }
            }
            Ok(Some(causes))
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
                    // The row's text as SQLite holds it, borrowed, so the
                    // parse is the one copy of the envelope, as the
                    // opening's is.
                    let parsed = statement.query_row(params![run_id, *seq as i64], |row| {
                        Ok(serde_json::from_str(row.get_ref(0)?.as_str()?))
                    })?;
                    Ok(parsed?)
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod park_tests;
#[cfg(test)]
mod tests;
