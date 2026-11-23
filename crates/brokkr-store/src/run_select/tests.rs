use std::path::Path;

use rusqlite::params;

use super::{intact, Store};
use crate::test_support::plant_run;

/// Plant `run_id` created at `at`, as a journal from before the writers
/// left it: the row the selector reads, no fold and no sorted list.
fn plant(conn: &rusqlite::Connection, run_id: &str, at: &str) {
    plant_run(conn, run_id, "{}").unwrap();
    conn.execute(
        "UPDATE runs SET created_at = ?1 WHERE run_id = ?2",
        params![at, run_id],
    )
    .unwrap();
}

/// The runs `list_runs` lists, as the selector reads them.
fn listed(store: &Store) -> Vec<(String, String)> {
    store
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|(run_id, _feature, created_at)| (run_id, created_at))
        .collect()
}

/// What `read` costs SQLite, in virtual-machine steps: the read counts
/// its own statements' steps, so the number is the statement's work
/// rather than the wall clock's.
fn steps<T>(store: &Store, read: impl FnOnce() -> T) -> (T, i64) {
    let read = read();
    (read, store.take_selector_steps())
}

/// A journal of `count` runs named `run-000000` onwards, each with its
/// own stamp, and one run named `special-000000`. The rows are planted
/// directly because the read's cost is what is measured, not a writer's.
fn planted(db: &Path, count: u32) -> Store {
    let mut store = Store::open(db).unwrap();
    let tx = store.conn.transaction().unwrap();
    for n in 0..count {
        plant(
            &tx,
            &format!("run-{n:06}"),
            &format!("2026-01-01T00:00:00.{n:09}Z"),
        );
    }
    plant(&tx, "special-000000", "2026-01-01T00:00:01.000000000Z");
    tx.commit().unwrap();
    store
}

/// [`planted`] twice in `dir`: a journal of ten runs and one of ten
/// thousand, so a read's cost is measured against the journal's size.
fn small_and_large(dir: &Path) -> (Store, Store) {
    let small = planted(&dir.join("small.db"), 10);
    (small, planted(&dir.join("large.db"), 10_000))
}

/// `latest` takes the LAST of equal maxima in `list_runs`' order, which
/// is the row inserted last on a `created_at` tie.
#[test]
fn latest_takes_the_last_run_list_runs_lists_on_a_stamp_tie() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    plant(&store.conn, "tie-older", "2026-01-01T00:00:00Z");
    plant(&store.conn, "tie-first", "2026-01-01T00:00:01Z");
    plant(&store.conn, "tie-second", "2026-01-01T00:00:01Z");

    assert_eq!(
        listed(&store),
        vec![
            ("tie-older".to_string(), "2026-01-01T00:00:00Z".to_string()),
            ("tie-first".to_string(), "2026-01-01T00:00:01Z".to_string()),
            ("tie-second".to_string(), "2026-01-01T00:00:01Z".to_string()),
        ],
        "list_runs breaks a stamp tie by insertion order"
    );
    assert_eq!(
        store.latest_run().unwrap(),
        Some(("tie-second".to_string(), "2026-01-01T00:00:01Z".to_string())),
        "the one-row read takes the tie's last run, as resolve does"
    );
}

/// A prefix lists its candidates in `list_runs`' order, never in id
/// order, and an exact id wins even where it is also another id's prefix.
#[test]
fn a_prefix_lists_its_candidates_in_list_runs_order_and_an_exact_id_wins() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    plant(&store.conn, "run-charlie", "2026-01-01T00:00:00Z");
    plant(&store.conn, "run-alpha", "2026-01-01T00:00:01Z");
    plant(&store.conn, "run-bravo", "2026-01-01T00:00:02Z");
    plant(&store.conn, "run-al", "2026-01-01T00:00:03Z");

    assert_eq!(
        store.run_candidates("run-").unwrap(),
        vec![
            (
                "run-charlie".to_string(),
                "2026-01-01T00:00:00Z".to_string()
            ),
            ("run-alpha".to_string(), "2026-01-01T00:00:01Z".to_string()),
            ("run-bravo".to_string(), "2026-01-01T00:00:02Z".to_string()),
            ("run-al".to_string(), "2026-01-01T00:00:03Z".to_string()),
        ],
        "the candidates are in the order list_runs lists them"
    );
    assert_eq!(
        store.run_candidates("run-al").unwrap(),
        vec![("run-al".to_string(), "2026-01-01T00:00:03Z".to_string())],
        "the exact id reads itself, not the ids it prefixes"
    );
    assert_eq!(
        store.run_candidates("").unwrap(),
        Vec::new(),
        "a blank selector reads nothing; resolve refuses it"
    );
}

/// The prefix is a key range, never a `LIKE` pattern: `_` and `%` are
/// ordinary characters in an id.
#[test]
fn a_prefix_is_a_key_range_not_a_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    plant(&store.conn, "r_ne", "2026-01-01T00:00:00Z");
    plant(&store.conn, "rXne", "2026-01-01T00:00:01Z");
    plant(&store.conn, "100%done", "2026-01-01T00:00:02Z");
    plant(&store.conn, "100xdone", "2026-01-01T00:00:03Z");

    assert_eq!(
        store.run_candidates("r_").unwrap(),
        vec![("r_ne".to_string(), "2026-01-01T00:00:00Z".to_string())],
        "'_' matched no other id"
    );
    assert_eq!(
        store.run_candidates("100%").unwrap(),
        vec![("100%done".to_string(), "2026-01-01T00:00:02Z".to_string())],
        "'%' matched no other id"
    );
}

/// The index is additive: a journal without it, read-only, resolves
/// correctly and is left without it; the next writable open adds it, and
/// the recorded schema stays 1.
#[test]
fn a_read_only_open_without_the_index_resolves_and_leaves_it_absent() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    {
        let store = Store::open(&db).unwrap();
        assert!(
            intact(&store.conn).unwrap(),
            "a fresh journal has the index"
        );
        plant(&store.conn, "run-older", "2026-01-01T00:00:00Z");
        plant(&store.conn, "run-newest", "2026-01-01T00:00:01Z");
        store
            .conn
            .execute_batch("DROP INDEX runs_created_at")
            .unwrap();
        assert!(!intact(&store.conn).unwrap(), "the journal predates it");
    }

    let read = Store::open_read_only(&db).unwrap();
    assert!(
        !intact(&read.conn).unwrap(),
        "a read-only open creates no index"
    );
    assert_eq!(
        read.latest_run().unwrap(),
        Some(("run-newest".to_string(), "2026-01-01T00:00:01Z".to_string())),
        "without the index, latest still scans to the right row"
    );
    assert_eq!(
        read.run_candidates("run-").unwrap().len(),
        2,
        "without the index, a prefix still reads its candidates"
    );
    let schema: String = read
        .conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'database_schema'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        schema, "1",
        "the index is additive; the schema does not move"
    );
    drop(read);

    let write = Store::open(&db).unwrap();
    assert!(
        intact(&write.conn).unwrap(),
        "a writable open adds the index"
    );
}

/// `latest`, an exact id and a unique prefix each cost one page whatever
/// the journal's size; a prefix with ten matches costs the same ten rows
/// in either journal, and the cost grows with the matches.
#[test]
fn latest_exact_and_a_unique_prefix_cost_a_page_and_an_ambiguous_one_its_matches() {
    let dir = tempfile::tempdir().unwrap();
    let (small, large) = small_and_large(dir.path());

    let (_, small_latest) = steps(&small, || small.latest_run().unwrap());
    let (_, large_latest) = steps(&large, || large.latest_run().unwrap());
    assert_eq!(large_latest, small_latest, "latest reads one row");

    let (_, small_exact) = steps(&small, || small.run_candidates("run-000005").unwrap());
    let (_, large_exact) = steps(&large, || large.run_candidates("run-000005").unwrap());
    assert_eq!(large_exact, small_exact, "an exact id reads one key");

    let (_, small_unique) = steps(&small, || small.run_candidates("special-").unwrap());
    let (_, large_unique) = steps(&large, || large.run_candidates("special-").unwrap());
    assert_eq!(
        large_unique, small_unique,
        "a unique prefix reads its one match"
    );

    // Ten matches in a journal a thousand times larger: the range reads
    // ten rows either way.
    let (_, small_ten) = steps(&small, || small.run_candidates("run-").unwrap());
    let (_, large_ten) = steps(&large, || large.run_candidates("run-00000").unwrap());
    assert_eq!(
        large_ten, small_ten,
        "ten matches cost the same in either journal"
    );

    let (_, large_all) = steps(&large, || large.run_candidates("run-").unwrap());
    assert!(
        large_all > large_ten,
        "an ambiguous prefix's cost grows with its matches: {large_all} against {large_ten}"
    );
}

/// A completer's prefix read takes at most the matches it asks for, so an
/// ambiguous prefix costs the same two rows in a journal of ten runs and in
/// one of ten thousand; an exact id and a unique prefix answer as before.
#[test]
fn a_bounded_prefix_read_costs_its_bound_whatever_the_prefix_names() {
    let dir = tempfile::tempdir().unwrap();
    let (small, large) = small_and_large(dir.path());

    let (small_two, small_steps) =
        steps(&small, || small.run_candidates_within("run-", 2).unwrap());
    let (large_two, large_steps) =
        steps(&large, || large.run_candidates_within("run-", 2).unwrap());
    assert_eq!(
        (small_two.len(), large_two.len()),
        (2, 2),
        "the bound is the page"
    );
    assert_eq!(
        large_steps, small_steps,
        "two matches cost the same in either journal"
    );
    let (_, large_all) = steps(&large, || large.run_candidates("run-").unwrap());
    assert!(
        large_all > large_steps * 100,
        "{large_all} against {large_steps}"
    );

    assert_eq!(
        large.run_candidates_within("run-000005", 2).unwrap(),
        large.run_candidates("run-000005").unwrap(),
        "an exact id wins"
    );
    assert_eq!(
        large.run_candidates_within("special-", 2).unwrap(),
        large.run_candidates("special-").unwrap(),
        "a unique prefix names its one run"
    );
    assert_eq!(large.run_candidates_within("", 2).unwrap(), Vec::new());
    assert_eq!(large.run_candidates_within("zzz", 2).unwrap(), Vec::new());
}

/// The completer's page (decision 0119 ruling 4): the newest runs in
/// `latest`'s order, ties to the row inserted last, each with the feature
/// line its summary folded, and no more than the limit.
#[test]
fn recent_runs_lists_the_newest_first_with_their_feature_lines_within_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    plant(&store.conn, "run-old", "2026-01-01T00:00:00Z");
    plant(&store.conn, "tie-first", "2026-01-01T00:00:01Z");
    plant(&store.conn, "tie-second", "2026-01-01T00:00:01Z");
    store
        .conn
        .execute(
            "UPDATE run_summary SET feature_line = 'the old one' WHERE run_id = 'run-old'",
            [],
        )
        .unwrap();

    assert_eq!(
        store.recent_runs(50).unwrap(),
        vec![
            ("tie-second".to_string(), None),
            ("tie-first".to_string(), None),
            ("run-old".to_string(), Some("the old one".to_string())),
        ],
        "newest first, a tie to the later row, the folded line beside its run"
    );
    assert_eq!(
        store.recent_runs(2).unwrap(),
        vec![
            ("tie-second".to_string(), None),
            ("tie-first".to_string(), None)
        ],
        "the limit is the page"
    );
}

/// A page of fifty costs the same in a journal of sixty runs and in one
/// of ten thousand: the read stops at the page.
#[test]
fn recent_runs_costs_one_page_whatever_the_journal_holds() {
    let dir = tempfile::tempdir().unwrap();
    let small = planted(&dir.path().join("small.db"), 60);
    let large = planted(&dir.path().join("large.db"), 10_000);

    let (small_page, small_steps) = steps(&small, || small.recent_runs(50).unwrap());
    let (large_page, large_steps) = steps(&large, || large.recent_runs(50).unwrap());
    assert_eq!((small_page.len(), large_page.len()), (50, 50));
    assert_eq!(
        large_page[0].0, "special-000000",
        "the newest run leads the page"
    );
    assert_eq!(large_steps, small_steps, "the page reads fifty rows");
}

/// `run`'s first `count` seat rows, each its own phase and seat, every
/// third a panel member's, planted directly as [`planted`]'s runs are.
fn seated(store: &mut Store, run: &str, count: u32) {
    let tx = store.conn.transaction().unwrap();
    for n in 0..count {
        let member = (n % 3 == 0).then(|| format!("member-{n}"));
        tx.execute(
            "INSERT INTO seat_summary (run_id, opened, phase, seat, member, digest)
             VALUES (?1, ?2, ?3, ?4, ?5, 'planted')",
            params![
                run,
                n,
                format!("phase-{n:05}"),
                format!("seat-{n:05}"),
                member
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
}

/// The completer's phase and seat page (decision 0119 ruling 4): the
/// run's own seat rows, in the order they opened, within the limit, and
/// none of another run's.
#[test]
fn run_seats_lists_the_runs_own_rows_in_their_order_within_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    seated(&mut store, "run-a", 3);
    seated(&mut store, "run-b", 2);
    let names = |n: u32, member: Option<&str>| super::SeatNames {
        phase: Some(format!("phase-{n:05}")),
        seat: Some(format!("seat-{n:05}")),
        member: member.map(str::to_string),
    };
    assert_eq!(
        store.run_seats("run-a", 50).unwrap(),
        vec![names(0, Some("member-0")), names(1, None), names(2, None)]
    );
    assert_eq!(
        store.run_seats("run-a", 2).unwrap(),
        vec![names(0, Some("member-0")), names(1, None)],
        "the limit is the page"
    );
    assert_eq!(store.run_seats("run-c", 50).unwrap(), vec![]);
}

/// A page of fifty seat rows costs the same in a run of sixty and in one
/// of ten thousand: the read stops at the page.
#[test]
fn run_seats_costs_one_page_whatever_the_run_holds() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    seated(&mut store, "small", 60);
    seated(&mut store, "large", 10_000);

    let (small, small_steps) = steps(&store, || store.run_seats("small", 50).unwrap());
    let (large, large_steps) = steps(&store, || store.run_seats("large", 50).unwrap());
    assert_eq!((small.len(), large.len()), (50, 50));
    assert_eq!(large_steps, small_steps, "the page reads fifty rows");
}
