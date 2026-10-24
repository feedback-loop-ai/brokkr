//! The store's seat rows read as the run view's participants (#753;
//! decision 0089 slice 2, the seat-rows read slice 2b pages by): every
//! fixture journal imported fresh, its rows read back by
//! `Store::seat_rows` and grouped by brokkr-core's `facts`, equals what
//! `run_view` shows of each participant, field for field, a price by its
//! bits, and of each phase's inner columns and each visit's models. The
//! count is of the participants, phases and visits compared.

use brokkr_store::test_support::import_fixtures;
use brokkr_store::{SeatRows, Store};
use brokkr_view::run_view;

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
                let (differ, count) = seat_facts::disagreements(run, &events, &rows);
                found.extend(differ);
                compared += count;
            }
            SeatRows::Stale => stale.push(run.as_str()),
        }
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(stale, ["conclude-broken-chain-hand-built"]);
    assert_eq!(compared, 52);
}

/// Every fixture's journal rows are caused, read by key from the event
/// index's ids (decision 0089 slice 2b), by the events `run_view` maps
/// from the run read whole: each row's cause by its seq, or none. The
/// broken chain does not load, and its index rows are held to its own
/// lines by brokkr-store's fixture test.
#[test]
fn every_fixture_s_causes_read_by_the_index_as_the_run_view_names_them() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (runs, _) = import_fixtures(&mut store);
    let (mut caused, mut unread) = (0, Vec::new());
    for run in &runs {
        let Ok(events) = store.load(run) else {
            unread.push(run.as_str());
            continue;
        };
        let ids: Vec<&str> = events
            .iter()
            .filter_map(|event| event.causation_id.as_deref())
            .collect();
        let causes = store.index_causes(run, &ids).unwrap().unwrap();
        let cause = |id: Option<&str>| id.and_then(|id| causes.get(id).copied());
        let read: Vec<Option<u64>> = events
            .iter()
            .map(|event| cause(event.causation_id.as_deref()))
            .collect();
        let journal = run_view(&events, None).journal;
        let shown: Vec<Option<u64>> = journal.iter().map(|row| row.causation_seq).collect();
        assert_eq!(read, shown, "{run}");
        caused += read.iter().flatten().count();
    }
    assert_eq!(unread, ["conclude-broken-chain-hand-built"]);
    assert_eq!(caused, 205);
}
