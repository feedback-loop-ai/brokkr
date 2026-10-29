//! `brokkr costs` and `brokkr compare` read the seat rows (#753; decision
//! 0089 slice 3's acceptance): every fixture journal imported fresh, and
//! every committed journal and hand-built seat journal planted as an
//! older writer leaves it and caught up by a writable open; each run's
//! `costs`, and `compare` of each run beside the next, print byte for byte
//! what they print of the same journal unmigrated, which they read whole
//! as they read every run before the rows (ruling 8's kept read), a run
//! they refuse refused in the same words. The counts are of the runs read
//! from their rows at their opening: by `costs`, and by `compare` with
//! the run's fold at its head.

use std::path::{Path, PathBuf};

use brokkr_store::test_support::unmigrate;
use brokkr_store::{RunRead, SeatRows, Store};

use crate::boundary_readouts::brokkr;
use crate::composed_view::imported;
use crate::seat_view::journals::journals;
use crate::seat_view::planted::{chained, committed_at, planted};

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
/// and `brokkr compare` of each beside the next, each by its arguments.
fn printed(db: &Path, runs: &[String]) -> Vec<(String, (String, String, bool))> {
    let (dir, at) = (db.parent().unwrap(), db.to_str().unwrap());
    let mut printed = Vec::new();
    for (place, run) in runs.iter().enumerate() {
        let next = &runs[(place + 1) % runs.len()];
        let costs = ["costs", "--run", run, "--db", at];
        let compare = ["compare", run, next, "--db", at];
        for args in [&costs[..], &compare[..]] {
            printed.push((args[..3].join(" "), brokkr(dir, args)));
        }
    }
    printed
}

/// How many of `runs` the journal at `db` reads from their rows at their
/// opening: their seat rows (`costs`), and their trail with its fold at
/// the head (`compare`).
fn read_from_rows(db: &Path, runs: &[String]) -> (usize, usize) {
    let store = Store::open_read_only(db).unwrap();
    let (mut costs, mut compare) = (0, 0);
    for run in runs {
        let Ok(opened) = store.open_run(run) else {
            continue;
        };
        let seats = store.seat_rows_at(run, &opened).unwrap();
        costs += usize::from(matches!(seats, SeatRows::Current(_)));
        let read = store.run_read(run, &opened, opened.seq).unwrap();
        let folded = matches!(read, RunRead::Current { state: Some(_), .. });
        compare += usize::from(folded);
    }
    (costs, compare)
}

/// Whether `costs` and `compare` print of `runs` in the journal at `db`
/// what they print of it unmigrated, the first difference named; and how
/// many runs each read from the rows, none of the unmigrated journal's.
fn read_as_today(db: &Path, runs: &[String]) -> (Option<String>, (usize, usize)) {
    let kept = unmigrated(db);
    assert_eq!(read_from_rows(&kept, runs), (0, 0), "read whole");
    let (rows, whole) = (printed(db, runs), printed(&kept, runs));
    assert_eq!(rows.len(), whole.len());
    let differ = rows.iter().zip(&whole).find(|(rows, whole)| rows != whole);
    let differ = differ.map(|(rows, whole)| format!("{rows:?} against {whole:?}"));
    (differ, read_from_rows(db, runs))
}

#[test]
fn every_fixture_s_costs_and_compare_read_from_the_rows_are_today_s() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (store, runs) = imported(&path);
    drop(store);
    assert_eq!(runs.len(), 6);
    assert_eq!(read_as_today(&path, &runs), (None, (5, 5)));
}

#[test]
fn every_committed_and_hand_built_journal_s_costs_and_compare_are_today_s() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let hand_built = journals().into_iter();
    let hand_built = hand_built.map(|(name, events)| (name.to_string(), chained(name, events)));
    let mut runs = committed_at(&path);
    runs.extend(planted(&path, hand_built.collect()));
    assert_eq!(runs.len(), 39);
    assert_eq!(read_as_today(&path, &runs), (None, (37, 16)));
}
