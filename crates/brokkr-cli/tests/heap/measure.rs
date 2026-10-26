//! The two counters a readout's live heap is read by (decision 0089 ruling
//! 9): dhat sees Rust's global allocator alone, and SQLite allocates beside
//! it, so SQLite's share is read from its own high-water. Shared by the
//! heap binaries that read the journal, and each binary's allocator.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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

/// Brokkr's peak heap while `read` runs, under dhat.
pub(crate) fn peak_of(read: impl FnOnce()) -> u64 {
    let profiler = dhat::Profiler::builder().testing().build();
    read();
    let peak = dhat::HeapStats::get().max_bytes as u64;
    drop(profiler);
    peak
}
