//! The dispatcher's claim (decision 0095 ruling 1): an entry is claimed
//! before its run exists, and each durable step of a start from the queue
//! is written before what it stands for. A `claim` makes a queued entry
//! `launching` under a token, the claim's seq; a `link` records the run
//! started under it; a `spawn` records that its driver is starting; and a
//! `reclaim` returns an entry whose start wrote no run to its place. Each
//! step is one `Immediate` transaction that checks its token inside it, and
//! is journaled as a queue command whose operator is `dispatch`, with the
//! reason the dispatcher gives.
//!
//! The claim re-checks what the pass that admitted the entry read and
//! measured (ruling 7): the entry still queued, under the pin and the
//! latch the pass read; no other entry launching; and the census, by each
//! run with an event past the high-water the pass read before counting,
//! handed to a judge the caller supplies. The entry's waits are not read
//! again: they are fixed, and one the pass found met stays met, since the
//! entry it awaits keeps the run it was linked to and a run that ended
//! stays ended.

use brokkr_core::envelope::{verify_chain, EventEnvelope};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::beside::standing_now;
use super::storage::queue_stored;
use super::{
    exists, leave_order, record, set_standing, standing, Attribution, EntryId, EntryState,
    QueueEntry, QueueRefusal, Seen, Standing,
};
use crate::read::rows_in;
use crate::{patiently, Store, StoreError};

/// The token a dispatcher's steps on one entry are taken under: the entry,
/// and the seq of the `claim` command that made it `launching`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claim {
    pub(crate) entry: EntryId,
    pub(crate) seq: i64,
}

/// The dispatcher a claim is taken by, as its generation (decision 0095
/// ruling 2), recorded on the claim. The holder rows that raise it, and
/// the check of a step against the latest of them, are #430's slice 3b.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(pub i64);

/// The operator a dispatcher's step is journaled under.
const DISPATCH: &str = "dispatch";

/// The steps of a start from the queue, by the word each is journaled as.
#[derive(Debug, Clone, Copy)]
enum Step {
    Claim,
    Link,
    Reclaim,
    Spawn,
}

impl Step {
    fn word(self) -> &'static str {
        match self {
            Step::Claim => "claim",
            Step::Link => "link",
            Step::Reclaim => "reclaim",
            Step::Spawn => "spawn",
        }
    }
}

/// A run with an event past a pass's high-water: its id, its events, the
/// chain verified, and its manifest as the journal holds it.
type Moved = (String, Vec<EventEnvelope>, String);

/// What a pass read and measured of the entry it admitted, which its
/// claim is taken over.
struct Over<'a> {
    entry: &'a QueueEntry,
    high_water: i64,
    digest: &'a str,
    generation: Generation,
}

/// One step's `Immediate` transaction: `body` checks and writes inside it,
/// on a queue this brokkr reads, and lands whole or not at all.
fn in_step<T>(
    conn: &mut Connection,
    body: impl FnOnce(&Transaction<'_>) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    queue_stored(&tx)?;
    let done = body(&tx)?;
    tx.commit()?;
    Ok(done)
}

/// Journal one step under the dispatcher, saying its place in the
/// journal of commands.
fn journal(
    tx: &Transaction<'_>,
    entry: EntryId,
    step: Step,
    reason: &str,
) -> Result<i64, StoreError> {
    let by = Attribution {
        operator: DISPATCH,
        reason,
    };
    record(tx, entry, step.word(), None, by)
}

/// Why an entry standing so cannot be claimed, or `None` when it is
/// queued.
fn unclaimable(entry: EntryId, state: EntryState) -> Option<QueueRefusal> {
    match state {
        EntryState::Queued => None,
        EntryState::Held => Some(QueueRefusal::Held(entry)),
        EntryState::Launching => Some(QueueRefusal::Launching(entry)),
        EntryState::Claimed { run } => Some(QueueRefusal::Claimed { entry, run }),
        EntryState::Dropped => Some(QueueRefusal::Dropped(entry)),
    }
}

/// Refuse a claim of `entry` unless it is queued, no other entry is
/// launching, and the pin and the latch standing on it are those `seen`.
fn claimable(tx: &Transaction<'_>, entry: EntryId, seen: Seen) -> Result<(), StoreError> {
    if let Some(refusal) = unclaimable(entry, standing(tx, entry)?) {
        return Err(refusal.into());
    }
    let launching: Option<i64> = tx
        .query_row(
            "SELECT entry_id FROM queue_entries WHERE state = ?1 AND run_id IS NULL
             ORDER BY entry_id LIMIT 1",
            params![Standing::Launching.word()],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(other) = launching {
        return Err(QueueRefusal::InFlight(EntryId(other)).into());
    }
    let (latch, pin) = standing_now(tx, entry)?;
    let now = Seen {
        pin,
        latch: latch.map(|latch| latch.seq),
    };
    match now == seen {
        true => Ok(()),
        false => Err(QueueRefusal::Unseen(entry).into()),
    }
}

/// Each run with an event past `high_water`, by id, as it stands in `tx`.
fn moved(tx: &Transaction<'_>, high_water: i64) -> Result<Vec<Moved>, StoreError> {
    let runs: Vec<String> = tx
        .prepare("SELECT DISTINCT run_id FROM events WHERE rowid > ?1 ORDER BY run_id")?
        .query_map([high_water], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    runs.into_iter()
        .map(|run| {
            let events = rows_in(tx, &run, None)?;
            verify_chain(&events)?;
            let manifest = tx.query_row(
                "SELECT manifest FROM runs WHERE run_id = ?1",
                [&run],
                |row| row.get(0),
            )?;
            Ok((run, events, manifest))
        })
        .collect()
}

fn claim_in<R>(
    tx: &Transaction<'_>,
    over: &Over<'_>,
    reason: &str,
    judge: &mut impl FnMut(&[Moved]) -> Result<(), R>,
) -> Result<Result<Claim, R>, StoreError> {
    let entry = over.entry.id;
    claimable(tx, entry, over.entry.seen())?;
    if let Err(refused) = judge(&moved(tx, over.high_water)?) {
        return Ok(Err(refused));
    }
    let seq = journal(tx, entry, Step::Claim, reason)?;
    tx.execute(
        "INSERT INTO queue_claims (seq, entry_id, generation, digest) VALUES (?1, ?2, ?3, ?4)",
        params![seq, entry.0, over.generation.0, over.digest],
    )?;
    set_standing(tx, entry, Standing::Launching)?;
    Ok(Ok(Claim { entry, seq }))
}

/// Refuse a step unless its entry stands launching under `claim`.
fn launching_under(tx: &Transaction<'_>, claim: Claim) -> Result<(), StoreError> {
    let state = standing(tx, claim.entry)?;
    let latest: Option<i64> = tx.query_row(
        "SELECT MAX(seq) FROM queue_claims WHERE entry_id = ?1",
        [claim.entry.0],
        |row| row.get(0),
    )?;
    match state == EntryState::Launching && latest == Some(claim.seq) {
        true => Ok(()),
        false => Err(QueueRefusal::Stale(claim).into()),
    }
}

fn link_in(tx: &Transaction<'_>, claim: Claim, run: &str, reason: &str) -> Result<(), StoreError> {
    launching_under(tx, claim)?;
    if !exists(tx, "SELECT 1 FROM runs WHERE run_id = ?1", &run)? {
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
        let entry = EntryId(other);
        return Err(QueueRefusal::RunTaken { run, entry }.into());
    }
    tx.execute(
        "UPDATE queue_entries SET run_id = ?1 WHERE entry_id = ?2",
        params![run, claim.entry.0],
    )?;
    tx.execute(
        "UPDATE queue_claims SET run_id = ?1 WHERE seq = ?2",
        params![run, claim.seq],
    )?;
    leave_order(tx, claim.entry)?;
    journal(tx, claim.entry, Step::Link, reason).map(drop)
}

fn spawn_in(tx: &Transaction<'_>, claim: Claim, reason: &str) -> Result<(), StoreError> {
    let unspawned: Option<i64> = tx
        .query_row(
            "SELECT 1 FROM queue_claims WHERE seq = ?1 AND entry_id = ?2 AND run_id IS NOT NULL
             AND NOT EXISTS (SELECT 1 FROM queue_commands WHERE entry_id = ?2 AND command = ?3)",
            params![claim.seq, claim.entry.0, Step::Spawn.word()],
            |row| row.get(0),
        )
        .optional()?;
    if unspawned.is_none() {
        return Err(QueueRefusal::Stale(claim).into());
    }
    journal(tx, claim.entry, Step::Spawn, reason).map(drop)
}

fn reclaim_in(tx: &Transaction<'_>, claim: Claim, reason: &str) -> Result<(), StoreError> {
    launching_under(tx, claim)?;
    set_standing(tx, claim.entry, Standing::Queued)?;
    journal(tx, claim.entry, Step::Reclaim, reason).map(drop)
}

impl Store {
    /// The highest rowid of the events table, `0` for none: what an
    /// admission pass reads before it counts the runs running, so that its
    /// claim re-reads each run with an event past it. Events are never
    /// updated or deleted, so it only rises.
    pub fn high_water(&self) -> Result<i64, StoreError> {
        patiently("high_water", self.patience, || {
            let query = "SELECT COALESCE(MAX(rowid), 0) FROM events";
            Ok(self.conn.query_row(query, [], |row| row.get(0))?)
        })
    }

    /// Claim `entry`, as a pass read it, for the dispatcher of
    /// `generation`: it stands `launching`, in its place, under the
    /// [`Claim`] returned, and the claim records `digest`, the manifest
    /// digest of the bundle the pass admitted (decision 0095 ruling 7).
    /// Refused, writing nothing, unless the entry is queued under the pin
    /// and the latch the pass read and no other entry is launching; and
    /// `judge` is handed each run with an event past `high_water`, the
    /// one the pass read before its census, by id with its events and
    /// manifest, inside the claim's transaction, whose refusal it returns
    /// as the claim's with nothing written.
    pub fn queue_claim_with<R>(
        &mut self,
        entry: &QueueEntry,
        high_water: i64,
        digest: &str,
        generation: Generation,
        reason: &str,
        mut judge: impl FnMut(&[Moved]) -> Result<(), R>,
    ) -> Result<Result<Claim, R>, StoreError> {
        let over = Over {
            entry,
            high_water,
            digest,
            generation,
        };
        let conn = &mut self.conn;
        patiently("queue_claim", self.patience, || {
            in_step(conn, |tx| claim_in(tx, &over, reason, &mut judge))
        })
    }

    /// Link the entry launching under `claim` to `run`, the run its start
    /// wrote, journaling the `link`: the entry leaves the queue's order,
    /// and neither its run nor the run's entry can be rewritten after.
    pub fn queue_link(&mut self, claim: Claim, run: &str, reason: &str) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_link", self.patience, || {
            in_step(conn, |tx| link_in(tx, claim, run, reason))
        })
    }

    /// Journal the `spawn` of the run `claim` was linked to, before its
    /// driver starts: once, so no run is spawned twice.
    pub fn queue_spawn(&mut self, claim: Claim, reason: &str) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_spawn", self.patience, || {
            in_step(conn, |tx| spawn_in(tx, claim, reason))
        })
    }

    /// Return the entry launching under `claim` to the queue, in its
    /// place, journaling the `reclaim`: for a start that wrote no run.
    /// Its next claim is a new one.
    pub fn queue_reclaim(&mut self, claim: Claim, reason: &str) -> Result<(), StoreError> {
        let conn = &mut self.conn;
        patiently("queue_reclaim", self.patience, || {
            in_step(conn, |tx| reclaim_in(tx, claim, reason))
        })
    }
}

#[cfg(test)]
mod tests;
