//! The line and branch edges of the paged read (#753 coverage round 753c8):
//! a paged or whole read standing before the screen asked for, the rows a
//! tall pane asks the detail and participant levels for, and the screens it
//! cannot paint. Each case drives `console`, the shell's own read.

use super::*;
use crate::run_pages::page::Checkpoints;

/// A run with `turns` checkpoints at work, its store, head and whole view.
fn at_work_journal(run: &str, turns: u64) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    at_work_in(&db, &[(run, turns, true)]);
    let store = Store::open_read_only(&db).unwrap();
    (dir, db, store)
}

/// A panel's journal, its store, and the keys of its participants, oldest
/// first: the fixture the two-participant detail proofs share.
fn panel_journal(run: &str, turns: u64) -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    planted(&db, vec![(run.to_string(), panel(run, turns, turns))]);
    let store = Store::open_read_only(&db).unwrap();
    (dir, db, store)
}

/// The keys of `run`'s participants, oldest first.
fn participant_keys(store: &Store, run: &str) -> Vec<String> {
    let events = store.load(run).unwrap();
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    whole
        .participants
        .iter()
        .map(|part| part.key.clone())
        .collect()
}

/// Plant `events` after the head at `db`, reopen the journal's writable
/// side, and answer the new head `store` reads.
fn landed_head(
    db: &std::path::Path,
    run: &str,
    store: &Store,
    events: &[EventEnvelope],
) -> (u64, String) {
    let journal = rusqlite::Connection::open(db).unwrap();
    for event in events {
        brokkr_store::test_support::plant_envelope(&journal, event).unwrap();
    }
    drop(journal);
    drop(Store::open(db).unwrap());
    store.head_hash(run).unwrap()
}

/// A detail page carried from `shown` at the same head and asked at its
/// first row: the one read both landing proofs compare.
fn carried_detail(store: &Store, run: &str, shown: Shown, head: &(u64, String)) -> (Shown, usize) {
    super::console(store, run, Console::Detail, Some(shown), Some(head), 0, 0).unwrap()
}

/// A detail page carried from `shown` and asked for a tall pane at the
/// same head: the two-participant refill proofs' read.
fn tall_carried_detail(
    store: &Store,
    run: &str,
    shown: Shown,
    head: &(u64, String),
) -> (Shown, usize) {
    super::console(
        store,
        run,
        Console::Detail,
        Some(shown),
        Some(head),
        10_000,
        0,
    )
    .unwrap()
}

/// A detail page read stands before the RUN level's ask and reads nothing
/// (the paged read paints a run screen; decision 0071 ruling 7), while the
/// RUN level's frame stands before a detail ask only when it holds the
/// detail's checkpoint, which it does not.
#[test]
fn a_paged_read_paints_the_run_level_and_not_the_detail() {
    let run = "paged stands";
    let (_dir, _db, store) = at_work_journal(run, 120);
    let head = store.head_hash(run).unwrap();
    let (fresh, read) = super::console(&store, run, Console::Run, None, None, 0, 0).unwrap();
    assert!(read > 0, "the run level's first read is a page");
    assert!(
        matches!(fresh.read, Held::Paged(..)),
        "the run level's page is the frame alone"
    );
    let (detail, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert!(
        matches!(detail.read, Held::Paged(..)),
        "the detail page is a paged read"
    );
    let (run_level, read) =
        super::console(&store, run, Console::Run, Some(detail), Some(&head), 0, 0).unwrap();
    assert_eq!(read, 0, "a paged read paints the run level");
    assert_eq!(run_level.asked, Console::Run);
    let (detail, read) = super::console(
        &store,
        run,
        Console::Detail,
        Some(run_level),
        Some(&head),
        0,
        0,
    )
    .unwrap();
    assert!(
        read > 0,
        "the run level holds no checkpoint, so the detail reads its own page"
    );
    assert!(matches!(detail.read, Held::Paged(..)));
}

/// A whole read paints every screen: it stands before the detail ask and
/// covers its rows, so the same run is not read again (ruling 7).
#[test]
fn a_whole_read_paints_the_detail_and_covers_its_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "whole stands";
    planted(&db, vec![(run.to_string(), collided(run))]);
    let store = Store::open_read_only(&db).unwrap();
    let head = store.head_hash(run).unwrap();
    let (whole, _) = super::console(&store, run, Console::Run, None, None, 0, 0).unwrap();
    assert!(
        matches!(whole.read, Held::Whole(..)),
        "a run that does not compose is read whole"
    );
    let (detail, read) =
        super::console(&store, run, Console::Detail, Some(whole), Some(&head), 0, 0).unwrap();
    assert_eq!(read, 0, "the whole read covers the detail's rows");
    assert!(matches!(detail.read, Held::Whole(..)));
    assert!(detail.paints(&Console::Detail));
    assert!(detail.paints(&Console::Run));
    assert!(detail.covers(&Console::Detail, usize::MAX, 0));
}

/// A whole read whose frame does not compose is no detail page: the
/// detail ask opens the run and reads it whole, never a paged frame
/// (`paged_at` has no view to page).
#[test]
fn a_frame_that_does_not_compose_is_read_whole_for_a_paged_screen() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "collided detail";
    planted(&db, vec![(run.to_string(), collided(run))]);
    let store = Store::open_read_only(&db).unwrap();
    let (detail, read) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert!(matches!(detail.read, Held::Whole(..)));
    assert!(read > 0, "the run is read whole");
}

/// A tall detail pane asks for every row and a standing page that ends
/// before them is read again; one that reaches the end stands.
#[test]
fn a_tall_detail_pane_reads_the_rows_it_draws() {
    let run = "tall detail";
    let (_dir, _db, store) = at_work_journal(run, 300);
    let head = store.head_hash(run).unwrap();
    let (top, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert_eq!(
        top.view().participants[0].checkpoints.len(),
        PAGE,
        "the top holds one page"
    );
    let (end, _) = super::console(
        &store,
        run,
        Console::Detail,
        Some(top),
        Some(&head),
        usize::MAX,
        0,
    )
    .unwrap();
    assert_eq!(
        end.view().participants[0].checkpoints.len(),
        300,
        "every checkpoint the pane draws"
    );
    let (again, read) = super::console(
        &store,
        run,
        Console::Detail,
        Some(end),
        Some(&head),
        usize::MAX,
        0,
    )
    .unwrap();
    assert_eq!(read, 0, "the end's page covers the tall pane");
    assert_eq!(again.view().participants[0].checkpoints.len(), 300);
}

/// A tall participant pane asks for its whole stream: a standing page
/// that reaches the stream's end covers it, and one that leaves a newer
/// page reads again (the participant's own row check).
#[test]
fn a_tall_participant_pane_reads_the_rows_it_draws() {
    let run = "tall participant";
    let (_dir, _db, store) = at_work_journal(run, 250);
    let head = store.head_hash(run).unwrap();
    let asked = Console::Participant("eff".to_string());
    let (top, _) = super::console(&store, run, asked.clone(), None, None, 0, 0).unwrap();
    assert_eq!(
        top.view().participants[0].checkpoints.len(),
        200,
        "the top holds one window"
    );
    let (whole, _) = super::console(
        &store,
        run,
        asked.clone(),
        Some(top),
        Some(&head),
        usize::MAX,
        0,
    )
    .unwrap();
    assert_eq!(
        whole.view().participants[0].checkpoints.len(),
        250,
        "the stream's end reads every row"
    );
    let (deeper, _) =
        super::console(&store, run, asked.clone(), Some(whole), Some(&head), 400, 0).unwrap();
    assert_eq!(
        deeper.view().participants[0].checkpoints.len(),
        250,
        "a taller ask reads the rows it paints"
    );
    let (end, _) = super::console(
        &store,
        run,
        asked.clone(),
        Some(deeper),
        Some(&head),
        usize::MAX,
        0,
    )
    .unwrap();
    assert_eq!(end.view().participants[0].checkpoints.len(), 250);
    let (again, read) =
        super::console(&store, run, asked, Some(end), Some(&head), usize::MAX, 0).unwrap();
    assert_eq!(read, 0, "the stream's end covers the tall pane");
    assert_eq!(again.view().participants[0].checkpoints.len(), 250);
}

/// A page read for one participant paints no other participant: the
/// standing page is not applied to the second level, which reads its own
/// page (ruling 5).
#[test]
fn a_participant_page_paints_no_other_participant() {
    let run = "two participants";
    let (_dir, _db, store) = panel_journal(run, 500);
    let head = store.head_hash(run).unwrap();
    let keys = participant_keys(&store, run);
    assert!(keys.len() >= 2, "the panel holds two participants");
    let first = Console::Participant(keys[0].clone());
    let second = Console::Participant(keys[1].clone());
    let (page, _) = super::console(&store, run, first, None, None, 0, 0).unwrap();
    assert!(matches!(page.read, Held::Paged(..)));
    let (other, read) = super::console(&store, run, second, Some(page), Some(&head), 0, 0).unwrap();
    assert!(read > 0, "the other participant's page is read");
    assert_eq!(other.asked, Console::Participant(keys[1].clone()));
    assert!(
        other
            .view()
            .participants
            .iter()
            .find(|part| part.key == keys[0])
            .unwrap()
            .checkpoints
            .is_empty(),
        "the first participant's page is not painted"
    );
}

/// The detail's absolute-end request reads its page afresh: `to_end`
/// chooses a read from the newest page, not the carried prefix.
#[test]
fn a_detail_end_request_reads_from_the_newest_page() {
    let run = "detail end";
    let (_dir, _db, store) = at_work_journal(run, 120);
    let head = store.head_hash(run).unwrap();
    let (top, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    let (end, _read) = super::console(
        &store,
        run,
        Console::Detail,
        Some(top),
        Some(&head),
        0,
        usize::MAX,
    )
    .unwrap();
    assert_eq!(end.end(&Console::Detail), Some(120), "the stream's rank");
    assert_eq!(
        end.view().participants[0].checkpoints.len(),
        120,
        "the end's page holds the stream"
    );
    assert_eq!(
        end.view().participants[0]
            .checkpoints
            .last()
            .unwrap()
            .turn
            .text,
        "120",
        "the newest row is held"
    );
}

/// A carried detail page takes the checkpoints that landed since it, so
/// `landed` is not empty when it is refilled (the branch a whole or first
/// page never takes).
#[test]
fn a_carried_detail_page_takes_what_landed() {
    let run = "detail landed";
    let (_dir, db, store) = at_work_journal(run, 120);
    let head = store.head_hash(run).unwrap();
    let (shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    let last = store.load(run).unwrap().len() as u64;
    let landed = at_work(run, 121, true).pop().unwrap();
    let landed = EventEnvelope {
        seq: last + 1,
        previous_hash: head.1.clone(),
        recorded_at: "2026-12-31T23:59:59Z".to_string(),
        ..landed
    }
    .sealed();
    let moved = landed_head(&db, run, &store, &[landed]);
    let (next, read) = carried_detail(&store, run, shown, &moved);
    assert_eq!(read, 1, "the one landed event is read");
    assert_eq!(
        next.view().participants[0]
            .checkpoints
            .last()
            .unwrap()
            .turn
            .text,
        "121",
        "the held rows and the arrival"
    );
}

/// A held page whose frame no longer composes covers no detail row: the
/// guard `covers` keeps against a frame it cannot paint.
#[test]
fn a_frame_that_does_not_compose_covers_no_detail_row() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "collided covers";
    planted(&db, vec![(run.to_string(), collided(run))]);
    let store = Store::open_read_only(&db).unwrap();
    let at = store.open_run(run).unwrap();
    let frame = Frame::read(&store, run, &at).unwrap().unwrap();
    assert!(frame.view().is_none(), "the frame does not compose");
    let shown = Shown {
        head: at,
        asked: Console::Detail,
        read: Held::Paged(frame, Checkpoints::new()),
    };
    assert!(!shown.covers(&Console::Detail, 0, 0));
    assert!(!shown.covers(&Console::Participant("eff".to_string()), 0, 0));
}

/// A held participant page for a participant the frame no longer paints
/// covers nothing, so the screen reads its own page rather than standing
/// on a page it would not draw (ruling 5).
#[test]
fn a_page_for_a_participant_the_frame_lost_covers_nothing() {
    let run = "lost participant";
    let (_dir, _db, store) = at_work_journal(run, 120);
    let at = store.open_run(run).unwrap();
    let frame = Frame::read(&store, run, &at).unwrap().unwrap();
    assert!(frame.view().is_some(), "the frame composes");
    let shown = Shown {
        head: at,
        asked: Console::Participant("gone".to_string()),
        read: Held::Paged(frame, Checkpoints::new()),
    };
    assert!(!shown.covers(&Console::Participant("gone".to_string()), 0, 0));
}

/// A carried detail page with a batch of landed checkpoints larger than a
/// window is refilled from the newest page, not extended row by row (F4).
#[test]
fn a_large_landing_refills_the_detail_window() {
    let run = "detail batch";
    let (_dir, db, store) = at_work_journal(run, 120);
    let (shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    let events = at_work(run, 321, true);
    let landed: Vec<EventEnvelope> = events.into_iter().filter(|event| event.seq > 124).collect();
    assert_eq!(landed.len(), 201, "the batch");
    let moved = landed_head(&db, run, &store, &landed);
    let (next, read) = carried_detail(&store, run, shown, &moved);
    assert_eq!(read, 201, "the batch is read");
    assert_eq!(
        next.view().participants[0]
            .checkpoints
            .last()
            .unwrap()
            .turn
            .text,
        "321",
        "the newest page is held"
    );
}

/// A carried detail page missing one live participant is refilled even
/// with nothing landed: the missing row is the reason (F1).
#[test]
fn a_missing_participant_page_refills_the_detail_window() {
    let run = "detail missing";
    let (_dir, _db, store) = panel_journal(run, 500);
    let head = store.head_hash(run).unwrap();
    let (mut shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    let missing = {
        let Held::Paged(_, held) = &mut shown.read else {
            panic!("the detail is a paged read");
        };
        let key = held.pages.keys().next().unwrap().clone();
        held.pages.remove(&key);
        key
    };
    let (next, _) = tall_carried_detail(&store, run, shown, &head);
    let Held::Paged(_, held) = &next.read else {
        panic!("the refilled detail is a paged read");
    };
    assert!(
        held.pages.contains_key(&missing),
        "the missing page is read"
    );
}

/// A carried detail page holding every live participant and nothing
/// landed keeps its prefix: the read is not refilled on a tall ask.
#[test]
fn a_complete_carried_detail_keeps_its_prefix() {
    let run = "detail complete";
    let (_dir, _db, store) = panel_journal(run, 500);
    let head = store.head_hash(run).unwrap();
    let (shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    let (next, _) = tall_carried_detail(&store, run, shown, &head);
    let Held::Paged(_, held) = &next.read else {
        panic!("the detail is a paged read");
    };
    assert!(held.pages.len() >= 2, "every participant's page is held");
}
