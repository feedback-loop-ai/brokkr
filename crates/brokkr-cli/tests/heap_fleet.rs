//! The fleet page's heap budgets (decision 0089 ruling 9): the console's
//! first page, the visible page and the one ahead, from its read-only open
//! to its painted frame, costs the same live heap however many events the
//! journal holds and however wide its runs' values are past their bounds.
//! A page's live heap is two shares, each read by its own counter: dhat
//! sees Rust's global allocator alone, and SQLite allocates beside it, so
//! SQLite's is read from its own high-water. The budgets live in
//! `quality/heap-bytes.json`; dhat allows one profiler per process, so
//! this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;

use std::path::{Path, PathBuf};

use brokkr_store::test_support::{fleet, generate, Generated, Width};
use brokkr_store::Store;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// The clock the page is read at.
const NOW: &str = "2026-10-20T00:00:00Z";

/// The terminal the page is painted into.
const FRAME: (u16, u16) = (80, 24);

/// The 2,000 runs of a journal of `events` a run, its widest rows each
/// value a mebibyte and ten thousand findings, or each cut to exactly its
/// bound, as `stretched` says.
fn runs(events: u64, stretched: Width) -> Vec<Generated> {
    fleet(NOW, events, stretched)
}

/// `runs` written into a fresh journal under `dir`, named `name`.
fn journal(dir: &Path, name: &str, runs: &[Generated]) -> PathBuf {
    let db = dir.join(name);
    let store = Store::open(&db).unwrap();
    generate(&store, runs).unwrap();
    db
}

/// SQLite's memory now, and its high-water since the last reset, which
/// `reset` starts again from now.
fn sqlite_memory(reset: bool) -> (i64, i64) {
    let (mut current, mut high) = (0i64, 0i64);
    // SAFETY: sqlite3_status64 writes the two counters through the two
    // pointers it is handed, each a live i64 on this stack, and reads
    // nothing else.
    let status = unsafe {
        rusqlite::ffi::sqlite3_status64(
            rusqlite::ffi::SQLITE_STATUS_MEMORY_USED,
            &mut current,
            &mut high,
            i32::from(reset),
        )
    };
    assert_eq!(status, rusqlite::ffi::SQLITE_OK);
    (current, high)
}

/// What one read of `db`'s first page cost: Brokkr's peak heap under dhat
/// to read and derive it, the same to read, derive and paint it, and
/// SQLite's high-water above where it stood before the open.
#[derive(Debug, Clone, Copy)]
struct Cost {
    page: u64,
    frame: u64,
    sqlite: u64,
}

/// Brokkr's peak heap while `read` runs, under dhat.
fn peak_of(read: impl FnOnce()) -> u64 {
    let profiler = dhat::Profiler::builder().testing().build();
    read();
    let peak = dhat::HeapStats::get().max_bytes as u64;
    drop(profiler);
    peak
}

/// The first page of `db`, read and derived, then read and painted.
fn measured(db: &Path) -> Cost {
    let (base, _) = sqlite_memory(true);
    let mut listed = 0;
    let page = peak_of(|| listed = brokkr_cli::fleet_page_for_budget(db, NOW).unwrap().count);
    let (_, high) = sqlite_memory(false);
    assert_eq!(listed, 100, "the visible page and the one ahead");
    let mut painted = String::new();
    let frame = peak_of(|| {
        painted = brokkr_cli::fleet_frame_for_budget(db, NOW, FRAME.0, FRAME.1).unwrap();
    });
    assert!(painted.contains("needs you"), "the page is painted");
    let sqlite = u64::try_from(high - base).unwrap();
    Cost {
        page,
        frame,
        sqlite,
    }
}

/// The wall time of `db`'s first page, open to painted frame, warm: the
/// least of five reads, in microseconds. Recorded by
/// scripts/measure-budgets.sh against ruling 9's 50 ms, and gated by no
/// check, since wall clock is the machine's.
fn first_page_micros(db: &Path) -> u128 {
    (0..5)
        .map(|_| {
            let started = std::time::Instant::now();
            brokkr_cli::fleet_frame_for_budget(db, NOW, FRAME.0, FRAME.1).unwrap();
            started.elapsed().as_micros()
        })
        .min()
        .unwrap()
}

/// Ruling 9: Brokkr's peak to read and derive a page is the same to the
/// byte at 10,000 and 1,000,000 events, and the same with its widest rows
/// stretched a mebibyte past their bounds as with each value cut to
/// exactly its bound, so the bound is what holds it; read and painted, it
/// holds within its budget; SQLite's high-water stays within its share in
/// every journal; and a page whose every row fills every bound holds
/// within its own budget. Painting goes through ratatui's layout solver,
/// whose hashing is seeded per process, so the painted peak varies by a
/// few kilobytes from one read to the next and is held to its budget,
/// not to the byte.
#[test]
fn a_fleet_page_costs_the_same_heap_at_every_journal_size() {
    let dir = tempfile::tempdir().unwrap();
    let small_db = journal(dir.path(), "small.db", &runs(5, Width::Wide));
    let large_db = journal(dir.path(), "large.db", &runs(500, Width::Wide));
    let bound_db = journal(dir.path(), "bound.db", &runs(5, Width::Bound));
    let every: Vec<Generated> = runs(5, Width::Bound)
        .into_iter()
        .map(|run| Generated {
            width: Width::Bound,
            ..run
        })
        .collect();
    let every_db = journal(dir.path(), "every.db", &every);
    // Read once unmeasured, so what a process allocates once and keeps
    // is allocated before any page is measured.
    measured(&small_db);
    let measures = [
        ("small", measured(&small_db)),
        ("large", measured(&large_db)),
        ("bound", measured(&bound_db)),
        ("every", measured(&every_db)),
    ];
    for (name, cost) in measures {
        let Cost {
            page,
            frame,
            sqlite,
        } = cost;
        eprintln!("{name}: page {page}, frame {frame}, sqlite {sqlite}");
    }
    let [small, large, bound, every] = measures.map(|(_, cost)| cost);
    // The records scripts/measure-budgets.sh reads, one a budget.
    let framed = [small, large, bound].map(|cost| cost.frame);
    let held = [small, large, bound, every].map(|cost| cost.sqlite);
    for (name, peak) in [
        ("fleet-page", small.page),
        ("fleet-page-at-every-bound", every.page),
        ("fleet-frame", framed.into_iter().max().unwrap()),
        ("fleet-frame-at-every-bound", every.frame),
        ("fleet-page-sqlite", held.into_iter().max().unwrap()),
    ] {
        println!("heap\t{name}\t{peak}");
    }
    for (name, db) in [("10k", &small_db), ("1m", &large_db)] {
        println!("wall\tfleet-first-page-{name}\t{}", first_page_micros(db));
    }
    assert_eq!(large.page, small.page, "1,000,000 events against 10,000");
    assert_eq!(
        bound.page, small.page,
        "a mebibyte past every bound against each at it"
    );
    let within = |name: &str, held: u64| {
        let budget = budgets::budget_named(name);
        assert!(
            held <= budget,
            "{name}: {held} bytes; the budget is {budget}"
        );
    };
    within("fleet-page", small.page);
    within("fleet-page-at-every-bound", every.page);
    for cost in [small, large, bound] {
        within("fleet-frame", cost.frame);
        within("fleet-page-sqlite", cost.sqlite);
    }
    within("fleet-frame-at-every-bound", every.frame);
    within("fleet-page-sqlite", every.sqlite);
}
