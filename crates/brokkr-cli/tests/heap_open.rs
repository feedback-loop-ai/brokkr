//! Opening a run's heap budgets (decision 0089 ruling 9): a readout of one
//! run opens it first, its whole chain read and verified one envelope at a
//! time, so the opening holds the same live heap however many events the
//! journal holds beside the run, a small part of what the run holds, and
//! grows only by what one envelope costs to verify when one is wide. A
//! readout's live heap is two shares, Brokkr's under dhat and SQLite's from
//! its own high-water. The budgets live in `quality/heap-bytes.json`; dhat
//! allows one profiler per process, so this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;
#[path = "heap/grown.rs"]
mod grown;
#[path = "heap/measure.rs"]
mod measure;

use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::test_support::{self, Generated, Width};
use brokkr_store::Store;
use measure::{peak_of, sqlite_memory};

/// The clock the generated runs beside the opened one are stamped by.
const NOW: &str = "2026-10-20T00:00:00Z";

/// How long the stretched envelope's text is: a mebibyte.
const STRETCH: usize = 1 << 20;

/// The seq of the envelope stretched: the run's middle.
const STRETCHED: usize = grown::EVENTS / 2;

/// The generated runs that bring a journal holding the grown run to
/// `events` events: the fleet's 2,000, the remainder on the first.
fn beside(events: usize) -> Vec<Generated> {
    let others = events - grown::EVENTS;
    let mut runs = test_support::fleet(NOW, (others / 2_000) as u64, Width::Narrow);
    runs[0].events += (others % 2_000) as u64;
    runs
}

/// The grown run, chained, its middle envelope's text stretched to
/// exactly [`STRETCH`] bytes when `stretched`.
fn opened_run(stretched: bool) -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    if stretched {
        let event = &mut events[STRETCHED - 1];
        event.payload["stretch"] = "".into();
        let text = serde_json::to_string(event).unwrap().len();
        event.payload["stretch"] = "s".repeat(STRETCH - text).into();
    }
    let events = grown::chained(events);
    let text = serde_json::to_string(&events[STRETCHED - 1]).unwrap().len();
    assert_eq!(text == STRETCH, stretched, "the stretched envelope's text");
    events
}

/// A journal under `dir` named `name` of `events` events: the grown run,
/// stretched or not, and the generated runs beside it, written in
/// seconds.
fn journal(dir: &Path, name: &str, events: usize, stretched: bool) -> PathBuf {
    let db = dir.join(name);
    let started = std::time::Instant::now();
    let store = Store::open(&db).unwrap();
    test_support::generate(&store, &beside(events)).unwrap();
    drop(store);
    grown::planted(&db, &opened_run(stretched));
    eprintln!("{name}: {events} events in {:?}", started.elapsed());
    db
}

/// What opening the grown run cost, from its read-only open to its
/// verified head: Brokkr's peak heap under dhat, and SQLite's high-water
/// above where it stood before the open.
#[derive(Debug, Clone, Copy)]
struct Cost {
    brokkr: u64,
    sqlite: u64,
}

/// The grown run in `db` opened whole, and what that cost.
fn opened(db: &Path) -> Cost {
    let (base, _) = sqlite_memory(true);
    let mut head = None;
    let brokkr = peak_of(|| {
        let store = Store::open_read_only(db).unwrap();
        head = Some(store.open_run(grown::RUN).unwrap().seq);
    });
    let (_, high) = sqlite_memory(false);
    assert_eq!(head, Some(grown::EVENTS as u64), "the whole run is opened");
    Cost {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// The median of eleven timed reads, in microseconds.
fn median_micros(mut read: impl FnMut()) -> u128 {
    let mut times: Vec<u128> = (0..11)
        .map(|_| {
            let started = std::time::Instant::now();
            read();
            started.elapsed().as_micros()
        })
        .collect();
    times.sort_unstable();
    times[times.len() / 2]
}

/// How many times its own length one envelope costs Brokkr to verify, at
/// most: its parse, and in brokkr-core's `compute_hash` its copy into the
/// `Value` the hash is taken over and that value's canonical bytes, whose
/// buffer serde_json grows to twice the envelope. 4,166,518 B were
/// measured for a 1,048,576 B envelope (2026-10-24).
const COPIES: u64 = 4;

/// Ruling 9 on opening a run: Brokkr's peak to open the grown run of
/// 7,510 events is the same to the byte in a journal of 10,000 events and
/// one of 1,000,000, and within its budget, which holds no more than a
/// tenth of the run's envelopes: the opening holds one envelope at a time,
/// never the run. With one envelope stretched to a mebibyte, Brokkr's peak
/// grows by what that one envelope costs to verify and no more, and
/// SQLite's, which holds the row it steps to, by that envelope; each
/// stays within its budget besides. Ruling 9's cap for Brokkr's share is
/// 256 KiB.
#[test]
fn opening_a_run_holds_one_envelope_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let small_db = journal(dir.path(), "small.db", 10_000, false);
    let large_db = journal(dir.path(), "large.db", 1_000_000, false);
    let wide_db = journal(dir.path(), "wide.db", 10_000, true);
    // Read once unmeasured, so what a process allocates once and keeps
    // is allocated before any opening is measured.
    opened(&small_db);
    let (small, large, wide) = (opened(&small_db), opened(&large_db), opened(&wide_db));
    eprintln!("small: {small:?}; large: {large:?}; wide: {wide:?}");
    let run_bytes: u64 = opened_run(false)
        .iter()
        .map(|event| serde_json::to_string(event).unwrap().len() as u64)
        .sum();
    eprintln!("the run's envelopes: {run_bytes} bytes");
    // The records scripts/measure-budgets.sh reads, each a budget.
    println!("heap\topen-run\t{}", small.brokkr.max(large.brokkr));
    println!("heap\topen-run-sqlite\t{}", small.sqlite.max(large.sqlite));
    let store = Store::open_read_only(&small_db).unwrap();
    let open = median_micros(|| drop(store.open_run(grown::RUN).unwrap()));
    let load = median_micros(|| drop(store.load(grown::RUN).unwrap()));
    println!("wall\topen-run-7510\t{open}");
    println!("wall\tload-run-7510\t{load}");
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let (brokkr, sqlite) = (
        budgets::budget_named("open-run"),
        budgets::budget_named("open-run-sqlite"),
    );
    assert!(
        brokkr <= 256 * 1024,
        "the budget {brokkr} is past ruling 9's 256 KiB"
    );
    assert!(
        brokkr <= run_bytes / 10,
        "the budget {brokkr} is past a tenth of the run's {run_bytes} bytes"
    );
    for (name, cost) in [("10,000", small), ("1,000,000", large)] {
        assert!(
            cost.brokkr <= brokkr,
            "{name}: {} bytes; the budget is {brokkr}",
            cost.brokkr
        );
        assert!(
            cost.sqlite <= sqlite,
            "{name}: SQLite's {} bytes; the budget is {sqlite}",
            cost.sqlite
        );
    }
    let stretch = STRETCH as u64;
    let growth = wide.brokkr - small.brokkr;
    assert!(
        (stretch..=COPIES * stretch).contains(&growth),
        "a mebibyte envelope grew the opening's peak by {growth} bytes"
    );
    assert!(
        wide.brokkr <= brokkr + COPIES * stretch,
        "{} bytes with one envelope a mebibyte; the budget is {brokkr} and the envelope",
        wide.brokkr
    );
    let held = wide.sqlite - small.sqlite;
    assert!(
        held <= stretch,
        "a mebibyte envelope grew SQLite's high-water by {held} bytes"
    );
    assert!(
        wide.sqlite <= sqlite + stretch,
        "SQLite's {} bytes with one envelope a mebibyte; the budget is {sqlite} and the envelope",
        wide.sqlite
    );
}
