//! The runtime store: bundled SQLite holding append-only facts.
//!
//! Application triggers reject UPDATE and DELETE on event rows — this
//! protects against ordinary defects, not a hostile database owner
//! (target-architecture). The journal is runtime truth; canonical NDJSON
//! export is the portable, human-auditable form. Single-host,
//! single-logical-writer *per run*: a concurrent writer on the same run
//! loses on the (run_id, seq) primary key inside its append transaction
//! — optimistic fencing instead of a lease service. A writer whose event
//! is legal only against the state it read fences on that state too, with
//! [`Store::append_next_if_head`].
//!
//! # Many fires, one journal
//!
//! Writers on *different* runs in the same file never contend for a
//! `(run_id, seq)` slot, so their chains are independent by
//! construction. What they do share is SQLite's database-wide write
//! lock, and that is a measured property, not an assumed one. Six
//! writers, each with its own [`Connection`] to one file, appending
//! back-to-back to six different runs, completed with zero errors and
//! every chain contiguous and verifying: WAL plus the busy timeout is
//! sufficient for the append path exactly as it stood, so nothing about
//! [`Store::append_next`] changed. Its `Immediate` transaction is the
//! reason — taking the write lock at `BEGIN` cannot deadlock against a
//! peer doing the same, and the busy handler resolves the wait.
//!
//! Two tests hold that ground.
//! `tests::parallel_burns_on_different_runs_share_one_journal` races
//! four writer threads, and `tests/concurrent_processes.rs` races four
//! writer *processes* — the faithful one, since POSIX advisory locks are
//! per-process and same-realm parallel burns are separate `brokkr`
//! processes.
//!
//! Opening the journal was the part that did not hold, and both defects
//! were in the ordering of [`Store::open`]'s prologue rather than in any
//! append. See [`Store::open`] and [`Store::migrate`] for what
//! measurement found and what each line now buys.
//!
//! # When a peer wins anyway
//!
//! The busy timeout is a budget and not a guarantee, and it is not even
//! always spent: SQLite returns `SQLITE_BUSY` *without* consulting the
//! busy handler whenever consulting it could deadlock, so a caller can
//! meet `database is locked` in microseconds with its whole patience
//! untouched. That is what killed a live engine on 2026-09-02, and
//! [`patiently`] is the answer — one place, one budget, spent to a
//! deadline by whichever line of defence does the waiting.
//!
//! What survives the budget is [`StoreError::Contended`]: typed, saying
//! nothing was written, and deliberately a different thing from the two
//! refusals beside it. [`StoreError::HeadMoved`] and
//! [`StoreError::AppendConflict`] are verdicts about content and are
//! never retried; contention is an accident of timing and the same call
//! made again is the same call.
//!
//! This is what lets a realm's `journal` path in `realms.json` be the
//! shared target for same-realm parallel burns: several `brokkr`
//! processes, each driving its own run, appending into one journal. A
//! worktree-local `.forge/forge.db` remains entirely legal — it is
//! emergency isolation now, not the assumed steady state.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
#[cfg(test)]
use brokkr_core::envelope::ChainError;
use brokkr_core::envelope::{verify_chain, EventEnvelope, EventType};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
mod acknowledgement;
pub mod aggregate;
pub mod backup;
mod chain;
mod damage;
mod demand;
mod error;
mod event_index;
mod import;
mod literal;
pub mod metered;
mod opening;
mod origin;
mod page;
mod patience;
pub mod ports;
mod queue;
mod read;
mod redact;
mod run_read;
mod run_select;
mod schema;
mod seat_read;
mod seat_record;
mod seat_summary;
mod seat_tree;
mod snapshot;
mod start;
mod summary;
mod terminal;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod untouched;
pub use acknowledgement::Acknowledgement;
pub use chain::{Chain, Rows};
pub use damage::Damage;
pub use demand::{Unended, UsageEvent, UsageRow};
pub use error::StoreError;
pub use event_index::IndexFault;
pub use import::{verified_events, verify_export, Adoption, Arrival, ImportError, VerifyError};
pub use opening::Opened;
pub use page::{keyed, place_of, Keyed, Paged, RowRefusal, Unread};
pub use ports::ManifestRefusal;
pub use queue::{
    claim_stands, Attribution, Claim, ClaimFacts, Claiming, Cooldown, CooldownAggregate,
    CooldownDuplicate, CooldownRound, CooldownRoundMismatch, Cooling, Disposal, EntryId,
    EntryState, Generation, Latch, NewEntry, NotAWait, QueueCommand, QueueEntry, QueueRefusal,
    RoundAppend, Seen, StartRefusal, Wait, WaitOn,
};
pub use redact::{redact_export, Redactor};
pub use run_read::RunRead;
pub use run_select::SeatNames;
pub use schema::DATABASE_SCHEMA;
pub use seat_read::SeatRead;
pub use seat_record::{validate_seat_record, SeatRecordError, SeatRecordVersion};
pub use seat_summary::SeatRows;
pub use seat_tree::{SeatTree, SeatTreeRecord, SeatTreeRefusal, Swept, TreeSweep, TreeSweepRecord};
pub use summary::Disagreement;

use brokkr_core::summary::Account;
pub use import::RUN_ID_MAX;
use patience::patiently;
use schema::ensure_wal;
use start::Row;

/// How long any statement waits for a peer's write lock before giving
/// up. Every connection sets this as its FIRST act, so no statement in
/// the crate — pragma, migration, or append — is ever the one that runs
/// without it.
///
/// Setting it is not the same as being covered by it, and that gap is
/// what [`patiently`] closes: SQLite declines to consult a busy handler
/// whenever consulting it could deadlock, and returns `SQLITE_BUSY` on
/// the spot instead. So this is the budget, not the mechanism — read it
/// as the whole time one operation may spend waiting, however the
/// waiting gets done.
///
/// Thirty seconds, not ten, and the number comes from a measurement.
/// SQLite's busy handler is not a fair queue: it wakes, retries, and
/// takes its chances, so waiting for the write lock is heavy-tailed
/// rather than bounded. Against a deliberately pathological peer —
/// another writer appending with *no* gap at all, tens of thousands of
/// appends a second — a second writer's wait for the lock ran to 8s and
/// 17s in different runs. It always eventually got the lock; nothing
/// deadlocks, because the peer makes progress and the wait is only for a
/// turn. But a tail like that under a ten-second budget is a coin flip,
/// and it came up wrong: three of forty `create_run` calls died with
/// `database is locked` at the old timeout, none at this one.
///
/// No timeout makes an unfair lock fair — a longer one buys margin, not
/// a guarantee. What makes the margin sufficient is that real cadence is
/// nowhere near the adversary: a burn appends around a driver that runs
/// for *seconds*, and at a peer gap of even 100µs the same wait falls to
/// 18ms, at 1ms to about one. The generous timeout is insurance against
/// the pathological case, not the price of the ordinary one.
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// A reader's page cache in KiB, a negative `cache_size`: 64 holds SQLite
/// 3.54's share within ruling 9's cap on every host, macOS measuring
/// above Linux (decision 0089 ruling 9, amended 2026-11-10 on #1043).
const READER_CACHE_KIB: i64 = 64;

pub struct Store {
    conn: Connection,
    path: PathBuf,
    /// The whole budget one store operation spends on a peer's lock —
    /// the connection's busy handler and [`patiently`]'s retry together,
    /// never one each. [`BUSY_TIMEOUT`] unless a caller says otherwise;
    /// see [`Store::set_patience`].
    patience: std::time::Duration,
    /// The writer's account of the run it appends to, which each append
    /// carries over what it seals and writes the run's summary from
    /// ([`Store::carry`]; decision 0089 ruling 3).
    carried: Option<summary::Carried>,
    /// What the tests' binding holds this store to when it drops.
    #[cfg(any(test, feature = "test-support"))]
    binding: test_support::Binding,
    /// The virtual-machine steps the selector's reads took, for the test
    /// that binds their cost ([`run_select`]). Only a test reads it.
    #[cfg(test)]
    selector_steps: std::cell::Cell<i64>,
    /// The VM instructions the last park sweep's selection read, its
    /// off-head scan and its stale seeks apart: what the test that binds
    /// its one-walk cost steps (decision 0071 ruling 9).
    #[cfg(test)]
    park_steps: std::cell::Cell<(i64, i64)>,
}

/// Refuse a run id the journal already carries, as
/// [`StoreError::RunExists`]: the check [`Store::create_run`] and an
/// adoption both make inside the transaction that would write the row.
fn refuse_existing(conn: &Connection, run_id: &str) -> Result<(), StoreError> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT run_id FROM runs WHERE run_id = ?1",
            params![run_id],
            |r| r.get(0),
        )
        .optional()?;
    match existing {
        Some(_) => Err(StoreError::RunExists(run_id.to_string())),
        None => Ok(()),
    }
}

/// A run's head: its last event's seq and hash, or 0 and [`ZERO_HASH`] for a
/// run with none. [`Store::head_hash`] and an append's transaction read it.
fn head_of(conn: &Connection, run_id: &str) -> Result<(u64, String), StoreError> {
    let head: Option<(i64, String)> = conn
        .query_row(
            "SELECT seq, event_hash FROM events WHERE run_id = ?1
             ORDER BY seq DESC LIMIT 1",
            params![run_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(match head {
        Some((seq, hash)) => (seq as u64, hash),
        None => (0, ZERO_HASH.to_string()),
    })
}

/// [`Store::create_run`]'s one transaction, as a body [`patiently`] may
/// run again. It reads and writes only inside that transaction, so an
/// attempt that ends busy has left the journal exactly as it found it.
/// The origin arrives already computed: nothing is asked of the machine
/// while the write lock is held.
fn create_run_once(conn: &mut Connection, row: &Row<'_>) -> Result<summary::Listed, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let listed = start::insert_run(&tx, row)?;
    tx.commit()?;
    Ok(listed)
}

/// The `engine` a run's manifest names, or NULL where it names none: a
/// manifest that is not JSON, or whose `engine` is not a string, names
/// none, as the whole-manifest parse this replaced read it. `CASE` runs
/// only the branch it takes, so `json_each` never meets invalid JSON,
/// which it would raise on. Of two `engine` keys the LAST answers, as
/// serde reads it at export; `json_extract` would answer the first.
const ENGINE_OF_RUN: &str = "SELECT CASE WHEN json_valid(manifest) THEN
         (SELECT CASE type WHEN 'text' THEN atom END FROM json_each(manifest)
          WHERE key = 'engine' ORDER BY id DESC LIMIT 1)
     END FROM runs WHERE run_id = ?1";

/// The hash of a run's event at a seq (NULL where it has none) and its
/// lowest seq, one primary-key seek each; no row for a run the journal lacks.
const HEAD_OF_RUN: &str = "SELECT (SELECT event_hash FROM events WHERE run_id = ?1 AND seq = ?2),
     (SELECT MIN(seq) FROM events WHERE run_id = ?1) FROM runs WHERE run_id = ?1";

/// The event one cool-down append writes, and the head it is fenced to
/// when the engine holds one: everything [`Store::append_next`] and
/// [`Store::append_next_if_head`] decide by their names, in one parameter
/// object (decision 0071 ruling 10, decision 0103 slice 4d1).
pub struct CooldownAppend<'a> {
    pub run_id: &'a str,
    /// The head this event must still find, as [`Store::append_next_if_head`]
    /// fences one; `None` appends wherever the head is.
    pub fenced: Option<(u64, &'a str)>,
    pub event_type: EventType,
    pub payload: Value,
    pub causation_id: Option<String>,
    pub attempt_id: Option<String>,
}

impl Store {
    /// Open (creating if absent) a journal for writing.
    ///
    /// The order of the four lines below is load-bearing, and both
    /// orderings it corrects were measured failures of six burns racing
    /// to open one shared realm journal:
    ///
    /// - `busy_timeout` comes FIRST, before any pragma. `journal_mode =
    ///   WAL` takes an exclusive lock to convert a fresh rollback-journal
    ///   file, and a connection that has not yet set a timeout has none
    ///   — it fails on the spot. Set after the pragmas, as it was, one
    ///   or two of six simultaneous first-opens died with `database is
    ///   locked`.
    /// - The WAL conversion is asked for only when the file is not
    ///   already in WAL, and retried when it is; see [`ensure_wal`].
    /// - The schema check is a *read* in the steady state; see
    ///   [`Store::migrate`] for why writing there starved.
    ///
    /// A file its bytes say is damaged refuses as [`StoreError::Damaged`]:
    /// every error leaving the open passes through [`damage::classified`],
    /// the catch-up's included, whose backfills read outside [`patiently`]
    /// and end quietly only on a peer's lock. The WAL read is the first
    /// (bound by
    /// `damage::tests::a_file_that_is_no_database_is_refused_by_both_opens`),
    /// the catch-up's the last (bound by
    /// `damage::tests::a_corrupt_page_the_catch_up_reads_refuses_the_writable_open`).
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        Store::open_writing(path).map_err(|error| damage::classified(path, error))
    }

    fn open_writing(path: &Path) -> Result<Store, StoreError> {
        let bound = literal::journal(path)?;
        // A bound path has a parent unless it is the root, which exists.
        let _ = std::fs::create_dir_all(bound.parent().unwrap_or(&bound));
        let mut conn =
            Connection::open(&bound).map_err(|error| literal::as_given(error, path, &bound))?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        ensure_wal(&conn, path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        patiently("migrate", BUSY_TIMEOUT, path, || Store::migrate(&mut conn))?;
        let mut store = Store::holding(conn, path);
        store.catch_up()?;
        Ok(store)
    }

    /// The whole budget every operation of this store spends on a peer's
    /// lock, set on both lines of defence at once — the connection's own
    /// busy handler and [`patiently`]'s retry — so the two can never add
    /// up to more than one wait.
    ///
    /// [`BUSY_TIMEOUT`] is the measured default and no operator has a
    /// reason to change it. Patience is an argument for the same reason
    /// it is one of [`schema::ensure_wal_by`]'s: the tests that prove a
    /// budget runs out must not have to wait it out.
    pub fn set_patience(&mut self, patience: std::time::Duration) -> Result<(), StoreError> {
        self.conn.busy_timeout(patience)?;
        self.patience = patience;
        Ok(())
    }

    /// The journal this connection opened, retained so deterministic
    /// exec seats can read the same journal the engine is driving.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Open an EXISTING journal for reading only. The connection carries
    /// `SQLITE_OPEN_READ_ONLY`, so every write refuses at the database
    /// rather than at a reviewer's memory: a read surface that opens
    /// this way is *unable* to append, which is what decision 0020 asks
    /// of Muninn. No database is created and no migration runs — a
    /// missing database is an error, never an empty fleet. Beside a WAL
    /// journal SQLite still creates its `-wal` and `-shm`, which this
    /// connection cannot remove; [`Store::read_untouched`] is the read
    /// that leaves none. A file its bytes say is damaged, a journal
    /// recording no schema among them, refuses as [`StoreError::Damaged`].
    pub fn open_read_only(path: &Path) -> Result<Store, StoreError> {
        Store::open_reading(path).map_err(|error| damage::classified(path, error))
    }

    fn open_reading(path: &Path) -> Result<Store, StoreError> {
        let bound = literal::journal(path)?;
        let conn = Connection::open_with_flags(&bound, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| literal::as_given(error, path, &bound))?;
        Store::reading(conn, path, BUSY_TIMEOUT)
    }

    /// A store on `conn`, which opened `path`.
    fn holding(conn: Connection, path: &Path) -> Store {
        Store {
            conn,
            path: path.to_path_buf(),
            patience: BUSY_TIMEOUT,
            carried: None,
            #[cfg(any(test, feature = "test-support"))]
            binding: Default::default(),
            #[cfg(test)]
            selector_steps: std::cell::Cell::new(0),
            #[cfg(test)]
            park_steps: std::cell::Cell::new((0, 0)),
        }
    }

    /// Declare a run in the journal. The existence check and the insert
    /// are one `Immediate` transaction — the write lock taken at `BEGIN`,
    /// the same discipline [`Store::append_next`] uses.
    ///
    /// A burn starting while a sibling burn is mid-flight is the ordinary
    /// case for a shared realm journal, and this was the last place it
    /// was unsafe. Read-then-insert on the bare connection left the
    /// insert in an implicit deferred transaction, which reads first and
    /// only then asks to upgrade to the write lock; measured against one
    /// peer appending back-to-back, three of forty `create_run` calls
    /// burned the entire ten-second timeout and failed with `database is
    /// locked`. Taking the lock up front, none do. It closes the
    /// check-then-insert window too: two writers claiming one `run_id`
    /// can no longer both pass the check, so the loser gets
    /// [`StoreError::RunExists`] rather than a primary-key error.
    pub fn create_run(
        &mut self,
        run_id: &str,
        feature: &str,
        bundle_name: &str,
        manifest: &Value,
    ) -> Result<(), StoreError> {
        self.create_run_from(run_id, feature, bundle_name, manifest, origin::local_host)
    }

    /// [`Store::create_run`] with where the run is driven from asked of
    /// `origin` — once, and before the transaction begins, so no process
    /// the fingerprint spawns runs under the journal's write lock or
    /// again on a busy retry.
    fn create_run_from(
        &mut self,
        run_id: &str,
        feature: &str,
        bundle_name: &str,
        manifest: &Value,
        origin: impl FnOnce() -> Option<String>,
    ) -> Result<(), StoreError> {
        import::recordable(run_id)?;
        let manifest = serde_json::to_string(manifest)?;
        let origin = origin();
        let row = Row {
            run_id,
            feature,
            bundle_name,
            manifest: &manifest,
            origin: origin.as_deref(),
        };
        let conn = &mut self.conn;
        let listed = patiently("create_run", self.patience, &self.path, || {
            create_run_once(conn, &row)
        })?;
        // Whoever creates a run holds its fold: the run with no event.
        self.carried = Some(summary::Carried {
            account: Account::empty(run_id),
            listed: Some(listed),
        });
        #[cfg(any(test, feature = "test-support"))]
        {
            self.binding.carried();
            self.binding.wrote(run_id);
        }
        Ok(())
    }

    /// Is this run still being driven from where it started — the same
    /// machine, the same account (decision 0030 ruling 4)?
    ///
    /// The only caller is the engine's session offer, and the only thing
    /// a `false` costs is a cold spawn. It is `false` for a run this
    /// journal never started: one adopted from elsewhere (decision
    /// 0027 leaves `origin_host` NULL, and an exported journal is events
    /// — the column does not travel with it), one started by a brokkr
    /// that predates this column, one whose journal file has been copied
    /// to another machine, and one this installation cannot place at all
    /// because it could read no machine identity. A provider session
    /// belongs to the credential that opened it, so "I cannot prove this
    /// is the same installation" and "it is not" get the same answer.
    ///
    /// It is deliberately NOT a fact about the run: an adopted run is a
    /// run (decision 0027), no event carries this, and no phase machine
    /// can see it. It is a fact about this journal file's relationship
    /// to this machine, which is why it lives here and not in the chain.
    pub fn started_here(&self, run_id: &str) -> Result<bool, StoreError> {
        self.started_under(run_id, origin::local_host())
    }

    /// [`Store::started_here`] against a given fingerprint, so the
    /// "nowhere in particular" answer is reachable from a test on a
    /// machine that does know who it is.
    fn started_under(&self, run_id: &str, local: Option<String>) -> Result<bool, StoreError> {
        let Some(local) = local else {
            return Ok(false);
        };
        let origin: Option<String> = patiently("started_here", self.patience, &self.path, || {
            Ok(self
                .conn
                .query_row(
                    "SELECT origin_host FROM runs WHERE run_id = ?1",
                    params![run_id],
                    |r| r.get(0),
                )
                .optional()?
                .flatten())
        })?;
        Ok(origin.as_deref() == Some(local.as_str()))
    }

    pub fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        let raw: Option<String> = patiently("manifest", self.patience, &self.path, || {
            Ok(self
                .conn
                .query_row(
                    "SELECT manifest FROM runs WHERE run_id = ?1",
                    params![run_id],
                    |r| r.get(0),
                )
                .optional()?)
        })?;
        let raw = raw.ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        patiently("list_runs", self.patience, &self.path, || {
            Ok(self
                .conn
                .prepare("SELECT run_id, feature, created_at FROM runs ORDER BY created_at")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        patiently("head", self.patience, &self.path, || {
            head_of(&self.conn, run_id)
        })
    }

    /// Build, seal, and durably append the next event in one transaction.
    /// Envelope identity (seq, previous_hash) comes from the journal head
    /// inside the transaction; a concurrent writer conflicts instead of
    /// forking the chain.
    ///
    /// The head is whatever the transaction finds, so this lands on any
    /// live run. It refuses a non-annotation once the run has reached a
    /// terminal event ([`StoreError::AfterTerminal`]), before anything is
    /// written (#440). A caller whose event is only legal against the head
    /// it read — anything it decided by folding — wants
    /// [`Store::append_next_if_head`] instead.
    pub fn append_next(
        &mut self,
        run_id: &str,
        event_type: EventType,
        payload: Value,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, StoreError> {
        self.append(
            CooldownAppend {
                run_id,
                fenced: None,
                event_type,
                payload,
                causation_id,
                attempt_id,
            },
            &[],
        )
    }

    /// Append, but only onto the head the caller decided against —
    /// compare-and-append.
    ///
    /// [`Store::append_next`] recomputes the head inside its own
    /// transaction and lands on any live run, which is right for a writer
    /// whose event is legal wherever it lands: the engine appending to its
    /// own run is the only writer of that run. It is wrong for a writer whose
    /// event is legal only against a particular state. An
    /// `operator/accepted` is the case that forced this: whether `fold`
    /// can read it back depends on the run's status at that exact seq, so
    /// a peer's append between the deciding fold and the write turns an
    /// acceptance into `FoldError::AfterTerminal` for every reader
    /// afterwards — and events are immutable, so nothing takes it back.
    ///
    /// The check runs INSIDE the same `Immediate` transaction that
    /// writes, under the same write lock, which is what makes decide-then-
    /// append atomic rather than merely narrow: either the head is still
    /// `(expected_seq, expected_hash)` and the event lands, or the head
    /// moved, nothing at all is written, and [`StoreError::HeadMoved`]
    /// sends the caller back to re-read and decide again. `expected_seq`
    /// of 0 with [`ZERO_HASH`] fences an append onto an empty run.
    // Two of these arguments are the fence; the rest are `append_next`'s.
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    pub fn append_next_if_head(
        &mut self,
        run_id: &str,
        expected_seq: u64,
        expected_hash: &str,
        event_type: EventType,
        payload: Value,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, StoreError> {
        self.append(
            CooldownAppend {
                run_id,
                fenced: Some((expected_seq, expected_hash)),
                event_type,
                payload,
                causation_id,
                attempt_id,
            },
            &[],
        )
    }

    /// Append `event` and `cooldowns` as one: the event and every row of
    /// its attempt's cool-downs land in ONE `Immediate` transaction, or
    /// neither does (decision 0103 ruling 2(a), slice 4d1). The store fills
    /// each row's `event_seq` with the seq the event takes inside that
    /// transaction, so the caller never guesses it; a row the table refuses
    /// rolls the event back with it, and a fence that moved writes nothing
    /// at all.
    pub fn append_with_cooldowns(
        &mut self,
        event: CooldownAppend<'_>,
        cooldowns: &[Cooling],
    ) -> Result<EventEnvelope, StoreError> {
        self.append(event, cooldowns)
    }

    fn append(
        &mut self,
        event: CooldownAppend<'_>,
        cooldowns: &[Cooling],
    ) -> Result<EventEnvelope, StoreError> {
        let (conn, carried) = (&mut self.conn, self.carried.as_ref());
        let (envelope, settled) = patiently("append", self.patience, &self.path, || {
            chain::append_once((conn, carried), &event, cooldowns)
        })?;
        self.settle((event.run_id, 1), settled);
        Ok(envelope)
    }

    /// Load and verify the full journal of a run. A journal that fails
    /// chain verification is corrupt and is never partially returned.
    pub fn load(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        patiently("load", self.patience, &self.path, || self.load_once(run_id))
    }

    fn load_once(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        // Every row of the run, at whatever seq the table holds it: a row
        // the chain does not cover is refused, never skipped.
        let events = self.rows(run_id, None)?;
        if events.is_empty() {
            // Distinguish "no such run" from "run without events".
            let _ = self.manifest(run_id)?;
        }
        verify_chain(&events)?;
        Ok(events)
    }

    /// Canonical NDJSON export: one sealed envelope per line, in order.
    pub fn export_ndjson(&self, run_id: &str) -> Result<String, StoreError> {
        let events = self.load(run_id)?;
        seat_record::validate_events(&events)?;
        ports::format_canonical_ndjson(&events)
    }

    /// The journal head hash — cheap identity for fencing and anchors.
    pub fn head_hash(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        self.head(run_id)
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("valid")
        .format(&Rfc3339)
        .expect("rfc3339 formats")
}

#[cfg(test)]
mod tests;
