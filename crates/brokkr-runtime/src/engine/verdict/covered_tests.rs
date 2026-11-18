//! Decision 0109 ruling 7's `head` arm through the real result path: a
//! gate's success is stamped with what it covered, the protected phase's
//! digest is pinned to the head decided on, and a seat cannot write it.

use std::process::Command;

use serde_json::{json, Value};

use super::tests::{driven, driven_over, driver, ended, ended_in, reported};
use crate::bundle::GateAssessment;
use crate::engine::heads::{HeadReads, Observed};
use crate::engine::tests::{bundle, git_commit, single_body, state};
use crate::engine::{git_head, Engine};
use brokkr_core::canonical::sha256_bytes;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{Cursor, RunState};
use brokkr_core::policy::Machine;
use brokkr_store::Store;

/// Every result the run journaled, in order.
fn results(events: &[EventEnvelope]) -> Vec<&Value> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .map(|event| &event.payload["result"])
        .collect()
}

/// The protected phase deciding an effect whose review gate's stable-head
/// check accepted the head `engine`'s repository stands at now: what a
/// hand-fed decide of review needs to record a reviewed head.
pub(in crate::engine) fn reviewing(engine: &mut Engine) -> RunState {
    let effect_id = "review-effect".to_string();
    engine.observed_head = Some(Observed {
        effect_id: effect_id.clone(),
        head: git_head(&engine.repo),
    });
    let result = Value::Null;
    state(Some("review"), Cursor::Decide { effect_id, result })
}

/// The run's first decision from `phase`.
fn decided_from<'a>(events: &'a [EventEnvelope], phase: &str) -> &'a Value {
    events
        .iter()
        .find(|event| {
            event.event_type == EventType::TransitionDecided && event.payload["from"] == phase
        })
        .map(|event| &event.payload)
        .unwrap()
}

/// The test table with ship's drift arm ahead of its plain one.
fn drift_refusing() -> Machine {
    Machine::from_table(&json!({
        "phases": ["work", "review", "ship", "done", "stop"],
        "initial": "work",
        "terminal": ["done", "stop"],
        "rules": [
            {"id": "WORK", "from": "work", "result": "complete", "next": "review", "reason": "work"},
            {"id": "REVIEW", "from": "review", "result": "clean", "next": "ship", "reason": "review"},
            {"id": "SHIP-DRIFT", "from": "ship", "result": "shipped", "when": {"drift_detected": true},
             "next": "stop", "severity": "hard", "reason": "drift"},
            {"id": "SHIP", "from": "ship", "result": "shipped", "next": "done", "reason": "ship"}
        ]
    }))
    .unwrap()
}

/// A gate outside the protected phase writes the new record: the commit
/// its stable-head check accepted and `dirty: true`, which every record
/// says until 0109b's tree check, even when HEAD reads otherwise on every
/// read after the check; a work effect writes none.
#[test]
fn a_gate_outside_the_protected_phase_records_the_commit_it_read() {
    let result = json!({"result": "complete", "findings": [reported("one", 1)]});
    let mut accepted = String::new();
    let (_dir, events) = driven_over(
        single_body(driver(&result)),
        |compiled| compiled.seats.get_mut("work").unwrap().gate = GateAssessment::AllGate,
        |repo| {
            accepted = git_head(repo).unwrap();
            let moved = git_commit(repo, "moved");
            HeadReads::scripted(&[&accepted, &accepted, &moved])
        },
    );
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert_eq!(
        succeeded[0]["result"]["covered"],
        json!({"kind": "head", "head": accepted, "dirty": true})
    );
    assert_eq!(succeeded[0]["result"]["findings"][0]["id"], "one");
    let (_dir, events) = driven(single_body(driver(&json!({"result": "complete"}))), false);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert!(
        succeeded[0]["result"].get("covered").is_none(),
        "{succeeded:?}"
    );
}

/// Ruling 7's one observation through a driven run whose protected phase
/// holds two commits: HEAD reads `entered` as review is entered, the
/// review gate's span reads `reviewed` at its start and its end, and
/// every read after the check reads `moved`. The covered digest and
/// `reviewed_heads` both name `reviewed`, the head the check accepted,
/// `entered` is its strict ancestor, and ship's drift check refuses the
/// moved head. A work-class review is a checked span too: it records no
/// `covered`, and its `reviewed_heads` is still the head its check
/// accepted, so ship's drift check refuses the moved head there as well
/// (fix round 0109a1r2, verify's return). No path in this slice writes
/// the `artifacts` arm or a follow-up.
#[test]
fn the_reviewed_head_is_the_one_the_stable_head_check_accepted() {
    let mut planned = Vec::new();
    let shape = |gate: bool| {
        move |compiled: &mut crate::bundle::Bundle| {
            compiled.machine = drift_refusing();
            let review = compiled.seats.get_mut("review").unwrap();
            review.body = single_body(driver(&json!({"result": "clean"})));
            if gate {
                review.gate = GateAssessment::AllGate;
            }
            let ship = compiled.seats.get_mut("ship").unwrap();
            ship.body = single_body(driver(&json!({"result": "shipped"})));
        }
    };
    let work = || single_body(driver(&json!({"result": "complete"})));
    let (dir, events) = driven_over(work(), shape(true), |repo| {
        let entered = git_head(repo).unwrap();
        git_commit(repo, "second");
        let reviewed = git_commit(repo, "reviewed");
        let moved = git_commit(repo, "moved");
        planned = vec![entered, reviewed, moved];
        let [entered, reviewed, moved] = [&planned[0], &planned[1], &planned[2]];
        HeadReads::scripted(&[entered, reviewed, reviewed, moved])
    });
    let [entered, reviewed, moved] = [&planned[0], &planned[1], &planned[2]];
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    let digest = sha256_bytes(reviewed.as_bytes());
    assert_eq!(
        succeeded[0]["result"]["covered"],
        json!({"kind": "head", "head_digest": digest, "dirty": true})
    );
    let decided = decided_from(&events, "review");
    assert_eq!(decided["rule_id"], "REVIEW", "{decided}");
    assert_eq!(
        decided["inputs"]["reviewed_heads"],
        json!({"repo": reviewed})
    );
    let phase_entered = events
        .iter()
        .find(|event| {
            event.event_type == EventType::PhaseEntered && event.payload["phase"] == "review"
        })
        .unwrap();
    assert_eq!(phase_entered.payload["head"], json!(entered));
    assert_ne!(entered, reviewed);
    let ancestry = Command::new("git")
        .args(["merge-base", "--is-ancestor", entered, reviewed])
        .current_dir(dir.path().join("work"))
        .status()
        .unwrap();
    assert!(ancestry.success(), "{entered} is an ancestor of {reviewed}");
    let shipped = decided_from(&events, "ship");
    assert_eq!(
        shipped["rule_id"], "SHIP-DRIFT",
        "{shipped} against {moved}"
    );
    assert_eq!(shipped["inputs"]["drift_detected"], json!(true));
    for result in results(&events) {
        assert!(result.get("follow_ups").is_none(), "{result}");
        let kind = result.pointer("/covered/kind");
        assert!(kind.is_none() || kind == Some(&json!("head")), "{result}");
    }
    let mut accepted = String::new();
    let (_dir, events) = driven_over(work(), shape(false), |repo| {
        accepted = git_commit(repo, "reviewed");
        let moved = git_commit(repo, "moved");
        HeadReads::scripted(&[&accepted, &accepted, &accepted, &moved])
    });
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    assert!(
        succeeded[0]["result"].get("covered").is_none(),
        "{succeeded:?}"
    );
    let decided = decided_from(&events, "review");
    assert_eq!(
        decided["inputs"]["reviewed_heads"],
        json!({"repo": accepted})
    );
    assert_eq!(decided_from(&events, "ship")["rule_id"], "SHIP-DRIFT");
}

/// A protected-phase result whose digest does not pin the head its
/// effect's stable-head check accepted is refused at decide with the
/// schema refusal and parks, and so is one decided with no observation
/// of its own effect, as after a restart; the one that pins it is ruled.
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
    let deciding = |effect_id: &str| {
        state(
            Some("review"),
            Cursor::Decide {
                effect_id: effect_id.into(),
                result: Value::Null,
            },
        )
    };
    let observed = |effect_id: &str| {
        Some(Observed {
            effect_id: effect_id.into(),
            head: Some(head.clone()),
        })
    };
    for (raw, observation) in [
        (
            json!({"result": "clean", "covered": covered("0123456789abcdef0123456789abcdef01234567")}),
            observed("review"),
        ),
        (
            json!({"result": "clean", "covered": {"kind": "head", "dirty": true}}),
            observed("review"),
        ),
        (json!({"result": "clean", "covered": covered(&head)}), None),
        (
            json!({"result": "clean", "covered": covered(&head)}),
            observed("another"),
        ),
        (
            json!({"result": "clean", "covered": covered(&head)}),
            observed("review"),
        ),
    ] {
        engine.observed_head = observation;
        engine.decide(&deciding("review"), raw).unwrap();
    }
    let events = engine.store.load(&engine.run_id).unwrap();
    let decided: Vec<&Value> = events
        .iter()
        .filter(|event| event.event_type == EventType::TransitionDecided)
        .map(|event| &event.payload)
        .collect();
    let refusal = "result failed schema validation: the result's covered head does not pin \
                   the head decided on (decision 0109 ruling 7)";
    for refused in &decided[..4] {
        assert_eq!(refused["result"], "__schema-invalid__");
        assert_eq!(refused["problem"], refusal);
    }
    assert_eq!(decided[4]["rule_id"], "REVIEW");
    assert_eq!(decided[4]["inputs"]["reviewed_heads"]["repo"], json!(head));
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
