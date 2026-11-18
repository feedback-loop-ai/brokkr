//! What each gate verdict recorded, read once (decision 0109 slice
//! 0109a1): a v7 result's findings and covered head, and nothing for a v6
//! result or a record that does not read.

use serde_json::{json, Value};

use super::*;
use crate::tests::ev;

const SITE: &str = "5a1e000000000000000000000000000000000000000000000000000000000000";
const T: &str = "2026-11-17T00:00:00Z";

fn run(results: &[(&str, Value)]) -> Vec<EventEnvelope> {
    let mut events = Vec::new();
    for (index, (phase, result)) in results.iter().enumerate() {
        let effect = format!("e{index}");
        let seq = events.len() as u64 + 1;
        events.push(ev(
            seq,
            EventType::EffectRequested,
            json!({"effect_id": effect, "phase": phase}),
            T,
        ));
        events.push(ev(
            seq + 1,
            EventType::EffectSucceeded,
            json!({"effect_id": effect, "attempt_id": "a", "result": result}),
            T,
        ));
    }
    events
}

fn finding(id: &str) -> Value {
    json!({"site": SITE, "id": id, "file": "a.rs", "summary": "s", "severity": "low",
           "security": false})
}

#[test]
fn a_v7_gate_reads_back_its_findings_and_covered_and_a_v6_one_reads_nothing() {
    let digest = "d".repeat(64);
    let events = run(&[
        ("implement", json!({"result": "complete", "notes": "n"})),
        (
            "verify",
            json!({"result": "pass",
                          "covered": {"kind": "head", "head": "a".repeat(40), "dirty": true}}),
        ),
        (
            "review",
            json!({"result": "residual", "findings": [finding("one"), finding("two")],
                          "covered": {"kind": "head", "head_digest": digest, "dirty": true}}),
        ),
        (
            "review",
            json!({"result": "residual", "inputs": {"max_residual_severity": "high"},
                          "notes": "one at a.rs:3"}),
        ),
        (
            "review",
            json!({"result": "residual", "findings": [{"id": "unstamped"}],
                          "covered": {"kind": "head", "dirty": true}}),
        ),
    ]);
    let records = gate_records(&events);
    let one = GateFinding {
        site: SITE.into(),
        id: "one".into(),
        file: "a.rs".into(),
        line: None,
        summary: "s".into(),
        severity: "low",
        security: false,
    };
    let two = GateFinding {
        id: "two".into(),
        ..one.clone()
    };
    assert_eq!(
        records,
        [
            GateRecord {
                phase: "verify".into(),
                findings: Vec::new(),
                covered: Some(format!("head {} · dirty", "a".repeat(40))),
            },
            GateRecord {
                phase: "review".into(),
                findings: vec![one, two],
                covered: Some(format!("head sha256 {digest} · dirty")),
            },
        ]
    );
    assert!(
        gate_records(&events[..2]).is_empty(),
        "a v6-shaped result records nothing"
    );
}
