//! `recipes/preflight`'s terminal shape, made a test rather than a
//! comment.
//!
//! The recipe exists so a contributor can point Brokkr's own verify
//! and review seats at their unmerged branch before opening a pull
//! request. The whole safety of that offer is what the table CANNOT do:
//! there is no intake to reframe the work, no implement to change the
//! branch, and no ship to merge it. The table ends after `review` with a
//! terminal ruling and nothing else.
//!
//! That is a structural claim, so it is asserted structurally. Adding a
//! ship phase back — or a rule that routes around `review`, or a park
//! that leaves a preflight waiting on an operator who was never
//! promised one — fails here.

use std::path::PathBuf;

use brokkr_runtime::Bundle;

/// The phases the table declares, in order.
const PHASES: [&str; 4] = ["verify", "review", "done", "stop"];

/// The phases that would each turn a preflight into something it is not:
/// a reframing, an edit, a merge.
const FORBIDDEN: [&str; 3] = ["intake", "implement", "ship"];

/// The workspace root: this file lives at `crates/brokkr-runtime/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn preflight() -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join("recipes/preflight"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("recipes/preflight must compile")
}

#[test]
fn the_table_ends_after_review_and_names_no_working_phase() {
    let bundle = preflight();
    let machine = &bundle.machine;

    assert_eq!(machine.phases, PHASES, "preflight's phases moved");
    assert_eq!(machine.initial, "verify", "preflight starts at verify");
    assert_eq!(
        machine.terminal,
        ["done", "stop"],
        "preflight's terminals moved"
    );

    // Nothing ships out of a preflight, so no phase is shippable from.
    // An empty list here is the honest statement of that, not an
    // oversight: `bundles/verify` names `review` because its ruling
    // feeds one, and this recipe's does not.
    assert!(
        machine.shippable_from.is_empty(),
        "a preflight ships nothing; shippable_from must stay empty"
    );

    for forbidden in FORBIDDEN {
        assert!(
            !machine.phases.iter().any(|phase| phase == forbidden),
            "preflight declares a '{forbidden}' phase; the recipe seats \
             only verify and review"
        );
        assert!(
            !bundle.seats.contains_key(forbidden),
            "preflight seats '{forbidden}'; the recipe seats only verify \
             and review"
        );
    }

    let seats: Vec<&str> = bundle.seats.keys().map(String::as_str).collect();
    assert_eq!(seats, ["review", "verify"], "preflight's seats moved");
}

#[test]
fn every_rule_rules_from_a_seated_phase_into_a_declared_one() {
    let bundle = preflight();
    let machine = &bundle.machine;

    assert!(
        !machine.rules.is_empty(),
        "a table with no rules rules nothing"
    );
    for rule in &machine.rules {
        assert!(
            rule.from == "verify" || rule.from == "review",
            "rule {} rules from '{}', which no seat occupies",
            rule.id,
            rule.from
        );
        // A park would leave a contributor's preflight waiting on an
        // operator they do not have. Every rule here takes a
        // transition, which is also why the table may declare v1.
        let next = rule.next.as_deref().unwrap_or_else(|| {
            panic!(
                "rule {} parks; a preflight has no operator to park for",
                rule.id
            )
        });
        assert!(
            machine.phases.iter().any(|phase| phase == next),
            "rule {} advances to '{next}', which the table does not declare",
            rule.id
        );
    }
}

#[test]
fn review_is_unavoidable_and_is_the_last_word() {
    let bundle = preflight();
    let machine = &bundle.machine;

    assert_eq!(
        bundle.protected_phase, "review",
        "review is the protected phase; the compiler's structural refusal \
         hangs off this name"
    );

    // The only non-terminal a verify rule may reach is review, so no
    // ruling can carry a run past the gate.
    for rule in machine.rules.iter().filter(|rule| rule.from == "verify") {
        let next = rule.next.as_deref().expect("checked above");
        assert!(
            next == "review" || machine.terminal.iter().any(|t| t == next),
            "rule {} leaves verify for '{next}', bypassing review",
            rule.id
        );
    }

    // And review itself only ever ends the run: there is nothing after
    // it to run, which is what "terminal after review" means.
    for rule in machine.rules.iter().filter(|rule| rule.from == "review") {
        let next = rule.next.as_deref().expect("checked above");
        assert!(
            machine.terminal.iter().any(|t| t == next),
            "rule {} leaves review for '{next}'; the table must end here",
            rule.id
        );
    }
    assert!(
        machine.rules.iter().any(|rule| rule.from == "review"),
        "review rules on nothing"
    );
}
