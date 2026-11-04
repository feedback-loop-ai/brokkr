//! The two counters a readout's live heap is read by (decision 0089 ruling
//! 9): dhat sees the measuring thread's Rust allocations alone, and SQLite
//! allocates beside it, so SQLite's share is read from its own high-water.
//! Shared by the heap binaries that read the journal, and each binary's
//! allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Whether this thread is the one [`peak_of`] measures.
    static MEASURING: Cell<bool> = const { Cell::new(false) };
}

/// dhat sees the measuring thread's allocations alone. It counts every
/// thread's, and libtest allocates its own "has been running for over 60
/// seconds" warning on the harness's thread at the 60-second mark: a
/// window that spans it reads the warning's bytes as the readout's, which
/// moved one hosted run's peak 622 B (#965). The read is the measuring
/// thread's and no other thread's, so the allocator sends that thread's
/// allocations to dhat and every other thread's to the system.
struct Measured;

// SAFETY: every block is the system allocator's, whichever side made it.
// dhat 0.3.3's `Alloc` (the version Cargo.lock pins) makes, grows and frees
// each block through `System` and only records it, so a block made inside a
// window and freed after it, or the reverse, is still freed by `System`.
// dhat ignores a free of a block it did not record, treats a realloc of one
// as a fresh allocation, and guards its own bookkeeping with `IgnoreAllocs`,
// so routing by `MEASURING` never corrupts its tables. A dhat that stopped
// delegating to `System` would break this.
unsafe impl GlobalAlloc for Measured {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            if MEASURING.get() {
                dhat::Alloc.alloc(layout)
            } else {
                System.alloc(layout)
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            if MEASURING.get() {
                dhat::Alloc.dealloc(ptr, layout)
            } else {
                System.dealloc(ptr, layout)
            }
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            if MEASURING.get() {
                dhat::Alloc.realloc(ptr, layout, new_size)
            } else {
                System.realloc(ptr, layout, new_size)
            }
        }
    }
}

#[global_allocator]
static ALLOC: Measured = Measured;

/// The measuring thread's window, closed when it drops however `read`
/// ends.
struct Window;

impl Window {
    /// Opens this thread's window: its allocations are the measured read's
    /// until the window drops.
    fn open() -> Self {
        MEASURING.with(|measuring| measuring.set(true));
        Self
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        MEASURING.with(|measuring| measuring.set(false));
    }
}

/// SQLite's budget on every host, 500 KiB: the fleet's by the operator's
/// ruling of 2026-10-21 on #753, the cap itself and not a measured peak
/// plus a tenth, since CI's macOS leg measures SQLite's high-water about a
/// tenth above Linux. Opening a run and the run view's first screen hold
/// SQLite's budget to it too.
pub(crate) const SQLITE_CAP: u64 = 500 * 1024;

/// Reports SQLite's share `held` of the readout `name` and its headroom
/// under [`SQLITE_CAP`], in bytes, and the share of the cap it holds. CI's
/// macOS leg measures SQLite about 12% above Linux (#753), so a Linux
/// figure under about 457,000 bytes is one the macOS leg holds too.
pub(crate) fn report_headroom(name: &str, held: u64) {
    let headroom = i128::from(SQLITE_CAP) - i128::from(held);
    let permille = held * 1000 / SQLITE_CAP;
    eprintln!(
        "{name}: SQLite's {held} bytes, {}.{}% of the cap of {SQLITE_CAP}; {headroom} under it",
        permille / 10,
        permille % 10
    );
}

/// SQLite's memory now, and its high-water since the last reset, which
/// `reset` starts again from now.
pub(crate) fn sqlite_memory(reset: bool) -> (i64, i64) {
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

/// Brokkr's peak heap while `read` runs, under dhat, on this thread alone.
pub(crate) fn peak_of(read: impl FnOnce()) -> u64 {
    let profiler = dhat::Profiler::builder().testing().build();
    {
        let _window = Window::open();
        read();
    }
    let peak = dhat::HeapStats::get().max_bytes as u64;
    drop(profiler);
    peak
}
