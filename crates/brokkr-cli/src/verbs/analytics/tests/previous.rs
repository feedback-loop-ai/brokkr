//! The preceding window's count on the row path (decision 0089 ruling 7,
//! slice 4's unit U5b2b1, U5b1's F4): the store's `finished_between`
//! leaves out the runs it leaves whole, and the hearth's whole read of
//! those runs counts them beside it, so the count is the whole read's.

use rusqlite::Connection;

use super::*;

/// A journal of one current stopped run in the Day window, a run an older
/// writer left behind its head and a run whose seat head is lost, each of
/// the last two completed in the window before.
fn behind_and_fallback(dir: &std::path::Path) -> std::path::PathBuf {
    let base = parse_millis(GENERATED_NOW).unwrap();
    let journal = small_journal(dir, "f4", 1);
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    let behind = sealed("f4-behind", &completed_run(base, -30 * 60));
    plant_older_writer(&store, "f4-behind", &behind);
    plant(&mut store, "f4-fallback", &completed_run(base, -31 * 60));
    drop(store);
    let writer = Connection::open(&journal).unwrap();
    let lost = writer
        .execute("DELETE FROM seat_head WHERE run_id = 'f4-fallback'", [])
        .unwrap();
    assert_eq!(lost, 1, "the fallback run's seat head is lost");
    journal
}

/// A behind run and a fallback run that completed in the preceding window
/// are in neither of the store's groups, and the row path's `previous`
/// still equals the whole read's, both runs counted.
#[test]
fn the_previous_count_is_the_whole_reads_with_runs_the_store_leaves_whole() {
    let dir = tempfile::tempdir().unwrap();
    let journal = behind_and_fallback(dir.path());
    let store = ready(&journal);
    assert_eq!(lane(&store, "f4-behind").as_deref(), Some("behind"));
    let mut fallbacks = Vec::new();
    store
        .window_fallbacks(0, 0, None, |run| fallbacks.push(run.run_id.to_string()))
        .unwrap();
    assert_eq!(fallbacks, ["f4-fallback"]);
    let query = Query {
        hearths: &[],
        range: Range::Day,
        realm: None,
        now: GENERATED_NOW,
    };
    let (_, before) = brokkr_view::windows(&query).unwrap();
    let mut grouped = 0;
    store
        .finished_between(before.from, before.to, |_, count| grouped += count)
        .unwrap();
    assert_eq!(grouped, 0, "the store leaves both runs out of its groups");
    drop(store);

    let rows = assert_rows_are_whole(&journal, "a behind and a fallback run before");
    assert_eq!(rows.headline.shipped.previous, 2, "both runs are counted");
    assert_reads_rows(&journal, "the hearth reads its rows");
}
