//! The journal's dispatchers and its dispatch settings (decision 0095
//! rulings 2 and 7): each dispatcher records the next generation, only the
//! latest one's steps land, the bound reads 3 unset and refuses a setting
//! it cannot read, and a counted refusal that reaches it holds the entry.

use std::num::NonZeroU32;

use rusqlite::Connection;

use super::super::tests::{
    claimed_now, commands, five_columns, queue, refusal, refused_as, rows, started, BY,
};
use super::*;
use crate::queue::{Claim, EntryId, EntryState, QueueCommand, StartRefusal};

/// Everything a refused step could have written: the entries, the
/// commands, and the count of claims and of counted refusals.
fn written(store: &Store) -> impl PartialEq + std::fmt::Debug {
    let count = |table: &str| -> i64 {
        let query = format!("SELECT COUNT(*) FROM {table}");
        store.conn.query_row(&query, [], |row| row.get(0)).unwrap()
    };
    let tables = [count("queue_claims"), count("queue_refusals")];
    (rows(store), commands(&store.conn), tables)
}

/// The refusal of a step under `generation`, which is not the latest
/// holder, and its text.
fn superseded(generation: Generation) -> (QueueRefusal, String) {
    let text = format!(
        "dispatcher generation {} is not the journal's latest holder; it writes nothing",
        generation.0
    );
    (QueueRefusal::Superseded(generation), text)
}

#[test]
fn each_dispatcher_records_the_next_generation_and_the_latest_is_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert_eq!(store.dispatch_holder().unwrap(), None);
    assert_eq!(store.dispatch_hold(41).unwrap(), Generation(1));
    assert_eq!(store.dispatch_hold(42).unwrap(), Generation(2));
    let query = "SELECT generation, host, pid, started_at, started_at FROM dispatch_holders
        ORDER BY generation";
    let held: Vec<(i64, Option<String>, u32, String, String)> = five_columns(&store.conn, query);
    let host = crate::origin::local_host();
    let pids: Vec<_> = (held.iter())
        .map(|row| (row.0, row.1.clone(), row.2))
        .collect();
    assert_eq!(pids, vec![(1, host.clone(), 41), (2, host, 42)]);
    let latest = store.dispatch_holder().unwrap();
    assert_eq!(latest, Some((Generation(2), 42, held[1].3.clone())));
    for statement in [
        "UPDATE dispatch_holders SET pid = 7",
        "DELETE FROM dispatch_holders",
    ] {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(error.to_string(), "dispatch holders are append-only");
    }
}

/// A claim under no holder, or under one a newer holder followed, is
/// refused by variant with nothing written, and the latest's claim lands.
#[test]
fn a_claim_under_a_generation_not_the_latest_holder_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let entry = queue(&mut store, 1)[0];
    let read = store.queue_entry(entry).unwrap();
    let claim = |store: &mut Store, generation| {
        let judge = |_, _: &[_], _: &[_]| Ok::<(), ()>(());
        store.queue_claim_with(&read, 0, "d", generation, "r", judge)
    };
    let before = written(&store);
    let (variant, text) = superseded(Generation(1));
    refused_as(refusal(claim(&mut store, Generation(1))), variant, &text);
    let first = store.dispatch_hold(1).unwrap();
    let second = store.dispatch_hold(2).unwrap();
    let (variant, text) = superseded(first);
    refused_as(refusal(claim(&mut store, first)), variant, &text);
    assert_eq!(written(&store), before);
    let claimed = claim(&mut store, second).unwrap().unwrap();
    assert_eq!(claimed, Claim { entry, seq: 2 });
}

/// A link, a spawn, a reclaim and the dispatcher's hold, each under a
/// claim its generation took before a newer dispatcher held the journal,
/// are refused by variant, with nothing written: a dispatcher fenced out
/// writes nothing.
#[test]
fn every_step_after_the_claim_under_a_superseded_generation_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let ids = queue(&mut store, 2);
    let first = store.dispatch_hold(1).unwrap();
    let linked = claimed_now(&mut store, ids[0]).unwrap().unwrap();
    started(&mut store, "run-a");
    store.queue_link(linked, first, "run-a", "r").unwrap();
    let launching = claimed_now(&mut store, ids[1]).unwrap().unwrap();
    started(&mut store, "run-b");
    store.dispatch_hold(2).unwrap();

    let before = written(&store);
    let (variant, text) = superseded(first);
    let compile = Some(StartRefusal::Compile);
    for refused in [
        refusal(store.queue_spawn(linked, first, "r")),
        refusal(store.queue_link(launching, first, "run-b", "r")),
        refusal(store.queue_reclaim(launching, first, None, NonZeroU32::MIN, "r")),
        refusal(store.queue_reclaim(launching, first, compile, NonZeroU32::MIN, "r")),
    ] {
        refused_as(refused, variant.clone(), &text);
    }
    assert_eq!(written(&store), before);
}

/// The settings: `seq`, bound, operator and reason.
fn settings(conn: &Connection) -> Vec<(i64, i64, String, String, String)> {
    let query = "SELECT seq, bound, operator, reason, recorded_at FROM dispatch_settings
        ORDER BY seq";
    five_columns(conn, query)
}

/// A journal whose operator never set a bound reads 3; each setting is
/// journaled with its operator and reason, and the latest is in force.
/// The column refuses, by its `CHECK`, anything but a whole number of at
/// least 1 that a `u32` holds, and the settings are never rewritten.
#[test]
fn the_bound_reads_3_unset_and_the_latest_setting_after() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert_eq!(store.dispatch_bound().unwrap().get(), 3);
    for bound in [2, 5] {
        let bound = NonZeroU32::new(bound).unwrap();
        store.set_dispatch_bound(bound, BY).unwrap();
        assert_eq!(store.dispatch_bound().unwrap(), bound);
    }
    let set: Vec<_> = (settings(&store.conn).into_iter())
        .map(|row| (row.0, row.1, row.2, row.3))
        .collect();
    let by = |seq, bound| (seq, bound, "vy".into(), "operator's word".into());
    assert_eq!(set, vec![by(1, 2), by(2, 5)]);
    let check = "CHECK constraint failed: \
        typeof(bound) = 'integer' AND bound BETWEEN 1 AND 4294967295";
    for (bound, refused) in [
        ("0", check),
        ("-1", check),
        ("4294967296", check),
        ("'x'", check),
        ("2.5", check),
        (
            "NULL",
            "NOT NULL constraint failed: dispatch_settings.bound",
        ),
    ] {
        let planted = format!(
            "INSERT INTO dispatch_settings (bound, operator, reason, recorded_at)
             VALUES ({bound}, 'vy', 'w', 'then')"
        );
        let error = store.conn.execute(&planted, []).unwrap_err();
        assert_eq!(error.to_string(), refused, "{bound}");
    }
    for statement in [
        "UPDATE dispatch_settings SET bound = 7",
        "DELETE FROM dispatch_settings",
    ] {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(error.to_string(), "dispatch settings are append-only");
    }
    assert_eq!(store.dispatch_bound().unwrap().get(), 5);
}

/// A latest setting written past the `CHECK` that does not read as a
/// bound of at least 1 refuses its reader with its typed error: no default
/// stands in for it. A readable setting after it is in force again.
#[test]
fn a_setting_that_does_not_read_as_a_bound_refuses_its_reader() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    let plant = |bound: &str| {
        let planted = format!(
            "INSERT INTO dispatch_settings (bound, operator, reason, recorded_at)
             VALUES ({bound}, 'vy', 'w', 'then')"
        );
        store.conn.execute(&planted, []).unwrap();
    };
    for bound in ["0", "-1", "4294967296", "'x'", "2.5"] {
        plant(bound);
        let error = store.dispatch_bound().unwrap_err();
        let StoreError::Sqlite(rusqlite::Error::FromSqlConversionFailure(_, _, source)) = &error
        else {
            panic!("not a conversion failure: {error:?}");
        };
        assert_eq!(source.downcast_ref::<UnreadBound>(), Some(&UnreadBound));
        assert_eq!(
            source.to_string(),
            "the journal's latest dispatch setting does not hold a whole number of at least 1, \
             so its bound on counted refusals is unreadable, and no default stands in for it \
             (decision 0095)",
            "{bound}"
        );
    }
    plant("4294967295");
    assert_eq!(store.dispatch_bound().unwrap(), NonZeroU32::MAX);
}

/// Claim `entry` and dispose of its start as `refused` under `bound`:
/// the count said, and where the entry stands after.
fn disposed(
    store: &mut Store,
    entry: EntryId,
    refused: Option<StartRefusal>,
    bound: NonZeroU32,
) -> (u32, EntryState) {
    let claim = claimed_now(store, entry).unwrap().unwrap();
    let count = (store.queue_reclaim(claim, Generation(1), refused, bound, "refused")).unwrap();
    (count, store.queue_entry(entry).unwrap().state)
}

/// Under the default bound, two counted refusals of one entry reclaim it
/// and the third holds it, by the dispatcher's `hold`, its refusal's row
/// keyed by the `hold`; a digest refusal or a `Contended` start between
/// them counts for nothing. A release resets the count. Under a bound of
/// 1, the first counted refusal holds.
#[test]
fn the_counted_refusal_that_reaches_the_bound_holds_the_entry_and_a_release_resets_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let ids = queue(&mut store, 2);
    let bound = store.dispatch_bound().unwrap();
    let (queued, held) = (EntryState::Queued, EntryState::Held);
    let mut dispose = |refused| disposed(&mut store, ids[0], refused, bound);
    let counts = [
        dispose(Some(StartRefusal::Compile)),
        dispose(None),
        dispose(Some(StartRefusal::Crossing)),
        dispose(None),
        dispose(Some(StartRefusal::Unboxable)),
    ];
    let expected = [
        (1, &queued),
        (1, &queued),
        (2, &queued),
        (2, &queued),
        (3, &held),
    ];
    assert_eq!(counts.each_ref().map(|(n, s)| (*n, s)), expected);
    let journaled = commands(&store.conn);
    let dispatch = |word: &str, why: &str| (1, word.into(), None, "dispatch".into(), why.into());
    let last = [dispatch("claim", "r"), dispatch("hold", "refused")];
    assert_eq!(journaled[10..], last);
    let held_at: i64 = store
        .conn
        .query_row("SELECT MAX(seq) FROM queue_refusals", [], |row| row.get(0))
        .unwrap();
    assert_eq!(held_at, 12, "the third refusal is keyed by the hold");
    let generation: i64 = store
        .conn
        .query_row(
            "SELECT generation FROM queue_claims WHERE seq = 11",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(generation, 1, "the hold ends the claim generation 1 took");

    store
        .queue_command(ids[0], QueueCommand::Release, BY)
        .unwrap();
    let after = disposed(&mut store, ids[0], Some(StartRefusal::Compile), bound);
    assert_eq!(after, (1, queued));
    let one = NonZeroU32::MIN;
    store.set_dispatch_bound(one, BY).unwrap();
    let bound = store.dispatch_bound().unwrap();
    let first = disposed(&mut store, ids[1], Some(StartRefusal::Compile), bound);
    assert_eq!(first, (1, held));
}
