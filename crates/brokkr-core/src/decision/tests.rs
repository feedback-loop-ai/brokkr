use super::*;
use serde_json::json;

fn decision(payload: Value) -> Decision {
    Decision::deserialize(&payload).unwrap()
}

/// Every way the contract's five optional fields can be written —
/// absent, `null` or a value, 3^5 payloads — reads and writes back to
/// exactly its own bytes: `null` and absent never collapse.
#[test]
fn every_decision_shape_round_trips_null_and_absent_apart() {
    let fields = [
        ("inputs", json!({"skip_verify": true, "nested": {"n": 1.5}})),
        ("next", json!("verify")),
        ("problem", json!("no rule matched")),
        ("rule_id", json!("IMPL-OK")),
        ("severity", json!("normal")),
    ];
    for shape in 0..3_u32.pow(5) {
        let mut payload = json!({"from": "implement", "result": "complete"});
        for (place, (name, value)) in fields.iter().enumerate() {
            match shape / 3_u32.pow(place as u32) % 3 {
                0 => {}
                1 => payload[name] = Value::Null,
                _ => payload[name] = value.clone(),
            }
        }
        let read = decision(payload.clone());
        assert_eq!(serde_json::to_value(&read).unwrap(), payload);
        assert_eq!(
            serde_json::to_string(&read).unwrap(),
            payload.to_string(),
            "the record prints its keys in the map's order"
        );
    }
}

#[test]
fn a_decision_reads_its_fields_as_recorded() {
    let full = decision(json!({
        "from": "review", "result": "clean", "rule_id": "REVIEW-OK",
        "next": "ship", "severity": "flagged", "problem": null,
        "inputs": {"reviewed_heads": {"repo": "abc"}, "spec_defect": null},
    }));
    assert_eq!(full.rule_id, Recorded::Present("REVIEW-OK".to_string()));
    assert_eq!(full.next.present().map(String::as_str), Some("ship"));
    assert_eq!(full.severity, Recorded::Present(RuleSeverity::Flagged));
    assert_eq!(full.problem, Recorded::Null);
    assert_eq!(full.problem.present(), None);
    assert_eq!(full.input("spec_defect"), Some(&Value::Null));
    assert_eq!(full.input("skip_verify"), None);
    let bare = decision(json!({"from": "review", "result": "clean"}));
    assert_eq!(bare.rule_id, Recorded::Absent);
    assert!(bare.rule_id.is_absent());
    assert!(!full.problem.is_absent());
    assert_eq!(bare.input("spec_defect"), None);
}

/// The contract closes the payload: a field outside it, or a field of
/// the wrong type, is refused, never dropped or coerced.
#[test]
fn a_decision_outside_its_contract_is_refused() {
    let refusal = |payload: Value| Decision::deserialize(&payload).unwrap_err().to_string();
    assert_eq!(
        refusal(json!({"from": "a", "result": "b", "verdict": "c"})),
        "unknown field `verdict`, expected one of `from`, `inputs`, `next`, `problem`, \
         `result`, `rule_id`, `severity`"
    );
    assert_eq!(
        refusal(json!({"from": "a", "result": "b", "next": 5})),
        "invalid type: integer `5`, expected a string"
    );
    assert_eq!(
        refusal(json!({"from": "a", "result": "b", "inputs": []})),
        "invalid type: sequence, expected a map"
    );
    assert_eq!(
        refusal(json!({"from": "a", "result": "b", "severity": "urgent"})),
        "unknown variant `urgent`, expected one of `normal`, `flagged`, `hard`"
    );
}

#[test]
fn a_phase_hands_forward_its_inputs_as_its_decision_recorded_them() {
    let handed = |payload: Value| json!(PhaseResult::of(&decision(payload)));
    assert_eq!(
        handed(json!({"from": "a", "result": "ok"})),
        json!({"inputs": {}, "result": "ok"})
    );
    assert_eq!(
        handed(json!({"from": "a", "result": "ok", "inputs": null})),
        json!({"inputs": null, "result": "ok"})
    );
    assert_eq!(
        handed(json!({"from": "a", "result": "ok", "inputs": {"change": "c1"}})),
        json!({"inputs": {"change": "c1"}, "result": "ok"})
    );
}

#[test]
fn reviewed_heads_count_every_realm_and_read_only_strings() {
    let mixed = ReviewedHeads::read(&json!({"alpha": "abc", "beta": 7, "gamma": "def"}));
    assert_eq!(mixed.heads().collect::<Vec<_>>(), vec!["abc", "def"]);
    assert_eq!(mixed.sole(), None, "three realms are ambiguous");
    assert_eq!(
        ReviewedHeads::read(&json!({"alpha": "abc"})).sole(),
        Some("abc")
    );
    assert_eq!(
        ReviewedHeads::read(&json!({"alpha": "abc", "beta": 7})).sole(),
        None,
        "a realm whose head does not read still counts"
    );
    assert_eq!(ReviewedHeads::read(&json!({"alpha": 7})).sole(), None);
    assert_eq!(ReviewedHeads::read(&json!({})).sole(), None);
    assert_eq!(ReviewedHeads::read(&json!(null)), ReviewedHeads::default());
}

#[test]
fn reviewed_heads_are_found_only_in_a_decisions_inputs() {
    let heads = |payload: Value| ReviewedHeads::recorded_in(&payload);
    assert_eq!(
        heads(json!({"inputs": {"reviewed_heads": {"repo": "abc"}}})),
        Some(ReviewedHeads::read(&json!({"repo": "abc"})))
    );
    assert_eq!(
        heads(json!({"inputs": {"reviewed_heads": null}})),
        Some(ReviewedHeads::default())
    );
    assert_eq!(heads(json!({"inputs": {"strategy": "feature"}})), None);
    assert_eq!(heads(json!({"inputs": ["reviewed_heads"]})), None);
    assert_eq!(heads(json!({"reviewed_heads": {"repo": "abc"}})), None);
}
