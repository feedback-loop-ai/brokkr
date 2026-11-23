//! Decision 0125 rulings 3 and 5: a recipe's window, its phases' means
//! and the phases a running run may still enter, from planted rows.

use brokkr_store::{UsageEvent, UsageRow};
use serde_json::json;

use super::{learn, Peak};

/// A usage object of one seat's peaks.
fn seat(memory: u64, load: u64) -> String {
    json!({"peak_memory_bytes": memory, "mean_memory_bytes": 1, "peak_load_millicores": load,
           "mean_load_millicores": 1, "cpu_ms": 1, "wall_ms": 1})
    .to_string()
}

fn rows(run: &str, recipe: &str, day: u32, events: &[UsageEvent]) -> Vec<UsageRow> {
    (events.iter())
        .map(|event| UsageRow {
            run: run.into(),
            recipe: recipe.into(),
            created_at: format!("2026-11-{day:02}T00:00:00Z"),
            event: event.clone(),
        })
        .collect()
}

fn entered(phase: &str) -> UsageEvent {
    UsageEvent::Entered(phase.into())
}

fn ended(usage: String) -> UsageEvent {
    UsageEvent::Ended(usage)
}

fn peak(memory_bytes: u64, load_millicores: u64) -> Peak {
    Peak {
        memory_bytes,
        load_millicores,
    }
}

/// The window is the two newest measured runs whichever journal holds
/// them: the newest run carries no usage and is passed over, the oldest
/// is past the window. A panel's members are summed, an ending before any
/// phase and a usage that is not `effect-usage/v1` are not counted, and a
/// mean is rounded up.
#[test]
fn a_recipes_window_is_its_newest_measured_runs_and_each_phase_the_mean_of_its_attempts() {
    let panel = json!({"members": {"a": serde_json::from_str::<serde_json::Value>(&seat(100, 1000)).unwrap(),
                                   "b": serde_json::from_str::<serde_json::Value>(&seat(50, 500)).unwrap()}});
    let first_journal = [
        rows(
            "old",
            "self",
            1,
            &[entered("implement"), ended(seat(9_999, 9_999))],
        ),
        rows(
            "mid",
            "self",
            2,
            &[
                ended(seat(7, 7)),
                entered("implement"),
                ended(seat(10, 2000)),
                entered("verify"),
                ended(panel.to_string()),
            ],
        ),
    ];
    let second_journal = [
        rows(
            "new",
            "self",
            3,
            &[
                entered("implement"),
                ended(seat(21, 1001)),
                ended("{\"cpu\": 1}".into()),
            ],
        ),
        rows("bare", "self", 4, &[entered("implement")]),
        rows("other", "verify", 3, &[entered("build"), ended(seat(1, 1))]),
    ];
    let learned = learn(first_journal.into_iter().chain(second_journal).flatten(), 2);
    let profile = &learned["self"];
    assert_eq!(profile.runs, 2);
    assert_eq!(
        profile
            .phases
            .iter()
            .map(|(p, peak)| (p.as_str(), *peak))
            .collect::<Vec<_>>(),
        vec![("implement", peak(16, 1501)), ("verify", peak(150, 1500))]
    );
    assert_eq!(profile.peak(), peak(150, 1501));
    assert_eq!(learned["verify"].peak(), peak(1, 1));
    assert_eq!(learned.len(), 2);
}

/// A run in implement still holds its verify build; one in ship holds only
/// ship's; one in a phase the window never entered holds every phase.
#[test]
fn a_running_runs_remaining_phases_are_its_current_and_those_entered_after_it() {
    let usage = |memory: u64| ended(seat(memory, memory));
    let profile = learn(
        rows(
            "a",
            "self",
            1,
            &[
                entered("intake"),
                usage(5),
                entered("implement"),
                usage(10),
                entered("verify"),
                usage(40),
                entered("implement"),
                entered("ship"),
                usage(2),
            ],
        ),
        20,
    )
    .remove("self")
    .unwrap();
    assert_eq!(profile.remaining(Some("implement")), peak(40, 40));
    assert_eq!(profile.remaining(Some("ship")), peak(2, 2));
    assert_eq!(profile.remaining(Some("triage")), peak(40, 40));
    assert_eq!(profile.remaining(None), peak(40, 40));
}
