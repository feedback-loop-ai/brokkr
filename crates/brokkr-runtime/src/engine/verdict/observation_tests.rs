//! Fix round 0109a1r3, H1: every protected effect reaches decide with one
//! durable checked head of its own, or is refused. A sequence's later
//! work step does not spend its gate step's observation, a sequence of
//! work steps is a checked span of its own, a resumed engine decides on
//! the head the journal recorded, and ship sees drift from that head.

use serde_json::{json, Value};

use super::covered_tests::{decided_from, drift_refusing, drift_table};
use super::span_tests::{checkpointed, moved_heads};
use super::tests::{begun, driven_over, driver, ended_in, in_own_engine};
use crate::bundle::{Bundle, GateAssessment, SeatBody, SeatClass, SequenceStep};
use crate::engine::heads::HeadReads;
use crate::engine::tests::{git_commit, single_body, step_body};
use crate::engine::verdict::ObservationProblem::{
    self, Beyond, Disagrees, Missing, Partial, Reordered, Unattributed, Uncovered, Unlawful,
    Unrecorded, Unselected,
};
use crate::engine::verdict::{checked_head, decided_head, Producer};
use crate::engine::Engine;
use brokkr_core::canonical::sha256_bytes;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::Cursor;
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

/// A sequence of the gate step `judge`, then a step `tidy` of `last`.
fn judge_then(last: SeatClass) -> SeatBody {
    SeatBody::Sequence {
        steps: vec![
            clean_step("judge", SeatClass::Gate),
            clean_step("tidy", last),
        ],
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
        let mut edit = Some(edit);
        let succeeded = |event: &EventEnvelope| event.event_type == EventType::EffectSucceeded;
        self.rewritten(succeeded, |event| {
            if let Some(edit) = edit.take().filter(|_| succeeded(event)) {
                edit(&mut event.payload["result"]);
            }
        });
    }

    /// Replace the journal's tail from review's step record named `step`
    /// by direct SQL, each event re-appended at its own position with
    /// `edit` applied, so the chain holds.
    fn replanted_from(&self, step: &str, edit: impl FnMut(&mut EventEnvelope)) {
        self.rewritten(
            |event| {
                event.event_type == EventType::EffectCheckpointed
                    && event.payload["checkpoint"]["step_name"] == step
            },
            edit,
        );
    }

    /// Review's step record `judge` with its result `edited`, and the
    /// terminal's `checked_head` replaced by the moved head, by direct SQL.
    fn judged_then_moved(&self, mut edited: impl FnMut(&mut Value)) {
        let moved = self.moved.clone();
        self.replanted_from("judge", |event| match event.event_type {
            EventType::EffectSucceeded => event.payload["result"]["checked_head"] = json!(moved),
            _ => edited(&mut event.payload["checkpoint"]["result"]),
        });
    }

    /// The journal's tail from the last event `from` picks deleted by
    /// direct SQL and re-appended in order, each with `edit` applied.
    fn rewritten(
        &self,
        from: impl Fn(&EventEnvelope) -> bool,
        mut edit: impl FnMut(&mut EventEnvelope),
    ) {
        self.respliced(from, |mut event| {
            edit(&mut event);
            vec![event]
        });
    }

    /// The journal's tail from the last event `from` picks deleted by
    /// direct SQL and re-appended in order, each replaced by what `splice`
    /// makes of it.
    fn respliced(
        &self,
        from: impl Fn(&EventEnvelope) -> bool,
        mut splice: impl FnMut(EventEnvelope) -> Vec<EventEnvelope>,
    ) {
        let db = self.dir.path().join("forge.db");
        let mut store = Store::open(&db).unwrap();
        let mut events = store.load(&self.run_id).unwrap();
        let at = events.iter().rposition(from).unwrap();
        let tail = events.split_off(at);
        let raw = rusqlite::Connection::open(&db).unwrap();
        raw.execute_batch("DROP TRIGGER events_append_only_delete;")
            .unwrap();
        for table in ["events", "event_index"] {
            let deleted = raw
                .execute(
                    &format!("DELETE FROM {table} WHERE run_id = ?1 AND seq >= ?2"),
                    (&self.run_id, tail[0].seq as i64),
                )
                .unwrap();
            assert_eq!(deleted, tail.len(), "{table}");
        }
        drop(raw);
        for event in tail.into_iter().flat_map(&mut splice) {
            let (cause, attempt) = (event.causation_id, event.attempt_id);
            store
                .append_next(
                    &self.run_id,
                    event.event_type,
                    event.payload,
                    cause,
                    attempt,
                )
                .unwrap();
        }
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

/// Fix round 0109a1r8, M2: a protected single gate decides only with the
/// `covered` digest its dispatch armed. Stopped after its durable success
/// on A, its `covered` removed by direct SQL and the run resumed on B, it
/// refuses with `Uncovered` and takes no transition, though its
/// `checked_head` still reads A.
#[test]
fn a_protected_single_gate_without_its_covered_digest_refuses() {
    let stopped = Stopped::after_review(true);
    stopped.replanted(|result| drop(result.as_object_mut().unwrap().remove("covered")));
    let events = stopped.resumed();
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    assert_eq!(
        succeeded[0]["result"]["checked_head"],
        json!(stopped.reviewed)
    );
    assert!(succeeded[0]["result"].get("covered").is_none());
    refused_at_decide(&events, &Uncovered);
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

/// Fix round 0109a1r5, V1: the terminal agrees with the attempt's latest
/// gate step record on every field, and its own `covered` never stands in
/// for a span. A gate step checked on A, then a work step, stopped once the
/// success is durable:
/// - the terminal's `checked_head` and `covered` both replaced by B and
///   SHA-256(B), the gate step's record still on A;
/// - both `checked_head` values replaced by B, the record's `covered` left
///   on A.
///
/// Each refuses with `Disagrees` on a clean B, and takes no transition.
#[test]
fn a_terminals_own_covered_never_outweighs_its_latest_gate_step_record() {
    let review = || SeatBody::Sequence {
        steps: vec![
            clean_step("judge", SeatClass::Gate),
            clean_step("tidy", SeatClass::Work),
        ],
    };
    let digest = |head: &str| json!({"kind": "head", "head_digest": sha256_bytes(head.as_bytes()), "dirty": true});

    let stopped = Stopped::after(review(), false);
    let moved = stopped.moved.clone();
    stopped.replanted(|result| {
        result["checked_head"] = json!(moved);
        result["covered"] = digest(&moved);
    });
    let events = stopped.resumed();
    assert_eq!(
        checkpointed(&events, "judge")["checked_head"],
        json!(stopped.reviewed)
    );
    refused_at_decide(&events, &Disagrees);

    let stopped = Stopped::after(review(), false);
    let moved = stopped.moved.clone();
    stopped.replanted_from("judge", |event| {
        let result = match event.event_type {
            EventType::EffectSucceeded => &mut event.payload["result"],
            _ => &mut event.payload["checkpoint"]["result"],
        };
        if result.get("checked_head").is_some() {
            result["checked_head"] = json!(moved);
        }
    });
    let events = stopped.resumed();
    let judged = checkpointed(&events, "judge");
    assert_eq!(judged["checked_head"], json!(stopped.moved));
    assert_eq!(judged["covered"], digest(&stopped.reviewed));
    refused_at_decide(&events, &Disagrees);
}

/// Fix round 0109a1r6, W1: an authored step record is whole or it
/// refuses, and the terminal is matched to the step that produced it.
/// - a gate step checked on A whose record keeps its `covered` for A but
///   lost its `checked_head`, then a work step whose terminal names B:
///   `Partial`, never the terminal alone;
/// - a gate step that ends the sequence, whose terminal lost the `covered`
///   its own record carries: `Disagrees`.
///
/// Each takes no transition.
#[test]
fn a_partial_step_record_or_a_terminal_unlike_its_own_gate_step_record_refuses() {
    let review = judge_then;
    let stopped = Stopped::after(review(SeatClass::Work), false);
    stopped.judged_then_moved(|result| {
        drop(result.as_object_mut().unwrap().remove("checked_head"));
    });
    let events = stopped.resumed();
    let judged = checkpointed(&events, "judge");
    assert!(judged.get("checked_head").is_none(), "{judged}");
    assert!(judged.get("covered").is_some(), "{judged}");
    refused_at_decide(&events, &Partial);

    let stopped = Stopped::after(review(SeatClass::Gate), false);
    stopped.replanted(|result| drop(result.as_object_mut().unwrap().remove("covered")));
    let events = stopped.resumed();
    assert!(checkpointed(&events, "tidy").get("covered").is_some());
    refused_at_decide(&events, &Disagrees);
}

/// The four-step body: draft (work), judge (gate), tidy (work), chief
/// (gate), draft ending early on `draft_ends`.
fn four_steps(draft_ends: &[&str]) -> Vec<SequenceStep> {
    let mut draft = clean_step("draft", SeatClass::Work);
    draft.ends_on = draft_ends.iter().map(|word| word.to_string()).collect();
    vec![
        draft,
        clean_step("judge", SeatClass::Gate),
        clean_step("tidy", SeatClass::Work),
        clean_step("chief", SeatClass::Gate),
    ]
}

/// Fix rounds 0109a1r6, W1, 0109a1r7, X1, and 0109a1r8, M1: the producer
/// is read from the compiled body by the step the success names, at a
/// position the body may end on with the terminal's word: its last step,
/// or one whose compiled `ends_on` holds it. Only a single body's success
/// names no step; a sequence's success naming none, a step the body does
/// not hold, a single body's success naming a step, a step the body does
/// not end on with that word, or a body that resolves no case, has no
/// producer.
#[test]
fn the_terminals_producer_is_read_from_the_compiled_body() {
    let dir = tempfile::tempdir().unwrap();
    let body = |draft_ends| SeatBody::Sequence {
        steps: four_steps(draft_ends),
    };
    let compiled = crate::engine::tests::bundle(dir.path(), body(&[]));
    let (sequence, single) = (&compiled.seats["work"], &compiled.seats["review"]);
    let mut gated = single.clone();
    gated.gate = GateAssessment::AllGate;
    let mut early = sequence.clone();
    early.body = body(&["blocked"]);
    let mut unselected = single.clone();
    unselected.body = SeatBody::Select {
        cases: [("other".to_string(), body(&[]))].into(),
        default: None,
    };
    let ended = |step: Option<&str>| {
        let mut state = crate::engine::tests::state(Some("review"), Cursor::RequestEffect);
        state.ending_step = step.map(str::to_string);
        state
    };
    let steps = vec![
        ("draft", SeatClass::Work),
        ("judge", SeatClass::Gate),
        ("tidy", SeatClass::Work),
        ("chief", SeatClass::Gate),
    ];
    let at = |at| {
        Ok(Producer::Step {
            steps: steps.clone(),
            at,
        })
    };
    let unlawful = |step: &str| Err(Unlawful { step: step.into() });
    let cases = [
        (single, None, "clean", Ok(Producer::Effect { gate: false })),
        (&gated, None, "clean", Ok(Producer::Effect { gate: true })),
        (sequence, Some("chief"), "clean", at(3)),
        (sequence, Some("draft"), "clean", unlawful("draft")),
        (sequence, Some("judge"), "clean", unlawful("judge")),
        (sequence, Some("tidy"), "clean", unlawful("tidy")),
        (&early, Some("draft"), "blocked", at(0)),
        (&early, Some("draft"), "clean", unlawful("draft")),
        (sequence, None, "clean", Err(Unattributed)),
        (sequence, Some("other"), "clean", Err(Unattributed)),
        (single, Some("tidy"), "clean", Err(Unattributed)),
        (&unselected, None, "clean", Err(Unselected)),
    ];
    for (seat, step, word, want) in cases {
        let producer = Producer::of(seat, &ended(step), Some(word));
        assert_eq!(producer, want, "{step:?} {word}");
    }
}

/// Fix round 0109a1r8, M1: no row's tag sets what a protected attempt
/// owes. A protected body whose work step precedes its gate step reaches
/// a durable success on A, judge's whole record on A. The success is
/// re-tagged from judge to draft, its `covered` removed and its
/// `checked_head` set to B, by direct SQL, and the run resumed on B:
/// - in the two-step body [draft, judge] and the four-step body [draft,
///   judge, tidy, chief], draft is no position the body ends on with
///   `clean`: `Unlawful`, never B;
/// - where the body resumed on may end early at draft on `clean`, as a
///   lawful early ending journals no record of draft, judge's whole
///   record lies past draft: `Beyond`;
/// - with judge's record removed too, the body holds a gate and the
///   attempt journaled no whole gate record to decide on: `Unrecorded`.
///
/// Each refuses, rather than deciding on judge's A, because the terminal
/// itself says it is not judge's result: a re-tagged terminal is no
/// record the compiled body produces, so no head of it is decided on.
#[test]
fn a_terminal_retagged_to_a_work_step_before_the_gate_refuses() {
    let two = |draft_ends: &[&str]| {
        let mut steps = four_steps(draft_ends);
        steps.truncate(2);
        SeatBody::Sequence { steps }
    };
    let four = || SeatBody::Sequence {
        steps: four_steps(&[]),
    };
    let unlawful = || Unlawful {
        step: "draft".into(),
    };
    let named = |problem: fn(String) -> ObservationProblem| problem("judge".into());
    let all: &[&str] = &["draft", "judge", "tidy", "chief"];
    for (ran, resumed_on, kept, problem) in [
        (two(&[]), two(&[]), all, unlawful()),
        (four(), four(), all, unlawful()),
        (
            two(&[]),
            two(&["clean"]),
            &["judge"][..],
            named(|step| Beyond { step }),
        ),
        (
            two(&[]),
            two(&["clean"]),
            &[][..],
            named(|step| Unrecorded { step }),
        ),
    ] {
        let mut stopped = Stopped::after(ran, false);
        stopped.drafted(kept);
        stopped.bundle.seats.get_mut("review").unwrap().body = resumed_on;
        let events = stopped.resumed();
        let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
        assert_eq!(succeeded[0]["step"], "draft");
        assert_eq!(succeeded[0]["result"]["checked_head"], json!(stopped.moved));
        refused_at_decide(&events, &problem);
    }
}

impl Stopped {
    /// Review's success re-tagged to `draft`, its `covered` removed and its
    /// `checked_head` the moved head, and only the step records `kept`
    /// left of the attempt's, by direct SQL. Every other step record loses
    /// its engine mark, so it is no record of the engine's, and the tail
    /// keeps its length, as the summary row's head needs.
    fn drafted(&self, kept: &[&str]) {
        let moved = self.moved.clone();
        let record = |event: &EventEnvelope| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["checkpoint"]["step"] == "sequence-step-finished"
        };
        let drafts = |event: &EventEnvelope| {
            record(event) && event.payload["checkpoint"]["step_name"] == "draft"
        };
        self.rewritten(drafts, |event| {
            let step = event.payload["checkpoint"]["step_name"].as_str();
            if record(event) && !step.is_some_and(|step| kept.contains(&step)) {
                let checkpoint = event.payload["checkpoint"].as_object_mut().unwrap();
                drop(checkpoint.remove(brokkr_core::findings::AUTHORED));
            }
            if event.event_type == EventType::EffectSucceeded {
                event.payload["step"] = json!("draft");
                let result = &mut event.payload["result"];
                drop(result.as_object_mut().unwrap().remove("covered"));
                result["checked_head"] = json!(moved);
            }
        });
    }
}

/// Fix round 0109a1r7, X1: what a protected attempt owes is read from its
/// compiled body, so a record removed or damaged in the journal never
/// removes the obligation. A gate step checked on A, then a work step,
/// stopped once the success is durable, then resumed on a clean B:
/// - the gate step's record with both `checked_head` and `covered`
///   blanked, the terminal's `checked_head` B: `Unrecorded` for judge;
/// - the gate step's record re-tagged as the final work step, which
///   journals no record of its own: `Beyond` for it (fix round 0109a1r8,
///   M1, reconciles every record before what is owed);
/// - a partial copy of the gate step's record journaled before the
///   complete one: `Partial`, latched.
///
/// A final gate step's success with its step tag and `covered` removed
/// has no producer: `Unattributed`. Each takes no transition.
#[test]
fn an_owed_gate_record_absent_or_malformed_or_an_untagged_success_refuses() {
    let review = judge_then;
    let judge = Unrecorded {
        step: "judge".into(),
    };
    let blank = |result: &mut Value| {
        let result = result.as_object_mut().unwrap();
        result.retain(|field, _| field != "checked_head" && field != "covered");
    };
    let stopped = Stopped::after(review(SeatClass::Work), false);
    stopped.judged_then_moved(blank);
    let events = stopped.resumed();
    assert!(checkpointed(&events, "judge").get("checked_head").is_none());
    refused_at_decide(&events, &judge);

    let stopped = Stopped::after(review(SeatClass::Work), false);
    stopped.replanted_from("judge", |event| {
        if event.event_type == EventType::EffectCheckpointed {
            event.payload["checkpoint"]["step_name"] = json!("tidy");
        }
    });
    let tidy = Beyond {
        step: "tidy".into(),
    };
    refused_at_decide(&stopped.resumed(), &tidy);

    let stopped = Stopped::after(review(SeatClass::Work), false);
    let judged = |event: &EventEnvelope| {
        event.event_type == EventType::EffectCheckpointed
            && event.payload["checkpoint"]["step_name"] == "judge"
    };
    stopped.respliced(judged, |event| {
        if !judged(&event) {
            return vec![event];
        }
        let mut partial = event.clone();
        let result = &mut partial.payload["checkpoint"]["result"];
        drop(result.as_object_mut().unwrap().remove("covered"));
        vec![partial, event]
    });
    let events = stopped.resumed();
    let records = (events.iter()).filter(|event| judged(event));
    let covered: Vec<bool> = records
        .map(|event| {
            event.payload["checkpoint"]["result"]
                .get("covered")
                .is_some()
        })
        .collect();
    assert_eq!(
        covered,
        [false, true],
        "the partial copy, then the whole one"
    );
    refused_at_decide(&events, &Partial);

    let stopped = Stopped::after(review(SeatClass::Gate), false);
    let succeeded = |event: &EventEnvelope| event.event_type == EventType::EffectSucceeded;
    stopped.rewritten(succeeded, |event| {
        if succeeded(event) {
            drop(event.payload.as_object_mut().unwrap().remove("step"));
            drop(
                event.payload["result"]
                    .as_object_mut()
                    .unwrap()
                    .remove("covered"),
            );
        }
    });
    refused_at_decide(&stopped.resumed(), &Unattributed);
}

/// Fix round 0109a1r5, V1: a legitimately newer final gate is the latest
/// record. A gate step checked on A, a work step after which HEAD reads B,
/// and a final gate step checked on B: the final gate step journals its own
/// record on B before the success, and review decides B.
#[test]
fn a_final_gate_step_checked_after_a_moving_work_step_decides_its_own_head() {
    if !in_own_engine() {
        return;
    }
    let steps = vec![
        clean_step("judge", SeatClass::Gate),
        clean_step("tidy", SeatClass::Work),
        clean_step("chief", SeatClass::Gate),
    ];
    let review = SeatBody::Sequence { steps };
    // Review's entry and judge's span read A; tidy leaves B for chief.
    let (events, heads) = reviewed_over(drift_refusing(), review, &[0, 0, 0, 1]);
    let (reviewed, moved) = (&heads[0], &heads[1]);
    assert_eq!(
        checkpointed(&events, "judge")["checked_head"],
        json!(reviewed)
    );
    let chief = checkpointed(&events, "chief");
    assert_eq!(chief["checked_head"], json!(moved));
    let digest = sha256_bytes(moved.as_bytes());
    let covered = json!({"kind": "head", "head_digest": digest, "dirty": true});
    assert_eq!(chief["covered"], covered);
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    assert_eq!(succeeded[0]["result"]["checked_head"], json!(moved));
    assert_eq!(succeeded[0]["result"]["covered"], covered);
    let decided = decided_from(&events, "review");
    assert_eq!(decided["rule_id"], "REVIEW", "{decided}");
    assert_eq!(decided["inputs"]["reviewed_heads"], json!({"repo": moved}));
}

/// Fix round 0109a1r8, M3: a record's journal order agrees with the
/// compiled order of its step. A protected review of three gate steps,
/// judge, aide and chief, each checked on A, stopped once its success is
/// durable. Judge's and aide's whole records swapped by direct SQL, the
/// final chief's record and success left valid: the fold reads the same
/// records by step, and decide refuses `Reordered`, taking no transition.
/// The genuinely newer final gate above still decides its own head.
#[test]
fn gate_records_journaled_out_of_their_compiled_order_refuse() {
    let steps = ["judge", "aide", "chief"].map(|name| clean_step(name, SeatClass::Gate));
    let review = SeatBody::Sequence {
        steps: steps.to_vec(),
    };
    let stopped = Stopped::after(review, false);
    let record = |event: &EventEnvelope, step: &str| {
        event.event_type == EventType::EffectCheckpointed
            && event.payload["checkpoint"]["step"] == "sequence-step-finished"
            && event.payload["checkpoint"]["step_name"] == step
    };
    let mut judge = None;
    stopped.respliced(
        |event| record(event, "judge"),
        |event| {
            if record(&event, "judge") {
                judge = Some(event);
                return vec![];
            }
            if record(&event, "aide") {
                return vec![event, judge.take().unwrap()];
            }
            vec![event]
        },
    );
    let events = stopped.resumed();
    let order: Vec<&str> = (events.iter())
        .filter(|event| record(event, "judge") || record(event, "aide") || record(event, "chief"))
        .map(|event| event.payload["checkpoint"]["step_name"].as_str().unwrap())
        .collect();
    assert_eq!(order, ["aide", "judge", "chief"]);
    for step in ["judge", "aide", "chief"] {
        assert_eq!(
            checkpointed(&events, step)["checked_head"],
            json!(stopped.reviewed)
        );
    }
    refused_at_decide(&events, &Reordered);
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

/// A [draft (work), judge (gate)] body whose draft answers `answer` and may
/// end the sequence on `held`, compiled as a hard word when `hard`.
fn held_early(answer: &str, hard: bool) -> SeatBody {
    let mut draft = clean_step("draft", SeatClass::Work);
    draft.results = vec!["clean".into(), "held".into()];
    draft.body = step_body(driver(&json!({"result": answer})));
    draft.ends_on = vec!["held".into()];
    draft.hard_results = hard.then(|| "held".to_string()).into_iter().collect();
    SeatBody::Sequence {
        steps: vec![draft, clean_step("judge", SeatClass::Gate)],
    }
}

/// #1221, #1212: only a work step before every gate step, ended on a word
/// its compiled `ends_on` holds that is a compiled hard word, is a stop. A
/// non-hard ending there, a hard word outside `ends_on`, a hard ending
/// after a gate step, and one in a body with no gate step are not; a stop
/// is reconciled against its position and decides on no head, and asked
/// for a head it refuses as a work producer before every gate does.
#[test]
fn only_a_compiled_hard_ending_before_every_gate_is_a_stop() {
    let dir = tempfile::tempdir().unwrap();
    let compiled = crate::engine::tests::bundle(dir.path(), completes());
    let seat = |body: SeatBody| {
        let mut seat = compiled.seats["review"].clone();
        seat.body = body;
        seat
    };
    let ending = |name: &str, class, ends: &[&str], hard: &[&str]| SequenceStep {
        ends_on: ends.iter().map(|word| word.to_string()).collect(),
        hard_results: hard.iter().map(|word| word.to_string()).collect(),
        ..clean_step(name, class)
    };
    let (work, gate) = (SeatClass::Work, SeatClass::Gate);
    let sequence = |steps| seat(SeatBody::Sequence { steps });
    let stop = seat(held_early("held", true));
    let soft = seat(held_early("held", false));
    let unended = sequence(vec![
        ending("draft", work, &[], &["held"]),
        clean_step("judge", gate),
    ]);
    let after = sequence(vec![
        clean_step("judge", gate),
        ending("tidy", work, &["held"], &["held"]),
        clean_step("chief", gate),
    ]);
    let ungated = sequence(vec![
        ending("draft", work, &["held"], &["held"]),
        clean_step("tidy", work),
    ]);
    let pair = vec![("draft", work), ("judge", gate)];
    let unlawful = || {
        Err(Unlawful {
            step: "draft".into(),
        })
    };
    let step = |steps: &[(&'static str, SeatClass)], at| Producer::Step {
        steps: steps.to_vec(),
        at,
    };
    let stopped = Ok(Producer::Stop {
        steps: pair.clone(),
        at: 0,
    });
    let after_steps = [("judge", gate), ("tidy", work), ("chief", gate)];
    let cases = [
        (&stop, "draft", "held", stopped),
        (&stop, "draft", "clean", unlawful()),
        (&soft, "draft", "held", Ok(step(&pair, 0))),
        (&unended, "draft", "held", unlawful()),
        (&after, "tidy", "held", Ok(step(&after_steps, 1))),
        (
            &ungated,
            "draft",
            "held",
            Ok(step(&[("draft", work), ("tidy", work)], 0)),
        ),
    ];
    for (seat, step, word, want) in cases {
        let mut state = crate::engine::tests::state(Some("review"), Cursor::RequestEffect);
        state.ending_step = Some(step.into());
        let producer = Producer::of(seat, &state, Some(word));
        assert_eq!(producer, want, "{step} {word}");
    }
}

/// #1221, #1212: a stop decides on no head once every record of its
/// attempt is reconciled against its position, and a latched malformed
/// record or a record past it refuses; asked for a head, it refuses as a
/// work producer before every gate does.
#[test]
fn a_stop_decides_on_no_head_and_is_reconciled_against_its_position() {
    let pair = vec![("draft", SeatClass::Work), ("judge", SeatClass::Gate)];
    let stopped = Producer::Stop { steps: pair, at: 0 };
    let raw = json!({"result": "held", "checked_head": "89abcdef0123456789abcdef0123456789abcdef"});
    let mut records = brokkr_core::fold::StepRecords::default();
    assert_eq!(decided_head(&raw, &records, &stopped), Ok(None));
    let judge = || "judge".to_string();
    assert_eq!(
        checked_head(&raw, &records, &stopped),
        Err(Unrecorded { step: judge() })
    );
    records.malformed = Some(brokkr_core::fold::StepObservation::Partial);
    assert_eq!(decided_head(&raw, &records, &stopped), Err(Partial));
    records.malformed = None;
    let unobserved = brokkr_core::fold::StepObservation::Unobserved;
    records.by_step.insert(judge(), unobserved);
    assert_eq!(
        decided_head(&raw, &records, &stopped),
        Err(Beyond { step: judge() })
    );
}

/// #1221, #1212 through the real engine: a protected [draft (work), judge
/// (gate)] body whose draft ends the sequence on `held`, a word that does
/// not stop the run, journals no gate record and no checked head, so decide
/// refuses it and the run parks: judge never ran and ship is never asked.
#[test]
fn a_protected_pre_gate_ending_that_does_not_stop_the_run_refuses_and_parks() {
    let shape = reviewed_by(drift_refusing(), held_early("held", false), false);
    let (_dir, events) = driven_over(
        completes(),
        |compiled| {
            shape(compiled);
            let review = compiled.seats.get_mut("review").unwrap();
            review.results.push("held".into());
        },
        |_| HeadReads::default(),
    );
    let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
    assert_eq!(succeeded[0]["step"], "draft");
    assert_eq!(succeeded[0]["result"].get("checked_head"), None);
    refused_at_decide(&events, &Missing);
    assert!(!(events.iter()).any(|event| event.payload["checkpoint"]["step_name"] == "judge"));
    assert_eq!(events.last().unwrap().event_type, EventType::RunParked);
}

/// #1221, #1212: where draft's `held` is a compiled hard ending, judge's
/// success re-tagged to draft by direct SQL on `clean`, a word outside
/// draft's `ends_on`, still refuses `Unlawful`; re-worded to the hard
/// `held` with judge's record kept, it is no stop, since that record lies
/// past draft: `Beyond`.
#[test]
fn a_success_retagged_to_a_hard_ending_work_step_still_refuses() {
    if !in_own_engine() {
        return;
    }
    for (worded, kept, problem) in [
        (
            None,
            &["judge"][..],
            Unlawful {
                step: "draft".into(),
            },
        ),
        (
            Some("held"),
            &["judge"][..],
            Beyond {
                step: "judge".into(),
            },
        ),
    ] {
        let mut stopped = Stopped::after(held_early("clean", true), false);
        stopped.drafted(kept);
        if let Some(word) = worded {
            stopped.replanted(|result| result["result"] = json!(word));
        }
        let review = stopped.bundle.seats.get_mut("review").unwrap();
        review.results.push("held".into());
        let events = stopped.resumed();
        let succeeded = ended_in(&events, "review", EventType::EffectSucceeded);
        assert_eq!(succeeded[0]["step"], "draft");
        refused_at_decide(&events, &problem);
    }
}
