//! The journal's schema (#354): the tables and guards a journal carries,
//! the migration that brings a file to them, and the WAL mode every
//! connection shares. Opening reads it; only a journal missing something
//! takes the write lock.

use rusqlite::{Connection, OptionalExtension};

use crate::acknowledgement::{acknowledgements_intact, migrate_acknowledgements};
use crate::event_index::{index_intact, migrate_index};
use crate::queue::{migrate_queue, queue_intact};
use crate::run_select;
use crate::seat_summary::{mark_seated, migrate_seats, seats_intact};
use crate::summary::{mark_backfilled, migrate_summary, summary_intact};
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
    // Nine additive repairs, each a READ in the steady state — the
    // starvation measurement holds — and each taking the immediate
    // transaction only when something is actually missing, so racing
    // openers serialise on the lock instead of colliding on the DDL.
    // First: a journal that predates the sidecar columns (arrival,
    // decision 0027; origin, decision 0030) grows them, presence
    // re-asked inside the transaction. Second: a journal whose append
    // guards predate compare-and-append re-runs the idempotent migration
    // batch that carries them.
    repair_tables(conn)?;
    repair_projections(conn)?;
    // Eighth: a journal from before it named itself (decision 0095, #430)
    // gets its id, once; a peer that named it first keeps its own.
    let missing = journal_named(conn)?.is_none();
    repair_if(conn, missing, name_journal)?;
    let missing = !guards_intact(conn)?;
    repair_if(conn, missing, |conn| Ok(conn.execute_batch(MIGRATION_V1)?))?;
    // Ninth: the run selector's `runs (created_at)` index (decision 0089
    // slice 4). A journal read-only without it still resolves — SQLite
    // scans — and the next writable open adds it.
    let missing = !run_select::intact(conn)?;
    repair_if(conn, missing, run_select::migrate)
}

/// The first, third and fourth of [`repair`]'s additive repairs, in its
/// order: the sidecar columns, the queue, the acknowledgements.
fn repair_tables(conn: &mut Connection) -> Result<(), StoreError> {
    let missing = sidecar_columns_missing(conn)?;
    repair_if(conn, missing, migrate_sidecar_columns)?;
    // Third: a journal from before the queue (decision 0068), or one that
    // lost a queue guard, gets the queue's tables and guards, on the same
    // terms; one that lost a queue table is refused. `DATABASE_SCHEMA`
    // does not move for it, for the reason it did not move for the
    // sidecar columns.
    let missing = !queue_intact(conn)?;
    repair_if(conn, missing, migrate_queue)?;
    // Fourth: a journal from before quarantine acknowledgements (#511),
    // or one that lost the table or a guard on it, gets them.
    let missing = !acknowledgements_intact(conn)?;
    repair_if(conn, missing, migrate_acknowledgements)
}

/// The fifth, sixth and seventh of [`repair`]'s additive repairs, in its
/// order: decision 0089's run summary, event index and seat summary.
fn repair_projections(conn: &mut Connection) -> Result<(), StoreError> {
    // Fifth: a journal from before the run summary (decision 0089), or
    // one that lost its table, index or a trigger, gets them, and a row
    // for every run it carries, kept at its head and folded by the
    // backfill that follows the open ([`Store::catch_up`]).
    let missing = !summary_intact(conn)?;
    repair_if(conn, missing, migrate_summary)?;
    // Sixth: a journal from before the event index (decision 0089 slice
    // 2), or one that lost its table, an index or a trigger, gets them,
    // and its rows from the backfill that follows the open
    // ([`Store::index_behind_with`]), which a repair marks unfinished.
    let missing = !index_intact(conn)?;
    repair_if(conn, missing, migrate_index)?;
    // Seventh: a journal from before the seat summary (decision 0089
    // slice 2), or one that lost a table of it, gets them, and its rows
    // from the walk that follows the open ([`Store::seat_behind_with`]),
    // which a repair marks unfinished.
    let missing = !seats_intact(conn)?;
    repair_if(conn, missing, migrate_seats)
}

/// Apply `fix` under the immediate transaction, and only when `missing`:
/// the steady path stays a read, and racing openers serialise on the
/// lock rather than collide on the DDL.
fn repair_if(
    conn: &mut Connection,
    missing: bool,
    fix: impl FnOnce(&Connection) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    if !missing {
        return Ok(());
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    fix(&tx)?;
    tx.commit()?;
    Ok(())
}

/// A journal with no schema recorded: the DDL and the seed row, under the
/// write lock taken up front.
fn initialize(conn: &mut Connection) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(MIGRATION_V1)?;
    initialize_additions(&tx)?;
    name_journal(&tx)?;
    tx.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('database_schema', ?1)",
        [&DATABASE_SCHEMA.to_string()],
    )?;
    let found = schema_version(&tx)?.unwrap_or(0);
    tx.commit()?;
    schema_supported(found)
}

/// [`initialize`]'s additive pieces, in [`repair`]'s order: the sidecar
/// columns, the queue, the acknowledgements, the run summary, the event
/// index and the seat summary, with the summary and seat marks a fresh
/// journal seeds.
fn initialize_additions(conn: &Connection) -> Result<(), StoreError> {
    migrate_sidecar_columns(conn)?;
    migrate_queue(conn)?;
    migrate_acknowledgements(conn)?;
    migrate_summary(conn)?;
    mark_backfilled(conn)?;
    // The selector's index is additive like the ones above, so a fresh
    // journal gets it here and an older one in `repair`.
    run_select::migrate(conn)?;
    // No index backfill is marked here: a journal with no schema recorded
    // may still carry events, and the open's walk, one read on an empty
    // journal, records the mark once every event has its row.
    migrate_index(conn)?;
    // A run without a seat head reads as stale whatever this key says, and
    // the open's walk folds it, so a new journal is marked here.
    migrate_seats(conn)?;
    mark_seated(conn)?;
    Ok(())
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
    all_present(conn, "trigger", &GUARD_TRIGGERS)
}

/// Does this journal carry every object of `kind` (`table`, `trigger`)
/// that `names` lists? A pure read of `sqlite_master`: the one presence
/// question every additive repair asks before it takes the write lock.
pub(crate) fn all_present(
    conn: &Connection,
    kind: &str,
    names: &[&str],
) -> Result<bool, StoreError> {
    let present: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1")?
        .query_map([kind], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(names
        .iter()
        .all(|name| present.iter().any(|found| found == name)))
}

/// Does `table` declare every one of `columns`? A pure read of its
/// columns in one statement, however many it asks after.
pub(crate) fn holds<'a>(
    conn: &Connection,
    table: &str,
    columns: impl IntoIterator<Item = &'a str>,
) -> Result<bool, StoreError> {
    let mut statement = conn.prepare("SELECT name FROM pragma_table_info(?1)")?;
    let found: Vec<String> = statement
        .query_map([table], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    let mut columns = columns.into_iter();
    Ok(columns.all(|column| found.iter().any(|name| name == column)))
}

/// The schema a journal records, or `None` for a file that has never
/// been migrated. A pure read: it asks `sqlite_master` whether `meta`
/// exists rather than provoking a "no such table" error, so it is safe
/// on a virgin file and takes no write lock on an established one.
pub(crate) fn schema_version(conn: &Connection) -> Result<Option<u32>, StoreError> {
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

/// The meta key a journal names itself by (decision 0095, #430): a random
/// id, written once, when the journal is created or first opened for
/// writing without one, and never rewritten. A byte copy carries it.
const JOURNAL_ID: &str = "journal_id";

/// The id `conn`'s journal names itself by, as stored, or `None` for a
/// journal not yet named. Read through `conn`, so it is the file the
/// connection holds, whatever its path names now.
pub(crate) fn journal_named(conn: &Connection) -> Result<Option<String>, StoreError> {
    let query = "SELECT value FROM meta WHERE key = ?1";
    Ok(conn
        .query_row(query, [JOURNAL_ID], |row| row.get(0))
        .optional()?)
}

/// Name a journal with a v4 UUID drawn from the OS's random source, unless
/// a peer named it first.
fn name_journal(conn: &Connection) -> Result<(), StoreError> {
    let id = uuid::Uuid::new_v4().to_string();
    let named = "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, ?2)";
    conn.execute(named, [JOURNAL_ID, id.as_str()])?;
    Ok(())
}

pub(crate) fn schema_supported(found: u32) -> Result<(), StoreError> {
    if found != DATABASE_SCHEMA {
        return Err(StoreError::SchemaMismatch { found });
    }
    Ok(())
}
