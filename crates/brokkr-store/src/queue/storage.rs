//! Where the queue is kept (decision 0068 ruling 1): its tables and
//! guards, the storage version a journal records for them, and the
//! migration that brings a journal to that version.

use rusqlite::{params, Connection, OptionalExtension};

use super::{CooldownDuplicate, EntryId, Latch, QueueRefusal, Wait};
use crate::schema::{all_present, holds, objects};
use crate::seat_tree::sweep::{refuse_sweep_duplicates, TREE_SWEEP_INDEX, TREE_SWEEP_TABLE};
use crate::seat_tree::{refuse_duplicates, SEAT_TREE_INDEX, SEAT_TREE_TABLE};
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
/// (decision 0095 rulings 1, 2 and 7). Version 4 adds the
/// `provider_cooldowns` table a rate-limited attempt's refused sites are
/// recorded in (decision 0103 ruling 2(a), slice 4d1). Version 5 adds the
/// `seat_trees` table a run-local seat scratch tree is recorded in before
/// it is made (decision 0104 ruling 6, slice 5b). Version 6 adds the
/// `tree_sweeps` table a complete removal is recorded in (decision 0104
/// ruling 8, slice 5c1). Version 7 adds `provider_cooldowns`' two nullable
/// columns, `held` and `max_seconds`, so a held wait's row carries its own
/// kind (decision 0106 ruling 2, slice 0106b1), and the `probe_holds` table
/// a probe in flight holds its route by (fix round 0106b1r4).
pub(crate) const QUEUE_SCHEMA: u32 = 7;

/// The `meta` key a journal records [`QUEUE_SCHEMA`] under: one home for
/// the spelling, read by this module and by a backup's missing-schema
/// refusal.
pub(crate) const QUEUE_SCHEMA_KEY: &str = "queue_schema";

/// The first queue storage, which held no pins: still read as it stands,
/// and brought to [`QUEUE_SCHEMA`] by the first open that may write.
pub(crate) const QUEUE_SCHEMA_V1: u32 = 1;

/// The second, which held no claims: read and migrated as the first is.
pub(crate) const QUEUE_SCHEMA_V2: u32 = 2;

/// The third, which held no provider cool-downs: read and migrated as the
/// second is, and, as slice 3a's build wrote it, perhaps no dispatchers or
/// dispatch settings either ([`QUEUE_TABLES_UNHELD`]).
pub(crate) const QUEUE_SCHEMA_V3: u32 = 3;

/// The fourth, which held no seat trees: read and migrated as the third
/// is.
pub(crate) const QUEUE_SCHEMA_V4: u32 = 4;

/// The fifth, which held no tree sweeps: read and migrated as the fourth
/// is.
pub(crate) const QUEUE_SCHEMA_V5: u32 = 5;

/// The sixth, whose cool-downs held no kind: read and migrated as the
/// fifth is, its every row an ordinary wait.
pub(crate) const QUEUE_SCHEMA_V6: u32 = 6;

/// A queue storage version this brokkr reads, matched by version wherever
/// what it holds differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stored {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
}

impl Stored {
    /// The version `found` names, or `None` for one this brokkr does not
    /// know.
    fn of(found: u32) -> Option<Stored> {
        match found {
            QUEUE_SCHEMA_V1 => Some(Stored::V1),
            QUEUE_SCHEMA_V2 => Some(Stored::V2),
            QUEUE_SCHEMA_V3 => Some(Stored::V3),
            QUEUE_SCHEMA_V4 => Some(Stored::V4),
            QUEUE_SCHEMA_V5 => Some(Stored::V5),
            QUEUE_SCHEMA_V6 => Some(Stored::V6),
            QUEUE_SCHEMA => Some(Stored::V7),
            _ => None,
        }
    }

    /// How many of [`QUEUE_TABLES`] this version holds. Version 7 adds
    /// cool-down columns and the probe holds' table.
    fn tables(self) -> usize {
        match self {
            Stored::V1 => 3,
            Stored::V2 => 5,
            Stored::V3 => QUEUE_TABLES_V3,
            Stored::V4 => QUEUE_TABLES_V4,
            Stored::V5 => QUEUE_TABLES_V5,
            Stored::V6 => QUEUE_TABLES_V6,
            Stored::V7 => QUEUE_TABLES.len(),
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
/// Slice 3a's build wrote version 3 without the holders and the settings,
/// and each slice lands alone, so a journal may record version 3 and hold
/// neither: [`queue_intact`] finds it unguarded, and this batch, run
/// again under the write lock, adds both, `IF NOT EXISTS` keeping the rest.
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

/// Version 4's addition (decision 0103 ruling 2(a), slice 4d1): one
/// append-only row per refused site and round of an attempt's episode,
/// written in the terminal event's own transaction. A row's key is its
/// rising `seq`; the unique index holds one row per site and round of an
/// episode, and the run is part of it because `events` is keyed by
/// `(run_id, seq)`, so two runs at the same `event_seq` never collide.
/// `until_at` and `recorded_at` are Unix seconds (UTC), never text: a text
/// time compared as a string is wrong across precisions and offsets, and
/// a `unixepoch(text)` CHECK passes on malformed text because the
/// function returns NULL and a NULL CHECK passes. The reader converts
/// them. An expired row is history and is never deleted.
const MIGRATION_QUEUE_V4: &str = r#"
CREATE TABLE IF NOT EXISTS provider_cooldowns (
    seq INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    event_seq INTEGER NOT NULL,
    site TEXT NOT NULL,
    provider TEXT NOT NULL CONSTRAINT provider_named CHECK (length(provider) > 0),
    route TEXT NOT NULL CONSTRAINT route_named CHECK (length(route) > 0),
    until_at INTEGER NOT NULL CONSTRAINT until_is_seconds CHECK (typeof(until_at) = 'integer'),
    reason TEXT NOT NULL CONSTRAINT reason_named CHECK (length(reason) > 0),
    retry_after_seconds INTEGER CONSTRAINT retry_after_positive
        CHECK (retry_after_seconds IS NULL
            OR (typeof(retry_after_seconds) = 'integer' AND retry_after_seconds > 0)),
    rounds INTEGER NOT NULL CONSTRAINT rounds_whole
        CHECK (typeof(rounds) = 'integer' AND rounds >= 1),
    recorded_at INTEGER NOT NULL CONSTRAINT recorded_is_seconds
        CHECK (typeof(recorded_at) = 'integer'),
    CONSTRAINT until_after_record CHECK (until_at > recorded_at),
    FOREIGN KEY (run_id, event_seq) REFERENCES events(run_id, seq)
);
CREATE UNIQUE INDEX IF NOT EXISTS provider_cooldowns_once
    ON provider_cooldowns (run_id, event_seq, site, rounds);
CREATE TRIGGER IF NOT EXISTS provider_cooldowns_append_only_update
    BEFORE UPDATE ON provider_cooldowns
    BEGIN SELECT RAISE(ABORT, 'provider cool-downs are append-only'); END;
CREATE TRIGGER IF NOT EXISTS provider_cooldowns_append_only_delete
    BEFORE DELETE ON provider_cooldowns
    BEGIN SELECT RAISE(ABORT, 'provider cool-downs are append-only'); END;
"#;

/// Version 5's addition (decision 0104 ruling 6, slice 5b): one append-only
/// row per run-local seat scratch tree, written before the tree is made.
/// The tree is always `root/attempt_id` and is never stored apart; `root`
/// is absolute and ends in the component `scratch`, as `SEAT_SCRATCH` names
/// it, and `attempt_id` is exactly the lower-case UUID the engine mints, so
/// it is one path component and never `..`. `recorded_at` is Unix seconds
/// (UTC), never text, for the reason version 4's is.
const MIGRATION_QUEUE_V5: &str = r#"
CREATE TABLE IF NOT EXISTS seat_trees (
    seq INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    effect_id TEXT NOT NULL CONSTRAINT effect_named CHECK (length(effect_id) > 0),
    attempt_id TEXT NOT NULL CONSTRAINT attempt_is_uuid CHECK (
        attempt_id GLOB '[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f]-[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'
        AND length(CAST(attempt_id AS BLOB)) = 36
    ),
    root TEXT NOT NULL CONSTRAINT root_absolute CHECK (substr(root, 1, 1) = '/')
        CONSTRAINT root_is_scratch CHECK (substr(root, -8) = '/scratch')
        CONSTRAINT root_has_no_nul CHECK (instr(CAST(root AS BLOB), x'00') = 0),
    recorded_at INTEGER NOT NULL CONSTRAINT recorded_is_seconds
        CHECK (typeof(recorded_at) = 'integer')
);
CREATE UNIQUE INDEX IF NOT EXISTS seat_trees_once ON seat_trees (run_id, attempt_id);
CREATE TRIGGER IF NOT EXISTS seat_trees_append_only_update
    BEFORE UPDATE ON seat_trees
    BEGIN SELECT RAISE(ABORT, 'seat trees are append-only'); END;
CREATE TRIGGER IF NOT EXISTS seat_trees_append_only_delete
    BEFORE DELETE ON seat_trees
    BEGIN SELECT RAISE(ABORT, 'seat trees are append-only'); END;
"#;

/// Version 6's addition (decision 0104 ruling 8, slice 5c1): one append-only
/// row per complete removal of a tree Brokkr made. A scratch row names the
/// `seat_trees` row it closes; a hands row names no seat tree, because a
/// hands tree belongs to a process and not to an effect. `bytes` is the
/// space the walk freed, and NULL when the tree was gone before the pass;
/// `removed_at` and `generation` are the integers the table records, the
/// latter the dispatch hold the pass took, which every pass takes. The path
/// is absolute and holds no NUL, as version 5's root is.
const MIGRATION_QUEUE_V6: &str = r#"
CREATE TABLE IF NOT EXISTS tree_sweeps (
    seq INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CONSTRAINT sweep_kind_named
        CHECK (kind IN ('scratch', 'hands')),
    tree_seq INTEGER REFERENCES seat_trees(seq)
        CONSTRAINT sweep_tree_matches_kind CHECK (
            (kind = 'scratch' AND tree_seq IS NOT NULL)
            OR (kind = 'hands' AND tree_seq IS NULL)),
    path TEXT NOT NULL
        CONSTRAINT sweep_path_absolute CHECK (substr(path, 1, 1) = '/')
        CONSTRAINT sweep_path_has_no_nul CHECK (instr(CAST(path AS BLOB), x'00') = 0),
    bytes INTEGER CONSTRAINT sweep_bytes_whole
        CHECK (bytes IS NULL OR (typeof(bytes) = 'integer' AND bytes >= 0)),
    removed_at INTEGER NOT NULL CONSTRAINT sweep_removed_is_seconds
        CHECK (typeof(removed_at) = 'integer'),
    generation INTEGER NOT NULL REFERENCES dispatch_holders(generation)
);
CREATE UNIQUE INDEX IF NOT EXISTS tree_sweeps_once ON tree_sweeps (tree_seq);
CREATE TRIGGER IF NOT EXISTS tree_sweeps_append_only_update
    BEFORE UPDATE ON tree_sweeps
    BEGIN SELECT RAISE(ABORT, 'tree sweeps are append-only'); END;
CREATE TRIGGER IF NOT EXISTS tree_sweeps_append_only_delete
    BEFORE DELETE ON tree_sweeps
    BEGIN SELECT RAISE(ABORT, 'tree sweeps are append-only'); END;
"#;

/// Version 7's additions (decision 0106 ruling 2, slice 0106b1): a cool-down
/// row's kind, beside version 4's columns. `max_seconds` is the declared
/// maximum a held wait's provider-stated end exceeded, and `held` is `1`
/// exactly where it is present; both are NULL on an ordinary wait's row and
/// on every row an older version wrote. A held row's `until_at` may lie past
/// `max_seconds` after `recorded_at`. SQLite has no `ADD COLUMN IF NOT
/// EXISTS`, so presence is asked rather than an error swallowed, and
/// `max_seconds` comes first because `held`'s CHECK names it.
const MIGRATION_QUEUE_V7: [(&str, &str); 2] = [
    (
        "max_seconds",
        "ALTER TABLE provider_cooldowns ADD COLUMN max_seconds INTEGER
            CONSTRAINT max_positive CHECK (max_seconds IS NULL
                OR (typeof(max_seconds) = 'integer' AND max_seconds > 0))",
    ),
    (
        "held",
        "ALTER TABLE provider_cooldowns ADD COLUMN held INTEGER
            CONSTRAINT held_names_its_maximum CHECK ((held IS NULL AND max_seconds IS NULL)
                OR (held IS 1 AND max_seconds IS NOT NULL))",
    ),
];

/// Version 7's probe holds (fix round 0106b1r4, F1; the operator's ruling
/// of 2026-11-18 amending decision 0103 ruling 2): one row per probe of a
/// cooled route, written before the probe is reserved, naming the run and
/// the probe's own attempt, the driver instance that holds it (fix round
/// 0106b1r12, S3). While it stands, unended, the pass and the claim read
/// its route as cooling down through the one in-force read. A refusal ends
/// it in the transaction that writes the renewed round, an answer or the
/// settlement of its probe's process tree ends it by its row or its
/// attempt, and nothing else does: `deadline_at` is a readout (S1). Its
/// instants are Unix seconds (UTC), as version 4's are. A hold's attempt,
/// route and deadline are fixed, it ends once, and it is never deleted.
const MIGRATION_QUEUE_V7_HOLDS: &str = r#"
CREATE TABLE IF NOT EXISTS probe_holds (
    seq INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    attempt_id TEXT NOT NULL CONSTRAINT hold_attempt_named CHECK (length(attempt_id) > 0),
    site TEXT NOT NULL,
    provider TEXT NOT NULL CONSTRAINT hold_provider_named CHECK (length(provider) > 0),
    route TEXT NOT NULL CONSTRAINT hold_route_named CHECK (length(route) > 0),
    recorded_at INTEGER NOT NULL CONSTRAINT hold_recorded_is_seconds
        CHECK (typeof(recorded_at) = 'integer'),
    deadline_at INTEGER NOT NULL CONSTRAINT hold_deadline_is_seconds
        CHECK (typeof(deadline_at) = 'integer'),
    ended_at INTEGER CONSTRAINT hold_ended_is_seconds
        CHECK (ended_at IS NULL OR typeof(ended_at) = 'integer')
);
CREATE TRIGGER IF NOT EXISTS probe_holds_fixed
    BEFORE UPDATE OF seq, run_id, attempt_id, site, provider, route, recorded_at, deadline_at
    ON probe_holds
    BEGIN SELECT RAISE(ABORT, 'a probe hold''s attempt, route and deadline are fixed'); END;
CREATE TRIGGER IF NOT EXISTS probe_holds_end_once
    BEFORE UPDATE OF ended_at ON probe_holds
    WHEN OLD.ended_at IS NOT NULL
    BEGIN SELECT RAISE(ABORT, 'a probe hold ends once'); END;
CREATE TRIGGER IF NOT EXISTS probe_holds_kept
    BEFORE DELETE ON probe_holds
    BEGIN SELECT RAISE(ABORT, 'probe holds are never deleted'); END;
"#;

/// Add version 7's columns that `provider_cooldowns` does not declare yet.
fn add_cool_down_kinds(conn: &Connection) -> Result<(), StoreError> {
    for (column, statement) in MIGRATION_QUEUE_V7 {
        if !holds(conn, COOLDOWN_TABLE, [column])? {
            conn.execute(statement, [])?;
        }
    }
    Ok(())
}

/// Does `provider_cooldowns` declare every version 7 column? A table that
/// is absent declares none.
fn cool_down_kinds_present(conn: &Connection) -> Result<bool, StoreError> {
    holds(
        conn,
        COOLDOWN_TABLE,
        MIGRATION_QUEUE_V7.iter().map(|(column, _)| *column),
    )
}

/// The guards the migrations install, by name, so an open can ask
/// whether a journal still carries every one.
const QUEUE_TRIGGERS: [&str; 30] = [
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
    "provider_cooldowns_append_only_update",
    "provider_cooldowns_append_only_delete",
    "seat_trees_append_only_update",
    "seat_trees_append_only_delete",
    "tree_sweeps_append_only_update",
    "tree_sweeps_append_only_delete",
    "probe_holds_fixed",
    "probe_holds_end_once",
    "probe_holds_kept",
];

/// The version 4 table and the unique index on it, by name beyond the
/// migration's DDL: the intact check asks for both, and a lost index over
/// rows that already hold a duplicate is refused by its key.
const COOLDOWN_TABLE: &str = "provider_cooldowns";
const COOLDOWN_INDEX: &str = "provider_cooldowns_once";

/// The indexes the migrations install, by name, so an open can ask whether
/// a journal still carries every one. A table's guards go with it when it
/// is dropped, and so does its unique index, which no trigger can stand in
/// for: without it two rows of one site and round both commit.
const QUEUE_INDEXES: [&str; 3] = [COOLDOWN_INDEX, SEAT_TREE_INDEX, TREE_SWEEP_INDEX];

/// The queue's tables, in the order the migrations create them: version
/// 1's three, then version 2's latches and pins, then version 3's claims,
/// refusals, holders and settings, then version 4's provider cool-downs,
/// then version 5's seat trees, then version 6's tree sweeps, then version
/// 7's probe holds.
const QUEUE_TABLES: [&str; 13] = [
    "queue_entries",
    "queue_waits",
    "queue_commands",
    "queue_latches",
    "queue_pins",
    "queue_claims",
    "queue_refusals",
    "dispatch_holders",
    "dispatch_settings",
    COOLDOWN_TABLE,
    SEAT_TREE_TABLE,
    TREE_SWEEP_TABLE,
    PROBE_HOLD_TABLE,
];

/// Version 7's probe holds, by name: the in-force read joins it.
pub(super) const PROBE_HOLD_TABLE: &str = "probe_holds";

/// How many of [`QUEUE_TABLES`] version 3 held: all but the version 4 and
/// version 5 tables. Slice 3a's build wrote version 3 without the holders
/// and the settings too (decision 0095 slice 3b), so a version 3 journal
/// may hold [`QUEUE_TABLES_UNHELD`] of them.
const QUEUE_TABLES_V3: usize = 9;

/// How many of [`QUEUE_TABLES`] version 4 held: all but the version 5 and
/// version 6 tables.
const QUEUE_TABLES_V4: usize = 10;

/// How many of [`QUEUE_TABLES`] version 5 held: all but the version 6 and
/// version 7 tables.
const QUEUE_TABLES_V5: usize = 11;

/// How many of [`QUEUE_TABLES`] version 6 held: all but the version 7
/// table.
const QUEUE_TABLES_V6: usize = 12;

/// How many of [`QUEUE_TABLES`] version 3 held as slice 3a's build wrote
/// it: all but the holders and the settings (decision 0095 slice 3b).
const QUEUE_TABLES_UNHELD: usize = 7;

/// Is the queue whole: stored in this binary's version, every guard in
/// place and every index? Pure reads, so the steady-state open takes no
/// write lock.
///
/// The recorded version is read first, before anything is repaired, and
/// a queue stored in a version this binary does not know is left exactly
/// as it was found: its guards are another binary's, and only the queue
/// verbs refuse it. A journal that records no queue is from before it, a
/// version 1 queue is from before its pins and a version 2 queue from
/// before its claims; each is migrated. One that records a queue whose
/// table is gone has lost what the queue held (SQLite drops a table's
/// guards with it), and is refused rather than recreated empty; so only a
/// guard missing from a table that is still there is ever repaired. The
/// one exception is version 3 as slice 3a wrote it, holding neither the
/// holders nor the settings: it gains both, empty, as it never held a row
/// of either; one of the two without the other was lost, and is refused.
pub(crate) fn queue_intact(conn: &Connection) -> Result<bool, StoreError> {
    let Some(found) = queue_schema(conn)? else {
        return Ok(false);
    };
    let Some(stored) = Stored::of(found) else {
        return Ok(true);
    };
    let present = objects(conn, "table")?;
    // The version 3-era held tables, not the version 4 one: a version 3
    // journal as slice 3a wrote it holds neither the holders nor the
    // settings, and may already hold the cool-down table a migration got
    // as far as creating.
    let held = QUEUE_TABLES[QUEUE_TABLES_UNHELD..QUEUE_TABLES_V3]
        .iter()
        .any(|table| present.iter().any(|name| name == table));
    let tables = &QUEUE_TABLES[..stored.tables()];
    // Holding neither, a version 3 journal holds none of their guards
    // either, so the version 3 batch runs again; any other recorded
    // version that lost a table lost what the queue held.
    let tables = match (stored, held) {
        (Stored::V3, false) => &tables[..QUEUE_TABLES_UNHELD],
        _ => tables,
    };
    if let Some(table) = tables
        .iter()
        .copied()
        .find(|table| !present.iter().any(|name| name == table))
    {
        return Err(QueueRefusal::TableLost { table }.into());
    }
    // An older queue has not every guard or index, and a version 6 queue's
    // cool-downs have no kind: migrated. The presence question is
    // `schema::all_present`'s, as it is for every other repair.
    Ok(all_present(conn, "trigger", &QUEUE_TRIGGERS)?
        && all_present(conn, "index", &QUEUE_INDEXES)?
        && cool_down_kinds_present(conn)?)
}

/// Create the queue's tables and guards and record its version, on a
/// connection already holding the write lock. A version 1 to 6 queue is
/// moved to this version; a version this binary does not know is not.
pub(crate) fn migrate_queue(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(MIGRATION_QUEUE_V1)?;
    conn.execute_batch(MIGRATION_QUEUE_V2)?;
    conn.execute_batch(MIGRATION_QUEUE_V3)?;
    refuse_cool_down_duplicates(conn)?;
    conn.execute_batch(MIGRATION_QUEUE_V4)?;
    refuse_duplicates(conn)?;
    conn.execute_batch(MIGRATION_QUEUE_V5)?;
    refuse_sweep_duplicates(conn)?;
    conn.execute_batch(MIGRATION_QUEUE_V6)?;
    add_cool_down_kinds(conn)?;
    conn.execute_batch(MIGRATION_QUEUE_V7_HOLDS)?;
    record_queue_version(conn)
}

/// The statement that records this binary's queue version, over every older
/// version it replaces.
const RECORD_QUEUE_VERSION: &str = "INSERT INTO meta (key, value) VALUES (?1, ?2)
     ON CONFLICT (key) DO UPDATE SET value = excluded.value
     WHERE value IN (?3, ?4, ?5, ?6, ?7, ?8)";

/// Record [`QUEUE_SCHEMA`] over a version 1 to 6 queue, and leave any
/// version this binary does not know as it was found.
fn record_queue_version(conn: &Connection) -> Result<(), StoreError> {
    let versions = [
        QUEUE_SCHEMA_V1.to_string(),
        QUEUE_SCHEMA_V2.to_string(),
        QUEUE_SCHEMA_V3.to_string(),
        QUEUE_SCHEMA_V4.to_string(),
        QUEUE_SCHEMA_V5.to_string(),
        QUEUE_SCHEMA_V6.to_string(),
    ];
    let current = QUEUE_SCHEMA.to_string();
    let values: [&dyn rusqlite::ToSql; 8] = [
        &QUEUE_SCHEMA_KEY,
        &current,
        &versions[0],
        &versions[1],
        &versions[2],
        &versions[3],
        &versions[4],
        &versions[5],
    ];
    conn.execute(RECORD_QUEUE_VERSION, values)?;
    Ok(())
}

/// Refuse to rebuild [`COOLDOWN_INDEX`] over rows that already hold a
/// duplicate of its key: the index can be put back only where every key is
/// still unique, and a journal that lost it and gained a second row is
/// named by that key rather than failing on SQLite's own constraint text.
/// A table that is absent is not yet a duplicate, and an index that is
/// present already refuses one, so neither is read.
fn refuse_cool_down_duplicates(conn: &Connection) -> Result<(), StoreError> {
    if !objects(conn, "table")?
        .iter()
        .any(|name| name == COOLDOWN_TABLE)
        || objects(conn, "index")?
            .iter()
            .any(|name| name == COOLDOWN_INDEX)
    {
        return Ok(());
    }
    let duplicate: Option<(String, i64, String, i64)> = conn
        .query_row(
            "SELECT run_id, event_seq, site, rounds FROM provider_cooldowns
             GROUP BY run_id, event_seq, site, rounds HAVING COUNT(*) > 1
             ORDER BY run_id, event_seq, site, rounds LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let Some((run_id, event_seq, site, rounds)) = duplicate else {
        return Ok(());
    };
    Err(QueueRefusal::CooldownDuplicate(Box::new(CooldownDuplicate {
        run_id,
        event_seq,
        site,
        rounds,
    }))
    .into())
}

/// The queue storage a journal records, or `None` for one that never held
/// a queue: a journal opened read-only from before the queue existed.
pub(crate) fn queue_schema(conn: &Connection) -> Result<Option<u32>, StoreError> {
    let recorded: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![QUEUE_SCHEMA_KEY],
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

/// Does the queue hold the cool-down table a reader may read? A version 1,
/// 2 or 3 queue is from before the table, and no queue at all holds none;
/// a version this binary does not know refuses, because only its own
/// binary can say what it holds.
pub(super) fn cool_downs_held(conn: &Connection) -> Result<bool, StoreError> {
    match queue_stored(conn)? {
        Some(Stored::V4 | Stored::V5 | Stored::V6 | Stored::V7) => Ok(true),
        Some(Stored::V1 | Stored::V2 | Stored::V3) | None => Ok(false),
    }
}

/// Does the queue hold the cool-down table a rows-bearing append writes?
/// Version 7 is the only one this binary writes. A version 1, 2 or 3
/// queue is from before the table, a version 4 to 6 queue is older than this
/// binary's version (and only an open that may write migrates it), no
/// queue at all holds none, and a version this binary does not know is a
/// newer binary's queue, left as found: in each case only the event lands.
pub(super) fn cool_downs_writable(conn: &Connection) -> Result<bool, StoreError> {
    Ok(queue_schema(conn)? == Some(QUEUE_SCHEMA))
}
