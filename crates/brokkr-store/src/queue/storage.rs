//! Where the queue is kept (decision 0068 ruling 1): its tables and
//! guards, the storage version a journal records for them, and the
//! migration that brings a journal to that version.

use rusqlite::{params, Connection, OptionalExtension};

use super::QueueRefusal;
use crate::StoreError;

/// The queue storage a journal records under `meta.queue_schema`, apart
/// from `database_schema`: the queue is additive, so an older binary still
/// reads a journal that carries one, and only the queue verbs refuse a
/// queue stored in a version this binary does not know. Version 2 adds
/// the pins an operator re-pins an entry to (#430's realm-drift ruling).
pub(crate) const QUEUE_SCHEMA: u32 = 2;

/// The first queue storage, which held no pins: still read as it stands,
/// and brought to [`QUEUE_SCHEMA`] by the first open that may write.
pub(crate) const QUEUE_SCHEMA_V1: u32 = 1;

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

/// Version 2's addition: the launch an entry was re-pinned to, one row
/// per `repin` command and keyed by it. An entry's launch is fixed where
/// it was queued, so a re-pin is written beside it, never over it; the
/// latest pin is the launch the entry stands for, and every earlier one
/// stays, as the commands do.
const MIGRATION_QUEUE_V2: &str = r#"
CREATE TABLE IF NOT EXISTS queue_pins (
    seq INTEGER PRIMARY KEY REFERENCES queue_commands(seq),
    entry_id INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    payload TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS queue_pins_append_only_update
    BEFORE UPDATE ON queue_pins
    BEGIN SELECT RAISE(ABORT, 'queue pins are append-only'); END;
CREATE TRIGGER IF NOT EXISTS queue_pins_append_only_delete
    BEFORE DELETE ON queue_pins
    BEGIN SELECT RAISE(ABORT, 'queue pins are append-only'); END;
"#;

/// The guards the migrations install, by name, so an open can ask
/// whether a journal still carries every one.
const QUEUE_TRIGGERS: [&str; 9] = [
    "queue_entries_fixed",
    "queue_entries_run_once",
    "queue_entries_kept",
    "queue_waits_fixed",
    "queue_waits_kept",
    "queue_commands_append_only_update",
    "queue_commands_append_only_delete",
    "queue_pins_append_only_update",
    "queue_pins_append_only_delete",
];

/// The queue's tables, in the order the migrations create them: version
/// 1's three, then version 2's pins.
const QUEUE_TABLES: [&str; 4] = [
    "queue_entries",
    "queue_waits",
    "queue_commands",
    "queue_pins",
];

/// Is the queue whole: stored in this binary's version and every guard
/// in place? Pure reads, so the steady-state open takes no write lock.
///
/// The recorded version is read first, before anything is repaired, and
/// a queue stored in a version this binary does not know is left exactly
/// as it was found: its guards are another binary's, and only the queue
/// verbs refuse it. A journal that records no queue is from before it,
/// and a version 1 queue is from before its pins; both are migrated. One
/// that records a queue whose table is gone has lost what the queue held
/// (SQLite drops a table's guards with it), and is refused rather than
/// recreated empty; so only a guard missing from a table that is still
/// there is ever repaired.
pub(crate) fn queue_intact(conn: &Connection) -> Result<bool, StoreError> {
    let tables = match queue_schema(conn)? {
        None => return Ok(false),
        Some(QUEUE_SCHEMA_V1) => &QUEUE_TABLES[..QUEUE_TABLES.len() - 1],
        Some(QUEUE_SCHEMA) => &QUEUE_TABLES[..],
        Some(_) => return Ok(true),
    };
    let named = |kind: &str| -> Result<Vec<String>, StoreError> {
        Ok(conn
            .prepare("SELECT name FROM sqlite_master WHERE type = ?1")?
            .query_map([kind], |row| row.get(0))?
            .collect::<Result<_, _>>()?)
    };
    let present = named("table")?;
    if let Some(table) = tables
        .iter()
        .copied()
        .find(|table| !present.iter().any(|name| name == table))
    {
        return Err(QueueRefusal::TableLost { table }.into());
    }
    // A version 1 queue has no pins, and so not every guard: migrated.
    let triggers = named("trigger")?;
    Ok(QUEUE_TRIGGERS
        .iter()
        .all(|guard| triggers.iter().any(|name| name == guard)))
}

/// Create the queue's tables and guards and record its version, on a
/// connection already holding the write lock. A version 1 queue is moved
/// to this version; a version this binary does not know is not.
pub(crate) fn migrate_queue(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(MIGRATION_QUEUE_V1)?;
    conn.execute_batch(MIGRATION_QUEUE_V2)?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('queue_schema', ?1)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value WHERE value = ?2",
        params![QUEUE_SCHEMA.to_string(), QUEUE_SCHEMA_V1.to_string()],
    )?;
    Ok(())
}

/// The queue storage a journal records, or `None` for one that never held
/// a queue: a journal opened read-only from before the queue existed.
pub(crate) fn queue_schema(conn: &Connection) -> Result<Option<u32>, StoreError> {
    let recorded: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'queue_schema'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(recorded.map(|value| value.parse().unwrap_or(0)))
}

/// The queue storage a journal holds, refused when it is a version this
/// binary does not know, or `None` when there is no queue at all.
pub(crate) fn queue_stored(conn: &Connection) -> Result<Option<u32>, StoreError> {
    match queue_schema(conn)? {
        None => Ok(None),
        Some(known @ (QUEUE_SCHEMA_V1 | QUEUE_SCHEMA)) => Ok(Some(known)),
        Some(found) => Err(QueueRefusal::SchemaMismatch { found }.into()),
    }
}
