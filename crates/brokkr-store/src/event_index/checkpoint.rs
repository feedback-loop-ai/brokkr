//! The checkpoints' dated facts (decision 0089 ruling 7): each
//! `effect/checkpointed` row keeps its time, the run's phase, the cost the
//! record reported or none, the five token counts and whether it is its
//! site's last finishing session record, with the mark that says they were
//! written. The facts are brokkr-core's one reading, so the writer, the
//! backfill and ruling 4's comparison derive them one way (decision 0071
//! ruling 5).
//!
//! What the record states the writer writes from the carried account at the
//! append it covers, one row per sealed checkpoint, so an append costs the
//! checkpoint it writes and never the run's earlier ones. A writer that
//! carries no account, as a second writable open does, writes the same
//! facts from the event alone, with the run's phase read by its key. A
//! session record that arrives later moves the flag off its site's earlier
//! one by the trigger that searches the partial index of session rows
//! alone, as #912's park trigger searches the parks' partial index. A
//! journal from before the facts is altered in place and walked once,
//! marked unfinished until the walk writes every row; a row core read no
//! facts for is still marked written, so a later open does not read its run
//! again.

use std::sync::LazyLock;

use brokkr_core::summary::{Account, Bounded, Checkpoint, NAME_BYTES};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};

use super::record;
use crate::event_index::{finished, mark_finished, CHECKPOINTED};
use crate::{head_of, patiently, Store, StoreError};

/// The meta key that records the checkpoint facts' backfill finished
/// (ruling 8): its own, so a journal from before them is walked once.
pub(super) const BACKFILLED: &str = "event_index_checkpoint_backfill";

/// The checkpoint facts' columns, in the order [`facts`] gives them: the
/// time, the run's phase, the cost and whether it was reported, the five
/// token counts, whether it is its site's session record, and the mark that
/// the row's facts were written at all. Each is NULL on a row that is not a
/// checkpoint.
pub(super) const COLUMNS: [&str; 11] = [
    "at_ms",
    "phase",
    "usd",
    "reported",
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_output_tokens",
    "session",
    "facts",
];

/// The two columns the event index already carries, which a checkpoint row
/// writes as a park row does.
const EXISTING: [&str; 2] = ["at_ms", "phase"];

/// The type of a column [`add_columns`] adds, the two it shares with a
/// park's left out.
fn kind(column: &str) -> &'static str {
    match column {
        "usd" => "REAL",
        _ => "INTEGER",
    }
}

/// The checkpoint facts' columns after the park's, the two it shares with
/// them left out, in [`facts`]' order: what a row's select adds.
pub(super) fn row_columns() -> Vec<&'static str> {
    COLUMNS
        .iter()
        .copied()
        .filter(|column| !EXISTING.contains(column))
        .collect()
}

/// The columns [`add_columns`] adds, declared for a table made anew.
pub(super) fn declared() -> String {
    COLUMNS
        .iter()
        .filter(|column| !EXISTING.contains(column))
        .map(|column| format!("{column} {}", kind(column)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Add the columns a journal from before the checkpoint facts lacks, one
/// `ALTER` each as SQLite has no `ADD COLUMN IF NOT EXISTS`, presence asked
/// rather than an error swallowed. The two shared with a park's are already
/// there.
pub(super) fn add_columns(conn: &Connection) -> Result<(), StoreError> {
    for column in COLUMNS.iter().filter(|column| !EXISTING.contains(column)) {
        let present = crate::schema::holds(conn, "event_index", [*column])?;
        if !present {
            conn.execute(
                &format!(
                    "ALTER TABLE event_index ADD COLUMN {column} {}",
                    kind(column)
                ),
                [],
            )?;
        }
    }
    Ok(())
}

/// The checkpoint facts in [`COLUMNS`]' order: the phase cut where it is
/// stored, the cost and its word, the five token counts, the session flag,
/// and the mark that says they were written. `None` is a checkpoint row core
/// read no facts for, whose stamp did not parse or which named no attempt:
/// it keeps no facts, but the mark is still written, so the row is
/// processed and a later open does not read its run for it again. The
/// writer's positional row and the comparison's named row take both from
/// here, so the cut has one home. A token count past the signed 64-bit
/// integer SQLite keeps is refused by name, never wrapped into a negative
/// fact (decision 0089 ruling 7).
pub(super) fn facts(checkpoint: Option<&Checkpoint>) -> Result<[Value; COLUMNS.len()], StoreError> {
    let Some(checkpoint) = checkpoint else {
        let mut values: [Value; COLUMNS.len()] = std::array::from_fn(|_| Value::Null);
        values[COLUMNS.len() - 1] = Value::Integer(1);
        return Ok(values);
    };
    let usage = checkpoint.meter.usage();
    let count = |column: &'static str, count: Option<u64>| match count {
        None => Ok(Value::Null),
        Some(count) => i64::try_from(count)
            .map(Value::Integer)
            .map_err(|_| StoreError::TokenCountOutOfRange(checkpoint.seq, column, count)),
    };
    let phase = checkpoint
        .phase
        .as_deref()
        .map(|phase| Bounded::within(phase, NAME_BYTES).cut);
    Ok([
        Value::Integer(checkpoint.at),
        phase.map_or(Value::Null, Value::Text),
        checkpoint.usd.map_or(Value::Null, Value::Real),
        Value::Integer(i64::from(checkpoint.reported)),
        count("input_tokens", usage.and_then(|usage| usage.input_tokens))?,
        count("output_tokens", usage.and_then(|usage| usage.output_tokens))?,
        count(
            "cache_read_tokens",
            usage.and_then(|usage| usage.cache_read_tokens),
        )?,
        count(
            "cache_write_tokens",
            usage.and_then(|usage| usage.cache_write_tokens),
        )?,
        count(
            "reasoning_output_tokens",
            usage.and_then(|usage| usage.reasoning_output_tokens),
        )?,
        Value::Integer(i64::from(checkpoint.session)),
        Value::Integer(1),
    ])
}

/// Whether `event` is a checkpoint whose record finishes a session: its
/// last `step` text ends "-session-finished", as [`Checkpoint::read`] reads
/// it. 0 where the record has no step or is no checkpoint.
pub(super) fn session_finished(event: &str) -> String {
    let record = record(event);
    format!(
        "COALESCE((SELECT CASE WHEN value LIKE '%-session-finished' THEN 1 ELSE 0 END
              FROM {record} WHERE key = 'step' AND type = 'text'
              ORDER BY id DESC LIMIT 1), 0)"
    )
}

/// The statement that moves the session flag off a site's earlier record
/// when `event` is a finishing checkpoint, inside the event's own
/// transaction: it searches the session partial index, never the run's
/// every event, so a writer from before the facts, whose event the trigger
/// indexes without them, still keeps the flag current (decision 0089
/// rulings 4 and 7).
pub(super) fn session_moved(event: &str, attempt: &str, member: &str) -> String {
    format!(
        "UPDATE event_index SET session = 0
                WHERE run_id = {event}.run_id
                  AND attempt_id IS {attempt}
                  AND member IS {member}
                  AND seq < {event}.seq
                  AND type = '{CHECKPOINTED}' AND session = 1"
    )
}

/// The predicate that names a `phase/entered` event's row on `events`,
/// guarded so an envelope no reader parses is never extracted: the partial
/// index and the writer's one phase lookup both read it.
fn phase_predicate() -> String {
    format!(
        "CASE WHEN json_valid(envelope) THEN json_extract(envelope, '$.type') END = '{}'",
        brokkr_core::EventType::PhaseEntered.as_str()
    )
}

/// The index that keys a `phase/entered` event's row on `events`: the phase
/// lookup reads the run's phase rows alone, never its every event.
pub(super) fn phase_index() -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS event_index_phase ON events (run_id, seq) WHERE {}",
        phase_predicate()
    )
}

/// The last `phase/entered` event before `seq`, by its key: the run's phase
/// when the event at `seq` was recorded, as brokkr-core's walk reads it.
pub(super) static PHASE_BEFORE: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT envelope FROM events
         WHERE run_id = ?1 AND seq < ?2 AND {}
         ORDER BY seq DESC LIMIT 1",
        phase_predicate()
    )
});

/// The statement that writes one checkpoint's facts, by its key.
pub(super) static WRITE: LazyLock<String> = LazyLock::new(|| {
    let assignments: Vec<String> = COLUMNS
        .iter()
        .enumerate()
        .map(|(at, column)| format!("{column} = ?{}", at + 3))
        .collect();
    format!(
        "UPDATE event_index SET {} WHERE run_id = ?1 AND seq = ?2",
        assignments.join(", ")
    )
});

/// Write one checkpoint row's facts, inside the caller's transaction.
fn write(tx: &Connection, run_id: &str, seq: u64, facts: &[Value]) -> Result<(), StoreError> {
    let mut bound: Vec<Value> = vec![Value::Text(run_id.to_string()), Value::Integer(seq as i64)];
    bound.extend_from_slice(facts);
    let mut statement = tx.prepare_cached(&WRITE)?;
    statement.execute(params_from_iter(bound))?;
    Ok(())
}

/// Write the facts of every checkpoint the append `sealed` carries, one row
/// each from the account carried over it: an append costs the checkpoints it
/// writes and never the run's earlier ones.
pub(crate) fn write_appended(
    tx: &Connection,
    account: &Account,
    sealed: &[brokkr_core::EventEnvelope],
) -> Result<(), StoreError> {
    for event in sealed {
        if event.event_type == brokkr_core::EventType::EffectCheckpointed {
            let facts = facts(account.checkpoint(event.seq))?;
            write(tx, account.run_id(), event.seq, &facts)?;
        }
    }
    Ok(())
}

/// The run's phase when the event at `seq` was recorded: the last
/// `phase/entered` event before it, read by its key through the phase index,
/// as brokkr-core's walk reads it from the events it carries.
fn phase_before(conn: &Connection, run_id: &str, seq: u64) -> Result<Option<String>, StoreError> {
    let found: Option<String> = conn
        .query_row(
            &PHASE_BEFORE,
            rusqlite::params![run_id, seq as i64],
            |row| row.get(0),
        )
        .optional()?;
    let Some(envelope) = found else {
        return Ok(None);
    };
    let event: brokkr_core::EventEnvelope = serde_json::from_str(&envelope)?;
    Ok(brokkr_core::summary::entered_phase(&event).map(str::to_string))
}

/// Write the facts of every checkpoint the append `sealed` carries from the
/// event alone, for a writer that carries no account of the run: the
/// record's own facts, with the run's phase before it read by its key. The
/// write is the update that moves a site's session flag off an earlier
/// record, through the row's trigger, so no append path leaves stale facts
/// or a false disagreement (decision 0089 rulings 4 and 7).
pub(crate) fn write_sealed(
    tx: &Connection,
    run_id: &str,
    sealed: &[brokkr_core::EventEnvelope],
) -> Result<(), StoreError> {
    for event in sealed {
        if event.event_type != brokkr_core::EventType::EffectCheckpointed {
            continue;
        }
        let phase = phase_before(tx, run_id, event.seq)?;
        let checkpoint = Checkpoint::read(event, phase.as_deref());
        write(tx, run_id, event.seq, &facts(checkpoint.as_ref())?)?;
    }
    Ok(())
}

/// Write the facts of every checkpoint of `account`'s run, at the head the
/// account covers: the backfill's walk and an adoption's whole write.
pub(crate) fn write_all(tx: &Connection, account: &Account) -> Result<(), StoreError> {
    for seq in account.checkpoint_rows() {
        let facts = facts(account.checkpoint(*seq))?;
        write(tx, account.run_id(), *seq, &facts)?;
    }
    Ok(())
}

/// Write every checkpoint row of `account`'s run that brokkr-core's
/// reading can express, leaving a row whose count the index's signed
/// columns cannot hold a lag for a reader to read whole and the doctor to
/// name: a legacy run is refused by name, never by failing the journal's
/// open (decision 0089 rulings 4 and 8).
pub(crate) fn write_all_available(tx: &Connection, account: &Account) -> Result<(), StoreError> {
    match write_all(tx, account) {
        Ok(()) | Err(StoreError::TokenCountOutOfRange(..)) => Ok(()),
        Err(error) => Err(error),
    }
}

/// One run of the checkpoint backfill, in its own transaction, written only
/// while the head is still the one `account` was read at; `true` where it
/// wrote, `false` where a peer moved the head and the run is left for the
/// next open, so the backfill is not marked finished over it.
pub(super) fn once(conn: &mut Connection, account: &Account) -> Result<bool, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (seq, hash) = head_of(&tx, account.run_id())?;
    let wrote = account.head() == (seq, hash.as_str());
    if wrote {
        write_all(&tx, account)?;
    }
    tx.commit()?;
    Ok(wrote)
}

/// The first run whose checkpoint rows need a write past the walk's cursor,
/// sought through the partial index of rows still to write. A journal may
/// hold the empty run id, which `run_id > ''` cannot name, so the first
/// step has no cursor at all.
fn stale(after: bool) -> String {
    let cursor = if after { "AND run_id > ?1" } else { "" };
    format!(
        "SELECT run_id FROM event_index INDEXED BY event_index_checkpoint_stale
         WHERE type = '{CHECKPOINTED}' AND facts IS NULL {cursor}
         ORDER BY run_id LIMIT 1"
    )
}

static STALE: LazyLock<String> = LazyLock::new(|| stale(true));
static STALE_FIRST: LazyLock<String> = LazyLock::new(|| stale(false));

/// One ascending walk of the runs whose checkpoint facts still need a
/// write, keyed by the runs' partial index so a journal whose facts are all
/// written reads one empty range and no run. It stops at the first run it
/// cannot load, leaving that run for a reader to read whole and the doctor
/// to name, and the backfill unfinished.
pub(super) struct Walk {
    after: Option<String>,
}

impl Walk {
    pub(super) fn new() -> Self {
        Self { after: None }
    }

    pub(super) fn next(&mut self, conn: &Connection) -> Result<Option<String>, StoreError> {
        let (sql, after) = match &self.after {
            Some(after) => (&*STALE, Some(after.as_str())),
            None => (&*STALE_FIRST, None),
        };
        let mut statement = conn.prepare_cached(sql)?;
        let found: Option<String> = statement
            .query_row(params_from_iter(after), |row| row.get(0))
            .optional()?;
        if let Some(run) = &found {
            self.after = Some(run.clone());
        }
        Ok(found)
    }
}

impl Store {
    /// Write every checkpoint's dated facts that a writer from before them,
    /// or a missing walk, left unwritten, one run per transaction (decision
    /// 0089 ruling 8), and let a run that does not load stay for a reader to
    /// read whole and the doctor to name. Every writable open makes it; the
    /// partial index holds only rows still to write, so a journal whose
    /// facts all stand finds none. A run whose head a peer moves while it is
    /// read writes nothing and leaves the backfill unfinished, so the
    /// completion claim never stands over a row it skipped. `between` runs
    /// on this connection after a run is read and before its transaction,
    /// the instant the tests stage a peer's lock at. The memory is the
    /// walk's cursor and the largest run's events.
    pub(crate) fn checkpoint_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        let mut walk = Walk::new();
        let mut unfinished = false;
        let mut cleared = false;
        while let Some(run_id) = walk.next(&self.conn)? {
            if !cleared {
                let conn = &self.conn;
                patiently("checkpoints", self.patience, || clear_backfilled(conn))?;
                cleared = true;
            }
            let Ok(events) = self.load(&run_id) else {
                unfinished = true;
                continue;
            };
            let account = Account::whole(&run_id, &events);
            between(&self.conn);
            let conn = &mut self.conn;
            let wrote = match patiently("checkpoints", self.patience, || once(conn, &account)) {
                Ok(wrote) => wrote,
                // A legacy checkpoint count the index's signed columns cannot
                // hold refuses its run by name, leaving its rows for a reader
                // to read whole and the doctor to name, and the backfill
                // unfinished rather than failing the whole journal's open
                // (decision 0089 rulings 4 and 8).
                Err(StoreError::TokenCountOutOfRange(..)) => {
                    unfinished = true;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !wrote {
                // A peer moved the head while the run was read: the walk
                // writes nothing for it, and the backfill stays unfinished
                // until an open writes it (rulings 4 and 8).
                unfinished = true;
            }
        }
        let done = !unfinished && (cleared || !finished(&self.conn, BACKFILLED)?);
        if done {
            let conn = &self.conn;
            patiently("checkpoints", self.patience, || {
                mark_finished(conn, BACKFILLED)
            })?;
        }
        Ok(())
    }
}

/// Mark the checkpoint facts' backfill unfinished, before a pass that has
/// rows to write: a run that does not load leaves it so.
fn clear_backfilled(conn: &Connection) -> Result<(), StoreError> {
    conn.execute("DELETE FROM meta WHERE key = ?1", [BACKFILLED])?;
    Ok(())
}

/// Does the table hold the checkpoint facts?
#[cfg(test)]
pub(super) fn checkpointed(conn: &Connection) -> Result<bool, StoreError> {
    crate::schema::holds(conn, "event_index", COLUMNS)
}

/// Has the checkpoint facts' backfill finished?
#[cfg(test)]
pub(super) fn backfilled(conn: &Connection) -> Result<bool, StoreError> {
    crate::event_index::finished(conn, BACKFILLED)
}
