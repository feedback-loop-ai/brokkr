use super::*;
use crate::tests::ev;
use serde_json::{json, Value};

const AT: &str = "2026-11-20T00:00:00Z";

/// One driver's measures, each a multiple of `n`.
fn measures(n: u64) -> Value {
    json!({
        "peak_memory_bytes": 100 * n,
        "mean_memory_bytes": 50 * n,
        "peak_load_millicores": 1000 * n,
        "mean_load_millicores": 500 * n,
        "cpu_ms": 30 * n,
        "wall_ms": 60 * n
    })
}

/// A run of `recipe` (none when `None`), its `steps` each a phase entered
/// or an ending of `event_type` with the given `usage`, if any.
fn run(
    recipe: Option<&str>,
    steps: &[(&str, Option<EventType>, Option<Value>)],
) -> Vec<EventEnvelope> {
    let manifest = recipe.map_or_else(|| json!({}), |recipe| json!({"bundle_name": recipe}));
    let mut events = vec![ev(
        1,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": manifest}),
        AT,
    )];
    for (index, (phase, ending, usage)) in steps.iter().enumerate() {
        let seq = events.len() as u64 + 1;
        let Some(event_type) = ending else {
            events.push(ev(
                seq,
                EventType::PhaseEntered,
                json!({"phase": phase}),
                AT,
            ));
            continue;
        };
        let mut payload =
            json!({"effect_id": format!("e{index}"), "attempt_id": format!("a{index}")});
        payload["error"] = json!("x");
        if let Some(usage) = usage {
            payload["usage"] = usage.clone();
        }
        events.push(ev(seq, *event_type, payload, AT));
    }
    events
}

fn spread(min: u64, average: u64, max: u64) -> Option<Spread> {
    Some(Spread { min, average, max })
}

/// A row whose measured drivers' multiples are `min`, `average` and `max`
/// of [`measures`].
fn row(recipe: &str, phase: &str, counts: [usize; 3], spreads: Option<[u64; 3]>) -> PhaseUsage {
    let [attempts, unmeasured, measured] = counts;
    let by = |unit: u64| {
        spreads.and_then(|[min, average, max]| spread(min * unit, average * unit, max * unit))
    };
    PhaseUsage {
        recipe: recipe.into(),
        phase: phase.into(),
        attempts,
        unmeasured,
        measured,
        peak_memory_bytes: by(100),
        mean_memory_bytes: by(50),
        peak_load_millicores: by(1000),
        mean_load_millicores: by(500),
        cpu_ms: by(30),
        wall_ms: by(60),
    }
}

fn runs() -> Vec<Vec<EventEnvelope>> {
    let (ok, failed, lost) = (
        Some(EventType::EffectSucceeded),
        Some(EventType::EffectFailed),
        Some(EventType::EffectIndeterminate),
    );
    vec![
        run(
            Some("self"),
            &[
                ("implement", None, None),
                ("", ok, Some(measures(2))),
                ("review", None, None),
                (
                    "",
                    failed,
                    Some(json!({"members": {"a": measures(1), "b": measures(5)}})),
                ),
            ],
        ),
        run(
            Some("self"),
            &[
                ("implement", None, None),
                ("", failed, None),
                ("", lost, Some(measures(6))),
                ("", ok, Some(json!({"peak_memory_bytes": 1}))),
            ],
        ),
        run(
            Some("verify"),
            &[("check", None, None), ("", lost, Some(measures(3)))],
        ),
        run(Some("verify"), &[("check", None, None), ("", ok, None)]),
        run(
            None,
            &[("implement", None, None), ("", ok, Some(measures(9)))],
        ),
    ]
}

/// Decision 0112 ruling 4: per recipe and per phase, the spread of each
/// measure over the drivers measured. An ending without usage, or with a
/// usage that does not read, is unmeasured and moves no figure; a run
/// that names no recipe is in none.
#[test]
fn each_recipe_s_phases_spread_their_measured_drivers_and_count_the_rest_apart() {
    assert_eq!(
        recipe_usage(&runs(), None),
        vec![
            row("self", "implement", [4, 2, 2], Some([2, 4, 6])),
            row("self", "review", [1, 0, 2], Some([1, 3, 5])),
            row("verify", "check", [2, 1, 1], Some([3, 3, 3])),
        ]
    );
}

#[test]
fn one_recipe_is_read_alone_and_a_phase_never_measured_has_no_figure() {
    let mut runs = runs();
    runs.push(run(
        Some("verify"),
        &[("gate", None, None), ("", ok(), None)],
    ));
    assert_eq!(
        recipe_usage(&runs, Some("verify")),
        vec![
            row("verify", "check", [2, 1, 1], Some([3, 3, 3])),
            row("verify", "gate", [1, 1, 0], None),
        ]
    );
    assert_eq!(recipe_usage(&runs, Some("absent")), Vec::new());
}

fn ok() -> Option<EventType> {
    Some(EventType::EffectSucceeded)
}
