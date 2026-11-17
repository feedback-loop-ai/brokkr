//! The readout's newest cool-downs' SQLite heap at volume (#1112, decision
//! 0089 ruling 9 as amended on 2026-11-13): the window form sorts the
//! table's rows by provider and route, and its sorter at 10,000 rows stays
//! under the operator's cap. The sorter carries only the four columns the
//! rank reads; the kept rows are read back by their `seq`.
//!
//! The sorter is not constant: its buffer doubles with the rows until
//! SQLite spills it, and from about 12,000 rows on it holds about 1.2 MB,
//! past the cap (#1112's finding). An index on `(provider, route,
//! until_at, seq)` would let the window read in order with no sorter, but
//! it moves queue storage and is asked for, not built here.
//!
//! The counters are the process's, so the measurement runs in a process of
//! its own, as [`crate::exclusion_heap`]'s does.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use rusqlite::{params, Connection};
use time::OffsetDateTime;

use crate::exclusion_heap::{high_water, measured_in_its_own_process, CAP, OWN_PROCESS};

/// How many times the read is measured at a size; the least is the
/// statement's own share.
const TAKES: usize = 5;

/// The routes the rows spread over.
const ROUTES: usize = 10;

/// The test the child process runs, by the name the test binary lists it.
const CHILD: &str = "cooldown_heap::the_newest_read_stays_under_the_cap";

/// A journal holding `rows` cool-downs over [`ROUTES`] routes, every one
/// expired by the read, with words as long as a provider's bounded reason.
fn journal(path: &Path, rows: usize) -> Store {
    let mut store = Store::open(path).unwrap();
    let conn = Connection::open(path).unwrap();
    conn.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES ('run-a', 'feat', 'self', '{}', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    store
        .append_next(
            "run-a",
            EventType::EffectRequested,
            serde_json::json!({"effect_id": "effect"}),
            None,
            None,
        )
        .unwrap();
    conn.execute_batch("BEGIN").unwrap();
    for at in 0..rows {
        conn.execute(
            "INSERT INTO provider_cooldowns
                (run_id, event_seq, site, provider, route, until_at, reason,
                 retry_after_seconds, rounds, recorded_at)
             VALUES ('run-a', 1, '', 'claude', ?1, ?2,
                     'rate_limit: the provider asked this seat to slow down', 30, ?3, ?2 - 1)",
            params![
                format!("route-{}", at % ROUTES),
                1_000 + at as i64,
                at as i64 + 1
            ],
        )
        .unwrap();
    }
    conn.execute_batch("COMMIT").unwrap();
    drop(conn);
    drop(store);
    Store::open_read_only(path).unwrap()
}

/// The read's least high-water over [`TAKES`] reads.
fn cost(store: &Store) -> i64 {
    let now = OffsetDateTime::from_unix_timestamp(1_000_000).unwrap();
    (0..TAKES)
        .map(|_| high_water(|| drop(store.cooldowns_newest(now).unwrap())))
        .min()
        .unwrap()
}

/// SQLite's share of `cooldowns_newest` at 10 and 10,000 rows stays under
/// the operator's 1,024,000-byte cap.
#[test]
fn the_newest_read_stays_under_the_cap() {
    if std::env::var_os(OWN_PROCESS).is_none() {
        measured_in_its_own_process(
            CHILD,
            &[
                "SQLite high-water, 10 cool-downs: ",
                "SQLite high-water, 10,000 cool-downs: ",
            ],
            1,
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let few = cost(&journal(&path, 10));
    eprintln!("SQLite high-water, 10 cool-downs: [{few}]");
    drop(std::fs::remove_file(&path));
    let many = cost(&journal(&path, 10_000));
    eprintln!("SQLite high-water, 10,000 cool-downs: [{many}]");
    assert!(
        many < CAP,
        "SQLite held {many} bytes at 10,000 cool-downs, past the {CAP} cap"
    );
}
