//! The operator's 2026-09-15 ruling, at the composition it has to hold in:
//! “keep decision 0030's rejoin live, do not regress codex.”
//!
//! Main rejoins the Codex work seats the shipped recipes INLINE, through a
//! raw `driver.command` rather than the agent library. `Bundle::compile_with`
//! reads the adapter each inline built-in driver names and carries that
//! adapter's measured resume assessment on the compiled bundle, so the
//! engine can place it in the driver's private start context exactly as it
//! does for an agent-resolved site. Before this, an inline seat carried no
//! assessment, the gate read `unsupported-resume` and main's shipping retry
//! spawned cold — the regression this test names.
//!
//! The other three shipped shapes stay unmeasured: they are new rejoins main
//! does not perform, and this slice enables none of them.

use std::path::PathBuf;

use brokkr_runtime::Bundle;

/// The workspace root: this file lives at `crates/brokkr-runtime/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn compile(relative: &str) -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join(relative),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|e| panic!("{relative} must compile: {e}"))
}

/// The shipped inline Codex work seats main rejoins carry the preserved
/// `work-site` assessment, scoped to the author-written coordinate
/// (`boundary: not applicable`) beside the engine-composed `harness` one.
#[test]
fn the_shipped_inline_codex_work_seats_carry_the_preserved_assessment() {
    for relative in ["recipes/standby", "recipes/wager-harness"] {
        let bundle = compile(relative);
        let assessment = bundle
            .inline_resume
            .get("implement")
            .unwrap_or_else(|| panic!("{relative} implements inline through codex"));
        assert_eq!(
            assessment["work-site"]["status"], "supported",
            "{relative} must preserve main's inline Codex rejoin: {assessment}"
        );
        let boundaries = assessment["work-site"]["boundaries"]
            .as_array()
            .unwrap_or_else(|| panic!("{relative}: boundaries must be an array"));
        assert!(
            boundaries.iter().any(|word| word == "not applicable"),
            "{relative}: the inline coordinate's boundary must be declared: {boundaries:?}"
        );
        assert_eq!(
            assessment["work-site"]["hands"], "none",
            "{relative}: an inline seat carries no hands"
        );
    }
}

/// The other three shipped shapes are not enabled by this ruling: an inline
/// site of each still carries its provider's own assessment, whose shape is
/// `unmeasured`, so the driver declines as `unsupported-resume`.
#[test]
fn the_other_shipped_shapes_stay_unmeasured_at_inline_sites() {
    for (relative, seat, shape) in [
        ("recipes/fast", "implement", "boxed-workspace"),
        ("recipes/wager-harness-dsh", "implement", "headless-work"),
    ] {
        let bundle = compile(relative);
        let assessment = bundle
            .inline_resume
            .get(seat)
            .unwrap_or_else(|| panic!("{relative}: seat '{seat}' is inline"));
        assert_eq!(
            assessment[shape]["status"], "unmeasured",
            "{relative} must stay disabled: {assessment}"
        );
    }
}
