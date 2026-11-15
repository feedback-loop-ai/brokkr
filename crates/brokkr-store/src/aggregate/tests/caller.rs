//! The median's pass and the preceding window's count as their caller
//! reads them (decision 0089 ruling 7, slice 4's unit U5b1): one row a
//! stored realm key, so a NULL realm is named after the caller's label and
//! a cut realm lands in its cut group, and neither counts a stale run.

use brokkr_core::stamp::parse_millis;

use super::*;

/// A journal holding one shipped run under `realm`, finished in the
/// window, and the store that reads it.
fn journal(realm: Option<&str>) -> (tempfile::TempDir, Store) {
    let (dir, store) = store();
    let finished = chain("shipped", realm, 0, &finished_steps("a feature"));
    plant_older_writer(&store, "shipped", &finished);
    (dir, refolded(store))
}

/// The two windows the preceding count is read over: the run at `start`
/// finishes inside the one that holds it.
fn windows() -> (i64, i64) {
    let from = parse_millis("2026-01-01T00:00:00Z").unwrap();
    (from, from + 60_000)
}

/// A NULL-realm run is counted, and yielded as NULL, so the caller names
/// it the hearth's label and filters it with every other row.
#[test]
fn a_null_realm_run_is_counted_under_its_label() {
    let (_dir, store) = journal(None);
    let (from, to) = windows();
    assert_eq!(
        finished_groups(&store, from, to),
        [(None, 1)],
        "the NULL realm group"
    );
    assert_eq!(durations(&store, from, to), [(None, 7_000)], "its span");
}

/// A realm past [`NAME_BYTES`] is stored cut where the writer wrote it, so
/// both reads yield the same cut group `realm_in_scope` compares.
#[test]
fn a_realm_past_its_bound_lands_in_its_cut_group() {
    let long = format!("{}é{}", "r".repeat(NAME_BYTES - 1), "tail");
    let cut = Bounded::within(&long, NAME_BYTES).cut;
    assert!(cut.len() < NAME_BYTES, "the cut is at a character boundary");
    let (_dir, store) = journal(Some(&long));
    let (from, to) = windows();
    assert_eq!(
        finished_groups(&store, from, to),
        [(Some(cut.clone()), 1)],
        "the cut group the caller's scope compares"
    );
    assert_eq!(durations(&store, from, to), [(Some(cut), 7_000)]);
}

/// A stale-seat run is counted by neither read: the fallback names it, and
/// the caller reads it whole instead.
#[test]
fn a_stale_seat_run_is_counted_by_neither() {
    let (_dir, store) = journal(Some("realm"));
    store
        .conn
        .execute("DELETE FROM seat_head WHERE run_id = 'shipped'", [])
        .unwrap();
    let (from, to) = windows();
    assert_eq!(finished_groups(&store, from, to), Vec::new(), "not counted");
    assert_eq!(durations(&store, from, to), Vec::new(), "not measured");
    let mut named = Vec::new();
    store
        .window_fallbacks(from, to, None, |run| named.push(run.run_id.to_string()))
        .unwrap();
    assert_eq!(named, ["shipped"], "the whole read names it");
}

/// The preceding window's count agrees with the window's runs filtered the
/// same way in this test: a run finished in the window whose ending
/// completed is one count, and the groups sum to it. The comparison with
/// `brokkr-view`'s `shipped_in` over a whole read is U5b2's, when the
/// streamed source lands.
#[test]
fn the_preceding_count_agrees_with_the_window_runs_count() {
    let (_dir, store) = store();
    for (run_id, realm, start, steps) in [
        ("before", Some("alpha"), 0, finished_steps("before")),
        ("shipped", Some("alpha"), 120, finished_steps("shipped")),
        ("stopped", Some("beta"), 130, stopped_steps()),
    ] {
        let events = chain(run_id, realm, start, &steps);
        plant_older_writer(&store, run_id, &events);
    }
    let store = refolded(store);
    let (before_from, before_to) = windows();
    let (from, to) = (before_to, before_to + 120_000);
    for (from, to) in [(before_from, before_to), (from, to)] {
        let mut whole = 0u64;
        store
            .window_runs(from, to, RunOrder::Unordered, None, |run| {
                let inside = run.finished_ms.is_some_and(|at| at >= from && at < to);
                let ships = run
                    .ending
                    .is_some_and(|ending| ending.status() == Status::Completed);
                if inside && ships {
                    whole += 1;
                }
            })
            .unwrap();
        assert_eq!(
            finished_groups(&store, from, to)
                .into_iter()
                .map(|(_, count)| count)
                .sum::<u64>(),
            whole,
            "the groups sum to the whole read's shipped count, {from}..{to}"
        );
    }
}
