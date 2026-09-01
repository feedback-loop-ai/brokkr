//! The old `forge` spellings, honored for one more release.
//!
//! Decision 0019 rulings 2 and 9 moved the harness's env overrides to
//! `BROKKR_*` and the bundle token to `{brokkr}`. An old spelling still
//! resolves to exactly what the new one resolves to — nothing about the
//! resolution differs — and says so ONCE per process, the first time any
//! of them is used. Once, not once per read: an operator needs telling,
//! not nagging, and law 4 keeps the line plain.
//!
//! One latch and one line for every old spelling in the tree, so "the
//! note is said once" has exactly one implementation to be true of.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

/// The process's one latch. Only `say_once` reads it; `say` takes the
/// latch it flips as an argument, which is what makes the once-ness
/// testable without a process boundary.
static SAID: AtomicBool = AtomicBool::new(false);

/// Both spellings on one line: the old one so an operator can find what
/// they wrote, the new one so they know what to write instead.
fn note(old: &str, new: &str) -> String {
    format!("notice: {old} is now named {new}; the old name works for one more release.")
}

fn say(latch: &AtomicBool, out: &mut impl Write, old: &str, new: &str) {
    if !latch.swap(true, Ordering::Relaxed) {
        // A closed stderr is not a reason to fail a run over a courtesy.
        let _ = writeln!(out, "{}", note(old, new));
    }
}

/// stderr, never stdout: a piped readout and every `--json` consumer
/// read exactly what they read without the fallback.
pub fn say_once(old: &str, new: &str) {
    say(&SAID, &mut std::io::stderr(), old, new);
}

/// The new spelling first, the old one after it — and the note when the
/// old one is what answered. `legacy: None` is a variable that was never
/// renamed: it has no old spelling and earns no note.
pub fn env(primary: &str, legacy: Option<&str>) -> Option<String> {
    if let Ok(value) = std::env::var(primary) {
        return Some(value);
    }
    let old = legacy?;
    let value = std::env::var(old).ok()?;
    say_once(old, primary);
    Some(value)
}

#[cfg(test)]
mod tests;
