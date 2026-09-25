use std::sync::atomic::AtomicBool;

use super::{env, say};
use crate::env_guard::EnvGuard;

#[test]
fn the_new_spelling_wins_when_both_are_set() {
    let mut vars = EnvGuard::lock();
    vars.set("BROKKR_RENAME_BOTH", "new");
    vars.set("FORGE_RENAME_BOTH", "old");
    assert_eq!(
        env("BROKKR_RENAME_BOTH", Some("FORGE_RENAME_BOTH")).as_deref(),
        Some("new")
    );
}

#[test]
fn the_old_spelling_answers_when_the_new_one_is_absent() {
    let mut vars = EnvGuard::lock();
    vars.remove("BROKKR_RENAME_OLD");
    vars.set("FORGE_RENAME_OLD", "old");
    assert_eq!(
        env("BROKKR_RENAME_OLD", Some("FORGE_RENAME_OLD")).as_deref(),
        Some("old")
    );
}

/// A variable deliberately configured without a legacy spelling reads one
/// name only, so there is nothing to fall back to and nothing to say.
#[test]
fn a_variable_with_no_old_spelling_reads_only_its_own_name() {
    let mut vars = EnvGuard::lock();
    vars.remove("BROKKR_RENAME_ONLY");
    vars.set("FORGE_RENAME_ONLY", "old");
    assert_eq!(env("BROKKR_RENAME_ONLY", None), None);
    vars.set("BROKKR_RENAME_ONLY", "new");
    assert_eq!(env("BROKKR_RENAME_ONLY", None).as_deref(), Some("new"));
}

#[test]
fn neither_spelling_set_resolves_to_nothing() {
    let mut vars = EnvGuard::lock();
    vars.remove("BROKKR_RENAME_NEITHER");
    vars.remove("FORGE_RENAME_NEITHER");
    assert_eq!(
        env("BROKKR_RENAME_NEITHER", Some("FORGE_RENAME_NEITHER")),
        None
    );
}

/// The property both fallbacks are held to: one line for the process,
/// whichever old spelling reaches it first and however many times any of
/// them is read afterwards.
#[test]
fn the_note_is_said_once_for_the_process_not_once_per_read() {
    let latch = AtomicBool::new(false);
    let mut out: Vec<u8> = Vec::new();
    say(&latch, &mut out, "{forge}", "{brokkr}");
    say(&latch, &mut out, "{forge}", "{brokkr}");
    say(&latch, &mut out, "FORGE_CODEX_BIN", "BROKKR_CODEX_BIN");

    let said = String::from_utf8(out).unwrap();
    assert_eq!(said.lines().count(), 1, "{said}");
    let line = said.lines().next().unwrap();
    assert!(line.starts_with("notice: "), "{line}");
    assert!(
        line.contains("{forge}") && line.contains("{brokkr}"),
        "{line}"
    );
    assert!(line.contains("one more release"), "{line}");
}
