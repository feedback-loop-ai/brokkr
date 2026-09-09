//! Decision 0054: `brokkr dsh-sandbox-runner` is the bwrap-compatible
//! runner `brokkr driver dsh` points dsh's sandbox provider at. It is
//! dispatched before clap because dsh hands it bubblewrap's own argv,
//! including the bare `--`; these tests drive the real binary and a fake
//! `bwrap`, so the argv hand-off is proven end to end without a model
//! loop. The bubblewrap to exec, the private git store to bind and the
//! staged files to mount are trusted argv paths, not lookups: the
//! boundary is what the driver measured.
//!
//! The scope checks read the host, so these fixtures are real linked
//! worktree layouts on disk rather than the invented paths an argv test
//! could otherwise get away with.

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

/// The administrative layout `git worktree add` writes, by hand: the
/// per-worktree directory under `<common>/worktrees/<name>` with its
/// `gitdir` back-pointer, the worktree's own `.git` file, and the two
/// directories the driver stages outside the workspace — the private
/// common directory the seat writes, and the read-only files the runner
/// mounts.
struct Layout {
    dir: tempfile::TempDir,
    workspace: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
    store: PathBuf,
    trusted: PathBuf,
}

impl Layout {
    fn linked() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let common_dir = dir.path().join("main/.git");
        let git_dir = common_dir.join("worktrees/wt");
        let workspace = dir.path().join("wt");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(
            workspace.join(".git"),
            format!("gitdir: {}\n", git_dir.display()),
        )
        .unwrap();
        std::fs::write(
            git_dir.join("gitdir"),
            format!("{}\n", workspace.join(".git").display()),
        )
        .unwrap();
        std::fs::write(git_dir.join("commondir"), "../..\n").unwrap();
        let store = dir.path().join("store");
        let trusted = dir.path().join("trusted");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::create_dir_all(&trusted).unwrap();
        std::fs::write(trusted.join("config-mask"), "").unwrap();
        std::fs::write(trusted.join("commondir"), format!("{}\n", store.display())).unwrap();
        std::fs::write(
            trusted.join("alternates"),
            format!("{}\n", common_dir.join("objects").display()),
        )
        .unwrap();
        Layout {
            workspace,
            git_dir,
            common_dir,
            store,
            trusted,
            dir,
        }
    }

    /// The runner's six trusted flags, as the sandbox row spells them.
    fn flags(&self, bwrap: &Path) -> Vec<String> {
        [
            "--workspace",
            self.workspace.to_str().unwrap(),
            "--git-dir",
            self.git_dir.to_str().unwrap(),
            "--common-dir",
            self.common_dir.to_str().unwrap(),
            "--bwrap",
            bwrap.to_str().unwrap(),
            "--store",
            self.store.to_str().unwrap(),
            "--trusted",
            self.trusted.to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }
}

/// `dsh-sandbox-runner`, the trusted flags, the profile and the command.
fn runner_args(layout: &Layout, bwrap: &Path, profile: &[&str], command: &[&str]) -> Vec<String> {
    let mut args = vec!["dsh-sandbox-runner".to_string()];
    args.extend(layout.flags(bwrap));
    args.extend(profile.iter().map(|part| part.to_string()));
    args.push("--".to_string());
    args.extend(command.iter().map(|part| part.to_string()));
    args
}

fn borrow(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

/// The workspace-write profile dsh composes, with the session workspace
/// spelled where the provider spells it.
fn profile_for(workspace: &Path) -> Vec<String> {
    [
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
        workspace.to_str().unwrap(),
        workspace.to_str().unwrap(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[cfg(unix)]
#[test]
fn the_runner_execs_bwrap_with_the_scoped_git_binds_and_the_command() {
    let layout = Layout::linked();
    let dump = layout.dir.path().join("argv");
    let bwrap = fake_bwrap(layout.dir.path());
    let profile = profile_for(&layout.workspace);
    let args = runner_args(
        &layout,
        &bwrap,
        &borrow(&profile),
        &["bash", "-lc", "git add -A"],
    );

    let run = brokkr(&borrow(&args), layout.dir.path(), Some(&dump));
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let seen = std::fs::read_to_string(&dump).unwrap();
    let git = layout.git_dir.display().to_string();
    let common = layout.common_dir.display().to_string();
    let store = layout.store.display().to_string();
    let mask = layout.trusted.join("config-mask").display().to_string();
    let pointer = layout.trusted.join("commondir").display().to_string();
    let alternates = layout.trusted.join("alternates").display().to_string();
    // Exactly two read-write mounts: the worktree's own administrative
    // directory, and the private common directory the seat writes.
    assert!(seen.contains(&format!("--bind\n{git}\n{git}\n")), "{seen}");
    assert!(
        seen.contains(&format!("--bind\n{store}\n{store}\n")),
        "{seen}"
    );
    // Nothing under the shared git directory is writable.
    for name in ["objects", "refs", "logs", "packed-refs"] {
        assert!(
            !seen.contains(&format!("--bind\n{common}/{name}\n")),
            "{seen}"
        );
        assert!(
            !seen.contains(&format!("--bind-try\n{common}/{name}\n")),
            "{seen}"
        );
    }
    // The hook path the seat's git uses is an empty tmpfs, and the shared
    // config and HEAD are read-only inside the private store.
    assert!(
        seen.contains(&format!("--tmpfs\n{store}/hooks\n")),
        "{seen}"
    );
    assert!(
        seen.contains(&format!("--ro-bind\n{common}/config\n{store}/config\n")),
        "{seen}"
    );
    assert!(
        seen.contains(&format!(
            "--ro-bind\n{alternates}\n{store}/objects/info/alternates\n"
        )),
        "{seen}"
    );
    // The per-worktree config paths are masked with the staged empty
    // regular file — never `--ro-bind-try`, which no-ops on the absent
    // file a linked worktree normally has, and never a device node, which
    // bubblewrap binds with MS_NODEV and git then calls fatal.
    assert!(
        seen.contains(&format!("--ro-bind\n{mask}\n{git}/config.worktree\n")),
        "{seen}"
    );
    assert!(
        seen.contains(&format!("--ro-bind\n{mask}\n{git}/config\n")),
        "{seen}"
    );
    assert!(!seen.contains("/dev/null"), "{seen}");
    // The redirect itself: the box's git reads a `commondir` naming the
    // private store, and cannot write over it.
    assert!(
        seen.contains(&format!("--ro-bind\n{pointer}\n{git}/commondir\n")),
        "{seen}"
    );
    assert!(seen.ends_with("--\nbash\n-lc\ngit add -A\n"), "{seen}");
}

#[cfg(unix)]
#[test]
fn the_runner_execs_the_absolute_bwrap_it_was_given_and_never_a_path_lookup() {
    let layout = Layout::linked();
    let chosen = layout.dir.path().join("chosen");
    std::fs::create_dir_all(&chosen).unwrap();
    let dump = layout.dir.path().join("argv");
    let recording = fake_bwrap(&chosen);

    // A decoy `bwrap` first on PATH: if the runner looked it up, this
    // would run and leave its marker.
    let decoy = layout.dir.path().join("decoy");
    std::fs::create_dir_all(&decoy).unwrap();
    let marker = layout.dir.path().join("decoy-ran");
    executable(
        &decoy,
        "bwrap",
        &format!("#!/bin/sh\n: > '{}'\nexit 7\n", marker.display()),
    );

    let workspace = layout.workspace.to_str().unwrap().to_string();
    let args = runner_args(
        &layout,
        &recording,
        &["--ro-bind", "/", "/", "--bind", &workspace, &workspace],
        &["true"],
    );
    let run = brokkr(&borrow(&args), &decoy, Some(&dump));
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
    let layout = Layout::linked();
    let workspace = layout.workspace.to_str().unwrap().to_string();

    // No `--` before the command: the profile is not one the runner knows.
    let mut args = vec!["dsh-sandbox-runner".to_string()];
    args.extend(layout.flags(Path::new("/bin/bwrap")));
    args.extend(["--ro-bind", "/", "/"].map(str::to_string));
    let run = brokkr(&borrow(&args), layout.dir.path(), None);
    assert_eq!(run.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("brokkr-dsh-sandbox-runner: "), "{stderr}");
    assert!(stderr.contains("no `--` before the command"), "{stderr}");

    // The profile is well formed but the named bubblewrap is not there:
    // the exec refusal carries the same runner-failure signature.
    let empty = tempfile::tempdir().unwrap();
    let args = runner_args(
        &layout,
        Path::new("/nonexistent/bwrap"),
        &["--ro-bind", "/", "/", "--bind", &workspace, &workspace],
        &["true"],
    );
    let run = brokkr(&borrow(&args), empty.path(), None);
    assert_eq!(run.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("brokkr-dsh-sandbox-runner: "), "{stderr}");

    // A relative bubblewrap is refused before any exec: the runner never
    // resolves one through a working directory the seat can move.
    let args = runner_args(
        &layout,
        Path::new("bwrap"),
        &["--ro-bind", "/", "/", "--bind", &workspace, &workspace],
        &["true"],
    );
    let run = brokkr(&borrow(&args), empty.path(), None);
    assert_eq!(run.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("is not an absolute path"), "{stderr}");
}

/// Decision 0054: the runner serves ONE layout, and refuses the rest with
/// the signature dsh classifies as a runner failure rather than mounting
/// them. A scope whose git directory IS the shared repository would need
/// the whole shared `.git` writable; a workspace-local git directory
/// naming an unrelated repository as its common directory would bind that
/// repository's objects and refs read-write.
#[cfg(unix)]
#[test]
fn the_runner_refuses_every_layout_that_is_not_a_linked_worktree() {
    let layout = Layout::linked();
    let dump = layout.dir.path().join("argv");
    let bwrap = fake_bwrap(layout.dir.path());
    let store = layout.store.to_str().unwrap();
    let trusted = layout.trusted.to_str().unwrap();

    let refuse = |workspace: &Path, git_dir: &Path, common_dir: &Path| {
        let workspace = workspace.to_str().unwrap();
        let run = brokkr(
            &[
                "dsh-sandbox-runner",
                "--workspace",
                workspace,
                "--git-dir",
                git_dir.to_str().unwrap(),
                "--common-dir",
                common_dir.to_str().unwrap(),
                "--bwrap",
                bwrap.to_str().unwrap(),
                "--store",
                store,
                "--trusted",
                trusted,
                "--ro-bind",
                "/",
                "/",
                "--bind",
                workspace,
                workspace,
                "--",
                "true",
            ],
            layout.dir.path(),
            Some(&dump),
        );
        assert_eq!(run.status.code(), Some(127));
        let stderr = String::from_utf8_lossy(&run.stderr).into_owned();
        assert!(stderr.contains("brokkr-dsh-sandbox-runner: "), "{stderr}");
        assert!(
            !dump.exists(),
            "bwrap must never be exec'd for a refused scope"
        );
        stderr
    };

    let shared = refuse(
        Path::new("/repo/src"),
        Path::new("/repo/.git"),
        Path::new("/repo/.git"),
    );
    assert!(
        shared.contains("shared repository's own git directory"),
        "{shared}"
    );

    // The redirect: a `.git` file and a fake administrative directory the
    // seat wrote inside its own workspace, naming another repository.
    let seat = layout.dir.path().join("seat");
    std::fs::create_dir_all(seat.join("fake")).unwrap();
    let redirected = refuse(&seat, &seat.join("fake"), &layout.common_dir);
    assert!(
        redirected.contains("administrative directories"),
        "{redirected}"
    );
}
