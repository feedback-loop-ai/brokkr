//! The dispatcher's claim and its steps (decision 0095 rulings 1 and 7):
//! a version 2 queue migrates with what it held, each step is journaled
//! under the dispatcher, the claim re-checks what its pass read, one entry
//! launches at a time, and every refusal is typed and writes nothing.

use std::path::Path;

use brokkr_core::EventType;
use rusqlite::Connection;
use serde_json::json;

use super::super::storage::{queue_schema, MIGRATION_QUEUE_V1, MIGRATION_QUEUE_V2};
use super::super::tests::{
    claimed_now, commands, five_columns, journal_at_mains_schema, opened_whole, payload, queue,
    refusal, refused_as, rows, seen, started, BY,
};
use super::*;
use crate::queue::{QueueCommand, Seen};

/// A journal whose queue is stored as main's binary leaves it at version
/// 2: entry 1 queued, re-pinned and then latched, 2 held, 3 linked to
/// `old-run` and 4 dropped, each with its commands.
fn journal_with_a_version_2_queue(path: &Path) {
    journal_at_mains_schema(path);
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(MIGRATION_QUEUE_V1).unwrap();
    conn.execute_batch(MIGRATION_QUEUE_V2).unwrap();
    conn.execute_batch(
        "INSERT INTO meta (key, value) VALUES ('queue_schema', '2');
         INSERT INTO queue_entries (payload, priority, added_at, state, position, run_id) VALUES
             ('launch 1', 0, 'then', 'queued', 1, NULL),
             ('launch 2', 0, 'then', 'held', 2, NULL),
             ('launch 3', 0, 'then', 'queued', NULL, 'old-run'),
             ('launch 4', 0, 'then', 'dropped', NULL, NULL);
         INSERT INTO queue_commands (entry_id, command, position, operator, reason, recorded_at)
         VALUES (1, 'add', 1, 'vy', 'w', 'then'), (2, 'add', 2, 'vy', 'w', 'then'),
             (3, 'add', 3, 'vy', 'w', 'then'), (4, 'add', 4, 'vy', 'w', 'then'),
             (1, 'repin', NULL, 'vy', 'w', 'then'), (1, 'latch', NULL, 'vy', 'w', 'then'),
             (2, 'hold', NULL, 'vy', 'w', 'then'), (4, 'drop', NULL, 'vy', 'w', 'then');
         INSERT INTO queue_pins (seq, entry_id, payload) VALUES (5, 1, 'launch 1, again');
         INSERT INTO queue_latches (seq, entry_id, finding) VALUES (6, 1, 'drifted');",
    )
    .unwrap();
}

/// A version 2 queue reads as it stands, and the first open that may
/// write brings it to version 3 with its order, holds, pins, latches and
/// links unchanged, and claims then work on it.
#[test]
fn a_version_2_queue_migrates_with_its_order_holds_pins_latches_and_links_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    journal_with_a_version_2_queue(&path);
    let read = Store::open_read_only(&path).unwrap();
    assert_eq!(queue_schema(&read.conn).unwrap(), Some(2));
    let at_version_2 = read.queue_list().unwrap();
    drop(read);

    let mut store = opened_whole(&path);
    assert_eq!(store.queue_list().unwrap(), at_version_2);
    let linked = EntryState::Claimed {
        run: "old-run".into(),
    };
    assert_eq!(
        rows(&store),
        vec![
            (
                1,
                Some(1),
                EntryState::Queued,
                0,
                vec![],
                "launch 1, again".into()
            ),
            (2, Some(2), EntryState::Held, 0, vec![], payload(2)),
            (3, None, linked, 0, vec![], payload(3)),
        ]
    );
    let first = store.queue_entry(EntryId(1)).unwrap();
    let standing = Seen {
        pin: Some(5),
        latch: Some(6),
    };
    assert_eq!(
        (first.seen(), first.latch.unwrap().finding),
        (standing, "drifted".into())
    );
    assert_eq!(
        store.queue_entry(EntryId(4)).unwrap().state,
        EntryState::Dropped
    );
    let claim = claimed_now(&mut store, EntryId(1)).unwrap().unwrap();
    assert_eq!(
        (claim, store.queue_entry(EntryId(1)).unwrap().state),
        (
            Claim {
                entry: EntryId(1),
                seq: 9
            },
            EntryState::Launching
        )
    );
}

/// The claim rows: seq, entry, generation, digest and run.
fn claims(conn: &Connection) -> Vec<(i64, i64, i64, String, Option<String>)> {
    let query = "SELECT seq, entry_id, generation, digest, run_id FROM queue_claims ORDER BY seq";
    five_columns(conn, query)
}

/// Two entries queued, `run-a` and `run-b` started, and the first entry
/// claimed.
fn two_runs_and_a_claim(dir: &Path) -> (Store, Vec<EntryId>, Claim) {
    let mut store = Store::open(&dir.join("forge.db")).unwrap();
    let ids = queue(&mut store, 2);
    started(&mut store, "run-a");
    started(&mut store, "run-b");
    let claim = claimed_now(&mut store, ids[0]).unwrap().unwrap();
    (store, ids, claim)
}

/// Claim `entry` as the store reads it now, as generation 7 admitting a
/// bundle of `digest`, judging no run.
fn claim_of(store: &mut Store, entry: EntryId, digest: &str) -> Claim {
    let read = store.queue_entry(entry).unwrap();
    let judged = store.queue_claim_with(&read, 0, digest, Generation(7), "admitted", |_| {
        Ok::<(), ()>(())
    });
    judged.unwrap().unwrap()
}

#[test]
fn each_step_is_journaled_by_the_dispatcher_and_the_claim_records_its_generation_and_digest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let ids = queue(&mut store, 2);
    started(&mut store, "run-a");
    let first = claim_of(&mut store, ids[0], "digest-1");
    // Claimed, it keeps its place until it is linked.
    let places = |store: &Store| -> Vec<(i64, Option<u32>, EntryState)> {
        rows(store).into_iter().map(|r| (r.0, r.1, r.2)).collect()
    };
    assert_eq!(
        places(&store),
        vec![
            (1, Some(1), EntryState::Launching),
            (2, Some(2), EntryState::Queued)
        ]
    );
    store.queue_link(first, "run-a", "started run-a").unwrap();
    store.queue_spawn(first, "driving run-a").unwrap();
    let second = claim_of(&mut store, ids[1], "digest-2");
    store
        .queue_reclaim(second, "its start wrote no run")
        .unwrap();
    let again = claim_of(&mut store, ids[1], "digest-2");
    drop(store);

    let store = Store::open(&path).unwrap();
    let run_a = EntryState::Claimed {
        run: "run-a".into(),
    };
    assert_eq!(
        places(&store),
        vec![(2, Some(1), EntryState::Launching), (1, None, run_a)]
    );
    let step = |entry, word: &str, reason: &str| {
        (entry, word.into(), None, "dispatch".into(), reason.into())
    };
    assert_eq!(
        commands(&store.conn)[2..],
        [
            step(1, "claim", "admitted"),
            step(1, "link", "started run-a"),
            step(1, "spawn", "driving run-a"),
            step(2, "claim", "admitted"),
            step(2, "reclaim", "its start wrote no run"),
            step(2, "claim", "admitted"),
        ]
    );
    // Each claim is keyed by its command, and a reclaimed entry's next
    // claim is a new one.
    assert_eq!((first.seq, second.seq, again.seq), (3, 6, 8));
    assert_eq!(
        claims(&store.conn),
        vec![
            (3, 1, 7, "digest-1".into(), Some("run-a".into())),
            (6, 2, 7, "digest-2".into(), None),
            (8, 2, 7, "digest-2".into(), None),
        ]
    );
}

#[test]
fn a_claim_is_refused_in_flight_while_another_entry_stands_launching() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let ids = queue(&mut store, 3);
    started(&mut store, "run-a");
    let first = claimed_now(&mut store, ids[0]).unwrap().unwrap();
    let text = |n| {
        format!(
            "queue entry {n} is launching, and no other entry is claimed until it is linked, \
             returned or held"
        )
    };
    let before = (rows(&store), commands(&store.conn));
    let refused = refusal(claimed_now(&mut store, ids[1]));
    refused_as(refused, QueueRefusal::InFlight(ids[0]), &text(1));
    assert_eq!((rows(&store), commands(&store.conn)), before);

    // Linked, the first no longer stands launching; the second is claimed,
    // and while it stands so the third is refused; returned, it frees it.
    store.queue_link(first, "run-a", "r").unwrap();
    let second = claimed_now(&mut store, ids[1]).unwrap().unwrap();
    let refused = refusal(claimed_now(&mut store, ids[2]));
    refused_as(refused, QueueRefusal::InFlight(ids[1]), &text(2));
    store.queue_reclaim(second, "r").unwrap();
    claimed_now(&mut store, ids[2]).unwrap().unwrap();
}

/// A hold, a re-pin or a latch written between the pass that read an
/// entry and its claim refuses the claim, and nothing is written.
#[test]
fn a_claim_over_a_hold_repin_or_latch_since_its_pass_read_the_entry_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let ids = queue(&mut store, 3);
    let read: Vec<QueueEntry> = (ids.iter())
        .map(|entry| store.queue_entry(*entry).unwrap())
        .collect();
    store.queue_command(ids[0], QueueCommand::Hold, BY).unwrap();
    let pinned = seen(&store, ids[1]);
    store.queue_repin(ids[1], "again", pinned, BY).unwrap();
    let latched = seen(&store, ids[2]);
    store.queue_latch(ids[2], "drifted", latched, BY).unwrap();
    let before = (rows(&store), commands(&store.conn));
    let mut claim = |read: &QueueEntry| {
        let judged = store.queue_claim_with(read, 0, "d", Generation(1), "r", |_| Ok::<(), ()>(()));
        refusal(judged)
    };
    refused_as(
        claim(&read[0]),
        QueueRefusal::Held(ids[0]),
        "queue entry 1 is held; release it first",
    );
    let unseen = |n| {
        format!(
            "queue entry {n} was re-pinned or latched after the pass that admitted it read it, \
             and it was not claimed"
        )
    };
    refused_as(claim(&read[1]), QueueRefusal::Unseen(ids[1]), &unseen(2));
    refused_as(claim(&read[2]), QueueRefusal::Unseen(ids[2]), &unseen(3));
    assert_eq!((rows(&store), commands(&store.conn)), before);
    // Read afresh, the re-pinned one is claimed.
    claimed_now(&mut store, ids[1]).unwrap().unwrap();
}

/// An operator command, admission's latch and a second claim on an entry
/// the dispatcher is launching are each refused, and nothing is written.
#[test]
fn every_command_on_a_launching_entry_is_refused_until_the_dispatcher_settles_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let ids = queue(&mut store, 2);
    let launching = ids[0];
    claimed_now(&mut store, launching).unwrap().unwrap();
    let before = (rows(&store), commands(&store.conn));
    let text = "queue entry 1 is launching; the dispatcher links, returns or holds it first";
    let commands_given = [
        QueueCommand::Move { to: 2 },
        QueueCommand::Hold,
        QueueCommand::Release,
        QueueCommand::Drop,
    ];
    let read = seen(&store, launching);
    let mut refused: Vec<QueueRefusal> = (commands_given.into_iter())
        .map(|command| refusal(store.queue_command(launching, command, BY)))
        .collect();
    refused.push(refusal(store.queue_repin(launching, "x", read, BY)));
    refused.push(refusal(store.queue_latch(launching, "x", read, BY)));
    refused.push(refusal(claimed_now(&mut store, launching)));
    for refused in refused {
        refused_as(refused, QueueRefusal::Launching(launching), text);
    }
    assert_eq!((rows(&store), commands(&store.conn)), before);
}

#[test]
fn a_step_under_a_claim_that_does_not_stand_for_it_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, ids, old) = two_runs_and_a_claim(dir.path());
    store.queue_reclaim(old, "r").unwrap();
    let standing = claimed_now(&mut store, ids[0]).unwrap().unwrap();
    let elsewhere = Claim {
        entry: ids[1],
        seq: standing.seq,
    };
    let stale = |claim: Claim| {
        let text = format!(
            "queue entry {}'s claim {} does not stand for this step",
            claim.entry, claim.seq
        );
        (QueueRefusal::Stale(claim), text)
    };
    let check = |store: &mut Store, claim: Claim| {
        let before = (rows(store), commands(&store.conn));
        let (variant, text) = stale(claim);
        for refused in [
            refusal(store.queue_link(claim, "run-a", "r")),
            refusal(store.queue_spawn(claim, "r")),
            refusal(store.queue_reclaim(claim, "r")),
        ] {
            refused_as(refused, variant.clone(), &text);
        }
        assert_eq!((rows(store), commands(&store.conn)), before);
    };
    // A claim since ended, and a token for an entry not launching.
    check(&mut store, old);
    check(&mut store, elsewhere);
    // Linked, the claim stands for its spawn alone, and that only once.
    store.queue_link(standing, "run-a", "r").unwrap();
    let (variant, text) = stale(standing);
    refused_as(
        refusal(store.queue_link(standing, "run-b", "r")),
        variant.clone(),
        &text,
    );
    refused_as(
        refusal(store.queue_reclaim(standing, "r")),
        variant.clone(),
        &text,
    );
    store.queue_spawn(standing, "r").unwrap();
    let before = commands(&store.conn);
    refused_as(refusal(store.queue_spawn(standing, "r")), variant, &text);
    assert_eq!(commands(&store.conn), before);
    let absent = Claim {
        entry: EntryId(99),
        seq: 1,
    };
    refused_as(
        refusal(store.queue_link(absent, "run-a", "r")),
        QueueRefusal::UnknownEntry(EntryId(99)),
        "queue entry 99 does not exist",
    );
}

/// The claim's judge is handed each run with an event past the
/// high-water the pass read, by id with its events and manifest: a run
/// that moved and one that started, never one that did not move or one
/// with no event. Its refusal is the claim's, and nothing is written.
#[test]
fn the_claim_hands_its_judge_each_run_past_the_high_water_and_its_refusal_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert_eq!(store.high_water().unwrap(), 0);
    let entry = queue(&mut store, 1)[0];
    let append = |store: &mut Store, run: &str, event: EventType| {
        let payload = json!({"feature": "f", "reason": "lost"});
        store.append_next(run, event, payload, None, None).unwrap();
    };
    for run in ["run-a", "run-b"] {
        started(&mut store, run);
        append(&mut store, run, EventType::RunStarted);
    }
    let high_water = store.high_water().unwrap();
    assert_eq!(high_water, 2);
    append(&mut store, "run-b", EventType::RunParked);
    started(&mut store, "run-c");
    append(&mut store, "run-c", EventType::RunStarted);
    started(&mut store, "run-d");

    let read = store.queue_entry(entry).unwrap();
    let before = (rows(&store), commands(&store.conn));
    let mut handed = Vec::new();
    let refused = store
        .queue_claim_with(&read, high_water, "d", Generation(1), "r", |moved| {
            handed = (moved.iter())
                .map(|(run, events, manifest)| (run.clone(), events.len(), manifest.clone()))
                .collect();
            Err("no room")
        })
        .unwrap();
    assert_eq!(refused, Err("no room"));
    let manifest = r#"{"schema":"run-manifest/v1"}"#.to_string();
    assert_eq!(
        handed,
        vec![
            ("run-b".to_string(), 2, manifest.clone()),
            ("run-c".to_string(), 1, manifest)
        ]
    );
    assert_eq!((rows(&store), commands(&store.conn)), before);
    assert!(claims(&store.conn).is_empty());

    // Judged to have room, it is claimed.
    let claimed = store.queue_claim_with(&read, high_water, "d", Generation(1), "r", |_| {
        Ok::<(), &str>(())
    });
    let claim = claimed.unwrap().unwrap();
    assert_eq!(
        claims(&store.conn),
        vec![(claim.seq, 1, 1, "d".into(), None)]
    );
}

/// Version 3's guards refuse every rewrite of what is fixed: an entry's
/// run written but while it is launching, a claim's entry, generation and
/// digest, its run once written, the claim itself, and a counted refusal.
#[test]
fn version_3s_guards_refuse_every_rewrite_of_what_is_fixed() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _, claim) = two_runs_and_a_claim(dir.path());
    store.queue_link(claim, "run-a", "r").unwrap();
    store
        .conn
        .execute(
            "INSERT INTO queue_refusals (seq, entry_id, refusal) VALUES (1, 1, 'compile')",
            [],
        )
        .unwrap();
    let fixed = "a queue claim's entry, generation and digest are fixed";
    let cases = [
        (
            "UPDATE queue_entries SET run_id = 'run-b' WHERE entry_id = 2",
            "a queue entry's run is written only while it is launching",
        ),
        ("UPDATE queue_claims SET digest = 'moved'", fixed),
        ("UPDATE queue_claims SET generation = 2", fixed),
        ("UPDATE queue_claims SET entry_id = 2", fixed),
        (
            "UPDATE queue_claims SET run_id = 'run-b'",
            "a queue claim's run is written once",
        ),
        ("DELETE FROM queue_claims", "queue claims are never deleted"),
        (
            "UPDATE queue_refusals SET refusal = 'rewritten'",
            "queue refusals are append-only",
        ),
        (
            "DELETE FROM queue_refusals",
            "queue refusals are append-only",
        ),
    ];
    for (statement, guard) in cases {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(error.to_string(), guard, "{statement}");
    }
}
