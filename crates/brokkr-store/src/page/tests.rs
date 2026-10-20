//! The fleet's page (decision 0089 ruling 6): each range a search of the
//! page index that visits only what it returns, merged into one order,
//! checked against its run's head, and read whole where it is cut.

use super::*;
use crate::test_support::{generate, plant_event, unmigrate, Generated, Standing, Width};
use crate::tests::store;
use brokkr_core::stamp::parse_millis;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const NOW: &str = "2026-10-20T00:00:00Z";
const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

fn now() -> i64 {
    parse_millis(NOW).unwrap()
}

/// The stamp `ms` milliseconds after the epoch.
fn stamp(ms: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}

/// `count` runs named `prefix-<n>` of `standing`, the n-th last active
/// `first` minus n `step`s before now.
fn runs(prefix: &str, standing: Standing, count: usize, first: i64, step: i64) -> Vec<Generated> {
    (0..count)
        .map(|n| Generated {
            run_id: format!("{prefix}-{n:05}"),
            standing,
            events: 3,
            last_at: stamp(now() - first - step * n as i64),
            width: Width::Narrow,
        })
        .collect()
}

fn lane(lane: Lane, bound: Option<Bound>) -> Range {
    Range { lane, bound }
}

/// The ranges of "needs you", "running" and "older", at three hours'
/// staleness and a day.
fn needs_you() -> [Range; 3] {
    [
        lane(Lane::Refused, None),
        lane(Lane::Parked, None),
        lane(Lane::Running, Some(Bound::Below(now() - 3 * HOUR))),
    ]
}

fn older() -> [Range; 2] {
    [
        lane(Lane::Finished, Some(Bound::AtOrBelow(now() - DAY))),
        lane(Lane::Acknowledged, None),
    ]
}

/// A journal of `size` runs, 45% of them acknowledged refusals, ahead of
/// fifty unacknowledged ones: every run within a page and two of the
/// deepest page read is the same at every size, and a larger journal
/// adds acknowledged refusals and older finished runs past that reach.
fn journal(size: usize) -> (tempfile::TempDir, Store) {
    let acknowledged = size * 45 / 100;
    let mut all = runs("refused", Standing::Refused, 50, 4 * HOUR, MINUTE);
    all.extend(runs("parked", Standing::Parked, 30, HOUR, 2 * MINUTE));
    all.extend(runs("stale", Standing::Running, 30, 5 * HOUR, MINUTE));
    all.extend(runs("fresh", Standing::Running, 20, 10 * MINUTE, MINUTE));
    all.extend(runs("recent", Standing::Finished, 40, 2 * HOUR, MINUTE));
    all.extend(runs(
        "ack",
        Standing::Acknowledged,
        acknowledged,
        HOUR,
        MINUTE,
    ));
    let rest = size - all.len();
    all.extend(runs("older", Standing::Finished, rest, 2 * DAY, MINUTE));
    let (dir, store) = store();
    generate(&store, &all).unwrap();
    (dir, store)
}

/// The run ids of `paged`, in order.
fn ids(paged: &[Paged]) -> Vec<&str> {
    paged.iter().map(|row| place_of(row).1).collect()
}

/// `ranges` read below `after`, with each statement's steps.
fn stepped(store: &Store, ranges: &[Range], after: Option<(i64, &str)>) -> (Vec<String>, Vec<i32>) {
    let mut steps = Vec::new();
    let read = store
        .section_with(ranges, after, 100, |_, taken| steps.push(taken))
        .unwrap();
    (ids(&read).into_iter().map(str::to_string).collect(), steps)
}

/// The first page of "needs you" and a page deep in "older", each read and
/// stepped, in a journal of `size` runs.
fn pages(size: usize) -> Vec<(Vec<String>, Vec<i32>)> {
    let (_dir, store) = journal(size);
    let first = stepped(&store, &needs_you(), None);
    let deep = store.section(&older(), None, 200).unwrap();
    let place = place_of(&deep[199]);
    let deeper = stepped(&store, &older(), Some(place));
    let running = [lane(
        Lane::Running,
        Some(Bound::AtOrAbove(now() - 3 * HOUR)),
    )];
    let recent = [lane(Lane::Finished, Some(Bound::Above(now() - DAY)))];
    vec![
        first,
        deeper,
        stepped(&store, &running, None),
        stepped(&store, &recent, None),
    ]
}

/// Ruling 9's independence: every range visits the rows it returns and one
/// more, whatever lies past them, so the same page costs the same steps at
/// 2,000 runs as at 20,000, with nine thousand acknowledged refusals more.
#[test]
fn a_page_visits_the_same_rows_at_every_journal_size() {
    let small = pages(2_000);
    let large = pages(20_000);
    assert_eq!(small, large);
    // A park newer than every refusal lists first, however many refusals
    // there are, and the stale runs older than all of them last.
    let expected: Vec<String> = [("parked", 30), ("refused", 50), ("stale", 20)]
        .into_iter()
        .flat_map(|(prefix, count)| (0..count).map(move |n| format!("{prefix}-{n:05}")))
        .collect();
    assert_eq!(small[0].0, expected);
    assert_eq!(small[0].1.len(), 3);
    // Deep in "older", by cursor: the acknowledged runs active within the
    // day list before the finished ones a day old.
    assert_eq!(small[1].0.first().map(String::as_str), Some("ack-00200"));
}

/// The plan SQLite gives `range` read below `after`, each line's detail.
fn plan(store: &Store, range: &Range, after: Option<(i64, &str)>) -> Vec<String> {
    let (filter, bound) = filter(range, after);
    let sql =
        format!("EXPLAIN QUERY PLAN SELECT {COLUMNS} FROM run_summary WHERE {filter} {ORDER}");
    let mut statement = store.conn.prepare(&sql).unwrap();
    statement
        .query_map(values(&bound).as_slice(), |row| row.get(3))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Ruling 9: each range is a search of the page index, never a scan or a
/// sort, under every bound and the cursor.
#[test]
fn every_range_is_a_search_of_the_page_index() {
    let (_dir, store) = journal(2_000);
    let at = now();
    let cursor = Some((at, "run"));
    let search = |detail: &str| {
        vec![format!(
            "SEARCH run_summary USING INDEX run_summary_page ({detail})"
        )]
    };
    let forms = [
        (lane(Lane::Refused, None), None, "lane=?"),
        (
            lane(Lane::Running, Some(Bound::Below(at))),
            None,
            "lane=? AND order_at<?",
        ),
        (
            lane(Lane::Running, Some(Bound::AtOrAbove(at))),
            None,
            "lane=? AND order_at>?",
        ),
        (
            lane(Lane::Finished, Some(Bound::Above(at))),
            None,
            "lane=? AND order_at>?",
        ),
        (
            lane(Lane::Finished, Some(Bound::AtOrBelow(at))),
            None,
            "lane=? AND order_at<?",
        ),
        (
            lane(Lane::Acknowledged, None),
            cursor,
            "lane=? AND (order_at,run_id)<(?,?)",
        ),
        (
            lane(Lane::Finished, Some(Bound::Above(at))),
            cursor,
            "lane=? AND order_at>? AND (order_at,run_id)<(?,?)",
        ),
    ];
    for (range, after, detail) in forms {
        assert_eq!(
            plan(&store, &range, after),
            search(detail),
            "{range:?} after {after:?}"
        );
    }
}

/// One run of `standing` and `width`, four events, in a fresh journal.
fn one(width: Width, standing: Standing) -> (tempfile::TempDir, Store) {
    let run = Generated {
        run_id: "run".to_string(),
        standing,
        events: 4,
        last_at: NOW.to_string(),
        width,
    };
    let (dir, store) = store();
    generate(&store, std::slice::from_ref(&run)).unwrap();
    (dir, store)
}

/// The one row of `standing`'s lane, read as a page reads it.
fn read_one(store: &Store, standing: Standing) -> Row {
    let mut read = store
        .section(&[lane(standing.lane(), None)], None, 2)
        .unwrap();
    assert_eq!(read.len(), 1);
    read.remove(0).unwrap()
}

/// Ruling 5: a row stands while its run's last event is the one it was
/// folded to, by seq and hash, and not once another lands.
#[test]
fn a_row_stands_at_its_head_until_an_event_lands_past_it() {
    let (_dir, store) = one(Width::Narrow, Standing::Running);
    let row = read_one(&store, Standing::Running);
    assert_eq!(row.folded, (4, format!("{:064}", 4)));
    let folded = (row.folded.0, row.folded.1.as_str());
    assert!(store.stands_at("run", folded).unwrap());
    assert!(!store.stands_at("run", (4, &format!("{:064}", 3))).unwrap());
    plant_event(&store.conn, "run", &5, &"f".repeat(64), "{}").unwrap();
    assert!(!store.stands_at("run", folded).unwrap());
}

/// The behind lane, newest first, at most as many as asked, and the rest
/// counted by the index.
#[test]
fn the_behind_lane_lists_its_newest_and_counts_the_rest() {
    let (_dir, store) = store();
    let behind = runs("behind", Standing::Running, 3, HOUR, MINUTE);
    generate(&store, &behind).unwrap();
    for run in &behind {
        plant_event(&store.conn, &run.run_id, &4, &"f".repeat(64), "{}").unwrap();
    }
    let newest = vec!["behind-00000".to_string(), "behind-00001".to_string()];
    assert_eq!(store.behind(2).unwrap(), (newest, 1));
    assert_eq!(store.behind(5).unwrap().1, 0);
    assert_eq!(store.counted(&lane(Lane::Behind, None)).unwrap(), 3);
    let running = lane(Lane::Running, Some(Bound::Below(now())));
    assert_eq!(store.counted(&running).unwrap(), 0);
}

/// A narrow row reads whole as it reads on a page; a wide one keeps each
/// value cut on the page and whole by its key, with every finding.
#[test]
fn a_row_reads_its_whole_values_by_its_key() {
    let (_dir, store) = one(Width::Narrow, Standing::Parked);
    let mut row = read_one(&store, Standing::Parked);
    let paged = row.clone();
    let whole = store.whole(&mut row).unwrap().unwrap();
    assert_eq!(row, paged);
    assert_eq!(whole.feature, "run feature\nin detail");
    assert_eq!((whole.residuals.len(), whole.acknowledged), (1, None));
    let (_dir, store) = one(Width::Wide, Standing::Parked);
    let mut row = read_one(&store, Standing::Parked);
    let wide = "w".repeat(1 << 20);
    assert_eq!(row.phase.as_ref().unwrap().cut, wide[..64]);
    assert_eq!(row.park_reason.as_ref().unwrap().cut, wide[..256]);
    assert_eq!(row.feature_line, wide[..1024]);
    let whole = store.whole(&mut row).unwrap().unwrap();
    let ruled = row.ruled.as_ref().unwrap();
    let wholes = [
        &row.phase.as_ref().unwrap().whole,
        &row.park_reason.as_ref().unwrap().whole,
        &ruled.from.whole,
        &ruled.rule_id.as_ref().unwrap().whole,
        &row.hired.as_ref().unwrap().seat.whole,
    ];
    assert!(wholes
        .iter()
        .all(|value| value.as_deref() == Some(wide.as_str())));
    let read = (whole.feature.len(), whole.residuals.len());
    assert_eq!(read, (1 << 20, 10_000));
}

/// A refused row reads its words and the acknowledgement's copy cut, and
/// its words and the acknowledgement whole by its key.
#[test]
fn an_acknowledged_refusal_reads_its_copy_and_its_words_whole() {
    let (_dir, store) = one(Width::Wide, Standing::Acknowledged);
    let mut row = read_one(&store, Standing::Acknowledged);
    assert_eq!((row.status, row.phase.as_ref()), (None, None));
    let copied = row.acknowledged.as_ref().unwrap();
    assert_eq!((copied.operator.len(), copied.reason.len()), (64, 256));
    assert_eq!(copied.at, "2026-01-01T00:00:00Z");
    let standing = store.whole(&mut row).unwrap().unwrap().acknowledged;
    let standing = standing.unwrap();
    let wide = (standing.operator.len(), standing.reason.len());
    assert_eq!(wide, (1 << 20, 1 << 20));
    let refused = row.refused.unwrap();
    assert_eq!((refused.seq, refused.words.cut.len()), (4, 512));
    let whole = refused.words.whole.map(|words| words.len());
    assert_eq!(whole, Some(1 << 20));
}

/// A value no writer writes is refused at the edge, at its place.
#[test]
fn a_row_no_writer_writes_is_refused_at_the_edge() {
    let null = "Invalid column type Null at index: 2, name: feature_line";
    let edits = [
        ("status = 'paused'", RowRefusal::Status("paused".into())),
        (
            "residual_open = 'dire'",
            RowRefusal::Severity("dire".into()),
        ),
        ("feature_line = NULL", RowRefusal::Column(null.into())),
    ];
    // A status no writer words lists as finished, as the lane reads it.
    let running = [lane(Lane::Running, None), lane(Lane::Finished, None)];
    for (edit, why) in edits {
        let (_dir, store) = one(Width::Narrow, Standing::Running);
        let edit = format!("UPDATE run_summary SET {edit}");
        store.conn.execute_batch(&edit).unwrap();
        let read = store.section(&running, None, 1).unwrap();
        let run_id = "run".to_string();
        assert_eq!(
            read,
            [Err(Unread {
                run_id,
                order_at: now(),
                why
            })]
        );
    }
    let (_dir, store) = one(Width::Narrow, Standing::Running);
    let long = format!("UPDATE run_summary SET run_id = '{}'", "r".repeat(200));
    store.conn.execute_batch(&long).unwrap();
    let read = store.section(&running, None, 1).unwrap();
    let why = &read[0].as_ref().unwrap_err().why;
    assert_eq!(why, &RowRefusal::RunIdPastBound);
    let words = "its run id is past 128 bytes, which no writer admits";
    assert_eq!(why.to_string(), words);
    // Its key is its whole id, which every cursor compares (ruling 6).
    assert_eq!(place_of(&read[0]).1, "r".repeat(200));
    let (_dir, store) = one(Width::Narrow, Standing::Parked);
    let unstated = r#"UPDATE run_summary SET residuals =
        replace(residuals, '"phase":"review"', '"phase":"design"')"#;
    store.conn.execute_batch(unstated).unwrap();
    let refused = store.whole(&mut read_one(&store, Standing::Parked));
    let words = "phase 'design' carries no residual finding at line 1 column 234";
    assert_eq!(refused.unwrap(), Err(RowRefusal::Residuals(words.into())));
}

/// A journal is paged once it carries the summary and its backfill
/// finished; a reader of one that does not reads it whole (ruling 8).
#[test]
fn a_journal_pages_only_once_its_summary_is_backfilled() {
    let (_dir, store) = store();
    assert!(store.pages().unwrap());
    let path = store.path().to_path_buf();
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    let unfinished = "DELETE FROM meta WHERE key = 'run_summary_backfill'";
    conn.execute_batch(unfinished).unwrap();
    assert!(!Store::open_read_only(&path).unwrap().pages().unwrap());
    unmigrate(&conn).unwrap();
    assert!(!Store::open_read_only(&path).unwrap().pages().unwrap());
}

/// What `brokkr runs` lists a run by: its `runs` row, or the refusal of a
/// run the journal does not carry.
#[test]
fn a_listing_is_read_by_its_run_s_key() {
    let (_dir, store) = one(Width::Narrow, Standing::Finished);
    let listed = (
        "run feature\nin detail".to_string(),
        "2026-01-01T00:00:00Z".to_string(),
    );
    assert_eq!(store.listing("run").unwrap(), listed);
    let missing = store.listing("nope").unwrap_err();
    assert!(matches!(missing, StoreError::RunNotFound(ref run) if run == "nope"));
}
