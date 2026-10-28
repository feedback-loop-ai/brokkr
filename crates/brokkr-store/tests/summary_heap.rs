//! `brokkr doctor`'s summary comparison, its seat rows' included, holds
//! one run at a time (decision 0089 ruling 4): its peak heap is the largest run's, never
//! the journal's, however many runs the journal lists. A writable open's
//! catch-up holds one behind run at a time too (rulings 3 and 8).
//!
//! The seam is this binary's allocator. It passes every call to the
//! system allocator and counts the bytes the process's Rust code holds,
//! and the most it has held since a test reset the count. SQLite's own
//! pages are its allocator's and are not counted: what is measured is
//! what the comparison keeps of the rows it reads. The count is the
//! process's, so the tests take turns measuring.

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering::SeqCst};
use std::sync::{Mutex, MutexGuard};

use brokkr_store::Store;
use serde_json::json;

static HELD: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);

fn held(delta: isize) {
    let now = HELD.fetch_add(delta, SeqCst) + delta;
    PEAK.fetch_max(now, SeqCst);
}

struct Counting;

// SAFETY: every allocation is the system allocator's, unchanged.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc`, passed through.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            held(layout.size() as isize);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract for `dealloc`, passed through.
        unsafe { System.dealloc(ptr, layout) };
        held(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract for `realloc`, passed through.
        let moved = unsafe { System.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            held(new_size as isize - layout.size() as isize);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Held by a test while it measures, so no other test's allocations
/// count against it.
static MEASURING: Mutex<()> = Mutex::new(());

/// This test's turn to measure, whether or not another test failed in
/// its own.
fn turn() -> MutexGuard<'static, ()> {
    MEASURING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The most `measured` holds at once.
fn peak_of<T>(measured: impl FnOnce() -> T) -> (isize, T) {
    let before = HELD.load(SeqCst);
    PEAK.store(before, SeqCst);
    let found = measured();
    (PEAK.load(SeqCst) - before, found)
}

/// Each run's feature, the largest thing its `runs` row lists.
const FEATURE: usize = 1 << 20;
const RUNS: usize = 32;

/// The most the comparison of every run in `store` holds at once.
fn peak_of_comparing(store: &Store) -> isize {
    let (peak, found) = peak_of(|| store.summary_disagreements().unwrap());
    assert_eq!(found.len(), 0);
    peak
}

/// The most the comparison holds over one run and over `RUNS`, each run
/// made by `create` in one fresh journal.
fn peaks_of_one_and_many(create: impl Fn(&mut Store, &str)) -> (isize, isize) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let run_id = |run: usize| format!("r{run:02}");
    create(&mut store, &run_id(0));
    let one = peak_of_comparing(&store);
    (1..RUNS).for_each(|run| create(&mut store, &run_id(run)));
    (one, peak_of_comparing(&store))
}

/// The comparison over `RUNS` runs, each listing a `FEATURE`-byte
/// feature, peaks within one feature of the comparison over one: one that
/// collected every run's listing first would hold all `RUNS` at once.
#[test]
fn the_comparison_holds_one_run_at_a_time() {
    let _turn = turn();
    let feature = "f".repeat(FEATURE);
    let (one, many) = peaks_of_one_and_many(|store, run_id| {
        store
            .create_run(run_id, &feature, "self", &json!({}))
            .unwrap();
    });
    let bound = one + FEATURE as isize;
    assert!(
        many < bound,
        "comparing {RUNS} runs of a {FEATURE}-byte feature held {many} bytes at its peak, \
         and one run {one}; the bound is {bound}"
    );
}

/// Each run's events, and the bytes of each one's payload.
const EVENTS: usize = 32;
const PAYLOAD: usize = 32 << 10;

/// The comparison over `RUNS` runs of `EVENTS` events each, every event
/// indexed and `PAYLOAD` bytes wide, peaks within one run's events of the
/// comparison over one: the run read whole and its index rows streamed
/// beside it, one run at a time (decision 0089 ruling 4).
#[test]
fn the_comparison_holds_one_run_s_events_and_index_at_a_time() {
    let _turn = turn();
    let wide = "w".repeat(PAYLOAD);
    let (one, many) = peaks_of_one_and_many(|store, run_id| {
        store
            .create_run(run_id, "feat", "self", &json!({}))
            .unwrap();
        for at in 0..EVENTS {
            let event = brokkr_core::EventType::PhaseEntered;
            let payload = json!({"phase": format!("p{at}"), "notes": wide});
            store
                .append_next(run_id, event, payload, None, None)
                .unwrap();
        }
    });
    let bound = one + (EVENTS * PAYLOAD) as isize;
    assert!(
        many < bound,
        "comparing {RUNS} runs of {EVENTS} {PAYLOAD}-byte events held {many} bytes at its \
         peak, and one run {one}; the bound is {bound}"
    );
}

/// Each run's requests, and the bytes of each one's effect id, which its
/// seat row holds.
const SEATS: usize = 32;
const EFFECT_ID: usize = 32 << 10;

/// The comparison over `RUNS` runs of `SEATS` seat rows each, every row
/// holding an `EFFECT_ID`-byte effect id, peaks within one run's rows of
/// the comparison over one: a run's seat rows are streamed beside its
/// fold, one run at a time (decision 0089 ruling 4).
#[test]
fn the_comparison_holds_one_run_s_seat_rows_at_a_time() {
    let _turn = turn();
    let (one, many) = peaks_of_one_and_many(|store, run_id| {
        store
            .create_run(run_id, "feat", "self", &json!({}))
            .unwrap();
        for at in 0..SEATS {
            let event = brokkr_core::EventType::EffectRequested;
            let payload = json!({"effect_id": format!("{at:0EFFECT_ID$}"), "seat": "worker"});
            store
                .append_next(run_id, event, payload, None, None)
                .unwrap();
        }
    });
    let bound = one + (SEATS * EFFECT_ID) as isize;
    assert!(
        many < bound,
        "comparing {RUNS} runs of {SEATS} seat rows of {EFFECT_ID}-byte effect ids held \
         {many} bytes at its peak, and one run {one}; the bound is {bound}"
    );
}

/// Each behind run's id, at the longest a run id may be.
const ID_BYTES: usize = 128;
const BEHIND_RUNS: usize = 4096;

/// A journal at `db` of `runs` runs as a writer from before the summary
/// leaves them, every row behind its head.
fn behind_journal(db: &Path, runs: usize) {
    drop(Store::open(db).unwrap());
    let mut conn = rusqlite::Connection::open(db).unwrap();
    let planting = conn.transaction().unwrap();
    for run in 0..runs {
        let run_id = format!("{run:0ID_BYTES$}");
        planting
            .execute(
                "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
                 VALUES (?1, 'feat', 'self', '{}', '2026-01-01T00:00:00Z')",
                [run_id],
            )
            .unwrap();
    }
    planting.commit().unwrap();
}

/// The most a writable open of a journal of `runs` runs, planted by
/// `plant`, holds while a peer holds the write lock, so its catch-up meets
/// the lock at its first refold and stops.
fn peak_of_opening(runs: usize, plant: impl Fn(&Path, usize)) -> isize {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    plant(&db, runs);
    let peer = rusqlite::Connection::open(&db).unwrap();
    peer.execute_batch("BEGIN IMMEDIATE").unwrap();
    let (peak, store) = peak_of(|| Store::open(&db).unwrap());
    drop(store);
    peer.execute_batch("ROLLBACK").unwrap();
    peak
}

/// A writable open of `BEHIND_RUNS` behind runs peaks within a fraction
/// of their ids' bytes of an open of one: a catch-up that collected every
/// behind run's id before refolding the first would hold them all at
/// once, however soon a peer's lock ended it.
#[test]
fn a_catch_up_holds_one_behind_run_at_a_time() {
    let _turn = turn();
    let one = peak_of_opening(1, behind_journal);
    let many = peak_of_opening(BEHIND_RUNS, behind_journal);
    let bound = one + (BEHIND_RUNS * ID_BYTES / 8) as isize;
    assert!(
        many < bound,
        "opening {BEHIND_RUNS} behind runs held {many} bytes at its peak, and one {one}; \
         the bound is {bound}"
    );
}

/// Each fleet run's park, at the longest a run id may be.
const OFF_HEAD_RUNS: usize = 64;

/// A journal at `db` of `runs` three-event parked runs whose park stamps
/// name seq 1 rather than their head, as a missing trigger leaves them;
/// their summary and seat rows stand at their heads, so only the parks'
/// sweep has work.
fn off_head_park_journal(db: &Path, runs: usize) {
    let mut store = Store::open(db).unwrap();
    for run in 0..runs {
        let run_id = format!("{run:0ID_BYTES$}");
        store
            .create_run(&run_id, "feat", "self", &json!({}))
            .unwrap();
        for (event, payload) in [
            (
                brokkr_core::EventType::RunStarted,
                json!({"feature": "feat", "manifest": {}}),
            ),
            (
                brokkr_core::EventType::PhaseEntered,
                json!({"phase": "implement"}),
            ),
            (
                brokkr_core::EventType::RunParked,
                json!({"reason": "waiting"}),
            ),
        ] {
            store
                .append_next(&run_id, event, payload, None, None)
                .unwrap();
        }
    }
    drop(store);
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "UPDATE event_index SET facts_seq = 1
             WHERE type = 'run/parked' AND facts_seq <> 1",
            [],
        )
        .unwrap();
}

/// A writable open of `OFF_HEAD_RUNS` off-head parks peaks within a
/// fraction of their ids' bytes of an open of one: a park walk that
/// collected every off-head run's id before refolding the first would hold
/// them all at once, however soon a peer's lock ended it (decision 0089
/// rulings 3 and 8).
#[test]
fn a_park_sweep_holds_one_off_head_run_at_a_time() {
    let _turn = turn();
    let one = peak_of_opening(1, off_head_park_journal);
    let many = peak_of_opening(OFF_HEAD_RUNS, off_head_park_journal);
    let bound = one + (OFF_HEAD_RUNS * ID_BYTES / 8) as isize;
    assert!(
        many < bound,
        "opening {OFF_HEAD_RUNS} off-head parks held {many} bytes at its peak, and one {one}; \
         the bound is {bound}"
    );
}
