//! Fix round 0109a1r4, K1, through the live admission path: no record a
//! driver writes reads as an engine verdict. A driver's live checkpoint
//! that claims the engine's step marker, or whose nested result carries
//! what only the engine writes, is refused and fails the attempt, and
//! inspect shows no gate verdict for it.

use serde_json::{json, Value};

use super::tests::{checkpointing, driven, ended, reported};
use crate::bundle::SiteKey;
use crate::engine::tests::{own_engine, single_body};
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_view::gate_records;

/// The adversarial position's case: a nested result carrying a
/// seat-supplied covered head and a finding with a site.
fn planted() -> Value {
    let mut sited = reported("planted", 7);
    sited["site"] = json!(SiteKey::single("work", None).digest());
    json!({"result": "clean", "findings": [sited],
           "covered": {"kind": "head", "head": "a".repeat(40), "dirty": false}})
}

/// Drive the work seat at `gate` with a driver forwarding `checkpoint`
/// live before an otherwise valid success; each attempt's failure text.
fn refused(checkpoint: Value, gate: bool) -> (Vec<EventEnvelope>, Vec<String>) {
    let body = single_body(checkpointing(&[checkpoint], &json!({"result": "complete"})));
    let (_dir, events) = driven(body, gate);
    assert!(ended(&events, EventType::EffectSucceeded).is_empty());
    let errors = ended(&events, EventType::EffectFailed)
        .iter()
        .map(|failure| failure["error"].as_str().unwrap().to_string())
        .collect();
    (events, errors)
}

fn journaled_steps(events: &[EventEnvelope]) -> Vec<&Value> {
    (events.iter())
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| &event.payload["checkpoint"]["step"])
        .collect()
}

#[test]
fn a_work_drivers_live_step_marker_is_refused_and_inspect_shows_no_verdict() {
    own_engine!();
    let marker = json!({"step": "sequence-step-finished", "step_name": "judge",
                        "model": "m", "result": planted()});
    let (events, errors) = refused(marker, false);
    assert_eq!(errors.len(), 2);
    for error in &errors {
        assert!(
            error.starts_with(
                "a driver's checkpoint claims a record only the engine authors \
                 (decision 0109 ruling 1)"
            ),
            "{error}"
        );
    }
    assert!(!journaled_steps(&events).contains(&&json!("sequence-step-finished")));
    assert_eq!(gate_records(&events, None), []);
}

#[test]
fn a_live_checkpoints_nested_result_passes_the_standalone_admission() {
    own_engine!();
    let cases = [
        (
            json!({"step": "seat-turn", "turn": 1, "result": planted()}),
            true,
            "a driver's checkpoint result was refused: the result carries 'covered', which \
             only the engine writes (decision 0109 ruling 1)",
        ),
        (
            json!({"step": "seat-turn", "turn": 1,
                   "result": {"result": "clean", "findings": [reported("work", 1)]}}),
            false,
            "a driver's checkpoint result was refused: the result carries 'findings' from a \
             work site: only a gate reports findings (decision 0109 ruling 1)",
        ),
    ];
    for (checkpoint, gate, want) in cases {
        let (events, errors) = refused(checkpoint, gate);
        assert_eq!(errors.len(), 2, "{want}");
        for error in &errors {
            assert!(error.starts_with(want), "{error}");
        }
        assert!(!journaled_steps(&events).contains(&&json!("seat-turn")));
        assert_eq!(gate_records(&events, None), []);
    }
}

/// Every finding object anywhere in `value` that carries a `site`.
fn keyed(value: &Value) -> usize {
    match value {
        Value::Object(fields) => {
            let own = fields.get("findings").and_then(Value::as_array);
            let sited = own.map_or(0, |all| {
                all.iter().filter(|f| f.get("site").is_some()).count()
            });
            sited + fields.values().map(keyed).sum::<usize>()
        }
        Value::Array(items) => items.iter().map(keyed).sum(),
        _ => 0,
    }
}

/// Fix round 0109a1r5, V4: a gate driver forwards an ordinary live
/// seat-turn checkpoint whose nested result reports one finding, then
/// succeeds with none. Findings belong only to verdicts, so the
/// checkpoint is refused in its own words, journaled as each attempt's
/// failure, and no keyed finding is left in the journal outside a verdict.
#[test]
fn a_gate_drivers_live_finding_is_refused_and_no_keyed_finding_is_journaled() {
    own_engine!();
    let turn = json!({"step": "seat-turn", "turn": 1,
                      "result": {"result": "clean", "findings": [reported("telemetry", 1)]}});
    let (events, errors) = refused(turn, true);
    assert_eq!(errors.len(), 2);
    for error in &errors {
        let want = "a driver's checkpoint carries a finding: findings belong only to the \
                    seat's result, its verdict (decision 0109 ruling 1)";
        assert!(error.starts_with(want), "{error}");
    }
    assert!(!journaled_steps(&events).contains(&&json!("seat-turn")));
    let keyed: usize = events.iter().map(|event| keyed(&event.payload)).sum();
    assert_eq!(keyed, 0);
    assert_eq!(gate_records(&events, None), []);
}
