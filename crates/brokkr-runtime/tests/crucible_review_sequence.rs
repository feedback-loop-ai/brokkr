//! `recipes/crucible` puts a panel-then-chief SEQUENCE on the protected
//! `review` phase — the one structural shape no shipped recipe had
//! before it (`recipes/sdd` sequences `design`, not `review`;
//! `recipes/panel-review` panels `review` flat, with no chief).
//!
//! The shape's own hazard is that the panel's verdict is NOT the
//! machine's verdict: `positions` is a non-final step, so its
//! `review-panel` output never reaches the rule table and is never
//! checked against the seat's declared results. A `security-hold` from
//! the panel therefore only stops the run if the chief reproduces it.
//!
//! Two things have to hold for that to be enforceable, and this file
//! pins both against the SHIPPED recipe rather than a fixture:
//!
//! 1. the structure — a two-step sequence, the panel work-class under
//!    the `review-panel` aggregate, the chief a single GATE step, and
//!    the seat still speaking `fast`'s review vocabulary so the
//!    inherited reforging ladder rules on it unmodified;
//! 2. the charter — `roles/review-chief.md` states the floor the chief
//!    may not rule below, naming `security-hold` explicitly.
//!
//! The third leg, that the panel's result actually ARRIVES at the
//! chief's driver input and that the chief's result is what the effect
//! reports, is behavioural and lives beside the sequence executor:
//! `chief_synthesis_carries_a_panel_security_hold_to_the_machine` in
//! `crates/brokkr-runtime/src/engine/tests.rs`.

use std::path::PathBuf;

use brokkr_runtime::{Aggregate, Bundle, SeatBody, StepBody};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn crucible() -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join("recipes/crucible"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("recipes/crucible must compile")
}

#[test]
fn review_is_a_positions_panel_followed_by_a_single_chief_gate() {
    let bundle = crucible();
    let review = &bundle.seats["review"];

    // The seat still speaks fast's review vocabulary, which is what lets
    // it inherit decision 0022's ladder with no policy table of its own.
    assert_eq!(
        review.results,
        vec![
            "clean".to_string(),
            "residual".to_string(),
            "security-hold".to_string()
        ],
        "crucible's review seat must keep the shared review vocabulary"
    );
    assert_eq!(bundle.protected_phase, "review");

    let SeatBody::Sequence { steps } = &review.body else {
        panic!("crucible's review seat is a sequence, not a flat panel");
    };
    assert_eq!(
        steps.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["positions", "chief"],
        "step ORDER is load-bearing: the panel states, then the chief rules"
    );

    let StepBody::Panel { members, aggregate } = &steps[0].body else {
        panic!("'positions' is a panel");
    };
    assert_eq!(*aggregate, Aggregate::ReviewPanel);
    assert_eq!(
        members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        ["correctness", "security"],
        "the panel reuses recipes/panel-review's member names"
    );

    // The chief is a SINGLE step, not a second panel: one seat rules, so
    // there is one place the verdict can be traced to.
    assert!(
        matches!(&steps[1].body, StepBody::Single { .. }),
        "'chief' must be a single driver — a panel here would have no judge"
    );
}

/// Decision 0021, read off the compiled bundle rather than the source:
/// the chief JUDGES and so pins the trusted adapter that authorised it;
/// the positions WORK and pin nothing, which is what makes seating a
/// challenger on one of them a lawful experiment (ruling 7).
#[test]
fn only_the_chief_step_is_witnessed_as_a_gate() {
    let bundle = crucible();
    let root = workspace();
    let adapters = brokkr_runtime::agents::Adapters::load(&root.join("adapters"))
        .expect("the shipped adapters load");
    let claude = adapters
        .digest("claude")
        .expect("the incumbent adapter is declared");

    let witnessed = bundle.manifest["drivers"]
        .as_object()
        .expect("crucible witnesses drivers for its gates");
    assert_eq!(
        witnessed.keys().map(String::as_str).collect::<Vec<_>>(),
        ["review:chief", "ship", "verify"],
        "the review gate is the chief STEP, never the seat"
    );
    assert_eq!(
        witnessed["review:chief"],
        serde_json::json!({ "claude": claude })
    );
    for working in ["review:positions:correctness", "review:positions:security"] {
        assert!(
            !witnessed.contains_key(working),
            "'{working}' works and consults nothing; witnessing it would \
             claim an authority it never asked for"
        );
    }
}

/// The chief's floor is a charter instruction, so the only thing a test
/// can pin is that the instruction is present and unambiguous. It is the
/// half of the safety property no compile-time refusal can carry: the
/// engine will accept any declared result the chief emits, which is
/// exactly why the sentence below has to survive an edit.
#[test]
fn the_chief_charter_states_the_floor_it_may_not_rule_below() {
    let charter =
        std::fs::read_to_string(workspace().join("recipes/crucible/roles/review-chief.md"))
            .expect("the chief charter ships with the recipe");
    // Matched against the prose with its line breaks collapsed, so
    // rewrapping a paragraph is not a test failure but deleting the
    // sentence is.
    let prose = charter.split_whitespace().collect::<Vec<_>>().join(" ");
    for required in [
        "context.prior_results.positions",
        "You may never rule below what the panel reported",
        "== \"security-hold\"` → your result is `security-hold`. Full stop.",
        "You may raise a severity; you may not lower one",
        "**Your result is the seat's result**",
    ] {
        assert!(
            prose.contains(required),
            "the chief charter no longer states: {required}"
        );
    }
}
