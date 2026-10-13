//! Fold behavior: protocol-shape enforcement, counters, replay
//! determinism, and the crash-recovery cursor positions.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::decision::{Decision, ReviewedHeads};
use brokkr_core::envelope::{verify_chain, EventEnvelope, EventType};
use brokkr_core::fold::{computed_inputs, fold, Cursor, FoldError, Status};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;
use envelope_builder::EnvelopeBuilder;

struct Journal {
    events: Vec<EventEnvelope>,
}

impl Journal {
    fn new() -> Self {
        let mut journal = Journal { events: Vec::new() };
        journal.append(
            EventType::RunStarted,
            json!({"feature": "t", "manifest": {}}),
        );
        journal
    }

    fn append(&mut self, event_type: EventType, payload: Value) -> &EventEnvelope {
        let seq = self.events.len() as u64 + 1;
        let previous_hash = self
            .events
            .last()
            .map(|e| e.event_hash.clone())
            .unwrap_or_else(|| ZERO_HASH.to_string());
        let envelope = EnvelopeBuilder::new(event_type, payload)
            .run("r1")
            .seq(seq)
            .event_id(format!("e{seq}"))
            .at("2026-08-23T00:00:00Z")
            .previous(previous_hash)
            .sealed();
        self.events.push(envelope);
        self.events.last().unwrap()
    }
}

fn effect_cycle(journal: &mut Journal, phase: &str, result_payload: Value) {
    journal.append(EventType::PhaseEntered, json!({"phase": phase}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": format!("fx-{phase}"), "phase": phase, "seat": phase,
               "idempotency_key": format!("k-{phase}"), "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectStarted,
        json!({"effect_id": format!("fx-{phase}"), "attempt_id": "a1", "driver": "fake"}),
    );
    journal.append(
        EventType::EffectSucceeded,
        json!({"effect_id": format!("fx-{phase}"), "attempt_id": "a1", "result": result_payload}),
    );
}

#[test]
fn happy_path_reaches_decide_cursor() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "implement", json!({"result": "complete"}));
    verify_chain(&journal.events).unwrap();
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.phase.as_deref(), Some("implement"));
    assert!(matches!(state.cursor, Cursor::Decide { .. }));
}

#[test]
fn the_last_successful_triage_result_is_the_engine_strategy() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "triage", json!({"result": "not-a-class"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from":"triage", "result":"not-a-class", "rule_id":"TRIAGE-OLD",
               "next":"triage", "severity":"normal", "inputs":{}, "problem":null}),
    );
    assert_eq!(fold(&journal.events).unwrap().strategy, None);

    effect_cycle(&mut journal, "triage", json!({"result": "feature"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from":"triage", "result":"feature", "rule_id":"TRIAGE-FEATURE",
               "next":"implement", "severity":"normal", "inputs":{}, "problem":null}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.strategy.as_deref(), Some("feature"));
    assert_eq!(
        computed_inputs(&state, "implement", "complete", false).get("strategy"),
        Some(&Value::from("feature"))
    );
}

#[test]
fn consecutive_failures_count_and_reset() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "implement", json!({"result": "broken"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "implement", "result": "broken", "rule_id": "IMPL-BROKEN-RETRY",
               "next": "implement", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.consecutive_failures.get("implement"), Some(&1));
    // The engine-side input counts the failure being decided.
    let inputs = computed_inputs(&state, "implement", "broken", false);
    assert_eq!(inputs.get("consecutive_failures"), Some(&Value::from(2)));

    effect_cycle(&mut journal, "implement", json!({"result": "complete"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "implement", "result": "complete", "rule_id": "IMPL-OK",
               "next": "verify", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.consecutive_failures.get("implement"), Some(&0));
    assert!(computed_inputs(&state, "verify", "fail", false)
        .get("consecutive_failures")
        .is_none());

    effect_cycle(&mut journal, "verify", json!({"result": "fail"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "verify", "result": "fail", "rule_id": "VERIFY-FAIL",
               "next": "verify", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.consecutive_failures.get("verify"), Some(&0));
}

#[test]
fn failed_effect_demands_park_and_operator_retry_reopens() {
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "implement"}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx", "phase": "implement", "seat": "implement",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a1", "driver": "fake"}),
    );
    journal.append(
        EventType::EffectFailed,
        json!({"effect_id": "fx", "attempt_id": "a1", "error": "driver crashed"}),
    );
    let state = fold(&journal.events).unwrap();
    // A determinate failure returns to the executable position with the
    // attempt counted; the engine decides retry-or-park (decision 0006).
    assert!(matches!(
        state.cursor,
        Cursor::ExecuteEffect {
            failed_attempts: 1,
            ..
        }
    ));

    journal.append(
        EventType::RunParked,
        json!({"reason": "effect fx failed", "evidence": {}}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.status, Status::AwaitingOperator);
    assert_eq!(state.cursor, Cursor::Idle);

    journal.append(
        EventType::OperatorCommanded,
        json!({"command_id": "c1", "command": "retry", "args": {}, "operator": "valentin"}),
    );
    journal.append(
        EventType::OperatorAccepted,
        json!({"command_id": "c1", "operator": "valentin", "reason": "fixed env"}),
    );
    let state = fold(&journal.events).unwrap();
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.cursor, Cursor::RequestEffect);
}

#[test]
fn requested_but_unstarted_effect_can_close_indeterminate() {
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "implement"}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx", "phase": "implement", "seat": "implement",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectIndeterminate,
        json!({"effect_id": "fx", "reason": "process died before start was durable"}),
    );
    let state = fold(&journal.events).unwrap();
    assert!(matches!(state.cursor, Cursor::Park { .. }));
}

#[test]
fn protocol_violations_fail_closed() {
    // Second effect while one is open.
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "implement"}));
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx1", "phase": "implement", "seat": "implement",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    journal.append(
        EventType::EffectRequested,
        json!({"effect_id": "fx2", "phase": "implement", "seat": "implement",
               "idempotency_key": "k2", "input_digest": "d"}),
    );
    assert!(matches!(
        fold(&journal.events),
        Err(FoldError::OutOfPlace { seq: 4, .. })
    ));

    // Event after terminal status.
    let mut journal = Journal::new();
    journal.append(EventType::PhaseEntered, json!({"phase": "done"}));
    journal.append(EventType::RunCompleted, json!({}));
    journal.append(EventType::PhaseEntered, json!({"phase": "intake"}));
    assert!(matches!(
        fold(&journal.events),
        Err(FoldError::AfterTerminal { seq: 4 })
    ));
}

#[test]
fn replay_is_deterministic() {
    let mut journal = Journal::new();
    effect_cycle(&mut journal, "implement", json!({"result": "complete"}));
    journal.append(
        EventType::TransitionDecided,
        json!({"from": "implement", "result": "complete", "rule_id": "IMPL-OK",
               "next": "verify", "severity": "normal", "inputs": {}, "problem": null}),
    );
    let a = format!("{:?}", fold(&journal.events).unwrap());
    let b = format!("{:?}", fold(&journal.events).unwrap());
    assert_eq!(a, b);
}

/// Decision 0023's compatibility law, measured against the one journal
/// this repository has committed as evidence: a run recorded before any
/// map existed folds to exactly the state it always folded to, its
/// unkeyed facts still read, and it gained no realm key anywhere.
#[test]
fn the_committed_fixture_journal_folds_exactly_as_it_did() {
    let fixture =
        repository().join("fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");
    let events = committed_journal(&fixture);
    verify_chain(&events).expect("the fixture's chain still verifies");

    let state = fold(&events).expect("and it still folds");
    assert_eq!(state.run_id, "tui-graph-the-selection-box-gets-80f98deb");
    assert_eq!(state.seq, 105);
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.phase.as_deref(), Some("verify"));
    assert_eq!(state.cursor, Cursor::Stop);

    // A journal written before the map gains nothing from it.
    for event in &events {
        let printed = event.payload.to_string();
        for key in ["\"realms\"", "\"realm_facts\""] {
            assert!(!printed.contains(key), "seq {} carries {key}", event.seq);
        }
    }

    // And whatever a decision of that vintage recorded about the
    // repository still reads: the per-realm reader answers with the
    // unkeyed head, whether or not it is given a realm to ask by.
    for event in &events {
        if let Some(recorded) = event.payload.pointer("/inputs/reviewed_heads") {
            let unkeyed = recorded.get("repo").and_then(Value::as_str);
            let heads = ReviewedHeads::read(recorded);
            assert_eq!(heads.head_for(None), unkeyed);
            assert_eq!(heads.head_for(Some("brokkr")), unkeyed);
        }
    }
    let era = json!({"repo": "0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f"});
    assert_eq!(
        ReviewedHeads::read(&era).head_for(Some("brokkr")),
        era["repo"].as_str(),
        "an unkeyed head still answers for the realm the run works in"
    );
}

/// #345 slice 2: the fold keeps a decision, the verdict each phase hands
/// forward and the reviewed heads as types, and each serializes to the
/// bytes the journal holds. Measured over every committed journal, at
/// every decision: the decision round-trips exactly, and the state folded
/// to that decision hands forward exactly the payload it was folded from.
#[test]
fn every_committed_decision_folds_to_its_own_bytes() {
    let mut decisions = 0;
    for (path, events) in committed_journals() {
        let mut heads = None;
        for (at, event) in events.iter().enumerate() {
            heads = ReviewedHeads::recorded_in(&event.payload).or(heads);
            if event.event_type != EventType::TransitionDecided {
                continue;
            }
            let place = format!("{} seq {}", path.display(), event.seq);
            let decision = Decision::deserialize(&event.payload).expect(&place);
            assert_eq!(json!(decision), event.payload, "{place}");
            let state = fold(&events[..=at]).expect(&place);
            assert_eq!(json!(state.last_decision), event.payload, "{place}");
            assert_eq!(
                json!(state.phase_results[&decision.from]),
                json!({"inputs": event.payload["inputs"], "result": decision.result}),
                "{place}"
            );
            assert_eq!(state.reviewed_heads, heads, "{place}");
            decisions += 1;
        }
    }
    assert_eq!(decisions, 68, "every committed decision was measured");
}

/// The repository root, where the committed journals live.
fn repository() -> PathBuf {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    crates.parent().unwrap().to_path_buf()
}

/// Every committed journal, the fixtures and the evidence, in path order.
#[expect(
    clippy::disallowed_methods,
    reason = "the test reads the committed journals"
)]
fn committed_journals() -> Vec<(PathBuf, Vec<EventEnvelope>)> {
    let mut paths: Vec<PathBuf> = ["fixtures/journals", "docs/evidence"]
        .into_iter()
        .flat_map(|dir| std::fs::read_dir(repository().join(dir)).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ndjson")
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let ndjson = std::fs::read_to_string(&path).expect("a committed journal");
            let events = ndjson
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str(line).expect("canonical NDJSON"))
                .collect();
            (path, events)
        })
        .collect()
}

/// One committed journal, by its path.
fn committed_journal(path: &Path) -> Vec<EventEnvelope> {
    let (_, events) = committed_journals()
        .into_iter()
        .find(|(listed, _)| listed == path)
        .expect("a committed journal");
    events
}
