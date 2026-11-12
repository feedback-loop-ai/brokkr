//! A provider's cool-downs in the queue (decision 0103 ruling 2(a), slice
//! 4d1): the rows land with their terminal event in one transaction or not
//! at all, the table's CHECK constraints and guards refuse every rewrite, the
//! aggregate is the latest end across every unexpired row, and a claim
//! reads the rows in force inside its own transaction.

use std::path::Path;

use brokkr_core::envelope::EventType;
use rusqlite::{params, Connection};
use serde_json::json;
use time::{Duration, OffsetDateTime};

use super::*;
use crate::queue::tests::queue;
use crate::queue::{Claiming, QueueRefusal};
use crate::StoreError;

/// A fixed instant after the event's own recorded time, so a row's end is
/// still in force when the writer records it; each test's `at(n)` is
/// `n` seconds after it.
fn base() -> OffsetDateTime {
    static BASE: std::sync::OnceLock<OffsetDateTime> = std::sync::OnceLock::new();
    *BASE.get_or_init(|| {
        OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .expect("a zero nanosecond is valid")
            + Duration::hours(1)
    })
}

/// The instant a test's clock reads, `seconds` after [`base`].
fn at(seconds: i64) -> OffsetDateTime {
    base() + Duration::seconds(seconds)
}

/// One row for `route`, ending at `until`, as a refused site records it.
fn row(site: &str, provider: &str, route: &str, until: OffsetDateTime) -> Cooling {
    Cooling {
        site: site.into(),
        provider: provider.into(),
        route: route.into(),
        until_at: until,
        reason: "rate_limit: slow down".into(),
        retry_after_seconds: Some(30),
        rounds: 1,
    }
}

/// A row of `route` whose words name it, so an aggregate's choice reads
/// back.
fn named(site: &str, route: &str, until: OffsetDateTime, reason: &str, rounds: u64) -> Cooling {
    Cooling {
        site: site.into(),
        provider: "claude".into(),
        route: route.into(),
        until_at: until,
        reason: reason.into(),
        retry_after_seconds: None,
        rounds,
    }
}

/// Append a terminal event carrying `rows` to `run`, unfenced.
fn append(
    store: &mut Store,
    run: &str,
    rows: &[Cooling],
) -> Result<brokkr_core::EventEnvelope, StoreError> {
    store.append_with_cooldowns(
        crate::CooldownAppend {
            run_id: run,
            fenced: None,
            event_type: EventType::EffectFailed,
            payload: json!({"effect_id": "effect", "error": "the provider's words"}),
            causation_id: None,
            attempt_id: None,
        },
        rows,
    )
}

/// Every column of every cool-down row, in seq order: the raw table, so a
/// test reads what the writer stored rather than what a reader names.
type Raw = (
    i64,
    String,
    i64,
    String,
    String,
    String,
    i64,
    String,
    Option<i64>,
    i64,
    i64,
);

fn raw(conn: &Connection) -> Vec<Raw> {
    let query = "SELECT seq, run_id, event_seq, site, provider, route, until_at, reason,
                        retry_after_seconds, rounds, recorded_at
                 FROM provider_cooldowns ORDER BY seq";
    conn.prepare(query)
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// A run the journal holds, planted as a journal an earlier binary left
/// would hold it: these tests write no summary, so no fold is carried.
fn planted(store: &Store, run: &str) {
    crate::test_support::plant_run(&store.conn, run, "{}").unwrap();
}

/// A store whose journal holds a run and one event at seq 1, so a raw row
/// has its parent event to reference.
fn journal(dir: &Path) -> Store {
    let mut store = Store::open(&dir.join("forge.db")).unwrap();
    planted(&store, "run-a");
    request(&mut store, "run-a");
    store
}

/// One effect request on `run`, so a later terminal event has a head to
/// chain onto.
fn request(store: &mut Store, run: &str) {
    store
        .append_next(
            run,
            EventType::EffectRequested,
            json!({"effect_id": "effect"}),
            None,
            None,
        )
        .unwrap();
}

/// The event and its rows land in one transaction, with the event's own
/// seq and recorded time on every row.
#[test]
fn a_terminal_event_and_its_rows_land_together() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    planted(&store, "run-a");
    let rows = [
        (row("a", "claude", "first", at(60))),
        (row("b", "codex", "second", at(90))),
    ];
    let event = append(&mut store, "run-a", &rows).unwrap();
    assert_eq!(event.seq, 1);

    let recorded = store.cooldowns_in_force(at(0)).unwrap();
    assert_eq!(recorded.len(), 2);
    // The rows read back are the rows written, at the event's recording
    // instant (to the second) and no other.
    for (read, wrote) in recorded.iter().zip(rows.iter()) {
        assert_eq!(read.site, wrote.site);
        assert_eq!(read.until_at, wrote.until_at);
    }
    let raw = raw(&store.conn);
    assert_eq!((raw[0].1.as_str(), raw[0].2), ("run-a", 1));
    assert_eq!(raw[0].3, "a");
    assert_eq!(raw[0].7, "rate_limit: slow down");
    assert_eq!(raw[0].8, Some(30));
    assert_eq!(raw[0].9, 1);
    assert_eq!(raw[1].2, 1);
    assert_eq!(raw[1].4, "codex");
    let event = store.load("run-a").unwrap().remove(0);
    let event = OffsetDateTime::parse(
        &event.recorded_at,
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap();
    assert_eq!(raw[0].10, event.unix_timestamp());
    assert_eq!(raw[0].6, rows[0].until_at.unix_timestamp());
    assert!(raw[0].6 > raw[0].10, "the until outlasts the record");
}

/// A row the table refuses rolls its event back with it: nothing is
/// written, not the row and not the event.
#[test]
fn a_row_the_table_refuses_leaves_neither_the_event_nor_a_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    planted(&store, "run-a");
    let mut bad = row("a", "claude", "first", at(60));
    bad.rounds = 0;
    let error = append(&mut store, "run-a", std::slice::from_ref(&bad)).unwrap_err();
    assert_eq!(
        error.to_string(),
        "sqlite: CHECK constraint failed: rounds_whole"
    );
    assert_eq!(store.load("run-a").unwrap().len(), 0);
    assert_eq!(raw(&store.conn), vec![]);
}

/// A wait whose end has already passed writes no row: the table holds
/// what is in force when the event records it, and an ended wait is
/// history. The event itself still lands.
#[test]
fn an_ended_wait_writes_no_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    planted(&store, "run-a");
    let ended = row("a", "claude", "first", at(-7200));
    let event = append(&mut store, "run-a", std::slice::from_ref(&ended)).unwrap();
    assert_eq!(event.seq, 1);
    assert_eq!(raw(&store.conn), vec![]);
}

/// A fenced append whose head moved writes neither the event nor a row.
#[test]
fn a_moved_head_writes_neither_the_event_nor_a_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    planted(&store, "run-a");
    request(&mut store, "run-a");
    let error = store
        .append_with_cooldowns(
            crate::CooldownAppend {
                run_id: "run-a",
                fenced: Some((7, "the head the pass read")),
                event_type: EventType::EffectFailed,
                payload: json!({"effect_id": "effect"}),
                causation_id: None,
                attempt_id: None,
            },
            &[row("a", "claude", "first", at(60))],
        )
        .unwrap_err();
    assert!(matches!(
        error,
        StoreError::HeadMoved {
            expected_seq: 7,
            found_seq: 1
        }
    ));
    assert_eq!(store.load("run-a").unwrap().len(), 1);
    assert_eq!(raw(&store.conn), vec![]);
}

/// Two runs at the same `event_seq` store both rows under their own run:
/// the unique key is `(run_id, event_seq, site, rounds)`, so a second
/// run's row never refuses its terminal event.
#[test]
fn two_runs_at_the_same_event_seq_store_both_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let request = |store: &mut Store, run: &str| {
        store
            .append_next(
                run,
                EventType::EffectRequested,
                json!({"effect_id": "effect"}),
                None,
                None,
            )
            .unwrap();
    };
    for run in ["run-a", "run-b"] {
        planted(&store, run);
        for _ in 0..6 {
            request(&mut store, run);
        }
    }
    // The same site and round in both runs: the run is what keeps the
    // key apart, so neither run's row refuses the other's terminal event.
    let a = append(&mut store, "run-a", &[row("a", "claude", "first", at(60))]).unwrap();
    let b = append(&mut store, "run-b", &[row("a", "codex", "second", at(60))]).unwrap();
    assert_eq!((a.seq, b.seq), (7, 7));
    let raw = raw(&store.conn);
    assert_eq!(raw.len(), 2);
    assert_eq!(
        (raw[0].1.as_str(), raw[0].2, raw[0].3.as_str()),
        ("run-a", 7, "a")
    );
    assert_eq!(
        (raw[1].1.as_str(), raw[1].2, raw[1].3.as_str()),
        ("run-b", 7, "a")
    );
}

/// One CHECK case: its name, the values its INSERT binds, and the
/// constraint that must refuse it.
type Case<'a> = (
    &'a str,
    (&'a str, &'a str, i64, &'a str, Option<i64>, i64, i64),
    &'a str,
);

/// Every CHECK the table carries refuses its own case, by the name of the
/// constraint it broke.
#[test]
fn every_check_refuses_its_own_case() {
    const INSERT: &str = "INSERT INTO provider_cooldowns
        (run_id, event_seq, site, provider, route, until_at, reason,
         retry_after_seconds, rounds, recorded_at)
        VALUES ('run-a', 1, '', ?1, ?2, ?3, ?4, ?5, ?6, ?7)";
    let cases: [Case; 8] = [
        (
            "the rounds are not whole",
            ("claude", "first", 60, "words", None, 1, 0),
            "rounds_whole",
        ),
        (
            "the rounds are zero",
            ("claude", "first", 60, "words", None, 0, 0),
            "rounds_whole",
        ),
        (
            "the retry-after is not positive",
            ("claude", "first", 60, "words", Some(0), 1, 0),
            "retry_after_positive",
        ),
        (
            "the until is not after the record",
            ("claude", "first", 0, "words", None, 1, 0),
            "until_after_record",
        ),
        (
            "the provider is empty",
            ("", "first", 60, "words", None, 1, 0),
            "provider_named",
        ),
        (
            "the route is empty",
            ("claude", "", 60, "words", None, 1, 0),
            "route_named",
        ),
        (
            "the reason is empty",
            ("claude", "first", 60, "", None, 1, 0),
            "reason_named",
        ),
        (
            "the until is text",
            ("claude", "first", 60, "words", None, 1, 0),
            "until_is_seconds",
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let store = journal(dir.path());
    for (what, (provider, route, until, reason, retry, rounds, recorded), broken) in cases {
        // SQLite binds a text where the column expects a whole number for
        // the last case, which is exactly the malformed text a CHECK must
        // refuse rather than pass on a NULL `unixepoch()`.
        let error = match broken {
            "until_is_seconds" => store
                .conn
                .execute(
                    INSERT,
                    params![
                        provider,
                        route,
                        "2026-11-01T00:00:00Z",
                        reason,
                        retry,
                        rounds,
                        recorded
                    ],
                )
                .expect_err(what),
            "rounds_whole" if what.contains("not whole") => store
                .conn
                .execute(
                    INSERT,
                    params![provider, route, until, reason, retry, "one", recorded],
                )
                .expect_err(what),
            _ => store
                .conn
                .execute(
                    INSERT,
                    params![provider, route, until, reason, retry, rounds, recorded],
                )
                .expect_err(what),
        };
        assert_eq!(
            error.to_string(),
            format!("CHECK constraint failed: {broken}"),
            "{what}"
        );
    }
    assert_eq!(raw(&store.conn), vec![]);
}

/// The table's guards refuse every update and every delete, as 0095's
/// tables do.
#[test]
fn the_cool_downs_are_append_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    append(&mut store, "run-a", &[row("a", "claude", "first", at(60))]).unwrap();
    for (statement, guard) in [
        (
            "UPDATE provider_cooldowns SET reason = 'rewritten'",
            "provider cool-downs are append-only",
        ),
        (
            "DELETE FROM provider_cooldowns",
            "provider cool-downs are append-only",
        ),
    ] {
        let error = store.conn.execute(statement, []).unwrap_err();
        assert_eq!(error.to_string(), guard, "{statement}");
    }
    assert_eq!(raw(&store.conn).len(), 1);
}

/// A journal over two planted runs, each recording one row: `a`'s for
/// run-a and `b`'s for run-b.
fn two_refusing_runs(dir: &Path, a: Cooling, b: Cooling) -> Store {
    let mut store = Store::open(&dir.join("forge.db")).unwrap();
    for (run, row) in [("run-a", a), ("run-b", b)] {
        planted(&store, run);
        append(&mut store, run, std::slice::from_ref(&row)).unwrap();
    }
    store
}

/// The aggregate is a monotonic MAX: two overlapping refusals from two
/// runs, the later one shorter, end at the longer, and the words are the
/// row that ends last.
#[test]
fn two_overlapping_refusals_aggregate_to_the_later_end() {
    let dir = tempfile::tempdir().unwrap();
    let store = two_refusing_runs(
        dir.path(),
        named("a", "first", at(30), "the earlier, longer", 1),
        named("b", "first", at(10), "the later, shorter", 1),
    );
    assert_eq!(
        store.cooldown("claude", "first", at(0)).unwrap(),
        Some(CooldownAggregate {
            until: at(30),
            reason: "the earlier, longer".into(),
        })
    );
    // An expired row is history: neither holds, and neither is deleted.
    assert_eq!(store.cooldown("claude", "first", at(30)).unwrap(), None);
    assert_eq!(raw(&store.conn).len(), 2);
    // A route no row names is held by nothing.
    assert_eq!(store.cooldown("codex", "second", at(0)).unwrap(), None);
}

/// The in-force read is every unexpired row and no expired one.
#[test]
fn an_expired_row_is_not_in_force() {
    let dir = tempfile::tempdir().unwrap();
    let store = two_refusing_runs(
        dir.path(),
        named("a", "first", at(30), "long", 1),
        named("b", "second", at(10), "short", 1),
    );
    let in_force = |store: &Store, now| -> Vec<String> {
        (store.cooldowns_in_force(now).unwrap())
            .into_iter()
            .map(|row| row.route)
            .collect()
    };
    assert_eq!(in_force(&store, at(0)), ["first", "second"]);
    assert_eq!(in_force(&store, at(10)), ["first"]);
    assert_eq!(in_force(&store, at(30)), Vec::<String>::new());
}

/// A stored instant this reader cannot spell is refused by name, never
/// read as expired (which would release the route) and never guessed at.
#[test]
fn a_time_the_reader_cannot_spell_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = journal(dir.path());
    let insert = "INSERT INTO provider_cooldowns
        (run_id, event_seq, site, provider, route, until_at, reason,
         retry_after_seconds, rounds, recorded_at)
        VALUES ('run-a', 1, '', 'claude', 'first', ?1, 'words', NULL, 1, 0)";
    store.conn.execute(insert, params![i64::MAX]).unwrap();
    let refusal = match store.cooldown("claude", "first", at(0)).unwrap_err() {
        StoreError::Queue(refusal) => refusal,
        other => panic!("not a queue refusal: {other:?}"),
    };
    assert_eq!(QueueRefusal::CooldownTime { seq: 1 }, refusal);
    assert_eq!(
        refusal.to_string(),
        "the journal's provider cool-down 1 records a time it cannot read"
    );
    assert!(matches!(
        store.cooldowns_in_force(at(0)),
        Err(StoreError::Queue(QueueRefusal::CooldownTime { seq: 1 }))
    ));
}

/// The claim reads the rows in force inside its own transaction: a row
/// recorded between the pass and the claim reaches the judge, a judge that
/// refuses on it writes nothing, and a row expired by the claim's `now`
/// is not in force.
#[test]
fn the_claim_reads_the_rows_in_force_inside_its_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let entry = queue(&mut store, 1)[0];
    planted(&store, "run-b");
    let read = store.queue_entry(entry).unwrap();
    // The pass reads no row in force, at its own instant.
    assert_eq!(store.cooldowns_in_force(at(0)).unwrap(), vec![]);
    // A row is recorded after the pass read the entry, before its claim.
    append(
        &mut store,
        "run-b",
        &[named(
            "b",
            "first",
            at(60),
            "between the pass and the claim",
            1,
        )],
    )
    .unwrap();
    let judged = |store: &mut Store, now| {
        crate::test_support::holding(store, crate::Generation(1));
        let claiming = Claiming {
            entry: &read,
            high_water: 0,
            digest: "d",
            generation: crate::Generation(1),
            now,
        };
        let mut seen = Vec::new();
        let refused = store.queue_claim_with(&claiming, "r", |facts| {
            seen = facts.cooling.iter().map(|row| row.route.clone()).collect();
            if facts.cooling.iter().any(|row| row.route == "first") {
                Err("the route is cooling")
            } else {
                Ok(())
            }
        });
        (refused, seen)
    };
    let before = (store.queue_list().unwrap(), raw(&store.conn));
    let (refused, seen) = judged(&mut store, at(10));
    assert_eq!(refused.unwrap(), Err("the route is cooling"));
    assert_eq!(seen, ["first"]);
    assert_eq!((store.queue_list().unwrap(), raw(&store.conn)), before);
    // Past the row's end, the claim reads nothing in force and lands.
    let (refused, seen) = judged(&mut store, at(60));
    assert!(matches!(refused, Ok(Ok(_))), "{refused:?}");
    assert_eq!(seen, Vec::<String>::new());
}
