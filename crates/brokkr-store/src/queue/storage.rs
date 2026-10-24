//! Where the queue is kept (decision 0068 ruling 1): its tables and
//! guards, the storage version a journal records for them, and the
//! migration that brings a journal to that version.

use rusqlite::{params, Connection, OptionalExtension};

use super::{EntryId, Latch, QueueRefusal, Wait};
use crate::StoreError;

/// The launch an entry stands for: the one it was last re-pinned to,
/// else the one it was queued with. A version 1 queue holds no pins.
pub(super) const PINNED_PAYLOAD: &str = "COALESCE((SELECT pin.payload FROM queue_pins AS pin
    WHERE pin.entry_id = queue_entries.entry_id ORDER BY pin.seq DESC LIMIT 1), payload)";

/// The seq of that pin, read in the same statement as [`PINNED_PAYLOAD`];
/// `NULL` while the entry stands for the launch it was queued with.
pub(super) const PINNED_SEQ: &str = "(SELECT MAX(pin.seq) FROM queue_pins AS pin
    WHERE pin.entry_id = queue_entries.entry_id)";

/// The seq of the pin an entry stands for now, as [`PINNED_SEQ`] reads it.
pub(super) const STANDING_PIN: &str = "SELECT MAX(seq) FROM queue_pins WHERE entry_id = ?1";

/// The latch that stands on an entry: its latest, unless a re-pin came
/// after it. [`latch_row`] reads its row.
pub(super) const STANDING_LATCH: &str =
    "SELECT latch.seq, latch.finding FROM queue_latches AS latch
    WHERE latch.entry_id = ?1 AND NOT EXISTS (SELECT 1 FROM queue_pins AS pin
        WHERE pin.entry_id = ?1 AND pin.seq > latch.seq)
    ORDER BY latch.seq DESC LIMIT 1";

pub(super) fn latch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Latch> {
    Ok(Latch {
        seq: row.get(0)?,
        finding: row.get(1)?,
    })
}

/// An entry's waits, by the entry each awaits. [`wait_row`] reads a row.
pub(super) const WAITS: &str =
    "SELECT awaits, condition FROM queue_waits WHERE entry_id = ?1 ORDER BY awaits";

pub(super) fn wait_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Wait> {
    Ok(Wait {
        entry: EntryId(row.get(0)?),
        on: row.get(1)?,
    })
}

/// The queue storage a journal records under `meta.queue_schema`, apart
/// from `database_schema`: the queue is additive, so an older binary still
/// reads a journal that carries one, and only the queue verbs refuse a
/// queue stored in a version this binary does not know. Version 2 adds
/// the realm-drift holds admission latches on an entry and the pins an
/// operator releases one with (#430's realm-drift ruling). Version 3 adds
/// the `launching` state, the dispatcher's claims, the refusals counted
/// against an entry, the journal's dispatchers and its dispatch settings
/// (decision 0095 rulings 1, 2 and 7).
pub(crate) const QUEUE_SCHEMA: u32 = 3;

/// The first queue storage, which held no pins: still read as it stands,
/// and brought to [`QUEUE_SCHEMA`] by the first open that may write.
pub(crate) const QUEUE_SCHEMA_V1: u32 = 1;

/// The second, which held no claims: read and migrated as the first is.
pub(crate) const QUEUE_SCHEMA_V2: u32 = 2;

/// A queue storage version this brokkr reads, matched by version wherever
/// what it holds differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stored {
    V1,
    V2,
    V3,
}

impl Stored {
    /// The version `found` names, or `None` for one this brokkr does not
    /// know.
    fn of(found: u32) -> Option<Stored> {
        match found {
            QUEUE_SCHEMA_V1 => Some(Stored::V1),
            QUEUE_SCHEMA_V2 => Some(Stored::V2),
            QUEUE_SCHEMA => Some(Stored::V3),
            _ => None,
        }
    }

    /// How many of [`QUEUE_TABLES`] this version holds.
    fn tables(self) -> usize {
        match self {
            Stored::V1 => 3,
            Stored::V2 => 5,
            Stored::V3 => QUEUE_TABLES.len(),
        }
    }
}

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

/// Version 2's additions. A latch: a realm-drift hold admission found on
/// an entry, one row per `latch` command and keyed by it, which stands
/// until a later re-pin of the entry. A pin: the launch an entry was
/// re-pinned to, one row per `repin` command and keyed by it. An entry's
/// launch is fixed where it was queued, so a re-pin is written beside it,
/// never over it; the latest pin is the launch the entry stands for, and
/// every earlier one stays, as the commands and the latches do.
pub(super) const MIGRATION_QUEUE_V2: &str = r#"
CREATE TABLE IF NOT EXISTS queue_latches (
    seq INTEGER PRIMARY KEY REFERENCES queue_commands(seq),
    entry_id INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    finding TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS queue_latches_append_only_update
    BEFORE UPDATE ON queue_latches
    BEGIN SELECT RAISE(ABORT, 'queue latches are append-only'); END;
CREATE TRIGGER IF NOT EXISTS queue_latches_append_only_delete
    BEFORE DELETE ON queue_latches
    BEGIN SELECT RAISE(ABORT, 'queue latches are append-only'); END;
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

/// Version 3's additions (decision 0095 ruling 1). A claim: the
/// dispatcher's hold on an entry before its run exists, one row per
/// `claim` command and keyed by it, naming the dispatcher's generation and
/// the manifest digest of the bundle its pass admitted, and the run once
/// the claim is linked to one, written once. An entry's run is written
/// only while it stands `launching`, under a claim. A refusal: one row per
/// refused start counted against its entry, keyed by the `reclaim` or
/// `hold` command that disposed of it. A holder: one row per dispatcher
/// that took the journal's lock (ruling 2), its generation rising with
/// each, since none is ever deleted. A setting: one row per command that
/// set the journal's bound on counted refusals (ruling 7), keyed by its
/// seq, the latest in force, and none holding anything but a whole number
/// of at least 1 and at most `u32::MAX`.
///
/// No build that wrote version 3 without the holders and the settings was
/// ever released: slice 3a's version 3 was never merged, so they join
/// version 3's own migration rather than a repair of it.
const MIGRATION_QUEUE_V3: &str = r#"
CREATE TABLE IF NOT EXISTS queue_claims (
    seq INTEGER PRIMARY KEY REFERENCES queue_commands(seq),
    entry_id INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    generation INTEGER NOT NULL,
    digest TEXT NOT NULL,
    run_id TEXT UNIQUE REFERENCES runs(run_id)
);
CREATE TRIGGER IF NOT EXISTS queue_claims_fixed
    BEFORE UPDATE OF seq, entry_id, generation, digest ON queue_claims
    BEGIN SELECT RAISE(ABORT, 'a queue claim''s entry, generation and digest are fixed'); END;
CREATE TRIGGER IF NOT EXISTS queue_claims_run_once
    BEFORE UPDATE OF run_id ON queue_claims
    WHEN OLD.run_id IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'a queue claim''s run is written once'); END;
CREATE TRIGGER IF NOT EXISTS queue_claims_kept
    BEFORE DELETE ON queue_claims
    BEGIN SELECT RAISE(ABORT, 'queue claims are never deleted'); END;
CREATE TRIGGER IF NOT EXISTS queue_entries_run_while_launching
    BEFORE UPDATE OF run_id ON queue_entries
    WHEN OLD.state IS NOT 'launching'
    BEGIN SELECT RAISE(ABORT, 'a queue entry''s run is written only while it is launching'); END;
CREATE TABLE IF NOT EXISTS queue_refusals (
    seq INTEGER PRIMARY KEY REFERENCES queue_commands(seq),
    entry_id INTEGER NOT NULL REFERENCES queue_entries(entry_id),
    refusal TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS queue_refusals_append_only_update
    BEFORE UPDATE ON queue_refusals
    BEGIN SELECT RAISE(ABORT, 'queue refusals are append-only'); END;
CREATE TRIGGER IF NOT EXISTS queue_refusals_append_only_delete
    BEFORE DELETE ON queue_refusals
    BEGIN SELECT RAISE(ABORT, 'queue refusals are append-only'); END;
CREATE TABLE IF NOT EXISTS dispatch_holders (
    generation INTEGER PRIMARY KEY,
    host TEXT,
    pid INTEGER NOT NULL,
    started_at TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS dispatch_holders_append_only_update
    BEFORE UPDATE ON dispatch_holders
    BEGIN SELECT RAISE(ABORT, 'dispatch holders are append-only'); END;
CREATE TRIGGER IF NOT EXISTS dispatch_holders_append_only_delete
    BEFORE DELETE ON dispatch_holders
    BEGIN SELECT RAISE(ABORT, 'dispatch holders are append-only'); END;
CREATE TABLE IF NOT EXISTS dispatch_settings (
    seq INTEGER PRIMARY KEY,
    bound INTEGER NOT NULL
        CHECK (typeof(bound) = 'integer' AND bound BETWEEN 1 AND 4294967295),
    operator TEXT NOT NULL,
    reason TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);
CREATE TRIGGER IF NOT EXISTS dispatch_settings_append_only_update
    BEFORE UPDATE ON dispatch_settings
    BEGIN SELECT RAISE(ABORT, 'dispatch settings are append-only'); END;
CREATE TRIGGER IF NOT EXISTS dispatch_settings_append_only_delete
    BEFORE DELETE ON dispatch_settings
    BEGIN SELECT RAISE(ABORT, 'dispatch settings are append-only'); END;
"#;

/// The guards the migrations install, by name, so an open can ask
/// whether a journal still carries every one.
const QUEUE_TRIGGERS: [&str; 21] = [
    "queue_entries_fixed",
    "queue_entries_run_once",
    "queue_entries_kept",
    "queue_waits_fixed",
    "queue_waits_kept",
    "queue_commands_append_only_update",
    "queue_commands_append_only_delete",
    "queue_latches_append_only_update",
    "queue_latches_append_only_delete",
    "queue_pins_append_only_update",
    "queue_pins_append_only_delete",
    "queue_claims_fixed",
    "queue_claims_run_once",
    "queue_claims_kept",
    "queue_entries_run_while_launching",
    "queue_refusals_append_only_update",
    "queue_refusals_append_only_delete",
    "dispatch_holders_append_only_update",
    "dispatch_holders_append_only_delete",
    "dispatch_settings_append_only_update",
    "dispatch_settings_append_only_delete",
];

/// The queue's tables, in the order the migrations create them: version
/// 1's three, then version 2's latches and pins, then version 3's claims,
/// refusals, holders and settings.
const QUEUE_TABLES: [&str; 9] = [
    "queue_entries",
    "queue_waits",
    "queue_commands",
    "queue_latches",
    "queue_pins",
    "queue_claims",
    "queue_refusals",
    "dispatch_holders",
    "dispatch_settings",
];

/// Is the queue whole: stored in this binary's version and every guard
/// in place? Pure reads, so the steady-state open takes no write lock.
///
/// The recorded version is read first, before anything is repaired, and
/// a queue stored in a version this binary does not know is left exactly
/// as it was found: its guards are another binary's, and only the queue
/// verbs refuse it. A journal that records no queue is from before it, a
/// version 1 queue is from before its pins and a version 2 queue from
/// before its claims; each is migrated. One that records a queue whose
/// table is gone has lost what the queue held (SQLite drops a table's
/// guards with it), and is refused rather than recreated empty; so only a
/// guard missing from a table that is still there is ever repaired.
pub(crate) fn queue_intact(conn: &Connection) -> Result<bool, StoreError> {
    let Some(found) = queue_schema(conn)? else {
        return Ok(false);
    };
    let Some(stored) = Stored::of(found) else {
        return Ok(true);
    };
    let tables = &QUEUE_TABLES[..stored.tables()];
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
    // An older queue has not every guard: migrated.
    let triggers = named("trigger")?;
    Ok(QUEUE_TRIGGERS
        .iter()
        .all(|guard| triggers.iter().any(|name| name == guard)))
}

/// Create the queue's tables and guards and record its version, on a
/// connection already holding the write lock. A version 1 or 2 queue is
/// moved to this version; a version this binary does not know is not.
pub(crate) fn migrate_queue(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(MIGRATION_QUEUE_V1)?;
    conn.execute_batch(MIGRATION_QUEUE_V2)?;
    conn.execute_batch(MIGRATION_QUEUE_V3)?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('queue_schema', ?1)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value WHERE value IN (?2, ?3)",
        params![
            QUEUE_SCHEMA.to_string(),
            QUEUE_SCHEMA_V1.to_string(),
            QUEUE_SCHEMA_V2.to_string()
        ],
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
pub(crate) fn queue_stored(conn: &Connection) -> Result<Option<Stored>, StoreError> {
    match queue_schema(conn)? {
        None => Ok(None),
        Some(found) => Stored::of(found)
            .map(Some)
            .ok_or(QueueRefusal::SchemaMismatch { found }.into()),
    }
}
