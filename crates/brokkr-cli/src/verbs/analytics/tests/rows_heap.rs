//! The row path holds no set of run ids (decision 0089 ruling 9's aggregate
//! exception, slice 4's unit U5b2a): the runs the store leaves whole are the
//! whole read's own and are held as it holds any run, and nothing of the
//! hearth's other runs is held beside them. A run's id is a long string
//! here, so a copy of it dwarfs every other per-run share, and the bytes a
//! hearth's read grows by per behind run say how many copies of the id it
//! holds.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use super::snapshot::read_between;
use super::*;

/// How long each behind run's id is: past every other per-run share of the
/// read, so the growth is counted in copies of the id.
const LONG: usize = 32 * 1024;

thread_local! {
    /// Whether this thread's allocations are the measured read's.
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    /// The bytes the measured read holds now, and its peak.
    static LIVE: Cell<i64> = const { Cell::new(0) };
    static PEAK: Cell<i64> = const { Cell::new(0) };
}

/// The system allocator, counting the measuring thread's live bytes.
struct Counting;

/// Account `bytes` more (or fewer) held by the measured read.
fn held(bytes: i64) {
    if MEASURING.get() {
        let live = LIVE.get() + bytes;
        LIVE.set(live);
        PEAK.set(PEAK.get().max(live));
    }
}

// SAFETY: every block is the system allocator's; the counters are
// thread-local cells with no destructor, so counting allocates nothing.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        held(layout.size() as i64);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        held(-(layout.size() as i64));
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        held(new_size as i64 - layout.size() as i64);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// The peak bytes this thread holds while `read` runs, above where it
/// stood when it began.
fn peak_of(read: impl FnOnce()) -> i64 {
    LIVE.set(0);
    PEAK.set(0);
    MEASURING.set(true);
    read();
    MEASURING.set(false);
    PEAK.get()
}

/// A journal of `runs` behind runs, each one event and an id [`LONG`] bytes
/// wide, so no copy of an id hides among the run's other bytes.
fn long_ids(dir: &std::path::Path, name: &str, runs: usize) -> std::path::PathBuf {
    let journal = dir.join(name);
    let store = brokkr_store::Store::open(&journal).unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    let start = started(Some("alpha"), "recipe", stamp(base, -30 * 60)).remove(0);
    for index in 0..runs {
        let run_id = format!("{index:06}{}", "x".repeat(LONG));
        let event = EnvelopeBuilder::new(start.0, start.1.clone())
            .run(&run_id)
            .seq(1)
            .event_id(format!("e{index}"))
            .at(start.2.clone())
            .previous(ZERO_HASH.to_string())
            .sealed();
        plant_older_writer(&store, &run_id, &[event]);
    }
    journal
}

/// The hearth's read grows by the whole read's own runs and no set of ids:
/// over 40 and 80 behind runs it holds as many copies of each run's id as
/// the whole read of the same runs does (about five), and a set, a map or a
/// clone of every behind run's id would be one more.
#[test]
fn the_row_path_holds_no_set_of_run_ids() {
    let dir = tempfile::tempdir().unwrap();
    let (few, many) = (40, 80);
    let small = long_ids(dir.path(), "few.db", few);
    let large = long_ids(dir.path(), "many.db", many);
    let first = format!("{:06}{}", 0, "x".repeat(LONG));
    assert_eq!(lane(&ready(&large), &first).as_deref(), Some("behind"));
    // Copies of a behind run's id the read holds at its peak, over the
    // journal of `few` runs and the one of `many`.
    let copies = |read: &dyn Fn(&std::path::Path)| {
        read(&small);
        let (fewer, more) = (peak_of(|| read(&small)), peak_of(|| read(&large)));
        let copies = (more - fewer) as f64 / ((many - few) * LONG) as f64;
        eprintln!("{fewer} B at {few} runs, {more} B at {many}: {copies:.2} copies of an id");
        copies
    };
    let row = copies(&|journal| drop(read_between(journal, |_| {})));
    let whole = copies(&|journal| {
        let hearths = [hearth(Vec::new(), journal)];
        drop(view(&hearths, Range::Day, None, GENERATED_NOW));
    });
    assert!(
        row <= whole + 0.5,
        "the row path holds {row:.2} copies of each behind run's id and the whole read {whole:.2}"
    );
}
