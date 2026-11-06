//! The run readers read through the pages print today's bytes (#753;
//! decision 0089 slice 3's acceptance): over every fixture journal
//! imported fresh, every committed journal and every hand-built seat
//! journal planted and caught up, each run's view as `inspect --json`,
//! `seats --json` and `/api/view` print it, and each terminal frame
//! `inspect`, `seats` and `watch` paint, whole and through every phase and
//! seat lens, equals today's of the run read whole, byte for byte, and
//! its fold or the fold's refusal is today's; so is each participant the
//! seat readers read, every field but its checkpoints, which none of them
//! prints, or the refusal they read instead. A run refused at its opening
//! is refused in the store's words; a stale run, an unindexed journal and
//! a journal without the projections take the kept read.

mod before_request;
mod console_end;
mod display_clamp;
mod far_work;
mod fetch_edges;
mod held_view;
mod paged_reads;

use std::path::Path;

use brokkr_core::fold::fold;
use brokkr_store::test_support::{
    import_fixtures, plant_older_writer, plant_summary_writer, statements_begun, statements_traced,
    unindex, unmigrate,
};
use brokkr_store::SeatRead;
use brokkr_view::{lens_for, Scope, Subject};
use serde_json::{json, Value};

use super::page::PAGE;
use super::*;
use crate::render::{self, Style};
use crate::tests::envelope_builder;
use crate::tui::{
    self, down_key, frame_for_test, live_frame_for_test, page_key, participant_level,
    run_level_screens, screens, top_key, Tui, Views, ENDS,
};
use crate::tui_read::tui_views;
use planted::{chained, committed_at, planted};

#[path = "../../tests/heap/grown.rs"]
mod grown;
#[path = "../../../../tests/support/seat_rows.rs"]
mod journals;
#[path = "../../../../tests/support/planted.rs"]
mod planted;
#[path = "../../../../tests/support/seat_steps.rs"]
mod seat_steps;

/// The fixture whose chain is broken, refused at its opening in the
/// store's words.
const BROKEN: &str =
    "conclude-broken-chain-hand-built: chain: event 11: event_hash does not match canonical content";

/// What a readout prints of `view` and its fold `folded`: `inspect
/// --json`'s and `seats --json`'s bytes, refused where the run does not
/// fold, and `/api/view`'s, served either way.
fn printed(view: RunView, folded: &Result<RunState, FoldError>) -> [String; 2] {
    let served = view.with_scopes();
    let pretty = match folded {
        Ok(_) => serde_json::to_string_pretty(&served).unwrap(),
        Err(refused) => format!("refused: {refused}"),
    };
    [pretty, serde_json::to_string(&served).unwrap()]
}

/// The terminal frames painted of `view`: `inspect`'s, whole and through
/// each phase's and each seat's lens, `watch`'s and `seats`', each in
/// colour and plain.
fn painted(view: &RunView) -> Vec<String> {
    let phases = view
        .phases
        .iter()
        .map(|phase| Scope::Phase(phase.name.clone()));
    let seats = view.participants.iter();
    let seats = seats.map(|participant| Scope::Seat(participant.label.clone()));
    let scopes: Vec<Option<Scope>> = [None]
        .into_iter()
        .chain(phases.chain(seats).map(Some))
        .collect();
    let mut frames = Vec::new();
    for style in [
        Style {
            color: false,
            width: 80,
        },
        Style {
            color: true,
            width: 132,
        },
    ] {
        for scope in &scopes {
            let lens = lens_for(view, scope.as_ref()).unwrap();
            frames.push(render::inspect(view, lens.as_ref(), true, &style));
        }
        frames.push(render::inspect(view, None, false, &style));
        frames.push(render::seats(view, &style));
    }
    frames
}

/// How a journal's runs read: each readout that printed otherwise than
/// today, as `run: readout`; the runs read the kept way for every row and
/// checkpoint, and for the frame; the runs refused, with the store's
/// words; and how many were read through their pages.
#[derive(Debug, Default, PartialEq)]
struct Read {
    found: Vec<String>,
    kept: Vec<String>,
    unframed: Vec<String>,
    refused: Vec<String>,
    paged: usize,
}

/// Whether `run` reads through its pages at its opening: for every row and
/// checkpoint, and for the frame.
fn through_pages(store: &Store, run: &str) -> (bool, bool) {
    let (opened, journal) = store.open_kept(run).unwrap();
    let everything = composed(store, run, &opened, Some(&journal)).unwrap();
    let frame = composed(store, run, &opened, None).unwrap();
    (everything.is_some(), frame.is_some())
}

/// Each of `participants` as a seat reader reads it, every field but its
/// checkpoints, which none of them prints.
fn seated(participants: &[Participant]) -> Vec<serde_json::Value> {
    let seated = participants.iter().map(|participant| {
        let mut seated = serde_json::to_value(participant).unwrap();
        seated.as_object_mut().unwrap().remove("checkpoints");
        seated
    });
    seated.collect()
}

/// What `run` printed otherwise than today, read from `store` as each
/// readout reads it, `events` the run read whole: the seat readers'
/// participants, or the fold's refusal of the run, among them.
fn found(store: &Store, run: &str, events: &[brokkr_core::EventEnvelope]) -> Vec<String> {
    let folded = fold(events);
    let today = brokkr_view::run_view(events, folded.as_ref().ok());
    let whole = viewed(store, run, Shows::Everything).unwrap();
    let frame = viewed(store, run, Shows::Frame).unwrap();
    let mut found = Vec::new();
    if painted(&frame.view) != painted(&today) {
        found.push(format!("{run}: the frame"));
    }
    let seats = participants(store, run).unwrap();
    let seats = seats.as_deref().map(seated).map_err(ToString::to_string);
    let expected = folded.as_ref().map(|_| seated(&today.participants));
    if seats != expected.map_err(ToString::to_string) {
        found.push(format!("{run}: the participants"));
    }
    let refusal =
        |folded: &Result<RunState, FoldError>| folded.as_ref().err().map(ToString::to_string);
    if [refusal(&whole.folded), refusal(&frame.folded)] != [refusal(&folded), refusal(&folded)] {
        found.push(format!("{run}: the fold"));
    }
    if printed(whole.view, &whole.folded) != printed(today, &folded) {
        found.push(format!("{run}: the view whole"));
    }
    found
}

/// `read` with `run` read from `store` as each readout reads it.
fn read_one(store: &Store, mut read: Read, run: &str) -> Read {
    let events = match store.load(run) {
        Ok(events) => events,
        Err(refused) => {
            for shows in [Shows::Everything, Shows::Frame] {
                let refusal = viewed(store, run, shows).err();
                let refusal = refusal.map(|error| error.to_string());
                assert_eq!(refusal, Some(refused.to_string()), "{run}: {shows:?}");
            }
            let seated = participants(store, run)
                .err()
                .map(|error| error.to_string());
            assert_eq!(seated, Some(refused.to_string()), "{run}: its participants");
            read.refused.push(format!("{run}: {refused}"));
            return read;
        }
    };
    read.found.extend(found(store, run, &events));
    match through_pages(store, run) {
        (true, true) => read.paged += 1,
        (everything, frame) => {
            if !everything {
                read.kept.push(run.to_string());
            }
            if !frame {
                read.unframed.push(run.to_string());
            }
        }
    }
    read
}

/// How each readout reads `runs` from the journal at `path`, opened
/// read-only.
fn read(path: &Path, runs: &[String]) -> Read {
    let store = Store::open_read_only(path).unwrap();
    let each = |read, run: &String| read_one(&store, read, run);
    runs.iter().fold(Read::default(), each)
}

/// Every fixture imported into a fresh journal at `path` and its runs
/// folded by a writable open: their run ids.
fn imported(path: &Path) -> Vec<String> {
    let (runs, _) = import_fixtures(&mut Store::open(path).unwrap());
    drop(Store::open(path).unwrap());
    runs
}

#[test]
fn every_fixture_s_readouts_read_through_the_pages_print_today_s_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let expected = Read {
        refused: vec![BROKEN.to_string()],
        paged: 5,
        ..Read::default()
    };
    assert_eq!(read(&path, &imported(&path)), expected);
}

#[test]
fn every_committed_and_hand_built_journal_s_readouts_print_today_s_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("committed.db");
    let committed = read(&path, &committed_at(&path));
    assert_eq!((committed.found, committed.refused), (vec![], vec![]));
    assert_eq!((committed.paged, committed.kept.len()), (16, 0));
    let hand_built = journals::journals().into_iter();
    let hand_built = hand_built.map(|(name, events)| (name.to_string(), chained(name, events)));
    let path = dir.path().join("hand-built.db");
    let runs = planted(&path, hand_built.collect());
    // No hand-built seat journal starts its run, so none folds, and each
    // is read whole, as a run whose summary row records the fold's
    // refusal is: `inspect` and `seats` refuse it in the fold's words.
    let expected = Read {
        kept: runs.clone(),
        unframed: runs.clone(),
        ..Read::default()
    };
    assert_eq!(read(&path, &runs), expected);
    let started = journals::journals().into_iter().map(|(name, events)| {
        let run = format!("{name} started");
        let events = chained(&run, started(events));
        (run, events)
    });
    let path = dir.path().join("started.db");
    let started = read(&path, &planted(&path, started.collect()));
    assert_eq!((started.found, started.refused), (vec![], vec![]));
    // Begun by a start, the retried seat's two journals fold and are read
    // through their pages; the rest still do not fold.
    let kept = (started.kept.len(), started.unframed.len());
    assert_eq!((started.paged, kept), (2, (21, 21)));
}

/// `events` begun by their run's start, as a writer's run begins, each
/// event after it a seq later.
fn started(events: Vec<brokkr_core::EventEnvelope>) -> Vec<brokkr_core::EventEnvelope> {
    let payload = serde_json::json!({"feature": "started", "manifest": {}});
    let start = envelope_builder::EnvelopeBuilder::new(brokkr_core::EventType::RunStarted, payload);
    let start = start.seq(1).at(seat_steps::at(0)).build();
    let after = events.into_iter().map(|event| brokkr_core::EventEnvelope {
        seq: event.seq + 1,
        ..event
    });
    std::iter::once(start).chain(after).collect()
}

/// A run an older writer planted after the journal's last writable open,
/// so its projections stand behind its head; every run of a journal
/// whose index is not read; and every run of a journal without the
/// projections: each takes the kept read and prints today's bytes, and
/// the console reads each whole at its run level and at each
/// participant's, and paints today's frames.
#[test]
fn a_stale_run_an_unindexed_journal_and_one_without_projections_read_the_kept_way() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut runs = imported(&path);
    let store = Store::open(&path).unwrap();
    let first = store.load(&runs[0]).unwrap();
    plant_older_writer(&store, "stale", &chained("stale", first));
    drop(store);
    runs.push("stale".to_string());
    let stale = read(&path, &runs);
    let opened: Vec<String> = runs
        .iter()
        .filter(|run| !BROKEN.starts_with(run.as_str()))
        .cloned()
        .collect();
    let expected = Read {
        kept: vec!["stale".to_string()],
        unframed: vec!["stale".to_string()],
        refused: vec![BROKEN.to_string()],
        paged: 5,
        ..Read::default()
    };
    assert_eq!(stale, expected);
    let console = consoled(&path, &["stale".to_string()]);
    let console = (console.found, console.paged, console.whole.len());
    assert_eq!(console, (vec![], 0, 1), "the stale run in the console");
    for unread in [unindex, unmigrate] {
        let journal = dir.path().join("unread.db");
        std::fs::copy(&path, &journal).unwrap();
        unread(&rusqlite::Connection::open(&journal).unwrap()).unwrap();
        let expected = Read {
            kept: opened.clone(),
            unframed: opened.clone(),
            refused: vec![BROKEN.to_string()],
            ..Read::default()
        };
        assert_eq!(read(&journal, &runs), expected);
        let console = consoled(&journal, &runs);
        let console = (
            console.found,
            console.paged,
            console.whole.len(),
            console.refused,
        );
        assert_eq!(console, (vec![], 0, opened.len(), vec![BROKEN.to_string()]));
        std::fs::remove_file(&journal).unwrap();
    }
}

/// A run whose head moved past its opening is no longer read there, so
/// its view whole, composed from the events its opening kept, and its
/// frame are read the kept way.
#[test]
fn a_run_past_its_opening_is_read_the_kept_way() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let runs = imported(&path);
    let store = Store::open_read_only(&path).unwrap();
    let (opened, journal) = store.open_kept(&runs[0]).unwrap();
    let read = |journal| {
        let read = composed(&store, &runs[0], &opened, journal).unwrap();
        read.map(|read| read.view.participants.len())
    };
    let seated = read(Some(&journal)).expect("the run is read at its opening");
    assert!(seated > 0, "the run seats a participant");
    assert_eq!(read(None), Some(seated));
    let head = journal.last().unwrap().clone();
    let moved = brokkr_core::EventEnvelope {
        seq: head.seq + 1,
        event_id: format!("{}-moved", head.event_id),
        previous_hash: head.event_hash.clone(),
        ..head
    };
    plant_summary_writer(&Store::open(&path).unwrap(), &runs[0], &[moved.sealed()]);
    assert_eq!([read(Some(&journal)), read(None)], [None, None]);
}

/// The statements a read at the opening's head makes of the projections:
/// that the event index, the run summary and the seat summary are read
/// and current, the event index with a park's dated facts, the run
/// summary with every aggregate column, the accepted-stop column and the
/// ending's (decision 0064), and the seat summary's `(started_ms)` index,
/// its head with the shape of the rows it vouches for, each table's columns
/// read once, the run's head, its seat rows, its summary row and its
/// trail's seqs, and the ordinal stale index's key, which the repair asks
/// before it may leave an intact schema alone. None of them reads an
/// envelope.
const PROJECTIONS: usize = 31;

/// The view whole reads its run once (ruling 7): on the grown run, 7,510
/// events and 151 journal pages, the read `inspect --json`, `seats --json`
/// and `/api/view` make begins the opening, which verifies and keeps every
/// event, first; then the projections' statements; and one envelope read
/// for each event of the trail, its 17 events but the checkpoints, by its
/// seq, and no other: no journal page and no participant's checkpoint
/// page, so its statements grow with the run's trail and never with its
/// journal.
#[test]
fn the_view_whole_reads_its_opening_its_trail_its_summary_and_its_seat_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("grown.db");
    drop(Store::open(&db).unwrap());
    let run = grown::chained(grown::journal());
    grown::planted(&db, &run);
    drop(Store::open(&db).unwrap());
    let mut store = Store::open_read_only(&db).unwrap();
    let (read, begun) = statements_traced(&mut store, |store| {
        viewed(store, grown::RUN, Shows::Everything).unwrap()
    });
    assert_eq!(read.view.journal.len(), grown::EVENTS);
    let words = |sql: &String| sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let begun: Vec<String> = begun.iter().map(words).collect();
    let keyed = format!(
        "SELECT envelope FROM events WHERE run_id = '{}'",
        grown::RUN
    );
    let at = format!("{keyed} AND seq = ");
    let envelopes = begun.iter().filter(|sql| sql.starts_with(&keyed));
    let envelopes: Vec<Option<u64>> = envelopes
        .map(|sql| sql.strip_prefix(&at).and_then(|seq| seq.parse().ok()))
        .collect();
    let checkpoint = brokkr_core::EventType::EffectCheckpointed;
    let trail = run.iter().filter(|event| event.event_type != checkpoint);
    let trail: Vec<Option<u64>> = trail.map(|event| Some(event.seq)).collect();
    assert_eq!(
        (begun[0].clone(), trail.len()),
        (format!("{keyed} ORDER BY seq"), 17)
    );
    assert_eq!(envelopes[1..], trail);
    assert_eq!(begun.len(), 1 + PROJECTIONS + trail.len());
}

/// The console's frame over the journal at `db`, forced, open on `run`, at
/// the participant `subject` names or away from one, by the console's own
/// read (`tui_read`): its fleet's rows and the run as the console read it.
fn console_frame(db: &Path, run: &str, subject: Option<Subject>) -> Views {
    console_rows(db, run, subject, false, 0)
}

/// [`console_frame`] as the shell's `run_level` asks it, showing `rows`
/// checkpoint rows: the run level's frame and no checkpoint when true,
/// the fleet detail's or a participant's page when false.
fn console_at(db: &Path, run: &str, subject: Option<Subject>, run_level: bool) -> Views {
    console_rows(db, run, subject, run_level, 0)
}

/// [`console_at`] showing `rows` checkpoint rows, the shell's `live_rows`.
fn console_rows(
    db: &Path,
    run: &str,
    subject: Option<Subject>,
    run_level: bool,
    rows: usize,
) -> Views {
    console_rows_at(db, run, subject, run_level, rows, 0)
}

/// [`console_rows`] at the screen's `offset`, so a participant pane's
/// window starts where its real scroll asks (F2/F3).
fn console_rows_at(
    db: &Path,
    run: &str,
    subject: Option<Subject>,
    run_level: bool,
    rows: usize,
    offset: usize,
) -> Views {
    let ask = tui::Ask {
        run: Some(run),
        subject,
        run_level,
        force: true,
        fleet: true,
        tab: 0,
        window: Default::default(),
        live_rows: rows,
        live_offset: offset,
    };
    let clock = || "2026-10-26T00:00:00Z".to_string();
    let views = tui_views(db, true, ask, &mut None, &mut None, clock).unwrap();
    views.expect("a forced frame is drawn")
}

/// The console's RUN level over `run` in the journal at `db`, asked as the
/// shell asks it (`run_level`), by the console's own read (`tui_read`).
fn console_run_level(db: &Path, run: &str) -> Views {
    console_at(db, run, None, true)
}

/// `db` opened read-only, the events of `run`, and the console's first
/// frame of it at `run`'s last stamp: the common start of the live
/// column's tests.
fn console_journal(db: &Path, run: &str) -> (Store, Vec<brokkr_core::EventEnvelope>, Views) {
    let store = Store::open_read_only(db).unwrap();
    let events = store.load(run).unwrap();
    let mut views = console_frame(db, run, None);
    views.now = events.last().unwrap().recorded_at.clone();
    (store, events, views)
}

/// `run`'s console frame with the whole read as its view, and the rank the
/// live column's tests compare against.
fn whole_console(db: &Path, run: &str, events: &[brokkr_core::EventEnvelope]) -> Views {
    let whole = brokkr_view::run_view(events, fold(events).as_ref().ok());
    let mut views = console_frame(db, run, None);
    views.now = events.last().unwrap().recorded_at.clone();
    views.run = Some(whole);
    views.start = 0;
    views.more = false;
    views
}

/// A participant-level console over the first participant of `whole`, its
/// subject and the terminal its pane tests draw at.
fn participant_console(run: &str, whole: &RunView) -> (Tui, Subject) {
    let subject = Subject::of(0, None, run.to_string(), &whole.participants[0]);
    let mut tui = Tui::new(Some(run.to_string()));
    participant_level(&mut tui, "eff");
    (tui.width, tui.height) = (80, 30);
    (tui, subject)
}

/// The whole read's participant-level views: every checkpoint, at the last
/// stamp, the pane painting from its top.
fn participant_whole_views(
    db: &Path,
    run: &str,
    events: &[EventEnvelope],
    subject: Subject,
) -> Views {
    let whole = brokkr_view::run_view(events, fold(events).as_ref().ok());
    let mut views = console_rows(db, run, Some(subject), false, usize::MAX);
    views.now = events.last().unwrap().recorded_at.clone();
    views.run = Some(whole);
    views.start = 0;
    views.more = false;
    views
}

/// The participant frame a paged console paints at `offset` equals the
/// whole read's at the same rank, `paged`'s bytes returned for the control
/// that reads them. `whole_views` is the shared fixture's whole read, so
/// the comparison does not rebuild it.
pub(super) fn participant_frame_whole_read(
    run: &str,
    whole: &RunView,
    whole_views: &Views,
    offset: usize,
    tui: &Tui,
    views: &Views,
) -> String {
    let (mut whole_tui, _) = participant_console(run, whole);
    whole_tui.offset = offset;
    let paged = frame_for_test(tui, views, 80, 30);
    assert_eq!(
        paged,
        frame_for_test(&whole_tui, whole_views, 80, 30),
        "the paged frame is the whole read's"
    );
    paged
}

/// The detail pane's views over a carried `Shown`, at `now`.
fn detail_shown_views(db: &Path, run: &str, shown: &Shown, now: &str) -> Views {
    let mut views = console_frame(db, run, None);
    views.now = now.to_string();
    views.run = Some(shown.view());
    views.start = shown.start(&Console::Detail);
    views.more = shown.more(&Console::Detail);
    views
}

/// A fleet console focused on the live column at `offset`.
fn live_tui(run: &str, offset: usize, width: u16, height: u16) -> Tui {
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(run.to_string());
    tui.pane = 2;
    (tui.width, tui.height) = (width, height);
    tui.live_offset = offset;
    tui
}

/// The console's read of `run` at the live column's ask.
fn live_read(db: &Path, run: &str, tui: &Tui, now: &str) -> Views {
    let mut views = console_rows_at(
        db,
        run,
        None,
        false,
        tui::asked_rows(tui),
        tui::ask_offset(tui),
    );
    views.now = now.to_string();
    views
}

/// Read and paint the live column against the whole read's, `what` naming
/// the step that failed.
fn paint_live(
    db: &Path,
    run: &str,
    tui: &Tui,
    whole: &Views,
    width: u16,
    height: u16,
    what: &str,
) -> Views {
    let views = live_read(db, run, tui, &whole.now);
    let paged = live_frame_for_test(tui, &views, run, width, height);
    let expected = live_frame_for_test(tui, whole, run, width, height);
    assert_eq!(paged, expected, "{what} at row {}", tui.live_offset);
    views
}

/// How the console read a journal's runs: each screen it painted
/// otherwise than today, as `run: asked`; each run whose run level it read
/// whole; the runs refused, in the store's words; and how many run levels
/// it read through the pages. A participant's level reads its run whole.
#[derive(Debug, Default, PartialEq)]
struct Painted {
    found: Vec<String>,
    whole: Vec<String>,
    refused: Vec<String>,
    paged: usize,
}

/// `painted` with `run` read from the journal at `db`, opened as `store`,
/// as the console reads it at its run level and its detail pane and at
/// each participant's level, each screen painted over that read and over
/// the run read whole, at the run's last stamp, so a run at work streams;
/// a run that does not load is drawn by no screen, its row quarantined in
/// the store's words.
fn painted_one(db: &Path, store: &Store, mut painted: Painted, run: &str) -> Painted {
    let events = match store.load(run) {
        Ok(events) => events,
        Err(refused) => {
            let views = console_frame(db, run, None);
            let row = views.runs.runs.into_iter().find(|row| row.run_id == run);
            let detail = row.and_then(|row| row.detail);
            assert_eq!(
                (views.run.is_some(), detail),
                (false, Some(refused.to_string()))
            );
            painted.refused.push(format!("{run}: {refused}"));
            return painted;
        }
    };
    let today = || brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let last = events.last().map(|event| event.recorded_at.clone());
    let mut asked = vec![(Console::Run, None), (Console::Detail, None)];
    for part in &today().participants {
        let subject = Subject::of(0, None, run.to_string(), part);
        asked.push((Console::Participant(part.key.clone()), Some(subject)));
    }
    for (console, subject) in asked {
        let key = subject.as_ref().map(|subject| subject.key.clone());
        let rows = match &console {
            Console::Participant(_) => usize::MAX,
            Console::Run | Console::Detail => 0,
        };
        let run_level = matches!(console, Console::Run);
        let mut views = console_rows(db, run, subject, run_level, rows);
        views.now = last.clone().unwrap_or_default();
        let pairs = match run_level {
            true => run_level_screens(&mut views, today(), run),
            false => screens(&mut views, today(), run, key.as_deref(), &ENDS),
        };
        if pairs.iter().any(|[paged, today]| paged != today) {
            painted.found.push(format!("{run}: {console:?}"));
        }
    }
    let (shown, _) = super::console(store, run, Console::Detail, None, None, 0, 0).unwrap();
    match shown.read {
        Held::Paged(..) => painted.paged += 1,
        Held::Whole(..) => painted.whole.push(run.to_string()),
    }
    painted
}

/// How the console reads `runs` from the journal at `path`, and paints
/// them.
fn consoled(path: &Path, runs: &[String]) -> Painted {
    let store = Store::open_read_only(path).unwrap();
    let each = |painted, run: &String| painted_one(path, &store, painted, run);
    runs.iter().fold(Painted::default(), each)
}

/// The console's run screens read through the pages paint today's frames
/// (decision 0089 slice 3's acceptance): over every fixture journal, every
/// committed journal and every hand-built seat journal, unstarted and
/// started, each run's fleet level beside its row, its dashboard and its
/// live column from their tops and at their ends, its run level and each
/// participant's level, at three terminals, read as the console reads it,
/// equal today's of the run read whole, byte for byte; the broken fixture
/// is refused in the store's words, as today.
#[test]
fn every_console_screen_read_through_the_pages_paints_today_s() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let fixtures = consoled(&path, &imported(&path));
    assert_eq!((fixtures.found, fixtures.whole), (vec![], vec![]));
    assert_eq!(fixtures.refused, [BROKEN]);
    let path = dir.path().join("committed.db");
    let committed = consoled(&path, &committed_at(&path));
    let committed_found = (committed.found, committed.whole, committed.refused);
    assert_eq!(committed_found, (vec![], vec![], vec![]));
    let hand_built = journals::journals().into_iter();
    let hand_built = hand_built.map(|(name, events)| (name.to_string(), chained(name, events)));
    let path = dir.path().join("hand-built.db");
    let hand_built = consoled(&path, &planted(&path, hand_built.collect()));
    assert_eq!((hand_built.found, hand_built.refused), (vec![], vec![]));
    let started = journals::journals().into_iter().map(|(name, events)| {
        let run = format!("{name} started");
        (run.clone(), chained(&run, started(events)))
    });
    let path = dir.path().join("started.db");
    let started = consoled(&path, &planted(&path, started.collect()));
    assert_eq!((started.found, started.refused), (vec![], vec![]));
    // Each run's run level: every fixture and committed run is read
    // through its pages; no hand-built journal folds, so each is read
    // whole, and of the started ones only the retried seat's two journals
    // fold and read through their pages.
    let read = [
        fixtures.paged,
        committed.paged,
        hand_built.paged,
        started.paged,
    ];
    let whole = [hand_built.whole.len(), started.whole.len()];
    assert_eq!((read, whole), ([5, 16, 0, 2], [23, 21]));
}

/// Run `run` at work: started, in phase `work`, its seat `eff` asked for
/// and started, and `checkpoints` turns of it checkpointed, each in its
/// attempt where `attempt` says, else naming none, sealed into one chain,
/// each event stamped in seq order, as a clock that never stepped back
/// leaves it.
fn at_work(run: &str, checkpoints: u64, attempt: bool) -> Vec<brokkr_core::EventEnvelope> {
    use brokkr_core::EventType;
    let mut events = Vec::from(crate::fleet::kept_tests::in_flight(run));
    for turn in 1..=checkpoints {
        let mut payload = json!({"effect_id": "eff", "checkpoint": {"step": "turn", "turn": turn}});
        if attempt {
            payload["attempt_id"] = json!("att");
        }
        events.push((EventType::EffectCheckpointed, payload));
    }
    stamped_chain(run, events)
}

/// `events` stamped two seconds apart in seq order and sealed into `run`
/// one chain, as a clock that never stepped back leaves it.
fn stamped_chain(run: &str, events: Vec<(EventType, Value)>) -> Vec<brokkr_core::EventEnvelope> {
    let events = journals::journal(events);
    let events = events.into_iter().map(|mut event| {
        event.recorded_at = stamped(2 * event.seq);
        event
    });
    chained(run, events.collect())
}

/// The in-flight run's four events, its one seat renamed `effect` and
/// `attempt`: the start's boundary is the caller's to add.
fn in_flight_as(run: &str, effect: &str, attempt: &str) -> Vec<(EventType, Value)> {
    let mut events = Vec::from(crate::fleet::kept_tests::in_flight(run));
    events[2].1["effect_id"] = json!(effect);
    events[3].1["effect_id"] = json!(effect);
    events[3].1["attempt_id"] = json!(attempt);
    events
}

/// The journal at `path` holding each of `runs`, planted and caught up.
fn at_work_in(path: &Path, runs: &[(&str, u64, bool)]) {
    let runs = runs
        .iter()
        .map(|(run, checkpoints, attempt)| (run.to_string(), at_work(run, *checkpoints, *attempt)));
    planted(path, runs.collect());
}

/// How many journal rows `view` holds, and how many checkpoints each of
/// its participants shows.
fn shown_of(view: Option<&RunView>) -> (usize, Vec<usize>) {
    let view = view.expect("the console drew the run");
    let participants = view.participants.iter();
    let checkpoints = participants.map(|participant| participant.checkpoints.len());
    (view.journal.len(), checkpoints.collect())
}

/// The turn texts `view`'s first participant holds, oldest first.
fn turns(view: &RunView) -> Vec<String> {
    let rows = view.participants[0].checkpoints.iter();
    rows.map(|row| row.turn.text.clone()).collect()
}

/// Which of `pairs` a screen painted otherwise than today.
fn differing(pairs: &[[String; 2]]) -> Vec<usize> {
    pairs
        .iter()
        .enumerate()
        .filter(|(_, [paged, today])| paged != today)
        .map(|(screen, _)| screen)
        .collect()
}

/// Each console screen reads what it shows (decision 0089 slice 3), by the
/// console's own read: over a run at work with 120 checkpoints stamped in
/// seq order, its run level reads the frame, whose journal is the trail's
/// four rows where the run read whole holds every event, and no checkpoint
/// of its seat at work; its detail pane reads that frame and the newest
/// page of 50; its participant level reads that frame and the newest page
/// of 50, and older pages as the operator scrolls. A run whose
/// checkpoints name no attempt, which no stream keys, is read whole at
/// the detail pane and the participant level, as today.
#[test]
fn every_console_screen_reads_through_the_pages() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    at_work_in(&db, &[("keyed", 120, true), ("unkeyed", 60, false)]);
    let store = Store::open_read_only(&db).unwrap();
    let whole = |run| brokkr_view::run_view(&store.load(run).unwrap(), None);
    let seat = |run| {
        let view = whole(run);
        Some(Subject::of(0, None, run.to_string(), &view.participants[0]))
    };
    let at = |run, subject| shown_of(console_frame(&db, run, subject).run.as_ref());
    assert_eq!(shown_of(Some(&whole("keyed"))), (124, vec![120]));
    assert_eq!(at("keyed", None), (4, vec![50]));
    assert_eq!(
        shown_of(console_run_level(&db, "keyed").run.as_ref()),
        (4, vec![0]),
        "the run level asks the frame and no checkpoint"
    );
    assert_eq!(at("keyed", seat("keyed")), (4, vec![120]));
    let page = console_frame(&db, "keyed", seat("keyed")).run.unwrap();
    let page: Vec<&str> = page.participants[0]
        .checkpoints
        .iter()
        .map(|row| row.turn.text.as_str())
        .collect();
    assert_eq!(
        (page.first().copied(), page.last().copied()),
        (Some("1"), Some("120")),
        "the participant level opens on its oldest page"
    );
    let participant = Console::Participant("eff".to_string());
    let (shown, _) =
        super::console(&store, "keyed", participant.clone(), None, None, 80, 0).unwrap();
    assert_eq!(shown_of(Some(&shown.view())), (4, vec![120]));
    let (shown, _) =
        super::console(&store, "keyed", participant, None, None, usize::MAX, 0).unwrap();
    assert_eq!(shown_of(Some(&shown.view())), (4, vec![120]));
    assert_eq!(at("unkeyed", None), (64, vec![60]));
    assert_eq!(at("unkeyed", seat("unkeyed")), (64, vec![60]));
    let asked = [
        (Console::Run, (4, vec![0])),
        (Console::Detail, (4, vec![50])),
    ];
    for (asked, shown) in asked {
        let (observed, read) = super::console(&store, "keyed", asked, None, None, 0, 0).unwrap();
        assert_eq!((read, shown_of(Some(&observed.view()))), (124, shown));
    }
    let asked = [Console::Detail, Console::Participant("eff".to_string())];
    for (asked, events) in asked.into_iter().zip([2 * 64, 2 * 64]) {
        let (shown, read) = super::console(&store, "unkeyed", asked, None, None, 0, 0).unwrap();
        assert!(matches!(shown.read, Held::Whole(..)));
        assert_eq!(read, events, "{:?}", shown.asked);
    }
}

/// The stamp `seconds` after the year's first midnight, a real RFC 3339
/// time, so a lexical comparison is the clock's.
fn stamped(seconds: u64) -> String {
    let (hours, minutes) = (seconds / 3600, seconds / 60 % 60);
    format!("2026-01-01T{hours:02}:{minutes:02}:{:02}Z", seconds % 60)
}

/// Run `run` at work, as [`at_work`] makes it with its checkpoints in
/// their attempt, each event stamped two seconds after the one before but
/// its first turn, stamped after its last, as a clock stepped back between
/// them leaves it: the newest by time, the oldest by seq. The step back is
/// outside the newest page, where only the index's stamps can see it.
fn rolled_back(run: &str, checkpoints: u64) -> Vec<brokkr_core::EventEnvelope> {
    let mut events = at_work(run, checkpoints, true);
    for event in &mut events {
        event.recorded_at = stamped(2 * event.seq);
    }
    events[4].recorded_at = stamped(2 * (checkpoints + 4) + 1);
    chained(run, events)
}

/// The live column orders a seat's checkpoints in journal order, by seq,
/// not by the time each was recorded (decision 0089, amended 2026-10-28):
/// over a run of 120 checkpoints whose first was stamped after its last, a
/// clock that stepped back, the newest page by seq is what the console
/// reads and paints, the first row is turn 120 and the last turn 1, and
/// the frame at the top equals the whole read's byte for byte. The one
/// output change is bound exactly here, where time order would put turn 1
/// first.
#[test]
fn the_live_column_reads_a_clock_that_stepped_back_in_journal_order() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "rolled back";
    planted(&db, vec![(run.to_string(), rolled_back(run, 120))]);
    let (store, events, _) = console_journal(&db, run);
    assert!(events[4].recorded_at > events.last().unwrap().recorded_at);
    let today = || brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert!(
        matches!(shown.read, Held::Paged(..)),
        "a clock that stepped back is journal order, never the whole read"
    );
    let streamed = |view: &RunView| -> Vec<String> {
        brokkr_view::working_checkpoints(view, "work")
            .into_iter()
            .map(|(_, row)| row.turn.text.clone())
            .collect()
    };
    let whole = streamed(&today());
    let page = streamed(&shown.view());
    assert_eq!(
        (
            whole.first().map(String::as_str),
            whole.last().map(String::as_str)
        ),
        (Some("120"), Some("1")),
        "the whole read's live column is seq's order"
    );
    assert_eq!(page, whole[..PAGE], "the newest page by seq");
    let mut views = console_rows(&db, run, None, false, usize::MAX);
    views.now = events.last().unwrap().recorded_at.clone();
    let pairs = screens(&mut views, today(), run, None, &[0]);
    assert_eq!(differing(&pairs), Vec::<usize>::new());
}

/// The live column reads a page of the seat at work and no more, and the
/// next page as the operator scrolls (decision 0089 ruling 7 and 9): over
/// a run of 120 checkpoints stamped in seq order, its newest page by seq
/// is its newest by time, 50 rows, which the console reads and derives;
/// the detail's next page holds 100, and all 120 at the stream's end, each
/// painted at the terminals whose pane it fills and at the page boundary
/// equal today's of the run read whole, byte for byte.
#[test]
fn the_live_column_reads_a_page_of_the_seat_at_work() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "in order";
    at_work_in(&db, &[(run, 120, true)]);
    let (store, events, views) = console_journal(&db, run);
    let today = || brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    assert_eq!(shown_of(views.run.as_ref()), (4, vec![50]));
    assert!(
        console_rows(&db, run, None, false, 0).more,
        "a held page with more says so"
    );
    assert!(
        !console_rows(&db, run, None, false, usize::MAX).more,
        "the stream's end says so"
    );
    let page = turns(views.run.as_ref().unwrap());
    assert_eq!(
        (
            page.first().map(String::as_str),
            page.last().map(String::as_str)
        ),
        (Some("71"), Some("120")),
        "the newest page by seq"
    );
    // The switched screens read their own path: the run level the frame
    // and no checkpoint, the detail pane that frame and the newest page.
    let head = store.head_hash(run).unwrap();
    let (run_level, read) = super::console(&store, run, Console::Run, None, None, 0, 0).unwrap();
    assert_eq!(shown_of(Some(&run_level.view())), (4, vec![0]));
    assert_eq!(read, events.len());
    let asked = Console::Detail;
    let (detail, read) =
        super::console(&store, run, asked, Some(run_level), Some(&head), 0, 0).unwrap();
    assert_eq!((read, shown_of(Some(&detail.view()))), (0, (4, vec![50])));
    // The 320x80 pane is taller than one page, so the shell asks for the
    // rows it draws and the read holds the next page.
    let (detail, read) = super::console(&store, run, Console::Detail, None, None, 80, 0).unwrap();
    assert_eq!(
        (read, shown_of(Some(&detail.view()))),
        (events.len(), (4, vec![80]))
    );
    assert_eq!(turns(&detail.view())[0], "41", "the depth the pane asks");
    // At its top every terminal paints today's first screen; at the page
    // boundary and the stream's end, the read holds what it paints.
    let mut paged = console_rows(&db, run, None, false, 80);
    paged.now = events.last().unwrap().recorded_at.clone();
    let pairs = screens(&mut paged, today(), run, None, &[0]);
    assert_eq!(
        differing(&pairs),
        Vec::<usize>::new(),
        "the pane's first screen"
    );
    let mut all = console_rows(&db, run, None, false, usize::MAX);
    all.now = events.last().unwrap().recorded_at.clone();
    assert_eq!(
        shown_of(all.run.as_ref()),
        (4, vec![120]),
        "the stream's end"
    );
    let pairs = screens(&mut all, today(), run, None, &[0, 50, usize::MAX]);
    assert_eq!((pairs.len(), differing(&pairs)), (12, Vec::<usize>::new()));
}

/// The console's read pays for what it shows (decision 0089 ruling 9): a
/// frame whose head stands and that asks the same reads nothing; one that
/// asks the same after one checkpoint landed reads that event, verified
/// onto the head it was read at, and what it shows at the new head, the
/// trail, the seat rows and its seat's checkpoints past those it holds, one
/// page, never the run whole: the same statements, counted, for a run of
/// 200 checkpoints and one of 2,000, none of them its opening, and every
/// checkpoint shown once. A participant's level at a standing head reads
/// the run whole, from its opening, and the run level asked of that read
/// at that head reads nothing. One whose head the journal no longer holds
/// opens the run again.
fn refresh_one(db: &Path, run: &str, events: usize) -> usize {
    let mut store = Store::open_read_only(db).unwrap();
    let (shown, read) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert_eq!(read, events, "{run}: the opening");
    let head = |store: &Store| store.head_hash(run).unwrap();
    let (shown, read) = super::console(
        &store,
        run,
        Console::Detail,
        Some(shown),
        Some(&head(&store)),
        0,
        0,
    )
    .unwrap();
    assert_eq!(read, 0, "{run}: a standing head");
    let landed = at_work(run, 201, true).pop().unwrap();
    let landed = brokkr_core::EventEnvelope {
        seq: events as u64 + 1,
        previous_hash: shown.head.hash.clone(),
        // After every checkpoint the run holds, so the newest page stays
        // in the clock's order.
        recorded_at: "2026-12-31T23:59:59Z".to_string(),
        ..landed
    }
    .sealed();
    let journal = rusqlite::Connection::open(db).unwrap();
    brokkr_store::test_support::plant_envelope(&journal, &landed).unwrap();
    drop(Store::open(db).unwrap());
    let moved = head(&store);
    let ((shown, read), begun) = statements_traced(&mut store, |store| {
        super::console(store, run, Console::Detail, Some(shown), Some(&moved), 0, 0).unwrap()
    });
    assert_eq!(read, 1, "{run}: the event landed");
    let whole = brokkr_view::run_view(&store.load(run).unwrap(), None);
    let shown_turns = turns(&shown.view());
    let whole_turns = turns(&whole);
    let held = shown_turns.len();
    assert_eq!(
        shown_turns,
        whole_turns[whole_turns.len() - held..],
        "{run}: the page is the whole read's tail"
    );
    assert_eq!(shown_of(Some(&shown.view())).1, [50]);
    let opening = format!("SELECT envelope FROM events WHERE run_id = '{run}' ORDER BY seq");
    let words = |sql: &String| sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let opened = begun.iter().map(words).any(|sql| sql == opening);
    assert!(
        !opened,
        "{run}: the refresh opened the run, its chain whole"
    );
    // A row the page already holds is not read again (decision 0089
    // ruling 9): only the event landed was read, and no older checkpoint's
    // body.
    let envelope = format!("SELECT envelope FROM events WHERE run_id = '{run}' AND seq = ");
    let read: Vec<u64> = begun
        .iter()
        .map(words)
        .filter_map(|sql| sql.strip_prefix(&envelope).and_then(|seq| seq.parse().ok()))
        .collect();
    let old: Vec<u64> = store
        .load(run)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| event.seq)
        .filter(|seq| *seq != events as u64 + 1)
        .collect();
    assert!(
        !read.iter().any(|seq| old.contains(seq)),
        "{run}: a held checkpoint's body was read"
    );
    switch_screens(&store, run, events, shown, &moved, &whole);
    begun.len()
}

/// The participant and run screens of a standing read, and an elsewhere
/// head: each switch reads its own page, and the participant starts at its
/// oldest row.
fn switch_screens(
    store: &Store,
    run: &str,
    events: usize,
    shown: Shown,
    moved: &(u64, String),
    whole: &RunView,
) {
    let asked = Console::Participant("eff".to_string());
    let (shown, read) = super::console(store, run, asked, Some(shown), Some(moved), 0, 0).unwrap();
    assert!(read > 0, "{run}: a screen switch reads its own page");
    let switched = turns(&shown.view());
    assert_eq!(switched.first().map(String::as_str), Some("1"));
    let whole_turns = turns(whole);
    assert_eq!(
        switched,
        whole_turns[..switched.len()],
        "{run}: the switched screen paints the participant's oldest page"
    );
    let (shown, read) =
        super::console(store, run, Console::Detail, Some(shown), Some(moved), 0, 0).unwrap();
    assert!(read > 0, "{run}: a screen switch reads its own page");
    assert_eq!(shown_of(Some(&shown.view())).1, [50]);
    let (shown, _) = super::console(store, run, Console::Detail, None, None, 0, 0).unwrap();
    let elsewhere = Shown {
        head: Opened {
            seq: 3,
            hash: "a head the journal never held".to_string(),
        },
        ..shown
    };
    let (_, read) =
        super::console(store, run, Console::Detail, Some(elsewhere), None, 0, 0).unwrap();
    assert_eq!(read, events + 1, "{run}: opened again");
}

#[test]
fn the_console_s_refresh_reads_a_page_never_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    at_work_in(&db, &[("short", 200, true), ("long", 2_000, true)]);
    let refreshed: Vec<usize> = [("short", 204), ("long", 2_004)]
        .into_iter()
        .map(|(run, events)| refresh_one(&db, run, events))
        .collect();
    assert_eq!(refreshed[0], refreshed[1], "a page, whatever the run holds");
}

/// An attempt restarted after the console painted it re-opens the boundary
/// every row of that attempt reads (F2, decision 0089 ruling 11): the
/// refresh reads the page again, and its rows equal a fresh whole read's,
/// row for row, not only their turns.
#[test]
fn the_console_s_refresh_follows_a_restarted_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "restarted";
    at_work_in(&db, &[(run, 120, true)]);
    let store = Store::open_read_only(&db).unwrap();
    let (shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    assert_eq!(shown_of(Some(&shown.view())), (4, vec![50]));
    let head = store.head_hash(run).unwrap();
    let failed = envelope_builder::EnvelopeBuilder::new(
        brokkr_core::EventType::EffectFailed,
        json!({"effect_id": "eff"}),
    )
    .run(run)
    .seq(head.0 + 1)
    .event_id(format!("{run}-failed"))
    .at("2026-01-01T00:09:59Z")
    .previous(head.1.clone())
    .sealed();
    let start = envelope_builder::EnvelopeBuilder::new(
        brokkr_core::EventType::EffectStarted,
        json!({
            "effect_id": "eff",
            "attempt_id": "att",
            "boundary": [{"member": null, "boundary": "open"}],
        }),
    )
    .run(run)
    .seq(head.0 + 2)
    .event_id(format!("{run}-restart"))
    .at("2026-01-01T00:10:00Z")
    .previous(failed.event_hash.clone())
    .sealed();
    let checkpoint = envelope_builder::EnvelopeBuilder::new(
        brokkr_core::EventType::EffectCheckpointed,
        json!({
            "effect_id": "eff",
            "attempt_id": "att",
            "checkpoint": {"step": "turn", "turn": 121},
        }),
    )
    .run(run)
    .seq(head.0 + 3)
    .event_id(format!("{run}-after-restart"))
    .at("2026-01-01T00:10:01Z")
    .previous(start.event_hash.clone())
    .sealed();
    let journal = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_envelope(&journal, &failed).unwrap();
    brokkr_store::test_support::plant_envelope(&journal, &start).unwrap();
    brokkr_store::test_support::plant_envelope(&journal, &checkpoint).unwrap();
    drop(Store::open(&db).unwrap());
    let moved = store.head_hash(run).unwrap();
    let (shown, read) = super::console(
        &store,
        run,
        Console::Detail,
        Some(shown),
        Some(&moved),
        0,
        0,
    )
    .unwrap();
    assert_eq!(read, 3, "the events landed");
    let whole = brokkr_view::run_view(&store.load(run).unwrap(), None);
    let oldest = shown.view().participants[0].checkpoints[0]
        .served
        .boundary
        .text
        .clone();
    assert_eq!(
        oldest, "open",
        "the restart's boundary reaches the oldest held row"
    );
    let rows = |view: &RunView| {
        let part = &view.participants[0];
        let rows = part.checkpoints.iter();
        rows.map(|row| serde_json::to_value(row).unwrap())
            .collect::<Vec<_>>()
    };
    let held = rows(&shown.view());
    let today = rows(&whole);
    assert_eq!(
        held,
        today[today.len() - held.len()..],
        "the restarted boundary, row for row, on the page the console holds"
    );
}

/// Plant a checkpoint on `run` at `head`, stamped `at`, and move `head` to
/// it: one writer's landing.
fn land(db: &Path, run: &str, head: &mut (u64, String), turn: u64, at: &str) {
    let event = landing(run, head, turn, at);
    *head = (event.seq, event.event_hash.clone());
    brokkr_store::test_support::plant_envelope(&rusqlite::Connection::open(db).unwrap(), &event)
        .unwrap();
}

/// A checkpoint landing on `run` after `head`, stamped `at`: the event a
/// writer appends to a run at work.
fn landing(run: &str, head: &(u64, String), turn: u64, at: &str) -> EventEnvelope {
    envelope_builder::EnvelopeBuilder::new(
        brokkr_core::EventType::EffectCheckpointed,
        json!({
            "effect_id": "eff",
            "attempt_id": "att",
            "checkpoint": {"step": "turn", "turn": turn},
        }),
    )
    .run(run)
    .seq(head.0 + 1)
    .event_id(format!("{run}-{turn}"))
    .at(at)
    .previous(head.1.clone())
    .sealed()
}

/// The console holds a window, not the stream, through a deep scroll and a
/// scroll back to the top (decision 0089 ruling 9): a run at work of 1,000
/// checkpoints shown at row 900 holds at most [`WINDOW`] rows of either
/// screen, and returning to the top reads the newest page again rather
/// than accepting the deep window's rows. A participant's first screen is
/// its newest page (ruling 7).
#[test]
fn the_console_holds_a_window_not_the_stream_through_a_deep_scroll() {
    use super::WINDOW;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "windowed";
    at_work_in(&db, &[(run, 1_000, true)]);
    let store = Store::open_read_only(&db).unwrap();
    let head = store.head_hash(run).unwrap();
    let rows = |shown: &Shown| shown.view().participants[0].checkpoints.len();
    let ask = Console::Detail;
    let (top, _) = super::console(&store, run, ask.clone(), None, None, 0, 0).unwrap();
    assert_eq!(rows(&top), 50, "the newest page");
    let (deep, _) = super::console(&store, run, ask.clone(), None, None, 900, 0).unwrap();
    assert!(
        rows(&deep) <= WINDOW,
        "{} rows at row 900, past the {WINDOW}-row window",
        rows(&deep)
    );
    let (back, read) = super::console(&store, run, ask, Some(deep), Some(&head), 0, 0).unwrap();
    assert!(read > 0, "a scroll above the window reads the run again");
    assert_eq!((rows(&back), back.start(&Console::Detail)), (50, 0));
    let participant = Console::Participant("eff".to_string());
    let (first, _) = super::console(&store, run, participant.clone(), None, None, 0, 0).unwrap();
    let page = turns(&first.view());
    assert_eq!(
        (
            page.first().map(String::as_str),
            page.last().map(String::as_str)
        ),
        (Some("1"), Some("200")),
        "the participant level opens on its oldest window"
    );
    let (deep, _) = super::console(&store, run, participant.clone(), None, None, 900, 900).unwrap();
    assert!(rows(&deep) <= WINDOW, "{} rows", rows(&deep));
    let (back, _) =
        super::console(&store, run, participant, Some(deep), Some(&head), 0, 0).unwrap();
    assert_eq!(back.start(&Console::Participant("eff".to_string())), 0);
    assert_eq!(turns(&back.view()).first().map(String::as_str), Some("1"));
}

/// A refresh refills a scrolled window and a batch larger than a window,
/// so its rows equal a fresh read's (R3, R4; decision 0089 ruling 9): over
/// 1,000 checkpoints asked at row 224, the window stands past the stream's
/// top; one checkpoint lands, then a batch larger than [`WINDOW`], and at
/// each moved head the refreshed page is the fresh read's, row for row,
/// full rows at the same start.
#[test]
fn the_console_s_refresh_refills_a_scrolled_window_and_a_large_batch() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "refilled";
    at_work_in(&db, &[(run, 1_000, true)]);
    let store = Store::open_read_only(&db).unwrap();
    let ask = Console::Detail;
    let rows = |shown: &Shown| -> Vec<serde_json::Value> {
        shown.view().participants[0]
            .checkpoints
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect()
    };
    let mut head = store.head_hash(run).unwrap();
    let refill = |shown: Shown, head: &mut (u64, String), turns: &[u64], what: &str| {
        for turn in turns {
            land(&db, run, head, *turn, "2027-01-01T00:00:00Z");
        }
        drop(Store::open(&db).unwrap());
        let moved = store.head_hash(run).unwrap();
        let (refresh, _) =
            super::console(&store, run, ask.clone(), Some(shown), Some(&moved), 224, 0).unwrap();
        let (fresh, _) = super::console(&store, run, ask.clone(), None, None, 224, 0).unwrap();
        assert_eq!(rows(&refresh), rows(&fresh), "{what}");
        assert_eq!(refresh.start(&ask), fresh.start(&ask), "{what}'s start");
    };
    let (shown, _) = super::console(&store, run, ask.clone(), None, None, 224, 0).unwrap();
    assert!(shown.start(&ask) > 0, "the ask holds a window past the top");
    refill(shown, &mut head, &[1_001], "one arrival");
    let (shown, _) = super::console(&store, run, ask.clone(), None, None, 224, 0).unwrap();
    let batch: Vec<u64> = (1_002..=1_260).collect();
    refill(shown, &mut head, &batch, "a batch past WINDOW");
}

/// A refresh that lands checkpoints one at a time keeps a page, not the
/// stream (decision 0089 rulings 7 and 9): over 120 moved heads, the live
/// column at the top holds at most one page and the next cursor always
/// names the older row it cut.
#[test]
fn the_console_s_refresh_holds_a_page_as_the_stream_grows() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "growing";
    at_work_in(&db, &[(run, 50, true)]);
    let store = Store::open_read_only(&db).unwrap();
    let (mut shown, _) = super::console(&store, run, Console::Detail, None, None, 0, 0).unwrap();
    let mut head = store.head_hash(run).unwrap();
    for turn in 51..=170 {
        land(&db, run, &mut head, turn, "2026-06-01T00:00:00Z");
        drop(Store::open(&db).unwrap());
        let moved = store.head_hash(run).unwrap();
        let (next, _) = super::console(
            &store,
            run,
            Console::Detail,
            Some(shown),
            Some(&moved),
            0,
            0,
        )
        .unwrap();
        let page = next.view().participants[0].checkpoints.len();
        assert!(page <= PAGE, "turn {turn}: {page} rows held");
        assert!(
            next.more(&Console::Detail) || page < PAGE,
            "turn {turn}: a full page with no older cursor"
        );
        shown = next;
    }
    let page = shown.view().participants[0].checkpoints.len();
    assert_eq!(page, 50, "the newest page at the moved head");
}

/// A run at work whose 1,000 checkpoints carry a target long enough to
/// wrap the live column's line at the fleet's narrowest column: every live
/// row is more than one drawn line, so a row offset and a line offset are
/// not the same number (R1).
fn wrapped_work(run: &str) -> Vec<EventEnvelope> {
    use brokkr_core::EventType;
    let mut events = Vec::from(crate::fleet::kept_tests::in_flight(run));
    let target = format!("crates/{}", "wide/".repeat(16));
    for turn in 1..=1_000 {
        let payload = json!({
            "effect_id": "eff",
            "attempt_id": "att",
            "checkpoint": {"step": "seat-turn", "turn": turn, "tool": "Read", "target": target},
        });
        events.push((EventType::EffectCheckpointed, payload));
    }
    stamped_chain(run, events)
}

/// The live column's own coordinates (R1): the real `PageDown`/`PageUp`
/// keys, the console's read at every frame, and the live column's draw,
/// which is the console's own, compared with the whole read's at the same
/// absolute row. Over 1,000 one-row-but-wrapped checkpoints the keys pass
/// [`WINDOW`] and come back to the top, and every frame is the whole
/// read's byte for byte: the offset is a stream row, translated once into
/// the window, not the window's local line.
#[test]
fn the_live_column_s_keys_reads_and_draws_keep_one_coordinate_past_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "wrapped live";
    planted(&db, vec![(run.to_string(), wrapped_work(run))]);
    let (_store, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let (width, height) = (375, 20);
    let mut tui = live_tui(run, 0, width, height);
    let mut views = live_read(&db, run, &tui, &whole_views.now);
    let mut beyond = false;
    let mut seen = Vec::new();
    for _ in 0..30 {
        page_key(&mut tui, &views, true);
        beyond |= tui.live_offset > WINDOW;
        views = paint_live(&db, run, &tui, &whole_views, width, height, "down");
        seen.push((tui.live_offset, views.start));
    }
    assert!(beyond, "the keys pass the {WINDOW}-row window: {seen:?}");
    for _ in 0..30 {
        page_key(&mut tui, &views, false);
        views = paint_live(&db, run, &tui, &whole_views, width, height, "back");
    }
    assert_eq!(tui.live_offset, 0, "the keys return to the top");
}

/// The participant level's coordinates (R1, R2): its newest page is the
/// first screen, and every `PageDown` across a finite page boundary paints
/// the whole read's rows at the same offset, byte for byte, until the
/// stream's end; every `PageUp` returns. The pane keeps today's ascending
/// paint, so its bottom is the row at the rank the scroll names and a
/// repeated row cannot appear above the newly fetched ones.
#[test]
fn the_participant_level_s_continuation_is_the_whole_read_s() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "continued";
    at_work_in(&db, &[(run, 120, true)]);
    let (_store, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let (width, height) = (tui.width, tui.height);
    let whole_views = participant_whole_views(&db, run, &events, subject.clone());
    let mut views = console_rows(&db, run, Some(subject.clone()), false, 0);
    let mut offsets = Vec::new();
    for _ in 0..12 {
        page_key(&mut tui, &views, true);
        let rows = tui::asked_rows(&tui);
        views = console_rows_at(&db, run, Some(subject.clone()), false, rows, tui.offset);
        offsets.push(tui.offset);
        let paged = tui::frame_for_test(&tui, &views, width, height);
        let expected = tui::frame_for_test(&tui, &whole_views, width, height);
        assert_eq!(paged, expected, "down at row {}", tui.offset);
    }
    assert!(
        offsets.iter().any(|offset| *offset >= PAGE),
        "the keys cross a page boundary: {offsets:?}"
    );
    for _ in 0..12 {
        page_key(&mut tui, &views, false);
        let rows = tui::asked_rows(&tui);
        views = console_rows_at(&db, run, Some(subject.clone()), false, rows, tui.offset);
        let paged = tui::frame_for_test(&tui, &views, width, height);
        let expected = tui::frame_for_test(&tui, &whole_views, width, height);
        assert_eq!(paged, expected, "back at row {}", tui.offset);
    }
    assert_eq!(tui.offset, 0, "the keys return to the top");
}

/// The participant level keeps today's top-anchored viewport (F2, F3):
/// over an ordinary 120-checkpoint journal its first screen paints the
/// oldest rows, turn 1 first, and the real paging keys reach the newest
/// rows, turns 119 and 120, inside the pane's own two-line border. It
/// differs from today nowhere: the ascending paint is the pre-change
/// renderer's, and only a clock that stepped back moves a row.
#[test]
fn the_participant_level_keeps_todays_top_anchored_viewport() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "top anchored";
    at_work_in(&db, &[(run, 120, true)]);
    let (_, events, _) = console_journal(&db, run);
    let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    let (mut tui, subject) = participant_console(run, &whole);
    let (width, height) = (tui.width, tui.height);
    let rows = tui::asked_rows(&tui);
    let views = console_rows_at(&db, run, Some(subject.clone()), false, rows, tui.offset);
    let first = tui::frame_for_test(&tui, &views, width, height);
    let lines = rows_of_text(&first);
    assert!(
        lines.iter().any(|line| line.contains("│1  turn")),
        "the first screen paints turn 1: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("│120  turn")),
        "the first screen is the top, not the tail"
    );
    let mut views = views;
    for _ in 0..20 {
        page_key(&mut tui, &views, true);
        if tui.offset >= 111 {
            break;
        }
        let rows = tui::asked_rows(&tui);
        views = console_rows_at(&db, run, Some(subject.clone()), false, rows, tui.offset);
    }
    assert!(tui.offset >= 111, "the keys reach the last screen");
    tui.offset = 112;
    let rows = tui::asked_rows(&tui);
    let views = console_rows_at(&db, run, Some(subject), false, rows, tui.offset);
    let last = tui::frame_for_test(&tui, &views, width, height);
    let lines = rows_of_text(&last);
    assert!(
        lines.iter().any(|line| line.contains("│119  turn")),
        "the newest rows are reachable: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("│120  turn")),
        "the last row is not cut by the border: {lines:?}"
    );
}

/// A carried detail window keeps the whole read's absolute rank (F1): the
/// same head asked for 224 rows then 274 holds the window at rank 74, not
/// the local 50 the trimmed page would recompute, and the real live draw
/// at row 250 paints the whole read's row, wrapped or merged.
#[test]
fn a_carried_detail_window_keeps_the_whole_read_s_absolute_rank() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let journals = [
        ("merged".to_string(), panel("merged", 1_000, 1_000)),
        ("wrapped".to_string(), wrapped_work("wrapped")),
    ];
    planted(&db, journals.into());
    let store = Store::open_read_only(&db).unwrap();
    let (width, height) = (375, 24);
    for run in ["merged", "wrapped"] {
        let head = store.head_hash(run).unwrap();
        let (first, _) = super::console(&store, run, Console::Detail, None, None, 224, 0).unwrap();
        assert_eq!(first.start(&Console::Detail), 24, "{run}: the first window");
        let (second, _) = super::console(
            &store,
            run,
            Console::Detail,
            Some(first),
            Some(&head),
            274,
            0,
        )
        .unwrap();
        assert_eq!(
            second.start(&Console::Detail),
            74,
            "{run}: the carried window's absolute rank"
        );
        let (_, events, _) = console_journal(&db, run);
        let whole = whole_console(&db, run, &events);
        let views = detail_shown_views(&db, run, &second, &whole.now);
        let tui = live_tui(run, 250, width, height);
        let paged = live_frame_for_test(&tui, &views, run, width, height);
        let expected = live_frame_for_test(&tui, &whole, run, width, height);
        assert_eq!(paged, expected, "{run}: the live draw at row 250");
    }
}

/// A run with two effects at work: `eff`'s 60 checkpoints, and `other`'s
/// `other` checkpoints, which are never `eff`'s.
fn two_effects(run: &str, other: u64) -> Vec<EventEnvelope> {
    use brokkr_core::EventType;
    let mut steps = in_flight_as(run, "eff", "att");
    steps.extend([
        (
            EventType::EffectRequested,
            json!({"effect_id": "other", "seat": "other", "phase": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "other", "attempt_id": "oth"}),
        ),
    ]);
    // Interleaved, so the other effect's seqs stand inside `eff`'s stream:
    // the page must still keep none of them.
    for turn in 1..=60 {
        steps.push(journals::checkpoint(
            "eff",
            "att",
            json!({"step": "seat-turn", "turn": turn}),
        ));
        if turn <= other {
            steps.push(journals::checkpoint(
                "other",
                "oth",
                json!({"step": "seat-turn", "turn": turn}),
            ));
        }
    }
    for turn in 61..=other {
        steps.push(journals::checkpoint(
            "other",
            "oth",
            json!({"step": "seat-turn", "turn": turn}),
        ));
    }
    chained(run, journals::journal(steps))
}

/// A seat's page reads no other effect's rows, and its cost is the page's:
/// over the same 60 `eff` checkpoints beside 80 and 2,000 `other` ones, the
/// page is `eff`'s newest 50 rows both times and begins the same number of
/// statements (R4, R5; decision 0089 ruling 9). The rollback lookup that
/// read every effect's stamps is gone.
#[test]
fn a_seat_s_page_reads_no_other_effect_and_costs_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let sizes = [("small", 80u64), ("large", 2_000u64)];
    let journals = sizes
        .iter()
        .map(|(run, other)| (run.to_string(), two_effects(run, *other)));
    planted(&db, journals.collect());
    let mut store = Store::open_read_only(&db).unwrap();
    let mut counts = Vec::new();
    for (run, _) in sizes {
        let opened = store.open_run(run).unwrap();
        let eff: Vec<u64> = store
            .load(run)
            .unwrap()
            .iter()
            .filter(|event| {
                event.event_type == EventType::EffectCheckpointed
                    && event.payload["effect_id"] == "eff"
            })
            .map(|event| event.seq)
            .collect();
        let (read, begun) = statements_begun(&mut store, |store| {
            let read = store.seat_read(run, &opened, ("eff", None), None, |_| {
                |event: &EventEnvelope| event.seq
            });
            read.unwrap()
        });
        let SeatRead::Current(seqs, _) = read else {
            panic!("{run}: the page is read at its head");
        };
        let wanted: Vec<u64> = eff.iter().rev().take(PAGE).copied().collect();
        assert_eq!(seqs, wanted, "{run}: the effect's own newest page");
        counts.push(begun);
    }
    assert_eq!(
        counts[0], counts[1],
        "the page's statements, whatever the run holds"
    );
}

/// A run two participants key alike: effect `e`'s member `x` and effect
/// `e:x`. It folds, and the whole view keeps both; the composed view
/// declines the key (R6).
fn collided(run: &str) -> Vec<EventEnvelope> {
    use brokkr_core::EventType;
    let mut steps = in_flight_as(run, "e", "a");
    steps[3].1["boundary"] = json!([{"member": null, "boundary": "harness"},
                                   {"member": "x", "boundary": "open"}]);
    steps.extend([
        journals::checkpoint("e", "a", json!({"step": "s1", "member": "x"})),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e", "attempt_id": "a", "result": {"result": "done", "why": "r6"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "work", "result": "done", "next": "work", "rule_id": "R6"}),
        ),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e:x", "seat": "work", "phase": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e:x", "attempt_id": "b"}),
        ),
        journals::checkpoint("e:x", "b", json!({"step": "s2"})),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e:x", "attempt_id": "b", "result": {"result": "done"}}),
        ),
    ]);
    chained(run, journals::journal(steps))
}

/// The run level takes ruling 8's whole read where composition declines
/// (R6): a folded journal whose seat rows key one participant twice is
/// painted as the whole read paints it, and the RUN screen does not unwind
/// on the frame it could not compose.
#[test]
fn the_run_level_takes_the_whole_read_where_composition_declines() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "collided";
    planted(&db, vec![(run.to_string(), collided(run))]);
    let store = Store::open_read_only(&db).unwrap();
    let (shown, _) = super::console(&store, run, Console::Run, None, None, 0, 0).unwrap();
    assert!(
        matches!(shown.read, Held::Whole(..)),
        "a key two participants make takes the whole read"
    );
    let events = store.load(run).unwrap();
    let today = || brokkr_view::run_view(&events, fold(&events).as_ref().ok());
    assert_eq!(
        serde_json::to_string(&shown.view().with_scopes()).unwrap(),
        serde_json::to_string(&today().with_scopes()).unwrap(),
        "the run level paints the whole read's bytes"
    );
    let mut views = console_run_level(&db, run);
    views.now = events.last().unwrap().recorded_at.clone();
    let pairs = screens(&mut views, today(), run, None, &ENDS);
    assert_eq!(differing(&pairs), Vec::<usize>::new());
}

/// A run whose seat at work is a panel of two members, `a` and `b`, each
/// checkpointing its turns in journal order, interleaved: `a`'s turn then
/// `b`'s, seq after seq, while each still has that turn.
fn panel(run: &str, a_turns: u64, b_turns: u64) -> Vec<EventEnvelope> {
    panel_with(run, a_turns, b_turns, None)
}

/// [`panel`] with every checkpoint's target `target`, where it is given,
/// so a narrow live column wraps the panel's rows.
fn panel_with(run: &str, a_turns: u64, b_turns: u64, target: Option<&str>) -> Vec<EventEnvelope> {
    panel_from(run, a_turns, b_turns, 1..=u64::MAX, target)
}

/// [`panel_with`] with `target` only on the turns in `wrapped`: a proof
/// that reads one turn's rows needs no other row wrapped, and every
/// wrapped row is laid out again on each key.
fn panel_from(
    run: &str,
    a_turns: u64,
    b_turns: u64,
    wrapped: std::ops::RangeInclusive<u64>,
    target: Option<&str>,
) -> Vec<EventEnvelope> {
    let mut steps = in_flight_as(run, "eff", "att");
    steps[3].1["boundary"] = json!([
        {"member": null, "boundary": "harness"},
        {"member": "a", "boundary": "open"},
        {"member": "b", "boundary": "open"},
    ]);
    for turn in 1..=a_turns.max(b_turns) {
        for (member, turns) in [("a", a_turns), ("b", b_turns)] {
            if turn <= turns {
                let mut checkpoint = json!({"member": member, "step": "seat-turn", "turn": turn});
                if let Some(target) = target.filter(|_| wrapped.contains(&turn)) {
                    checkpoint["target"] = json!(target);
                }
                steps.push(journals::checkpoint("eff", "att", checkpoint));
            }
        }
    }
    chained(run, journals::journal(steps))
}

/// The live column's one coordinate over a panel (R1, F1): two members'
/// 1,000 checkpoints interleaved, read at ask 224 and drawn at row 200,
/// paints the whole read's row 200, and the real `PageDown`/`PageUp` keys
/// past [`WINDOW`] and back paint every frame the whole read's, because
/// the window is one contiguous slice of the merged stream and `start` is
/// that slice's rank, never one member's.
#[test]
fn the_live_column_s_merged_window_is_contiguous_across_a_panel() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "panel live";
    planted(&db, vec![(run.to_string(), panel(run, 1_000, 1_000))]);
    let (_store, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let whole = whole_views.run.as_ref().unwrap();
    assert!(
        brokkr_view::working_checkpoints(whole, "work").len() == 2_000,
        "the panel's two members are at work"
    );
    let (width, height) = (375, 24);
    let mut tui = live_tui(run, 200, width, height);
    let mut views = live_read(&db, run, &tui, &whole_views.now);
    assert_eq!(views.start, 24, "the merged window starts at rank 24");
    views = paint_live(&db, run, &tui, &whole_views, width, height, "the window");
    let mut beyond = false;
    for _ in 0..30 {
        page_key(&mut tui, &views, true);
        beyond |= tui.live_offset > WINDOW;
        views = paint_live(&db, run, &tui, &whole_views, width, height, "down");
    }
    assert!(beyond, "the keys pass the {WINDOW}-row window");
    for _ in 0..100 {
        page_key(&mut tui, &views, false);
        views = paint_live(&db, run, &tui, &whole_views, width, height, "back");
    }
    assert_eq!(tui.live_offset, 0, "the keys return to the top");
}

/// `g` reaches the stream's top from a deep window (R1, F2): at row 200 of
/// 1,000 checkpoints, the real `g` leaves offset zero, and the read and
/// the live column then paint the whole read's first screen, turn 1,000
/// first, not the held window's first row.
#[test]
fn the_live_column_s_top_is_the_stream_s_rank_zero() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "top";
    at_work_in(&db, &[(run, 1_000, true)]);
    let (_store, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let (width, height) = (375, 20);
    let mut tui = live_tui(run, 200, width, height);
    let mut views = live_read(&db, run, &tui, &whole_views.now);
    assert!(views.start > 0, "the window starts past the top");
    top_key(&mut tui, &views);
    assert_eq!(tui.live_offset, 0, "g leaves offset zero");
    views = paint_live(&db, run, &tui, &whole_views, width, height, "g");
    let first = views.run.as_ref().and_then(|view| {
        brokkr_view::working_checkpoints(view, "work")
            .first()
            .map(|(_, row)| row.turn.text.clone())
    });
    assert_eq!(first.as_deref(), Some("1000"), "the top is turn 1,000");
}

/// A run at work whose newest checkpoint carries a 3,000-character target,
/// so its live line wraps past the pane.
fn tall_work(run: &str) -> Vec<EventEnvelope> {
    use brokkr_core::EventType;
    let mut steps = Vec::from(crate::fleet::kept_tests::in_flight(run));
    for turn in 1..=2 {
        steps.push((
            EventType::EffectCheckpointed,
            json!({
                "effect_id": "eff",
                "attempt_id": "att",
                "checkpoint": {"step": "seat-turn", "turn": turn, "target": "x".repeat(3_000)},
            }),
        ));
    }
    stamped_chain(run, steps)
}

/// Down moves one drawn line, not one checkpoint (F3): a newest row taller
/// than the pane is entered line by line, so its continuation below the
/// first screen is reachable, and each frame is the whole read's at the
/// same row and line. A row-based Down would jump to the older checkpoint
/// and leave `live_line` zero, which fails here.
#[test]
fn the_live_column_s_down_reaches_a_wrapped_rows_continuation() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "tall";
    planted(&db, vec![(run.to_string(), tall_work(run))]);
    let (_store, events, _) = console_journal(&db, run);
    let now = events.last().unwrap().recorded_at.clone();
    let (width, height) = (375, 12);
    let mut tui = live_tui(run, 0, width, height);
    let mut views;
    let mut first = None;
    for line in 0..20 {
        views = live_read(&db, run, &tui, &now);
        assert_eq!(tui.live_offset, 0, "the newest row is held at line {line}");
        assert_eq!(tui.live_line, line, "Down moved one drawn line");
        let paged = live_frame_for_test(&tui, &views, run, width, height);
        match line {
            0 => first = Some(paged),
            _ => assert_ne!(
                Some(&paged),
                first.as_ref(),
                "the continuation is drawn at line {line}"
            ),
        }
        down_key(&mut tui, &views);
    }
    assert!(
        tui.live_line > usize::from(height),
        "a continuation below the first screen is reachable: {}",
        tui.live_line
    );
}

/// The participant pane keeps today's ascending order (F4): over an
/// ordinary journal of two checkpoints whose clock runs forward, the pane
/// paints turn 1 before turn 2, exactly the committed snapshot, and the
/// read through the page paints the same bytes as the whole read.
#[test]
fn the_participant_level_paints_today_s_ascending_order() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "ascending";
    at_work_in(&db, &[(run, 2, true)]);
    let (events, whole) = {
        let store = Store::open_read_only(&db).unwrap();
        let events = store.load(run).unwrap();
        let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
        (events, whole)
    };
    let (tui, subject) = participant_console(run, &whole);
    let (width, height) = (tui.width, tui.height);
    let mut views = console_rows(&db, run, Some(subject.clone()), false, 0);
    views.now = events.last().unwrap().recorded_at.clone();
    let mut whole_views = console_rows(&db, run, Some(subject), false, usize::MAX);
    whole_views.now = views.now.clone();
    whole_views.run = Some(whole);
    whole_views.start = 0;
    whole_views.more = false;
    let painted = tui::frame_for_test(&tui, &views, width, height);
    let expected = tui::frame_for_test(&tui, &whole_views, width, height);
    assert_eq!(painted, expected, "the page paints the whole read's bytes");
    let read = rows_of_text(&painted);
    let first = read.iter().position(|line| line.contains("│1  turn"));
    let second = read.iter().position(|line| line.contains("│2  turn"));
    assert!(
        first.is_some() && second.is_some() && first < second,
        "turn 1 precedes turn 2: {read:?}"
    );
}

/// The lines of a painted frame, for an order assertion.
fn rows_of_text(frame: &str) -> Vec<String> {
    frame.lines().map(str::to_string).collect()
}

/// A skewed panel's merged window has no gap (R1, F1): member `a` has 500
/// checkpoints and `b` 20, so their per-member windows start at different
/// ranks; at row 200 the live column still paints the whole read's row
/// 200, because every page is cut at one seq boundary rather than each at
/// its own.
#[test]
fn the_live_column_s_merged_window_keeps_a_skewed_panel_contiguous() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = "panel skew";
    planted(&db, vec![(run.to_string(), panel(run, 500, 20))]);
    let (_store, events, _) = console_journal(&db, run);
    let whole_views = whole_console(&db, run, &events);
    let (width, height) = (375, 24);
    let tui = live_tui(run, 200, width, height);
    let views = paint_live(
        &db,
        run,
        &tui,
        &whole_views,
        width,
        height,
        "the skewed window",
    );
    assert!(views.start > 0, "the window starts past the top");
}
