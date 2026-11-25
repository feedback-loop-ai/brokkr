//! A test re-executed in a child of its own with git's command-scope
//! configuration cleared (#936), beside the fixture repositories of
//! `git.rs`, whose scrub it shares. Its own file, so a test binary that
//! builds repositories but re-executes no test includes none of it (fix
//! round 0109a1r3).

use std::process::Command;

use super::git::clean_command_scope;

/// Set in the child a re-executed test runs under: git's command-scope
/// configuration is absent from its environment.
const CLEAN: &str = "BROKKR_TEST_GIT_CONFIG_CLEAN";

/// Whether this process is the child that runs this test with git's
/// command-scope configuration cleared, where the caller runs its body;
/// otherwise this test was run so, alone, and passed, and the caller
/// returns.
///
/// A host hands git `-c` settings through `GIT_CONFIG_COUNT` with numbered
/// `GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n`, or through
/// `GIT_CONFIG_PARAMETERS`. Those outrank a repository's own
/// `core.excludesFile`, and they reach every git child the code under test
/// spawns, not only this module's own calls. So a test whose fixture must
/// answer for itself runs again where they are removed (#936; decision 0071
/// ruling 9). The process's own environment is never changed: a test binary
/// runs its tests in parallel.
///
/// libtest names a test's thread after the test's full path, which is the
/// name `--exact` runs, so the helper derives the name from the calling
/// test rather than trusting an argument: a call site that named another
/// test would otherwise pass with its own body never run, a verdict that is
/// not the test's own (decision 0071 ruling 9).
pub(crate) fn in_clean_git_config() -> bool {
    if std::env::var_os(CLEAN).is_some() {
        return true;
    }
    let test = std::thread::current()
        .name()
        .expect("libtest names a test's thread after the test")
        .to_owned();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .arg("--exact")
        .arg(&test)
        .args(["--nocapture", "--test-threads=1"]);
    // A test binary moves the process's working directory in some suites
    // (queued_launch's `Standing`), and a sibling may have moved it into a
    // fixture this process then drops before the child starts. The child
    // gets a directory that outlives every test instead of that inherited
    // cwd (#936).
    child.current_dir(env!("CARGO_MANIFEST_DIR"));
    child.env(CLEAN, "1");
    clean_command_scope(&mut child);
    let out = child.output().expect("the re-executed test binary runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success() && stdout.contains("test result: ok. 1 passed"),
        "{test} under a cleared git command-scope configuration: {}\n{stdout}\n{stderr}",
        out.status
    );
    false
}
