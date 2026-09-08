//! Decision 0053: `brokkr dsh-sandbox-runner` is the bwrap-compatible
//! runner `brokkr driver dsh` points dsh's sandbox provider at. It is
//! dispatched before clap because dsh hands it bubblewrap's own argv,
//! including the bare `--`; these tests drive the real binary and a fake
//! `bwrap`, so the argv hand-off is proven end to end without a model
//! loop. The bubblewrap to exec is a trusted argv path, not a `PATH`
//! lookup: the boundary is the binary the driver probed.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn brokkr(args: &[&str], path: &Path, dump: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command.args(args).env("PATH", path);
    if let Some(dump) = dump {
        command.env("DUMP", dump);
    }
    command.output().unwrap()
}

#[cfg(unix)]
fn executable(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A `bwrap` that records the argv the runner hands it and exits clean.
#[cfg(unix)]
fn fake_bwrap(dir: &Path) -> PathBuf {
    executable(
        dir,
        "bwrap",
        "#!/bin/sh\n: > \"$DUMP\"\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> \"$DUMP\"; done\nexit 0\n",
    )
}

#[cfg(unix)]
#[test]
fn the_runner_execs_bwrap_with_the_scoped_git_binds_and_the_command() {
    let dir = tempfile::tempdir().unwrap();
    let dump = dir.path().join("argv");
    let bwrap = fake_bwrap(dir.path());

    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/work/wt",
            "--git-dir",
            "/main/.git/worktrees/wt",
            "--common-dir",
            "/main/.git",
            "--bwrap",
            bwrap.to_str().unwrap(),
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
    // `config.worktree` is masked with an empty read-only file, not
    // `--ro-bind-try`: the per-worktree file is normally absent and the
    // writable directory around it would let the box create one.
    assert!(
        seen.contains("--ro-bind\n/dev/null\n/main/.git/worktrees/wt/config.worktree\n"),
        "{seen}"
    );
    assert!(
        seen.contains(
            "--ro-bind-try\n/main/.git/worktrees/wt/commondir\n/main/.git/worktrees/wt/commondir\n"
        ),
        "{seen}"
    );
    assert!(seen.ends_with("--\nbash\n-lc\ngit add -A\n"), "{seen}");
}

#[cfg(unix)]
#[test]
fn the_runner_execs_the_absolute_bwrap_it_was_given_and_never_a_path_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let chosen = dir.path().join("chosen");
    std::fs::create_dir_all(&chosen).unwrap();
    let dump = dir.path().join("argv");
    let recording = fake_bwrap(&chosen);

    // A decoy `bwrap` first on PATH: if the runner looked it up, this
    // would run and leave its marker.
    let decoy = dir.path().join("decoy");
    std::fs::create_dir_all(&decoy).unwrap();
    let marker = dir.path().join("decoy-ran");
    executable(
        &decoy,
        "bwrap",
        &format!("#!/bin/sh\n: > '{}'\nexit 7\n", marker.display()),
    );

    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/work/wt",
            "--git-dir",
            "/main/.git/worktrees/wt",
            "--common-dir",
            "/main/.git",
            "--bwrap",
            recording.to_str().unwrap(),
            "--ro-bind",
            "/",
            "/",
            "--bind",
            "/work/wt",
            "/work/wt",
            "--",
            "true",
        ],
        &decoy,
        Some(&dump),
    );
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(dump.exists(), "the chosen bwrap recorded its argv");
    assert!(!marker.exists(), "the bwrap on PATH must never run");
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
            "--bwrap",
            "/bin/bwrap",
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

    // The profile is well formed but the named bubblewrap is not there:
    // the exec refusal carries the same runner-failure signature.
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
            "--bwrap",
            "/nonexistent/bwrap",
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

    // A relative bubblewrap is refused before any exec: the runner never
    // resolves one through a working directory the seat can move.
    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/work/wt",
            "--git-dir",
            "/main/.git/worktrees/wt",
            "--common-dir",
            "/main/.git",
            "--bwrap",
            "bwrap",
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
    assert!(stderr.contains("is not an absolute path"), "{stderr}");
}

/// Decision 0053 addendum: a scope whose git directory IS the shared
/// repository would need the whole shared `.git` writable, so the real
/// runner refuses it with the signature dsh classifies as a runner
/// failure rather than mounting it.
#[cfg(unix)]
#[test]
fn the_runner_refuses_a_scope_that_would_mount_the_whole_shared_git() {
    let dir = tempfile::tempdir().unwrap();
    let dump = dir.path().join("argv");
    let bwrap = fake_bwrap(dir.path());
    let run = brokkr(
        &[
            "dsh-sandbox-runner",
            "--workspace",
            "/repo/src",
            "--git-dir",
            "/repo/.git",
            "--common-dir",
            "/repo/.git",
            "--bwrap",
            bwrap.to_str().unwrap(),
            "--ro-bind",
            "/",
            "/",
            "--bind",
            "/repo/src",
            "/repo/src",
            "--",
            "true",
        ],
        dir.path(),
        Some(&dump),
    );
    assert_eq!(run.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("brokkr-dsh-sandbox-runner: "), "{stderr}");
    assert!(
        stderr.contains("shared repository's own git directory"),
        "{stderr}"
    );
    assert!(
        !dump.exists(),
        "bwrap must never be exec'd for a refused scope"
    );
}
