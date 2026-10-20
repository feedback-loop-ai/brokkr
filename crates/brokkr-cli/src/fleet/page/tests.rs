//! The fleet a page at a time (decision 0089 rulings 5, 6, 8 and 9): the
//! paged read lists every section as the whole read does, at every bound;
//! a run behind its summary is placed by its fold; a row its run does not
//! stand at is read whole; and a journal the summary does not page is
//! read the kept way.

use std::collections::HashSet;
use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
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
use crate::fleet::{list_hearth, read_hearth, ListedRun, AHEAD_ROWS, PAGE_ROWS};
use crate::tests::envelope_builder::EnvelopeBuilder;

const NOW: &str = "2026-10-20T00:00:00Z";
const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
/// The staleness bound `brokkr_view` holds a running run to.
const STALE: i64 = 3 * HOUR;

fn now() -> i64 {
    parse_millis(NOW).unwrap()
}

fn stamp(ms: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}

/// How a generated run stands once folded.
#[derive(Clone, Copy)]
enum Shape {
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
fn plant(journal: &rusqlite::Connection, run_id: &str, shape: Shape, at: &str) {
    plant_run(journal, run_id, "{}").unwrap();
    for event in chained(run_id, (0, ZERO_HASH.to_string()), steps(shape), at) {
        plant_envelope(journal, &event).unwrap();
    }
}

/// Every fixture journal, imported where the import takes it and planted
/// where it does not, as the store's own fixture test reads them.
fn fixture_journal(db: &Path) {
    import_fixtures(&mut Store::open(db).unwrap());
}

/// The runs that split a paged read from a whole one (ruling 9): a park
/// and a stale run newer than a page and more of refusals, an old
/// refusal, a run stamped `+02:00`, one whose stamp does not read, an
/// acknowledged one active within the day, and runs either side of the
/// day. Each run a clock is set by, and its last activity in milliseconds.
fn generated(db: &Path) -> Vec<(&'static str, i64)> {
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
fn whole_rows(db: &Path) -> Vec<RunRow> {
    let store = Store::open_read_only(db).unwrap();
    let listed = read_hearth(&store).listed().unwrap();
    let entries: Vec<_> = listed.iter().map(ListedRun::entry).collect();
    brokkr_view::run_rows(&entries).runs
}

/// `rows` as the whole read lists them by section at [`NOW`].
fn arranged(rows: &[RunRow]) -> Vec<RunRow> {
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
const STYLE: crate::render::Style = crate::render::Style {
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
fn folded(db: &Path, plant_with: impl FnOnce(&rusqlite::Connection)) {
    drop(Store::open(db).unwrap());
    plant_with(&rusqlite::Connection::open(db).unwrap());
    drop(Store::open(db).unwrap());
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
fn behind_in_every_section(db: &Path) {
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
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        plant(conn, "done", Shape::Finished, &stamp(now() - HOUR));
    });
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
        crate::verbs::readouts::hearth_runs(&db, asked, &style, &mut out).unwrap();
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
        let one = printed(&db, None, |lister, out| lister.one(out, page, true, &style));
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
        lister.one(out, (None, 1), false, &style)
    });
    let first = format!(
        "next page  brokkr runs --limit 1 --after needs-you:{}:park-new\n",
        now() - HOUR
    );
    assert!(table.ends_with(&first), "{table}");
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
