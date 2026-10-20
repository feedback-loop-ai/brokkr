//! The console's production read pays for what changed (#744): the
//! fleet's cadence over a journal that has not moved reads nothing, the
//! selected run included.

use brokkr_core::EventType;
use serde_json::json;

use super::*;
use crate::fleet::kept_tests::{
    lost_in_its_log, swapped_in, unlisted, without_its_first_event, Swap, FIRST_EVENT_GONE,
    UNLISTED,
};
use crate::tests::running_store;
use brokkr_store::test_support::unmigrate;

/// The checkpoints the selected run carries beside its opening events.
const CHECKPOINTS: usize = 200;

/// Append `event_type` with `payload` to `run_id` as the engine would,
/// handing the store the fold it appends onto, so the run's summary is
/// kept at its head (decision 0089 ruling 3).
fn append(db: &Path, run_id: &str, event_type: EventType, payload: serde_json::Value) {
    let mut store = Store::open(db).unwrap();
    let events = store.load(run_id).unwrap();
    store.carry(brokkr_core::summary::Account::whole(run_id, &events));
    store
        .append_next(run_id, event_type, payload, None, None)
        .unwrap();
}

/// A running run with an effect in flight, checkpointing.
fn checkpointing(db: &Path, run_id: &str) {
    running_store(db, run_id);
    append(
        db,
        run_id,
        EventType::EffectRequested,
        json!({"effect_id": "eff", "seat": "work", "phase": "work"}),
    );
    append(
        db,
        run_id,
        EventType::EffectStarted,
        json!({"effect_id": "eff", "attempt_id": "att"}),
    );
    for _ in 0..CHECKPOINTS {
        append(
            db,
            run_id,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff"}),
        );
    }
}

/// One frame of `brokkr tui` over `db`, open on `run`: how many events it
/// read, and the run view it drew, as JSON.
fn frame(db: &Path, kept: &mut Kept, run: &str, force: bool) -> (usize, String) {
    let (read, view, _) = framed(db, kept, run, force);
    (read, view)
}

/// [`frame`], and the words the fleet's row of `run` carries.
fn framed(db: &Path, kept: &mut Kept, run: &str, force: bool) -> (usize, String, Option<String>) {
    let ask = tui::Ask {
        run: Some(run),
        subject: None,
        force,
        fleet: true,
        tab: 0,
        window: Default::default(),
    };
    let views = read_views(db, true, ask, kept, &mut None, || "now".to_string())
        .unwrap()
        .expect("the fleet's cadence draws a frame");
    let view = serde_json::to_string(&views.run).unwrap();
    let row = views.runs.runs.into_iter().find(|row| row.run_id == run);
    (kept.read, view, row.and_then(|row| row.detail))
}

/// The acceptance at the console's own source: with a run selected, the
/// fleet's cadence over a journal that has not moved reads no event, the
/// run's or the fleet's, and draws the run as it drew it. The fleet is a
/// page of the summary (decision 0089), which reads no event: what lands
/// in another run reads none; what lands in the selected run reads it
/// again; and `r` reads it whole. Loading the selected run at every frame
/// (a `run_of` that keeps nothing) reads its 204 events at the idle tick.
#[test]
fn the_fleet_cadence_reads_nothing_of_a_journal_that_has_not_moved() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    checkpointing(&db, "selected");
    running_store(&db, "other");
    let selected = 4 + CHECKPOINTS;
    let mut kept = Kept::default();

    let (read, drawn) = frame(&db, &mut kept, "selected", true);
    assert_eq!(read, selected, "the run, and none of the fleet's events");
    assert_eq!(frame(&db, &mut kept, "selected", false), (0, drawn.clone()));

    append(
        &db,
        "other",
        EventType::PhaseEntered,
        json!({"phase": "on"}),
    );
    assert_eq!(frame(&db, &mut kept, "selected", false), (0, drawn.clone()));

    append(
        &db,
        "selected",
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff"}),
    );
    let (read, moved) = frame(&db, &mut kept, "selected", false);
    assert_eq!(read, selected + 1, "the run again");
    assert_ne!(moved, drawn);
    assert_eq!(frame(&db, &mut kept, "selected", false), (0, moved.clone()));
    assert_eq!(
        frame(&db, &mut kept, "selected", true),
        (selected + 1, moved)
    );
}

/// The open run is read again with the fleet when the journal beneath
/// the console is replaced by one that keeps its head and lost its first
/// event, moved into place or copied over it in place at its length: the
/// fleet's cadence draws no run, as the whole read loads none, where a
/// `moved` that asks only the head draws the run kept from the journal
/// that was moved, and a `run_of` that does not read the run whole when
/// the fleet does draws it from the one copied over.
#[test]
fn the_open_run_is_read_again_from_a_journal_replaced_beneath_it() {
    use std::os::unix::fs::MetadataExt;
    for swap in [Swap::Moved, Swap::InPlace] {
        let dir = tempfile::tempdir().unwrap();
        let (db, mut kept) = opened(dir.path());
        let before = std::fs::metadata(&db).unwrap();

        swapped_in(&without_its_first_event(&db, "selected"), &db, swap);
        let after = std::fs::metadata(&db).unwrap();
        let same = after.ino() == before.ino();
        assert_eq!(same, swap == Swap::InPlace, "{swap:?}");
        assert_eq!(after.len(), before.len(), "{swap:?}: the same length");
        read_again(&db, &mut kept, &format!("{swap:?}"));
    }
}

/// A console open on a checkpointing run, `selected`, in a fresh journal
/// under `dir` as a binary from before the summary left it, so the fleet
/// is the kept read (decision 0089 ruling 8), its first frame drawn:
/// where the journal is, and what the console keeps.
fn opened(dir: &Path) -> (std::path::PathBuf, Kept) {
    let db = dir.join("forge.db");
    checkpointing(&db, "selected");
    unmigrate(&rusqlite::Connection::open(&db).unwrap()).unwrap();
    let mut kept = Kept::default();
    let (_, drawn) = frame(&db, &mut kept, "selected", true);
    assert_ne!(drawn, "null");
    (db, kept)
}

/// The fleet's next frame over `db` draws no run, as the whole read loads
/// none, and shows the run quarantined in the whole read's words. `case`
/// names the case.
fn read_again(db: &Path, kept: &mut Kept, case: &str) {
    let drawn = frame(db, kept, "selected", false);
    assert_eq!(drawn, (0, "null".into()), "{case}");
    let Pages::Kept { rows, .. } = &kept.fleet else {
        panic!("{case}: a journal the summary does not page is read the kept way");
    };
    let words = rows[0].detail.as_deref();
    assert_eq!(words, Some(FIRST_EVENT_GONE), "{case}");
}

/// On a journal the summary pages, a row lost below the selected run's
/// head is not seen by the fleet, whose page checks a row against its
/// run's head alone (decision 0089 ruling 5), but the opened run is: its
/// rows are counted at every frame, it is read again and refused, and the
/// refusal is the fleet's row of it, quarantined in the store's words.
#[test]
fn the_opened_run_is_read_again_when_a_row_below_its_head_is_lost() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    checkpointing(&db, "selected");
    let mut kept = Kept::default();
    let (_, drawn, healthy) = framed(&db, &mut kept, "selected", true);
    assert_ne!(drawn, "null");
    assert_eq!(healthy, None);
    let _held = lost_in_its_log(&db, "selected");
    let refused = Some(FIRST_EVENT_GONE.to_string());
    let frame = framed(&db, &mut kept, "selected", false);
    assert_eq!(frame, (0, "null".into(), refused));
    assert!(matches!(kept.fleet, Pages::Summary(_)));
}

/// The open run is read again with the fleet when its first event is lost
/// into the write-ahead log of a connection held open, the journal file
/// untouched: the fleet's cadence draws no run and shows the whole read's
/// quarantine, where a `refresh` that counts only when the journal file's
/// length or times moved draws the run as it was kept.
#[test]
fn the_open_run_is_read_again_when_a_row_is_lost_in_the_write_ahead_log() {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut kept) = opened(dir.path());
    let _held = lost_in_its_log(&db, "selected");
    read_again(&db, &mut kept, "in the log");
}

/// Decision 0089 ruling 6: under a filter the console reads the fleet
/// forward by keyset and keeps only the rows the filter keeps, the page
/// of them and the one ahead, with the key past the first page of them;
/// the window there holds the rest. Of 300 runs, 100 are kept.
#[test]
fn a_filtered_window_holds_the_page_it_keeps_and_the_one_ahead() {
    use brokkr_store::test_support::{generate, Generated, Standing, Width};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let now = "2026-10-20T00:00:00Z";
    let at = time::OffsetDateTime::parse(now, &time::format_description::well_known::Rfc3339);
    let runs: Vec<Generated> = (0..300)
        .map(|n| Generated {
            run_id: match n % 3 {
                0 => format!("kept-{n:03}"),
                _ => format!("other-{n:03}"),
            },
            standing: Standing::Finished,
            events: 3,
            last_at: (at.unwrap() - time::Duration::minutes(n))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            width: Width::Narrow,
        })
        .collect();
    generate(&Store::open(&db).unwrap(), &runs).unwrap();
    let store = Store::open_read_only(&db).unwrap();
    let window = |after| tui::Window {
        after,
        filter: "kept".to_string(),
    };
    let read = |window: &tui::Window| {
        let listed = fleet_page(&store, &mut Kept::default(), now, (window, None), None);
        let listed = listed.unwrap();
        let ids: Vec<String> = listed.runs.runs.into_iter().map(|row| row.run_id).collect();
        (ids, listed.ahead)
    };
    let (first, ahead) = read(&window(None));
    let kept: Vec<String> = (0..100).map(|n| format!("kept-{:03}", 3 * n)).collect();
    assert_eq!(first, kept);
    let ahead = ahead.unwrap();
    assert_eq!(ahead.run_id, "kept-147");
    let (second, past) = read(&window(Some(ahead)));
    assert_eq!((second, past), (kept[50..].to_vec(), None));
}

/// A journal whose runs the store cannot list is the frame's refusal, in
/// the store's own words: the console draws no fleet it could not read.
#[test]
fn a_journal_whose_runs_cannot_be_listed_refuses_the_frame_in_the_stores_words() {
    let dir = tempfile::tempdir().unwrap();
    let db = unlisted(dir.path());
    let ask = tui::Ask {
        run: None,
        subject: None,
        force: false,
        fleet: true,
        tab: 0,
        window: Default::default(),
    };
    let refused = read_views(&db, true, ask, &mut Kept::default(), &mut None, || {
        "now".to_string()
    })
    .err()
    .expect("a fleet the store cannot list is refused");
    assert_eq!(refused.to_string(), UNLISTED);
}
