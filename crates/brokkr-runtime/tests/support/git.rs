//! A real git repository at a fixture's root, shared by the unit and
//! integration fixtures that mark a workspace root with `.git` (#908): a
//! bundle's walk asks git in the repository it stands in, so a bare `.git`
//! directory is a refusal to answer, not a marker. A fixture's git keeps the
//! host's own configuration out (#908, fix round 908e), command-scope
//! settings included (#936): a global, system or `-c` `core.excludesFile`
//! must not decide what a fixture's walk pins. A test whose repository must
//! answer for itself, past a host's command-scope settings, runs again in a
//! child of its own ([`in_clean_git_config`]).

use std::path::Path;
use std::process::{Command, Output};

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

/// Remove git's command-scope settings from `command`'s environment: the
/// variables a host uses to hand git `-c`, which outrank a repository's own
/// configuration and would reach every child it starts (#936).
fn clean_command_scope(command: &mut Command) {
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy();
        if key == "GIT_CONFIG_COUNT"
            || key == "GIT_CONFIG_PARAMETERS"
            || numbered(&key, "GIT_CONFIG_KEY_")
            || numbered(&key, "GIT_CONFIG_VALUE_")
        {
            command.env_remove(key.as_ref());
        }
    }
}

/// Whether `key` is `prefix` followed by a whole number, git's shape for a
/// command-scope setting's key or value.
fn numbered(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix)
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

/// `git args` run in `dir` with the host's own configuration isolated: the
/// global and system files are the empty `/dev/null` (or disabled), and the
/// command-scope settings a host exported are absent. A fixture's `git init`
/// and `git config` must answer whatever the calling shell inherited (#936).
pub(crate) fn run(dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    clean_command_scope(&mut command);
    command.output().expect("git")
}

/// `dir` made a real git repository, creating the directory when needed.
pub(crate) fn test_repository(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let out = run(dir, &["init", "-q"]);
    assert!(
        out.status.success(),
        "git init -q at {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    // The repository's own configuration overrides any host global or
    // system `core.excludesFile` for every later git question about `dir`,
    // the compile's own included: a test cannot set that process's
    // environment, so the fixture must answer for itself (#908, fix round
    // 908e). Its `.gitignore` and `info/exclude` still apply.
    let out = run(
        dir,
        &["config", "--local", "core.excludesFile", "/dev/null"],
    );
    assert!(
        out.status.success(),
        "git config core.excludesFile at {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}
