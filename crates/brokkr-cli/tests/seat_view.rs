//! The seat view read from the store a page at a time is today's (#753;
//! decision 0089 slice 2b's acceptance): every fixture journal imported
//! fresh, and every committed journal and hand-built seat journal planted
//! as an older writer leaves it and caught up by a writable open; each
//! participant of each run, its run opened and its checkpoints read a page
//! of 50 at a time, newest first, at the head the opening verified, holds
//! today's `Participant.checkpoints`: the pages concatenated are its rows
//! read from the newest, row for row and serialized byte for byte, every
//! page but the last full. Joined with the composed run view, the
//! composition is today's whole `run_view`, checkpoints and journal
//! included. A run whose chain is broken is refused at its opening; a run
//! past its opening, and every run of a journal whose index is not read,
//! takes the kept read.

use std::path::Path;

use brokkr_core::fold::fold;
use brokkr_store::test_support::{plant_summary_writer, unindex};
use brokkr_store::Store;
use brokkr_view::{run_view, CheckpointRow, RunView};

use crate::composed_view::{composed_read, imported, next, BROKEN};
use planted::{chained, committed_at, planted};

#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;
#[path = "../../../tests/support/seat_rows.rs"]
mod journals;
#[path = "../../../tests/support/planted.rs"]
mod planted;
#[path = "../../../tests/support/seat_read.rs"]
mod seat_read;

/// The checkpoints one page holds (ruling 7).
const PAGE: usize = 50;

/// How a journal's participants read: each whose pages are not its
/// checkpoints read whole, as `run: key`; each read the kept way; each run
/// refused, with the store's words; and how many were compared.
#[derive(Debug, Default, PartialEq)]
struct Read {
    found: Vec<String>,
    kept: Vec<String>,
    refused: Vec<String>,
    compared: usize,
}

/// `rows`, each serialized.
fn served<'a>(rows: impl Iterator<Item = &'a CheckpointRow>) -> Vec<String> {
    let rows = rows.map(|row| serde_json::to_string(row).unwrap());
    rows.collect()
}

/// Do `pages`, a participant's every page, newest first, hold `whole`, its
/// checkpoints read whole, row for row and byte for byte from the newest,
/// every page but the last full?
fn paged_as(pages: &[Vec<CheckpointRow>], whole: &[CheckpointRow]) -> bool {
    let full = pages.iter().rev().skip(1).all(|page| page.len() == PAGE);
    full && served(pages.iter().flatten()) == served(whole.iter().rev())
}

/// The participant whose run view key is `key` and member `member`: the
/// effect the key names before the member it ends with.
fn seat_of<'a>(key: &'a str, member: Option<&'a str>) -> seat_read::Seat<'a> {
    let effect = member.map_or(Some(key), |member| {
        let effect = key.strip_suffix(member)?;
        effect.strip_suffix(':')
    });
    (effect.expect("a member's key ends with it"), member)
}

/// How `runs` read from the journal at `path`, opened read-only.
fn read(path: &Path, runs: &[String]) -> Read {
    let store = Store::open_read_only(path).unwrap();
    let mut read = Read::default();
    for run in runs {
        let Ok(events) = store.load(run) else {
            let refused = seat_read::pages(&store, run, ("any", None)).err();
            read.refused.push(format!("{run}: {}", refused.unwrap()));
            continue;
        };
        let whole = run_view(&events, fold(&events).ok().as_ref());
        for participant in &whole.participants {
            let seat = seat_of(&participant.key, participant.member.as_deref());
            let named = format!("{run}: {}", participant.key);
            match seat_read::pages(&store, run, seat).unwrap() {
                Some(pages) if paged_as(&pages, &participant.checkpoints) => {}
                Some(_) => read.found.push(named),
                None => {
                    read.kept.push(named);
                    continue;
                }
            }
            read.compared += 1;
        }
    }
    read
}

/// Every fixture imported into a fresh journal at `path` and its runs
/// folded, as the run view's parity imports them: their run ids.
fn fixtures(path: &Path) -> Vec<String> {
    imported(path).1
}

#[test]
fn every_fixture_s_seat_view_read_a_page_at_a_time_is_its_checkpoints() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let expected = Read {
        refused: vec![BROKEN.to_string()],
        compared: 18,
        ..Read::default()
    };
    assert_eq!(read(&path, &fixtures(&path)), expected);
}

#[test]
fn every_committed_and_hand_built_journal_s_seat_view_is_its_checkpoints() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("committed.db");
    let committed = read(&path, &committed_at(&path));
    let expected = Read {
        compared: 72,
        ..Read::default()
    };
    assert_eq!(committed, expected);
    let hand_built = journals::journals().into_iter();
    let hand_built = hand_built.map(|(name, events)| (name.to_string(), chained(name, events)));
    let path = dir.path().join("hand-built.db");
    // A seat whose records name no attempt, which no stream keys; a
    // sequence's seat whose members' checkpoints stand between its own,
    // which only their bodies tell from records the seat's rows read as its
    // own; the seats whose attempt started before their effect was asked,
    // a start their rows do not keep, which a row's boundary reads; and the
    // two runs whose rows keep no live block, so no seat head, whose seat
    // rows read stale, are read whole.
    let kept = [
        "unnamed: s1",
        "sequenced: s1",
        "unasked: u1",
        "again: u1",
        "unasked in flight: u1",
        "again in flight: u1",
        "again in flight: u1:review:a",
        "effectless: other",
        "never asked: e1",
    ];
    let expected = Read {
        kept: kept.map(str::to_string).to_vec(),
        compared: 37,
        ..Read::default()
    };
    assert_eq!(read(&path, &planted(&path, hand_built.collect())), expected);
}

/// `run`'s view composed a journal page at a time and joined with its seat
/// view: the pages' journal rows in order, and each participant's
/// checkpoints its seat view's pages, oldest first as the view shows them.
/// `None` where either read says the run is kept.
fn joined(store: &Store, run: &str) -> Option<RunView> {
    let pages = composed_read::pages(store, run).unwrap()?;
    let mut view = pages.into_iter().reduce(|mut view, page| {
        view.journal.extend(page.journal);
        view
    })?;
    for participant in &mut view.participants {
        let seat = seat_of(&participant.key, participant.member.as_deref());
        let pages = seat_read::pages(store, run, seat).unwrap()?;
        let mut rows: Vec<CheckpointRow> = pages.into_iter().flatten().collect();
        rows.reverse();
        participant.checkpoints = rows;
    }
    Some(view)
}

/// The composed run view joined with the seat view is today's whole
/// `run_view`, serialized byte for byte, for every fixture run that is
/// read at its head.
#[test]
fn the_run_view_composed_and_joined_with_its_seat_views_is_today_s_run_view() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let runs = fixtures(&path);
    let store = Store::open_read_only(&path).unwrap();
    let mut joins = 0;
    for run in runs.iter().filter(|run| !BROKEN.starts_with(run.as_str())) {
        let events = store.load(run).unwrap();
        let whole = run_view(&events, fold(&events).ok().as_ref());
        let joined = joined(&store, run).expect("a fixture run is read at its head");
        let json = |view: &RunView| serde_json::to_string(view).unwrap();
        assert_eq!(json(&joined), json(&whole), "{run}");
        joins += 1;
    }
    assert_eq!(joins, 5);
}

/// A run read at its opening, then appended to and caught up by a current
/// writer's open, reads the kept way at that opening, never the page past
/// it; and every run of a journal whose index is not read is kept.
#[test]
fn a_run_past_its_opening_and_an_unindexed_journal_are_read_the_kept_way() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let runs = fixtures(&path);
    let reader = Store::open_read_only(&path).unwrap();
    let moved = &runs[0];
    let opened = reader.open_run(moved).unwrap();
    let after = next(&reader.load(moved).unwrap());
    let effect = after.payload["effect_id"].as_str().unwrap().to_string();
    plant_summary_writer(&Store::open(&path).unwrap(), moved, &[after]);
    drop(Store::open(&path).unwrap());
    let at = |opened| seat_read::page(&reader, moved, opened, (&effect, None), None).unwrap();
    assert_eq!(at(&opened).map(|(rows, _)| rows.len()), None);
    let reopened = reader.open_run(moved).unwrap();
    let (rows, _) = at(&reopened).expect("the run is read at its new head");
    assert_eq!(rows[0].served.model.text, "lagged");
    drop(reader);
    unindex(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let unindexed = read(&path, &runs);
    assert_eq!(
        (unindexed.compared, unindexed.refused),
        (0, vec![BROKEN.to_string()])
    );
    assert_eq!(unindexed.kept.len(), 18);
}
