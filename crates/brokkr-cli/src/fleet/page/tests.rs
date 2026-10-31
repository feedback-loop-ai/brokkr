//! The fleet a page at a time (decision 0089 rulings 5, 6, 8 and 9): the
//! paged read lists every section as the whole read does, at every bound;
//! a run behind its summary is placed by its fold; a row its run does not
//! stand at is read whole; and a journal the summary does not page is
//! read the kept way.

use std::collections::HashSet;
use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::fold::fold;
use brokkr_core::stamp::parse_millis;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::test_support::{import_fixtures, plant_envelope, plant_run, unmigrate};
use brokkr_store::{Attribution, Store};
use brokkr_view::{RunRow, Section};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::*;
use crate::fleet::listing::{list_hearth_in, Lister};
use crate::fleet::{list_hearth, read_hearth, Called, ListedRun, AHEAD_ROWS, PAGE_ROWS};
use crate::tests::envelope_builder::EnvelopeBuilder;

pub(crate) const NOW: &str = "2026-10-20T00:00:00Z";
pub(crate) const MINUTE: i64 = 60_000;
pub(crate) const HOUR: i64 = 60 * MINUTE;
pub(crate) const DAY: i64 = 24 * HOUR;
/// The staleness bound `brokkr_view` holds a running run to.
const STALE: i64 = 3 * HOUR;

pub(crate) fn now() -> i64 {
    parse_millis(NOW).unwrap()
}

pub(crate) fn stamp(ms: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}

/// How a generated run stands once folded.
#[derive(Clone, Copy)]
pub(crate) enum Shape {
    Running,
    /// Reviewed and ruled to hold, its park not yet journaled.
    Held,
    Parked,
    Finished,
    Refused,
}

fn steps(shape: Shape) -> Vec<(EventType, Value)> {
    let started = (
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {}}),
    );
    let reviewed = [
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "reviewer"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a1"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e1", "result": {"result": "hold"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "review", "result": "hold", "rule_id": "REVIEW-HOLD",
                   "problem": "held", "inputs": {"max_residual_severity": "high"}}),
        ),
    ];
    let parked = (
        EventType::RunParked,
        json!({"reason": "REVIEW-HOLD for (review, hold): held"}),
    );
    let mut steps = vec![started];
    match shape {
        Shape::Running => {}
        Shape::Held => steps.extend(reviewed),
        Shape::Parked => steps.extend(reviewed.into_iter().chain([parked])),
        Shape::Finished => steps.push((EventType::RunCompleted, json!({}))),
        Shape::Refused => steps.push((
            EventType::OperatorAccepted,
            json!({"command_id": "none", "operator": "op", "reason": "r"}),
        )),
    }
    steps
}

/// `steps` chained onto `after` as run `run_id`, each stamped `at`.
fn chained(
    run_id: &str,
    after: (u64, String),
    steps: Vec<(EventType, Value)>,
    at: &str,
) -> Vec<EventEnvelope> {
    let (mut seq, mut previous) = after;
    let mut chain = Vec::new();
    for (event_type, payload) in steps {
        seq += 1;
        let event = EnvelopeBuilder::new(event_type, payload)
            .run(run_id)
            .seq(seq)
            .event_id(format!("{run_id}-{seq}"))
            .at(at)
            .previous(previous)
            .sealed();
        previous = event.event_hash.clone();
        chain.push(event);
    }
    chain
}

/// Plant run `run_id` of `shape`, last active `at`, as a writer from
/// before the summary leaves it: the next writable open folds it.
pub(crate) fn plant(journal: &rusqlite::Connection, run_id: &str, shape: Shape, at: &str) {
    plant_run(journal, run_id, "{}").unwrap();
    for event in chained(run_id, (0, ZERO_HASH.to_string()), steps(shape), at) {
        plant_envelope(journal, &event).unwrap();
    }
}

/// Every proper prefix of `id`, at a character boundary: the selectors a
/// run id answers to besides itself, for the selector's parity tests.
pub(crate) fn prefixes(id: &str) -> impl Iterator<Item = &str> + '_ {
    id.char_indices().skip(1).map(|(at, _)| &id[..at])
}

/// The selectors a journal's run ids answer to, plus `latest`, a blank
/// and a missing one: the cases both parity tests compare whole.
pub(crate) fn selector_cases(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut selectors = vec![
        crate::selector::LATEST.to_string(),
        String::new(),
        "nobody-at-all".to_string(),
    ];
    for run_id in ids {
        selectors.push(run_id.clone());
        selectors.extend(prefixes(&run_id).map(str::to_string));
    }
    selectors
}

/// Every fixture journal, imported where the import takes it and planted
/// where it does not, as the store's own fixture test reads them.
pub(crate) fn fixture_journal(db: &Path) {
    import_fixtures(&mut Store::open(db).unwrap());
}

/// The runs that split a paged read from a whole one (ruling 9): a park
/// and a stale run newer than a page and more of refusals, an old
/// refusal, a run stamped `+02:00`, one whose stamp does not read, an
/// acknowledged one active within the day, and runs either side of the
/// day. Each run a clock is set by, and its last activity in milliseconds.
pub(crate) fn generated(db: &Path) -> Vec<(&'static str, i64)> {
    let conn = rusqlite::Connection::open(db).unwrap();
    let at = |ago: i64| stamp(now() - ago);
    let keys = [
        ("park-new", HOUR),
        ("stale-new", STALE + 30 * MINUTE),
        ("fresh", 10 * MINUTE),
        ("refused-old", 30 * DAY),
        ("ack-recent", 2 * HOUR),
        ("done-recent", 5 * HOUR),
        ("done-old", 3 * DAY),
    ];
    let shapes = [
        Shape::Parked,
        Shape::Running,
        Shape::Running,
        Shape::Refused,
        Shape::Refused,
    ];
    for ((run_id, ago), shape) in keys
        .iter()
        .zip(shapes.into_iter().chain([Shape::Finished; 2]))
    {
        plant(&conn, run_id, shape, &at(*ago));
    }
    for n in 0..60 {
        plant(
            &conn,
            &format!("refused-{n:02}"),
            Shape::Refused,
            &at(4 * HOUR + n * MINUTE),
        );
    }
    plant(
        &conn,
        "offset",
        Shape::Finished,
        "2026-10-19T01:30:00+02:00",
    );
    plant(&conn, "unparsed", Shape::Finished, "not a time");
    drop(conn);
    let mut store = Store::open(db).unwrap();
    let head = store.head_hash("ack-recent").unwrap();
    let by = Attribution {
        operator: "op",
        reason: "seen",
    };
    store
        .acknowledge_quarantine("ack-recent", (head.0, &head.1), by)
        .unwrap();
    let offset = parse_millis("2026-10-19T01:30:00+02:00").unwrap();
    let mut keyed: Vec<(&str, i64)> = keys.iter().map(|(run, ago)| (*run, now() - ago)).collect();
    keyed.push(("offset", offset));
    keyed
}

/// The fleet read whole, its rows ordered by ruling 6's key.
pub(crate) fn whole_rows(db: &Path) -> Vec<RunRow> {
    let store = Store::open_read_only(db).unwrap();
    let listed = read_hearth(&store).listed().unwrap();
    let entries: Vec<_> = listed.iter().map(ListedRun::entry).collect();
    brokkr_view::run_rows(&entries).runs
}

/// `rows` as the whole read lists them by section at [`NOW`].
pub(crate) fn arranged(rows: &[RunRow]) -> Vec<RunRow> {
    let sections = brokkr_view::sections(rows, NOW).into_iter();
    sections
        .flat_map(|(_, rows)| rows.into_iter().cloned())
        .collect()
}

/// What `write` prints of the hearth at `db` at [`NOW`], listed as `brokkr
/// runs` lists it, in one read: every run, `batched` keys of the runs
/// behind past a refresh at a time, when it names them, else a page.
fn printed(
    db: &Path,
    batched: Option<usize>,
    write: impl FnOnce(&Lister<'_>, &mut Vec<u8>) -> anyhow::Result<()>,
) -> String {
    let store = Store::open_read_only(db).unwrap();
    let mut out = Vec::new();
    let journal = (Journal::at(db), NOW);
    list_hearth_in(&store, journal, batched, |lister| write(lister, &mut out)).unwrap();
    String::from_utf8(out).unwrap()
}

/// The table's style here: no colour, 120 columns.
pub(crate) const STYLE: crate::render::Style = crate::render::Style {
    color: false,
    width: 120,
};

/// Every run of the hearth at `db`, in its JSON and its table, `keys` of
/// the runs behind past a refresh keyed at a time, is `view`.
fn lists_every_run_as(db: &Path, keys: usize, view: &brokkr_view::RunsView) {
    let json = printed(db, Some(keys), |lister, out| lister.every_json(out));
    assert_eq!(
        json,
        format!("{}\n", serde_json::to_string_pretty(view).unwrap()),
        "{keys} keys a batch"
    );
    let text = printed(db, Some(keys), |lister, out| lister.every_text(out, &STYLE));
    assert_eq!(text, crate::render::runs(view, NOW, &STYLE));
}

/// What `rows` list under each section at `now`, by id.
fn sectioned(rows: &[RunRow], now: &str) -> Vec<(Section, Vec<String>)> {
    brokkr_view::sections(rows, now)
        .into_iter()
        .map(|(section, rows)| (section, rows.iter().map(|row| row.run_id.clone()).collect()))
        .collect()
}

/// Every page from the first, `limit` rows a page, at `now`: a walk that
/// reaches the fleet's end within a thousand pages.
fn paged(
    pages: &Pages,
    store: &Store,
    now: &str,
    limit: usize,
    whole: bool,
) -> Vec<(Section, RunRow)> {
    let mut rows = Vec::new();
    let mut after = None;
    let ask = Ask {
        now,
        whole: &|_| whole,
        keep: &|_| true,
    };
    for _ in 0..1_000 {
        let page = pages.page(store, ask, after.as_ref(), limit).unwrap();
        rows.extend(page.rows);
        match page.next {
            Some(next) => after = Some(next),
            None => return rows,
        }
    }
    let ids: Vec<&str> = rows.iter().map(|(_, row)| row.run_id.as_str()).collect();
    panic!("no end in a thousand pages; listed {ids:?}");
}

/// `rows` by section, in [`Section::ORDER`], as [`sectioned`] states them.
fn by_section(rows: &[(Section, RunRow)]) -> Vec<(Section, Vec<String>)> {
    Section::ORDER
        .into_iter()
        .map(|section| {
            let held = rows.iter().filter(|(listed, _)| *listed == section);
            (section, held.map(|(_, row)| row.run_id.clone()).collect())
        })
        .collect()
}

fn refreshed(db: &Path, behind: usize) -> (Store, Pages, usize) {
    let store = Store::open_read_only(db).unwrap();
    let mut pages = Pages::default();
    let read = pages.refresh(&store, Journal::at(db), behind).unwrap();
    (store, pages, read)
}

/// Ruling 9's `every_page_is_its_sections`: over every fixture journal
/// and the generated one, at a clock either side of every staleness and
/// day bound, the rows each section pages, in order, are the rows the
/// whole read lists there; and read with their values whole, every row
/// is the whole read's, byte for byte.
#[test]
fn every_page_is_its_sections() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    fixture_journal(&db);
    let keyed = generated(&db);
    let whole = whole_rows(&db);
    let (store, pages, _) = refreshed(&db, AHEAD_ROWS);
    assert!(matches!(pages, Pages::Summary(_)));
    let mut clocks = vec![now()];
    for (_, at) in keyed {
        for edge in [at + DAY - 1, at + DAY, at + STALE, at + STALE + 1] {
            clocks.push(edge);
        }
    }
    for clock in clocks {
        let now = stamp(clock);
        let listed = paged(&pages, &store, &now, 7, false);
        assert_eq!(by_section(&listed), sectioned(&whole, &now), "at {now}");
    }
    let listed = paged(&pages, &store, NOW, 50, true);
    let wire = |row: &RunRow| (row.run_id.clone(), serde_json::to_value(row).unwrap());
    let mut paged_wire: Vec<_> = listed.iter().map(|(_, row)| wire(row)).collect();
    let mut whole_wire: Vec<_> = whole.iter().map(wire).collect();
    paged_wire.sort_by(|a, b| a.0.cmp(&b.0));
    whole_wire.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(paged_wire, whole_wire);
    let first: Vec<&str> = listed
        .iter()
        .take(3)
        .map(|(_, row)| row.run_id.as_str())
        .collect();
    assert_eq!(first, ["park-new", "stale-new", "refused-00"]);
}

/// A fresh journal at `db`, its runs planted by `plant_with` and then
/// folded by a current writer's open.
pub(crate) fn folded(db: &Path, plant_with: impl FnOnce(&rusqlite::Connection)) {
    drop(Store::open(db).unwrap());
    plant_with(&rusqlite::Connection::open(db).unwrap());
    drop(Store::open(db).unwrap());
}

/// A fresh journal, folded, of one run, `done`, finished an hour before
/// [`NOW`]: its directory, and where the journal is.
fn done_an_hour_ago() -> (tempfile::TempDir, std::path::PathBuf) {
    finished_hours_ago(&["done"])
}

/// A fresh journal, folded, of the runs `ids`, the first finished an hour
/// before [`NOW`] and each after it an hour earlier: its directory, and
/// where the journal is.
fn finished_hours_ago(ids: &[&str]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        for (hours, run_id) in (1..).zip(ids) {
            plant(conn, run_id, Shape::Finished, &stamp(now() - hours * HOUR));
        }
    });
    (dir, db)
}

/// `event_type` appended to `run_id` as an older writer appends it: its
/// summary row is left behind its head.
fn appended_by_an_older_writer(db: &Path, run_id: &str, event_type: EventType, payload: Value) {
    appended_at(db, run_id, (event_type, payload), NOW);
}

/// [`appended_by_an_older_writer`], the event stamped `at`.
fn appended_at(db: &Path, run_id: &str, step: (EventType, Value), at: &str) {
    let head = Store::open_read_only(db)
        .unwrap()
        .head_hash(run_id)
        .unwrap();
    let conn = rusqlite::Connection::open(db).unwrap();
    for event in chained(run_id, head, vec![step], at) {
        plant_envelope(&conn, &event).unwrap();
    }
}

/// The generated runs at `db`, and 230 more a current writer folded
/// running and an older one then moved, so they stand behind their
/// summary: a quarter each refused, still running (fresh, or stale),
/// finished within the day and finished days ago. A refresh reads the
/// hundred last by run id; the rest list in every section, among rows
/// that stand at their heads.
pub(crate) fn behind_in_every_section(db: &Path) {
    drop(Store::open(db).unwrap());
    generated(db);
    folded(db, |conn| {
        for n in 0..230 {
            let at = stamp(now() - 10 * DAY);
            plant(conn, &format!("lag-{n:03}"), Shape::Running, &at);
        }
    });
    for n in 0..230 {
        let ago = n * 3 * MINUTE;
        let accepted = json!({"command_id": "none", "operator": "op", "reason": "r"});
        let (step, at) = match n % 4 {
            0 => ((EventType::OperatorAccepted, accepted), ago),
            1 => ((EventType::PhaseEntered, json!({"phase": "review"})), ago),
            2 => ((EventType::RunCompleted, json!({})), ago),
            _ => ((EventType::RunCompleted, json!({})), 2 * DAY + ago),
        };
        appended_at(db, &format!("lag-{n:03}"), step, &stamp(now() - at));
    }
}

/// Ruling 9's `a_run_behind_its_summary_is_placed_by_its_fold`: a run
/// summarized running, then parked by an append that wrote nothing
/// folded, lists under "needs you" as parked; and with 151 runs behind,
/// a refresh reads the newest 100 whole, counts the rest, and reads none
/// of them again while their heads stand.
#[test]
fn a_run_behind_its_summary_is_placed_by_its_fold() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        plant(conn, "held", Shape::Held, &stamp(now() - 10 * MINUTE));
        for n in 0..150 {
            let at = stamp(now() - HOUR - n * MINUTE);
            plant(conn, &format!("behind-{n:03}"), Shape::Running, &at);
        }
    });
    let reason = json!({"reason": "REVIEW-HOLD for (review, hold): held"});
    appended_by_an_older_writer(&db, "held", EventType::RunParked, reason);
    for n in 0..150 {
        let run_id = format!("behind-{n:03}");
        appended_by_an_older_writer(&db, &run_id, EventType::RunCompleted, json!({}));
    }
    let (store, mut pages, read) = refreshed(&db, AHEAD_ROWS);
    assert_eq!((read, pages.unread()), (7 + 99 * 2, 51));
    assert!(pages.was_read_whole("held") && !pages.was_read_whole("behind-149"));
    let listed = paged(&pages, &store, NOW, PAGE_ROWS, false);
    let (section, held) = &listed[0];
    assert_eq!(
        (*section, held.run_id.as_str()),
        (Section::NeedsYou, "held")
    );
    assert_eq!(held.verdict.standing, brokkr_view::Standing::Parked);
    assert_eq!(listed.len(), 100);
    assert_eq!(
        pages.refresh(&store, Journal::at(&db), AHEAD_ROWS).unwrap(),
        0
    );
    assert!(!pages.was_read_whole("held"));
    assert!(!pages.replaced(Journal::at(&db)));
    assert!(pages.replaced(None));
}

/// Ruling 5: a row whose run's last event is not the head it was folded
/// to, or that holds a value no writer writes, is read whole and listed
/// as its fold says, in the section its fold places it in, whichever
/// range its row was read from and on whichever page, a run a page; a
/// row whose run id no writer admits is listed refused, never read.
#[test]
fn a_row_its_run_does_not_stand_at_is_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        plant(conn, "edited", Shape::Parked, &stamp(now() - HOUR));
        plant(conn, "paused", Shape::Running, &stamp(now() - MINUTE));
        plant(conn, "unstated", Shape::Parked, &stamp(now() - 3 * DAY));
        plant_run(conn, &"r".repeat(200), "{}").unwrap();
    });
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch(&format!(
            "UPDATE run_summary SET folded_hash = '{hash}', phase = 'edited',
                 status = 'completed' WHERE run_id = 'edited';
             UPDATE run_summary SET status = 'paused' WHERE run_id = 'paused';
             UPDATE run_summary SET status = 'completed',
                 residuals = replace(residuals, '\"phase\":\"review\"', '\"phase\":\"design\"')
                 WHERE run_id = 'unstated';",
            hash = "f".repeat(64)
        ))
        .unwrap();
    let (store, pages, _) = refreshed(&db, AHEAD_ROWS);
    let listed = paged(&pages, &store, NOW, 1, true);
    let row = |run_id: &str| -> (Section, RunRow) {
        let found = listed
            .iter()
            .find(|(_, row)| row.run_id.starts_with(run_id));
        found.unwrap().clone()
    };
    // Its row reads finished, within the day: its fold is parked.
    let (section, edited) = row("edited");
    assert_eq!(section, Section::NeedsYou);
    assert_eq!(edited.phase.as_deref(), Some("review"));
    assert_eq!(edited.feature, "feat");
    assert_eq!(row("paused").1.status.as_deref(), Some("running"));
    // Its row reads finished, days old, its findings in no writer's
    // words: read whole, its fold is parked and its finding stated.
    let (section, unstated) = row("unstated");
    assert_eq!(section, Section::NeedsYou);
    assert_eq!(unstated.status.as_deref(), Some("awaiting_operator"));
    assert_eq!(unstated.residuals.len(), 1);
    let ids: Vec<&str> = listed.iter().map(|(_, row)| row.run_id.as_str()).collect();
    assert_eq!(ids.len(), 4, "{ids:?}");
    let long = row("rrrr").1;
    assert_eq!(long.run_id, format!("{}…", "r".repeat(128)));
    let words = "its run id is past 128 bytes, which no writer admits";
    assert_eq!(long.detail.as_deref(), Some(words));
    assert_eq!(long.quarantine, Some(brokkr_view::Quarantine::DoesNotLoad));
    // Listed at the place the writer gave it: an empty run, refused at
    // its creation.
    assert_eq!(long.order_at, parse_millis("2026-01-01T00:00:00Z").unwrap());
}

/// Ruling 6: a row whose run id no writer admits is listed by the cut of
/// its id and paged by its whole id, so a cursor past it names the place
/// the store compares: a fleet holding two such ids, at one time with a
/// third run, lists to its end a run a page, every run once.
#[test]
fn a_refused_over_long_id_pages_to_the_fleet_s_end() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let (first, second) = ("r".repeat(200), format!("{}s", "r".repeat(199)));
    folded(&db, |conn| {
        for run_id in [first.as_str(), &second, "empty"] {
            plant_run(conn, run_id, "{}").unwrap();
        }
        plant(conn, "done", Shape::Finished, &stamp(now() - HOUR));
    });
    let (store, pages, _) = refreshed(&db, AHEAD_ROWS);
    let walked = paged(&pages, &store, NOW, 1, false);
    let ids: Vec<&str> = walked.iter().map(|(_, row)| row.run_id.as_str()).collect();
    let cut = format!("{}…", "r".repeat(128));
    assert_eq!(ids, [cut.as_str(), &cut, "empty", "done"]);
}

/// Rulings 2 and 5: a run id no writer admits, standing behind its
/// summary, is listed refused wherever the behind lane hands it out: by
/// the refresh's first hundred, on the console's page, and by the keys
/// past them, in `brokkr runs` and `/api/runs`; and the fleet still lists
/// every run once, to its end.
#[test]
fn an_over_long_id_behind_its_summary_is_listed_refused_by_every_read() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    behind_in_every_section(&db);
    // Unfolded, the first stands at the lane's top; the second's row is
    // planted at the lane's oldest place, past the hundred a refresh reads.
    let (top, tail) = ("t".repeat(200), "u".repeat(200));
    let conn = rusqlite::Connection::open(&db).unwrap();
    for run_id in [&top, &tail] {
        plant_run(&conn, run_id, "{}").unwrap();
    }
    let oldest = "UPDATE run_summary SET last_at_ms = ?1 WHERE run_id = ?2";
    conn.execute(oldest, rusqlite::params![now() - 30 * DAY, tail])
        .unwrap();
    drop(conn);
    let cut = |run_id: &str| format!("{}…", &run_id[..128]);
    let words = "its run id is past 128 bytes, which no writer admits";
    let (store, pages, _) = refreshed(&db, AHEAD_ROWS);
    assert_eq!(pages.unread(), 132);
    let listed = paged(&pages, &store, NOW, PAGE_ROWS, false);
    let refused: Vec<(&str, Option<&str>)> = listed
        .iter()
        .filter(|(_, row)| row.run_id.len() > 128)
        .map(|(_, row)| (row.run_id.as_str(), row.detail.as_deref()))
        .collect();
    assert_eq!(refused, [(cut(&top).as_str(), Some(words))]);
    let mut every: Vec<String> = whole_rows(&db)
        .into_iter()
        .map(|row| match row.run_id.len() > 128 {
            true => cut(&row.run_id),
            false => row.run_id,
        })
        .collect();
    every.sort();
    let lists_every_run_refusing_both = |runs: &Value| {
        let runs = runs.as_array().unwrap();
        let mut ids: Vec<&str> = runs
            .iter()
            .map(|run| run["run_id"].as_str().unwrap())
            .collect();
        ids.sort();
        assert_eq!(ids, every);
        for run_id in [&top, &tail] {
            let shown = runs.iter().find(|run| run["run_id"] == cut(run_id));
            let shown = shown.unwrap();
            let read = (&shown["detail"], &shown["quarantine"], &shown["status"]);
            assert_eq!(read, (&json!(words), &json!("does_not_load"), &Value::Null));
        }
    };
    for keys in [3, TAIL_KEYS] {
        let json = printed(&db, Some(keys), |lister, out| lister.every_json(out));
        let printed: Value = serde_json::from_str(&json).unwrap();
        lists_every_run_refusing_both(&printed["runs"]);
    }
    let served = crate::ui::handle(&db, "/api/runs").body;
    let served: Value = serde_json::from_str(&served).unwrap();
    lists_every_run_refusing_both(&served);
}

/// Ruling 6: two run ids no writer admits, behind their summary and cut
/// to one shown id, one of them that cut itself, are kept between
/// refreshes by their whole ids: a second refresh holds each at its own
/// key, none of them read whole, and a page a run lists every run once.
#[test]
fn refused_ids_cut_alike_are_kept_by_their_whole_ids() {
    let (_dir, db) = done_an_hour_ago();
    let (cut, long) = (format!("{}…", "v".repeat(128)), "v".repeat(200));
    let conn = rusqlite::Connection::open(&db).unwrap();
    for run_id in [&cut, &long] {
        plant_run(&conn, run_id, "{}").unwrap();
    }
    drop(conn);
    let (store, mut pages, _) = refreshed(&db, AHEAD_ROWS);
    assert!(!pages.was_read_whole(&cut) && !pages.was_read_whole(&long));
    assert_eq!(
        pages.refresh(&store, Journal::at(&db), AHEAD_ROWS).unwrap(),
        0
    );
    let ask = Ask {
        now: NOW,
        whole: &|_| false,
        keep: &|_| true,
    };
    let (mut listed, mut keys, mut after) = (Vec::new(), Vec::new(), None);
    for _ in 0..4 {
        let page = pages.page(&store, ask, after.as_ref(), 1).unwrap();
        listed.extend(page.rows.into_iter().map(|(_, row)| row.run_id));
        keys.extend(page.next.iter().map(|next| next.run_id.clone()));
        after = page.next;
        if after.is_none() {
            break;
        }
    }
    assert_eq!(listed, [cut.as_str(), &cut, "done"]);
    assert_eq!(keys, [cut, long]);
}

/// Ruling 2: a run id no writer admits whose 128th byte falls inside a
/// character is listed by its bytes cut at the boundary below its bound,
/// and paged by its whole id.
#[test]
fn an_over_long_id_is_cut_at_the_character_boundary_below_its_bound() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let long = format!("{}{}", "r".repeat(127), "é".repeat(40));
    folded(&db, |conn| {
        plant_run(conn, &long, "{}").unwrap();
    });
    let (store, pages, _) = refreshed(&db, AHEAD_ROWS);
    let listed = paged(&pages, &store, NOW, PAGE_ROWS, false);
    let shown: Vec<(&str, &str)> = listed
        .iter()
        .map(|(_, row)| (row.run_id.as_str(), row.place().1))
        .collect();
    let cut = format!("{}…", "r".repeat(127));
    assert_eq!(shown, [(cut.as_str(), long.as_str())]);
}

/// Ruling 6: a run kept behind its summary whose head an older writer
/// moves between two refreshes is read whole again by the second, and
/// lists as its fold now says.
#[test]
fn a_behind_run_whose_head_moves_is_read_whole_again() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        plant(conn, "moving", Shape::Running, &stamp(now() - HOUR));
    });
    let review = json!({"phase": "review"});
    appended_by_an_older_writer(&db, "moving", EventType::PhaseEntered, review);
    let (store, mut pages, read) = refreshed(&db, AHEAD_ROWS);
    assert_eq!(read, 2);
    appended_by_an_older_writer(&db, "moving", EventType::RunCompleted, json!({}));
    let journal = Journal::at(&db);
    assert_eq!(pages.refresh(&store, journal, AHEAD_ROWS).unwrap(), 3);
    assert!(pages.was_read_whole("moving"));
    let listed = paged(&pages, &store, NOW, PAGE_ROWS, false);
    let rows: Vec<(Section, Option<&str>)> = listed
        .iter()
        .map(|(section, row)| (*section, row.status.as_deref()))
        .collect();
    assert_eq!(rows, [(Section::Recent, Some("completed"))]);
}

/// Rulings 3 and 6: an acknowledgement lands beside the chain, so a
/// quarantined run behind its summary that the operator acknowledges at
/// its unmoved head is read whole again by the next refresh and lists
/// under "older" with it, as the kept read lists it; and is kept after.
#[test]
fn a_behind_run_acknowledged_at_its_unmoved_head_lists_acknowledged() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    drop(Store::open(&db).unwrap());
    let conn = rusqlite::Connection::open(&db).unwrap();
    plant(&conn, "refused", Shape::Refused, &stamp(now() - MINUTE));
    let (store, mut pages, read) = refreshed(&db, AHEAD_ROWS);
    assert_eq!(read, 2);
    let listed = |pages: &Pages| {
        let rows = paged(pages, &store, NOW, PAGE_ROWS, false);
        let rows = rows.into_iter().map(|(section, row)| {
            let acknowledged = row.acknowledgement.map(|ack| ack.seq);
            (section, row.run_id, acknowledged)
        });
        rows.collect::<Vec<_>>()
    };
    let refused = "refused".to_string();
    assert_eq!(listed(&pages), [(Section::NeedsYou, refused.clone(), None)]);
    let (seq, hash) = store.head_hash("refused").unwrap();
    conn.execute(
        "INSERT INTO quarantine_acknowledgements
             (run_id, head_seq, head_hash, operator, reason, recorded_at)
         VALUES ('refused', ?1, ?2, 'op', 'read it', ?3)",
        rusqlite::params![seq, hash, NOW],
    )
    .unwrap();
    let journal = Journal::at(&db);
    assert_eq!(pages.refresh(&store, journal, AHEAD_ROWS).unwrap(), 2);
    assert!(pages.was_read_whole("refused"));
    assert_eq!(listed(&pages), [(Section::Older, refused, Some(seq))]);
    assert_eq!(pages.refresh(&store, journal, AHEAD_ROWS).unwrap(), 0);
}

/// Ruling 6: a run a refresh read whole as behind its summary, which a
/// writer catches up before the page is read, lists once: by the read
/// that kept it, never again from the range its row stands in now.
#[test]
fn a_behind_run_a_writer_catches_up_between_the_reads_lists_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        plant(conn, "caught", Shape::Running, &stamp(now() - HOUR));
        plant(conn, "beside", Shape::Finished, &stamp(now() - 2 * HOUR));
    });
    appended_by_an_older_writer(&db, "caught", EventType::RunCompleted, json!({}));
    let (store, pages, read) = refreshed(&db, AHEAD_ROWS);
    assert_eq!(read, 2, "the run behind, read whole");
    drop(Store::open(&db).unwrap());
    let lane = brokkr_store::test_support::lane;
    assert_eq!(lane(&store, "caught").as_deref(), Some("finished"));
    let listed = paged(&pages, &store, NOW, PAGE_ROWS, false);
    let ids: Vec<(Section, &str)> = listed
        .iter()
        .map(|(section, row)| (*section, row.run_id.as_str()))
        .collect();
    assert_eq!(
        ids,
        [(Section::Recent, "caught"), (Section::Recent, "beside")]
    );
}

/// Ruling 6: one page of `brokkr runs` or `/api/runs` reads at most a
/// page and one ahead of the runs behind their summary and counts the
/// rest, which its JSON states and its table prints as one line; every
/// run, asked of either, lists each run once.
#[test]
fn a_page_asked_reads_at_most_a_page_ahead_of_the_runs_behind() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        for n in 0..230 {
            let at = stamp(now() - HOUR - n * MINUTE);
            plant(conn, &format!("behind-{n:03}"), Shape::Running, &at);
        }
    });
    for n in 0..230 {
        let run_id = format!("behind-{n:03}");
        appended_by_an_older_writer(&db, &run_id, EventType::RunCompleted, json!({}));
    }
    let style = STYLE;
    let listed = |asked: (bool, Option<u32>)| {
        let mut out = Vec::new();
        let asked = (asked.0, asked.1, None);
        crate::verbs::readouts::hearth_runs(&db, asked, Called::default(), &style, &mut out)
            .unwrap();
        String::from_utf8(out).unwrap()
    };
    let page: Value = serde_json::from_str(&listed((true, Some(5)))).unwrap();
    assert_eq!(
        (page["runs"].as_array().unwrap().len(), &page["behind"]),
        (5, &json!(130))
    );
    let once = |runs: &Value| -> usize {
        let ids = runs.as_array().unwrap().iter().map(|run| &run["run_id"]);
        ids.map(Value::to_string).collect::<HashSet<_>>().len()
    };
    let every: Value = serde_json::from_str(&listed((true, None))).unwrap();
    assert_eq!((&every["count"], every.get("behind")), (&json!(230), None));
    assert_eq!(once(&every["runs"]), 230);
    let line = "130 more runs behind their summary, unread until a writer catches them up\n";
    assert!(listed((false, Some(5))).contains(line));
    let served =
        |path: &str| -> Value { serde_json::from_str(&crate::ui::handle(&db, path).body).unwrap() };
    let page = served("/api/runs?limit=5");
    assert_eq!(
        (page["runs"].as_array().unwrap().len(), &page["behind"]),
        (5, &json!(130))
    );
    assert_eq!(once(&served("/api/runs")), 230);
}

/// Ruling 6: every run, of a hearth with more runs behind their summary
/// than a refresh reads, lists as the whole read lists it, in its JSON
/// and its table alike: the runs past those a refresh reads each in the
/// section its fold places it in and at its place there, among the rows
/// that stand at their heads.
#[test]
fn every_run_lists_the_runs_behind_past_a_refresh_where_the_whole_read_does() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    behind_in_every_section(&db);
    let store = Store::open_read_only(&db).unwrap();
    assert_eq!(
        Pages::listing(&store, Journal::at(&db)).unwrap().unread(),
        130
    );
    let whole = whole_rows(&db);
    for (section, ids) in sectioned(&whole, NOW) {
        let past = ids
            .iter()
            .filter(|id| id.starts_with("lag-") && id.as_str() < "lag-130");
        let held = ids.iter().filter(|id| !id.starts_with("lag-"));
        assert!(past.count() > 0 && held.count() > 0, "{section:?}: {ids:?}");
    }
    let view = brokkr_view::RunsView::of(arranged(&whole));
    // A batch smaller than any section's runs behind, so each section is
    // keyed by several passes that meet at their places.
    for keys in [3, TAIL_KEYS] {
        lists_every_run_as(&db, keys, &view);
    }
}

/// What a listing writes, with `between` run before its first write: a
/// writer that moves the journal while the listing is printed.
struct Between<F: FnOnce()> {
    out: Vec<u8>,
    between: Option<F>,
}

impl<F: FnOnce()> std::io::Write for Between<F> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Some(between) = self.between.take() {
            between();
        }
        self.out.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Ruling 6: a listing of every run is one read of the journal, so a
/// current writer that catches every run behind its summary up while
/// the listing prints, after the runs past a refresh are keyed and
/// before any page is read, moves no run: each lists once, where the
/// whole read lists it.
#[test]
fn every_run_lists_once_however_a_writer_catches_the_runs_behind_up_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    behind_in_every_section(&db);
    let view = brokkr_view::RunsView::of(arranged(&whole_rows(&db)));
    let store = Store::open_read_only(&db).unwrap();
    let mut out = Between {
        out: Vec::new(),
        between: Some(|| drop(Store::open(&db).unwrap())),
    };
    let journal = (Journal::at(&db), NOW);
    list_hearth(&store, journal, true, |lister| lister.every_json(&mut out)).unwrap();
    let lane = brokkr_store::test_support::lane;
    let caught = (lane(&store, "lag-000"), lane(&store, "lag-129"));
    assert_eq!(caught, (Some("refused".into()), Some("running".into())));
    assert_eq!(
        String::from_utf8(out.out).unwrap(),
        format!("{}\n", serde_json::to_string_pretty(&view).unwrap())
    );
}

/// Ruling 8: a reader that cannot write reads a journal the summary does
/// not page the kept way, pages it in memory by the same sections, and
/// leaves it unmigrated; once a writer has migrated it, it is paged.
#[test]
fn a_journal_the_summary_does_not_page_is_read_the_kept_way() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    fixture_journal(&db);
    drop(Store::open(&db).unwrap());
    let whole = whole_rows(&db);
    unmigrate(&rusqlite::Connection::open(&db).unwrap()).unwrap();
    let (store, mut pages, read) = refreshed(&db, AHEAD_ROWS);
    assert!(matches!(pages, Pages::Kept { .. }));
    assert_eq!(read, 210);
    let listed = paged(&pages, &store, NOW, 3, true);
    assert_eq!(by_section(&listed), sectioned(&whole, NOW));
    let every = brokkr_view::RunsView::of(arranged(&whole));
    assert_eq!(
        printed(&db, Some(TAIL_KEYS), |lister, out| lister.every_json(out)),
        format!("{}\n", serde_json::to_string_pretty(&every).unwrap())
    );
    let older = brokkr_view::sections(&whole, NOW)[3].1.len();
    assert_eq!(
        (pages.older(&store, NOW).unwrap(), pages.unread()),
        (older, 0)
    );
    let tables: i64 = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'run_summary'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);
    assert!(pages.was_read_whole("conclude-parked-hand-built"));
    assert!(!pages.replaced(Journal::at(&db)));
    drop(Store::open(&db).unwrap());
    pages.refresh(&store, Journal::at(&db), AHEAD_ROWS).unwrap();
    assert!(matches!(pages, Pages::Summary(_)));
    assert_eq!(pages.older(&store, NOW).unwrap(), older);
}

/// The acceptance (ruling 11): `brokkr runs` lists every run of the
/// fixture journals and the generated one a page at a time, as the
/// whole read lists them by section, in its JSON and its table alike;
/// one page states the cursor of the next, until the last.
#[test]
fn brokkr_runs_lists_every_run_a_page_at_a_time_as_the_whole_read_does() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    fixture_journal(&db);
    generated(&db);
    let arranged = arranged(&whole_rows(&db));
    assert!(arranged.len() > PAGE_ROWS);
    let style = STYLE;
    let ids: Vec<String> = arranged.iter().map(|row| row.run_id.clone()).collect();
    lists_every_run_as(&db, TAIL_KEYS, &brokkr_view::RunsView::of(arranged));
    let mut walked = Vec::new();
    let mut after: Option<brokkr_view::Cursor> = None;
    loop {
        let page = (after.as_ref(), 30);
        let one = printed(&db, None, |lister, out| {
            lister.one(out, page, true, Called::default(), &style)
        });
        let page: Value = serde_json::from_str(&one).unwrap();
        let runs = page["runs"].as_array().unwrap();
        walked.extend(
            runs.iter()
                .map(|row| row["run_id"].as_str().unwrap().to_string()),
        );
        match page["next"].as_str() {
            Some(next) => after = Some(next.parse().unwrap()),
            None => break,
        }
    }
    assert_eq!(walked, ids);
    let table = printed(&db, None, |lister, out| {
        lister.one(out, (None, 1), false, Called::default(), &style)
    });
    let first = format!(
        "next page  brokkr runs --limit 1 --after needs-you:{}:park-new\n",
        now() - HOUR
    );
    assert!(table.ends_with(&first), "{table}");
}

/// The next page's command a table of one page of the runs at `db` ends
/// with, `called` as `brokkr runs` was.
fn hint(db: &Path, called: Called<'_>) -> String {
    let table = printed(db, None, |lister, out| {
        lister.one(out, (None, 1), false, called, &STYLE)
    });
    table.lines().last().unwrap().to_string()
}

/// `hint`'s command split as a shell splits it and parsed as `brokkr`
/// parses it.
fn run_as_typed(hint: &str) -> crate::cli_args::RunsArgs {
    use clap::Parser;
    let line = hint.strip_prefix("next page  ").unwrap();
    let [argv] = &crate::cli_reference_tests::words(line).unwrap()[..] else {
        panic!("the next page is one command: {line}");
    };
    let parsed = crate::Cli::try_parse_from(argv).map(|cli| cli.command);
    let crate::Cmd::Runs(args) = parsed.unwrap_or_else(|refused| panic!("{refused}")) else {
        panic!("`brokkr runs` parses as itself");
    };
    args
}

/// #820 F1: a run id a shell would split, unquote or expand is one quoted
/// word in the next page's command, and that command, run, pages on.
#[test]
fn the_next_page_command_quotes_the_cursor_as_one_word_and_pages_on() {
    let (_dir, db) = finished_hours_ago(&["a b'c$d", "e"]);
    let hint = hint(&db, Called::default());
    let quoted = format!(
        r"next page  brokkr runs --limit 1 --after 'recent:{}:a b'\''c$d'",
        now() - HOUR
    );
    assert_eq!(hint, quoted);
    let args = run_as_typed(&hint);
    let after = args.after.as_ref().map(ToString::to_string);
    let read = format!("recent:{}:a b'c$d", now() - HOUR);
    assert_eq!((args.limit, after), (Some(1), Some(read)));
    let asked = (args.after.as_ref(), 1);
    let page = printed(&db, None, |lister, out| {
        lister.one(out, asked, true, Called::default(), &STYLE)
    });
    let page: Value = serde_json::from_str(&page).unwrap();
    let runs = page["runs"].as_array().unwrap().iter();
    let ids: Vec<&str> = runs.map(|run| run["run_id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["e"]);
}

/// #820 F2: the next page's command names the journal flags the listing
/// was called with, `--db` then `--realms`, each path one shell word
/// joined by `=` where it starts with `-`, and reads back as the paths
/// typed; called with neither, it names none.
#[test]
fn the_next_page_command_keeps_the_listings_db_and_realms() {
    let (_dir, db) = finished_hours_ago(&["done", "e"]);
    let other = Path::new("/w/other forge.db");
    let map = Path::new("map/realms.json");
    let tail = format!("--limit 1 --after recent:{}:done", now() - HOUR);
    let cases = [
        (None, None, String::new()),
        (Some(other), None, " --db '/w/other forge.db'".to_string()),
        (None, Some(map), " --realms map/realms.json".to_string()),
        (
            Some(other),
            Some(map),
            " --db '/w/other forge.db' --realms map/realms.json".to_string(),
        ),
        (
            Some(Path::new("-o my.db")),
            Some(Path::new("-map.json")),
            " --db='-o my.db' --realms=-map.json".to_string(),
        ),
    ];
    for (db_flag, realms, flags) in cases {
        let called = Called {
            db: db_flag,
            realms,
        };
        let hint = hint(&db, called);
        assert_eq!(hint, format!("next page  brokkr runs{flags} {tail}"));
        let args = run_as_typed(&hint);
        assert_eq!(
            (args.db.as_deref(), args.realms.as_deref()),
            (db_flag, realms)
        );
    }
}

/// #820 F2: a journal path that is not UTF-8 has no printed word that
/// names it, so the next page is stated without a command, never as one
/// that would read another file.
#[test]
fn a_path_that_is_not_utf8_prints_no_next_page_command() {
    use std::os::unix::ffi::OsStrExt;
    let (_dir, db) = finished_hours_ago(&["done", "e"]);
    let bytes = Path::new(std::ffi::OsStr::from_bytes(b"review-\xff.db"));
    let stated = format!(
        "next page  --limit 1 --after recent:{}:done, with this listing's --db and --realms: \
         a path that is not UTF-8 is not printed as a command",
        now() - HOUR
    );
    let ok = Some(Path::new("ok.db"));
    let cases = [(Some(bytes), None), (None, Some(bytes)), (ok, Some(bytes))];
    for (db_flag, realms) in cases {
        let called = Called {
            db: db_flag,
            realms,
        };
        assert_eq!(hint(&db, called), stated);
    }
}

/// Every run a walk of pages of one lists, from the first page, each read
/// by `read` after the `next` the page before it stated, to the fleet's
/// end within four pages.
fn walked(read: impl Fn(Option<&str>) -> Value) -> Vec<String> {
    let (mut listed, mut next) = (Vec::new(), None::<String>);
    for _ in 0..4 {
        let page = read(next.as_deref());
        let runs = page["runs"].as_array().unwrap();
        listed.extend(
            runs.iter()
                .map(|run| run["run_id"].as_str().unwrap().to_string()),
        );
        next = page["next"].as_str().map(str::to_string);
        if next.is_none() {
            return listed;
        }
    }
    panic!("no end in four pages; listed {listed:?}");
}

/// The page of one `/api/runs` at `db` serves after `next`, which goes
/// percent-encoded as a client encodes a query value.
fn served_after(db: &Path, next: Option<&str>) -> Value {
    let query = next.map_or(String::new(), |next| {
        let encoded = next.replace(':', "%3A").replace('\0', "%00");
        format!("&after={encoded}")
    });
    let response = crate::ui::handle(db, &format!("/api/runs?limit=1{query}"));
    assert_eq!(response.status, "200 OK", "{}", response.body);
    serde_json::from_str(&response.body).unwrap()
}

/// The page of one `brokkr runs --json` prints of `db` after `after`.
fn printed_after(db: &Path, after: Option<&brokkr_view::Cursor>) -> Value {
    let mut out = Vec::new();
    let asked = (true, Some(1), after);
    crate::verbs::readouts::hearth_runs(db, asked, Called::default(), &STYLE, &mut out).unwrap();
    serde_json::from_str(&String::from_utf8(out).unwrap()).unwrap()
}

/// Rulings 2, 5 and 6: a run id holding a NUL lists as itself, never as
/// the run named the text before its NUL, whether its row stands at its
/// head or behind it: refused in the store's words beside that run, at
/// one time and in one section, alike in the console's page, `brokkr
/// runs` and `/api/runs`; and a page of one reaches each once, by the
/// `next` each states, percent-encoded at the route. A NUL travels on no
/// argv, so `--after` is walked by the cursor its text parses to.
#[test]
fn an_id_holding_a_nul_lists_as_itself_in_every_lane() {
    let words = "event 2: operator/accepted without a matching command";
    for behind in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("forge.db");
        let at = stamp(now() - HOUR);
        let parked = if behind { Shape::Held } else { Shape::Parked };
        folded(&db, |conn| {
            plant(conn, "r\0a", Shape::Refused, &at);
            plant(conn, "r", parked, &at);
        });
        if behind {
            let accepted = json!({"command_id": "none", "operator": "op", "reason": "r"});
            appended_at(&db, "r\0a", (EventType::OperatorAccepted, accepted), &at);
            let reason = json!({"reason": "REVIEW-HOLD for (review, hold): held"});
            appended_at(&db, "r", (EventType::RunParked, reason), &at);
        }
        let (store, pages, _) = refreshed(&db, AHEAD_ROWS);
        assert_eq!(pages.was_read_whole("r\0a"), behind);
        let listed = paged(&pages, &store, NOW, 1, false);
        let listed: Vec<(Section, &str, Option<&str>)> = listed
            .iter()
            .map(|(section, row)| (*section, row.run_id.as_str(), row.detail.as_deref()))
            .collect();
        let needs_you = Section::NeedsYou;
        let expected = [(needs_you, "r\0a", Some(words)), (needs_you, "r", None)];
        assert_eq!(listed, expected, "behind: {behind}");
        let whole = whole_rows(&db);
        lists_every_run_as(&db, TAIL_KEYS, &brokkr_view::RunsView::of(arranged(&whole)));
        let mut every = Vec::new();
        let asked = (true, None, None);
        crate::verbs::readouts::hearth_runs(&db, asked, Called::default(), &STYLE, &mut every)
            .unwrap();
        let every: Value = serde_json::from_slice(&every).unwrap();
        let served = crate::ui::handle(&db, "/api/runs").body;
        assert_eq!(
            serde_json::from_str::<Value>(&served).unwrap(),
            every["runs"]
        );
        let ids = ["r\0a", "r"];
        assert_eq!(walked(|next| served_after(&db, next)), ids);
        let cursor = |next: &str| crate::fleet::cursor(next).unwrap();
        let printed = walked(|next| printed_after(&db, next.map(cursor).as_ref()));
        assert_eq!(printed, ids);
    }
}

/// Ruling 6: every `next` a page states reads back at both edges, the
/// route's query and `brokkr runs --after` on its argv, the `next` at the
/// empty run id a writer admits as at an ordinary one, so a walk of
/// pages of one reaches every run.
#[test]
fn every_next_a_page_states_reads_back_at_both_edges() {
    use clap::Parser;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        plant(conn, "", Shape::Parked, &stamp(now() - HOUR));
        plant(conn, "x", Shape::Parked, &stamp(now() - 2 * HOUR));
        plant(conn, "w", Shape::Parked, &stamp(now() - 3 * HOUR));
    });
    let ids = ["", "x", "w"];
    assert_eq!(walked(|next| served_after(&db, next)), ids);
    let argv = |next: Option<&str>| {
        let mut line = vec!["brokkr", "runs", "--json", "--limit", "1"];
        line.extend(next.into_iter().flat_map(|next| ["--after", next]));
        let parsed = crate::Cli::try_parse_from(line).map(|cli| cli.command);
        let crate::Cmd::Runs(args) = parsed.unwrap_or_else(|refused| panic!("{refused}")) else {
            panic!("`brokkr runs` parses as itself");
        };
        printed_after(&db, args.after.as_ref())
    };
    assert_eq!(walked(argv), ids);
}

/// An empty hearth lists no run, in the shape a listing of none always
/// printed.
#[test]
fn brokkr_runs_lists_an_empty_hearth_as_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    drop(Store::open(&db).unwrap());
    let out = printed(&db, Some(TAIL_KEYS), |lister, out| lister.every_json(out));
    let none = serde_json::to_string_pretty(&brokkr_view::RunsView::of(Vec::new())).unwrap();
    assert_eq!(out, format!("{none}\n"));
}

/// A writer that refuses every write, as a closed pipe does, and counts
/// the writes asked of it.
#[derive(Default)]
struct Refusing {
    writes: usize,
}

impl std::io::Write for Refusing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        self.writes += 1;
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `brokkr runs` whose output refuses its first write is refused in the
/// output's words, and writes nothing more; and the table of a last page
/// prints its runs alone, naming no next page.
#[test]
fn a_listing_its_output_refuses_is_refused_and_a_last_page_names_no_next() {
    let (_dir, db) = done_an_hour_ago();
    let store = Store::open_read_only(&db).unwrap();
    let journal = (Journal::at(&db), NOW);
    let mut out = Refusing::default();
    let refused = list_hearth(&store, journal, true, |lister| lister.every_json(&mut out));
    let refused = refused.unwrap_err().to_string();
    assert_eq!((refused.as_str(), out.writes), ("broken pipe", 1));
    let table = printed(&db, None, |lister, out| {
        lister.one(out, (None, 1), false, Called::default(), &STYLE)
    });
    let view = brokkr_view::RunsView::of(whole_rows(&db));
    assert_eq!(table, crate::render::runs(&view, NOW, &STYLE));
}

/// `--after` takes a page's `next` and refuses text that names no place in
/// the view's own words; `--limit` takes a page of at least one run; and
/// neither pages a world of several hearths, whose pages no one cursor
/// names.
#[test]
fn brokkr_runs_pages_one_hearth_by_a_cursor_it_reads_at_the_edge() {
    use clap::Parser;
    let parsed = |line: &[&str]| crate::Cli::try_parse_from(line).map(|cli| cli.command);
    let asked = parsed(&["brokkr", "runs", "--limit", "2", "--after", "older:5:a:b"]);
    let Ok(crate::Cmd::Runs(args)) = asked else {
        panic!("a page of runs parses");
    };
    let after = args.after.map(|after| after.to_string());
    assert_eq!(
        (args.limit, after.as_deref()),
        (Some(2), Some("older:5:a:b"))
    );
    let refused = parsed(&["brokkr", "runs", "--after", "soon"])
        .err()
        .unwrap();
    let words = "cursor 'soon' is not <section>:<order>:<run id>, as a page's `next` states it";
    assert!(refused.to_string().contains(words), "{refused}");
    let none = parsed(&["brokkr", "runs", "--limit", "0"]).err().unwrap();
    assert_eq!(none.kind(), clap::error::ErrorKind::ValueValidation);
    assert!(
        none.to_string()
            .contains("invalid value '0' for '--limit <N>'"),
        "{none}"
    );
    let dir = tempfile::tempdir().unwrap();
    crate::tests::two_hearths(dir.path());
    let args = crate::cli_args::RunsArgs {
        realms: Some(dir.path().join("realms.json")),
        db: None,
        json: true,
        limit: Some(2),
        after: None,
    };
    let refused = crate::verbs::readouts::runs(dir.path(), args).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "--limit and --after page one hearth's runs, and this world has 2 hearths: name one \
         journal with --db"
    );
}

/// The two committed journals that end at an accepted stop with no
/// ending, and the seq each accepted at (#909).
const STOPPING: [(&str, u64); 2] = [
    ("conclude-stopped-mid-effect-hand-built", 11),
    ("tui-graph-the-selection-box-gets-80f98deb", 93),
];

/// A fresh store holding every committed fixture journal.
fn fixture_hearth() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    fixture_journal(&db);
    (dir, db)
}

/// The fleet rows of the hearth at `db`, read whole.
fn listed_rows(db: &Path) -> brokkr_view::RunsView {
    let store = Store::open_read_only(db).unwrap();
    let read = read_hearth(&store);
    let entries: Vec<_> = read.runs.iter().map(ListedRun::entry).collect();
    brokkr_view::run_rows(&entries)
}

/// #909: a run whose accepted stop has no ending reads as stopping on
/// `brokkr runs`'s table and JSON and in the fleet's sections, and never
/// among the running.
#[test]
fn an_unapplied_stop_reads_as_stopping_on_every_readout() {
    let (_dir, db) = fixture_hearth();
    let rows = listed_rows(&db);
    let row = |run_id: &str| rows.runs.iter().find(|row| row.run_id == run_id).unwrap();
    for (run_id, seq) in STOPPING {
        let row = row(run_id);
        assert_eq!(row.verdict.standing, brokkr_view::Standing::Stopping);
        assert_eq!(row.status_word(), "stopping");
        assert_eq!(
            row.verdict.detail.as_deref(),
            Some(brokkr_view::stopping_detail(seq).as_str())
        );
        assert_eq!(brokkr_view::section_of(row, NOW), Section::NeedsYou);
    }
    let running: Vec<&str> = brokkr_view::sections(&rows.runs, NOW)
        .into_iter()
        .find(|(section, _)| *section == Section::Running)
        .unwrap()
        .1
        .iter()
        .map(|row| row.run_id.as_str())
        .collect();
    for (run_id, _) in STOPPING {
        assert!(!running.contains(&run_id), "{run_id} is not running");
    }
    let text = crate::render::runs(&rows, NOW, &STYLE);
    assert!(text.contains("stopping"), "{text}");
    assert!(
        text.contains("stop accepted at seq 93; no ending written."),
        "{text}"
    );
    assert!(
        text.contains("stop accepted at seq 11; no ending written."),
        "{text}"
    );
    let json = serde_json::to_value(&rows).unwrap();
    for (run_id, seq) in STOPPING {
        let listed = json["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["run_id"] == run_id)
            .unwrap();
        assert_eq!(listed["verdict"]["standing"], "stopping");
        assert_eq!(
            listed["verdict"]["detail"],
            brokkr_view::stopping_detail(seq)
        );
    }
}

/// #909: `inspect` prints the stopping status line, the accepted stop's
/// detail, and the command and acceptance in its text trail; once the
/// driver writes `run/stopped`, the same journal reads as today.
#[test]
fn inspect_shows_the_stop_accepted_with_no_ending_and_its_ending_reads_as_before() {
    let (_dir, db) = fixture_hearth();
    let store = Store::open_read_only(&db).unwrap();
    for (run_id, seq) in STOPPING {
        let events = store.load(run_id).unwrap();
        let state = fold(&events).unwrap();
        let view = brokkr_view::run_view(&events, Some(&state));
        let text = crate::render::inspect(&view, None, true, &STYLE);
        assert!(text.contains("stopping · phase"), "{run_id}: {text}");
        assert!(
            text.contains(brokkr_view::stopping_detail(seq).as_str()),
            "{run_id}: {text}"
        );
        assert!(text.contains("operator/commanded"), "{run_id}: {text}");
        assert!(text.contains("operator/accepted"), "{run_id}: {text}");
        assert!(
            text.contains(&format!("(command at seq {})", seq - 1)),
            "{run_id}: {text}"
        );
    }

    // The driver writes the ending: stopped by the operator, as today.
    let mut writable = Store::open(&db).unwrap();
    writable
        .append_next(
            "tui-graph-the-selection-box-gets-80f98deb",
            EventType::RunStopped,
            json!({}),
            None,
            None,
        )
        .unwrap();
    drop(writable);
    let rows = listed_rows(&db);
    let ended = rows
        .runs
        .iter()
        .find(|row| row.run_id == "tui-graph-the-selection-box-gets-80f98deb")
        .unwrap();
    assert_eq!(
        ended.verdict.standing,
        brokkr_view::Standing::OperatorStopped
    );
    assert_eq!(ended.status_word(), "stopped");
    assert_eq!(ended.verdict.detail, None);
    let text = crate::render::runs(&rows, NOW, &STYLE);
    let line = text
        .lines()
        .find(|line| line.starts_with("tui-graph-the-selection-box-gets-80f98deb"))
        .unwrap();
    assert!(line.contains("stopped"), "{line}");
    assert!(!line.contains("stopping"), "{line}");
}
