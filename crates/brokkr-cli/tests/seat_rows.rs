//! The store's seat rows read as the run view's participants (#753;
//! decision 0089 slice 2, the seat-rows read slice 2b pages by): every
//! fixture journal imported fresh, its rows read back by
//! `Store::seat_rows` and grouped by brokkr-core's `facts`, equals what
//! `run_view` shows of each participant, field for field, a price by its
//! bits.

use brokkr_core::seat::row::facts;
use brokkr_store::test_support::import_fixtures;
use brokkr_store::{SeatRows, Store};

#[path = "../../../tests/support/seat_facts.rs"]
mod seat_facts;

#[test]
fn every_fixture_s_seat_rows_read_from_the_store_as_its_participants() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let (runs, _) = import_fixtures(&mut store);
    drop(store);
    // The fixture the import refuses is planted as an older writer's, and
    // this open folds its seat rows; the broken chain does not load.
    let store = Store::open(&path).unwrap();
    let (mut found, mut compared, mut stale) = (Vec::new(), 0, Vec::new());
    for run in &runs {
        match store.seat_rows(run).unwrap() {
            SeatRows::Current(rows) => {
                let events = store.load(run).unwrap();
                let (differ, count) = seat_facts::disagreements(run, &events, &facts(&rows));
                found.extend(differ);
                compared += count;
            }
            SeatRows::Stale => stale.push(run.as_str()),
        }
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(stale, ["conclude-broken-chain-hand-built"]);
    assert_eq!(compared, 18);
}
