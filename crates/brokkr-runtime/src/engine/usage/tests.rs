use super::*;
use crate::engine::tests::{engine, single_body};
use crate::engine::{Engine, Selection};
use brokkr_core::fold::fold;
use brokkr_core::EventEnvelope;
use brokkr_protocol::AttemptOutcome;
use serde_json::json;

const MEASURED: AttemptUsage = AttemptUsage {
    peak_memory_bytes: 64 << 20,
    mean_memory_bytes: 48 << 20,
    peak_load_millicores: 2_000,
    mean_load_millicores: 833,
    cpu_ms: 2_500,
    wall_ms: 3_000,
};

fn measured_json() -> Value {
    json!({
        "peak_memory_bytes": 67_108_864,
        "mean_memory_bytes": 50_331_648,
        "peak_load_millicores": 2000,
        "mean_load_millicores": 833,
        "cpu_ms": 2500,
        "wall_ms": 3000
    })
}

/// The engine's own test report for `outcome`, measured as `usage`.
fn report(outcome: AttemptOutcome, usage: Option<AttemptUsage>) -> AttemptReport {
    AttemptReport {
        usage: usage.map(Box::new),
        ..crate::engine::tests::report(outcome, "")
    }
}

/// The three outcomes a seat reaches, each journaled as its own ending.
fn outcomes() -> [(AttemptOutcome, EventType); 3] {
    let succeeded = json!({"result": "complete", "inputs": {}, "notes": "done"});
    [
        (
            AttemptOutcome::Succeeded { result: succeeded },
            EventType::EffectSucceeded,
        ),
        (
            AttemptOutcome::Failed {
                error: "boom".into(),
            },
            EventType::EffectFailed,
        ),
        (
            AttemptOutcome::Indeterminate {
                reason: "lost".into(),
            },
            EventType::EffectIndeterminate,
        ),
    ]
}

/// Conclude `run` as the single seat's attempt `attempt_id` on `engine`,
/// and read back the event it ended in.
fn conclude(engine: &mut Engine, attempt_id: &str, run: DriverRun) -> EventEnvelope {
    let selection = Selection::new();
    let concluded = engine.conclude_single("effect", attempt_id, run, &selection, None);
    concluded.unwrap();
    let journal = engine.store.load(&engine.run_id).unwrap();
    journal.into_iter().next_back().unwrap()
}

/// Decision 0112 ruling 3: every ending carries the attempt's usage when
/// its report has it, and the same ending without one keeps its v1 keys.
#[test]
fn each_ending_carries_the_attempt_s_usage_and_omits_it_without() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    for (index, (outcome, event_type)) in outcomes().into_iter().enumerate() {
        let measured = report(outcome.clone(), Some(MEASURED));
        let ending = conclude(&mut engine, &format!("m{index}"), DriverRun::Ran(measured));
        assert_eq!(ending.event_type, event_type);
        assert_eq!(ending.payload["usage"], measured_json(), "{event_type:?}");
        let bare = report(outcome, None);
        let ending = conclude(&mut engine, &format!("b{index}"), DriverRun::Ran(bare));
        assert_eq!(ending.event_type, event_type);
        assert_eq!(ending.payload.get("usage"), None, "{event_type:?}");
    }
    // A driver that never spawned measured nothing, after one that did.
    let unspawned = DriverRun::SpawnFailed("no such driver".into());
    let ending = conclude(&mut engine, "s", unspawned);
    assert_eq!(ending.event_type, EventType::EffectFailed);
    assert_eq!(ending.payload.get("usage"), None);
}

/// A panel carries one entry per member a tick measured, by name; an
/// event of another attempt or of another type is left as it was.
#[test]
fn a_panel_carries_one_entry_per_measured_member_on_its_own_ending_only() {
    let failed = |usage| report(AttemptOutcome::Failed { error: "x".into() }, usage);
    let mut pending = Pending::default();
    pending.panel(
        "a1",
        &[
            ("left".into(), failed(Some(MEASURED))),
            (
                "right".into(),
                failed(Some(AttemptUsage {
                    cpu_ms: 1,
                    ..MEASURED
                })),
            ),
            ("unticked".into(), failed(None)),
        ],
    );
    let ending = json!({"effect_id": "e", "attempt_id": "a1", "error": "x"});
    let stamped = pending.stamp(EventType::EffectFailed, ending.clone());
    let mut right = measured_json();
    right["cpu_ms"] = json!(1);
    assert_eq!(
        stamped["usage"],
        json!({"members": {"left": measured_json(), "right": right}})
    );
    let other = json!({"effect_id": "e", "attempt_id": "a2", "error": "x"});
    assert_eq!(pending.stamp(EventType::EffectFailed, other.clone()), other);
    let checkpoint = json!({"effect_id": "e", "attempt_id": "a1", "checkpoint": {}});
    assert_eq!(
        pending.stamp(EventType::EffectCheckpointed, checkpoint.clone()),
        checkpoint
    );
    pending.panel("a1", &[("unticked".into(), failed(None))]);
    assert_eq!(
        pending.stamp(EventType::EffectFailed, ending.clone()),
        ending
    );
}

/// The fold reads none of it: the journal the engine wrote, with usage on
/// its ending, and the same journal without it fold to the same state.
#[test]
fn the_fold_is_blind_to_usage() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let requested = json!({"effect_id": "effect", "phase": "work", "seat": "work",
                           "idempotency_key": "k", "input_digest": "d"});
    let started = json!({"effect_id": "effect", "attempt_id": "a", "driver": "driver"});
    let entered = json!({"phase": "work"});
    engine
        .append(EventType::PhaseEntered, entered, None)
        .unwrap();
    engine
        .append(EventType::EffectRequested, requested, None)
        .unwrap();
    engine
        .append(EventType::EffectStarted, started, Some("a".into()))
        .unwrap();
    let failed = AttemptOutcome::Failed {
        error: "boom".into(),
    };
    conclude(
        &mut engine,
        "a",
        DriverRun::Ran(report(failed, Some(MEASURED))),
    );
    let with = engine.store.load(&engine.run_id).unwrap();
    let without: Vec<EventEnvelope> = with
        .iter()
        .cloned()
        .map(|mut event| {
            event.payload.as_object_mut().unwrap().remove("usage");
            event
        })
        .collect();
    let carried = with
        .iter()
        .filter(|event| event.payload.get("usage").is_some());
    assert_eq!(carried.count(), 1, "the ending carried usage");
    assert_eq!(
        format!("{:?}", fold(&with).unwrap()),
        format!("{:?}", fold(&without).unwrap())
    );
}
