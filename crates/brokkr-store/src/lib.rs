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

use std::path::{Path, PathBuf};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain, ChainError, EventEnvelope, EventType};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

mod seat_record;

pub use seat_record::{validate_seat_record, SeatRecordError, SeatRecordVersion};

pub const DATABASE_SCHEMA: u32 = 1;

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

/// Why an adoption refused. Every variant refuses the import WHOLE:
/// nothing is written until all of them have had their say, so a
/// destination journal never holds a prefix of a run it declined.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error(
        "refused: '{0}' is a redacted derivative — redaction rewrites payload \
         bytes and leaves the recorded hashes behind, so its chain can never \
         verify; an import of one could only ever adopt unverifiable content"
    )]
    Redacted(String),
    #[error(transparent)]
    Verify(#[from] VerifyError),
    #[error(
        "refused: this journal already carries run '{0}' — the run_id is hashed \
         into every envelope of its chain, so an adoption can neither rename it \
         nor overwrite what is already here; the collision is the operator's to rule on"
    )]
    Collision(String),
    #[error(
        "refused: '{0}' is not a run id this journal will carry — event hashes are \
         unkeyed, so a chain proves its bytes were not altered and never that whoever \
         sealed them was entitled to the name, and the name does not stay in the \
         database: `brokkr export` composes '<out>/<run_id>.ndjson' from it and every \
         readout prints it. An adoptable id is 1 to {RUN_ID_MAX} characters of ASCII \
         letters, digits, '-' and '_'"
    )]
    UnadoptableRunId(String),
    #[error(
        "refused: the export's run/started event carries no {field} — an adoption \
         derives the destination's runs row from the verified chain, never from \
         the sidecar manifest that no hash covers, and a readout that shows this \
         run must not show a blank where the chain said nothing"
    )]
    Unattested { field: &'static str },
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// A run's arrival: store bookkeeping BESIDE the chain, never inside it.
/// Absent means the run was driven here natively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    /// When the run landed in THIS journal (RFC3339).
    pub imported_at: String,
    /// Whence it came — the export this journal adopted.
    pub imported_from: String,
}

/// What an adoption did, for the operator's readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adoption {
    pub run_id: String,
    pub events: usize,
    /// The adopted chain's head — unchanged from the export, since the
    /// bytes it was computed over are unchanged.
    pub head_hash: String,
    pub arrival: Arrival,
}

/// The longest run id an adoption will carry. Native ids are a feature
/// slug of at most 32 characters and eight hex ones, so this is roomy
/// three times over; it exists because the id becomes a path component
/// on the next export, and a bound is cheaper than the error that would
/// otherwise come back from the filesystem.
const RUN_ID_MAX: usize = 128;

/// What a run id may contain for this journal to adopt it.
///
/// A native id is a lowercased feature slug and eight hex characters
/// (`Engine::start_in_world`), so this set is generous already. What it
/// excludes is what a *foreign* id could otherwise do here: `.` (and so
/// `..`), the path separators, and every control and formatting
/// character. Import is the first path by which a run id somebody else
/// authored reaches the `runs` table, and from there `brokkr export`'s
/// `<out>/<run_id>.ndjson` and the operator's terminal.
fn adoptable(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '_'
}

/// A rejected run id as it may be written into a refusal. Every
/// character the gate does not allow becomes its codepoint, so the one
/// line naming the refusal is ASCII by construction — an id that reached
/// here precisely by being unprintable does not get to reorder the
/// sentence that turns it away. The predicate is the gate's own, so the
/// two can never drift apart.
fn escaped(run_id: &str) -> String {
    run_id
        .chars()
        .map(|character| {
            if adoptable(character) {
                character.to_string()
            } else {
                format!("\\u{{{:04x}}}", character as u32)
            }
        })
        .collect()
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

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    feature TEXT NOT NULL,
    bundle_name TEXT NOT NULL,
    manifest TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    seq INTEGER NOT NULL,
    event_hash TEXT NOT NULL,
    envelope TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);
CREATE TRIGGER IF NOT EXISTS events_append_only_update
    BEFORE UPDATE ON events
    BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TRIGGER IF NOT EXISTS events_append_only_delete
    BEFORE DELETE ON events
    BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TRIGGER IF NOT EXISTS runs_manifest_immutable
    BEFORE UPDATE OF manifest, run_id ON runs
    BEGIN SELECT RAISE(ABORT, 'run manifests are immutable'); END;
"#;

/// Bookkeeping beside the chain: facts about a run that belong to this
/// journal rather than to the record inside it. Additive columns on a
/// table that is already store bookkeeping — no event carries any of
/// them, and no fold can observe any of them.
///
/// Arrival, decision 0027: where an adopted run came from and when it
/// landed. NULL means the run was driven here natively.
///
/// Origin, decision 0030: the machine and account this journal was
/// driving the run FROM when it created the run. It is written by
/// [`Store::create_run`] and by nothing else — an adopted run's row is
/// written by [`Store::import_run`], which leaves it NULL, and an
/// exported journal is events, so the column never travels with one.
/// That is the whole mechanism: [`Store::started_here`] answers "is this
/// run still being driven where it started", and a session handle is
/// offered to nobody else.
///
/// `DATABASE_SCHEMA` deliberately does not move. The version guards
/// *compatibility*, and columns nobody selects break nothing in either
/// direction: an older binary reads a migrated journal exactly as
/// before, and this binary migrates an older journal the moment it
/// opens it read-write. Bumping it would refuse both, buying nothing.
const SIDECAR_COLUMNS: [(&str, &str); 3] = [
    (
        "imported_at",
        "ALTER TABLE runs ADD COLUMN imported_at TEXT",
    ),
    (
        "imported_from",
        "ALTER TABLE runs ADD COLUMN imported_from TEXT",
    ),
    (
        "origin_host",
        "ALTER TABLE runs ADD COLUMN origin_host TEXT",
    ),
];

/// Where a machine says who it is, in the order asked. The machine id
/// where the OS publishes one; the kernel's hostname where it does not.
const MACHINE_SOURCES: [&str; 3] = [
    "/etc/machine-id",
    "/var/lib/dbus/machine-id",
    "/proc/sys/kernel/hostname",
];

/// An opaque fingerprint of the machine and the account this process is
/// running as: the first `sources` entry that can be read, else
/// `fallback`, folded together with the account's home directory — which
/// is where every driver brokkr ships keeps the credential that OWNS a
/// provider session (`~/.codex`, `~/.claude`). Hashed and clipped, so
/// what lands in the journal file is an equality token rather than an
/// operator's hostname and home path.
///
/// `None` when nothing identifying can be read, and a `None` is never
/// equal to anything — an installation that cannot say where it is
/// hands out no sessions, which is exactly what brokkr did before
/// decision 0030.
fn host_from(sources: &[&str], fallback: Option<String>) -> Option<String> {
    let machine = sources
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .or(fallback)?;
    let machine = machine.trim();
    if machine.is_empty() {
        return None;
    }
    let home = account_home();
    let digest = brokkr_core::canonical::sha256_hex(&serde_json::json!([machine, home]));
    Some(digest[..16].to_string())
}

/// The account's home directory, spelled the way each platform exports
/// it: `HOME` on unix, `USERPROFILE` on Windows. Empty when neither is
/// set, so the fingerprint still folds and the machine half decides.
fn account_home() -> String {
    home_from(&["HOME", "USERPROFILE"])
}

/// The first of `variables` that is set, else empty.
fn home_from(variables: &[&str]) -> String {
    variables
        .iter()
        .find_map(|variable| std::env::var(variable).ok())
        .unwrap_or_default()
}

/// The machine's name where no identity file can be read — the case on
/// every released platform but Linux. The first of `variables` that is
/// set and non-blank wins (`HOSTNAME`, which POSIX shells set but rarely
/// export; `COMPUTERNAME`, which Windows always publishes); failing
/// both, `ask` is consulted once. Blank answers count as none.
fn machine_name(variables: &[&str], ask: impl FnOnce() -> Option<String>) -> Option<String> {
    variables
        .iter()
        .find_map(|variable| std::env::var(variable).ok())
        .filter(|name| !name.trim().is_empty())
        .or_else(ask)
        .filter(|name| !name.trim().is_empty())
}

/// What `hostname` prints: the same spelling `/proc/sys/kernel/hostname`
/// carries on Linux, and the one thing macOS ships that names the
/// machine without a daemon or a crate. `None` when the command is
/// missing or fails, and a `None` hands out no sessions.
fn hostname_command() -> Option<String> {
    hostname_from("hostname")
}

/// `program`'s standard output when it runs and succeeds; `None` when
/// it is missing or exits nonzero.
fn hostname_from(program: &str) -> Option<String> {
    let out = std::process::Command::new(program).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// This machine and this account, as [`host_from`] fingerprints them.
fn local_host() -> Option<String> {
    host_from(
        &MACHINE_SOURCES,
        machine_name(&["HOSTNAME", "COMPUTERNAME"], hostname_command),
    )
}

/// Add the sidecar columns to a journal that predates them. SQLite has
/// no `ADD COLUMN IF NOT EXISTS`, so presence is asked rather than an
/// error swallowed — a swallowed error is how a real migration failure
/// would hide here.
/// Does the `runs` table lack any sidecar column? A read, so the
/// steady-state open writes nothing while an older journal still gets
/// noticed.
fn sidecar_columns_missing(conn: &Connection) -> Result<bool, StoreError> {
    let present: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('runs')")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SIDECAR_COLUMNS
        .iter()
        .any(|(column, _)| !present.iter().any(|name| name == column)))
}

fn migrate_sidecar_columns(conn: &Connection) -> Result<(), StoreError> {
    let present: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('runs')")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for (column, statement) in SIDECAR_COLUMNS {
        if !present.iter().any(|name| name == column) {
            conn.execute(statement, [])?;
        }
    }
    Ok(())
}

/// [`Store::create_run`]'s one transaction, as a body [`patiently`] may
/// run again. It reads and writes only inside that transaction, so an
/// attempt that ends busy has left the journal exactly as it found it.
fn create_run_once(
    conn: &mut Connection,
    run_id: &str,
    feature: &str,
    bundle_name: &str,
    manifest: &str,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let existing: Option<String> = tx
        .query_row(
            "SELECT run_id FROM runs WHERE run_id = ?1",
            params![run_id],
            |r| r.get(0),
        )
        .optional()?;
    if existing.is_some() {
        return Err(StoreError::RunExists(run_id.to_string()));
    }
    tx.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at, origin_host)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            run_id,
            feature,
            bundle_name,
            manifest,
            now_rfc3339(),
            local_host()
        ],
    )?;
    tx.commit()?;
    Ok(())
}

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
#[allow(clippy::too_many_arguments)]
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
    let head: Option<(i64, String)> = tx
        .query_row(
            "SELECT seq, event_hash FROM events WHERE run_id = ?1
             ORDER BY seq DESC LIMIT 1",
            params![run_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (last_seq, previous_hash) = match head {
        Some((seq, hash)) => (seq as u64, hash),
        None => (0, ZERO_HASH.to_string()),
    };
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
    /// [`ensure_wal_by`]'s deadline is one: the tests that prove a budget
    /// runs out must not have to wait it out.
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

    /// Bring a journal to [`DATABASE_SCHEMA`], writing only when it is
    /// not already there.
    ///
    /// Opening is the one thing every burn does, so it must not take the
    /// database's write lock in the steady state. The prologue used to
    /// run `MIGRATION_V1` and an unconditional `INSERT OR IGNORE INTO
    /// meta` on every open. Measured against a single peer appending
    /// back-to-back, that insert starved past a *whole* busy timeout,
    /// over and over: an implicit statement runs in a deferred
    /// transaction, which reads first and only then asks to upgrade to
    /// the write lock, and against a steady writer that upgrade loses
    /// indefinitely. The DDL itself was innocent — `CREATE … IF NOT
    /// EXISTS` short-circuits without a write lock once the objects
    /// exist — but the insert dragged it down with it.
    ///
    /// So: read the recorded schema first, and return on the common path
    /// having written nothing. Only a journal with no schema recorded
    /// gets the DDL and the seed row, and those go inside an `Immediate`
    /// transaction — the write lock taken up front, the same discipline
    /// [`Store::append_next`] uses and the one measurement shows
    /// survives contention. A peer that wins the initialization race in
    /// between is harmless: the DDL is `IF NOT EXISTS`, the seed is
    /// `INSERT OR IGNORE`, and the schema is re-read inside the
    /// transaction, so both writers agree on what they found.
    ///
    /// One thing the old unconditional `MIGRATION_V1` did buy, and this
    /// keeps: a journal whose append-only guards have gone missing gets
    /// them back. `IF NOT EXISTS` DDL on every open repaired that for
    /// free; reading the schema instead would have left a journal with a
    /// recorded version and no triggers unguarded forever. So the guards
    /// are counted — a read, on the steady path, like the schema — and
    /// only a journal actually missing one takes the write lock.
    fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
        if let Some(found) = schema_version(conn)? {
            schema_supported(found)?;
            // Two additive repairs, each a READ in the steady state —
            // the starvation measurement holds — and each taking the
            // immediate transaction only when something is actually
            // missing, so racing openers serialise on the lock instead
            // of colliding on the DDL. First: a journal that predates
            // the sidecar columns (arrival, decision 0027; origin,
            // decision 0030) grows them, presence re-asked inside the
            // transaction. Second: a journal whose append guards predate
            // compare-and-append re-runs the idempotent migration batch
            // that carries them.
            if sidecar_columns_missing(conn)? {
                let tx =
                    conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
                migrate_sidecar_columns(&tx)?;
                tx.commit()?;
            }
            if guards_intact(conn)? {
                return Ok(());
            }
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute_batch(MIGRATION_V1)?;
            tx.commit()?;
            return Ok(());
        }
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V1)?;
        migrate_sidecar_columns(&tx)?;
        tx.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('database_schema', ?1)",
            [&DATABASE_SCHEMA.to_string()],
        )?;
        let found = schema_version(&tx)?.unwrap_or(0);
        tx.commit()?;
        schema_supported(found)
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
            Ok(conn.query_row(
                "SELECT value FROM meta WHERE key = 'database_schema'",
                [],
                |row| row.get(0),
            )?)
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
        let manifest = serde_json::to_string(manifest)?;
        let conn = &mut self.conn;
        patiently("create_run", self.patience, || {
            create_run_once(conn, run_id, feature, bundle_name, &manifest)
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
        self.started_under(run_id, local_host())
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
        let mut stmt = self
            .conn
            .prepare("SELECT run_id, feature, created_at FROM runs ORDER BY created_at")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        patiently("head", self.patience, || self.head_once(run_id))
    }

    fn head_once(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        let head: Option<(i64, String)> = self
            .conn
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
    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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
        let mut stmt = self
            .conn
            .prepare("SELECT envelope FROM events WHERE run_id = ?1 ORDER BY seq")?;
        let events = stmt
            .query_map(params![run_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|raw| serde_json::from_str::<EventEnvelope>(&raw))
            .collect::<Result<Vec<_>, _>>()?;
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

    /// Adopt an exported run into this journal byte-identically — the
    /// verb paired with [`Store::export_ndjson`]. Journals never merge,
    /// but one run can relocate.
    ///
    /// Nothing lands until every gate passes, in this order:
    ///
    /// 1. The export must not be a **redacted derivative**, refused by
    ///    its own markers before its content is read at all — the
    ///    `.redacted.` filename [`redact_export`] mandates, or a
    ///    `"redacted": true` sidecar manifest. Redaction rewrites
    ///    payload bytes and leaves the recorded hashes behind, so a
    ///    redacted export's chain can never verify; there is no import
    ///    of one that would not be an adoption of unverifiable content.
    /// 2. The **chain must verify whole**. One broken link refuses the
    ///    import — never a good prefix of it.
    /// 3. Every checkpoint and successful result must satisfy the frozen
    ///    **seat-record contract**. No malformed record is adopted as
    ///    evidence merely because its enclosing hash chain is intact.
    /// 4. The events must **fold**, and a `FoldError` refuses with the
    ///    same citation a quarantined run shows anywhere else.
    /// 5. The **run_id must be one this journal would carry**. A
    ///    verified chain proves its bytes were not altered; the hashes
    ///    are unkeyed, so it never proves the sealer was entitled to the
    ///    name. See [`adoptable`].
    /// 6. The destination must not already carry this **run_id**. The
    ///    run_id is hashed into every envelope, so a collision is
    ///    structurally not a rename-and-retry: it is the operator's to
    ///    rule on. A second import of the same export refuses here for
    ///    exactly the same reason a genuinely different run sharing a
    ///    run_id does — adoption is not idempotent, it is once.
    ///
    /// The adopted events are then written exactly as exported: same
    /// bytes, same hashes, same seqs, same `recorded_at`, same run_id.
    /// [`Store::append_next`] is the wrong primitive for that by
    /// design — it seals a *fresh* envelope from the destination's head
    /// — so this appends pre-sealed envelopes verbatim, all of them in
    /// one transaction or none of them.
    ///
    /// The `runs` row is derived from the **verified chain**: the run
    /// manifest rides inside `run/started`'s payload, so every column
    /// comes from bytes a hash covers. The sidecar manifest is consulted
    /// for one thing only — its redaction marker — where trusting an
    /// uncovered file can only ever cause a refusal. `created_at` is the
    /// first event's `recorded_at`, so the adopted run sorts into the
    /// fleet where it actually ran.
    ///
    /// Where the run arrived from and when is recorded in the `runs`
    /// table BESIDE the chain, never inside it. `brokkr_core::fold`
    /// cannot observe it, so `state = fold(events)` holds identically
    /// for a native run and an adopted one.
    pub fn import_run(
        &mut self,
        ndjson: &str,
        manifest: &Value,
        origin: &Path,
    ) -> Result<Adoption, ImportError> {
        let named = origin
            .file_name()
            .unwrap_or(origin.as_os_str())
            .to_string_lossy()
            .to_string();
        // Gate 1, twice — the two marks `export --redact` writes. The
        // filename catches the pair as published; the manifest flag
        // catches a copy somebody renamed back.
        if named.contains(".redacted.") {
            return Err(ImportError::Redacted(named));
        }
        if manifest.get("redacted") == Some(&Value::Bool(true)) {
            return Err(ImportError::Redacted(named));
        }

        // Gates 2 and 3: parse, chain, fold — the same three checks
        // `verify-run` makes offline, on the same bytes, before this
        // journal is touched.
        let (events, state) = verified_events(ndjson)?;
        // The fold proved the first event is `run/started`, so it
        // exists; what it does not prove is that the payload carries
        // everything a `runs` row needs.
        let started = &events[0];
        // Gate 4, before the name is derived from, stored under, or
        // printed with. `verify_chain` proved every envelope carries
        // this one run_id (`ChainError::ForeignRun`) and that no byte of
        // any of them was altered — so checking the first is checking
        // them all, and what is left to check is the name itself.
        if started.run_id.is_empty()
            || started.run_id.chars().count() > RUN_ID_MAX
            || !started.run_id.chars().all(adoptable)
        {
            return Err(ImportError::UnadoptableRunId(escaped(&started.run_id)));
        }
        let feature = state
            .feature
            .ok_or(ImportError::Unattested { field: "feature" })?;
        let run_manifest = started
            .payload
            .get("manifest")
            .ok_or(ImportError::Unattested { field: "manifest" })?;
        // `runs.bundle_name` is a denormalization of the manifest key,
        // and nothing ever selects the column — `Store::manifest` and
        // every reader of it go to `runs.manifest`, which carries the
        // chain's own manifest verbatim. So a manifest without the key
        // is copied as faithfully as it can be, not refused over a
        // column no readout would have shown.
        let bundle_name = run_manifest
            .get("bundle_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let run_id = started.run_id.clone();
        let head_hash = events
            .last()
            .expect("a folded journal has a last event")
            .event_hash
            .clone();
        let arrival = Arrival {
            imported_at: now_rfc3339(),
            imported_from: origin.display().to_string(),
        };

        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(StoreError::from)?;
        // Gate 5 inside the transaction that would write, so a
        // concurrent adoption of the same run loses here rather than
        // forking the destination.
        let existing: Option<String> = tx
            .query_row(
                "SELECT run_id FROM runs WHERE run_id = ?1",
                params![&run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)?;
        if existing.is_some() {
            return Err(ImportError::Collision(run_id));
        }
        tx.execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at,
                               imported_at, imported_from)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &run_id,
                feature,
                bundle_name,
                serde_json::to_string(run_manifest).map_err(StoreError::from)?,
                started.recorded_at,
                arrival.imported_at,
                arrival.imported_from,
            ],
        )
        .map_err(StoreError::from)?;
        for event in &events {
            // Serialized the way `append_next` stores its own — the
            // export form is re-derived by `export_ndjson`, so a
            // re-export of this run reproduces the source bytes.
            tx.execute(
                "INSERT INTO events (run_id, seq, event_hash, envelope)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    &run_id,
                    event.seq as i64,
                    event.event_hash,
                    serde_json::to_string(event).map_err(StoreError::from)?,
                ],
            )
            .map_err(StoreError::from)?;
        }
        tx.commit().map_err(StoreError::from)?;
        Ok(Adoption {
            run_id,
            events: events.len(),
            head_hash,
            arrival,
        })
    }

    /// How a run got here: `None` for a run driven natively in this
    /// journal, `Some` for one adopted by [`Store::import_run`].
    /// Provenance is queryable without reading a single event, which is
    /// the whole point of recording it beside the chain.
    ///
    /// A journal opened read-only that predates the arrival columns has
    /// none to select; a read-write open migrates before anything asks.
    pub fn arrival(&self, run_id: &str) -> Result<Option<Arrival>, StoreError> {
        let row: Option<(Option<String>, Option<String>)> = self
            .conn
            .query_row(
                "SELECT imported_at, imported_from FROM runs WHERE run_id = ?1",
                params![run_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (imported_at, imported_from) =
            row.ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(match (imported_at, imported_from) {
            (Some(imported_at), Some(imported_from)) => Some(Arrival {
                imported_at,
                imported_from,
            }),
            _ => None,
        })
    }

    /// The journal head hash — cheap identity for fencing and anchors.
    pub fn head_hash(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        self.head(run_id)
    }
}

/// Verify an exported NDJSON journal offline: parse, chain, seat record, fold.
pub fn verify_export(ndjson: &str) -> Result<brokkr_core::RunState, VerifyError> {
    verified_events(ndjson).map(|(_, state)| state)
}

/// The same four checks as [`verify_export`], handing back the
/// envelopes as well as the state they fold to. An adoption needs the
/// envelopes themselves — it writes their exact bytes — and must make
/// the checks on the very bytes it adopts, not on a second parse of
/// them.
pub fn verified_events(
    ndjson: &str,
) -> Result<(Vec<EventEnvelope>, brokkr_core::RunState), VerifyError> {
    let events = ndjson
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<EventEnvelope>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(VerifyError::Parse)?;
    verify_chain(&events).map_err(VerifyError::Chain)?;
    seat_record::validate_events(&events).map_err(VerifyError::SeatRecord)?;
    let state = brokkr_core::fold(&events).map_err(VerifyError::Fold)?;
    Ok((events, state))
}

/// Sanitize a canonical export for publication: every absolute
/// filesystem path inside event payload string fields is rewritten to a
/// stable placeholder — `[path-1]`, `[path-2]`, … in first-appearance
/// order, the same original path always to the same placeholder,
/// distinct paths to distinct placeholders — so operator-machine detail
/// (and any username inside a path) never leaves the machine. Envelope
/// fields, seqs, structure, and the recorded hashes are untouched, which
/// means the output's event hashes no longer re-verify by construction:
/// a redacted export is evidence of shape, never of authorship. Hash
/// verification applies only to the verbatim export, and the caller must
/// mark the copy unmistakably (the CLI writes `.redacted.` filenames and
/// a marked manifest). The recognizer covers POSIX (`/`-rooted),
/// Windows drive-letter (`C:/`, `C:\\`), and UNC (`\\\\server`) paths,
/// quote-aware so a quoted path with spaces moves whole; scheme URLs
/// (`://`, `file:///…` included) survive verbatim as a declared bound.
/// A caller publishing more than the journal — the CLI also writes a
/// manifest — scrubs the whole set through one [`Redactor`] instead, so
/// the files it publishes agree.
pub fn redact_export(ndjson: &str) -> Result<String, serde_json::Error> {
    Redactor::learn(&[ndjson]).journal(ndjson)
}

/// One redaction, shared across every document published together.
///
/// A journal and the manifest beside it are one piece of evidence in two
/// files, so they must be scrubbed with ONE vocabulary: the same original
/// path always becomes the same `[path-N]` in both, and machine detail
/// that the journal hides cannot survive in the manifest. Two files
/// scrubbed independently would be worse than either alone — they would
/// disagree about what `[path-1]` means, and the un-scrubbed one would
/// hand back exactly what the other was published to withhold.
pub struct Redactor {
    users: Vec<String>,
    table: PathTable,
}

impl Redactor {
    /// Learn the machine detail in every document that will be scrubbed
    /// together, so the username numbering does not depend on which
    /// document happens to be scrubbed first.
    pub fn learn(sources: &[&str]) -> Redactor {
        // A username learned from a home-directory path is machine
        // detail wherever it reappears — the journal records the
        // operator by OS username — so it is scrubbed even outside
        // paths.
        Redactor {
            users: harvest_usernames(&sources.join("\n")),
            table: PathTable::default(),
        }
    }

    /// Scrub an NDJSON journal: payloads only, envelopes untouched.
    pub fn journal(&mut self, ndjson: &str) -> Result<String, serde_json::Error> {
        let mut out = String::new();
        for line in ndjson.lines().filter(|line| !line.trim().is_empty()) {
            let mut envelope: Value = serde_json::from_str(line)?;
            if let Some(payload) = envelope.get_mut("payload") {
                redact_value(payload, &mut self.table, &self.users);
            }
            out.push_str(&serde_json::to_string(&envelope)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// Scrub a whole document — a run manifest, which is not an event
    /// and has no payload to descend into.
    pub fn document(&mut self, value: &Value) -> Value {
        let mut scrubbed = value.clone();
        redact_value(&mut scrubbed, &mut self.table, &self.users);
        scrubbed
    }
}

/// Every username the export leaks through a home-directory path, in
/// first-appearance order so `[user-N]` numbering is deterministic.
fn harvest_usernames(ndjson: &str) -> Vec<String> {
    let mut found: Vec<(usize, String)> = Vec::new();
    for marker in ["/home/", "/Users/", "\\Users\\", ":/Users/"] {
        for (at, _) in ndjson.match_indices(marker) {
            let user: String = ndjson[at + marker.len()..]
                .chars()
                .take_while(|c| is_username_char(*c))
                .collect();
            if !user.is_empty() {
                found.push((at, user));
            }
        }
    }
    found.sort();
    let mut users = Vec::new();
    for (_, user) in found {
        if !users.contains(&user) {
            users.push(user);
        }
    }
    users
}

/// Original path → placeholder, shared across the whole export so
/// distinctness is preserved journal-wide, not per event.
#[derive(Default)]
struct PathTable {
    placeholders: std::collections::HashMap<String, String>,
}

impl PathTable {
    fn placeholder(&mut self, path: &str) -> &str {
        let next = self.placeholders.len() + 1;
        self.placeholders
            .entry(path.to_string())
            .or_insert_with(|| format!("[path-{next}]"))
    }
}

fn redact_value(value: &mut Value, table: &mut PathTable, users: &[String]) {
    match value {
        Value::String(text) => {
            let replaced = redact_usernames(&redact_string(text, table), users);
            *text = replaced;
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item, table, users);
            }
        }
        Value::Object(fields) => {
            for field in fields.values_mut() {
                redact_value(field, table, users);
            }
        }
        _ => {}
    }
}

/// Bare occurrences of a harvested username become `[user-N]`. Only
/// whole tokens move: a username embedded in a longer path-ish word
/// (`carolyn`, `alice.txt`) is somebody else's text.
fn redact_usernames(input: &str, users: &[String]) -> String {
    let mut text = input.to_string();
    for (index, user) in users.iter().enumerate() {
        let placeholder = format!("[user-{}]", index + 1);
        text = replace_bounded(&text, user, &placeholder);
    }
    text
}

fn replace_bounded(text: &str, needle: &str, placeholder: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(needle) {
        let end = at + needle.len();
        let before_ok =
            at == 0 || !is_username_char(rest[..at].chars().next_back().expect("at > 0"));
        let after_ok =
            end == rest.len() || !is_username_char(rest[end..].chars().next().expect("end < len"));
        out.push_str(&rest[..at]);
        if before_ok && after_ok {
            out.push_str(placeholder);
        } else {
            out.push_str(&rest[at..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn redact_string(input: &str, table: &mut PathTable) -> String {
    let mut out = String::with_capacity(input.len());
    let mut from = 0;
    while let Some((start, end)) = next_absolute_path(input, from) {
        out.push_str(&input[from..start]);
        out.push_str(table.placeholder(&input[start..end]));
        from = end;
    }
    out.push_str(&input[from..]);
    out
}

/// Find the next absolute path: POSIX (`/…`), drive-letter (`C:/…`,
/// `C:\…`), or UNC (`\\server\…`). A path opened right after a quote
/// runs to the matching quote, so spaces inside quoted paths survive as
/// path text; an unquoted one runs to the next separator. `://` never
/// opens a path, so scheme URLs (`file:///…` included) pass verbatim —
/// a declared bound of the scheme, recorded in the manifest marking.
fn next_absolute_path(text: &str, from: usize) -> Option<(usize, usize)> {
    for (offset, _) in text[from..].char_indices() {
        let start = from + offset;
        if is_path_start(text, start) {
            let quoted_by = text[..start]
                .chars()
                .next_back()
                .filter(|character| matches!(character, '\'' | '"' | '`'));
            // A drive-letter path owns the colon at its second
            // character; every other colon ends a path, which is what
            // splits `PATH=/usr/bin:/home/x` into two redactions.
            let drive = text.as_bytes()[start..].get(1) == Some(&b':');
            let end = text[start..]
                .char_indices()
                .skip(1)
                .find_map(|(offset, character)| {
                    let ends = match quoted_by {
                        Some(quote) => character == quote,
                        None => {
                            path_end(character) || (character == ':' && !(drive && offset == 1))
                        }
                    };
                    ends.then_some(start + offset)
                })
                .unwrap_or(text.len());
            // A separator alone (`/`, `end /`) is punctuation, not a
            // path: something must follow the opening marker.
            if text[start..end].chars().count() >= 2 {
                return Some((start, end));
            }
        }
    }
    None
}

fn is_path_start(text: &str, start: usize) -> bool {
    let bytes = &text.as_bytes()[start..];
    let previous = text[..start].chars().next_back();
    let boundary = previous.is_none_or(path_boundary)
        || (previous == Some(':') && bytes.get(1) != Some(&b'/'));
    if !boundary {
        return false;
    }

    bytes.first() == Some(&b'/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
        || bytes.starts_with(b"\\\\")
}

fn path_boundary(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '\'' | '"' | '`' | '=' | '(' | '[' | '{' | ',' | ';' | '|' | '&' | '<' | '>'
        )
}

fn path_end(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '\'' | '"' | '`' | ',' | ';' | '|' | '&' | '<' | '>' | ')' | ']' | '}'
        )
}

/// A username token's characters: any alphabetic script counts, so
/// `józef` bounds the same way `alice` does.
fn is_username_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '_' | '-')
}

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("parse: {0}")]
    Parse(serde_json::Error),
    #[error("chain: {0}")]
    Chain(ChainError),
    #[error(transparent)]
    SeatRecord(SeatRecordError),
    #[error("fold: {0}")]
    Fold(brokkr_core::FoldError),
}

/// Put the journal in WAL, which is what lets readers and a writer share
/// one realm file at all.
///
/// The busy timeout does not cover this one. Converting a database to
/// WAL needs a moment with no other connection on the file, and SQLite
/// answers `SQLITE_BUSY` *without* consulting the busy handler when it
/// cannot get it — so a plain `pragma_update` is a single throw of the
/// dice. Measured on a virgin realm journal opened by six burns at once,
/// one of the six lost that throw and the whole open failed, even with
/// the timeout already set.
///
/// The conversion is also unnecessary almost always: a journal is WAL
/// from birth and stays that way, so ask what mode the file is in first
/// and, in the overwhelmingly common case, write nothing. Only a file
/// that really is not in WAL attempts the conversion, and it retries
/// until [`BUSY_TIMEOUT`] elapses — the race is self-resolving, because
/// a peer that beats us to it leaves the file in exactly the mode we
/// wanted, which the next read observes.
fn ensure_wal(conn: &Connection) -> Result<(), StoreError> {
    ensure_wal_by(conn, std::time::Instant::now() + BUSY_TIMEOUT)
}

/// [`ensure_wal`] with the deadline in hand: patience is an argument so
/// the test that proves it runs out does not have to wait it out.
fn ensure_wal_by(conn: &Connection, deadline: std::time::Instant) -> Result<(), StoreError> {
    loop {
        let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        if mode.eq_ignore_ascii_case("wal") {
            return Ok(());
        }
        match conn.pragma_update(None, "journal_mode", "WAL") {
            Ok(()) => return Ok(()),
            Err(error) if is_busy(&error) && std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(error) => return Err(error.into()),
        }
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

/// The append-only and immutability triggers `MIGRATION_V1` installs, by
/// name. Named here so [`Store::migrate`] can ask whether a journal still
/// carries all of them.
const GUARD_TRIGGERS: [&str; 3] = [
    "events_append_only_update",
    "events_append_only_delete",
    "runs_manifest_immutable",
];

/// Does this journal still carry every guard trigger? A pure read of
/// `sqlite_master`, so an open that finds them all takes no write lock.
fn guards_intact(conn: &Connection) -> Result<bool, StoreError> {
    let present: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'trigger'
         AND name IN (?1, ?2, ?3)",
        params![GUARD_TRIGGERS[0], GUARD_TRIGGERS[1], GUARD_TRIGGERS[2]],
        |row| row.get(0),
    )?;
    Ok(present == GUARD_TRIGGERS.len() as i64)
}

/// The schema a journal records, or `None` for a file that has never
/// been migrated. A pure read: it asks `sqlite_master` whether `meta`
/// exists rather than provoking a "no such table" error, so it is safe
/// on a virgin file and takes no write lock on an established one.
fn schema_version(conn: &Connection) -> Result<Option<u32>, StoreError> {
    let has_meta = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !has_meta {
        return Ok(None);
    }
    let recorded: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'database_schema'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(recorded.map(|value| value.parse().unwrap_or(0)))
}

fn schema_supported(found: u32) -> Result<(), StoreError> {
    if found != DATABASE_SCHEMA {
        return Err(StoreError::SchemaMismatch { found });
    }
    Ok(())
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
