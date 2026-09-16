//! Decision 0042's SDD table and dialect composition.

use std::path::{Path, PathBuf};

use brokkr_runtime::bundle::{SeatBody, StepBody};
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::Bundle;
use serde_json::json;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn compile(dialect: Option<&Dialect>) -> Result<Bundle, brokkr_runtime::CompileError> {
    let root = root();
    Bundle::compile_with_realm(
        &root.join("recipes/triage"),
        &root.join("agents"),
        &root.join("adapters"),
        Some("brokkr"),
        dialect,
        brokkr_core::realms::Boundary::Namespace,
    )
}

#[test]
fn artifact_work_refuses_the_bootstrap_realm_and_accepts_a_declared_dialect() {
    let message = compile(None).unwrap_err().to_string();
    assert!(message.contains("realm 'brokkr'"), "{message}");
    assert!(message.contains("phase 'specify' needs one"), "{message}");

    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    assert!(compile(Some(&dialect)).is_ok());

    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root().join("dialects/openspec.json")).unwrap())
            .unwrap();
    value["phases"]["specify"]["steps"][0]["instructions"] = json!("openspec/missing.md");
    let missing = Dialect::parse("missing-instructions.json", &value.to_string())
        .unwrap()
        .0;
    let message = compile(Some(&missing)).unwrap_err().to_string();
    assert!(message.contains("pin carries no rendered instructions for phase 'specify'"));
}

#[test]
fn the_implement_prompt_carries_the_dialects_archive_instruction() {
    // Decision 0042's addendum of 2026-09-06: the archive step is the
    // step that records which change wrote which capability, so the
    // dialect that promotes a truth tree renders its instruction to the
    // smith who folds the change.
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = compile(Some(&dialect)).unwrap();
    let prompt = &bundle.dialect_prompts["implement"];
    assert!(prompt.contains("## Provenance"), "{prompt}");
    assert!(
        prompt.contains("never rewrite, reorder or remove"),
        "{prompt}"
    );
}

/// Proposed decision 0056 ruling 10: the SDD smith's progress-timing and
/// recovery clauses reach the seat through the OFFICE, so both dialects
/// inherit them without either one carrying a copy.
///
/// The proof is where the rule sits, not what a model does with it. The
/// implement seat resolves to `implementer-sdd`'s one charter — the same
/// charter whichever dialect is composed, because the recipe names an
/// agent and the dialect supplies only its own instructions — and no
/// dialect gains an implement phase, a task template clause or a
/// duplicate timing rule (decision 0042 ruling 6). What a live model then
/// does with the instruction is judgment's to check, which the charter
/// and the guides both say out loud.
#[test]
fn the_progress_rule_reaches_the_smith_through_its_office_not_a_dialect() {
    let charter = root().join("agents/charters/implementer-sdd.md");
    let flat = std::fs::read_to_string(&charter)
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(flat.contains("before starting the next group"));
    assert!(flat.contains("resumed or started cold"));

    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = compile(Some(&dialect)).unwrap();
    let SeatBody::Select { cases, .. } = &bundle.seats["implement"].body else {
        panic!("the implement seat selects by delivery class");
    };
    let SeatBody::Single { role_path, .. } = &cases["design"] else {
        panic!("the SDD delivery class seats one smith");
    };
    assert_eq!(
        role_path.canonicalize().unwrap(),
        charter.canonicalize().unwrap(),
        "the SDD smith reads the one charter that carries the rule"
    );

    // Neither dialect carries a copy, an implement phase of its own, or
    // a duplicate timing rule in its task instructions.
    for name in ["openspec", "speckit"] {
        let document = std::fs::read_to_string(root().join(format!("dialects/{name}.json")))
            .unwrap()
            .to_lowercase();
        assert!(
            !document.contains("\"implement\""),
            "{name} gains no implement phase"
        );
        for directory in [
            root().join("dialects").join(name),
            root().join("crates/brokkr-cli/dialects").join(name),
        ] {
            for entry in std::fs::read_dir(&directory).unwrap() {
                let path = entry.unwrap().path();
                let body = std::fs::read_to_string(&path).unwrap_or_default();
                assert!(
                    !body.contains("before starting the next group"),
                    "{} duplicates the office's timing rule",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn every_artifact_phase_ends_in_the_boxed_dialect_validator() {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = compile(Some(&dialect)).unwrap();
    let implement = bundle
        .machine
        .phases
        .iter()
        .position(|phase| phase == "implement")
        .unwrap();
    assert_eq!(
        &bundle.machine.phases[implement - 5..implement],
        ["specify", "clarify", "design", "tasks", "analyze"]
    );
    for phase in ["specify", "design", "tasks"] {
        let SeatBody::Sequence { steps } = &bundle.seats[phase].body else {
            panic!("{phase} is a sequence");
        };
        let validate = steps.last().unwrap();
        assert_eq!(validate.name, "validate");
        assert!(validate.class == brokkr_runtime::SeatClass::Gate);
        let StepBody::Dialect { execution } = &validate.body else {
            panic!("the final {phase} step comes from the dialect");
        };
        assert_eq!(
            execution.argv,
            [
                "openspec",
                "validate",
                "{change}",
                "--strict",
                "--no-interactive"
            ]
        );
        assert!(bundle.hands.contains_key(&format!("{phase}:validate")));
        assert!(bundle.dialect_prompts[phase].contains("OpenSpec"));
    }
    assert!(!Path::new(&root().join("recipes/triage/drivers/speckit_check.sh")).exists());

    let SeatBody::Sequence { steps } = &bundle.seats["verify"].body else {
        panic!("verify appends the dialect verifier after its own checks");
    };
    assert_eq!(steps.last().unwrap().name, "dialect-verify");
    assert!(bundle.hands.contains_key("verify:dialect-verify"));
}

#[test]
fn loops_are_check_then_judge_and_only_zero_advances() {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = compile(Some(&dialect)).unwrap();
    for (phase, clean, next) in [
        ("clarify", "clear", "design"),
        ("analyze", "consistent", "implement"),
    ] {
        let SeatBody::Sequence { steps } = &bundle.seats[phase].body else {
            panic!("{phase} is a sequence");
        };
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name, "check");
        assert!(matches!(steps[0].body, StepBody::Dialect { .. }));
        assert_eq!(steps[1].name, "judge");
        let rule = bundle
            .machine
            .rules
            .iter()
            .find(|rule| rule.from == phase && rule.result == clean)
            .unwrap();
        assert_eq!(rule.next.as_deref(), Some(next));
        assert!(bundle.machine.rules.iter().any(|rule| {
            rule.from == phase && rule.next.is_none() && rule.id.ends_with("EXHAUSTED")
        }));
    }
}

#[test]
fn a_loop_check_is_skipped_only_when_the_dialect_declares_it_unsupported() {
    let path = root().join("dialects/openspec.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    value["phases"]["clarify"]["check"] =
        json!({"unsupported": "this test dialect has no deterministic count"});
    let mut dialect = Dialect::parse("unsupported-check.json", &value.to_string())
        .unwrap()
        .0;
    dialect.render(&root().join("dialects")).unwrap();
    let bundle = compile(Some(&dialect)).unwrap();
    let SeatBody::Sequence { steps } = &bundle.seats["clarify"].body else {
        panic!("clarify remains an executable sequence");
    };
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].name, "judge");
}

#[test]
fn upstream_and_review_defects_return_to_the_earliest_owning_artifact() {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = compile(Some(&dialect)).unwrap();
    let edge = |from: &str, result: &str| {
        bundle
            .machine
            .rules
            .iter()
            .find(|rule| rule.from == from && rule.result == result && rule.next.is_some())
            .and_then(|rule| rule.next.as_deref())
    };
    assert_eq!(edge("design", "upstream"), Some("specify"));
    assert_eq!(edge("tasks", "upstream"), Some("design"));
    assert_eq!(edge("review", "clean"), Some("specify"));
    assert!(bundle.machine.rules.iter().any(|rule| {
        rule.from == "specify" && rule.result == "upstream" && rule.next.is_none()
    }));

    let review_gate = bundle.protected_phase.clone();
    assert_eq!(review_gate, "review");
}
