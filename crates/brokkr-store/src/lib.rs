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
use brokkr_core::envelope::{verify_chain, ChainError, EventEnvelope, EventType};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

mod import;
mod origin;
mod queue;
mod read;
mod redact;
mod schema;
mod seat_record;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use import::{verified_events, verify_export, Adoption, Arrival, ImportError, VerifyError};
pub use queue::{
    Attribution, EntryId, EntryState, Latch, NewEntry, NotAWait, QueueCommand, QueueEntry,
    QueueRefusal, Seen, Wait, WaitOn,
};
pub use redact::{redact_export, Redactor};
pub use schema::DATABASE_SCHEMA;
pub use seat_record::{validate_seat_record, SeatRecordError, SeatRecordVersion};

use schema::{ensure_wal, schema_supported, RECORDED_SCHEMA};

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

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("run '{0}' not found")]
    RunNotFound(String),
    #[error("run '{0}' already exists")]
    RunExists(String),
    #[error("chain: {0}")]
    Chain(#[from] ChainError),
    #[error(transparent)]
    SeatRecord(#[from] SeatRecordError),
    #[error("database schema {found} unsupported (want {DATABASE_SCHEMA})")]
    SchemaMismatch { found: u32 },
    #[error("append conflict: seq {seq} already written by another writer")]
    AppendConflict { seq: u64 },
    #[error("head moved: expected seq {expected_seq}, found {found_seq}")]
    HeadMoved { expected_seq: u64, found_seq: u64 },
    /// [`Store::load_after`] was handed a head the run never had: no
    /// event at that seq, or one with another hash.
    #[error("unknown head: the run holds no event {seq} with the hash given")]
    UnknownHead { seq: u64 },
    /// [`Store::load_after`] found a row below seq 1, which no suffix read
    /// selects; [`Store::load`] refuses the same journal whole.
    #[error("run '{run_id}' holds a row at seq {seq}, before its chain's first event")]
    RowBeforeChain { run_id: String, seq: i64 },
    /// The queue refused an operator command or a claim (decision 0068).
    #[error(transparent)]
    Queue(#[from] QueueRefusal),
    /// A peer still held the journal's write lock when this operation's
    /// whole patience ran out. **Nothing was written.**
    ///
    /// This is a third thing, and the distinction is the point. An
    /// [`StoreError::AppendConflict`] means a peer took the seq; a
    /// [`StoreError::HeadMoved`] means a peer moved the head the caller
    /// decided against — both are *refusals*, verdicts about content,
    /// and neither may ever be retried into place. Contention is an
    /// accident of timing on a lock: the same call, made again, is the
    /// same call. It is typed separately so a caller can end lawfully on
    /// it instead of dying as if the journal had refused it something.
    #[error(
        "contended: a peer still held the journal's write lock after {waited_ms}ms of \
         {operation}; nothing was written"
    )]
    Contended {
        operation: &'static str,
        waited_ms: u128,
    },
}

impl StoreError {
    /// Is this the contention accident rather than a refusal or a
    /// defect? The one predicate callers branch on — no error-text
    /// matching anywhere, and [`StoreError::HeadMoved`] answers `false`
    /// here forever.
    pub fn is_contention(&self) -> bool {
        matches!(self, StoreError::Contended { .. })
    }
}

pub struct Store {
    conn: Connection,
    path: PathBuf,
    /// The whole budget one store operation spends on a peer's lock —
    /// the connection's busy handler and [`patiently`]'s retry together,
    /// never one each. [`BUSY_TIMEOUT`] unless a caller says otherwise;
    /// see [`Store::set_patience`].
    patience: std::time::Duration,
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

/// A run's head: its last event's seq and hash, or 0 and [`ZERO_HASH`]
/// for a run with none. The one read both [`Store::head_hash`] and an
/// append's transaction make.
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
fn create_run_once(
    conn: &mut Connection,
    run_id: &str,
    feature: &str,
    bundle_name: &str,
    manifest: &str,
    origin: Option<&str>,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    refuse_existing(&tx, run_id)?;
    tx.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at, origin_host)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            run_id,
            feature,
            bundle_name,
            manifest,
            now_rfc3339(),
            origin
        ],
    )?;
    tx.commit()?;
    Ok(())
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

/// [`Store::append_next`]'s one transaction, as a body [`patiently`] may
/// run again.
///
/// Every fact it writes — the head it chains onto, the seq, the seal —
/// is derived INSIDE the transaction, so a retry is the same call made
/// again rather than a second one: an attempt that ends busy committed
/// nothing and the next attempt reads the journal fresh. The envelope it
/// discards was never sealed into anything.
///
/// The two refusals it can return, [`StoreError::HeadMoved`] and
/// [`StoreError::AppendConflict`], leave here untouched. They are not
/// busy errors, so [`patiently`] does not look at them — which is what
/// keeps a fence a fence.
fn append_once(
    conn: &mut Connection,
    run_id: &str,
    expected_head: Option<(u64, &str)>,
    event_type: EventType,
    payload: Value,
    causation_id: Option<String>,
    attempt_id: Option<String>,
) -> Result<EventEnvelope, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (last_seq, previous_hash) = head_of(&tx, run_id)?;
    if let Some((expected_seq, expected_hash)) = expected_head {
        if last_seq != expected_seq || previous_hash != expected_hash {
            // Dropping the transaction rolls it back: a fence that
            // fails writes nothing, not even the row it was building.
            return Err(StoreError::HeadMoved {
                expected_seq,
                found_seq: last_seq,
            });
        }
    }
    // The seat-record fence (decision 0034, ruling 6): a checkpoint or a
    // successful result that violates the contract is refused HERE, at
    // the seq it would have taken, before it is sealed into the chain.
    // The journal is append-only, so a nonconforming record that landed
    // could never be corrected — only discovered at export, when the run
    // was already wanted as evidence, and by then every verb that makes
    // evidence of it (export, anchor, offline verify) refuses the whole
    // run forever. Dropping the transaction rolls it back: a refused
    // record writes nothing, like a fence that fails.
    if let Some(record) = seat_record::record_of(event_type, &payload) {
        // Which contract this run writes under is settled once, at
        // `run/started`, and `runs.manifest` is immutable by trigger —
        // so the engine named there is the same answer the export and
        // verify sweeps reach through the journal, read here inside the
        // same transaction the row would be sealed in. SQLite extracts
        // the one string, so the manifest is not parsed whole under the
        // lock on every record (#354).
        let engine: Option<String> = tx
            .query_row(ENGINE_OF_RUN, params![run_id], |r| r.get(0))
            .optional()?
            .flatten();
        let version = SeatRecordVersion::of_manifest(engine.as_deref());
        seat_record::validate_seat_record(record, last_seq + 1, version)?;
    }
    let envelope = EventEnvelope {
        run_id: run_id.to_string(),
        seq: last_seq + 1,
        event_id: uuid::Uuid::new_v4().to_string(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id,
        correlation_id: run_id.to_string(),
        attempt_id,
        recorded_at: now_rfc3339(),
        previous_hash,
        event_hash: String::new(),
    }
    .sealed();
    let serialized = serde_json::to_string(&envelope)?;
    let inserted = tx.execute(
        "INSERT OR IGNORE INTO events (run_id, seq, event_hash, envelope)
         VALUES (?1, ?2, ?3, ?4)",
        params![run_id, envelope.seq as i64, envelope.event_hash, serialized],
    )?;
    if inserted == 0 {
        return Err(StoreError::AppendConflict { seq: envelope.seq });
    }
    tx.commit()?;
    Ok(envelope)
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
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        ensure_wal(&conn)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        patiently("migrate", BUSY_TIMEOUT, || Store::migrate(&mut conn))?;
        Ok(Store {
            conn,
            path: path.to_path_buf(),
            patience: BUSY_TIMEOUT,
        })
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
    /// of Muninn. No file is created and no migration runs — a missing
    /// database is an error, never an empty fleet.
    pub fn open_read_only(path: &Path) -> Result<Store, StoreError> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        let found: String = patiently("open_read_only", BUSY_TIMEOUT, || {
            Ok(conn.query_row(RECORDED_SCHEMA, [], |row| row.get(0))?)
        })?;
        schema_supported(found.parse().unwrap_or(0))?;
        Ok(Store {
            conn,
            path: path.to_path_buf(),
            patience: BUSY_TIMEOUT,
        })
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
        let manifest = serde_json::to_string(manifest)?;
        let origin = origin();
        let conn = &mut self.conn;
        patiently("create_run", self.patience, || {
            create_run_once(
                conn,
                run_id,
                feature,
                bundle_name,
                &manifest,
                origin.as_deref(),
            )
        })
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
        let origin: Option<String> = patiently("started_here", self.patience, || {
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
        let raw: Option<String> = patiently("manifest", self.patience, || {
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
        patiently("list_runs", self.patience, || {
            Ok(self
                .conn
                .prepare("SELECT run_id, feature, created_at FROM runs ORDER BY created_at")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        patiently("head", self.patience, || head_of(&self.conn, run_id))
    }

    /// Build, seal, and durably append the next event in one transaction.
    /// Envelope identity (seq, previous_hash) comes from the journal head
    /// inside the transaction; a concurrent writer conflicts instead of
    /// forking the chain.
    ///
    /// The head is whatever the transaction finds, so this always lands.
    /// A caller whose event is only legal against the head it read —
    /// anything it decided by folding — wants
    /// [`Store::append_next_if_head`] instead.
    pub fn append_next(
        &mut self,
        run_id: &str,
        event_type: EventType,
        payload: Value,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, StoreError> {
        self.append(run_id, None, event_type, payload, causation_id, attempt_id)
    }

    /// [`Store::append_next`], spending none of this store's patience: a
    /// peer that holds the write lock returns [`StoreError::Contended`] at
    /// once, and nothing was written. For a writer that keeps the row and
    /// offers it again later, so a peer's lock costs it no wait (#394).
    pub fn append_next_without_waiting(
        &mut self,
        run_id: &str,
        event_type: EventType,
        payload: Value,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, StoreError> {
        let patience = self.patience;
        self.set_patience(std::time::Duration::ZERO)?;
        let appended = self.append_next(run_id, event_type, payload, causation_id, attempt_id);
        self.set_patience(patience)?;
        appended
    }

    /// Append, but only onto the head the caller decided against —
    /// compare-and-append.
    ///
    /// [`Store::append_next`] recomputes the head inside its own
    /// transaction and always succeeds, which is right for a writer whose
    /// event is legal wherever it lands: the engine appending to its own
    /// run is the only writer of that run. It is wrong for a writer whose
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
            run_id,
            Some((expected_seq, expected_hash)),
            event_type,
            payload,
            causation_id,
            attempt_id,
        )
    }

    fn append(
        &mut self,
        run_id: &str,
        expected_head: Option<(u64, &str)>,
        event_type: EventType,
        payload: Value,
        causation_id: Option<String>,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, StoreError> {
        let conn = &mut self.conn;
        patiently("append", self.patience, || {
            append_once(
                conn,
                run_id,
                expected_head,
                event_type,
                payload.clone(),
                causation_id.clone(),
                attempt_id.clone(),
            )
        })
    }

    /// Load and verify the full journal of a run. A journal that fails
    /// chain verification is corrupt and is never partially returned.
    pub fn load(&self, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
        patiently("load", self.patience, || self.load_once(run_id))
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
        let mut out = String::new();
        for event in &events {
            out.push_str(&serde_json::to_string(&serde_json::to_value(event)?)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// The journal head hash — cheap identity for fencing and anchors.
    pub fn head_hash(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        self.head(run_id)
    }
}

fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
}

/// How long [`patiently`] waits between retries. Short enough that the
/// case it exists for — a `SQLITE_BUSY` returned in microseconds,
/// without the busy handler ever being asked — does not turn a
/// millisecond of contention into a visible stall; long enough that
/// losing a whole patience to a genuinely held lock costs a bounded
/// number of wakeups rather than a spin.
const CONTENTION_PAUSE: std::time::Duration = std::time::Duration::from_millis(5);

/// The ONE place a `SQLITE_BUSY` is handled, and the only one. Every
/// store operation an engine can reach runs inside it; no call site
/// anywhere else in the workspace retries anything.
///
/// The connection's busy handler is the first line and covers almost
/// everything, but it is not a promise SQLite always keeps. Its own
/// documentation says so: *"If SQLite determines that invoking the busy
/// handler could result in a deadlock, it will go ahead and return
/// SQLITE_BUSY to the application instead of invoking the busy
/// handler."* The btree layer's retry loop is guarded by
/// `pBt->inTransaction==TRANS_NONE`, so a write that follows a read
/// still open on the same connection is refused the handler outright and
/// comes back busy in **microseconds** — measured here at 12µs against a
/// peer's held write lock, versus the 30.03s the same contention costs a
/// connection the handler does cover. That is the shape a bounded
/// timeout cannot see: the budget was never spent, so nothing waited,
/// and the caller met `database is locked` with 29.99 seconds of
/// patience left in its pocket.
///
/// So patience is spent to a DEADLINE rather than counted in attempts:
/// whichever line of defence does the waiting, the total is one
/// [`Store::set_patience`] budget and never two. A handler that spends
/// the whole budget lands on the deadline and reports
/// [`StoreError::Contended`] at once; a busy the handler declined
/// returns instantly and this loop spends the rest.
///
/// What it retries is exactly one thing: a raw `SQLITE_BUSY` /
/// `SQLITE_LOCKED`. Every other error — including
/// [`StoreError::HeadMoved`] and [`StoreError::AppendConflict`], which
/// are refusals and not accidents — returns on the first attempt,
/// untouched. `attempt` must therefore write nothing outside its own
/// transaction, which is what makes re-running it the same call rather
/// than a second one.
fn patiently<T>(
    operation: &'static str,
    patience: std::time::Duration,
    mut attempt: impl FnMut() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let started = std::time::Instant::now();
    let deadline = started + patience;
    loop {
        match attempt() {
            Err(StoreError::Sqlite(error)) if is_busy(&error) => {
                if std::time::Instant::now() >= deadline {
                    return Err(StoreError::Contended {
                        operation,
                        waited_ms: started.elapsed().as_millis(),
                    });
                }
                std::thread::sleep(CONTENTION_PAUSE);
            }
            settled => return settled,
        }
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
