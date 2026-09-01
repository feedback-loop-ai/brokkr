//! `recipes/wager-harness` is decision 0021 ruling 7's wager written as
//! a recipe: the incumbent's strategy with exactly one seat's driver
//! swapped for a challenger's. Its whole evidentiary value rests on
//! there being ONE difference, and the original wager's second
//! confession (`docs/essays/the-wager.md`) is what happens when there is
//! a second one nobody noticed: round one blocked mid-implement because
//! the challenger ran in a tighter sandbox than the incumbent, and the
//! result had to be thrown out.
//!
//! Items of that README's parity checklist which are mechanical facts of
//! the two bundles are asserted here, so a future edit that quietly
//! introduces a second variable fails a test instead of a wager.

use std::path::PathBuf;

use brokkr_runtime::{Bundle, SeatBody};

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

/// Checklist item 4: same charter, byte for byte. Not "equivalent", not
/// "adapted for the challenger" — a charter rewritten to suit one arm is
/// a second variable, and two variables measure nothing.
#[test]
fn the_challenger_implements_under_the_incumbents_charter_byte_for_byte() {
    let root = workspace();
    let incumbent = std::fs::read(root.join("recipes/fast/roles/implementer.md"))
        .expect("fast's implementer charter");
    let challenger = std::fs::read(root.join("recipes/wager-harness/roles/implementer.md"))
        .expect("wager-harness's implementer charter");
    assert_eq!(
        challenger, incumbent,
        "recipes/wager-harness/roles/implementer.md has diverged from \
         recipes/fast/roles/implementer.md; the wager measures the driver \
         swap ONLY while the charter is identical (parity checklist item 4)"
    );
}

/// Checklist items 5 and 7: same limits, one seat overridden. Everything
/// the challenger's bundle does not restate is inherited, so the seats
/// are compared here against `fast`'s own compiled values rather than
/// against numbers copied into this test.
#[test]
fn exactly_one_seat_differs_and_it_differs_only_in_its_driver() {
    let incumbent = compile("recipes/fast");
    let challenger = compile("recipes/wager-harness");

    assert_eq!(
        challenger.seats.keys().collect::<Vec<_>>(),
        incumbent.seats.keys().collect::<Vec<_>>(),
        "the wager adds and removes no seat"
    );
    assert_eq!(
        challenger.protected_phase, incumbent.protected_phase,
        "same protected phase, or the arms answer to different judges"
    );

    for (phase, mine) in &challenger.seats {
        let theirs = &incumbent.seats[phase];
        assert_eq!(&mine.results, &theirs.results, "seat '{phase}' results");
        assert_eq!(&mine.inputs, &theirs.inputs, "seat '{phase}' inputs");
        assert_eq!(&mine.secrets, &theirs.secrets, "seat '{phase}' secrets");
        assert_eq!(
            mine.limits.max_attempts, theirs.limits.max_attempts,
            "seat '{phase}' max_attempts: a challenger given more attempts \
             is not being compared (parity checklist item 5)"
        );
        assert_eq!(
            mine.limits.timeout_seconds, theirs.limits.timeout_seconds,
            "seat '{phase}' timeout_seconds (parity checklist item 5)"
        );
    }

    // Checklist item 7: the ONE difference, named.
    let mut differing: Vec<&str> = Vec::new();
    for (phase, mine) in &challenger.seats {
        let (
            SeatBody::Single { command: mine, .. },
            SeatBody::Single {
                command: theirs, ..
            },
        ) = (&mine.body, &incumbent.seats[phase].body)
        else {
            panic!("both arms seat single drivers throughout");
        };
        if mine != theirs {
            differing.push(phase);
        }
    }
    assert_eq!(
        differing,
        ["implement"],
        "the wager overrides exactly one seat; anything else is a second \
         variable (parity checklist item 7)"
    );
}

/// Checklist item 6: same gates, same judge. The challenger holds a WORK
/// seat and nothing else — decision 0021 ruling 7 admits it there freely
/// and refuses it at a gate at compile time, which is what makes the
/// comparison measure the crews rather than the referee.
#[test]
fn the_challenger_holds_no_gate_and_the_gates_stay_on_the_incumbent() {
    let incumbent = compile("recipes/fast");
    let challenger = compile("recipes/wager-harness");
    assert_eq!(
        challenger.manifest["drivers"], incumbent.manifest["drivers"],
        "the judging seats — and the adapter declarations authorising \
         them — are identical in both arms (parity checklist item 6)"
    );

    let SeatBody::Single { command, .. } = &challenger.seats["implement"].body else {
        panic!("the challenger's implement seat is a single driver")
    };
    assert!(
        command.iter().any(|part| part == "codex"),
        "the challenger's implement driver dispatches to codex: {command:?}"
    );
    assert!(
        !command.iter().any(|part| part == "{forge}"),
        "a NEW recipe should not ship the retired {{forge}} token \
         (decision 0019): {command:?}"
    );
}
