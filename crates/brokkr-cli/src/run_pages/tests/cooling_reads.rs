//! A frame read at one instant: a trail that ends in a repaired tail
//! stands no wait, and a frame kept at a standing head re-derives the
//! aggregate a probe's later round extends (decision 0108 ruling 4;
//! decision 0103 slice 4f).

use super::*;

#[path = "../../../../../tests/support/late_wait.rs"]
mod late_wait;

/// A frame whose trail ends in a repaired tail, its repair record outside
/// this read, paints no wait: the tail is kept and shown and never applied
/// (decision 0108 ruling 4; decision 0103 slice 4f). Reading the journal's
/// last row instead paints the late wait.
#[test]
fn a_frame_whose_trail_ends_in_a_late_wait_paints_none() {
    let events = late_wait::trail("late");
    let mut repaired = crate::tests::run_state::run_state(
        "late",
        8,
        crate::tests::run_state::standing(brokkr_core::fold::Status::Stopped, ""),
    );
    repaired.late_tail = Some(late_wait::tail());
    repaired.terminal_seq = Some(7);
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let now = "2026-11-13T00:00:00Z";
    let waits = RunWaits::read(&store, "late", now, &events, Some(&repaired)).unwrap();
    let frame = Frame {
        head: 8,
        trail: events.to_vec(),
        seats: Vec::new(),
        causes: std::collections::BTreeMap::new(),
        state: Box::new(repaired),
        last_at: now.to_string(),
        waits,
    };
    let view = frame.view().expect("the frame composes");
    assert!(view.waits.waits.is_empty(), "{:?}", view.waits);
    assert_eq!(view.waits.refused, None);
}

/// The console refreshes the waits of a frame whose head stands: a probe
/// can update the wait's end and kind without appending an event.
#[test]
fn a_frame_kept_at_a_standing_head_re_derives_the_probe_s_later_end() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let event_seq = crate::tests::cooling::cooling_attempt(&db);
    let mut store = Store::open(&db).unwrap();
    let now = "2026-11-20T00:00:00Z";
    let reading = Reading::new(0, 0, now);
    let head = store.head_hash("r1").unwrap();
    let (shown, _) = console_with(&store, "r1", Console::Run, None, Some(&head), reading).unwrap();
    assert_eq!(
        shown.view().waits.waits[0].until,
        crate::tests::cooling::COOLING_FIRST
    );
    crate::tests::cooling::extend_held_cooling(&mut store, event_seq, "a", "anthropic");
    assert_eq!(store.head_hash("r1").unwrap(), head);
    let (shown, read) = console_with(
        &store,
        "r1",
        Console::Run,
        Some(shown),
        Some(&head),
        reading,
    )
    .unwrap();
    assert_eq!(read, 0);
    let view = shown.view();
    let wait = &view.waits.waits[0];
    assert_eq!(wait.until, crate::tests::cooling::COOLING_LATER);
    assert_eq!(
        wait.kind,
        brokkr_view::WaitKind::Held {
            max_seconds: crate::tests::cooling::PROBE_MAX,
        }
    );
}

/// A run concluded, then a chained `effect/failed` carrying waits and no
/// repair: the fold refuses, so the whole read `inspect` makes paints no
/// wait from the journal's last row. The run is refused, and the late
/// event is never applied (decision 0108 ruling 4; decision 0103 slice
/// 4f).
#[test]
fn a_whole_read_refused_by_a_late_wait_paints_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    crate::tests::cooling::cooling_attempt(&db);
    let mut store = Store::open(&db).unwrap();
    store
        .append_next(
            "r1",
            brokkr_core::EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
    let head = store.load("r1").unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_chained(
        &connection,
        "r1",
        &head,
        &[(
            brokkr_core::EventType::EffectFailed,
            json!({"effect_id": "late", "waits": [{
                "site": "a", "provider": "claude", "route": "anthropic",
                "words": "rate_limit (HTTP 429)", "seconds": 600,
                "until": "2099-01-01T02:00:00Z", "rounds": 1
            }]}),
        )],
    )
    .unwrap();
    drop(connection);

    let store = Store::open_read_only(&db).unwrap();
    let viewed = whole(&store, "r1", "2099-01-01T00:00:00Z").unwrap();
    assert!(viewed.folded.is_err(), "the fold refuses the late event");
    assert!(
        viewed.view.waits.waits.is_empty(),
        "a refused fold stands no wait: {:?}",
        viewed.view.waits
    );
    assert_eq!(viewed.view.waits.refused, None);
}
