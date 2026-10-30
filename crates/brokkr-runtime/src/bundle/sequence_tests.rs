//! Decision 0050 ruling 7: a sequence's endings are compiled from the
//! step vocabularies and the composed table, never compared at run time,
//! and an arm no step can reach is refused at compile.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::sequence::hard_results;
use super::{Bundle, CompileError, SeatBody, SequenceStep, StepBody};
use crate::bundle::Invocation;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn shipped_bundles() -> Vec<(String, Bundle)> {
    let root = workspace();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("recipes"))
        .expect("recipes/ is readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("bundle.json").is_file())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .map(|dir| {
            let label = format!(
                "recipes/{}",
                dir.file_name().expect("a recipe name").to_string_lossy()
            );
            let bundle = Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters"))
                .unwrap_or_else(|error| panic!("{label} compiles: {error}"));
            (label, bundle)
        })
        .collect()
}

/// Every sequence a bundle can select, with the seat's vocabulary.
fn sequences(bundle: &Bundle) -> Vec<(String, &[String], &[SequenceStep])> {
    let mut out = Vec::new();
    for (phase, seat) in &bundle.seats {
        collect(phase, &seat.body, &seat.results, &mut out);
    }
    out
}

/// Every compiled ending in a bundle, in seat and step order.
fn endings(bundle: &Bundle) -> Vec<String> {
    sequences(bundle)
        .into_iter()
        .flat_map(|(_, _, steps)| {
            steps
                .iter()
                .flat_map(|step| step.ends_on.iter().cloned())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn collect<'a>(
    phase: &str,
    body: &'a SeatBody,
    seat_results: &'a [String],
    out: &mut Vec<(String, &'a [String], &'a [SequenceStep])>,
) {
    match body {
        SeatBody::Invocation(_) => {}
        SeatBody::Sequence { steps } => out.push((phase.to_string(), seat_results, steps)),
        SeatBody::Select { cases, default } => {
            for case in cases.values() {
                collect(phase, case, seat_results, out);
            }
            if let Some(default) = default.as_deref() {
                collect(phase, default, seat_results, out);
            }
        }
    }
}

/// The words the old run-time rule ended on: the seat's vocabulary, minus
/// what a later step can emit.
fn run_time_endings(
    step: &SequenceStep,
    later: &[SequenceStep],
    seat_results: &[String],
) -> BTreeSet<String> {
    let later: BTreeSet<&str> = later
        .iter()
        .flat_map(|later| later.results.iter().map(String::as_str))
        .collect();
    step.results
        .iter()
        .filter(|word| seat_results.contains(word) && !later.contains(word.as_str()))
        .cloned()
        .collect()
}

/// The compiled set is the old rule's set plus the phase's hard words.
/// Each difference is named so a moved set says which word moved.
#[test]
fn every_shipped_sequence_compiles_its_run_time_endings_plus_the_hard_words() {
    for (label, bundle) in shipped_bundles() {
        for (phase, seat_results, steps) in sequences(&bundle) {
            let hard = hard_results(&bundle.machine, &phase);
            for (index, step) in steps.iter().enumerate() {
                if index + 1 == steps.len() {
                    assert!(
                        step.ends_on.is_empty() && step.hard_results.is_empty(),
                        "{label} {phase}:{} is final and carries no ending",
                        step.name
                    );
                    continue;
                }
                let mut expected = run_time_endings(step, &steps[index + 1..], seat_results);
                let hard_here: BTreeSet<String> = step
                    .results
                    .iter()
                    .filter(|word| hard.contains(*word))
                    .cloned()
                    .collect();
                expected.extend(hard_here.iter().cloned());
                let compiled: BTreeSet<String> = step.ends_on.iter().cloned().collect();
                assert_eq!(
                    compiled,
                    expected,
                    "{label} {phase}:{} compiled {:?}, expected {:?} (old rule {:?}, hard {:?})",
                    step.name,
                    step.ends_on,
                    expected,
                    run_time_endings(step, &steps[index + 1..], seat_results),
                    hard_here,
                );
                let compiled_hard: BTreeSet<String> = step.hard_results.iter().cloned().collect();
                assert_eq!(
                    compiled_hard, hard_here,
                    "{label} {phase}:{} names its hard endings",
                    step.name
                );
            }
        }
    }
}

/// `ends_on` is derived from the recipe and the composed table, so it is
/// absent from the manifest and a recompile (the resume path) reproduces
/// it rather than reading it back.
#[test]
fn a_manifest_without_endings_recompiles_to_the_same_compiled_endings() {
    let root = workspace();
    let first = Bundle::compile_with(
        &root.join("recipes/triage"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("triage compiles");
    let written = first.manifest.to_string();
    assert!(
        !written.contains("ends_on") && !written.contains("hard_results"),
        "the endings are compiled facts, never serialized into the manifest"
    );
    let first_endings = endings(&first);
    assert!(
        first_endings.contains(&"security-hold".to_string()),
        "the triage review sequence ends on the panel's hard word"
    );

    let again = Bundle::compile_with(
        &root.join("recipes/triage"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("triage compiles again");
    let again_endings = endings(&again);
    assert_eq!(
        first_endings, again_endings,
        "a resume recompiles the bundle and derives the same endings"
    );
}

fn fixture(policy: Value, seat: Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a fixture directory");
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).expect("roles/");
    std::fs::write(bundle.join("roles/role.md"), "# role\n").expect("role");
    std::fs::write(
        bundle.join("policy.json"),
        serde_json::to_string(&policy).expect("policy json"),
    )
    .expect("policy");
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_string(&json!({
            "name": "sequence-endings",
            "policy": "policy.json",
            "seats": {
                "work": {"results": ["complete"],
                         "role": "roles/role.md",
                         "driver": {"command": ["driver"]}},
                "review": seat,
            },
        }))
        .expect("bundle json"),
    )
    .expect("bundle");
    dir
}

fn policy(rules: Value) -> Value {
    json!({
        "schema": "forge.phase-machine/v1",
        "phases": ["work", "review", "done", "stop"],
        "initial": "work",
        "terminal": ["done", "stop"],
        "rules": std::iter::once(json!({
            "id": "WORK-ENTER", "from": "work", "result": "complete",
            "next": "review", "reason": "enter review"
        }))
        .chain(rules.as_array().unwrap().iter().cloned())
        .collect::<Vec<_>>(),
    })
}

fn sequence_seat(results: Value) -> Value {
    json!({
        "results": results,
        "sequence": [
            {"name": "first", "results": ["a", "hold"],
             "role": "roles/role.md", "driver": {"command": ["driver"]}},
            {"name": "last",
             "role": "roles/role.md", "driver": {"command": ["driver"]}},
        ],
    })
}

fn compile(dir: &Path) -> Result<Bundle, CompileError> {
    let root = workspace();
    Bundle::compile_with(
        &dir.join("bundle"),
        &root.join("agents"),
        &root.join("adapters"),
    )
}

/// The reverse of the coverage rule: an arm whose word no step of the
/// sequence can emit is named at compile, with a valid counterpart that
/// compiles.
#[test]
fn an_arm_no_step_can_emit_is_refused_naming_the_rule_and_the_word() {
    let rules = |ghost: bool| {
        let mut rules = vec![
            json!({"id": "WORK-A", "from": "review", "result": "a", "next": "done",
                   "reason": "a"}),
            json!({"id": "WORK-B", "from": "review", "result": "b", "next": "done",
                   "reason": "b"}),
            json!({"id": "WORK-HOLD", "from": "review", "result": "hold", "next": "stop",
                   "severity": "hard", "reason": "hold"}),
        ];
        if ghost {
            rules.push(
                json!({"id": "WORK-GHOST", "from": "review", "result": "ghost",
                              "next": "done", "reason": "no step emits this"}),
            );
        }
        json!(rules)
    };
    let seat = sequence_seat(json!(["a", "b", "hold"]));

    let refused = fixture(policy(rules(true)), seat.clone());
    let refusal = compile(refused.path()).expect_err("the ghost arm is refused");
    let text = refusal.to_string();
    assert!(text.contains("WORK-GHOST"), "{text}");
    assert!(text.contains("'ghost'"), "{text}");
    assert!(text.contains("no step"), "{text}");

    let valid = fixture(policy(rules(false)), seat);
    let compiled = compile(valid.path()).expect("the reachable table compiles");
    let steps = match &compiled.seats["review"].body {
        SeatBody::Sequence { steps } => steps,
        other => panic!("the fixture is a sequence: {other:?}"),
    };
    assert_eq!(
        steps[0].ends_on,
        vec!["hold".to_string()],
        "the hard word ends the first step; `a` is not used because the final step emits it"
    );
    assert!(
        matches!(
            steps[1].body,
            StepBody::Invocation(Invocation::Single { .. })
        ),
        "the final step is the seat boundary"
    );
}
