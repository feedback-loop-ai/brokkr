//! The queue in the journal (decision 0068 ruling 1): order, holds and
//! run linkage survive a reopen and a migration, every refusal is typed
//! and writes nothing, and every guard bites.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::json;

use super::storage::queue_schema;
use super::*;
use crate::schema::{MIGRATION_V1, SIDECAR_COLUMNS};
use crate::test_support::plant_run;
use crate::DATABASE_SCHEMA;

const BY: Attribution<'static> = Attribution {
    operator: "vy",
    reason: "operator's word",
};

/// An entry's payload, named by its number so a listing reads back.
fn payload(n: u32) -> String {
    format!("launch {n}")
}

/// Queue `count` entries with no waits and priority 0.
fn queue(store: &mut Store, count: u32) -> Vec<EntryId> {
    (1..=count)
        .map(|n| {
            let new = NewEntry {
                payload: &payload(n),
                priority: 0,
                waits: &[],
            };
            store.queue_add(new, BY).unwrap()
        })
        .collect()
}

/// The listing without its arrival stamps: id, place, state, priority,
/// waits and payload.
type Row = (i64, Option<u32>, EntryState, i64, Vec<Wait>, String);

fn rows(store: &Store) -> Vec<Row> {
    store
        .queue_list()
        .unwrap()
        .into_iter()
        .map(|e| (e.id.0, e.position, e.state, e.priority, e.waits, e.payload))
        .collect()
}

/// The operator commands journaled so far: entry, word, place, operator
/// and reason.
fn commands(conn: &Connection) -> Vec<(i64, String, Option<u32>, String, String)> {
    conn.prepare(
        "SELECT entry_id, command, position, operator, reason FROM queue_commands ORDER BY seq",
    )
    .unwrap()
    .query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn refusal(result: Result<impl std::fmt::Debug, StoreError>) -> QueueRefusal {
    match result.unwrap_err() {
        StoreError::Queue(refusal) => refusal,
        other => panic!("not a queue refusal: {other:?}"),
    }
}

/// What a caller reads of `entry` now, to write a latch or re-pin over.
fn seen(store: &Store, entry: EntryId) -> Seen {
    store.queue_entry(entry).unwrap().seen()
}

/// A read of an entry never re-pinned, with latch `seq` standing on it.
fn over(seq: Option<i64>) -> Seen {
    Seen {
        pin: None,
        latch: seq,
    }
}

fn started(store: &mut Store, run: &str) {
    store
        .create_run(
            run,
            "feature",
            "self",
            &json!({"schema": "run-manifest/v1"}),
        )
        .unwrap();
}

/// A journal exactly as main's binary leaves it: the v1 tables, the
/// sidecar columns and the recorded schema, with one run in it.
fn journal_at_mains_schema(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(MIGRATION_V1).unwrap();
    for (_, statement) in SIDECAR_COLUMNS {
        conn.execute(statement, []).unwrap();
    }
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('database_schema', ?1)",
        [&DATABASE_SCHEMA.to_string()],
    )
    .unwrap();
    plant_run(&conn, "old-run", "{}").unwrap();
}

#[test]
fn order_holds_waits_and_run_linkage_survive_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let ids = queue(&mut store, 3);
    let waits = [Wait {
        entry: ids[0],
        on: WaitOn::Completed,
    }];
    let fourth = NewEntry {
        payload: &payload(4),
        priority: 7,
        waits: &waits,
    };
    let fourth = store.queue_add(fourth, BY).unwrap();
    store
        .queue_command(fourth, QueueCommand::Move { to: 1 }, BY)
        .unwrap();
    store.queue_command(ids[1], QueueCommand::Hold, BY).unwrap();
    store.queue_command(ids[2], QueueCommand::Drop, BY).unwrap();
    started(&mut store, "run-a");
    store.queue_claim(ids[0], "run-a").unwrap();
    drop(store);

    let store = Store::open(&path).unwrap();
    let claimed = EntryState::Claimed {
        run: "run-a".into(),
    };
    assert_eq!(
        rows(&store),
        vec![
            (
                4,
                Some(1),
                EntryState::Queued,
                7,
                waits.to_vec(),
                payload(4)
            ),
            (2, Some(2), EntryState::Held, 0, vec![], payload(2)),
            (1, None, claimed, 0, vec![], payload(1)),
        ]
    );
    let word = |n: i64, word: &str, place: Option<u32>| {
        (
            n,
            word.to_string(),
            place,
            "vy".to_string(),
            BY.reason.to_string(),
        )
    };
    assert_eq!(
        commands(&store.conn),
        vec![
            word(1, "add", Some(1)),
            word(2, "add", Some(2)),
            word(3, "add", Some(3)),
            word(4, "add", Some(4)),
            word(4, "move", Some(1)),
            word(2, "hold", None),
            word(3, "drop", None),
        ]
    );
    // Released, the held entry keeps its place; moved last, it takes it.
    let mut store = store;
    store
        .queue_command(ids[1], QueueCommand::Release, BY)
        .unwrap();
    store
        .queue_command(ids[1], QueueCommand::Move { to: 1 }, BY)
        .unwrap();
    let places: Vec<(i64, Option<u32>, EntryState)> =
        rows(&store).into_iter().map(|r| (r.0, r.1, r.2)).collect();
    assert_eq!(
        places[..2],
        [
            (2, Some(1), EntryState::Queued),
            (4, Some(2), EntryState::Queued)
        ]
    );
}

#[test]
fn a_journal_at_mains_schema_migrates_to_the_queue_and_keeps_its_runs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    journal_at_mains_schema(&path);

    // Read as it stands, it has an empty queue and no queue storage.
    assert_eq!(
        Store::open_read_only(&path).unwrap().queue_list().unwrap(),
        vec![]
    );

    let mut store = opened_whole(&path);
    let entry = queue(&mut store, 1)[0];
    store.queue_claim(entry, "old-run").unwrap();
    drop(store);

    let store = Store::open_read_only(&path).unwrap();
    assert_eq!(store.list_runs().unwrap()[0].0, "old-run");
    let claimed = EntryState::Claimed {
        run: "old-run".into(),
    };
    assert_eq!(
        rows(&store),
        vec![(1, None, claimed, 0, vec![], payload(1))]
    );
}

/// Open the journal at `path` to write, and find its queue whole at this
/// binary's version.
fn opened_whole(path: &Path) -> Store {
    let store = Store::open(path).unwrap();
    assert_eq!(queue_schema(&store.conn).unwrap(), Some(QUEUE_SCHEMA));
    assert!(queue_intact(&store.conn).unwrap());
    store
}

/// A new journal in `dir`, with `surgery` done to it.
fn journal_after(dir: &Path, surgery: &str) -> (PathBuf, Store) {
    let path = dir.join("forge.db");
    let store = Store::open(&path).unwrap();
    store.conn.execute_batch(surgery).unwrap();
    (path, store)
}

#[test]
fn a_journal_that_lost_a_queue_guard_gets_it_back_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let (path, store) = journal_after(dir.path(), "DROP TRIGGER queue_entries_run_once");
    assert!(!queue_intact(&store.conn).unwrap());
    drop(store);
    assert!(queue_intact(&Store::open(&path).unwrap().conn).unwrap());
}

/// A journal that records its queue and has lost a queue table is refused
/// at every writable open, and the queue is not recreated empty: what it
/// held, entries, run links and commands, cannot be read.
#[test]
fn a_journal_that_lost_a_queue_table_is_refused_on_open_not_recreated_empty() {
    let dir = tempfile::tempdir().unwrap();
    for (lose, table) in [
        (
            "DROP TABLE queue_waits; DROP TABLE queue_commands; DROP TABLE queue_entries;",
            "queue_entries",
        ),
        ("DROP TABLE queue_waits", "queue_waits"),
        ("DROP TABLE queue_commands", "queue_commands"),
        ("DROP TABLE queue_latches", "queue_latches"),
        ("DROP TABLE queue_pins", "queue_pins"),
    ] {
        let path = dir.path().join(format!("{table}.db"));
        let mut store = Store::open(&path).unwrap();
        queue(&mut store, 1);
        store.conn.execute_batch(lose).unwrap();
        let tables = |conn: &Connection| -> Vec<String> {
            conn.prepare("SELECT name FROM sqlite_master WHERE name LIKE 'queue_%' ORDER BY name")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let left = tables(&store.conn);
        drop(store);
        let refused = match Store::open(&path).err().expect("refused") {
            StoreError::Queue(refused) => refused,
            other => panic!("not a queue refusal: {other}"),
        };
        refused_as(
            refused,
            QueueRefusal::TableLost { table },
            &format!(
                "the journal records a queue but its {table} table is gone; what the queue held \
                 cannot be read, and it is never recreated empty"
            ),
        );
        let conn = Connection::open(&path).unwrap();
        assert_eq!(tables(&conn), left);
        assert_eq!(queue_schema(&conn).unwrap(), Some(QUEUE_SCHEMA));
    }
}

/// A queue with an entry in every standing: 1 queued, 2 held, 3 dropped
/// and 4 claimed by `run-b`, with `run-a` started and claimed by none.
fn every_standing(dir: &Path) -> (Store, [EntryId; 4]) {
    let mut store = Store::open(&dir.join("forge.db")).unwrap();
    let ids = queue(&mut store, 4);
    store.queue_command(ids[1], QueueCommand::Hold, BY).unwrap();
    store.queue_command(ids[2], QueueCommand::Drop, BY).unwrap();
    started(&mut store, "run-a");
    started(&mut store, "run-b");
    store.queue_claim(ids[3], "run-b").unwrap();
    (store, [ids[0], ids[1], ids[2], ids[3]])
}

/// A refusal is the variant expected, saying exactly this.
fn refused_as(refused: QueueRefusal, variant: QueueRefusal, text: &str) {
    assert_eq!((refused.to_string(), refused), (text.to_string(), variant));
}

fn claimed_by_run_b(entry: EntryId) -> QueueRefusal {
    QueueRefusal::Claimed {
        entry,
        run: "run-b".into(),
    }
}

#[test]
fn every_command_refusal_is_typed_says_why_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, [queued, held, dropped, claimed]) = every_standing(dir.path());
    let before = (rows(&store), commands(&store.conn));
    let mut command = |entry, command| refusal(store.queue_command(entry, command, BY));
    let absent = EntryId(99);
    let past = |to| QueueRefusal::PastTheEnds {
        entry: queued,
        to,
        last: 2,
    };
    let text =
        |to| format!("queue entry 1 cannot move to place {to}: the queue's places run 1 to 2");
    refused_as(
        command(absent, QueueCommand::Hold),
        QueueRefusal::UnknownEntry(absent),
        "queue entry 99 does not exist",
    );
    for to in [0, 3] {
        refused_as(
            command(queued, QueueCommand::Move { to }),
            past(to),
            &text(to),
        );
    }
    refused_as(
        command(held, QueueCommand::Hold),
        QueueRefusal::AlreadyHeld(held),
        "queue entry 2 is already held",
    );
    refused_as(
        command(queued, QueueCommand::Release),
        QueueRefusal::NotHeld(queued),
        "queue entry 1 is not held",
    );
    refused_as(
        command(dropped, QueueCommand::Release),
        QueueRefusal::Dropped(dropped),
        "queue entry 3 was dropped",
    );
    refused_as(
        command(claimed, QueueCommand::Drop),
        claimed_by_run_b(claimed),
        "queue entry 4 is claimed by run 'run-b'",
    );
    assert_eq!((rows(&store), commands(&store.conn)), before);
}

#[test]
fn every_add_and_claim_refusal_is_typed_says_why_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, [queued, held, dropped, claimed]) = every_standing(dir.path());
    let before = (rows(&store), commands(&store.conn));
    let absent = EntryId(99);
    let wait = |entry| Wait {
        entry,
        on: WaitOn::Ended,
    };
    let mut add = |waits: &[Wait]| {
        let new = NewEntry {
            payload: "x",
            priority: 0,
            waits,
        };
        refusal(store.queue_add(new, BY))
    };
    refused_as(
        add(&[wait(absent)]),
        QueueRefusal::UnknownWait(absent),
        "a new queue entry cannot wait for entry 99, which does not exist",
    );
    refused_as(
        add(&[wait(queued), wait(queued)]),
        QueueRefusal::DuplicateWait(queued),
        "a new queue entry names entry 1 to wait for twice",
    );
    let mut claim = |entry, run| refusal(store.queue_claim(entry, run));
    refused_as(
        claim(held, "run-a"),
        QueueRefusal::Held(held),
        "queue entry 2 is held; release it first",
    );
    refused_as(
        claim(dropped, "run-a"),
        QueueRefusal::Dropped(dropped),
        "queue entry 3 was dropped",
    );
    refused_as(
        claim(claimed, "run-a"),
        claimed_by_run_b(claimed),
        "queue entry 4 is claimed by run 'run-b'",
    );
    let taken = QueueRefusal::RunTaken {
        run: "run-b".into(),
        entry: claimed,
    };
    refused_as(
        claim(queued, "run-b"),
        taken,
        "run 'run-b' already started queue entry 4",
    );
    let unstarted = store.queue_claim(queued, "never-started").unwrap_err();
    assert!(matches!(&unstarted, StoreError::RunNotFound(run) if run == "never-started"));
    assert_eq!((rows(&store), commands(&store.conn)), before);
}

/// A journal that records the queue's storage but has lost its entries
/// table is not read as an empty queue: the listing fails with SQLite's
/// own words, where a journal from before the queue lists empty.
#[test]
fn a_queue_whose_entries_table_is_gone_fails_to_list() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .conn
        .execute_batch(
            "DROP TABLE queue_waits; DROP TABLE queue_commands; DROP TABLE queue_entries;",
        )
        .unwrap();
    let error = store.queue_list().unwrap_err();
    assert!(matches!(error, StoreError::Sqlite(_)), "{error}");
    assert_eq!(error.to_string(), "sqlite: no such table: queue_entries");
}

#[test]
fn a_queue_stored_in_a_version_this_brokkr_does_not_know_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let entry = queue(&mut store, 1)[0];
    for (recorded, found) in [("3", 3), ("two", 0)] {
        store
            .conn
            .execute(
                "UPDATE meta SET value = ?1 WHERE key = 'queue_schema'",
                [recorded],
            )
            .unwrap();
        let mismatch = QueueRefusal::SchemaMismatch { found };
        let text = format!("queue storage {found} unsupported (want 2)");
        for refused in [
            refusal(store.queue_list()),
            refusal(store.queue_entry(entry)),
            refusal(store.queue_repin(entry, "x", Seen::default(), BY)),
            refusal(store.queue_latch(entry, "x", Seen::default(), BY)),
            refusal(store.queue_command(entry, QueueCommand::Hold, BY)),
            refusal(store.queue_claim(entry, "r")),
            refusal(store.queue_add(
                NewEntry {
                    payload: "x",
                    priority: 0,
                    waits: &[],
                },
                BY,
            )),
        ] {
            assert_eq!((refused.to_string(), &refused), (text.clone(), &mismatch));
        }
    }
}

#[test]
fn a_state_word_this_brokkr_does_not_know_is_refused_not_guessed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let first = queue(&mut store, 2)[0];
    // Planted on the entry nothing names: the queue's order reads it too.
    store
        .conn
        .execute(
            "UPDATE queue_entries SET state = 'paused' WHERE entry_id = 2",
            [],
        )
        .unwrap();
    let journaled = commands(&store.conn);
    let unknown = |error: StoreError, column: usize| {
        let StoreError::Sqlite(rusqlite::Error::FromSqlConversionFailure(at, _, source)) = &error
        else {
            panic!("not a conversion failure: {error:?}");
        };
        let unknown = source.downcast_ref::<UnknownWord>().unwrap();
        assert_eq!(
            (*at, unknown.to_string(), unknown.column),
            (
                column,
                "queue column queue_entries.state holds 'paused', which this brokkr does not know"
                    .to_string(),
                "queue_entries.state"
            )
        );
    };
    unknown(store.queue_list().unwrap_err(), 2);
    let moved = store.queue_command(first, QueueCommand::Move { to: 1 }, BY);
    unknown(moved.unwrap_err(), 1);
    unknown(
        store
            .queue_command(first, QueueCommand::Drop, BY)
            .unwrap_err(),
        1,
    );
    let new = NewEntry {
        payload: "launch 3",
        priority: 0,
        waits: &[],
    };
    unknown(store.queue_add(new, BY).unwrap_err(), 1);
    assert_eq!(commands(&store.conn), journaled);
}

#[test]
fn a_wait_reads_back_as_written_and_nothing_else_reads_as_one() {
    for (on, text) in [
        (WaitOn::Completed, "3:completed"),
        (WaitOn::Ended, "3:ended"),
    ] {
        let wait = Wait {
            entry: EntryId(3),
            on,
        };
        assert_eq!(
            (wait.to_string(), text.parse()),
            (text.to_string(), Ok(wait))
        );
    }
    for text in ["3", "x:completed", "3:named", "3:"] {
        let refused = text.parse::<Wait>().unwrap_err();
        let said =
            format!("'{text}' is not ENTRY:CONDITION, with CONDITION one of completed, ended");
        assert_eq!(
            (refused.to_string(), refused),
            (said, NotAWait(text.to_string()))
        );
    }
    let claimed = EntryState::Claimed { run: "r".into() };
    let states = [
        (EntryState::Queued, "queued", None),
        (EntryState::Held, "held", None),
        (claimed, "claimed", Some("r")),
        (EntryState::Dropped, "dropped", None),
    ];
    for (state, word, run) in states {
        assert_eq!((state.word(), state.run()), (word, run));
    }
}

/// The guards the migration installs refuse every rewrite of what is
/// fixed: an entry's launch, its run once written, the entry itself, its
/// waits, and the journaled commands.
#[test]
fn the_queue_guards_refuse_every_rewrite_of_what_is_fixed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let first = queue(&mut store, 1)[0];
    let waits = [Wait {
        entry: first,
        on: WaitOn::Completed,
    }];
    let new = NewEntry {
        payload: "second",
        priority: 0,
        waits: &waits,
    };
    let second = store.queue_add(new, BY).unwrap();
    store
        .queue_latch(second, "drifted", Seen::default(), BY)
        .unwrap();
    store
        .queue_repin(second, "second, re-pinned", over(Some(3)), BY)
        .unwrap();
    started(&mut store, "run-a");
    started(&mut store, "run-b");
    store.queue_claim(first, "run-a").unwrap();
    let cases = [
        (
            "UPDATE queue_entries SET payload = 'moved'",
            "a queue entry's launch, priority and arrival are fixed",
        ),
        (
            "UPDATE queue_entries SET priority = 9",
            "a queue entry's launch, priority and arrival are fixed",
        ),
        (
            "UPDATE queue_entries SET run_id = 'run-b' WHERE entry_id = 1",
            "a queue entry's run is written once",
        ),
        (
            "UPDATE queue_entries SET run_id = NULL WHERE entry_id = 1",
            "a queue entry's run is written once",
        ),
        (
            "DELETE FROM queue_entries WHERE entry_id = 2",
            "queue entries are never deleted; drop one",
        ),
        (
            "UPDATE queue_waits SET condition = 'ended'",
            "queue waits are fixed",
        ),
        ("DELETE FROM queue_waits", "queue waits are fixed"),
        (
            "UPDATE queue_commands SET reason = 'rewritten'",
            "queue commands are append-only",
        ),
        (
            "DELETE FROM queue_commands",
            "queue commands are append-only",
        ),
        (
            "UPDATE queue_pins SET payload = 'rewritten'",
            "queue pins are append-only",
        ),
        ("DELETE FROM queue_pins", "queue pins are append-only"),
        (
            "UPDATE queue_latches SET finding = 'rewritten'",
            "queue latches are append-only",
        ),
        ("DELETE FROM queue_latches", "queue latches are append-only"),
    ];
    for (statement, guard) in cases {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(error.to_string(), guard, "{statement}");
    }
}

/// A journal whose queue is stored as slice 1's binary leaves it: version
/// 1, with no pins, holding one entry and its `add`.
fn journal_with_a_version_1_queue(path: &Path) {
    journal_at_mains_schema(path);
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(storage::MIGRATION_QUEUE_V1).unwrap();
    conn.execute_batch(
        "INSERT INTO meta (key, value) VALUES ('queue_schema', '1');
         INSERT INTO queue_entries (payload, priority, added_at, state, position)
             VALUES ('launch 1', 0, 'then', 'queued', 1);
         INSERT INTO queue_commands (entry_id, command, position, operator, reason, recorded_at)
             VALUES (1, 'add', 1, 'vy', 'operator''s word', 'then');",
    )
    .unwrap();
}

/// A version 1 queue reads as it stands, and the first open that may
/// write brings it to version 2 with its entries and commands kept, and
/// re-pins then work on it.
#[test]
fn a_version_1_queue_is_read_as_it_stands_and_migrated_with_its_entries_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    journal_with_a_version_1_queue(&path);
    let first = (1, Some(1), EntryState::Queued, 0, vec![], payload(1));
    let read = Store::open_read_only(&path).unwrap();
    assert_eq!(queue_schema(&read.conn).unwrap(), Some(1));
    assert_eq!(rows(&read), vec![first.clone()]);
    let entry = read.queue_entry(EntryId(1)).unwrap();
    assert_eq!((entry.payload, entry.latch), (payload(1), None));
    drop(read);

    let mut store = opened_whole(&path);
    assert_eq!(rows(&store), vec![first]);
    store
        .queue_repin(EntryId(1), "launch 1, re-pinned", Seen::default(), BY)
        .unwrap();
    assert_eq!(
        store.queue_list().unwrap()[0].payload,
        "launch 1, re-pinned"
    );
    let word = |command: &str, place| (1, command.into(), place, "vy".into(), BY.reason.into());
    assert_eq!(
        commands(&store.conn),
        vec![word("add", Some(1)), word("repin", None)]
    );
}

/// The recorded version is read before anything is repaired (#430's L4):
/// a queue stored by a brokkr this one does not know keeps the guards and
/// tables it was found with, whatever this binary's own queue would hold.
#[test]
fn a_queue_stored_in_a_version_this_brokkr_does_not_know_is_left_as_found() {
    let dir = tempfile::tempdir().unwrap();
    let surgery = "UPDATE meta SET value = '3' WHERE key = 'queue_schema';
                   DROP TRIGGER queue_entries_run_once;
                   DROP TABLE queue_pins;";
    let (path, store) = journal_after(dir.path(), surgery);
    drop(store);
    let store = Store::open(&path).unwrap();
    let named = |name: &str| {
        let query = "SELECT count(*) FROM sqlite_master WHERE name = ?1";
        store
            .conn
            .query_row(query, [name], |row| row.get::<_, i64>(0))
            .unwrap()
    };
    assert_eq!(
        (named("queue_entries_run_once"), named("queue_pins")),
        (0, 0)
    );
    assert_eq!(queue_schema(&store.conn).unwrap(), Some(3));
}

#[test]
fn a_repin_stands_beside_the_launch_queued_and_survives_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let ids = queue(&mut store, 2);
    store.queue_command(ids[1], QueueCommand::Hold, BY).unwrap();
    for (entry, pinned) in [(0, "1, again"), (1, "2, again"), (0, "1, a third time")] {
        let read = seen(&store, ids[entry]);
        store.queue_repin(ids[entry], pinned, read, BY).unwrap();
    }
    drop(store);

    let store = Store::open(&path).unwrap();
    assert_eq!(
        rows(&store),
        vec![
            (
                1,
                Some(1),
                EntryState::Queued,
                0,
                vec![],
                "1, a third time".into()
            ),
            (2, Some(2), EntryState::Held, 0, vec![], "2, again".into()),
        ]
    );
    assert_eq!(store.queue_entry(ids[1]).unwrap().payload, "2, again");
    let read = |query: &str| -> Vec<(i64, i64, String)> {
        store
            .conn
            .prepare(query)
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    // The launches queued stay, and each pin is keyed by its command.
    assert_eq!(
        read("SELECT entry_id, priority, payload FROM queue_entries"),
        vec![(1, 0, payload(1)), (2, 0, payload(2))]
    );
    let pins = read("SELECT seq, entry_id, payload FROM queue_pins ORDER BY seq");
    let repins = read(
        "SELECT seq, entry_id, command FROM queue_commands WHERE command = 'repin' ORDER BY seq",
    );
    assert_eq!(
        pins,
        vec![
            (4, 1, "1, again".into()),
            (5, 2, "2, again".into()),
            (6, 1, "1, a third time".into())
        ]
    );
    let keyed: Vec<(i64, i64)> = repins.iter().map(|r| (r.0, r.1)).collect();
    assert_eq!(keyed, vec![(4, 1), (5, 2), (6, 1)]);
}

#[test]
fn a_repin_of_an_entry_that_left_the_queue_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, [_, _, dropped, claimed]) = every_standing(dir.path());
    let before = (rows(&store), commands(&store.conn));
    let absent = EntryId(99);
    let mut repin = |entry| refusal(store.queue_repin(entry, "x", Seen::default(), BY));
    refused_as(
        repin(dropped),
        QueueRefusal::Dropped(dropped),
        "queue entry 3 was dropped",
    );
    refused_as(
        repin(claimed),
        claimed_by_run_b(claimed),
        "queue entry 4 is claimed by run 'run-b'",
    );
    refused_as(
        repin(absent),
        QueueRefusal::UnknownEntry(absent),
        "queue entry 99 does not exist",
    );
    assert_eq!((rows(&store), commands(&store.conn)), before);
    // One entry is read whatever its standing; one never queued is not.
    assert_eq!(
        store.queue_entry(dropped).unwrap().state,
        EntryState::Dropped
    );
    refused_as(
        refusal(store.queue_entry(absent)),
        QueueRefusal::UnknownEntry(absent),
        "queue entry 99 does not exist",
    );
    // A journal from before the queue holds no entry at all.
    let path = dir.path().join("before.db");
    journal_at_mains_schema(&path);
    let before_the_queue = Store::open_read_only(&path).unwrap();
    refused_as(
        refusal(before_the_queue.queue_entry(EntryId(1))),
        QueueRefusal::UnknownEntry(EntryId(1)),
        "queue entry 1 does not exist",
    );
}

/// A latch stands on its entry until a later re-pin of that entry, and
/// across a reopen; a later latch stands in its place, and every one is
/// journaled as a `latch`. Nothing else clears one (#430's H1).
#[test]
fn a_latch_stands_until_a_later_repin_and_survives_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let ids = queue(&mut store, 2);
    let standing = |store: &Store| -> Vec<Option<Latch>> {
        let listed = store.queue_list().unwrap();
        listed.into_iter().map(|entry| entry.latch).collect()
    };
    let latch = |seq, finding: &str| {
        Some(Latch {
            seq,
            finding: finding.into(),
        })
    };
    for finding in ["boundary moved", "boundary back"] {
        let read = seen(&store, ids[0]);
        store.queue_latch(ids[0], finding, read, BY).unwrap();
    }
    store.queue_command(ids[0], QueueCommand::Hold, BY).unwrap();
    store
        .queue_command(ids[0], QueueCommand::Release, BY)
        .unwrap();
    drop(store);

    let mut store = Store::open(&path).unwrap();
    assert_eq!(standing(&store), vec![latch(4, "boundary back"), None]);
    store
        .queue_repin(ids[0], "1, again", over(Some(4)), BY)
        .unwrap();
    assert_eq!(standing(&store), vec![None, None]);
    let read = seen(&store, ids[0]);
    assert_eq!(
        read,
        Seen {
            pin: Some(7),
            latch: None
        }
    );
    store.queue_latch(ids[0], "grant added", read, BY).unwrap();
    assert_eq!(
        store.queue_entry(ids[0]).unwrap().latch,
        latch(8, "grant added")
    );
    let word = |command: &str| (1, command.into(), None, "vy".into(), BY.reason.into());
    assert_eq!(
        commands(&store.conn)[2..],
        [
            word("latch"),
            word("latch"),
            word("hold"),
            word("release"),
            word("repin"),
            word("latch")
        ]
    );
}

#[test]
fn a_latch_on_an_entry_that_left_the_queue_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, [_, _, dropped, claimed]) = every_standing(dir.path());
    let before = (rows(&store), commands(&store.conn));
    let mut latch = |entry| refusal(store.queue_latch(entry, "x", Seen::default(), BY));
    refused_as(
        latch(dropped),
        QueueRefusal::Dropped(dropped),
        "queue entry 3 was dropped",
    );
    refused_as(
        latch(claimed),
        claimed_by_run_b(claimed),
        "queue entry 4 is claimed by run 'run-b'",
    );
    assert_eq!((rows(&store), commands(&store.conn)), before);
}

/// A re-pin is written only over the latch its caller read (#430's H2):
/// a peer's latch or re-pin since that read refuses it in the writing
/// transaction, naming the latch that stands now, and nothing is written.
#[test]
fn a_repin_over_a_latch_that_no_longer_stands_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let entry = queue(&mut store, 1)[0];
    for finding in ["open -> harness", "open -> namespace"] {
        let read = seen(&store, entry);
        store.queue_latch(entry, finding, read, BY).unwrap();
    }
    let before = (rows(&store), commands(&store.conn));
    let peer = Latch {
        seq: 3,
        finding: "open -> namespace".into(),
    };
    let moved = |standing| QueueRefusal::LatchMoved { entry, standing };
    let text = "queue entry 1's latched hold is not the one the re-pin was shown";
    for read in [Some(2), None] {
        let refused = refusal(store.queue_repin(entry, "x", over(read), BY));
        refused_as(refused, moved(Some(peer.clone())), text);
    }
    assert_eq!((rows(&store), commands(&store.conn)), before);

    // A peer's re-pin since: no latch stands to take.
    store
        .queue_repin(entry, "1, again", over(Some(3)), BY)
        .unwrap();
    let refused = refusal(store.queue_repin(entry, "x", over(Some(3)), BY));
    refused_as(refused, moved(None), text);
    assert_eq!(store.queue_entry(entry).unwrap().payload, "1, again");
}

/// A latch is written only over the pin and the latch its finding was
/// measured against (#430's H3): a peer's re-pin or latch since refuses
/// it in the writing transaction, and a re-pin over a pin it was not
/// shown is refused too, whatever latch stands; nothing is written.
#[test]
fn a_latch_or_repin_over_a_pin_or_latch_that_no_longer_stands_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let entry = queue(&mut store, 1)[0];
    let measured = seen(&store, entry);
    store.queue_latch(entry, "found", measured, BY).unwrap();
    let unmeasured = QueueRefusal::Unmeasured { entry };
    let text = "queue entry 1 was re-pinned or latched after its finding was measured, and the \
                finding was not latched";
    // A peer latched since the finding was measured.
    let before = (rows(&store), commands(&store.conn));
    let refused = refusal(store.queue_latch(entry, "stale", measured, BY));
    refused_as(refused, unmeasured.clone(), text);
    assert_eq!((rows(&store), commands(&store.conn)), before);

    // A peer re-pinned since, and no latch stands before or after.
    store
        .queue_repin(entry, "1, again", over(Some(2)), BY)
        .unwrap();
    let before = (rows(&store), commands(&store.conn));
    let refused = refusal(store.queue_latch(entry, "stale", measured, BY));
    refused_as(refused, unmeasured, text);
    assert_eq!((rows(&store), commands(&store.conn)), before);

    // A re-pin shown the latch standing, but not the pin: refused.
    let pinned = seen(&store, entry);
    store.queue_latch(entry, "found again", pinned, BY).unwrap();
    let stale = Seen {
        pin: None,
        latch: Some(4),
    };
    let before = (rows(&store), commands(&store.conn));
    refused_as(
        refusal(store.queue_repin(entry, "x", stale, BY)),
        QueueRefusal::PinMoved { entry },
        "queue entry 1 was re-pinned after the re-pin was shown",
    );
    assert_eq!((rows(&store), commands(&store.conn)), before);
    assert_eq!(store.queue_entry(entry).unwrap().latch.unwrap().seq, 4);
}
