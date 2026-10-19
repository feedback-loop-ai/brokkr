//! `brokkr doctor`'s summary comparison holds one run at a time
//! (decision 0089 ruling 4): its peak heap is the largest run's, never
//! the journal's, however many runs the journal lists.
//!
//! The seam is this binary's allocator. It passes every call to the
//! system allocator and counts the bytes the process's Rust code holds,
//! and the most it has held since a test reset the count. SQLite's own
//! pages are its allocator's and are not counted: what is measured is
//! what the comparison keeps of the rows it reads.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering::SeqCst};

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

/// Each run's feature, the largest thing its `runs` row lists.
const FEATURE: usize = 1 << 20;
const RUNS: usize = 32;

/// The most the comparison of every run in `store` holds at once.
fn peak_of_comparing(store: &Store) -> isize {
    let before = HELD.load(SeqCst);
    PEAK.store(before, SeqCst);
    let found = store.summary_disagreements().unwrap();
    let peak = PEAK.load(SeqCst) - before;
    assert_eq!(found.len(), 0);
    peak
}

/// The comparison over `RUNS` runs, each listing a `FEATURE`-byte
/// feature, peaks within one feature of the comparison over one: one that
/// collected every run's listing first would hold all `RUNS` at once.
#[test]
fn the_comparison_holds_one_run_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let feature = "f".repeat(FEATURE);
    let create = |store: &mut Store, run: usize| {
        let run_id = format!("r{run:02}");
        store
            .create_run(&run_id, &feature, "self", &json!({}))
            .unwrap();
    };
    create(&mut store, 0);
    let one = peak_of_comparing(&store);
    (1..RUNS).for_each(|run| create(&mut store, run));
    let many = peak_of_comparing(&store);
    let bound = one + FEATURE as isize;
    assert!(
        many < bound,
        "comparing {RUNS} runs of a {FEATURE}-byte feature held {many} bytes at its peak, \
         and one run {one}; the bound is {bound}"
    );
}
