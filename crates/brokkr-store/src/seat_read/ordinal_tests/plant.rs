//! The ordinal tests' journals planted in one transaction (#1130), through
//! [`plant_after`], and the test that holds every shape they plant to the
//! rows `append_next` leaves.

use brokkr_core::envelope::EventType;
use serde_json::Value as Json;

use crate::test_support::{append, opened};
use crate::tests::plant::{assert_planted_rows, plant_after};
use crate::Store;

/// Run `run_id` opened in `implement` by [`opened`], then `events`
/// planted in one transaction by [`plant_after`].
pub(super) fn plant(store: &mut Store, run_id: &str, events: Vec<(EventType, Json)>) {
    opened(store, run_id, "implement", &[]);
    plant_after(store, run_id, events);
}

/// Every shape the ordinal tests plant, one run each: the planted
/// participants, H1's interleaving, two effects on one attempt, three
/// disjoint spans, result-only records on the target's attempt and on
/// another, a member's record, a NUL member and more streams than one.
fn shapes() -> Vec<Vec<(EventType, Json)>> {
    use super::*;
    let mut sequential = two_effects_on_one_attempt();
    sequential.extend(own("gx", "a", 2));
    let stale = |attempt: &str| {
        let mut events = asked_and_started("fx", "a");
        events.push(checked("fx", "a", Some("target"), 1));
        events.extend(results("fx", attempt, "m", 3));
        events.push(checked("fx", "a", Some("target"), 3));
        events
    };
    let mut recorded = interleaved_member();
    recorded.extend(results("fx", "b", "m", 1));
    recorded.push(checked("fx", "b", Some("m"), 2));
    let mut nul = asked_and_started("fx", "a");
    nul.extend(turns("fx", "a", Some("m\u{0}suffix"), 3));
    let mut streams = vec![(EventType::EffectRequested, json!({"effect_id": "fx"}))];
    for attempt in ["a0", "a1", "a2"] {
        streams.push(started("fx", attempt));
        streams.extend(own("fx", attempt, 2));
    }
    vec![
        planted(),
        h1_events(),
        sequential,
        three_spans(4),
        stale("a"),
        stale("b"),
        recorded,
        nul,
        streams,
    ]
}

/// A planted journal holds the rows the writer appends: every shape the
/// ordinal tests use, appended event by event into one journal and
/// planted into another, holds the same rows in every table, `events`,
/// `run_summary`, the seat rows and each index among them, as
/// [`assert_planted_rows`] compares them.
#[test]
fn a_planted_journal_holds_the_rows_the_writer_appends() {
    let (_written_dir, mut written) = crate::tests::store();
    let (_planted_dir, mut planted) = crate::tests::store();
    let mut runs = Vec::new();
    for (at, events) in shapes().into_iter().enumerate() {
        let run_id = format!("r{at}");
        opened(&mut written, &run_id, "implement", &[]);
        for (kind, payload) in events.clone() {
            append(&mut written, &run_id, kind, payload);
        }
        plant(&mut planted, &run_id, events);
        runs.push(run_id);
    }
    let held = [
        "event_index",
        "events",
        "run_summary",
        "seat_head",
        "seat_summary",
    ];
    assert_planted_rows(&planted, &written, &runs, &held);
}
