//! `brokkr costs` reads the seat rows (#753; decision 0089 slice 3's
//! acceptance): every fixture journal imported fresh, and every committed
//! journal and hand-built seat journal planted as an older writer leaves
//! it and caught up by a writable open; each run's `costs` prints byte for
//! byte what it prints of the same journal unmigrated, which it reads
//! whole as it read every run before the rows (ruling 8's kept read), a
//! run it refuses refused in the same words. The counts are of the runs
//! read from their seat rows at their opening.

use std::path::{Path, PathBuf};

use brokkr_store::test_support::unmigrate;
use brokkr_store::{SeatRows, Store};

use crate::boundary_readouts::brokkr;
use crate::composed_view::imported;
use crate::seat_view::{committed_runs, hand_built_runs, planted};

/// The journal at `db` copied beside it, then unmigrated: the journal a
/// binary from before the projections left.
fn unmigrated(db: &Path) -> PathBuf {
    let kept = db.with_extension("kept.db");
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute("VACUUM INTO ?1", [kept.to_str().unwrap()])
        .unwrap();
    unmigrate(&rusqlite::Connection::open(&kept).unwrap()).unwrap();
    kept
}

/// What `brokkr costs` prints of each of `runs` from the journal at `db`,
/// each by its run.
fn printed(db: &Path, runs: &[String]) -> Vec<(String, (String, String, bool))> {
    let (dir, at) = (db.parent().unwrap(), db.to_str().unwrap());
    let printed = |run: &String| {
        let costs = ["costs", "--run", run, "--db", at];
        (run.clone(), brokkr(dir, &costs))
    };
    runs.iter().map(printed).collect()
}

/// How many of `runs` the journal at `db` reads from their seat rows at
/// their opening.
fn read_from_rows(db: &Path, runs: &[String]) -> usize {
    let store = Store::open_read_only(db).unwrap();
    let current = |run: &&String| {
        let Ok(opened) = store.open_run(run) else {
            return false;
        };
        let seats = store.seat_rows_at(run, &opened).unwrap();
        matches!(seats, SeatRows::Current(_))
    };
    runs.iter().filter(current).count()
}

/// Whether `costs` prints of `runs` in the journal at `db` what it prints
/// of it unmigrated, the first difference named; and how many runs it
/// reads from the rows, none of the unmigrated journal's.
fn read_as_today(db: &Path, runs: &[String]) -> (Option<String>, usize) {
    let kept = unmigrated(db);
    assert_eq!(read_from_rows(&kept, runs), 0, "read whole");
    let (rows, whole) = (printed(db, runs), printed(&kept, runs));
    assert_eq!(rows.len(), whole.len());
    let differ = rows.iter().zip(&whole).find(|(rows, whole)| rows != whole);
    let differ = differ.map(|(rows, whole)| format!("{rows:?} against {whole:?}"));
    (differ, read_from_rows(db, runs))
}

#[test]
fn every_fixture_s_costs_read_from_the_rows_are_today_s() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (store, runs) = imported(&path);
    drop(store);
    assert_eq!(runs.len(), 6);
    assert_eq!(read_as_today(&path, &runs), (None, 5));
}

#[test]
fn every_committed_and_hand_built_journal_s_costs_are_today_s() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let journals = committed_runs().into_iter().chain(hand_built_runs());
    let runs = planted(&path, journals.collect());
    assert_eq!(runs.len(), 39);
    assert_eq!(read_as_today(&path, &runs), (None, 37));
}
