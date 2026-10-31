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

use std::path::Path;

use brokkr_core::fold::fold;
use brokkr_store::test_support::{
    import_fixtures, plant_older_writer, plant_summary_writer, statements_traced, unindex,
    unmigrate,
};
use brokkr_view::{lens_for, Scope};

use super::*;
use crate::render::{self, Style};
use crate::tests::envelope_builder;
use planted::{chained, committed_at, planted};

#[path = "../../tests/heap/grown.rs"]
mod grown;
#[path = "../../../../tests/support/seat_rows.rs"]
mod journals;
#[path = "../../../../tests/support/planted.rs"]
mod planted;

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
    let start = start.seq(1).at(journals::at(0)).build();
    let after = events.into_iter().map(|event| brokkr_core::EventEnvelope {
        seq: event.seq + 1,
        ..event
    });
    std::iter::once(start).chain(after).collect()
}

/// A run an older writer planted after the journal's last writable open,
/// so its projections stand behind its head; every run of a journal
/// whose index is not read; and every run of a journal without the
/// projections: each takes the kept read and prints today's bytes.
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
/// summary with every aggregate column and the accepted-stop column, and
/// the seat head with the shape of the rows it vouches for, each table's
/// columns read once, the run's head, its seat rows, its summary row and
/// its trail's seqs. None of them reads an envelope.
const PROJECTIONS: usize = 27;

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
