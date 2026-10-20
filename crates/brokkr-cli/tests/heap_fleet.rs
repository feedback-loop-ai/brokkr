//! The fleet page's heap budgets (decision 0089 ruling 9): the console's
//! window, the visible page and the one ahead, from its read-only open to
//! its rows, costs the same live heap however many events the journal
//! holds, however wide its runs' values are past their bounds and however
//! far the list has scrolled; painted, it costs ratatui's buffers besides.
//! A page's live heap is two shares, each read by its own counter: dhat
//! sees Rust's global allocator alone, and SQLite allocates beside it, so
//! SQLite's is read from its own high-water. The budgets live in
//! `quality/heap-bytes.json`; dhat allows one profiler per process, so
//! this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;

use std::path::{Path, PathBuf};

use brokkr_cli::{fleet_window_for_budget, FleetScroll};
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

/// What reading `db`'s fleet cost: Brokkr's peak heap under dhat to read
/// and derive the console's first window, the page there and the one
/// ahead; the same scrolled [`DEEP`] pages down, and twice as deep with
/// `a` pressed; the least of [`DRAWS`] reads of the first window read,
/// derived and painted at 80x24; and SQLite's high-water above where it
/// stood before each window's open, the most of the three.
#[derive(Debug, Clone, Copy)]
struct Cost {
    page: u64,
    deep: u64,
    deeper: u64,
    frame: u64,
    sqlite: u64,
}

impl Cost {
    /// The most any window held to read and derive.
    fn read(self) -> u64 {
        self.page.max(self.deep).max(self.deeper)
    }
}

/// How many pages down the list is scrolled when its deep window is read:
/// past the eight pages the console keeps the keys of, so the deep window
/// and the one twice as deep hold as many keys.
const DEEP: usize = 12;

/// How many times the painted frame is read; its peak is the least.
const DRAWS: usize = 8;

/// How far one painted frame's peak moves from one draw to the next in a
/// process: ratatui's layout solver hashes with a seed drawn anew each
/// solve, and its allocations move with it. 12,084 B were measured over
/// 48 draws of one frame, 513,210 to 525,294 B (2026-10-20).
const SOLVER_SPREAD: u64 = 16 * 1024;

/// Brokkr's peak heap while `read` runs, under dhat.
fn peak_of(read: impl FnOnce()) -> u64 {
    let profiler = dhat::Profiler::builder().testing().build();
    read();
    let peak = dhat::HeapStats::get().max_bytes as u64;
    drop(profiler);
    peak
}

/// The console's list over `db` scrolled `pages` pages down a frame at a
/// time, each by the key past its window's first page, `a` pressed or not.
fn scrolled(db: &Path, pages: usize, all: bool) -> FleetScroll {
    let mut scroll = FleetScroll {
        all,
        ..FleetScroll::default()
    };
    for _ in 0..pages {
        let window = fleet_window_for_budget(&scroll);
        let (_, ahead) = brokkr_cli::fleet_page_for_budget(db, NOW, &window).unwrap();
        scroll
            .scrolled
            .push(ahead.expect("the fleet runs on past the page"));
    }
    scroll
}

/// Brokkr's peak heap to read and derive the console's window over `db`
/// as `scroll` stands, which lists the page there and the one ahead, with
/// the keys the console keeps to step back by held while it reads, and
/// SQLite's high-water above where it stood before the window's open.
fn window_cost(db: &Path, scroll: &FleetScroll) -> (u64, u64) {
    let (base, _) = sqlite_memory(true);
    let mut listed = 0;
    let peak = peak_of(|| {
        let list = fleet_window_for_budget(scroll);
        let (runs, _) = brokkr_cli::fleet_page_for_budget(db, NOW, &list).unwrap();
        listed = runs.count;
    });
    let (_, high) = sqlite_memory(false);
    assert_eq!(listed, 100, "the visible page and the one ahead");
    (peak, u64::try_from(high - base).unwrap())
}

/// The first window of `db`, read and derived, then the window scrolled
/// deep and twice as deep with `a`, then the first read and painted.
fn measured(db: &Path) -> Cost {
    let (page, first) = window_cost(db, &FleetScroll::default());
    let (deep, scrolled_deep) = window_cost(db, &scrolled(db, DEEP, false));
    let (deeper, every_run) = window_cost(db, &scrolled(db, 2 * DEEP, true));
    let mut painted = String::new();
    let frame = (0..DRAWS)
        .map(|_| {
            peak_of(|| {
                painted = brokkr_cli::fleet_frame_for_budget(db, NOW, FRAME.0, FRAME.1).unwrap();
            })
        })
        .min()
        .unwrap();
    assert!(painted.contains("needs you"), "the page is painted");
    Cost {
        page,
        deep,
        deeper,
        frame,
        sqlite: first.max(scrolled_deep).max(every_run),
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
/// exactly its bound, so the bound is what holds it. The console's window
/// scrolled deep, with the keys the console keeps to step back by, holds
/// the same to the byte as one twice as deep with `a` pressed, at both
/// sizes: it reads by keyset and keeps the keys of a bounded number of
/// pages, so neither scrolling nor `a` grows what the console holds to
/// read a frame (ruling 6). Every window holds within the
/// budget ruling 9 caps at 256 KiB, one whose every row fills every bound
/// within 640 KiB, and SQLite's high-water within 400 KiB; each budget
/// sits at or under its cap. The painted frame, which adds ratatui's
/// buffers at 80x24 to the read, is a budget of its own, and does not
/// move between the two journals by more than the layout solver moves one
/// frame from draw to draw.
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
        eprintln!("{name}: {cost:?}");
    }
    let [small, large, bound, every] = measures.map(|(_, cost)| cost);
    // The records scripts/measure-budgets.sh reads, one a budget.
    let read = [small, large, bound].map(Cost::read);
    let framed = [small, large, bound].map(|cost| cost.frame);
    let held = [small, large, bound, every].map(|cost| cost.sqlite);
    for (name, peak) in [
        ("fleet-page", read.into_iter().max().unwrap()),
        ("fleet-page-at-every-bound", every.read()),
        ("fleet-painted-frame", framed.into_iter().max().unwrap()),
        ("fleet-painted-frame-at-every-bound", every.frame),
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
    for cost in [small, large] {
        assert_eq!(cost.deeper, cost.deep, "twice as deep, with `a`");
    }
    assert_eq!(
        large.deep, small.deep,
        "deep, 1,000,000 events against 10,000"
    );
    let moved = large.frame.abs_diff(small.frame);
    assert!(
        moved <= SOLVER_SPREAD,
        "the painted frame moved {moved} bytes from 10,000 events to 1,000,000"
    );
    let within = |name: &str, held: u64| {
        let budget = budgets::budget_named(name);
        assert!(
            held <= budget,
            "{name}: {held} bytes; the budget is {budget}"
        );
    };
    for cost in [small, large, bound] {
        within("fleet-page", cost.read());
        within("fleet-painted-frame", cost.frame);
        within("fleet-page-sqlite", cost.sqlite);
    }
    within("fleet-page-at-every-bound", every.read());
    within("fleet-painted-frame-at-every-bound", every.frame);
    within("fleet-page-sqlite", every.sqlite);
    // Ruling 9's caps, which no measurement raises.
    for (name, cap) in [
        ("fleet-page", 256 * 1024),
        ("fleet-page-at-every-bound", 640 * 1024),
        ("fleet-page-sqlite", 400 * 1024),
    ] {
        let budget = budgets::budget_named(name);
        assert!(
            budget <= cap,
            "{name}: the budget {budget} is past ruling 9's {cap}"
        );
    }
}
