//! The one writer of the process environment in a test binary (#357).
//!
//! A test binary has one environment and runs its tests on many threads,
//! so every test that writes a variable, or reads one another test
//! writes, holds this guard for its whole body. The guard records what
//! each write replaced and undoes them all on `Drop`, which runs on
//! unwind too, so a failing assert can no longer leave its variables
//! behind. The lock ignores poison: a test that panics while holding it
//! fails alone instead of failing every later test with `PoisonError`.
//!
//! Each binary includes this file once through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, MutexGuard, PoisonError};

static ENVIRONMENT: Mutex<()> = Mutex::new(());

/// Exclusive use of the process environment until drop, with every
/// variable it changed put back.
pub struct EnvGuard {
    saved: Vec<(OsString, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// Wait for the environment. A reader that changes nothing holds the
    /// guard as well, so it never observes another test's value.
    pub fn lock() -> Self {
        Self {
            saved: Vec::new(),
            _lock: ENVIRONMENT.lock().unwrap_or_else(PoisonError::into_inner),
        }
    }

    pub fn set(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        self.save(key.as_ref());
        write(key.as_ref(), Some(value.as_ref()));
    }

    pub fn remove(&mut self, key: impl AsRef<OsStr>) {
        self.save(key.as_ref());
        write(key.as_ref(), None);
    }

    fn save(&mut self, key: &OsStr) {
        self.saved.push((key.to_owned(), std::env::var_os(key)));
    }
}

impl Drop for EnvGuard {
    /// Undo every write, last first, so each variable ends at the value
    /// it held before its first write.
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..).rev() {
            write(&key, value.as_deref());
        }
    }
}

fn write(key: &OsStr, value: Option<&OsStr>) {
    match value {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

/// A test that panics while it holds the guard poisons the lock and
/// skips its own restore code. The next test still gets the lock, and
/// finds every variable as it was before the failed test began.
#[test]
fn a_panic_under_the_guard_restores_the_environment_and_does_not_cascade() {
    const SET: &str = "BROKKR_ENV_GUARD_PROBE_SET";
    const REMOVED: &str = "BROKKR_ENV_GUARD_PROBE_REMOVED";
    // A guard whose record is cleared keeps its writes: this is how the
    // test plants a value that was there before the failing test began.
    let mut planted = EnvGuard::lock();
    planted.remove(SET);
    planted.set(REMOVED, "before");
    planted.saved.clear();
    drop(planted);
    let failed = std::panic::catch_unwind(|| {
        let mut env = EnvGuard::lock();
        env.set(SET, "first");
        env.set(SET, "second");
        env.remove(REMOVED);
        panic!("a planted failure");
    });
    let payload = failed.expect_err("the planted failure panics");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"a planted failure"));
    assert!(ENVIRONMENT.is_poisoned(), "the panic poisoned the lock");
    let mut env = EnvGuard::lock();
    assert_eq!(std::env::var_os(SET), None);
    assert_eq!(std::env::var_os(REMOVED), Some("before".into()));
    env.remove(REMOVED);
    env.saved.clear();
}

/// Decision 0071 ruling 9: nothing outside this file writes the process
/// environment. The walk reads every Rust file a test binary can compile
/// (the crates and this support tree) and refuses the two writers by
/// name wherever they appear, including in comments, `use` aliases and
/// calls split across lines. A file it cannot read as text is refused.
#[test]
fn nothing_but_the_guard_writes_the_process_environment() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("a crate sits at crates/<name>");
    let mut offenders = Vec::new();
    for tree in ["crates", "tests"] {
        writers_under(root, std::path::Path::new(tree), &mut offenders);
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "write the environment through EnvGuard (tests/support/env_guard.rs)"
    );
}

/// Walk `relative` under `root`, naming each offender by its path from
/// the workspace root.
fn writers_under(root: &std::path::Path, relative: &std::path::Path, offenders: &mut Vec<String>) {
    let dir = root.join(relative);
    for entry in
        std::fs::read_dir(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
    {
        let name = relative.join(entry.expect("directory entry").file_name());
        let kind = std::fs::symlink_metadata(root.join(&name))
            .expect("metadata")
            .file_type();
        if kind.is_dir() {
            writers_under(root, &name, offenders);
        } else if kind.is_symlink() {
            offenders.push(format!(
                "{}: a link the walk will not follow",
                name.display()
            ));
        } else if name.extension().is_some_and(|extension| extension == "rs")
            && name != std::path::Path::new("tests/support/env_guard.rs")
        {
            writers_in(root, &name, offenders);
        }
    }
}

fn writers_in(root: &std::path::Path, name: &std::path::Path, offenders: &mut Vec<String>) {
    let text = std::fs::read_to_string(root.join(name))
        .unwrap_or_else(|error| panic!("{}: {error}", name.display()));
    for (index, line) in text.lines().enumerate() {
        if ["set_var", "remove_var"]
            .iter()
            .any(|writer| line.contains(writer))
        {
            offenders.push(format!("{}:{}", name.display(), index + 1));
        }
    }
}
