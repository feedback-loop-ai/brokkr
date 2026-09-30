//! The journal's schema (#354): the tables and guards a journal carries,
//! the migration that brings a file to them, and the WAL mode every
//! connection shares. Opening reads it; only a journal missing something
//! takes the write lock.

use rusqlite::{params, Connection, OptionalExtension};

use crate::queue::{migrate_queue, queue_intact};
use crate::{patiently, Store, StoreError, BUSY_TIMEOUT};

pub const DATABASE_SCHEMA: u32 = 1;

/// The schema a journal records. One statement for both readers of it:
/// [`schema_version`] on a file that may be virgin, and
/// [`Store::open_read_only`] on one that must not be.
pub(crate) const RECORDED_SCHEMA: &str = "SELECT value FROM meta WHERE key = 'database_schema'";

pub(crate) const MIGRATION_V1: &str = r#"
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
CREATE TRIGGER IF NOT EXISTS events_seq_is_a_position
    BEFORE INSERT ON events
    WHEN typeof(NEW.seq) != 'integer' OR NEW.seq < 1
    BEGIN SELECT RAISE(ABORT, 'events.seq is an integer from 1'); END;
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
pub(crate) const SIDECAR_COLUMNS: [(&str, &str); 3] = [
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

/// The columns the `runs` table carries, by name: the one question both
/// the steady-state check and the migration ask.
fn runs_columns(conn: &Connection) -> Result<Vec<String>, StoreError> {
    Ok(conn
        .prepare("SELECT name FROM pragma_table_info('runs')")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?)
}

/// Does the `runs` table lack any sidecar column? A read, so the
/// steady-state open writes nothing while an older journal still gets
/// noticed.
pub(crate) fn sidecar_columns_missing(conn: &Connection) -> Result<bool, StoreError> {
    let present = runs_columns(conn)?;
    Ok(SIDECAR_COLUMNS
        .iter()
        .any(|(column, _)| !present.iter().any(|name| name == column)))
}

/// Add the sidecar columns to a journal that predates them. SQLite has
/// no `ADD COLUMN IF NOT EXISTS`, so presence is asked rather than an
/// error swallowed — a swallowed error is how a real migration failure
/// would hide here.
fn migrate_sidecar_columns(conn: &Connection) -> Result<(), StoreError> {
    let present = runs_columns(conn)?;
    for (column, statement) in SIDECAR_COLUMNS {
        if !present.iter().any(|name| name == column) {
            conn.execute(statement, [])?;
        }
    }
    Ok(())
}

impl Store {
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
    pub(crate) fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
        match schema_version(conn)? {
            Some(found) => repair(conn, found),
            None => initialize(conn),
        }
    }
}

/// A journal that records a schema: supported, and whole.
fn repair(conn: &mut Connection, found: u32) -> Result<(), StoreError> {
    schema_supported(found)?;
    // Three additive repairs, each a READ in the steady state — the
    // starvation measurement holds — and each taking the immediate
    // transaction only when something is actually missing, so racing
    // openers serialise on the lock instead of colliding on the DDL.
    // First: a journal that predates the sidecar columns (arrival,
    // decision 0027; origin, decision 0030) grows them, presence
    // re-asked inside the transaction. Second: a journal whose append
    // guards predate compare-and-append re-runs the idempotent migration
    // batch that carries them.
    if sidecar_columns_missing(conn)? {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        migrate_sidecar_columns(&tx)?;
        tx.commit()?;
    }
    // Third: a journal from before the queue (decision 0068), or one that
    // lost a queue guard, gets the queue's tables and guards, on the same
    // terms. `DATABASE_SCHEMA` does not move for it, for the reason it
    // did not move for the sidecar columns.
    if !queue_intact(conn)? {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        migrate_queue(&tx)?;
        tx.commit()?;
    }
    if guards_intact(conn)? {
        return Ok(());
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(MIGRATION_V1)?;
    tx.commit()?;
    Ok(())
}

/// A journal with no schema recorded: the DDL and the seed row, under the
/// write lock taken up front.
fn initialize(conn: &mut Connection) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(MIGRATION_V1)?;
    migrate_sidecar_columns(&tx)?;
    migrate_queue(&tx)?;
    tx.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('database_schema', ?1)",
        [&DATABASE_SCHEMA.to_string()],
    )?;
    let found = schema_version(&tx)?.unwrap_or(0);
    tx.commit()?;
    schema_supported(found)
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
/// that really is not in WAL attempts the conversion, and [`patiently`]
/// retries it until [`BUSY_TIMEOUT`] elapses — the race is
/// self-resolving, because a peer that beats us to it leaves the file in
/// exactly the mode we wanted, which the next read observes.
pub(crate) fn ensure_wal(conn: &Connection) -> Result<(), StoreError> {
    ensure_wal_by(conn, BUSY_TIMEOUT)
}

/// [`ensure_wal`] with the patience in hand: an argument so the test
/// that proves it runs out does not have to wait it out. Each attempt
/// asks the mode afresh, so a retry after a busy conversion sees a peer's.
pub(crate) fn ensure_wal_by(
    conn: &Connection,
    patience: std::time::Duration,
) -> Result<(), StoreError> {
    patiently("ensure_wal", patience, || {
        let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        Ok(())
    })
}

/// The append-only and immutability triggers `MIGRATION_V1` installs, by
/// name. Named here so [`Store::migrate`] can ask whether a journal still
/// carries all of them.
const GUARD_TRIGGERS: [&str; 4] = [
    "events_append_only_update",
    "events_append_only_delete",
    "events_seq_is_a_position",
    "runs_manifest_immutable",
];

/// Does this journal still carry every guard trigger? A pure read of
/// `sqlite_master`, so an open that finds them all takes no write lock.
pub(crate) fn guards_intact(conn: &Connection) -> Result<bool, StoreError> {
    let present: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'trigger'
         AND name IN (?1, ?2, ?3, ?4)",
        params![
            GUARD_TRIGGERS[0],
            GUARD_TRIGGERS[1],
            GUARD_TRIGGERS[2],
            GUARD_TRIGGERS[3]
        ],
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
        .query_row(RECORDED_SCHEMA, [], |row| row.get(0))
        .optional()?;
    Ok(recorded.map(|value| value.parse().unwrap_or(0)))
}

pub(crate) fn schema_supported(found: u32) -> Result<(), StoreError> {
    if found != DATABASE_SCHEMA {
        return Err(StoreError::SchemaMismatch { found });
    }
    Ok(())
}
