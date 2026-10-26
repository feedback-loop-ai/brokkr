//! `brokkr costs`' and `brokkr compare`'s heap budgets (decision 0089
//! ruling 9): each run opened, then its seat rows read at the head the
//! opening verified, and compare's trails, so the verbs hold the same live
//! heap however many events the journal holds beside the two runs they
//! read, each of 100 seat rows, and however many checkpoints no row reads
//! the runs hold. A readout's live heap is two shares,
//! Brokkr's under dhat and SQLite's from its own high-water. The budgets
//! live in `quality/heap-bytes.json`; dhat allows one profiler per process,
//! so this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;
#[path = "heap/measure.rs"]
mod measure;

use std::path::{Path, PathBuf};

use brokkr_cli::{compare_for_budget, costs_for_budget};
use brokkr_core::envelope::EventType;
use brokkr_store::test_support::{fleet, generate, plans_begun, Width};
use brokkr_store::{Opened, SeatRows, Store};
use measure::{peak_of, sqlite_memory};
use serde_json::{json, Value};

/// The clock the generated runs beside the two are stamped by.
const NOW: &str = "2026-10-20T00:00:00Z";

/// The two runs the verbs read.
const RUNS: [&str; 2] = ["costs-a", "costs-b"];

/// Each run's events in the journal of 10,000, so the two and the fleet's
/// generated runs beside them bring it to 10,000 events.
const EVENTS: usize = 4_000;

/// Each run's events in the journal of 1,000,000: its last effect's
/// history grows by checkpoints no seat row, trail or printed figure
/// reads, its seqs as wide as in the journal of 10,000, so its seat rows
/// and trail hold to the byte while every event of the run grows.
const LONG: usize = 5_000;

/// The effects each run asks for, each started once: its request's own
/// row and its attempt's, 100 seat rows a run.
const EFFECTS: usize = 50;

/// What every effect's events are but its turns: its phase entered, its
/// request, its start, its session's finish, its success and its
/// decision.
const EACH: usize = 6;

/// The events and their payloads of run `run`: started, then each effect
/// in its phase, asked of one of ten seats, started, its turns and its
/// session's cost checkpointed, succeeded and decided, then completed:
/// `length` events, mostly checkpoints, as a long run's journal is. Past
/// [`EVENTS`], each is a progress checkpoint of the last effect's attempt
/// before its session's finish.
fn asked(run: &str, length: usize) -> Vec<(EventType, Option<String>, Value)> {
    let turns = EVENTS - 2 - EACH * EFFECTS;
    let mut events = vec![(EventType::RunStarted, None, json!({"feature": run}))];
    for effect in 0..EFFECTS {
        let (id, attempt) = (format!("e{effect}"), Some(format!("a{effect}")));
        let ids = json!({"effect_id": id, "attempt_id": attempt});
        let checkpoint = |checkpoint: Value| {
            let mut payload = ids.clone();
            payload["checkpoint"] = checkpoint;
            (EventType::EffectCheckpointed, attempt.clone(), payload)
        };
        let seat = format!("seat-{}", effect % 10);
        events.push((EventType::PhaseEntered, None, json!({"phase": "work"})));
        let request = json!({"effect_id": id, "seat": seat, "phase": "work"});
        events.push((EventType::EffectRequested, None, request));
        events.push((EventType::EffectStarted, attempt.clone(), ids.clone()));
        let own = turns / EFFECTS + usize::from(effect < turns % EFFECTS);
        for turn in 1..=own {
            let said = json!({"step": "seat-turn", "turn": turn, "model": "claude-opus-5-5",
                              "tool": "Read", "target": "crates/brokkr-cli/src/compare.rs"});
            events.push(checkpoint(said));
        }
        if effect + 1 == EFFECTS {
            for _ in EVENTS..length {
                events.push(checkpoint(json!({"step": "seat-progress"})));
            }
        }
        let cost = 0.01 * (effect + 1) as f64;
        events.push(checkpoint(json!({"step": "claude-session-finished",
            "total_cost_usd": cost, "model": "claude-opus-5-5", "num_turns": own})));
        let mut done = ids.clone();
        done["result"] = json!({"result": "done"});
        events.push((EventType::EffectSucceeded, attempt.clone(), done));
        let next = if effect + 1 == EFFECTS {
            "done"
        } else {
            "work"
        };
        let decided = json!({"from": "work", "result": "done", "rule_id": "WORK-OK",
            "next": next, "severity": null, "inputs": {}, "problem": null});
        events.push((EventType::TransitionDecided, None, decided));
    }
    events.push((EventType::RunCompleted, None, json!({})));
    events
}

/// A journal under `dir` named `name` of `events` events: the fleet's
/// generated runs, then the two runs of `each` events appended by the
/// store's writer, which keeps their projections as it appends.
fn journal(dir: &Path, name: &str, events: usize, each: usize) -> PathBuf {
    let db = dir.join(name);
    let started = std::time::Instant::now();
    let mut store = Store::open(&db).unwrap();
    let beside = (events - RUNS.len() * each) / 2_000;
    generate(&store, &fleet(NOW, beside as u64, Width::Narrow)).unwrap();
    for run in RUNS {
        store.create_run(run, run, "bundle", &json!({})).unwrap();
        for (event_type, attempt, payload) in asked(run, each) {
            store
                .append_next(run, event_type, payload, None, attempt)
                .unwrap();
        }
    }
    eprintln!("{name}: {events} events in {:?}", started.elapsed());
    db
}

/// The median of eleven timed reads of `db`, its openings included, each
/// printing `prints`, in microseconds.
fn wall(db: &Path, read: Read, prints: &[String]) -> u128 {
    let mut times = [0; 11];
    for time in &mut times {
        let started = std::time::Instant::now();
        assert_eq!(opened_and_read(db, read), prints);
        *time = started.elapsed().as_micros();
    }
    times.sort_unstable();
    times[times.len() / 2]
}

/// What a read cost: Brokkr's peak heap under dhat after the runs'
/// openings, to the bytes it prints, and SQLite's high-water above where
/// it stood before the journal's read-only open, the openings included.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cost {
    brokkr: u64,
    sqlite: u64,
}

/// A read of the two runs after their openings: the bytes it prints.
type Read = fn(&Store, &[Opened; 2]) -> Vec<String>;

/// `costs` of each run in `store`, opened at `opened`: the bytes each
/// prints.
fn costs(store: &Store, opened: &[Opened; 2]) -> Vec<String> {
    let runs = RUNS.iter().zip(opened);
    let printed = runs.map(|(run, opened)| costs_for_budget(store, run, opened).unwrap());
    printed.collect()
}

/// `compare` of the two runs in `store`, opened at `opened`: the bytes it
/// prints.
fn compare(store: &Store, [a, b]: &[Opened; 2]) -> Vec<String> {
    vec![compare_for_budget(store, [(RUNS[0], a), (RUNS[1], b)]).unwrap()]
}

/// `read` of the journal at `db`, from its read-only open and the two
/// runs' openings: the bytes it prints.
fn opened_and_read(db: &Path, read: Read) -> Vec<String> {
    let store = Store::open_read_only(db).unwrap();
    read(&store, &RUNS.map(|run| store.open_run(run).unwrap()))
}

/// What `read` of `db` cost, holding that it prints `prints`.
fn measured(db: &Path, read: Read, prints: &[String]) -> Cost {
    let (base, _) = sqlite_memory(true);
    let store = Store::open_read_only(db).unwrap();
    let opened = RUNS.map(|run| store.open_run(run).unwrap());
    let mut printed = None;
    let brokkr = peak_of(|| printed = Some(read(&store, &opened)));
    let (_, high) = sqlite_memory(false);
    assert_eq!(printed.as_deref(), Some(prints), "what the read prints");
    Cost {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// The plans of the statements a run's opening and its reads by `costs`
/// and `compare` begin in `db`, each step once, in the order first taken.
fn plans(db: &Path) -> Vec<String> {
    let mut store = Store::open_read_only(db).unwrap();
    let ((), begun) = plans_begun(&mut store, |store| {
        let opened = store.open_run(RUNS[0]).unwrap();
        let seats = store.seat_rows_at(RUNS[0], &opened).unwrap();
        let SeatRows::Current(rows) = seats else {
            panic!("the run's seat rows stand at its opening");
        };
        assert_eq!(rows.len(), 2 * EFFECTS, "the run's seat rows");
        store.run_read(RUNS[0], &opened, opened.seq).unwrap();
    });
    let mut plans = Vec::new();
    for step in begun {
        if !plans.contains(&step) {
            plans.push(step);
        }
    }
    plans
}

/// The plan of every statement a run's opening and its reads begin: each
/// by its key, but the schema's own checks of the projections' tables.
const PLANS: [&str; 11] = [
    "SEARCH events USING INDEX sqlite_autoindex_events_1 (run_id=?)",
    "SCAN sqlite_master",
    "SCAN pragma_table_info VIRTUAL TABLE INDEX 0:",
    "SEARCH meta USING INDEX sqlite_autoindex_meta_1 (key=?)",
    "SEARCH seat_summary USING PRIMARY KEY (run_id=?)",
    "SEARCH h USING PRIMARY KEY (run_id=?)",
    "SEARCH r USING COVERING INDEX sqlite_autoindex_runs_1 (run_id=?)",
    "SEARCH s USING PRIMARY KEY (run_id=?) LEFT-JOIN",
    "SEARCH run_summary USING INDEX sqlite_autoindex_run_summary_1 (run_id=?)",
    "SEARCH event_index USING INDEX event_index_trail (run_id=?)",
    "SEARCH events USING INDEX sqlite_autoindex_events_1 (run_id=? AND seq=?)",
];

/// Ruling 9 on `costs` and `compare`: after their openings, Brokkr's peak
/// for `costs` of each of two runs of 100 seat rows, to the bytes each
/// prints, is the same to the byte in a journal of 10,000 events and one
/// of 1,000,000, whose two runs each hold 1,000 checkpoints more that no
/// seat row, trail or printed figure reads, and within its budget and
/// ruling 9's 256 KiB; `compare`'s
/// of the two is the same in both journals and within its own budget,
/// ruling 9's row as amended on 2026-10-26: the runs' seat rows as read,
/// their trails and the views composed of them, bounded by the runs'
/// seats; SQLite's for each, from the journal's open, the openings
/// included, within its cap. Each run's seat rows and trail are read by
/// their keys. The wall times in both journals, their openings included,
/// are reported against ruling 9's 50 ms, not gated.
#[test]
fn costs_and_compare_hold_their_openings_and_the_runs_seats() {
    let dir = tempfile::tempdir().unwrap();
    for each in [EVENTS, LONG] {
        assert_eq!(asked(RUNS[0], each).len(), each);
    }
    let small_db = journal(dir.path(), "small.db", 10_000, EVENTS);
    let large_db = journal(dir.path(), "large.db", 1_000_000, LONG);
    for (db, events) in [(&small_db, 10_000), (&large_db, 1_000_000)] {
        let conn = rusqlite::Connection::open(db).unwrap();
        let count = "SELECT count(*) FROM events";
        assert_eq!(conn.query_row(count, [], |row| row.get(0)), Ok(events));
    }
    // Each read once unmeasured, so what a process allocates once and
    // keeps is allocated before any read is measured; and what it prints
    // of that journal, whose writer stamped it.
    let measure = |db: &Path| {
        let (printed, compared) = (opened_and_read(db, costs), opened_and_read(db, compare));
        let costs = measured(db, costs, &printed);
        (costs, measured(db, compare, &compared))
    };
    let ((small, small_compare), (large, large_compare)) = (measure(&small_db), measure(&large_db));
    eprintln!("costs: small {small:?}, large {large:?}; compare: small {small_compare:?}, large {large_compare:?}");
    // The records scripts/measure-budgets.sh reads, each a budget.
    println!("heap\tcosts\t{}", small.brokkr.max(large.brokkr));
    println!("heap\tcosts-sqlite\t{}", small.sqlite.max(large.sqlite));
    println!(
        "heap\tcompare\t{}",
        small_compare.brokkr.max(large_compare.brokkr)
    );
    println!(
        "heap\tcompare-sqlite\t{}",
        small_compare.sqlite.max(large_compare.sqlite)
    );
    for (name, db) in [("10k", &small_db), ("1m", &large_db)] {
        let (printed, compared) = (opened_and_read(db, costs), opened_and_read(db, compare));
        println!("wall\tcosts-{name}\t{}", wall(db, costs, &printed));
        println!("wall\tcompare-{name}\t{}", wall(db, compare, &compared));
    }
    for db in [&small_db, &large_db] {
        assert_eq!(plans(db), PLANS, "{db:?}");
    }
    assert_eq!(
        large.brokkr, small.brokkr,
        "costs: 1,000,000 events against 10,000"
    );
    assert_eq!(
        large_compare.brokkr, small_compare.brokkr,
        "compare: 1,000,000 events against 10,000"
    );
    let budget = |name: &str| budgets::budget_named(name);
    let costs = budget("costs");
    assert!(
        costs <= 256 * 1024,
        "the budget {costs} is past ruling 9's 256 KiB"
    );
    let pairs = [
        ("costs", small, large),
        ("compare", small_compare, large_compare),
    ];
    for (name, small, large) in pairs {
        let (brokkr, sqlite) = (budget(name), budget(&format!("{name}-sqlite")));
        let cap = measure::SQLITE_CAP;
        assert_eq!(
            sqlite, cap,
            "{name}: SQLite's budget is the fleet's cap on every host"
        );
        for (size, cost) in [("10,000", small), ("1,000,000", large)] {
            let within = cost.brokkr <= brokkr && cost.sqlite <= sqlite;
            assert!(
                within,
                "{name}, {size}: {cost:?}; the budgets are {brokkr} and {sqlite}"
            );
        }
    }
}
