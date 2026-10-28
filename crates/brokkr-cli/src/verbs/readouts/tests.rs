//! What `inspect --capability-call` writes of a verified response on a
//! terminal and off one (decision 0065 slice two, U8g). The binary's
//! output is never a terminal in the CLI suite, which proves the bytes
//! off one; this proves the terminal's line.

use std::path::{Path, PathBuf};

use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::json;

use super::{run_view_of, shown, CallRefused};
use crate::cli_args::{InspectArgs, SeatsArgs};
use crate::run_pages::{viewed, Shows};
use crate::tests::running_store;
use crate::{render, watch_loop, write_frame};

/// Runs `r1` and `r2` started and in phase `work`, `r2` by an engine that
/// writes seat-record v6 and its seat `e` started and checkpointed once,
/// by an attributed call `c1` that cites no retained response, their
/// projections current; then the event index keying `r1`'s phase entry as
/// a checkpoint, and `r2`'s checkpoint as another member's. A read through
/// the pages, whose trail is the index's, misses `r1`'s phase, which the
/// run read whole shows; `r2`'s view whole, its checkpoints made from the
/// events its opening kept, shows the seat's checkpoint, which the seat's
/// pages, read through the index, miss.
fn keyed_off_its_trail(db: &Path) {
    running_store(db, "r1");
    let mut store = Store::open(db).unwrap();
    let manifest = json!({"files": {}, "engine": "0.12.0"});
    store
        .create_run("r2", "feature", "test", &manifest)
        .unwrap();
    let attempt = json!({"effect_id": "e", "attempt_id": "a"});
    let mut checkpointed = attempt.clone();
    checkpointed["checkpoint"] = json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch",
        "capability": "web-search", "dialect": "claude-native-search", "call_id": "c1",
        "call_state": "observed"});
    let started = json!({"feature": "feature", "manifest": manifest});
    let asked = json!({"effect_id": "e", "seat": "s", "phase": "work"});
    let seat = [
        (EventType::RunStarted, started),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (EventType::EffectRequested, asked),
        (EventType::EffectStarted, attempt),
        (EventType::EffectCheckpointed, checkpointed),
    ];
    for (kind, payload) in seat {
        store.append_next("r2", kind, payload, None, None).unwrap();
    }
    drop(store);
    drop(Store::open(db).unwrap());
    let journal = rusqlite::Connection::open(db).unwrap();
    let keyed =
        "UPDATE event_index SET type = 'effect/checkpointed' WHERE run_id = 'r1' AND seq = 2";
    assert_eq!(journal.execute(keyed, []).unwrap(), 1);
    let elsewhere = "UPDATE event_index SET member = 'elsewhere' WHERE run_id = 'r2' AND seq = 5";
    assert_eq!(journal.execute(elsewhere, []).unwrap(), 1);
}

/// A journal [`keyed_off_its_trail`] in a directory of its own, its path,
/// and a read of it.
fn keyed_journal() -> (tempfile::TempDir, PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    keyed_off_its_trail(&db);
    let store = Store::open_read_only(&db).unwrap();
    (dir, db, store)
}

/// The phases `view` shows, by name, and how many checkpoints each of its
/// participants shows.
fn shown_of(view: &brokkr_view::RunView) -> (Vec<&str>, Vec<usize>) {
    let phases = view.phases.iter().map(|phase| phase.name.as_str());
    let participants = view.participants.iter();
    let checkpoints = participants.map(|participant| participant.checkpoints.len());
    (phases.collect(), checkpoints.collect())
}

/// Each run reader reads through the pages (decision 0089 slice 3), by its
/// own entry point: `inspect` and `seats`' view, whole and framed, `watch`'s
/// frame and `/api/view`'s body are the paged read's, which misses the
/// phase the run read whole shows; and the view whole, `inspect --json`'s,
/// `seats --json`'s and `/api/view`'s, shows the checkpoint its opening
/// kept, which the seat's pages miss.
#[test]
fn every_run_reader_reads_through_the_pages() {
    let (dir, db, store) = keyed_journal();
    let whole = |run| brokkr_view::run_view(&store.load(run).unwrap(), None);
    let read = |run, shows| {
        let db = Some(db.clone());
        run_view_of(dir.path(), None, db, run, shows, &viewed).unwrap()
    };
    assert_eq!(shown_of(&whole("r2")), (vec!["work"], vec![1]));
    for (shows, checkpoints) in [(Shows::Everything, 1), (Shows::Frame, 0)] {
        assert_eq!(shown_of(&read("r1", shows)), (vec![], vec![]), "{shows:?}");
        let seated = read("r2", shows);
        let shown = (vec!["work"], vec![checkpoints]);
        assert_eq!(shown_of(&seated), shown, "{shows:?}");
    }
    let whole = whole("r1");
    assert_eq!(shown_of(&whole), (vec!["work"], vec![]));
    let style = render::Style::plain(80);
    let frame = viewed(&store, "r1", Shows::Frame).unwrap().view;
    let clock = || "2026-01-01T00:00:00Z".to_string();
    let mut painted = Vec::new();
    let frame = render::inspect(&frame, None, false, &style);
    write_frame(&mut painted, &frame, false, &mut clock.clone()).unwrap();
    let mut watched = Vec::new();
    let mut tick = clock;
    watch_loop(
        &db,
        "r1",
        1000,
        false,
        &style,
        &mut watched,
        &mut tick,
        &mut |_| {},
        1,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(watched).unwrap(),
        String::from_utf8(painted).unwrap()
    );
    let paged = viewed(&store, "r1", Shows::Everything).unwrap().view;
    let paged = serde_json::to_string(&paged.with_scopes()).unwrap();
    assert_ne!(paged, serde_json::to_string(&whole.with_scopes()).unwrap());
    let kept = viewed(&store, "r2", Shows::Everything).unwrap().view;
    assert_eq!(shown_of(&kept), (vec!["work"], vec![1]));
    let kept = serde_json::to_string(&kept.with_scopes()).unwrap();
    for (run, body) in [("r1", paged), ("r2", kept)] {
        let served = crate::ui::handle(&db, &format!("/api/view/{run}"));
        assert_eq!((served.status, served.body), ("200 OK", body), "{run}");
    }
}

/// `brokkr inspect --run <run>`, `--json` as `json` asks, over `db`, and
/// `--capability-call` where `call` names one.
fn inspect_args(db: &Path, run: &str, json: bool, call: Option<&str>) -> InspectArgs {
    let (realms, db, capability_call) = (None, Some(db.to_path_buf()), call.map(String::from));
    let (run, phase, seat, repo) = (run.to_string(), None, None, None);
    InspectArgs {
        run,
        realms,
        db,
        json,
        phase,
        seat,
        capability_call,
        repo,
    }
}

/// `brokkr seats --run <run>`, `--json` as `json` asks, over `db`.
fn seats_args(db: &Path, run: &str, json: bool) -> SeatsArgs {
    let (run, realms, db) = (run.to_string(), None, Some(db.to_path_buf()));
    SeatsArgs {
        run,
        realms,
        db,
        json,
    }
}

/// `inspect` and `seats` print the pages (decision 0089 slice 3), each
/// asking for the read it shows: their frames are the frame's read, which
/// misses `r1`'s phase and `r2`'s checkpoint; their `--json` is the view
/// whole, which shows the checkpoint its opening kept; and `inspect
/// --capability-call` reads the view whole, so it finds the call the
/// seat's pages miss and refuses it for the response it cites none of.
/// Their own entry points print the bytes the reads they ask for print.
#[test]
fn inspect_and_seats_print_the_pages_by_their_own_entry_points() {
    let (dir, db, store) = keyed_journal();
    let style = render::Style::detect();
    let read = |run, shows| viewed(&store, run, shows).unwrap().view;
    let today = |run| {
        let events = store.load(run).unwrap();
        brokkr_view::run_view(&events, brokkr_core::fold::fold(&events).as_ref().ok())
    };
    let served = |view: brokkr_view::RunView| {
        let served = serde_json::to_string_pretty(&view.with_scopes()).unwrap();
        format!("{served}\n")
    };
    let asked = std::cell::RefCell::new(Vec::new());
    let look = |store: &Store, run: &str, shows| {
        asked.borrow_mut().push(shows);
        viewed(store, run, shows)
    };
    let printed = |out: Vec<u8>| String::from_utf8(out).unwrap();
    let inspect = |run, json, call| {
        let mut out = Vec::new();
        let args = inspect_args(&db, run, json, call);
        super::inspect_with(dir.path(), args, &mut out, &look).map(|_| printed(out))
    };
    let seats = |run, json| {
        let mut out = Vec::new();
        super::seats_with(dir.path(), seats_args(&db, run, json), &mut out, &look).unwrap();
        printed(out)
    };
    for run in ["r1", "r2"] {
        let frame = read(run, Shows::Frame);
        let inspected = render::inspect(&frame, None, true, &style);
        assert_eq!(inspect(run, false, None).unwrap(), inspected, "{run}");
        assert_eq!(seats(run, false), render::seats(&frame, &style), "{run}");
        let whole = served(read(run, Shows::Everything));
        assert_eq!(inspect(run, true, None).unwrap(), whole, "{run}");
        assert_eq!(seats(run, true), whole, "{run}");
    }
    for json in [false, true] {
        let refused = inspect("r2", json, Some("c1")).unwrap_err();
        assert_eq!(refused.downcast_ref(), Some(&CallRefused::NoRetained));
    }
    let shown = [
        Shows::Frame,
        Shows::Frame,
        Shows::Everything,
        Shows::Everything,
    ];
    let both = [Shows::Everything; 2];
    assert_eq!(asked.take(), [&shown[..], &shown, &both].concat());
    // What the bytes tell apart: `r1`'s pages from its run read whole, and
    // `r2`'s view whole from its frame.
    let painted = |view| render::inspect(&view, None, true, &style);
    assert_ne!(painted(read("r1", Shows::Frame)), painted(today("r1")));
    assert_ne!(served(read("r1", Shows::Everything)), served(today("r1")));
    let framed = served(read("r2", Shows::Frame));
    assert_ne!(served(read("r2", Shows::Everything)), framed);
    for (run, json) in [("r1", false), ("r1", true), ("r2", true)] {
        let (mut inspected, mut seated) = (Vec::new(), Vec::new());
        super::inspect(
            dir.path(),
            inspect_args(&db, run, json, None),
            &mut inspected,
        )
        .unwrap();
        super::seats(dir.path(), seats_args(&db, run, json), &mut seated).unwrap();
        let printed = (printed(inspected), printed(seated));
        let expected = (inspect(run, json, None).unwrap(), seats(run, json));
        assert_eq!(printed, expected, "{run} {json}");
    }
}

/// Off a terminal, a response is written exactly, a byte that is not
/// text and an escape included; on one, its text loses every control and
/// reordering character, C1 and bidirectional marks included, and its
/// line is ended.
#[test]
fn a_response_is_exact_off_a_terminal_and_cannot_steer_one() {
    let bytes = "{\"t\":\"a\u{1b}[2Jb\u{9b}c\u{202e}d\u{2066}e\"}\n".as_bytes();
    let bytes = [bytes, &[0xff]].concat();
    assert_eq!(&*shown(&bytes, false), &bytes[..]);
    let painted = "{\"t\":\"a[2Jbcde\"}\u{fffd}\n";
    assert_eq!(&*shown(&bytes, true), painted.as_bytes());
}
