//! The console's production read pays for what changed (#744): the
//! fleet's cadence over a journal that has not moved reads nothing, the
//! selected run included.

use brokkr_core::EventType;
use serde_json::json;

use super::*;
use crate::tests::running_store;

/// The checkpoints the selected run carries beside its opening events.
const CHECKPOINTS: usize = 200;

/// Append `event_type` with `payload` to `run_id` as the engine would.
fn append(db: &Path, run_id: &str, event_type: EventType, payload: serde_json::Value) {
    Store::open(db)
        .unwrap()
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
    let ask = tui::Ask {
        run: Some(run),
        subject: None,
        force,
        fleet: true,
        tab: 0,
    };
    let views = read_views(db, true, ask, kept, &mut None, || "now".to_string())
        .unwrap()
        .expect("the fleet's cadence draws a frame");
    let view = serde_json::to_string(&views.run).unwrap();
    (kept.read, view)
}

/// The acceptance at the console's own source: with a run selected, the
/// fleet's cadence over a journal that has not moved reads no event, the
/// run's or the fleet's, and draws the run as it drew it. What lands in
/// another run is read alone; what lands in the selected run reads it
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
    assert_eq!(
        read,
        selected + 2 + selected,
        "the fleet whole, and the run"
    );
    assert_eq!(frame(&db, &mut kept, "selected", false), (0, drawn.clone()));

    append(
        &db,
        "other",
        EventType::PhaseEntered,
        json!({"phase": "on"}),
    );
    assert_eq!(frame(&db, &mut kept, "selected", false), (1, drawn.clone()));

    append(
        &db,
        "selected",
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff"}),
    );
    let (read, moved) = frame(&db, &mut kept, "selected", false);
    assert_eq!(read, 1 + selected + 1, "what landed, and the run again");
    assert_ne!(moved, drawn);
    assert_eq!(frame(&db, &mut kept, "selected", false), (0, moved.clone()));
    assert_eq!(
        frame(&db, &mut kept, "selected", true),
        (selected + 1, moved)
    );
}
