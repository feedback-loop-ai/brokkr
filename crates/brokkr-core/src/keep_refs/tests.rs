use super::*;
use serde_json::json;

const A: &str = "1111111111111111111111111111111111111111";
const B: &str = "2222222222222222222222222222222222222222";
const C: &str = "3333333333333333333333333333333333333333";
/// A SHA-256 repository's object name — 64 hex, equally an exhibit.
const WIDE: &str = "44444444444444444444444444444444444444444444444444444444444444ff";

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EventEnvelope {
        run_id: "r1".into(),
        seq: 2,
        event_id: "e2".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "r1".into(),
        attempt_id: None,
        recorded_at: "2026-08-23T00:00:00Z".into(),
        previous_hash: String::new(),
        event_hash: "hash".into(),
    }
}

fn decided(inputs: Value) -> EventEnvelope {
    event(
        EventType::TransitionDecided,
        json!({"from": "review", "result": "ok", "next": "ship", "inputs": inputs}),
    )
}

#[test]
fn every_head_a_run_ever_cited_is_collected_in_both_shapes() {
    // A reforged run revisits review and cites a second head; ship then
    // records a third per realm. State would keep only the last — the
    // exhibits are all three, plus the legacy unkeyed shape.
    let events = [
        decided(json!({"reviewed_heads": {"the-forge": A}})),
        decided(json!({"reviewed_heads": {"the-forge": B}})),
        decided(json!({"reviewed_heads": {"repo": WIDE}})),
        decided(json!({
            "reviewed_heads": {"the-forge": B},
            "realm_facts": {"the-forge": {"head": C, "dirty_worktrees": false}},
        })),
    ];
    assert_eq!(
        cited_shas(&events),
        BTreeSet::from([
            A.to_string(),
            B.to_string(),
            C.to_string(),
            WIDE.to_string()
        ]),
    );
}

#[test]
fn one_head_cited_twice_is_one_exhibit_whatever_its_case() {
    let events = [
        decided(json!({"reviewed_heads": {"the-forge": A}})),
        decided(json!({"reviewed_heads": {"the-forge": A}})),
        decided(json!({"reviewed_heads": {"the-forge": A.to_uppercase()}})),
        decided(json!({"realm_facts": {"the-forge": {"head": A}}})),
    ];
    assert_eq!(cited_shas(&events), BTreeSet::from([A.to_string()]));
}

#[test]
fn only_decisions_and_only_object_names_are_citations() {
    // A head-shaped string anywhere else in the vocabulary is not a
    // citation, and neither is a branch name, a truncated head or a
    // non-string sitting where a head would be.
    let events = [
        event(EventType::RunStarted, json!({"feature": A})),
        event(
            EventType::EffectSucceeded,
            json!({"inputs": {"reviewed_heads": {"the-forge": A}}}),
        ),
        decided(json!({"reviewed_heads": {"the-forge": "main"}})),
        decided(json!({"reviewed_heads": {"the-forge": &A[..12]}})),
        decided(json!({"reviewed_heads": {"the-forge": 7}})),
        decided(json!({"reviewed_heads": "not-an-object"})),
        decided(json!({"realm_facts": {"the-forge": {"dirty_worktrees": true}}})),
        decided(json!({"realm_facts": "not-an-object"})),
        event(EventType::TransitionDecided, json!({"from": "review"})),
        decided(json!({"reviewed_heads": {"the-forge": B}})),
    ];
    assert_eq!(cited_shas(&events), BTreeSet::from([B.to_string()]));
}

#[test]
fn a_journal_that_cites_nothing_plants_nothing() {
    assert!(cited_shas(&[]).is_empty());
    assert!(cited_shas(&[event(EventType::RunStarted, json!({}))]).is_empty());
}
