//! A real git repository at a fixture's root, shared by the unit and
//! integration fixtures that mark a workspace root with `.git` (#908): a
//! bundle's walk asks git in the repository it stands in, so a bare `.git`
//! directory is a refusal to answer, not a marker. A fixture's git keeps the
//! host's own configuration out (#908, fix round 908e): a global or system
//! `core.excludesFile` must not decide what a fixture's walk pins.

use std::path::Path;
use std::process::{Command, Output};

/// `git args` run in `dir` with the host's own configuration isolated: the
/// global and system files are the empty `/dev/null` (or disabled).
pub(crate) fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git")
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
