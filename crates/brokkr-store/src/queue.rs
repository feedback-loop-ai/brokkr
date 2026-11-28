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
//! run it started, and every operator command and re-pin once written.
//!
//! The payload is the launch an entry will make, as the runtime encodes
//! it; the store keeps it verbatim and never reads it. An operator's
//! re-pin writes a new one beside it, which the entry stands for from
//! then on. A latch is a realm-drift hold admission found on an entry,
//! kept verbatim as the runtime encodes it too, and it stands until the
//! entry is re-pinned after it: nothing else clears one, and a latch or a
//! re-pin is written only over the pin and the latch its caller read
//! ([`Seen`]). Priority and
//! waits are operator data, stored and listed here; the queue's order
//! reads priority first, and admission weighs the waits.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{now_rfc3339, patiently, Store, StoreError};

mod beside;
mod claim;
mod cooldown;
mod driven;
mod entry;
mod holder;
mod refusal;
pub(crate) mod storage;

pub use beside::Seen;
use beside::{beside_once, Beside};
pub use claim::{claim_stands, Claim, ClaimFacts, Claiming, Disposal, Generation, StartRefusal};
pub(crate) use cooldown::{
    active_in_force as cooldowns_in_force_conn, insert as insert_cooldowns,
    insert_idempotent as insert_cooldowns_idempotent, time_of as cooldown_time_of,
    until_second as cooldown_until_second,
};
pub use cooldown::{
    Cooldown, CooldownAggregate, CooldownRound, Cooling, ProbeHold, ProbeHoldId, RoundAppend,
    RowEnd,
};
#[cfg(test)]
use entry::UnknownWord;
use entry::{read_word, Standing};
pub use entry::{EntryState, Latch, NewEntry, NotAWait, QueueCommand, WaitOn};
pub use refusal::{
    CooldownDuplicate, CooldownRoundMismatch, QueueRefusal, UnsettledHold, UnwrittenHold,
    UnwrittenRow,
};
pub(crate) use storage::{
    has_foreign_queue_triggers, migrate_queue, queue_intact, queue_schema, QUEUE_SCHEMA,
    QUEUE_SCHEMA_KEY,
};
use storage::{
    latch_row, queue_stored, wait_row, Stored, PINNED_PAYLOAD, PINNED_SEQ, STANDING_LATCH, WAITS,
};

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

/// An entry as `list` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub id: EntryId,
    /// Its place in the queue from 1, while it waits; `None` once claimed.
    pub position: Option<u32>,
    pub state: EntryState,
    pub priority: i64,
    pub waits: Vec<Wait>,
    /// The launch, as the runtime encoded it: the one the entry was last
    /// re-pinned to, else the one it was queued with.
    pub payload: String,
    /// The `repin` command that wrote that launch, by its seq; `None`
    /// while it stands for the launch it was queued with.
    pub(crate) pin: Option<i64>,
    pub added_at: String,
    /// The realm-drift hold latched on it since it was last re-pinned,
    /// the latest one; `None` when none stands.
    pub latch: Option<Latch>,
}

impl QueueEntry {
    /// Construct a [`QueueEntry`] from an external storage port backend
    /// (decision 0126).
    pub fn new(
        id: EntryId,
        position: Option<u32>,
        state: EntryState,
        priority: i64,
        waits: Vec<Wait>,
        payload: String,
        added_at: String,
    ) -> Self {
        Self {
            id,
            position,
            state,
            priority,
            waits,
            payload,
            pin: None,
            added_at,
            latch: None,
        }
    }

    /// The pin and the latch read here, which a latch or a re-pin of this
    /// entry is written over.
    pub fn seen(&self) -> Seen {
        Seen {
            pin: self.pin,
            latch: self.latch.as_ref().map(|latch| latch.seq),
        }
    }
}

/// Who issued an operator command, and why (decision 0068 ruling 1: each
/// is journaled with its reason).
#[derive(Debug, Clone, Copy)]
pub struct Attribution<'a> {
    pub operator: &'a str,
    pub reason: &'a str,
}

/// Whether a `merged` record of a `queue_entries` row stands, read in the
/// statement that reads its standing, the command's word bound as `?2`.
const MERGED: &str = "EXISTS (SELECT 1 FROM queue_commands
    WHERE queue_commands.entry_id = queue_entries.entry_id AND command = ?2)";

/// The standing of one entry, read inside the transaction that acts on it.
fn standing(tx: &Transaction<'_>, entry: EntryId) -> Result<EntryState, StoreError> {
    let row: Option<(Standing, Option<String>, bool)> = tx
        .query_row(
            &format!("SELECT state, run_id, {MERGED} FROM queue_entries WHERE entry_id = ?1"),
            params![entry.0, QueueCommand::Merged.word()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (standing, run, merged) = row.ok_or(QueueRefusal::UnknownEntry(entry))?;
    Ok(EntryState::of(standing, run, merged))
}

/// Does the table hold a row with this key?
fn exists(
    tx: &Transaction<'_>,
    query: &str,
    key: &dyn rusqlite::ToSql,
) -> Result<bool, StoreError> {
    Ok(tx.query_row(query, [key], |_| Ok(())).optional()?.is_some())
}

/// The queue's one order (decision 0068 ruling 2, and the operator's
/// ruling on #1191: "`queue move` orders entries within one priority, and
/// priority always wins"): priority, higher first, then place, then id.
/// Every read of the queue sorts by it, every write of the places writes
/// them in it, and every read gives each waiting entry its place in it,
/// so a queue journaled before this order, its places still arrival's,
/// lists and moves by the same places.
const ORDER: &str = "priority DESC, position, entry_id";

/// The completer's page of waiting entries ([`Store::queue_waiting`]):
/// unclaimed rows in either waiting standing, newest first, within a limit.
const WAITING_SQL: &str = "SELECT entry_id, state FROM queue_entries
     WHERE run_id IS NULL AND state IN (?1, ?2)
     ORDER BY entry_id DESC LIMIT ?3";

/// A waiting entry and the priority that orders it first.
type Waiting = (EntryId, i64);

/// The entries in the queue's order: queued, held and launching, since a
/// claimed entry keeps its place until it is linked. Every unclaimed
/// entry's state is read as a [`Standing`], so one this brokkr does not
/// know refuses the command rather than being placed.
fn waiting(conn: &Connection) -> Result<Vec<Waiting>, StoreError> {
    let query = format!(
        "SELECT entry_id, state, priority FROM queue_entries
         WHERE run_id IS NULL ORDER BY {ORDER}"
    );
    let read = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(Waiting, Standing)> {
        Ok(((EntryId(row.get(0)?), row.get(2)?), row.get(1)?))
    };
    let mut statement = conn.prepare(&query)?;
    let unclaimed = (statement.query_map([], read)?).collect::<Result<Vec<_>, _>>()?;
    Ok(unclaimed
        .into_iter()
        .filter(|(_, standing)| *standing != Standing::Dropped)
        .map(|(entry, _)| entry)
        .collect())
}

/// The place `entry` holds in `order`, from 1.
fn place_in(order: &[Waiting], entry: EntryId) -> u32 {
    order.iter().take_while(|(id, _)| *id != entry).count() as u32 + 1
}

/// Write the queue's order back as places from 1.
fn renumber(tx: &Transaction<'_>, order: &[Waiting]) -> Result<(), StoreError> {
    for (place, (entry, _)) in (1u32..).zip(order) {
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

/// Journal one operator command, in the transaction that carried it out,
/// and say its place in the journal of commands.
fn record_at(
    tx: &Transaction<'_>,
    entry: EntryId,
    word: &str,
    position: Option<u32>,
    by: Attribution<'_>,
    now: &str,
) -> Result<i64, StoreError> {
    tx.execute(
        "INSERT INTO queue_commands (entry_id, command, position, operator, reason, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![entry.0, word, position, by.operator, by.reason, now],
    )?;
    Ok(tx.last_insert_rowid())
}

fn record(
    tx: &Transaction<'_>,
    entry: EntryId,
    word: &str,
    position: Option<u32>,
    by: Attribution<'_>,
) -> Result<i64, StoreError> {
    record_at(tx, entry, word, position, by, &now_rfc3339())
}

/// Refuse a command the entry's standing cannot take: a `merged` record
/// only of an entry that started a run, every other command only of one
/// that waits.
fn admit(entry: EntryId, state: EntryState, command: QueueCommand) -> Result<(), QueueRefusal> {
    use QueueCommand::{Drop, Hold, Merged, Move, Release};
    match (state, command) {
        (EntryState::Dropped, _) => Err(QueueRefusal::Dropped(entry)),
        (EntryState::Claimed { .. }, Merged) => Ok(()),
        (EntryState::Claimed { run, .. }, _) => Err(QueueRefusal::Claimed { entry, run }),
        (EntryState::Launching, _) => Err(QueueRefusal::Launching(entry)),
        (EntryState::Queued | EntryState::Held, Merged) => Err(QueueRefusal::Unstarted(entry)),
        (EntryState::Held, Hold) => Err(QueueRefusal::AlreadyHeld(entry)),
        (EntryState::Queued, Release) => Err(QueueRefusal::NotHeld(entry)),
        (EntryState::Queued, Move { .. } | Hold | Drop)
        | (EntryState::Held, Move { .. } | Release | Drop) => Ok(()),
    }
}

/// Refuse waits on a condition the column's reader would refuse, such as
/// `unclassified`, and waits that name an entry twice or one the queue
/// never held. Checked before the new entry exists, so no wait can name
/// it, and every entry waited for is older than it: waits never close a
/// cycle.
fn admit_waits(tx: &Transaction<'_>, waits: &[Wait]) -> Result<(), StoreError> {
    for (index, wait) in waits.iter().enumerate() {
        if !WaitOn::ALL.contains(&wait.on) {
            return Err(QueueRefusal::NotACondition(*wait).into());
        }
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

pub(crate) fn add_at(
    conn: &Connection,
    new: NewEntry<'_>,
    by: Attribution<'_>,
    now: &str,
) -> Result<EntryId, StoreError> {
    let tx = Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;
    queue_stored(&tx)?;
    admit_waits(&tx, new.waits)?;
    let end = waiting(&tx)?.len() as u32 + 1;
    tx.execute(
        "INSERT INTO queue_entries (payload, priority, added_at, state, position)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new.payload, new.priority, now, Standing::Queued.word(), end],
    )?;
    let entry = EntryId(tx.last_insert_rowid());
    // Added at the end, it stands last among its own priority; the
    // renumber writes that place.
    let order = waiting(&tx)?;
    renumber(&tx, &order)?;
    let place = place_in(&order, entry);
    for wait in new.waits {
        tx.execute(
            "INSERT INTO queue_waits (entry_id, awaits, condition) VALUES (?1, ?2, ?3)",
            params![entry.0, wait.entry.0, wait.on.word()],
        )?;
    }
    record_at(&tx, entry, "add", Some(place), by, now)?;
    tx.commit()?;
    Ok(entry)
}

fn add_once(
    conn: &mut Connection,
    new: NewEntry<'_>,
    by: Attribution<'_>,
) -> Result<EntryId, StoreError> {
    add_at(conn, new, by, &now_rfc3339())
}

fn command_once(
    conn: &mut Connection,
    entry: EntryId,
    command: QueueCommand,
    by: Attribution<'_>,
) -> Result<Option<u32>, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    queue_stored(&tx)?;
    admit(entry, standing(&tx, entry)?, command)?;
    let position = apply(&tx, entry, command)?;
    record(&tx, entry, command.word(), position, by)?;
    tx.commit()?;
    Ok(position)
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
        QueueCommand::Merged => Ok(None),
    }
}

/// Move `entry` toward place `to` among the waiting entries, refusing a
/// place past either end, and say the place it holds after. Priority
/// still orders first: the entry moves only among those of its own
/// priority, so a place across a priority's edge leaves it at that edge.
fn move_entry(tx: &Transaction<'_>, entry: EntryId, to: u32) -> Result<u32, StoreError> {
    let order = waiting(tx)?;
    let last = order.len() as u32;
    if to == 0 || to > last {
        return Err(QueueRefusal::PastTheEnds { entry, to, last }.into());
    }
    let (moved, mut order): (Vec<Waiting>, Vec<Waiting>) =
        order.into_iter().partition(|(id, _)| *id == entry);
    let at = to as usize - 1;
    order.splice(at..at, moved);
    // Stable, so the move's order within each priority stands.
    order.sort_by_key(|(_, priority)| std::cmp::Reverse(*priority));
    renumber(tx, &order)?;
    Ok(place_in(&order, entry))
}

/// The entries `filter` selects with `key`, in the queue's order, each
/// with its waits, the launch it stands for and the latch that stands on
/// it. A waiting entry's place is read as its place in the queue's order,
/// not as stored. A version 1 queue holds no pins and no latches. The
/// caller reads it under one snapshot, so the order the rows list in and
/// the places read for them are one state's. `between` runs after the
/// rows are read and before their places are: the instant a peer's
/// command lands, which the tests stage.
fn entries(
    conn: &Connection,
    stored: Stored,
    filter: &str,
    key: &dyn rusqlite::ToSql,
    between: &mut dyn FnMut(),
) -> Result<Vec<QueueEntry>, StoreError> {
    let (payload, pin, latched) = match stored {
        Stored::V2 | Stored::V3 | Stored::V4 | Stored::V5 | Stored::V6 | Stored::V7 => {
            (PINNED_PAYLOAD, PINNED_SEQ, true)
        }
        Stored::V1 => ("payload", "NULL", false),
    };
    let mut entries = conn.prepare(&format!(
        "SELECT entry_id, position, state, run_id, priority, {payload}, added_at, {pin}, {MERGED}
         FROM queue_entries WHERE {filter}
         ORDER BY position IS NULL, {ORDER}"
    ))?;
    let mut listed = entries
        .query_map(params![key, QueueCommand::Merged.word()], |row| {
            Ok(QueueEntry {
                id: EntryId(row.get(0)?),
                position: row.get(1)?,
                state: EntryState::of(row.get(2)?, row.get(3)?, row.get(8)?),
                priority: row.get(4)?,
                payload: row.get(5)?,
                added_at: row.get(6)?,
                pin: row.get(7)?,
                waits: Vec::new(),
                latch: None,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    between();
    let mut waits = conn.prepare(WAITS)?;
    let mut latches = latched.then(|| conn.prepare(STANDING_LATCH)).transpose()?;
    let order = waiting(conn)?;
    for entry in &mut listed {
        entry.position = entry.position.map(|_| place_in(&order, entry.id));
        if let Some(latches) = &mut latches {
            entry.latch = latches.query_row([entry.id.0], latch_row).optional()?;
        }
        entry.waits = (waits.query_map([entry.id.0], wait_row)?).collect::<Result<_, _>>()?;
    }
    Ok(listed)
}

/// The queue under one read snapshot, `between` run inside it as
/// [`entries`] says.
pub(crate) fn list_once(
    conn: &Connection,
    between: &mut dyn FnMut(),
) -> Result<Vec<QueueEntry>, StoreError> {
    let _snapshot = conn.unchecked_transaction()?;
    match queue_stored(conn)? {
        Some(stored) => entries(
            conn,
            stored,
            "state != ?1",
            &Standing::Dropped.word(),
            between,
        ),
        None => Ok(Vec::new()),
    }
}

/// One entry under one read snapshot, its place read in the same state as
/// its row, `between` run inside it as [`entries`] says.
fn entry_once(
    conn: &Connection,
    entry: EntryId,
    between: &mut dyn FnMut(),
) -> Result<QueueEntry, StoreError> {
    let _snapshot = conn.unchecked_transaction()?;
    let stored = queue_stored(conn)?.ok_or(QueueRefusal::UnknownEntry(entry))?;
    let mut found = entries(conn, stored, "entry_id = ?1", &entry.0, between)?;
    found.pop().ok_or(QueueRefusal::UnknownEntry(entry).into())
}

impl Store {
    /// Queue a new entry last among those of its own priority, journaling
    /// the `add` with the place it takes.
    /// Every entry it waits for must already be in the queue.
    pub fn queue_add(
        &mut self,
        new: NewEntry<'_>,
        by: Attribution<'_>,
    ) -> Result<EntryId, StoreError> {
        let conn = &mut self.conn;
        patiently("queue_add", self.patience, &self.path, || {
            add_once(conn, new, by)
        })
    }

    /// Move, hold, release or drop an entry, journaling the command. The
    /// check and the write are one `Immediate` transaction, so a peer's
    /// command between them cannot slip past the refusal.
    ///
    /// Returns the place the command journaled: `Some` with the place a
    /// move left the entry holding, which priority can make other than
    /// the place asked, and `None` for a hold, a release, a drop or a
    /// `merged` record, which moves no standing and no place. It is
    /// the same transaction's, so a caller never reads it back after a
    /// peer's command may have moved it.
    pub fn queue_command(
        &mut self,
        entry: EntryId,
        command: QueueCommand,
        by: Attribution<'_>,
    ) -> Result<Option<u32>, StoreError> {
        let conn = &mut self.conn;
        patiently("queue_command", self.patience, &self.path, || {
            command_once(conn, entry, command, by)
        })
    }

    /// The queue: waiting entries in the queue's order (priority, higher
    /// first, then place, then id), then the claimed ones in the same
    /// order less the place they gave up.
    /// Dropped entries are not listed. A journal from before the queue
    /// has an empty one.
    pub fn queue_list(&self) -> Result<Vec<QueueEntry>, StoreError> {
        patiently("queue_list", self.patience, &self.path, || {
            list_once(&self.conn, &mut || {})
        })
    }

    /// The newest `limit` entries still waiting to start, queued or held,
    /// each with its standing's word: the entries an operator command
    /// names, read as one page from the unclaimed rows and never decoded
    /// (decision 0119 ruling 4). A journal from before the queue has none.
    pub fn queue_waiting(&self, limit: u32) -> Result<Vec<(EntryId, String)>, StoreError> {
        patiently("queue_waiting", self.patience, &self.path, || {
            if queue_stored(&self.conn)?.is_none() {
                return Ok(Vec::new());
            }
            let waiting = [Standing::Queued.word(), Standing::Held.word()];
            let mut statement = self.conn.prepare(WAITING_SQL)?;
            let found = statement
                .query_map(params![waiting[0], waiting[1], limit], |row| {
                    Ok((EntryId(row.get(0)?), row.get(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(found)
        })
    }

    /// One entry, whatever its standing, dropped included.
    pub fn queue_entry(&self, entry: EntryId) -> Result<QueueEntry, StoreError> {
        patiently("queue_entry", self.patience, &self.path, || {
            entry_once(&self.conn, entry, &mut || {})
        })
    }

    /// Re-pin a waiting entry to `payload`, journaling the `repin` with
    /// its reason: from now on the entry stands for that launch, and the
    /// one it was queued with stays beside it, and no latch from before
    /// stands on it. This is how the operator releases an entry admission
    /// latched a realm-drift hold on (#430): the caller pins the map now
    /// on disk, and only when it is the map the latch found. `seen` is the
    /// pin and the latch the caller read: the re-pin is refused unless
    /// both stand in the transaction that writes it, so a peer's latch or
    /// re-pin in between is never cleared unseen.
    pub fn queue_repin(
        &mut self,
        entry: EntryId,
        payload: &str,
        seen: Seen,
        by: Attribution<'_>,
    ) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_repin", self.patience, &self.path, || {
            beside_once(conn, entry, Beside::Pin, seen, payload, by)
        })
    }

    /// Latch a realm-drift hold on a waiting entry, journaling the
    /// `latch` with its reason: `finding` stands on the entry until a
    /// later re-pin, whatever admission sees after (#430). A later latch
    /// stands in its place, and every one stays. `seen` is the pin and the
    /// latch the finding was measured against: the latch is refused unless
    /// both stand in the transaction that writes it, so a finding measured
    /// against a pin since replaced never stands.
    pub fn queue_latch(
        &mut self,
        entry: EntryId,
        finding: &str,
        seen: Seen,
        by: Attribution<'_>,
    ) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_latch", self.patience, &self.path, || {
            beside_once(conn, entry, Beside::Latch, seen, finding, by)
        })
    }
}

#[cfg(test)]
mod tests;
