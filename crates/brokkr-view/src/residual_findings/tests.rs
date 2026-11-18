//! What each gate verdict recorded, read once (decision 0109 slice
//! 0109a1): a v7 result's findings and covered head, nothing for a v6
//! result or a covered record that does not read, and findings that do
//! not read said not to read, never shown as none (fix round 0109a1r3).

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
        participant: "e2".into(),
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
                key: "e1".into(),
                findings: Vec::new(),
                covered: Some(format!("head {} · dirty", "a".repeat(40))),
                unread: false,
            },
            GateRecord {
                phase: "review".into(),
                key: "e2".into(),
                findings: vec![one, two],
                covered: Some(format!("head sha256 {digest} · dirty")),
                unread: false,
            },
            GateRecord {
                phase: "review".into(),
                key: "e4".into(),
                findings: Vec::new(),
                covered: None,
                unread: true,
            },
        ]
    );
    assert!(
        gate_records(&events[..2]).is_empty(),
        "a v6-shaped result records nothing"
    );
}

/// A panel member's findings name the member whose checkpoints carry
/// their site, so a seat scope can keep them; a site no member stamped
/// is the effect's own.
#[test]
fn a_finding_names_the_panel_member_whose_checkpoints_carry_its_site() {
    let other = "07e4000000000000000000000000000000000000000000000000000000000000";
    let mut sited = finding("theirs");
    sited["site"] = json!(other);
    let mut unstamped = finding("nobody");
    unstamped["site"] = json!("f".repeat(64));
    let result = json!({"result": "residual", "findings": [finding("mine"), sited, unstamped]});
    let mut events = run(&[("review", result)]);
    for (seq, member, site) in [(3, "a", SITE), (4, "b", other)] {
        let checkpoint = json!({"member": member, "site_ref": site, "model": "m"});
        let payload = json!({"effect_id": "e0", "attempt_id": "a", "checkpoint": checkpoint});
        events.insert(1, ev(seq, EventType::EffectCheckpointed, payload, T));
    }
    let records = gate_records(&events);
    let named: Vec<&str> = (records[0].findings.iter())
        .map(|finding| finding.participant.as_str())
        .collect();
    assert_eq!(named, ["e0:a", "e0:b", "e0"]);
    assert_eq!(records[0].key, "e0");
}
