//! Decision 0042's first SDD slice: the realm supplies validation and the
//! retired modification-time grep is no longer part of the strategy.

use std::path::{Path, PathBuf};

use brokkr_runtime::bundle::{SeatBody, StepBody};
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::Bundle;

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
    )
}

#[test]
fn artifact_work_refuses_the_bootstrap_realm_and_accepts_a_declared_dialect() {
    let message = compile(None).unwrap_err().to_string();
    assert!(message.contains("realm 'brokkr'"), "{message}");
    assert!(message.contains("phase 'design' needs one"), "{message}");

    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    assert!(compile(Some(&dialect)).is_ok());
}

#[test]
fn design_ends_in_the_boxed_dialect_validator_and_the_mtime_driver_is_gone() {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = compile(Some(&dialect)).unwrap();
    let SeatBody::Sequence { steps } = &bundle.seats["design"].body else {
        panic!("design is a sequence");
    };
    let validate = steps.last().unwrap();
    assert_eq!(validate.name, "validate");
    assert!(validate.class == brokkr_runtime::SeatClass::Gate);
    let StepBody::Dialect { execution } = &validate.body else {
        panic!("the final design step comes from the dialect");
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
    assert!(bundle.hands.contains_key("design:validate"));
    assert!(!Path::new(&root().join("recipes/triage/drivers/speckit_check.sh")).exists());

    let SeatBody::Sequence { steps } = &bundle.seats["verify"].body else {
        panic!("verify appends the dialect verifier after its own checks");
    };
    assert_eq!(steps.last().unwrap().name, "dialect-verify");
    assert!(bundle.hands.contains_key("verify:dialect-verify"));
}
