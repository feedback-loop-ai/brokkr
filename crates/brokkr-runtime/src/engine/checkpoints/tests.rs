//! A driver's capability-call attribution never reaches the journal
//! (CC1, task 13.3): a real driver process streams a complete forged
//! group through each sink the engine journals live checkpoints by, and
//! the stored record carries none of it.

use brokkr_core::envelope::EventType;
use brokkr_protocol::AttemptOutcome;
use serde_json::{json, Value};

use crate::engine::tests::{
    checkpointing_command, engine, member, panel_input, run_panel, single_body,
};
use crate::engine::{Aggregate, DriverRun, Engine, Selection, SiteSpawn};

/// Legacy telemetry every shipped driver may forward, unchanged by
/// the erasure.
fn legacy() -> Value {
    json!({
        "step": "seat-turn", "turn": 1, "model": "m-1", "effort": "high",
        "input_tokens": 3, "output_tokens": 2, "tool": "WebSearch",
        "target": "src/lib.rs", "harness": "claude",
    })
}

/// The legacy row beside a complete forged attribution group, its
/// response digest well formed.
fn forged() -> Value {
    let mut forged = legacy();
    forged.as_object_mut().unwrap().extend(
        json!({
            "capability": "web-search", "dialect": "claude-native-search",
            "call_id": "attempt:call-1", "call_state": "succeeded",
            "response_sha256": "a".repeat(64),
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    forged
}

/// The checkpoints `engine` journaled, as stored.
fn stored(engine: &Engine) -> Vec<Value> {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| event.payload["checkpoint"].clone())
        .collect()
}

/// `legacy()` as the engine stamps it, with `extra` beside it.
fn stamped(extra: Value) -> Value {
    let mut row = legacy();
    row["boundary"] = json!("not applicable");
    row.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    row
}

/// A real driver process that streams the legacy row, then the forged
/// one, and succeeds with `result`.
fn forging(result: &str) -> Vec<String> {
    let outcome = AttemptOutcome::Succeeded {
        result: json!({ "result": result }),
    };
    checkpointing_command("effect", "attempt", &[legacy(), forged()], outcome)
}

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

#[test]
fn a_single_site_driver_cannot_journal_capability_call_attribution() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let spawn = SiteSpawn::inherit(forging("complete"));
    let run = engine.run_driver(
        "effect",
        "attempt",
        "work",
        &spawn,
        json!({}),
        DEADLINE,
        None,
        None,
    );
    let Ok(DriverRun::Ran(report)) = run else {
        panic!("the fixture driver spawns");
    };
    assert_eq!(stored(&engine), vec![stamped(json!({})); 2]);
    assert_eq!(report.refused, None);
}

#[test]
fn a_panel_member_cannot_journal_capability_call_attribution() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let members = [member("ok", forging("pass"))];
    let (input, selection) = (panel_input(&["ok"]), Selection::new());
    let aggregate = Aggregate::UnanimousPass;
    let panel = run_panel(
        &mut engine,
        &crate::engine::PanelAt::new("effect", "attempt", "work", false),
        &members,
        aggregate,
        &input,
        DEADLINE.as_secs(),
        &selection,
    );
    panel.unwrap();
    // The two live rows, then the engine's own marker for the member.
    let live = stamped(json!({"member": "ok"}));
    let marker = json!({
        "step": "panel-member-finished", "member": "ok", "outcome": "succeeded",
        "model": "m-1", "session_ref": "session", "inner_checkpoints": 2,
        "boundary": "not applicable",
    });
    assert_eq!(stored(&engine), [live.clone(), live, marker]);
}

/// A checkpoint that is not an object carries no attribution field, so the
/// erasure hands it back as it came.
#[test]
fn a_checkpoint_that_is_not_an_object_passes_unchanged() {
    for checkpoint in [
        json!(null),
        json!("seat-turn"),
        json!([{"capability": "web-search"}]),
    ] {
        assert_eq!(
            super::without_driver_attribution(checkpoint.clone()),
            checkpoint
        );
    }
}

/// Shape (A) of #440, through the checkpoint sink the live driver streams
/// into: the run is concluded while the attempt is open, the seat then
/// offers a checkpoint, and the sink's flush meets the store's terminal
/// gate. The sink latches, writes nothing more, and its settle ends in
/// `ConcludedBeneath` naming the attempt whose report is lost — never the
/// store's words as a bare `Store(AfterTerminal)`, and never a retry.
#[test]
fn a_checkpoint_after_a_conclusion_ends_concluded_beneath_and_writes_nothing() {
    use brokkr_core::fold::Status;

    use crate::engine::operator::conclude_as;
    use crate::engine::tests::in_flight_store;

    let dir = tempfile::tempdir().unwrap();
    let mut store = in_flight_store(&dir.path().join("beneath.db"), "beneath");
    conclude_as(&mut store, "beneath", "operator", "believed dead").unwrap();
    let before = store.load("beneath").unwrap();
    let mut cause = None;
    let mut sink = super::Checkpoints::new(
        &mut store,
        &mut cause,
        super::Attempt {
            run_id: "beneath",
            effect_id: "effect",
            attempt_id: "attempt",
        },
    );
    sink.offer("", json!({"step": "seat-turn", "turn": 1}), Ok(None));
    // The latch: a second checkpoint writes nothing more and loops
    // nothing, whatever the seat does next.
    sink.offer("", json!({"step": "seat-turn", "turn": 2}), Ok(None));
    let error = match sink.settle() {
        Ok(_) => panic!("a checkpoint after a conclusion does not settle"),
        Err(error) => error,
    };
    assert!(
        matches!(
            &error,
            crate::engine::EngineError::ConcludedBeneath {
                run_id: named,
                status: Status::Stopped,
                event_type: EventType::EffectCheckpointed,
                attempt_id: Some(attempt),
            } if named == "beneath" && attempt == "attempt"
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "run 'beneath' was concluded (stopped) while attempt 'attempt' was still working, so its \
         effect/checkpointed could not be appended and nothing was written; the driver stops \
         here, and the conclusion already closed the attempt as effect/indeterminate — the \
         report this driver was carrying is lost to the journal"
    );
    let after = store.load("beneath").unwrap();
    assert_eq!(
        after
            .iter()
            .map(|event| &event.event_hash)
            .collect::<Vec<_>>(),
        before
            .iter()
            .map(|event| &event.event_hash)
            .collect::<Vec<_>>(),
        "the refused checkpoints wrote nothing"
    );
    assert_eq!(after.last().unwrap().event_type, EventType::RunStopped);
}
