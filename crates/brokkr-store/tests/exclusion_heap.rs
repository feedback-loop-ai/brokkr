//! The exclusion's SQLite heap at volume (decision 0089 ruling 9's
//! aggregate exception, as amended on 2026-11-13 for SQLite without
//! lookaside): every window statement probes the whole read's keys, so
//! 5,000 stale runs add no more heap than 10. A materialized set would
//! hold every stale row in a temporary B-tree and grow past 64 KiB here
//! (the review measured 1.4 MB at the real store's row width).
//!
//! This is an integration test because `brokkr-store` forbids unsafe code
//! and SQLite exposes its memory high-water only through the C API; the
//! library's own tests cannot call it.

use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_store::aggregate::RunOrder;
use brokkr_store::metered::SiteOrder;
use brokkr_store::Store;
use rusqlite::{params, Connection};

/// The fixed window and a day.
const FROM: i64 = 1_700_000_000_000;
const TO: i64 = FROM + 60_000;

/// The operator's SQLite cap, 1,000 KiB (2026-11-13).
const CAP: i64 = 1_024_000;

/// The parks statement's measured 152,024 B plus a tenth, the tighter
/// bound the operator's ruling asks for while that is under [`CAP`]. The
/// classes' table costs one preparation whatever the journal holds; the
/// figure is not a materialized set and does not follow the arm count.
const PARKS_BOUND: i64 = 167_227;

/// How much a volume measurement may exceed the ten-stale measurement:
/// above the probes' per-row share and below the materialized set's growth
/// at this size (about 1.4 MB).
const SLACK: i64 = 64 * 1_024;

/// How many times each statement is measured at a size; the least is the
/// statement's own share, with another test's concurrent allocation left
/// out.
const TAKES: usize = 5;

/// SQLite's memory counters: what it holds now and its high-water.
#[derive(Clone, Copy)]
struct Memory {
    now: i64,
    high: i64,
}

/// SQLite's counters, with `reset` starting the high-water again from now.
fn memory(reset: bool) -> Memory {
    let (mut now, mut high) = (0i64, 0i64);
    // SAFETY: `sqlite3_status64` writes the two counters through the two
    // pointers it is handed, each a live i64 on this stack, and reads
    // nothing else.
    let status = unsafe {
        rusqlite::ffi::sqlite3_status64(
            rusqlite::ffi::SQLITE_STATUS_MEMORY_USED,
            &mut now,
            &mut high,
            i32::from(reset),
        )
    };
    assert_eq!(status, 0, "SQLite reads its own memory status");
    Memory { now, high }
}

/// SQLite's high-water above where it stood before `read`, in bytes.
fn high_water(read: impl FnOnce()) -> i64 {
    let base = memory(true).now;
    read();
    memory(false).high - base
}

/// Each statement's least high-water over [`TAKES`] reads.
fn costs(store: &Store) -> [i64; 6] {
    let mut least = [i64::MAX; 6];
    for _ in 0..TAKES {
        let read = [
            high_water(|| {
                store
                    .window_runs(FROM, TO, RunOrder::Unordered, None, |_| {})
                    .unwrap()
            }),
            high_water(|| store.window_parks(FROM, TO, TO, None, |_| {}).unwrap()),
            high_water(|| store.window_spends(FROM, TO, None, |_| {}).unwrap()),
            high_water(|| {
                store
                    .window_sites(FROM, TO, None, SiteOrder::Model, |_| {})
                    .unwrap()
            }),
            high_water(|| store.durations(FROM, TO, |_, _| {}).unwrap()),
            high_water(|| store.finished_between(FROM, TO, |_, _| {}).unwrap()),
        ];
        for (least, read) in least.iter_mut().zip(read) {
            *least = (*least).min(read);
        }
    }
    least
}

/// One stale run's row (no seat head) and its summary stamps.
fn run(conn: &Connection, run_id: &str) {
    conn.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES (?1, 'feat', 'self', '{}', '2026-01-01T00:00:00Z')",
        [run_id],
    )
    .unwrap();
    conn.execute(
        "UPDATE run_summary SET realm = 'alpha', status = ?2, started_ms = ?3,
            finished_ms = ?3, ending = ?4, folded_seq = 0, folded_hash = ?5,
            aggregated_seq = 0, stop_folded_seq = 0, ending_folded_seq = 0,
            late_folded_seq = 0, implement_visits = 0, review_first = 0,
            review_second = 0, review_back = 0
          WHERE run_id = ?1",
        params![run_id, "completed", FROM + 30_000, "shipped", ZERO_HASH],
    )
    .unwrap();
}

/// The journal with `stale` stale runs in the window, opened for reading.
fn journal(path: &Path, stale: usize) -> Store {
    drop(Store::open(path).unwrap());
    let conn = Connection::open(path).unwrap();
    for index in 0..stale {
        let run_id = format!("stale-{index}");
        run(&conn, &run_id);
    }
    drop(conn);
    Store::open_read_only(path).unwrap()
}

/// The window statements' SQLite heap does not grow with the stale set:
/// 5,000 stale runs add no more than ten. All six statements stay under the
/// operator's 1,024,000-byte cap, and the parks statement, whose classes'
/// table replaced sixty-four prepared arms in this unit, is held to its
/// measured figure plus a tenth.
#[test]
fn the_exclusion_holds_no_stale_set_in_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let ten = costs(&journal(&path, 10));
    eprintln!("SQLite high-water, 10 stale: {ten:?}");
    drop(std::fs::remove_file(&path));

    let many = costs(&journal(&path, 5_000));
    eprintln!("SQLite high-water, 5,000 stale: {many:?}");

    // The exclusion's share is the growth over ten stale runs, not the
    // statement's constant preparation.
    for (at, (few, lots)) in ten.iter().zip(&many).enumerate() {
        assert!(
            *lots - *few <= SLACK,
            "statement {at}: SQLite grew from {few} to {lots} bytes with the stale set"
        );
    }
    for (at, figure) in many.iter().enumerate() {
        assert!(
            *figure < CAP,
            "statement {at}: SQLite held {figure} bytes at 5,000 stale, past the {CAP} cap"
        );
    }
    assert_eq!(
        many[1], ten[1],
        "the parks statement's preparation is the same at both sizes"
    );
    assert!(
        many[1] < PARKS_BOUND,
        "the parks statement: SQLite held {} bytes, past its measured figure's tenth ({PARKS_BOUND})",
        many[1]
    );
}
