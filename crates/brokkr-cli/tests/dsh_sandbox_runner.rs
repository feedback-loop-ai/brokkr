//! Decision 0053: `brokkr dsh-sandbox-runner` is the bwrap-compatible
//! runner `brokkr driver dsh` points dsh's sandbox provider at. It is
//! dispatched before clap because dsh hands it bubblewrap's own argv,
//! including the bare `--`; these tests drive the real binary and a fake
//! `bwrap`, so the argv hand-off is proven end to end without a model
//! loop.

use std::path::Path;
use std::process::{Command, Output};

fn brokkr(args: &[&str], path: &Path, dump: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command.args(args).env("PATH", path);
    if let Some(dump) = dump {
        command.env("DUMP", dump);
    }
    command.output().unwrap()
}

/// A `bwrap` that records the argv the runner hands it and exits clean.
#[cfg(unix)]
fn fake_bwrap(dir: &Path) -> std::path::PathBuf {
    let bwrap = dir.join("bwrap");
    std::fs::write(
        &bwrap,
        "#!/bin/sh\n: > \"$DUMP\"\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> \"$DUMP\"; done\nexit 0\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bwrap, std::fs::Permissions::from_mode(0o755)).unwrap();
    bwrap
}

#[cfg(unix)]
#[test]
fn the_runner_execs_bwrap_with_the_scoped_git_binds_and_the_command() {
    let dir = tempfile::tempdir().unwrap();
    let dump = dir.path().join("argv");
    fake_bwrap(dir.path());

    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/work/wt",
            "--git-dir",
            "/main/.git/worktrees/wt",
            "--common-dir",
            "/main/.git",
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--unshare-pid",
            "--proc",
            "/proc",
            "--die-with-parent",
            "--tmpfs",
            "/tmp",
            "--bind",
            "/work/wt",
            "/work/wt",
            "--",
            "bash",
            "-lc",
            "git add -A",
        ],
        dir.path(),
        Some(&dump),
    );
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let seen = std::fs::read_to_string(&dump).unwrap();
    assert!(
        seen.contains("--bind\n/main/.git/worktrees/wt\n/main/.git/worktrees/wt\n"),
        "{seen}"
    );
    assert!(
        seen.contains("--bind-try\n/main/.git/objects\n/main/.git/objects\n"),
        "{seen}"
    );
    assert!(seen.contains("--tmpfs\n/main/.git/hooks\n"), "{seen}");
    assert!(
        seen.contains("--ro-bind-try\n/main/.git/config\n/main/.git/config\n"),
        "{seen}"
    );
    assert!(seen.ends_with("--\nbash\n-lc\ngit add -A\n"), "{seen}");
}

#[cfg(unix)]
#[test]
fn the_runner_refuses_a_malformed_profile_and_a_missing_bwrap_as_runner_failures() {
    // No `--` before the command: the profile is not one the runner knows.
    let dir = tempfile::tempdir().unwrap();
    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/w",
            "--git-dir",
            "/g",
            "--common-dir",
            "/c",
            "--ro-bind",
            "/",
            "/",
        ],
        dir.path(),
        None,
    );
    assert_eq!(run.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("brokkr-dsh-sandbox-runner: "), "{stderr}");
    assert!(stderr.contains("no `--` before the command"), "{stderr}");

    // The profile is well formed but bubblewrap is not on PATH: the exec
    // refusal carries the same runner-failure signature.
    let empty = tempfile::tempdir().unwrap();
    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/work/wt",
            "--git-dir",
            "/main/.git/worktrees/wt",
            "--common-dir",
            "/main/.git",
            "--ro-bind",
            "/",
            "/",
            "--bind",
            "/work/wt",
            "/work/wt",
            "--",
            "true",
        ],
        empty.path(),
        None,
    );
    assert_eq!(run.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("brokkr-dsh-sandbox-runner: "), "{stderr}");
}
