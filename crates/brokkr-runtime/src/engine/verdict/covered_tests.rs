//! Decision 0109 ruling 7's `head` arm through the real result path: a
//! gate's success is stamped with what it covered, the protected phase's
//! digest is pinned to the head decided on, and a seat cannot write it.

use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

use super::tests::{driven, driven_with, driver, ended, ended_in, reported};
use crate::bundle::GateAssessment;
use crate::engine::tests::{bundle, git_commit, single_body, state};
use crate::engine::{git_head, Engine};
use brokkr_core::canonical::sha256_bytes;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::Cursor;
use brokkr_store::Store;

fn head_of(dir: &Path) -> String {
    git_head(&dir.join("work")).unwrap()
}

/// Every result the run journaled, in order.
fn results(events: &[EventEnvelope]) -> Vec<&Value> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .map(|event| &event.payload["result"])
        .collect()
}

/// A gate outside the protected phase writes the new record: the commit
/// it read and `dirty: true`, which every record says until 0109b's tree
/// check; a work effect writes none.
#[test]
fn a_gate_outside_the_protected_phase_records_the_commit_it_read() {
    let result = json!({"result": "complete", "findings": [reported("one", 1)]});
    let (dir, events) = driven(single_body(driver(&result)), true);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert_eq!(
        succeeded[0]["result"]["covered"],
        json!({"kind": "head", "head": head_of(dir.path()), "dirty": true})
    );
    assert_eq!(succeeded[0]["result"]["findings"][0]["id"], "one");
    let (_dir, events) = driven(single_body(driver(&result)), false);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert!(
        succeeded[0]["result"].get("covered").is_none(),
        "{succeeded:?}"
    );
}

/// The protected phase writes the digest field set: the SHA-256 of the
/// head its decision records as `reviewed_heads`, whose `phase/entered`
/// head is an ancestor. No path in this slice writes the `artifacts` arm
/// or a follow-up.
#[test]
fn the_protected_phase_pins_its_covered_digest_to_the_reviewed_head() {
    let work = single_body(driver(&json!({"result": "complete"})));
    let review = single_body(driver(&json!({"result": "clean"})));
    let (dir, events) = driven_with(work, |compiled| {
        let seat = compiled.seats.get_mut("review").unwrap();
        seat.body = review;
        seat.gate = GateAssessment::AllGate;
    });
    let head = head_of(dir.path());
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    let digest = sha256_bytes(head.as_bytes());
    assert_eq!(
        succeeded[0]["result"]["covered"],
        json!({"kind": "head", "head_digest": digest, "dirty": true})
    );
    let decided = events
        .iter()
        .find(|event| {
            event.event_type == EventType::TransitionDecided && event.payload["from"] == "review"
        })
        .unwrap();
    assert_eq!(decided.payload["rule_id"], "REVIEW", "{}", decided.payload);
    let reviewed = decided.payload["inputs"]["reviewed_heads"]["repo"]
        .as_str()
        .unwrap();
    assert_eq!(reviewed, head);
    assert_eq!(sha256_bytes(reviewed.as_bytes()), digest);
    let entered = events
        .iter()
        .find(|event| {
            event.event_type == EventType::PhaseEntered && event.payload["phase"] == "review"
        })
        .unwrap();
    let entered = entered.payload["head"].as_str().unwrap();
    let ancestry = Command::new("git")
        .args(["merge-base", "--is-ancestor", entered, reviewed])
        .current_dir(dir.path().join("work"))
        .status()
        .unwrap();
    assert!(ancestry.success(), "{entered} is an ancestor of {reviewed}");
    for result in results(&events) {
        assert!(result.get("follow_ups").is_none(), "{result}");
        let kind = result.pointer("/covered/kind");
        assert!(kind.is_none() || kind == Some(&json!("head")), "{result}");
    }
}

/// A protected-phase result whose digest does not pin the decide-time
/// head is refused at decide with the schema refusal and parks; the one
/// that pins it is ruled as before.
#[test]
fn a_covered_digest_that_does_not_pin_the_decide_time_head_refuses_at_decide() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let head = git_commit(&repo, "reviewed");
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let compiled = bundle(dir.path(), single_body(vec!["driver".into()]));
    let mut engine = Engine::start(store, compiled, "pinned", Some(repo.clone())).unwrap();
    let covered = |head: &str| json!({"kind": "head", "head_digest": sha256_bytes(head.as_bytes()), "dirty": true});
    let review = state(Some("review"), Cursor::RequestEffect);
    for raw in [
        json!({"result": "clean", "covered": covered("0123456789abcdef0123456789abcdef01234567")}),
        json!({"result": "clean", "covered": {"kind": "head", "dirty": true}}),
        json!({"result": "clean", "covered": covered(&head)}),
    ] {
        engine.decide(&review, raw).unwrap();
    }
    let events = engine.store.load(&engine.run_id).unwrap();
    let decided: Vec<&Value> = events
        .iter()
        .filter(|event| event.event_type == EventType::TransitionDecided)
        .map(|event| &event.payload)
        .collect();
    let refusal = "result failed schema validation: the result's covered head does not pin \
                   the head decided on (decision 0109 ruling 7)";
    for refused in &decided[..2] {
        assert_eq!(refused["result"], "__schema-invalid__");
        assert_eq!(refused["problem"], refusal);
    }
    assert_eq!(decided[2]["rule_id"], "REVIEW");
    assert_eq!(decided[2]["inputs"]["reviewed_heads"]["repo"], json!(head));
}

/// Ruling 1 through the real result path: a seat-supplied `covered` or
/// `follow_ups` fails the attempt with the refusal named and is never
/// journaled, even from a gate whose success the engine would stamp.
#[test]
fn a_seat_supplied_covered_or_follow_ups_fails_the_attempt() {
    let head = json!({"kind": "head", "head": "0".repeat(40), "dirty": false});
    for (field, value) in [("covered", head), ("follow_ups", json!([]))] {
        let mut result = json!({"result": "complete"});
        result[field] = value;
        let (_dir, events) = driven(single_body(driver(&result)), true);
        assert!(
            ended(&events, EventType::EffectSucceeded).is_empty(),
            "{field}"
        );
        let failed = ended(&events, EventType::EffectFailed);
        assert_eq!(failed.len(), 2, "{field}");
        let want = format!(
            "the result was refused: the result carries '{field}', which only the engine \
             writes (decision 0109 ruling 1); stderr tail: "
        );
        for failure in failed {
            assert!(
                failure["error"].as_str().unwrap().starts_with(&want),
                "{failure}"
            );
        }
    }
}
