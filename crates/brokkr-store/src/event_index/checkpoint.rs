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
//! checkpoint it writes and never the run's earlier ones. A session record
//! that arrives later moves the flag off its site's earlier one by the
//! trigger that searches the partial index of session rows alone, as #912's
//! park trigger searches the parks' partial index. A journal from before the
//! facts is altered in place and walked once, marked unfinished until the
//! walk writes every row; a row core read no facts for is still marked
//! written, so a later open does not read its run again.

use std::sync::LazyLock;

use brokkr_core::summary::{Account, Bounded, Checkpoint, NAME_BYTES};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};

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
/// here, so the cut has one home.
pub(super) fn facts(checkpoint: Option<&Checkpoint>) -> [Value; COLUMNS.len()] {
    let Some(checkpoint) = checkpoint else {
        let mut values: [Value; COLUMNS.len()] = std::array::from_fn(|_| Value::Null);
        values[COLUMNS.len() - 1] = Value::Integer(1);
        return values;
    };
    let usage = checkpoint.meter.usage();
    let count =
        |count: Option<u64>| count.map_or(Value::Null, |count| Value::Integer(count as i64));
    let phase = checkpoint
        .phase
        .as_deref()
        .map(|phase| Bounded::within(phase, NAME_BYTES).cut);
    [
        Value::Integer(checkpoint.at),
        phase.map_or(Value::Null, Value::Text),
        checkpoint.usd.map_or(Value::Null, Value::Real),
        Value::Integer(i64::from(checkpoint.reported)),
        count(usage.and_then(|usage| usage.input_tokens)),
        count(usage.and_then(|usage| usage.output_tokens)),
        count(usage.and_then(|usage| usage.cache_read_tokens)),
        count(usage.and_then(|usage| usage.cache_write_tokens)),
        count(usage.and_then(|usage| usage.reasoning_output_tokens)),
        Value::Integer(i64::from(checkpoint.session)),
        Value::Integer(1),
    ]
}

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
            let facts = facts(account.checkpoint(event.seq));
            write(tx, account.run_id(), event.seq, &facts)?;
        }
    }
    Ok(())
}

/// Write the facts of every checkpoint of `account`'s run, at the head the
/// account covers: the backfill's walk and an adoption's whole write.
pub(crate) fn write_all(tx: &Connection, account: &Account) -> Result<(), StoreError> {
    for seq in account.checkpoint_rows() {
        let facts = facts(account.checkpoint(*seq));
        write(tx, account.run_id(), *seq, &facts)?;
    }
    Ok(())
}

/// One run of the checkpoint backfill, in its own transaction, written only
/// while the head is still the one `account` was read at.
pub(super) fn once(conn: &mut Connection, account: &Account) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (seq, hash) = head_of(&tx, account.run_id())?;
    if account.head() == (seq, hash.as_str()) {
        write_all(&tx, account)?;
    }
    tx.commit()?;
    Ok(())
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
    /// facts all stand finds none. `between` runs on this connection after a
    /// run is read and before its transaction, the instant the tests stage a
    /// peer's lock at. The memory is the walk's cursor and the largest
    /// run's events.
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
            patiently("checkpoints", self.patience, || once(conn, &account))?;
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
