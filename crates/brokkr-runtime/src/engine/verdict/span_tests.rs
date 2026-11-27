//! Fix round 0109a1r2, R2: every gate-produced result is stamped from
//! the span of the gate step that produced it, inside a sequence too,
//! where the seat as a whole is not all-gate. A dialect check is a gate
//! by its compiled class (`bundle::sequence::step_class`), whatever its
//! recipe wrote.

use std::os::unix::fs::MetadataExt;

use serde_json::{json, Value};

use super::tests::{begun, driven_over, driver, ended};
use crate::argv::Executable;
use crate::bundle::{
    Bundle, DialectExecution, GateAssessment, SeatBody, SeatClass, SequenceStep, StepBody,
};
use crate::engine::capability_tests::provision_tests::survey_tests::installed;
use crate::engine::git_head;
use crate::engine::heads::HeadReads;
use crate::engine::tests::{git_commit, own_engine, single_body, step_body};
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::ParkReason;
use brokkr_core::gate_moved_head::{self, MovedHead};
use brokkr_protocol::process::DriverStderr;

/// A sequence step of `class` and `results` over `body`.
fn step(name: &str, class: SeatClass, results: &[&str], body: StepBody) -> SequenceStep {
    uncompiled_sequence_step! {
        name: name.into(),
        class,
        results: results.iter().map(|word| word.to_string()).collect(),
        body,
    }
}

/// A dialect check, compiled as the compiler compiles every one: a gate.
fn dialect(name: &str, results: &[&str]) -> SequenceStep {
    let execution = DialectExecution {
        argv: vec!["check".into()],
        state: None,
    };
    step(
        name,
        SeatClass::Gate,
        results,
        StepBody::Dialect { execution },
    )
}

const DIALECT_EXEC: &str = "dialect-exec";

/// The `work` seat as a sequence of `steps` reporting `results`, whose
/// dialect steps the engine runs through an executable that answers
/// with `dialect_result`, as `driver exec` would.
fn shaped(
    steps: Vec<SequenceStep>,
    results: &[&str],
    dialect_result: &str,
) -> impl FnOnce(&mut Bundle) {
    let results: Vec<String> = results.iter().map(|word| word.to_string()).collect();
    let answer = driver(&json!({"result": dialect_result}));
    move |compiled: &mut Bundle| {
        let source = compiled.dir.join("dialect-exec.source");
        std::fs::write(&source, format!("#!/bin/sh\n{}", answer[2])).unwrap();
        compiled.executable = Executable(Some(installed(&compiled.dir, DIALECT_EXEC, &source)));
        let work = compiled.seats.get_mut("work").unwrap();
        work.body = SeatBody::Sequence { steps };
        work.results = results;
    }
}

/// Commit `count` commits on top of `repo`'s base and plan the seam's
/// reads as `plan` over the base (0) and those commits (1..=count).
fn planned(
    repo: &std::path::Path,
    count: usize,
    plan: &[usize],
    heads: &mut Vec<String>,
) -> HeadReads {
    heads.push(git_head(repo).unwrap());
    for index in 1..=count {
        heads.push(git_commit(repo, &format!("commit-{index}")));
    }
    let reads: Vec<&str> = plan.iter().map(|index| heads[*index].as_str()).collect();
    HeadReads::scripted(&reads)
}

/// The result each sequence step's checkpoint journaled, by step name.
pub(super) fn checkpointed<'a>(events: &'a [EventEnvelope], name: &str) -> &'a Value {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| &event.payload["checkpoint"])
        .find(|checkpoint| checkpoint["step_name"] == name)
        .map(|checkpoint| &checkpoint["result"])
        .unwrap()
}

fn head(commit: &str) -> Value {
    json!({"kind": "head", "head": commit, "dirty": true})
}

/// Triage's `specify` driven over two commits with the seam reading
/// `plan`: a work author, then the dialect validator. The run's events
/// and the heads the plan indexes.
fn specify(plan: &[usize]) -> (Vec<EventEnvelope>, Vec<String>) {
    let author = step(
        "author",
        SeatClass::Work,
        &["drafted"],
        step_body(driver(&json!({"result": "drafted"}))),
    );
    let steps = vec![author, dialect("validate", &["drafted", "fail"])];
    let mut heads = Vec::new();
    let (_dir, events) = driven_over(
        SeatBody::Sequence { steps: Vec::new() },
        shaped(steps, &["drafted", "fail"], "drafted"),
        |repo| planned(repo, 2, plan, &mut heads),
    );
    (events, heads)
}

/// Triage's `specify`: the seat is not all-gate, and its result is the
/// validator's, stamped with the head the validator's own span accepted;
/// the author's checkpoint, a work step's, carries none.
#[test]
fn a_work_then_gate_sequence_stamps_the_gate_steps_span() {
    own_engine!();
    let (events, heads) = specify(&[1, 1, 2]);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert_eq!(
        succeeded[0]["result"]["covered"],
        head(&heads[1]),
        "{succeeded:?}"
    );
    assert!(checkpointed(&events, "author").get("covered").is_none());
}

/// The dialect shim runs while a writer is still open on its previous
/// inode. On Linux this exercises the same kernel check as a sibling
/// fork keeping an inherited writer before exec (#255).
#[test]
fn the_dialect_fixture_replaces_an_inode_with_an_open_writer() {
    own_engine!();
    let shape = shaped(
        vec![dialect("check", &["complete"])],
        &["complete"],
        "complete",
    );
    let mut writer = None;
    let (dir, events) = driven_over(
        SeatBody::Sequence { steps: Vec::new() },
        |compiled| {
            writer = Some(std::fs::File::create(compiled.dir.join(DIALECT_EXEC)).unwrap());
            shape(compiled);
        },
        |_| HeadReads::default(),
    );
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert_eq!(
        succeeded.len(),
        1,
        "the work effect succeeds with its previous inode still open for writing: {:?}",
        ended(&events, EventType::EffectFailed)
    );
    assert_eq!(succeeded[0]["result"]["result"], "complete");
    let previous = writer.unwrap().metadata().unwrap();
    let current = std::fs::metadata(dir.path().join(DIALECT_EXEC)).unwrap();
    assert_ne!(previous.ino(), current.ino());
}

/// Triage's `analyze`: an unclassed dialect check, a gate by its compiled
/// class, then the gate judge. Each gate step's result is stamped from
/// its own span: the check's checkpoint with the head its span accepted,
/// the judge's result, the effect's, with the later head its span did.
#[test]
fn an_analyze_sequence_stamps_each_gate_step_from_its_own_span() {
    own_engine!();
    let judge = step(
        "judge",
        SeatClass::Gate,
        &["consistent", "drift"],
        step_body(driver(&json!({"result": "consistent"}))),
    );
    let steps = vec![dialect("check", &["consistent", "drift"]), judge];
    let mut heads = Vec::new();
    let (_dir, events) = driven_over(
        SeatBody::Sequence { steps: Vec::new() },
        shaped(steps, &["consistent", "drift"], "consistent"),
        |repo| planned(repo, 3, &[1, 1, 2, 2, 3], &mut heads),
    );
    assert_eq!(checkpointed(&events, "check")["covered"], head(&heads[1]));
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert_eq!(
        succeeded[0]["result"]["covered"],
        head(&heads[2]),
        "{succeeded:?}"
    );
}

/// A gate step that moved HEAD inside its own span ends the attempt
/// indeterminate with both heads as its reason, as an all-gate body's
/// effect does, and nothing is stamped or journaled as succeeded.
#[test]
fn a_gate_step_that_moved_head_in_its_span_refuses_as_an_all_gate_body_does() {
    own_engine!();
    let (events, heads) = specify(&[1, 2]);
    assert!(ended(&events, EventType::EffectSucceeded).is_empty());
    let moved = ended(&events, EventType::EffectIndeterminate);
    assert_eq!(moved_heads(&moved), (heads[1].clone(), heads[2].clone()));
}

#[test]
fn a_gate_that_moves_head_names_its_stderr_and_parks_with_both_heads() {
    own_engine!();
    const SAID: &str = "unbound gate diagnostic\n";
    let mut command = driver(&json!({"result": "complete"}));
    command[2] = format!(
        "git commit -q --allow-empty -m gate-moved-head || exit 1\nprintf '%s' '{SAID}' >&2\n{}",
        command[2]
    );
    let (_dir, mut engine) = begun(
        single_body(command),
        |bundle| bundle.seats.get_mut("work").unwrap().gate = GateAssessment::AllGate,
        |_| HeadReads::default(),
    );
    let start = git_head(&engine.repo).unwrap();
    let end = engine.drive().unwrap();
    let head = git_head(&engine.repo).unwrap();
    assert_ne!(start, head);
    assert_eq!(
        end.state.run.park_reason(),
        Some(&ParkReason::GateMovedHead)
    );

    let events = engine.store.load(&engine.run_id).unwrap();
    let heads = json!({"head_at_end": head, "head_at_start": start});
    let digest = &brokkr_core::canonical::sha256_bytes(SAID.as_bytes())[..16];
    let expected_reason = format!(
        "{} {heads}; driver stderr: {} bytes (first byte: letter; sha256: {digest})",
        gate_moved_head::PREFIX,
        SAID.len()
    );
    let indeterminate = ended(&events, EventType::EffectIndeterminate);
    assert_eq!(indeterminate.len(), 1);
    assert_eq!(indeterminate[0]["reason"], expected_reason);
    let parked = events
        .iter()
        .find(|event| event.event_type == EventType::RunParked)
        .unwrap();
    assert_eq!(parked.payload["reason"], gate_moved_head::PREFIX);
    assert_eq!(parked.payload["evidence"], heads);
}

#[test]
fn the_moved_head_decoder_reads_the_driver_stderr_namers_output() {
    let moved = MovedHead {
        head_at_end: Some("bbb".into()),
        head_at_start: Some("aaa".into()),
    };
    let reason = DriverStderr::of("gate diagnostic").named(&gate_moved_head::encode(&moved));
    assert_eq!(
        gate_moved_head::decode::<MovedHead>(&reason).and_then(Result::ok),
        Some(moved)
    );
}

/// The start and end heads the first of `indeterminate` ends recorded as
/// its moved-head reason.
pub(super) fn moved_heads(indeterminate: &[Value]) -> (String, String) {
    let reason = indeterminate[0]["reason"].as_str().unwrap();
    let decoded = gate_moved_head::decode::<Value>(reason).unwrap().unwrap();
    let head = |end: &str| decoded[end].as_str().unwrap().to_string();
    (head("head_at_start"), head("head_at_end"))
}
