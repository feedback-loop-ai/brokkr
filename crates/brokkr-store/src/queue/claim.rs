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
//! latch the pass read; no other entry launching; and, handed to a judge
//! the caller supplies, the entry's waits, each with what its awaited
//! entry has come to now, and the census, by each run with an event past
//! the high-water the pass read before counting.
//!
//! Every step after the claim checks, inside its transaction, that the
//! generation it is taken under is the one the claim row recorded. Whether
//! that generation is still the journal's latest holder, so that a
//! dispatcher fenced out writes nothing, is #430's slice 3b, for all four
//! steps alike, since the holder rows it reads are 3b's.
//!
//! A linked entry keeps the stored word `launching`, its run set: it
//! reads `Claimed`, and no longer refuses another claim as `InFlight`.
//! That is how ruling 7's "while another entry stands launching" is read
//! here: an entry stands launching until its run is linked.
//!
//! A reclaim of a start its admission refused (the crossing fence, the
//! compile or `refuse_unboxable`) writes a `queue_refusals` row with the
//! `reclaim`, keyed by its seq, and says the entry's count of such rows
//! since its latest `release`, read in the same transaction; a digest
//! refusal or a `Contended` start reclaims uncounted. Holding an entry at
//! the journal's bound reads `dispatch_settings`, and is slice 3b's.

use std::os::unix::fs::MetadataExt;

use brokkr_core::envelope::{verify_chain, EventEnvelope};
use rusqlite::types::{FromSql, FromSqlResult, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::beside::standing_now;
use super::storage::{queue_stored, wait_row, WAITS};
use super::{
    exists, leave_order, read_word, record, set_standing, standing, Attribution, EntryId,
    EntryState, QueueEntry, QueueRefusal, Seen, Standing, Wait,
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

/// Why a start's own admission refused it (decision 0095 ruling 7): a
/// refusal that may recur on the same bytes, so its reclaim counts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartRefusal {
    /// The crossing fence.
    Crossing,
    /// The bundle does not compile as the start compiles it.
    Compile,
    /// `refuse_unboxable`.
    Unboxable,
}

impl StartRefusal {
    const ALL: [StartRefusal; 3] = [
        StartRefusal::Crossing,
        StartRefusal::Compile,
        StartRefusal::Unboxable,
    ];

    fn word(self) -> &'static str {
        match self {
            StartRefusal::Crossing => "crossing",
            StartRefusal::Compile => "compile",
            StartRefusal::Unboxable => "unboxable",
        }
    }
}

/// A counted refusal as `queue_refusals` holds it: one whose word is a
/// [`StartRefusal`]'s, so a word this brokkr does not know refuses the
/// count rather than being counted.
struct Counted;

impl FromSql for Counted {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let all = &StartRefusal::ALL;
        read_word(value, "queue_refusals.refusal", all, StartRefusal::word).map(|_| Counted)
    }
}

/// A run with an event past a pass's high-water: its id, its events, the
/// chain verified, and its manifest as the journal holds it.
type Moved = (String, Vec<EventEnvelope>, String);

/// One wait of the entry claimed, with what its awaited entry has come to
/// now: its standing, and when it is linked, its run's events, the chain
/// verified; none for an entry with no run.
type Awaiting = (Wait, EntryState, Vec<EventEnvelope>);

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
            let events = verified(tx, &run)?;
            let manifest = tx.query_row(
                "SELECT manifest FROM runs WHERE run_id = ?1",
                [&run],
                |row| row.get(0),
            )?;
            Ok((run, events, manifest))
        })
        .collect()
}

/// A run's events as `tx` holds them, refused unless they chain.
fn verified(tx: &Transaction<'_>, run: &str) -> Result<Vec<EventEnvelope>, StoreError> {
    let events = rows_in(tx, run, None)?;
    verify_chain(&events)?;
    Ok(events)
}

/// Each wait of `entry`, with what its awaited entry has come to in `tx`.
fn awaited(tx: &Transaction<'_>, entry: EntryId) -> Result<Vec<Awaiting>, StoreError> {
    let waits: Vec<Wait> =
        (tx.prepare(WAITS)?.query_map([entry.0], wait_row)?).collect::<Result<_, _>>()?;
    waits
        .into_iter()
        .map(|wait| {
            let state = standing(tx, wait.entry)?;
            let events = match state.run() {
                Some(run) => verified(tx, run)?,
                None => Vec::new(),
            };
            Ok((wait, state, events))
        })
        .collect()
}

fn claim_in<R>(
    tx: &Transaction<'_>,
    over: &Over<'_>,
    reason: &str,
    judge: &mut impl FnMut(&[Awaiting], &[Moved]) -> Result<(), R>,
) -> Result<Result<Claim, R>, StoreError> {
    let entry = over.entry.id;
    claimable(tx, entry, over.entry.seen())?;
    let waits = awaited(tx, entry)?;
    if let Err(refused) = judge(&waits, &moved(tx, over.high_water)?) {
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

/// Refuse a step taken under `generation` unless the claim row of
/// `claim` recorded it. Read after the step's own check of its token, so
/// the row is there.
fn claimed_by(
    tx: &Transaction<'_>,
    claim: Claim,
    generation: Generation,
) -> Result<(), StoreError> {
    let recorded: i64 = tx.query_row(
        "SELECT generation FROM queue_claims WHERE seq = ?1",
        [claim.seq],
        |row| row.get(0),
    )?;
    match recorded == generation.0 {
        true => Ok(()),
        false => Err(QueueRefusal::OtherGeneration(claim, generation).into()),
    }
}

/// Refuse a step unless its entry stands launching under `claim`, taken
/// by `generation`.
fn launching_under(
    tx: &Transaction<'_>,
    claim: Claim,
    generation: Generation,
) -> Result<(), StoreError> {
    let state = standing(tx, claim.entry)?;
    let latest: Option<i64> = tx.query_row(
        "SELECT MAX(seq) FROM queue_claims WHERE entry_id = ?1",
        [claim.entry.0],
        |row| row.get(0),
    )?;
    match state == EntryState::Launching && latest == Some(claim.seq) {
        true => claimed_by(tx, claim, generation),
        false => Err(QueueRefusal::Stale(claim).into()),
    }
}

/// A step under a claim: its token, the generation it is taken under,
/// and the reason the dispatcher gives.
#[derive(Clone, Copy)]
struct Under<'a> {
    claim: Claim,
    generation: Generation,
    reason: &'a str,
}

fn link_in(tx: &Transaction<'_>, under: Under<'_>, run: &str) -> Result<(), StoreError> {
    let Under { claim, reason, .. } = under;
    launching_under(tx, claim, under.generation)?;
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

fn spawn_in(tx: &Transaction<'_>, under: Under<'_>) -> Result<(), StoreError> {
    let Under { claim, reason, .. } = under;
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
    claimed_by(tx, claim, under.generation)?;
    journal(tx, claim.entry, Step::Spawn, reason).map(drop)
}

/// The refusals counted against `entry` since its latest `release`, or
/// since it was queued, each read by its word.
fn counted(tx: &Transaction<'_>, entry: EntryId) -> Result<Vec<Counted>, StoreError> {
    let query = "SELECT refusal FROM queue_refusals WHERE entry_id = ?1
        AND seq > COALESCE((SELECT MAX(seq) FROM queue_commands
            WHERE entry_id = ?1 AND command = ?2), 0)
        ORDER BY seq";
    let release = super::QueueCommand::Release.word();
    Ok(tx
        .prepare(query)?
        .query_map(params![entry.0, release], |row| row.get(0))?
        .collect::<Result<_, _>>()?)
}

fn reclaim_in(
    tx: &Transaction<'_>,
    under: Under<'_>,
    refused: Option<StartRefusal>,
) -> Result<u32, StoreError> {
    let Under { claim, reason, .. } = under;
    launching_under(tx, claim, under.generation)?;
    set_standing(tx, claim.entry, Standing::Queued)?;
    let seq = journal(tx, claim.entry, Step::Reclaim, reason)?;
    if let Some(refused) = refused {
        tx.execute(
            "INSERT INTO queue_refusals (seq, entry_id, refusal) VALUES (?1, ?2, ?3)",
            params![seq, claim.entry.0, refused.word()],
        )?;
    }
    Ok(counted(tx, claim.entry)?.len() as u32)
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

    /// The file this store's connection holds, by its device and inode,
    /// as read when it opened: the file SQLite resolved the store's path
    /// to, so through a symlink the file the link named then, however the
    /// link is pointed since, and never read through [`Store::path`] as
    /// given. `None` when it could not be read then, or when the name it
    /// was resolved to no longer holds it: the file was unlinked, or
    /// another was renamed into its place.
    pub fn journal(&self) -> Option<(u64, u64)> {
        let named = Store::identity(&self.conn);
        self.opened.filter(|opened| named == Some(*opened))
    }

    /// The device and inode of the file `conn` holds, as the name SQLite
    /// resolved its path to when it opened reads now.
    pub(crate) fn identity(conn: &Connection) -> Option<(u64, u64)> {
        let query = "SELECT file FROM pragma_database_list WHERE name = 'main'";
        let file: String = conn.query_row(query, [], |row| row.get(0)).ok()?;
        let found = std::fs::metadata(file).ok()?;
        Some((found.dev(), found.ino()))
    }

    /// Claim `entry`, as a pass read it, for the dispatcher of
    /// `generation`: it stands `launching`, in its place, under the
    /// [`Claim`] returned, and the claim records `digest`, the manifest
    /// digest of the bundle the pass admitted (decision 0095 ruling 7).
    /// Refused, writing nothing, unless the entry is queued under the pin
    /// and the latch the pass read and no other entry is launching; and
    /// `judge` is handed, inside the claim's transaction, each of the
    /// entry's waits with what its awaited entry has come to, and each run
    /// with an event past `high_water`, the one the pass read before its
    /// census, by id with its events and manifest; its refusal is returned
    /// as the claim's, with nothing written.
    pub fn queue_claim_with<R>(
        &mut self,
        entry: &QueueEntry,
        high_water: i64,
        digest: &str,
        generation: Generation,
        reason: &str,
        mut judge: impl FnMut(&[Awaiting], &[Moved]) -> Result<(), R>,
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

    /// Link the entry launching under `claim`, taken by `generation`, to
    /// `run`, the run its start wrote, journaling the `link`: the entry
    /// leaves the queue's order, and neither its run nor the run's entry
    /// can be rewritten after.
    pub fn queue_link(
        &mut self,
        claim: Claim,
        generation: Generation,
        run: &str,
        reason: &str,
    ) -> Result<(), StoreError> {
        let under = Under {
            claim,
            generation,
            reason,
        };
        let conn = &mut self.conn;
        patiently("queue_link", self.patience, || {
            in_step(conn, |tx| link_in(tx, under, run))
        })
    }

    /// Journal the `spawn` of the run `claim`, taken by `generation`, was
    /// linked to, before its driver starts: once, so no run is spawned
    /// twice.
    pub fn queue_spawn(
        &mut self,
        claim: Claim,
        generation: Generation,
        reason: &str,
    ) -> Result<(), StoreError> {
        let under = Under {
            claim,
            generation,
            reason,
        };
        let conn = &mut self.conn;
        patiently("queue_spawn", self.patience, || {
            in_step(conn, |tx| spawn_in(tx, under))
        })
    }

    /// Return the entry launching under `claim`, taken by `generation`, to
    /// the queue, in its place, journaling the `reclaim`: for a start that
    /// wrote no run. Its next claim is a new one. A start its admission
    /// `refused` is counted against the entry by a `queue_refusals` row
    /// keyed by the `reclaim`; a digest refusal or a `Contended` start
    /// passes `None` and is not. Says the entry's count since its latest
    /// `release`, this refusal included, read in the same transaction.
    pub fn queue_reclaim(
        &mut self,
        claim: Claim,
        generation: Generation,
        refused: Option<StartRefusal>,
        reason: &str,
    ) -> Result<u32, StoreError> {
        let under = Under {
            claim,
            generation,
            reason,
        };
        let conn = &mut self.conn;
        patiently("queue_reclaim", self.patience, || {
            in_step(conn, |tx| reclaim_in(tx, under, refused))
        })
    }
}

#[cfg(test)]
mod tests;
