//! The two counters a readout's live heap is read by (decision 0089 ruling
//! 9): dhat sees Rust's global allocator alone, and SQLite allocates beside
//! it, so SQLite's share is read from its own high-water. Shared by the
//! heap binaries that read the journal, and each binary's allocator.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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
