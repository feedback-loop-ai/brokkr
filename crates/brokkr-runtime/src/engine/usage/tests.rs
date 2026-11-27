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
    let stamped = pending.stamp(EventType::EffectFailed, Some("a1"), ending.clone());
    let mut right = measured_json();
    right["cpu_ms"] = json!(1);
    assert_eq!(
        stamped["usage"],
        json!({"members": {"left": measured_json(), "right": right}})
    );
    let other = json!({"effect_id": "e", "attempt_id": "a2", "error": "x"});
    let failed_type = EventType::EffectFailed;
    assert_eq!(pending.stamp(failed_type, Some("a2"), other.clone()), other);
    assert_eq!(pending.stamp(failed_type, None, ending.clone()), ending);
    assert_eq!(
        pending.stamp(failed_type, Some("a1"), Value::Null),
        Value::Null
    );
    let checkpoint = json!({"effect_id": "e", "attempt_id": "a1", "checkpoint": {}});
    assert_eq!(
        pending.stamp(
            EventType::EffectCheckpointed,
            Some("a1"),
            checkpoint.clone()
        ),
        checkpoint
    );
    pending.panel("a1", &[("unticked".into(), failed(None))]);
    assert_eq!(
        pending.stamp(failed_type, Some("a1"), ending.clone()),
        ending
    );
}

/// A sequence's steps add up under the attempt they ran in, each keyed by
/// its tag; a step of the next attempt starts afresh, and a panel or a
/// seat replaces what was held.
#[test]
fn a_sequence_s_steps_add_up_by_tag_within_their_attempt() {
    let failed = |usage| report(AttemptOutcome::Failed { error: "x".into() }, usage);
    let ran = |usage| DriverRun::Ran(failed(usage));
    let ending = json!({"effect_id": "e", "attempt_id": "a1", "error": "x"});
    let usage_of = |pending: &Pending, attempt: &str| {
        let stamped = pending.stamp(EventType::EffectFailed, Some(attempt), ending.clone());
        stamped.get("usage").cloned()
    };
    let mut pending = Pending::default();
    pending.step("a1", "draft", &ran(Some(MEASURED)));
    pending.step("a1", "lost", &DriverRun::SpawnFailed("gone".into()));
    let members = [
        ("left".into(), failed(Some(MEASURED))),
        ("right".into(), failed(Some(MEASURED))),
    ];
    pending.panel_step("a1", "finish:", &members);
    let measured = measured_json();
    assert_eq!(
        usage_of(&pending, "a1"),
        Some(
            json!({"members": {"draft": measured, "finish:left": measured,
                                "finish:right": measured}})
        )
    );
    pending.step("a2", "draft", &ran(None));
    assert_eq!(usage_of(&pending, "a1"), None);
    assert_eq!(usage_of(&pending, "a2"), None);
    pending.step("a2", "draft", &ran(Some(MEASURED)));
    pending.seat("a2", &ran(Some(MEASURED)));
    assert_eq!(usage_of(&pending, "a2"), Some(measured_json()));
}

/// The engine stamps a sequence's ending with every step driver a tick
/// measured: the single step's under its name, the panel step's members
/// under `<step>:<member>`.
#[test]
fn a_sequence_attempt_s_ending_carries_each_step_driver_s_usage() {
    use crate::engine::tests::{
        draft_then_finish, driver_command, member, run_vocabulary, vocabulary_input,
    };
    // Each driver lives past several of the tracker's 100 ms ticks.
    let slow = |outcome| {
        let mut command = driver_command("vocabulary-effect", "vocabulary-attempt", outcome);
        command[2] = format!("sleep 0.6; {}", command[2]);
        command
    };
    let drafted = json!({"result": "drafted"});
    let failed = || AttemptOutcome::Failed { error: "no".into() };
    let left = member("left", slow(failed()));
    let right = member("right", slow(failed()));
    let draft = slow(AttemptOutcome::Succeeded { result: drafted });
    let steps = draft_then_finish("role.md", draft, &["drafted"], vec![left, right]);
    let body = crate::bundle::SeatBody::Sequence {
        steps: steps.clone(),
    };
    let (_dir, mut engine) = engine(body);
    let input = vocabulary_input(&engine);
    run_vocabulary(&mut engine, &steps, &input).unwrap();
    let journal = engine.store.load(&engine.run_id).unwrap();
    let ending = journal.last().unwrap();
    assert_eq!(ending.event_type, EventType::EffectFailed, "{journal:#?}");
    let tags: Vec<_> = ending.payload["usage"]["members"]
        .as_object()
        .into_iter()
        .flat_map(serde_json::Map::keys)
        .map(String::as_str)
        .collect();
    let expected = ["draft", "finish:left", "finish:right"];
    assert_eq!(
        tags,
        expected,
        "missing usage tags {:?}\nending payload\n{:#}",
        expected
            .into_iter()
            .filter(|tag| !tags.contains(tag))
            .collect::<Vec<_>>(),
        ending.payload
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
