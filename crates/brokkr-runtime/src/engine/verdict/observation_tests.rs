//! Fix round 0109a1r3, H1: every protected effect reaches decide with one
//! durable checked head of its own, or is refused. A sequence's later
//! work step does not spend its gate step's observation, a sequence of
//! work steps is a checked span of its own, a resumed engine decides on
//! the head the journal recorded, and ship sees drift from that head.

use serde_json::{json, Value};

use super::covered_tests::{decided_from, drift_refusing, drift_table};
use super::span_tests::{checkpointed, moved_heads};
use super::tests::{begun, driven_over, driver, ended_in};
use crate::bundle::{Bundle, GateAssessment, SeatBody, SeatClass, SequenceStep};
use crate::engine::heads::HeadReads;
use crate::engine::tests::{git_commit, single_body, step_body};
use crate::engine::verdict::ObservationProblem::{self, Disagrees, Missing};
use crate::engine::Engine;
use brokkr_core::canonical::sha256_bytes;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::policy::Machine;
use brokkr_store::Store;

/// A sequence step of `class` whose driver answers `clean`.
fn clean_step(name: &str, class: SeatClass) -> SequenceStep {
    uncompiled_sequence_step! {
        name: name.into(),
        class,
        results: vec!["clean".into()],
        body: step_body(driver(&json!({"result": "clean"}))),
    }
}

fn completes() -> SeatBody {
    single_body(driver(&json!({"result": "complete"})))
}

/// The bundle shaped under `machine` with review's body `review`, all-gate
/// when `gate`, and a ship that answers `shipped`.
fn reviewed_by(machine: Machine, review: SeatBody, gate: bool) -> impl FnOnce(&mut Bundle) {
    move |compiled: &mut Bundle| {
        compiled.machine = machine;
        let seat = compiled.seats.get_mut("review").unwrap();
        seat.body = review;
        if gate {
            seat.gate = GateAssessment::AllGate;
        }
        let ship = compiled.seats.get_mut("ship").unwrap();
        ship.body = single_body(driver(&json!({"result": "shipped"})));
    }
}

/// Commit a reviewed head and a moved one, pushed onto `heads`, and plan
/// the seam's reads as `plan` over them (0 the reviewed, 1 the moved).
fn planned(repo: &std::path::Path, plan: &[usize], heads: &mut Vec<String>) -> HeadReads {
    heads.push(git_commit(repo, "reviewed"));
    heads.push(git_commit(repo, "moved"));
    let reads: Vec<&str> = plan.iter().map(|index| heads[*index].as_str()).collect();
    HeadReads::scripted(&reads)
}

/// A run whose review is `review` under `machine`, driven with the seam
/// reading `plan` over the reviewed and the moved heads: its events and
/// those two heads.
fn reviewed_over(
    machine: Machine,
    review: SeatBody,
    plan: &[usize],
) -> (Vec<EventEnvelope>, Vec<String>) {
    let mut heads = Vec::new();
    let (_dir, events) = driven_over(completes(), reviewed_by(machine, review, false), |repo| {
        planned(repo, plan, &mut heads)
    });
    (events, heads)
}

/// A protected sequence whose gate step is clean on A and whose later
/// work step is clean too: the effect journals A as its checked head, the
/// gate step's checkpoint journals it beside its covered digest, review
/// records A, and ship takes its drift arm on B, not SHIP-OK.
#[test]
fn a_protected_sequences_later_work_step_does_not_spend_its_gate_steps_observation() {
    let steps = vec![
        clean_step("judge", SeatClass::Gate),
        clean_step("tidy", SeatClass::Work),
    ];
    let review = SeatBody::Sequence { steps };
    let (events, heads) = reviewed_over(drift_refusing(), review, &[0, 0, 0, 1]);
    let (reviewed, moved) = (&heads[0], &heads[1]);
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    assert_eq!(succeeded[0]["result"]["checked_head"], json!(reviewed));
    assert!(succeeded[0]["result"].get("covered").is_none());
    let judged = checkpointed(&events, "judge");
    assert_eq!(judged["checked_head"], json!(reviewed));
    let digest = sha256_bytes(reviewed.as_bytes());
    assert_eq!(
        judged["covered"],
        json!({"kind": "head", "head_digest": digest, "dirty": true})
    );
    let decided = decided_from(&events, "review");
    assert_eq!(
        decided["inputs"]["reviewed_heads"],
        json!({"repo": reviewed})
    );
    let shipped = decided_from(&events, "ship");
    assert_eq!(shipped["rule_id"], "SHIP-DRIFT", "{shipped} on {moved}");
}

/// A protected sequence of work steps only is a checked span of its own:
/// its observation is journaled and recorded, and a move inside it ends
/// indeterminate with both heads, as a gate's does.
#[test]
fn a_protected_sequence_of_work_steps_is_a_checked_span_of_its_own() {
    let review = || SeatBody::Sequence {
        steps: vec![
            clean_step("draft", SeatClass::Work),
            clean_step("tidy", SeatClass::Work),
        ],
    };
    let (events, heads) = reviewed_over(drift_refusing(), review(), &[0, 0, 0, 1]);
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    assert_eq!(succeeded[0]["result"]["checked_head"], json!(heads[0]));
    // Only gate work is stamped `covered` (fix round 0109a1r4, K3).
    assert_eq!(succeeded[0]["result"].get("covered"), None);
    let steps = (events.iter()).filter(|event| event.event_type == EventType::EffectCheckpointed);
    for step in steps {
        assert_eq!(step.payload["checkpoint"]["result"].get("covered"), None);
    }
    let decided = decided_from(&events, "review");
    assert_eq!(
        decided["inputs"]["reviewed_heads"],
        json!({"repo": heads[0]})
    );
    assert_eq!(decided_from(&events, "ship")["rule_id"], "SHIP-DRIFT");

    let (events, heads) = reviewed_over(drift_refusing(), review(), &[0, 0, 1]);
    assert!(ended_in(&events, "review", EventType::EffectSucceeded).is_empty());
    let moved = ended_in(&events, "review", EventType::EffectIndeterminate);
    assert_eq!(moved_heads(&moved), (heads[0].clone(), heads[1].clone()));
}

/// A run stopped once review's success is durable, before its decision.
struct Stopped {
    dir: tempfile::TempDir,
    run_id: String,
    bundle: Bundle,
    reviewed: String,
    moved: String,
}

impl Stopped {
    /// Drive a run turn by turn, with every HEAD read answering the
    /// reviewed head, and stop once review's success, all-gate when
    /// `gate`, is journaled.
    fn after_review(gate: bool) -> Stopped {
        Stopped::after(single_body(driver(&json!({"result": "clean"}))), gate)
    }

    /// [`Stopped::after_review`], with review's body `review`.
    fn after(review: SeatBody, gate: bool) -> Stopped {
        let mut heads = Vec::new();
        let shape = reviewed_by(drift_refusing(), review, gate);
        let (dir, mut engine) = begun(completes(), shape, |repo| planned(repo, &[0], &mut heads));
        let (reviewed, moved) = (heads[0].clone(), heads[1].clone());
        let run_id = engine.run_id.clone();
        let succeeded = |events: &[EventEnvelope]| {
            let kind = |event: &&EventEnvelope| event.event_type == EventType::EffectSucceeded;
            events.iter().filter(kind).count()
        };
        for _ in 0..64 {
            engine.drive_once().unwrap();
            if succeeded(&engine.store.load(&run_id).unwrap()) == 2 {
                break;
            }
        }
        let events = engine.store.load(&run_id).unwrap();
        assert_eq!(succeeded(&events), 2, "work's and review's");
        assert_eq!(
            events.last().unwrap().event_type,
            EventType::EffectSucceeded
        );
        Stopped {
            dir,
            run_id,
            bundle: engine.bundle.clone(),
            reviewed,
            moved,
        }
    }

    /// Replace the journaled success by direct SQL with one whose result
    /// `edit` changed, appended at the same position so the chain holds.
    fn replanted(&self, edit: impl FnOnce(&mut Value)) {
        let db = self.dir.path().join("forge.db");
        let mut store = Store::open(&db).unwrap();
        let last = store.load(&self.run_id).unwrap().pop().unwrap();
        let raw = rusqlite::Connection::open(&db).unwrap();
        raw.execute_batch("DROP TRIGGER events_append_only_delete;")
            .unwrap();
        for table in ["events", "event_index"] {
            let deleted = raw
                .execute(
                    &format!("DELETE FROM {table} WHERE run_id = ?1 AND seq = ?2"),
                    (&self.run_id, last.seq as i64),
                )
                .unwrap();
            assert_eq!(deleted, 1, "{table}");
        }
        drop(raw);
        let mut payload = last.payload;
        edit(&mut payload["result"]);
        let (cause, attempt) = (last.causation_id, last.attempt_id);
        store
            .append_next(
                &self.run_id,
                EventType::EffectSucceeded,
                payload,
                cause,
                attempt,
            )
            .unwrap();
    }

    /// Resume the run on the same bundle in a new engine whose every HEAD
    /// read answers the moved head, and drive it to wherever it stops.
    fn resumed(&self) -> Vec<EventEnvelope> {
        let store = Store::open(&self.dir.path().join("forge.db")).unwrap();
        let repo = Some(self.dir.path().join("work"));
        let mut engine = Engine::resume(store, self.bundle.clone(), &self.run_id, repo).unwrap();
        engine.heads = HeadReads::scripted(&[&self.moved]);
        engine.drive().unwrap();
        engine.store.load(&self.run_id).unwrap()
    }
}

/// Stop after a protected review's durable success on A, move HEAD to B,
/// and resume on the same bundle: a work-class review and a valid gate
/// alike decide on A, deterministically, never parking as schema-invalid,
/// and ship takes its drift arm on B.
#[test]
fn a_resumed_engine_decides_on_the_head_the_journal_recorded() {
    for gate in [false, true] {
        let stopped = Stopped::after_review(gate);
        let events = stopped.resumed();
        let decided = decided_from(&events, "review");
        assert_eq!(decided["rule_id"], "REVIEW", "gate {gate}: {decided}");
        assert_eq!(
            decided["inputs"]["reviewed_heads"],
            json!({"repo": stopped.reviewed})
        );
        let shipped = decided_from(&events, "ship");
        assert_eq!(shipped["rule_id"], "SHIP-DRIFT", "gate {gate}: {shipped}");
        assert_eq!(shipped["inputs"]["drift_detected"], json!(true));
    }
}

/// A protected success whose journaled checked head is removed, and one
/// whose covered digest names another head, each refuse at decide with
/// their own typed problem, record no reviewed head and never reach ship.
#[test]
fn a_missing_or_inconsistent_journaled_observation_refuses_at_decide() {
    let moved_digest = |stopped: &Stopped| json!({"kind": "head", "head_digest": sha256_bytes(stopped.moved.as_bytes()), "dirty": true});
    for (gate, problem) in [(false, Missing), (true, Missing), (true, Disagrees)] {
        let stopped = Stopped::after_review(gate);
        let covered = moved_digest(&stopped);
        stopped.replanted(|result| match problem {
            Missing => drop(result.as_object_mut().unwrap().remove("checked_head")),
            _ => result["covered"] = covered,
        });
        refused_at_decide(&stopped.resumed(), &problem);
    }
}

/// Review's decision in `events` refused `problem`: schema-invalid with
/// its text, no next phase, no reviewed head, and ship never requested.
fn refused_at_decide(events: &[EventEnvelope], problem: &ObservationProblem) {
    let decided = decided_from(events, "review");
    assert_eq!(decided["result"], "__schema-invalid__", "{decided}");
    assert_eq!(decided["next"], Value::Null);
    let text = format!("result failed schema validation: {problem}");
    assert_eq!(decided["problem"], text);
    assert!(decided["inputs"].get("reviewed_heads").is_none());
    assert!(!events.iter().any(|event| {
        event.event_type == EventType::EffectRequested && event.payload["phase"] == "ship"
    }));
}

/// Fix round 0109a1r4, K2: a protected sequence whose gate step was
/// checked on A and whose later work step was clean, stopped once its
/// success is durable. Left as journaled, the resumed engine decides on A.
/// With only the terminal result's checked head replaced by B, the gate
/// step's checkpoint still saying A, decide reads both durable
/// observations, refuses with `Disagrees` and takes no transition.
#[test]
fn decide_refuses_while_two_durable_observations_of_its_attempt_disagree() {
    let review = || SeatBody::Sequence {
        steps: vec![
            clean_step("judge", SeatClass::Gate),
            clean_step("tidy", SeatClass::Work),
        ],
    };
    let agreed = Stopped::after(review(), false);
    let events = agreed.resumed();
    assert_eq!(
        checkpointed(&events, "judge")["checked_head"],
        json!(agreed.reviewed)
    );
    let decided = decided_from(&events, "review");
    assert_eq!(decided["rule_id"], "REVIEW", "{decided}");
    assert_eq!(
        decided["inputs"]["reviewed_heads"],
        json!({"repo": agreed.reviewed})
    );

    let stopped = Stopped::after(review(), false);
    let moved = stopped.moved.clone();
    stopped.replanted(|result| result["checked_head"] = json!(moved));
    let events = stopped.resumed();
    assert_eq!(
        checkpointed(&events, "judge")["checked_head"],
        json!(stopped.reviewed)
    );
    refused_at_decide(&events, &Disagrees);
}

/// A run reviewed twice, on A and then on B: the later review's decision
/// records B and replaces A, so ship on B sees no drift, and no stale
/// head survives it.
#[test]
fn a_later_review_replaces_an_earlier_reviews_head() {
    // Review's first visit sends the run back to work; its second ships.
    let mut table = drift_table();
    table["rules"][1]["when"] = json!({"visits_review_gte": 2});
    let again = json!({"id": "REVIEW-AGAIN", "from": "review", "result": "clean",
                       "next": "work", "reason": "again"});
    table["rules"].as_array_mut().unwrap().insert(2, again);
    let machine = Machine::from_table(&table).unwrap();
    let review = single_body(driver(&json!({"result": "clean"})));
    let (events, heads) = reviewed_over(machine, review, &[0, 0, 0, 1]);
    let reviews: Vec<&Value> = events
        .iter()
        .filter(|event| {
            event.event_type == EventType::TransitionDecided && event.payload["from"] == "review"
        })
        .map(|event| &event.payload)
        .collect();
    assert_eq!(reviews.len(), 2, "{reviews:?}");
    assert_eq!(reviews[0]["rule_id"], "REVIEW-AGAIN");
    assert_eq!(
        reviews[0]["inputs"]["reviewed_heads"],
        json!({"repo": heads[0]})
    );
    assert_eq!(
        reviews[1]["inputs"]["reviewed_heads"],
        json!({"repo": heads[1]})
    );
    let shipped = decided_from(&events, "ship");
    assert_eq!(shipped["rule_id"], "SHIP", "{shipped}");
    assert_eq!(shipped["inputs"]["drift_detected"], json!(false));
}
