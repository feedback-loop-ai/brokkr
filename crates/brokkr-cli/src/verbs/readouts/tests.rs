//! What `inspect --capability-call` writes of a verified response on a
//! terminal and off one (decision 0065 slice two, U8g). The binary's
//! output is never a terminal in the CLI suite, which proves the bytes
//! off one; this proves the terminal's line.

use std::path::Path;

use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::json;

use super::{run_view_of, shown};
use crate::run_pages::{viewed, Shows};
use crate::tests::running_store;
use crate::{render, watch_loop, write_frame};

/// Runs `r1` and `r2` started and in phase `work`, `r2`'s seat `e`
/// started and checkpointed once, their projections current; then the
/// event index keying `r1`'s phase entry as a checkpoint, and `r2`'s
/// checkpoint as another member's. A read through the pages, whose trail
/// is the index's, misses `r1`'s phase, which the run read whole shows;
/// `r2`'s view whole, its checkpoints made from the events its opening
/// kept, shows the seat's checkpoint, which the seat's pages, read through
/// the index, miss.
fn keyed_off_its_trail(db: &Path) {
    running_store(db, "r1");
    running_store(db, "r2");
    let mut store = Store::open(db).unwrap();
    let attempt = json!({"effect_id": "e", "attempt_id": "a"});
    let mut checkpointed = attempt.clone();
    checkpointed["checkpoint"] = json!({"step": "s"});
    let asked = json!({"effect_id": "e", "seat": "s", "phase": "work"});
    let seat = [
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
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    keyed_off_its_trail(&db);
    let store = Store::open_read_only(&db).unwrap();
    let whole = |run| brokkr_view::run_view(&store.load(run).unwrap(), None);
    let read = |run, shows| run_view_of(dir.path(), None, Some(db.clone()), run, shows).unwrap();
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
