//! Decision 0065 slice two, U4g: the view derives each checkpoint's
//! capability-call evidence once, from the journal's settled row alone.
//! A child of the owning `tests.rs`, which sits at its line baseline.

use super::*;
use crate::capability_calls::{CallState, CapabilityCall, NO_RETAINED, UNRECORDED};

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// One seat attempt whose checkpoints are `checkpoints`, in order.
fn seat_with(checkpoints: Vec<Value>) -> Vec<EventEnvelope> {
    let mut events = vec![
        ev(1, EventType::RunStarted, json!({"feature": "call it"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "research"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "research", "phase": "research"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
            T0,
        ),
    ];
    for checkpoint in checkpoints {
        let seq = events.len() as u64 + 1;
        events.push(ev(
            seq,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1", "checkpoint": checkpoint}),
            T1,
        ));
    }
    events
}

/// A seat-turn checkpoint carrying a complete attribution group.
fn attributed(capability: &str, dialect: &str, tool: &str, call: &str, state: &str) -> Value {
    json!({"step": "seat-turn", "tool": tool, "capability": capability,
           "dialect": dialect, "call_id": call, "call_state": state})
}

fn call(capability: &str, dialect: &str, tool: &str, id: &str, state: CallState) -> CapabilityCall {
    CapabilityCall {
        capability: capability.to_string(),
        dialect: dialect.to_string(),
        tool: tool.to_string(),
        call_id: id.to_string(),
        state,
        response_sha256: None,
    }
}

/// Every one of v6's five states reaches the row exactly: the native
/// observation claims no completion, the refusal names the tool it
/// denied without a digest or a success, and only a kept response says
/// so. Two rows sharing a call id stay two rows: no stage is grouped.
#[test]
fn every_recorded_call_state_reaches_its_checkpoint_row_exactly() {
    let mut observed = attributed(
        "web-search",
        "claude-native-search",
        "WebSearch",
        "att1:toolu_1",
        "observed",
    );
    observed["turn"] = json!(1);
    let mut succeeded = attributed(
        "library-docs",
        "cap-library-docs",
        "lookup",
        "att1:b:1",
        "succeeded",
    );
    succeeded["response_sha256"] = json!(DIGEST);
    let view = run_view(
        &seat_with(vec![
            observed,
            succeeded,
            attributed(
                "library-docs",
                "cap-library-docs",
                "lookup",
                "att1:b:2",
                "failed",
            ),
            attributed(
                "library-docs",
                "cap-library-docs",
                "admin",
                "att1:b:3",
                "refused",
            ),
            attributed(
                "library-docs",
                "cap-library-docs",
                "lookup",
                "att1:b:3",
                "interrupted",
            ),
        ]),
        None,
    );
    let rows = &view.participants[0].checkpoints;
    assert_eq!(rows.len(), 5);
    let calls: Vec<_> = rows
        .iter()
        .map(|row| row.capability_call.call.clone())
        .collect();
    let docs = |tool, id, state| call("library-docs", "cap-library-docs", tool, id, state);
    let mut kept = docs("lookup", "att1:b:1", CallState::Succeeded);
    kept.response_sha256 = Some(DIGEST.to_string());
    assert_eq!(
        calls,
        vec![
            Some(call(
                "web-search",
                "claude-native-search",
                "WebSearch",
                "att1:toolu_1",
                CallState::Observed
            )),
            Some(kept),
            Some(docs("lookup", "att1:b:2", CallState::Failed)),
            Some(docs("admin", "att1:b:3", CallState::Refused)),
            Some(docs("lookup", "att1:b:3", CallState::Interrupted)),
        ]
    );
    let texts: Vec<_> = rows
        .iter()
        .map(|row| row.capability_call.cell.text.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "web-search via claude-native-search · WebSearch observed",
            "library-docs via cap-library-docs · lookup succeeded · response retained",
            "library-docs via cap-library-docs · lookup failed",
            "library-docs via cap-library-docs · admin refused",
            "library-docs via cap-library-docs · lookup interrupted",
        ]
    );
    assert!(rows.iter().all(|row| !row.capability_call.cell.absent));
    let wire = serde_json::to_value(&rows[3].capability_call).unwrap();
    assert_eq!(wire["call"]["state"], "refused");
    assert_eq!(wire["call"]["response_sha256"], Value::Null);
}

/// A v5 `WebSearch` checkpoint stays unrecorded: no name supplies the
/// capability it never carried. So does an ordinary tool, a non-tool
/// step, a group missing one field or carrying an empty one, and a state
/// outside the vocabulary — a broker's private `started` is never a
/// public call.
#[test]
fn a_checkpoint_without_a_whole_recorded_group_reads_unrecorded() {
    let mut partial = attributed(
        "web-search",
        "claude-native-search",
        "WebSearch",
        "c1",
        "observed",
    );
    partial.as_object_mut().unwrap().remove("call_id");
    let view = run_view(
        &seat_with(vec![
            json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch"}),
            json!({"step": "seat-turn", "turn": 2, "tool": "Read", "target": "a.rs"}),
            json!({"step": "claude-session-finished", "total_cost_usd": 0.5}),
            partial,
            attributed("", "claude-native-search", "WebSearch", "c3", "observed"),
            attributed(
                "library-docs",
                "cap-library-docs",
                "lookup",
                "c2",
                "started",
            ),
        ]),
        None,
    );
    let rows = &view.participants[0].checkpoints;
    assert_eq!(rows.len(), 6);
    for row in rows {
        let evidence = &row.capability_call;
        assert_eq!(evidence.call, None, "{}", row.step);
        assert_eq!(evidence.cell.text, ABSENT, "{}", row.step);
        assert!(evidence.cell.absent, "{}", row.step);
        assert_eq!(
            evidence.cell.note.as_deref(),
            Some(UNRECORDED),
            "{}",
            row.step
        );
    }
    assert_eq!(UNRECORDED, "capability attribution unrecorded");
}

/// U8f: a checkpoint's retained response reads three ways. A historical
/// row is unrecorded; an attributed call of any settled outcome that
/// cites no digest says only that none is recorded — no veto, no grant,
/// no delivery is inferred from the absence; and a succeeded or failed
/// call that kept one carries its digest.
#[test]
fn the_response_cell_tells_unrecorded_from_uncited_from_retained() {
    let docs = |call: &str, state: &str| {
        attributed("library-docs", "cap-library-docs", "lookup", call, state)
    };
    let mut kept = docs("att1:b:1", "succeeded");
    kept["response_sha256"] = json!(DIGEST);
    let mut failed_kept = docs("att1:b:2", "failed");
    failed_kept["response_sha256"] = json!(DIGEST);
    let view = run_view(
        &seat_with(vec![
            json!({"step": "seat-turn", "turn": 1, "tool": "WebSearch"}),
            attributed(
                "web-search",
                "claude-native-search",
                "WebSearch",
                "att1:t",
                "observed",
            ),
            kept,
            failed_kept,
            docs("att1:b:3", "succeeded"),
            docs("att1:b:4", "failed"),
            docs("att1:b:5", "refused"),
            docs("att1:b:6", "interrupted"),
        ]),
        None,
    );
    let responses: Vec<_> = view.participants[0]
        .checkpoints
        .iter()
        .map(|row| &row.capability_call.response)
        .collect();
    let absent = |note: &str| Cell {
        text: ABSENT.to_string(),
        absent: true,
        note: Some(note.to_string()),
    };
    let retained = || Cell {
        text: DIGEST.to_string(),
        absent: false,
        note: None,
    };
    let expected = [
        absent(UNRECORDED),
        absent(NO_RETAINED),
        retained(),
        retained(),
        absent(NO_RETAINED),
        absent(NO_RETAINED),
        absent(NO_RETAINED),
        absent(NO_RETAINED),
    ];
    assert_eq!(responses, expected.iter().collect::<Vec<_>>());
    let refused = &view.participants[0].checkpoints[6].capability_call;
    assert_eq!(
        refused.call.as_ref().map(|call| call.state),
        Some(CallState::Refused)
    );
    assert_eq!(
        NO_RETAINED,
        "no retained response is recorded for this call"
    );
}

/// U8f: the projection's wire form is pinned — each state's one spelling,
/// the call, its line and its response cell, and an unrecorded row.
#[test]
fn the_call_evidence_serializes_exactly() {
    let states = [
        CallState::Observed,
        CallState::Succeeded,
        CallState::Failed,
        CallState::Refused,
        CallState::Interrupted,
    ];
    assert_eq!(
        serde_json::to_value(states).unwrap(),
        json!(["observed", "succeeded", "failed", "refused", "interrupted"])
    );
    let mut kept = attributed(
        "library-docs",
        "cap-library-docs",
        "lookup",
        "att1:b:1",
        "failed",
    );
    kept["response_sha256"] = json!(DIGEST);
    let view = run_view(
        &seat_with(vec![kept, json!({"step": "seat-turn", "turn": 2})]),
        None,
    );
    let wire: Vec<_> = view.participants[0]
        .checkpoints
        .iter()
        .map(|row| serde_json::to_value(&row.capability_call).unwrap())
        .collect();
    assert_eq!(
        wire,
        [
            json!({
                "call": {"capability": "library-docs", "dialect": "cap-library-docs",
                         "tool": "lookup", "call_id": "att1:b:1", "state": "failed",
                         "response_sha256": DIGEST},
                "cell": {"text": "library-docs via cap-library-docs · lookup failed · response retained",
                         "absent": false, "note": null},
                "response": {"text": DIGEST, "absent": false, "note": null},
            }),
            json!({
                "call": null,
                "cell": {"text": ABSENT, "absent": true, "note": UNRECORDED},
                "response": {"text": ABSENT, "absent": true, "note": UNRECORDED},
            }),
        ]
    );
}

/// U8f: the group is parsed once, so a field of the wrong type fails it
/// whole, while a digest that is not a nonempty string is no digest and
/// costs the call none of its attribution.
#[test]
fn a_wrongly_typed_field_fails_the_group_but_a_bad_digest_only_itself() {
    let mut numbered = attributed("library-docs", "cap-library-docs", "lookup", "c1", "failed");
    numbered["call_id"] = json!(7);
    let mut odd = attributed(
        "library-docs",
        "cap-library-docs",
        "lookup",
        "c2",
        "succeeded",
    );
    odd["response_sha256"] = json!(7);
    let mut empty = attributed("library-docs", "cap-library-docs", "lookup", "c3", "failed");
    empty["response_sha256"] = json!("");
    let view = run_view(&seat_with(vec![numbered, odd, empty]), None);
    let calls: Vec<_> = view.participants[0]
        .checkpoints
        .iter()
        .map(|row| row.capability_call.call.clone())
        .collect();
    let docs = |id, state| call("library-docs", "cap-library-docs", "lookup", id, state);
    assert_eq!(
        calls,
        [
            None,
            Some(docs("c2", CallState::Succeeded)),
            Some(docs("c3", CallState::Failed)),
        ]
    );
}
