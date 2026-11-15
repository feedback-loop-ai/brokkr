//! Every window statement leaves out what the whole read takes (decision
//! 0089 ruling 7, slice 4's unit U5b1): one oracle over every lane and lag
//! shape, writable and read-only, and each statement's VM steps at two
//! journal sizes.

use std::collections::BTreeSet;

use brokkr_core::canonical::ZERO_HASH;
use rusqlite::{params, Connection};

use super::*;
use crate::metered::SiteOrder;
use crate::seat_summary::SHAPE;
use crate::test_support::plant_run;

/// How a run's seat head stands against its summary row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seat {
    Current,
    Headless,
    SeqLag,
    ShapeLag,
}

/// The lane a planted run's row reads from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lane {
    Current,
    Behind,
    Refused,
    Acknowledged,
}

/// The damage a planted run carries besides its seat head.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Damage {
    None,
    CheckpointStale,
    OpenPark,
}

/// One planted run and every fact a window statement could read of it.
struct Planted {
    id: &'static str,
    realm: Option<&'static str>,
    finished: Option<i64>,
    ending: Option<&'static str>,
    status: &'static str,
    lane: Lane,
    seat: Seat,
    damage: Damage,
}

/// How a planted run ended, or that it is still open.
#[derive(Clone, Copy)]
struct Life {
    finished: Option<i64>,
    ending: Option<&'static str>,
    status: &'static str,
}

/// One planted run, spelled once so the journal's list stays short.
fn planted(
    id: &'static str,
    realm: Option<&'static str>,
    life: Life,
    lane: Lane,
    seat: Seat,
    damage: Damage,
) -> Planted {
    Planted {
        id,
        realm,
        finished: life.finished,
        ending: life.ending,
        status: life.status,
        lane,
        seat,
        damage,
    }
}

/// The journal the oracle reads: a current finished run and an open one,
/// every lag shape, and the behind, refused and acknowledged lanes. Each
/// run carries a site, a cost and a park inside the window, so a statement
/// that failed to leave it out would name it. `open-stale` is a stale run
/// still open with an unanswered class-64 park, and `checkpoint-stale` is
/// current in its seat head but stale in its checkpoint facts.
fn journal() -> Vec<Planted> {
    let finished = Some(FROM + 30_000);
    let done = Life {
        finished,
        ending: Some("shipped"),
        status: "completed",
    };
    let open = Life {
        finished: None,
        ending: None,
        status: "running",
    };
    let parked = Life {
        finished: None,
        ending: None,
        status: "awaiting_operator",
    };
    vec![
        planted(
            "current-finished",
            Some("alpha"),
            done,
            Lane::Current,
            Seat::Current,
            Damage::None,
        ),
        planted(
            "current-open",
            Some("alpha"),
            open,
            Lane::Current,
            Seat::Current,
            Damage::None,
        ),
        planted(
            "headless",
            Some("alpha"),
            done,
            Lane::Current,
            Seat::Headless,
            Damage::None,
        ),
        planted(
            "seq-lag",
            Some("beta"),
            done,
            Lane::Current,
            Seat::SeqLag,
            Damage::None,
        ),
        planted(
            "shape-lag",
            Some("beta"),
            done,
            Lane::Current,
            Seat::ShapeLag,
            Damage::None,
        ),
        planted(
            "behind",
            Some("alpha"),
            done,
            Lane::Behind,
            Seat::Current,
            Damage::None,
        ),
        planted(
            "open-stale",
            Some("beta"),
            parked,
            Lane::Current,
            Seat::Headless,
            Damage::OpenPark,
        ),
        planted(
            "checkpoint-stale",
            Some("alpha"),
            done,
            Lane::Current,
            Seat::Current,
            Damage::CheckpointStale,
        ),
        planted(
            "refused",
            Some("alpha"),
            done,
            Lane::Refused,
            Seat::Current,
            Damage::None,
        ),
        planted(
            "acknowledged",
            Some("alpha"),
            done,
            Lane::Acknowledged,
            Seat::Current,
            Damage::None,
        ),
    ]
}

/// Plant one run's summary row, seat head and in-window facts.
fn plant(store: &Store, run: &Planted) {
    plant_run(&store.conn, run.id, "{}").unwrap();
    let refusal = matches!(run.lane, Lane::Refused | Lane::Acknowledged);
    let acknowledged = run.lane == Lane::Acknowledged;
    let stamp = if run.lane == Lane::Behind { 1 } else { 0 };
    store
        .conn
        .execute(
            "UPDATE run_summary SET realm = ?2, status = ?3, started_ms = ?4, finished_ms = ?4,
                ending = ?5, folded_seq = 0, folded_hash = ?6, aggregated_seq = ?7,
                stop_folded_seq = ?7, ending_folded_seq = ?7, late_folded_seq = ?7,
                implement_visits = 0, review_first = 0, review_second = 0, review_back = 0,
                refusal_words = CASE WHEN ?8 THEN 'stored refusal' END, acknowledged = ?9
              WHERE run_id = ?1",
            params![
                run.id,
                run.realm,
                run.status,
                run.finished,
                run.ending,
                ZERO_HASH,
                stamp,
                refusal,
                acknowledged
            ],
        )
        .unwrap();
    if run.seat != Seat::Headless {
        let folded = if run.seat == Seat::SeqLag { -1 } else { 0 };
        let shape = if run.seat == Seat::ShapeLag {
            "another-build"
        } else {
            SHAPE.as_str()
        };
        store
            .conn
            .execute(
                "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash, shape)
                 VALUES (?1, ?2, ?3, ?4)",
                params![run.id, folded, ZERO_HASH, shape],
            )
            .unwrap();
    }
    store
        .conn
        .execute(
            "INSERT INTO seat_summary
                (run_id, opened, attempt_id, member, started_seq, started_ms,
                 phase_key, provider_key, model_key, metering, site, digest)
             VALUES (?1, 0, 'a', NULL, 1, ?2, 'implement', 'alpha', 'm', 1, 0, 'd')",
            params![run.id, FROM + 10],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO event_index
                (run_id, seq, type, attempt_id, at_ms, reported, turn, session, repeat,
                 input_tokens, output_tokens, usd, facts, facts_seq)
             VALUES (?1, 2, 'effect/checkpointed', 'a', ?2, 1, 1, 0, 0, 1, 0, 0.5, 1, 2)",
            params![run.id, FROM + 20],
        )
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = CASE WHEN ?2 THEN NULL ELSE 2 END
              WHERE run_id = ?1 AND seq = 2",
            params![run.id, run.damage == Damage::CheckpointStale],
        )
        .unwrap();
    let park = if run.damage == Damage::OpenPark {
        ParkRow {
            seq: 3,
            phase: "review",
            at: FROM + 1_000,
            answered: None,
            ended: None,
            class: 64,
        }
    } else {
        ParkRow {
            seq: 3,
            phase: "review",
            at: FROM + 1_000,
            answered: Some(FROM + 2_000),
            ended: Some(FROM + 2_000),
            class: 10,
        }
    };
    plant_park(&store.conn, run.id, park);
}

/// The set a list of run ids names.
fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// The runs the whole read takes: `behind_runs` union `window_fallbacks`.
fn whole(store: &Store) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    store
        .behind_runs(|run_id, _created_at| {
            found.insert(run_id.to_string());
        })
        .unwrap();
    store
        .window_fallbacks(FROM, TO, None, |run| {
            found.insert(run.run_id.to_string());
        })
        .unwrap();
    found
}

/// The run ids one window statement hands back.
fn statement_ids(store: &Store, which: Statement) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    match which {
        Statement::Runs(order) => store
            .window_runs(FROM, TO, order, None, |row| {
                found.insert(row.run_id.to_string());
            })
            .unwrap(),
        Statement::Parks => store
            .window_parks(FROM, TO, TO, None, |park| {
                found.insert(park.run_id.to_string());
            })
            .unwrap(),
        Statement::Spends => store
            .window_spends(FROM, TO, None, |spend| {
                found.insert(spend.run_id.to_string());
            })
            .unwrap(),
        Statement::Sites => store
            .window_sites(FROM, TO, None, SiteOrder::Model, |site| {
                found.insert(site.run_id.to_string());
            })
            .unwrap(),
    }
    found
}

/// One window statement the oracle reads.
#[derive(Clone, Copy)]
enum Statement {
    Runs(RunOrder),
    Parks,
    Spends,
    Sites,
}

impl Statement {
    fn label(self) -> &'static str {
        match self {
            Statement::Runs(_) => "runs",
            Statement::Parks => "parks",
            Statement::Spends => "spends",
            Statement::Sites => "sites",
        }
    }
}

/// Every statement's rows, and the runs the whole read takes, partition
/// the journal: a statement names no run the whole read takes, and a
/// current run each statement should serve is served.
fn holds(store: &Store, what: &str) {
    let whole = whole(store);
    assert_eq!(
        whole,
        set(&[
            "behind",
            "headless",
            "seq-lag",
            "shape-lag",
            "open-stale",
            "checkpoint-stale",
        ]),
        "{what}: the whole read's runs"
    );
    let live = set(&["current-finished", "current-open"]);
    for order in [
        RunOrder::Unordered,
        RunOrder::ByEndingRule,
        RunOrder::ByRealmThenRecipe,
    ] {
        let named = statement_ids(store, Statement::Runs(order));
        assert_eq!(named, live, "{what}: runs {order:?}");
        assert!(named.is_disjoint(&whole), "{what}: runs {order:?}");
    }
    for which in [Statement::Parks, Statement::Spends, Statement::Sites] {
        let named = statement_ids(store, which);
        assert_eq!(named, live, "{what}: {}", which.label());
        assert!(named.is_disjoint(&whole), "{what}: {}", which.label());
    }
    // The median's pass and the preceding count measure the finished runs
    // alone, and neither names a whole-read run.
    let mut spans = Vec::new();
    store
        .durations(FROM, TO, |realm, span| {
            spans.push((realm.map(str::to_string), span))
        })
        .unwrap();
    assert_eq!(spans, [(Some("alpha".to_string()), 0)], "{what}: durations");
    let mut groups = Vec::new();
    store
        .finished_between(FROM, TO, |realm, count| {
            groups.push((realm.map(str::to_string), count));
        })
        .unwrap();
    assert_eq!(
        groups,
        [(Some("alpha".to_string()), 1)],
        "{what}: finished_between"
    );
}

/// Build the journal, read it writable while the set is intact, then read
/// it read-only after the set is dropped, so both arms answer as the whole
/// read does.
#[test]
fn every_statement_leaves_out_the_whole_reads_runs() {
    let (dir, store) = store();
    for run in journal() {
        plant(&store, &run);
    }
    assert!(crate::seat_summary::stale_intact(&store.conn).unwrap());
    holds(&store, "intact");
    let path = dir.path().join("forge.db");
    store.conn.execute_batch("DROP TABLE seat_lag;").unwrap();
    drop(store);

    let reader = Store::open_read_only(&path).unwrap();
    assert!(
        !crate::seat_summary::stale_intact(&reader.conn).unwrap(),
        "the set is not intact, so the rule's arm is read"
    );
    holds(&reader, "by rule");
}

/// Plant a finished run at `at`, its stamps current and its realm alpha;
/// `current` writes the seat head that keeps it out of the whole read.
fn plant_finished(store: &Store, run: &str, at: i64, current: bool) {
    plant_run(&store.conn, run, "{}").unwrap();
    store
        .conn
        .execute(
            "UPDATE run_summary SET realm = 'alpha', status = 'completed',
                started_ms = ?2, finished_ms = ?2, ending = 'shipped', folded_seq = 0,
                folded_hash = ?3, aggregated_seq = 0, stop_folded_seq = 0,
                ending_folded_seq = 0, late_folded_seq = 0
              WHERE run_id = ?1",
            params![run, at, ZERO_HASH],
        )
        .unwrap();
    if current {
        store
            .conn
            .execute(
                "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash, shape)
                 VALUES (?1, 0, ?2, ?3)",
                params![run, ZERO_HASH, SHAPE.as_str()],
            )
            .unwrap();
    }
}

/// Plant `count` current runs finished at `at`, each with a matching seat
/// head, so none is in the whole read and none is served.
fn plant_current(store: &Store, first: usize, count: usize, at: i64) {
    for index in first..first + count {
        plant_finished(store, &format!("current-{index}"), at, true);
    }
}

/// Plant ten stale runs inside the window: no seat head, so the set holds
/// them and both arms leave them out, while each statement still reads the
/// one served run and evaluates its exclusion.
fn plant_stale(store: &Store) {
    for index in 0..10 {
        plant_finished(store, &format!("stale-{index}"), FROM + 30_000, false);
    }
}

/// Every statement's VM steps, in the order the bound names them.
fn measures(store: &Store) -> [i64; 6] {
    [
        store
            .window_runs_steps(FROM, TO, RunOrder::Unordered, None)
            .unwrap(),
        store.window_parks_steps(FROM, TO, TO, None).unwrap(),
        store.window_spends_steps(FROM, TO, None).unwrap(),
        store
            .window_sites_steps(FROM, TO, None, SiteOrder::Model)
            .unwrap(),
        store.durations_steps(FROM, TO).unwrap(),
        store.finished_between_steps(FROM, TO).unwrap(),
    ]
}

/// Open a journal whose set is gone for reading, so its statements take
/// the rule's arm, and measure them.
fn by_rule(path: &std::path::Path) -> [i64; 6] {
    let damaged = Connection::open(path).unwrap();
    damaged.execute_batch("DROP TABLE seat_lag;").unwrap();
    drop(damaged);
    let reader = Store::open_read_only(path).unwrap();
    assert_eq!(
        whole(&reader).len(),
        10,
        "the stale runs are the whole read"
    );
    measures(&reader)
}

/// The intact-set exclusion does not grow with the current runs: 1,000
/// more current runs, outside the window and stale-free, leave every
/// statement's VM steps no larger. They may fall when the planner takes the
/// finished index at the larger size; the exclusion itself visits nothing
/// for a current run. The rule's arm reads the journal's `run_summary` to
/// find a lagging run, so its figures are reported at both sizes rather
/// than held.
#[test]
fn the_intact_exclusion_does_not_grow_with_the_current_runs() {
    let (dir, store) = store();
    plant(
        &store,
        &Planted {
            id: "served",
            realm: Some("alpha"),
            finished: Some(FROM + 30_000),
            ending: Some("shipped"),
            status: "completed",
            lane: Lane::Current,
            seat: Seat::Current,
            damage: Damage::None,
        },
    );
    plant_current(&store, 1, 1_000, FROM - DAY);
    plant_stale(&store);
    let intact_base = measures(&store);
    eprintln!("intact 1,000: {intact_base:?}");
    let path = dir.path().join("forge.db");
    drop(store);

    let rule_base = by_rule(&path);
    eprintln!("by rule 1,000: {rule_base:?}");

    let store = Store::open(&path).unwrap();
    plant_current(&store, 1_001, 1_000, FROM - DAY);
    let intact_grown = measures(&store);
    eprintln!("intact 2,000: {intact_grown:?}");
    drop(store);
    let rule_grown = by_rule(&path);
    eprintln!("by rule 2,000: {rule_grown:?}");

    for (at, (before, after)) in intact_base.iter().zip(&intact_grown).enumerate() {
        assert!(
            after <= before,
            "statement {at}: the intact exclusion grew with the current runs, {before} -> {after}"
        );
    }
    for (at, (before, after)) in rule_base.iter().zip(&rule_grown).enumerate() {
        eprintln!("statement {at}: by rule {before} -> {after}");
    }
}
