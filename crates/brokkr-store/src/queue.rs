//! The dispatcher's queue (decision 0068 ruling 1): entries waiting to
//! become runs, and the operator commands that placed and moved them,
//! held in the journal's own database so they outlive the session that
//! wrote them.
//!
//! Queue state is not run events. An entry exists before any run does,
//! belongs to no run's hash chain, and moves (its place, its hold) where
//! an event never may; so it lives in tables of its own, beside `runs`
//! and `events` and never inside them. Nothing here writes an event, and
//! nothing in the append path reads a queue table, so the events table's
//! append-only guard is untouched. What must not move is guarded as the
//! events are, by trigger: an entry's payload, priority and waits, the
//! run it started, and every operator command once written.
//!
//! The payload is the launch an entry will make, as the runtime encodes
//! it; the store keeps it verbatim and never reads it. Priority and waits
//! are operator data, stored and listed here; admission weighs them.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use thiserror::Error;

use crate::{now_rfc3339, patiently, Store, StoreError};

/// The queue storage a journal records under `meta.queue_schema`, apart
/// from `database_schema`: the queue is additive, so an older binary still
/// reads a journal that carries one, and only the queue verbs refuse a
/// queue stored in a version this binary does not know.
pub(crate) const QUEUE_SCHEMA: u32 = 1;

/// The queue's tables and guards. `IF NOT EXISTS` throughout, so racing
/// openers that both found it missing agree.
pub(crate) const MIGRATION_QUEUE_V1: &str = r#"
CREATE TABLE IF NOT EXISTS queue_entries (
    entry_id INTEGER PRIMARY KEY,
    payload TEXT NOT NULL,
    priority INTEGER NOT NULL,
    added_at TEXT NOT NULL,
    state TEXT NOT NULL,
    position INTEGER,
    run_id TEXT UNIQUE REFERENCES runs(run_id)
);
CREATE TABLE IF NOT EXISTS queue_waits (
    entry_id INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    awaits INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    condition TEXT NOT NULL,
    PRIMARY KEY (entry_id, awaits)
);
CREATE TABLE IF NOT EXISTS queue_commands (
    seq INTEGER PRIMARY KEY,
    entry_id INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    command TEXT NOT NULL,
    position INTEGER,
    operator TEXT NOT NULL,
    reason TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS queue_entries_fixed
    BEFORE UPDATE OF entry_id, payload, priority, added_at ON queue_entries
    BEGIN SELECT RAISE(ABORT, 'a queue entry''s launch, priority and arrival are fixed'); END;
CREATE TRIGGER IF NOT EXISTS queue_entries_run_once
    BEFORE UPDATE OF run_id ON queue_entries
    WHEN OLD.run_id IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'a queue entry''s run is written once'); END;
CREATE TRIGGER IF NOT EXISTS queue_entries_kept
    BEFORE DELETE ON queue_entries
    BEGIN SELECT RAISE(ABORT, 'queue entries are never deleted; drop one'); END;
CREATE TRIGGER IF NOT EXISTS queue_waits_fixed
    BEFORE UPDATE ON queue_waits
    BEGIN SELECT RAISE(ABORT, 'queue waits are fixed'); END;
CREATE TRIGGER IF NOT EXISTS queue_waits_kept
    BEFORE DELETE ON queue_waits
    BEGIN SELECT RAISE(ABORT, 'queue waits are fixed'); END;
CREATE TRIGGER IF NOT EXISTS queue_commands_append_only_update
    BEFORE UPDATE ON queue_commands
    BEGIN SELECT RAISE(ABORT, 'queue commands are append-only'); END;
CREATE TRIGGER IF NOT EXISTS queue_commands_append_only_delete
    BEFORE DELETE ON queue_commands
    BEGIN SELECT RAISE(ABORT, 'queue commands are append-only'); END;
"#;

/// The guards [`MIGRATION_QUEUE_V1`] installs, by name, so an open can
/// ask whether a journal still carries every one.
pub(crate) const QUEUE_TRIGGERS: [&str; 7] = [
    "queue_entries_fixed",
    "queue_entries_run_once",
    "queue_entries_kept",
    "queue_waits_fixed",
    "queue_waits_kept",
    "queue_commands_append_only_update",
    "queue_commands_append_only_delete",
];

/// The queue's tables, in the order [`MIGRATION_QUEUE_V1`] creates them.
const QUEUE_TABLES: [&str; 3] = ["queue_entries", "queue_waits", "queue_commands"];

/// Is the queue whole: its storage version recorded and every guard in
/// place? Pure reads, so the steady-state open takes no write lock. A
/// journal that records no queue is from before it, and the queue is
/// installed. One that records a queue whose table is gone has lost what
/// the queue held (SQLite drops a table's guards with it), and is refused
/// rather than recreated empty; so only a guard missing from a table that
/// is still there is ever repaired.
pub(crate) fn queue_intact(conn: &Connection) -> Result<bool, StoreError> {
    if queue_schema(conn)?.is_none() {
        return Ok(false);
    }
    let named = |kind: &str| -> Result<Vec<String>, StoreError> {
        Ok(conn
            .prepare("SELECT name FROM sqlite_master WHERE type = ?1")?
            .query_map([kind], |row| row.get(0))?
            .collect::<Result<_, _>>()?)
    };
    let tables = named("table")?;
    if let Some(table) = QUEUE_TABLES
        .into_iter()
        .find(|table| !tables.iter().any(|name| name == table))
    {
        return Err(QueueRefusal::TableLost { table }.into());
    }
    let triggers = named("trigger")?;
    Ok(QUEUE_TRIGGERS
        .iter()
        .all(|guard| triggers.iter().any(|name| name == guard)))
}

/// Create the queue's tables and guards and record its version, on a
/// connection already holding the write lock.
pub(crate) fn migrate_queue(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(MIGRATION_QUEUE_V1)?;
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('queue_schema', ?1)",
        [&QUEUE_SCHEMA.to_string()],
    )?;
    Ok(())
}

/// The queue storage a journal records, or `None` for one that never held
/// a queue: a journal opened read-only from before the queue existed.
fn queue_schema(conn: &Connection) -> Result<Option<u32>, StoreError> {
    let recorded: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'queue_schema'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(recorded.map(|value| value.parse().unwrap_or(0)))
}

/// Refuse a queue stored in a version this binary does not know; answer
/// whether there is a queue at all.
fn queue_present(conn: &Connection) -> Result<bool, StoreError> {
    match queue_schema(conn)? {
        None => Ok(false),
        Some(QUEUE_SCHEMA) => Ok(true),
        Some(found) => Err(QueueRefusal::SchemaMismatch { found }.into()),
    }
}

/// A queue entry's id: assigned at `add`, never reused, since no entry is
/// ever deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(pub i64);

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for EntryId {
    type Err = std::num::ParseIntError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(EntryId)
    }
}

/// A word a queue table holds that this binary does not know. Only the
/// store's own writers write these columns, so reading one is a journal
/// written by something else, refused rather than guessed at.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("queue column {column} holds '{word}', which this brokkr does not know")]
pub(crate) struct UnknownWord {
    pub(crate) column: &'static str,
    pub(crate) word: String,
}

/// Read one word of a closed vocabulary: the member whose `word` it is.
fn read_word<T: Copy>(
    value: ValueRef<'_>,
    column: &'static str,
    all: &[T],
    word: fn(T) -> &'static str,
) -> FromSqlResult<T> {
    let text = value.as_str()?;
    all.iter()
        .copied()
        .find(|known| word(*known) == text)
        .ok_or_else(|| {
            FromSqlError::Other(Box::new(UnknownWord {
                column,
                word: text.to_string(),
            }))
        })
}

/// What an entry waits on in the entry it names (decision 0068 ruling 2).
/// A named ending (decision 0064) joins when #317 builds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitOn {
    /// The awaited entry's run completed.
    Completed,
    /// The awaited entry's run reached any terminal status.
    Ended,
}

impl WaitOn {
    pub const ALL: [WaitOn; 2] = [WaitOn::Completed, WaitOn::Ended];

    pub fn word(self) -> &'static str {
        match self {
            WaitOn::Completed => "completed",
            WaitOn::Ended => "ended",
        }
    }
}

impl FromSql for WaitOn {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        read_word(value, "queue_waits.condition", &WaitOn::ALL, WaitOn::word)
    }
}

/// One entry another waits for, and on what. Written `ENTRY:CONDITION`,
/// as `3:completed`, and read back the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wait {
    pub entry: EntryId,
    pub on: WaitOn,
}

impl std::fmt::Display for Wait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.entry, self.on.word())
    }
}

/// Text that is not a [`Wait`].
#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "'{0}' is not ENTRY:CONDITION, with CONDITION one of {words}",
    words = WaitOn::ALL.map(WaitOn::word).join(", ")
)]
pub struct NotAWait(pub String);

impl std::str::FromStr for Wait {
    type Err = NotAWait;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let refused = || NotAWait(text.to_string());
        let (entry, on) = text.split_once(':').ok_or_else(refused)?;
        Ok(Wait {
            entry: entry.parse().map_err(|_| refused())?,
            on: WaitOn::ALL
                .into_iter()
                .find(|known| known.word() == on)
                .ok_or_else(refused)?,
        })
    }
}

/// The stored state word. Claimed is not one: an entry is claimed exactly
/// when it names a run, and that column is its one home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Queued,
    Held,
    Dropped,
}

impl Standing {
    const ALL: [Standing; 3] = [Standing::Queued, Standing::Held, Standing::Dropped];

    fn word(self) -> &'static str {
        match self {
            Standing::Queued => "queued",
            Standing::Held => "held",
            Standing::Dropped => "dropped",
        }
    }
}

impl FromSql for Standing {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        read_word(value, "queue_entries.state", &Standing::ALL, Standing::word)
    }
}

/// Where an entry stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryState {
    /// Waiting its turn.
    Queued,
    /// Kept in its place, and not to be started until released.
    Held,
    /// Started: the run it produced, recorded once.
    Claimed { run: String },
    /// Taken out of the queue by the operator.
    Dropped,
}

impl EntryState {
    /// The state as the queue's readouts name it.
    pub fn word(&self) -> &'static str {
        match self {
            EntryState::Queued => Standing::Queued.word(),
            EntryState::Held => Standing::Held.word(),
            EntryState::Claimed { .. } => "claimed",
            EntryState::Dropped => Standing::Dropped.word(),
        }
    }

    /// The run a claimed entry started.
    pub fn run(&self) -> Option<&str> {
        match self {
            EntryState::Claimed { run } => Some(run),
            EntryState::Queued | EntryState::Held | EntryState::Dropped => None,
        }
    }

    fn of(standing: Standing, run: Option<String>) -> EntryState {
        match (run, standing) {
            (Some(run), Standing::Queued | Standing::Held | Standing::Dropped) => {
                EntryState::Claimed { run }
            }
            (None, Standing::Queued) => EntryState::Queued,
            (None, Standing::Held) => EntryState::Held,
            (None, Standing::Dropped) => EntryState::Dropped,
        }
    }
}

/// An entry as `list` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub id: EntryId,
    /// Its place in the queue from 1, while it waits; `None` once claimed.
    pub position: Option<u32>,
    pub state: EntryState,
    pub priority: i64,
    pub waits: Vec<Wait>,
    /// The launch, as the runtime encoded it.
    pub payload: String,
    pub added_at: String,
}

/// Who issued an operator command, and why (decision 0068 ruling 1: each
/// is journaled with its reason).
#[derive(Debug, Clone, Copy)]
pub struct Attribution<'a> {
    pub operator: &'a str,
    pub reason: &'a str,
}

/// A new entry.
#[derive(Debug, Clone, Copy)]
pub struct NewEntry<'a> {
    pub payload: &'a str,
    pub priority: i64,
    pub waits: &'a [Wait],
}

/// The operator commands that change an entry already queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueCommand {
    /// Put the entry at this place, from 1.
    Move {
        to: u32,
    },
    Hold,
    Release,
    Drop,
}

impl QueueCommand {
    fn word(self) -> &'static str {
        match self {
            QueueCommand::Move { .. } => "move",
            QueueCommand::Hold => "hold",
            QueueCommand::Release => "release",
            QueueCommand::Drop => "drop",
        }
    }
}

/// Why the queue refused, with the operator's text. Nothing was written.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum QueueRefusal {
    #[error("queue entry {0} does not exist")]
    UnknownEntry(EntryId),
    #[error("a new queue entry cannot wait for entry {0}, which does not exist")]
    UnknownWait(EntryId),
    #[error("a new queue entry names entry {0} to wait for twice")]
    DuplicateWait(EntryId),
    #[error("queue entry {entry} cannot move to place {to}: the queue's places run 1 to {last}")]
    PastTheEnds { entry: EntryId, to: u32, last: u32 },
    #[error("queue entry {0} is already held")]
    AlreadyHeld(EntryId),
    #[error("queue entry {0} is not held")]
    NotHeld(EntryId),
    #[error("queue entry {0} is held; release it first")]
    Held(EntryId),
    #[error("queue entry {entry} is claimed by run '{run}'")]
    Claimed { entry: EntryId, run: String },
    #[error("queue entry {0} was dropped")]
    Dropped(EntryId),
    #[error("run '{run}' already started queue entry {entry}")]
    RunTaken { run: String, entry: EntryId },
    #[error("queue storage {found} unsupported (want {QUEUE_SCHEMA})")]
    SchemaMismatch { found: u32 },
    /// A journal that records its queue, and has lost one of its tables.
    #[error(
        "the journal records a queue but its {table} table is gone; what the queue held cannot \
         be read, and it is never recreated empty"
    )]
    TableLost { table: &'static str },
}

/// The standing of one entry, read inside the transaction that acts on it.
fn standing(tx: &Transaction<'_>, entry: EntryId) -> Result<EntryState, StoreError> {
    let row: Option<(Standing, Option<String>)> = tx
        .query_row(
            "SELECT state, run_id FROM queue_entries WHERE entry_id = ?1",
            params![entry.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (standing, run) = row.ok_or(QueueRefusal::UnknownEntry(entry))?;
    Ok(EntryState::of(standing, run))
}

/// Does the table hold a row with this key?
fn exists(
    tx: &Transaction<'_>,
    query: &str,
    key: &dyn rusqlite::ToSql,
) -> Result<bool, StoreError> {
    Ok(tx.query_row(query, [key], |_| Ok(())).optional()?.is_some())
}

/// The entries in the queue's order: queued and held, by place. Every
/// unclaimed entry's state is read as a [`Standing`], so one this brokkr
/// does not know refuses the command rather than being placed.
fn waiting(tx: &Transaction<'_>) -> Result<Vec<EntryId>, StoreError> {
    let unclaimed = tx
        .prepare(
            "SELECT entry_id, state FROM queue_entries
             WHERE run_id IS NULL ORDER BY position",
        )?
        .query_map([], |row| Ok((EntryId(row.get(0)?), row.get(1)?)))?
        .collect::<Result<Vec<(EntryId, Standing)>, _>>()?;
    Ok(unclaimed
        .into_iter()
        .filter(|(_, standing)| *standing != Standing::Dropped)
        .map(|(entry, _)| entry)
        .collect())
}

/// Write the queue's order back as places from 1.
fn renumber(tx: &Transaction<'_>, order: &[EntryId]) -> Result<(), StoreError> {
    for (place, entry) in (1u32..).zip(order) {
        tx.execute(
            "UPDATE queue_entries SET position = ?1 WHERE entry_id = ?2",
            params![place, entry.0],
        )?;
    }
    Ok(())
}

/// Take an entry out of the queue's order, closing the gap it leaves.
fn leave_order(tx: &Transaction<'_>, entry: EntryId) -> Result<(), StoreError> {
    tx.execute(
        "UPDATE queue_entries SET position = NULL WHERE entry_id = ?1",
        params![entry.0],
    )?;
    renumber(tx, &waiting(tx)?)
}

fn set_standing(
    tx: &Transaction<'_>,
    entry: EntryId,
    standing: Standing,
) -> Result<(), StoreError> {
    tx.execute(
        "UPDATE queue_entries SET state = ?1 WHERE entry_id = ?2",
        params![standing.word(), entry.0],
    )?;
    Ok(())
}

/// Journal one operator command, in the transaction that carried it out.
fn record(
    tx: &Transaction<'_>,
    entry: EntryId,
    word: &str,
    position: Option<u32>,
    by: Attribution<'_>,
) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO queue_commands (entry_id, command, position, operator, reason, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry.0,
            word,
            position,
            by.operator,
            by.reason,
            now_rfc3339()
        ],
    )?;
    Ok(())
}

/// Refuse a command the entry's standing cannot take.
fn admit(entry: EntryId, state: EntryState, command: QueueCommand) -> Result<(), QueueRefusal> {
    use QueueCommand::{Drop, Hold, Move, Release};
    match (state, command) {
        (EntryState::Dropped, _) => Err(QueueRefusal::Dropped(entry)),
        (EntryState::Claimed { run }, _) => Err(QueueRefusal::Claimed { entry, run }),
        (EntryState::Held, Hold) => Err(QueueRefusal::AlreadyHeld(entry)),
        (EntryState::Queued, Release) => Err(QueueRefusal::NotHeld(entry)),
        (EntryState::Queued, Move { .. } | Hold | Drop)
        | (EntryState::Held, Move { .. } | Release | Drop) => Ok(()),
    }
}

/// Refuse waits that name an entry twice or one the queue never held.
/// Checked before the new entry exists, so no wait can name it, and every
/// entry waited for is older than it: waits never close a cycle.
fn admit_waits(tx: &Transaction<'_>, waits: &[Wait]) -> Result<(), StoreError> {
    for (index, wait) in waits.iter().enumerate() {
        if waits[..index].iter().any(|seen| seen.entry == wait.entry) {
            return Err(QueueRefusal::DuplicateWait(wait.entry).into());
        }
        let query = "SELECT 1 FROM queue_entries WHERE entry_id = ?1";
        if !exists(tx, query, &wait.entry.0)? {
            return Err(QueueRefusal::UnknownWait(wait.entry).into());
        }
    }
    Ok(())
}

fn add_once(
    conn: &mut Connection,
    new: NewEntry<'_>,
    by: Attribution<'_>,
) -> Result<EntryId, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    queue_present(&tx)?;
    admit_waits(&tx, new.waits)?;
    let place = waiting(&tx)?.len() as u32 + 1;
    tx.execute(
        "INSERT INTO queue_entries (payload, priority, added_at, state, position)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new.payload,
            new.priority,
            now_rfc3339(),
            Standing::Queued.word(),
            place
        ],
    )?;
    let entry = EntryId(tx.last_insert_rowid());
    for wait in new.waits {
        tx.execute(
            "INSERT INTO queue_waits (entry_id, awaits, condition) VALUES (?1, ?2, ?3)",
            params![entry.0, wait.entry.0, wait.on.word()],
        )?;
    }
    record(&tx, entry, "add", Some(place), by)?;
    tx.commit()?;
    Ok(entry)
}

fn command_once(
    conn: &mut Connection,
    entry: EntryId,
    command: QueueCommand,
    by: Attribution<'_>,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    queue_present(&tx)?;
    admit(entry, standing(&tx, entry)?, command)?;
    let position = apply(&tx, entry, command)?;
    record(&tx, entry, command.word(), position, by)?;
    tx.commit()?;
    Ok(())
}

/// Carry out an admitted command on `entry`, and say the place a move
/// gave it.
fn apply(
    tx: &Transaction<'_>,
    entry: EntryId,
    command: QueueCommand,
) -> Result<Option<u32>, StoreError> {
    match command {
        QueueCommand::Move { to } => move_entry(tx, entry, to).map(Some),
        QueueCommand::Hold => set_standing(tx, entry, Standing::Held).map(|()| None),
        QueueCommand::Release => set_standing(tx, entry, Standing::Queued).map(|()| None),
        QueueCommand::Drop => {
            set_standing(tx, entry, Standing::Dropped)?;
            leave_order(tx, entry)?;
            Ok(None)
        }
    }
}

/// Move `entry` to place `to` among the waiting entries, refusing a place
/// past either end.
fn move_entry(tx: &Transaction<'_>, entry: EntryId, to: u32) -> Result<u32, StoreError> {
    let mut order = waiting(tx)?;
    let last = order.len() as u32;
    if to == 0 || to > last {
        return Err(QueueRefusal::PastTheEnds { entry, to, last }.into());
    }
    order.retain(|id| *id != entry);
    order.insert(to as usize - 1, entry);
    renumber(tx, &order)?;
    Ok(to)
}

fn claim_once(conn: &mut Connection, entry: EntryId, run: &str) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    queue_present(&tx)?;
    match standing(&tx, entry)? {
        EntryState::Queued => {}
        EntryState::Held => return Err(QueueRefusal::Held(entry).into()),
        EntryState::Dropped => return Err(QueueRefusal::Dropped(entry).into()),
        EntryState::Claimed { run } => return Err(QueueRefusal::Claimed { entry, run }.into()),
    }
    if !exists(&tx, "SELECT 1 FROM runs WHERE run_id = ?1", &run)? {
        return Err(StoreError::RunNotFound(run.to_string()));
    }
    let taken: Option<i64> = tx
        .query_row(
            "SELECT entry_id FROM queue_entries WHERE run_id = ?1",
            params![run],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(other) = taken {
        let run = run.to_string();
        return Err(QueueRefusal::RunTaken {
            run,
            entry: EntryId(other),
        }
        .into());
    }
    tx.execute(
        "UPDATE queue_entries SET run_id = ?1 WHERE entry_id = ?2",
        params![run, entry.0],
    )?;
    leave_order(&tx, entry)?;
    tx.commit()?;
    Ok(())
}

fn list_once(conn: &Connection) -> Result<Vec<QueueEntry>, StoreError> {
    if !queue_present(conn)? {
        return Ok(Vec::new());
    }
    let mut entries = conn.prepare(
        "SELECT entry_id, position, state, run_id, priority, payload, added_at
         FROM queue_entries WHERE state != ?1
         ORDER BY position IS NULL, position, entry_id",
    )?;
    let mut listed = entries
        .query_map([Standing::Dropped.word()], |row| {
            Ok(QueueEntry {
                id: EntryId(row.get(0)?),
                position: row.get(1)?,
                state: EntryState::of(row.get(2)?, row.get(3)?),
                priority: row.get(4)?,
                payload: row.get(5)?,
                added_at: row.get(6)?,
                waits: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut waits = conn
        .prepare("SELECT awaits, condition FROM queue_waits WHERE entry_id = ?1 ORDER BY awaits")?;
    for entry in &mut listed {
        entry.waits = waits
            .query_map([entry.id.0], |row| {
                Ok(Wait {
                    entry: EntryId(row.get(0)?),
                    on: row.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
    }
    Ok(listed)
}

impl Store {
    /// Queue a new entry at the end of the queue, journaling the `add`.
    /// Every entry it waits for must already be in the queue.
    pub fn queue_add(
        &mut self,
        new: NewEntry<'_>,
        by: Attribution<'_>,
    ) -> Result<EntryId, StoreError> {
        let conn = &mut self.conn;
        patiently("queue_add", self.patience, || add_once(conn, new, by))
    }

    /// Move, hold, release or drop an entry, journaling the command. The
    /// check and the write are one `Immediate` transaction, so a peer's
    /// command between them cannot slip past the refusal.
    pub fn queue_command(
        &mut self,
        entry: EntryId,
        command: QueueCommand,
        by: Attribution<'_>,
    ) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_command", self.patience, || {
            command_once(conn, entry, command, by)
        })
    }

    /// Record the run a queued entry started, once: the entry leaves the
    /// queue's order, and neither the entry's run nor the run's entry can
    /// be rewritten after. The dispatcher's claim (#430's third slice)
    /// is its caller.
    pub fn queue_claim(&mut self, entry: EntryId, run: &str) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_claim", self.patience, || {
            claim_once(conn, entry, run)
        })
    }

    /// The queue: waiting entries in order, then the claimed ones by id.
    /// Dropped entries are not listed. A journal from before the queue
    /// has an empty one.
    pub fn queue_list(&self) -> Result<Vec<QueueEntry>, StoreError> {
        patiently("queue_list", self.patience, || list_once(&self.conn))
    }
}

#[cfg(test)]
mod tests;
