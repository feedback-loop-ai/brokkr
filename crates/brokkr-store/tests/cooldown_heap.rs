//! The readout's newest cool-downs' SQLite heap at volume (#1112, decision
//! 0089 ruling 9 as amended on 2026-11-13): the read is one scan of the
//! table in `seq` order, so SQLite sorts nothing and holds no more at
//! 10,000 rows than at 10, however wide a route's name. A window statement
//! that ranked the rows by provider and route sorted every row's names and
//! passed the cap at 10,000 rows once a name was 24 characters wide (the
//! review's finding on #1112).
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

/// A short route name's width, and a wide one's: the review measured the
/// name-keyed sorter past the cap from 24 characters on (#1112).
const NARROW: usize = 7;
const WIDE: usize = 66;

/// How much the wide names may add to the narrow names' figure: a sorter
/// holding a name per row is what this bound refuses.
const SLACK: i64 = 64 * 1_024;

/// The test the child process runs, by the name the test binary lists it.
const CHILD: &str = "cooldown_heap::the_newest_read_stays_under_the_cap";

/// Route `route`'s name, `width` characters wide.
fn route_name(route: usize, width: usize) -> String {
    let mut name = format!("route-{route}-");
    while name.len() < width {
        name.push('w');
    }
    name
}

/// A journal holding `rows` cool-downs over [`ROUTES`] routes whose names
/// are `width` wide, ending from second 1,000 on, with words as long as a
/// provider's bounded reason.
fn journal(path: &Path, rows: usize, width: usize) -> Store {
    drop(std::fs::remove_file(path));
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
                route_name(at % ROUTES, width),
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

/// The read's least high-water over [`TAKES`] reads at second `now`.
fn cost(store: &Store, now: i64) -> i64 {
    let now = OffsetDateTime::from_unix_timestamp(now).unwrap();
    (0..TAKES)
        .map(|_| high_water(|| drop(store.cooldowns_newest(now).unwrap())))
        .min()
        .unwrap()
}

/// Every row expired by the read.
const EXPIRED: i64 = 1_000_000;

/// Every row in force at the read.
const IN_FORCE: i64 = 0;

/// SQLite's share of `cooldowns_newest` at 10,000 rows stays under the
/// operator's 1,024,000-byte cap with every row expired and with every row
/// in force, at a narrow route name and a wide one, and the wide names add
/// no more than [`SLACK`].
#[test]
fn the_newest_read_stays_under_the_cap() {
    let labels = [
        "SQLite high-water, 10 cool-downs: ",
        "SQLite high-water, 10,000 expired, narrow names: ",
        "SQLite high-water, 10,000 expired, wide names: ",
        "SQLite high-water, 10,000 in force, wide names: ",
    ];
    if std::env::var_os(OWN_PROCESS).is_none() {
        measured_in_its_own_process(CHILD, &labels, 1);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let figures = [
        cost(&journal(&path, 10, NARROW), EXPIRED),
        cost(&journal(&path, 10_000, NARROW), EXPIRED),
        cost(&journal(&path, 10_000, WIDE), EXPIRED),
        cost(&journal(&path, 10_000, WIDE), IN_FORCE),
    ];
    for (label, figure) in labels.iter().zip(figures) {
        eprintln!("{label}[{figure}]");
    }
    for (label, figure) in labels.iter().zip(figures) {
        assert!(figure < CAP, "{label}{figure} bytes, past the {CAP} cap");
    }
    let [_, narrow, wide, _] = figures;
    assert!(
        wide <= narrow + SLACK,
        "wide names held {wide} bytes against narrow names' {narrow}: a name is in the sorter"
    );
}
