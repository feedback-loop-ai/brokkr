use super::*;

/// The workspace-write profile `@deepseek-ai/dsh-sandbox-local` composes,
/// mirrored from `bwrapProfileArgs` as measured on 0.1.2-rc.1: a read-only
/// host root, a fresh `/dev` and `/proc`, an ephemeral `/tmp`, and the
/// session workspace bound read-write. The runner under test receives
/// exactly this shape.
fn dsh_workspace_write_profile(workspace: &Path) -> Vec<String> {
    let workspace = workspace.to_string_lossy().into_owned();
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
        &workspace,
        &workspace,
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn linked_scope() -> GitScope {
    GitScope {
        workspace: PathBuf::from("/work/wt"),
        git_dir: PathBuf::from("/main/.git/worktrees/wt"),
        common_dir: PathBuf::from("/main/.git"),
    }
}

fn runner_args(scope: &GitScope, profile: &[String], command: &[&str]) -> Vec<String> {
    let mut args = vec![
        "--workspace".to_string(),
        scope.workspace.to_string_lossy().into_owned(),
        "--git-dir".to_string(),
        scope.git_dir.to_string_lossy().into_owned(),
        "--common-dir".to_string(),
        scope.common_dir.to_string_lossy().into_owned(),
    ];
    args.extend(profile.iter().cloned());
    args.push("--".to_string());
    args.extend(command.iter().map(|part| part.to_string()));
    args
}

#[test]
fn the_runner_adds_the_scoped_git_binds_and_nothing_wider() {
    let scope = linked_scope();
    let profile = dsh_workspace_write_profile(&scope.workspace);
    let args = runner_args(&scope, &profile, &["bash", "-lc", "git add -A"]);
    let argv = runner_argv(&args).unwrap();
    assert_eq!(argv[0], "bwrap");
    let text = argv.join(" ");

    // The per-worktree directory and the shared write set a commit needs.
    assert!(text.contains("--bind /main/.git/worktrees/wt /main/.git/worktrees/wt"));
    for name in ["objects", "refs", "logs"] {
        assert!(
            text.contains(&format!("--bind-try /main/.git/{name} /main/.git/{name}")),
            "{name}: {text}"
        );
    }
    assert!(text.contains("--bind-try /main/.git/packed-refs /main/.git/packed-refs"));

    // Hooks are an empty tmpfs and config is read-only, per-worktree and shared.
    assert!(text.contains("--tmpfs /main/.git/worktrees/wt/hooks"));
    assert!(text.contains("--tmpfs /main/.git/hooks"));
    assert!(text.contains("--ro-bind-try /main/.git/config /main/.git/config"));
    assert!(text
        .contains("--ro-bind-try /main/.git/worktrees/wt/config /main/.git/worktrees/wt/config"));

    // The whole shared `.git` is never writable, and no sibling is named.
    assert!(
        !text.contains("--bind /main/.git /main/.git"),
        "the whole common dir must not be mounted writable: {text}"
    );
    assert!(!text.contains("worktrees/sibling"), "{text}");

    // The profile and the command travel verbatim, separated by the bare `--`.
    assert!(text.contains("--ro-bind / / --dev /dev"));
    assert!(text.ends_with("-- bash -lc git add -A"), "{text}");
}

#[test]
fn a_read_only_profile_and_a_primary_checkout_get_no_scoped_binds() {
    let scope = linked_scope();

    // `read-only`: no workspace bind at all, so the runner adds nothing.
    let read_only: Vec<String> = [
        "--ro-bind",
        "/",
        "/",
        "--dev",
        "/dev",
        "--unshare-pid",
        "--proc",
        "/proc",
        "--die-with-parent",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let argv = runner_argv(&runner_args(&scope, &read_only, &["true"])).unwrap();
    assert_eq!(
        argv,
        [
            "bwrap",
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--unshare-pid",
            "--proc",
            "/proc",
            "--die-with-parent",
            "--",
            "true"
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>()
    );

    // `--bind-try` is the same workspace grant when the source exists.
    let mut try_profile = read_only.clone();
    try_profile.extend([
        "--bind-try".to_string(),
        scope.workspace.to_string_lossy().into_owned(),
        scope.workspace.to_string_lossy().into_owned(),
    ]);
    let argv = runner_argv(&runner_args(&scope, &try_profile, &["true"])).unwrap();
    assert!(
        argv.join(" ").contains("--bind-try /main/.git/objects"),
        "a --bind-try workspace is writable too"
    );

    // A bind that is not the workspace is stepped over, and the scan
    // continues to the workspace grant that follows it.
    let mut mixed = read_only.clone();
    mixed.extend([
        "--bind".to_string(),
        "/other".to_string(),
        "/other".to_string(),
    ]);
    mixed.extend([
        "--bind".to_string(),
        scope.workspace.to_string_lossy().into_owned(),
        scope.workspace.to_string_lossy().into_owned(),
    ]);
    let argv = runner_argv(&runner_args(&scope, &mixed, &["true"])).unwrap();
    assert!(
        argv.join(" ").contains("--bind-try /main/.git/objects"),
        "an unrelated bind does not end the scan"
    );

    // A primary checkout: the git directory sits under the workspace the
    // provider already made writable, so no scoped bind is added.
    let primary = GitScope {
        workspace: PathBuf::from("/repo"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    let profile = dsh_workspace_write_profile(&primary.workspace);
    let argv = runner_argv(&runner_args(&primary, &profile, &["true"])).unwrap();
    let text = argv.join(" ");
    assert!(!text.contains("--tmpfs /repo/.git/hooks"), "{text}");
    assert!(!text.contains("--bind-try /repo/.git/objects"), "{text}");
    assert!(text.contains("--bind /repo /repo"), "{text}");
}

#[test]
fn a_per_worktree_dir_inside_the_workspace_is_not_bound_twice() {
    // `--separate-git-dir` can put the per-worktree directory under the
    // workspace while the shared directory stays outside it: the common
    // subdirectories are still bound, the per-worktree one is not.
    let scope = GitScope {
        workspace: PathBuf::from("/work/wt"),
        git_dir: PathBuf::from("/work/wt/.git-meta"),
        common_dir: PathBuf::from("/shared/.git"),
    };
    let profile = dsh_workspace_write_profile(&scope.workspace);
    let argv = runner_argv(&runner_args(&scope, &profile, &["true"])).unwrap();
    let text = argv.join(" ");
    assert!(!text.contains("--bind /work/wt/.git-meta"), "{text}");
    assert!(text.contains("--bind-try /shared/.git/objects /shared/.git/objects"));
}

#[test]
fn the_runner_refuses_a_malformed_scope_or_profile() {
    let scope = linked_scope();
    let profile = dsh_workspace_write_profile(&scope.workspace);
    let s = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };

    assert!(runner_argv(&s(&["--workspace"]))
        .unwrap_err()
        .contains("--workspace needs a path"));
    assert!(runner_argv(&s(&[
        "--workspace",
        "/w",
        "--git-dir",
        "/g",
        "--common-dir",
        "/c",
        "--workspace",
        "/x"
    ]))
    .unwrap_err()
    .contains("--workspace given twice"));
    assert!(runner_argv(&s(&["--git-dir", "/g"]))
        .unwrap_err()
        .contains("--workspace is required"));
    assert!(runner_argv(&s(&["--workspace", "/w"]))
        .unwrap_err()
        .contains("--git-dir is required"));
    assert!(runner_argv(&s(&["--workspace", "/w", "--git-dir", "/g"]))
        .unwrap_err()
        .contains("--common-dir is required"));

    // A profile with no command separator, and a profile that does not
    // stand for the boundary the runner knows.
    let mut no_separator = runner_args(&scope, &profile, &[]);
    no_separator.truncate(no_separator.len() - 1);
    assert!(runner_argv(&no_separator)
        .unwrap_err()
        .contains("no `--` before the command"));
    let bad_profile = vec!["--ro-bind".to_string(), "/tmp".to_string()];
    let bad = runner_args(&scope, &bad_profile, &["true"]);
    assert!(runner_argv(&bad).unwrap_err().contains("--ro-bind / /"));
}

#[test]
fn the_overlay_row_quotes_every_path_and_refuses_a_line_break() {
    let scope = GitScope {
        workspace: PathBuf::from("/work/it's a path"),
        git_dir: PathBuf::from("/main/.git/worktrees/wt"),
        common_dir: PathBuf::from("/main/.git"),
    };
    let row = sandbox_row("/opt/brokkr's bin/brokkr", &scope).unwrap();
    assert!(row.contains("- id: sandbox\n"), "{row}");
    assert!(row.contains("    runnerCommand:\n"), "{row}");
    assert!(
        row.contains("      - '/opt/brokkr''s bin/brokkr'\n"),
        "{row}"
    );
    assert!(row.contains(&format!("      - '{RUNNER_VERB}'\n")), "{row}");
    assert!(row.contains("      - '--workspace'\n"), "{row}");
    assert!(row.contains("      - '/work/it''s a path'\n"), "{row}");
    assert!(row.contains("      - '--git-dir'\n"), "{row}");
    assert!(row.contains("      - '--common-dir'\n"), "{row}");
    assert!(
        row.contains(&format!("      - '{RUNNER_FAILURE_SIGNATURE}'\n")),
        "{row}"
    );
    assert!(row.contains("      - 'bwrap: '\n"), "{row}");

    let broken = GitScope {
        workspace: PathBuf::from("/work\nline"),
        ..scope.clone()
    };
    assert!(sandbox_row("brokkr", &broken)
        .unwrap_err()
        .contains("spans more than one line"));
    // The program itself is a scalar too, and gets the same refusal.
    assert!(sandbox_row("/opt/brokkr\nline", &scope)
        .unwrap_err()
        .contains("spans more than one line"));
    assert!(sandbox_row("/opt/brokkr\rline", &scope)
        .unwrap_err()
        .contains("spans more than one line"));
}

#[test]
fn a_bwrap_that_is_not_there_is_not_usable() {
    let missing = std::path::Path::new("/nonexistent/bwrap");
    assert!(!bwrap_usable(missing));
    let refusal = require_usable_bwrap(missing).unwrap_err();
    assert!(
        refusal.contains("cannot build the empty-root namespace"),
        "{refusal}"
    );
}

/// The boundary, for real: Linux with bubblewrap on PATH, which is where
/// dsh's own sandbox runs and the only place this runner is claimed.
/// The provider's profile confines the worktree; the runner widens it by
/// exactly the scoped git metadata, so a linked worktree commits, the
/// parent and sibling stay unreachable, and hooks and config cannot be
/// written.
#[cfg(target_os = "linux")]
#[test]
fn a_linked_worktree_commits_under_the_dsh_profile_and_the_boundary_holds() {
    if std::env::var_os(crate::hands::HANDS_BOX_ENV).is_some() {
        eprintln!("skipped: this environment is already a box");
        return;
    }
    let Ok(bwrap) = crate::hands::require_bwrap() else {
        eprintln!("skipped: no bubblewrap on PATH");
        return;
    };
    if !bwrap_usable(&bwrap) {
        eprintln!("skipped: this environment cannot create a bubblewrap namespace");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    // A nested path with a space, so the argv carries what a host path
    // really can carry.
    let main = dir.path().join("main repo");
    std::fs::create_dir_all(&main).unwrap();
    let git = |cwd: &Path, args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
            .env("GIT_CONFIG_VALUE_0", "false")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&main, &["init", "-q", "-b", "main"]);
    git(&main, &["config", "user.name", "Host Operator"]);
    git(&main, &["config", "user.email", "host@example.invalid"]);
    git(&main, &["config", "commit.gpgsign", "true"]);
    git(&main, &["config", "gpg.program", "/nonexistent/gpg"]);
    std::fs::write(main.join("a.txt"), "a\n").unwrap();
    git(&main, &["add", "-A"]);
    git(&main, &["commit", "-q", "--no-gpg-sign", "-m", "base"]);

    let worktree = dir.path().join("wt with space");
    let sibling = dir.path().join("sibling");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            worktree.to_str().unwrap(),
            "-b",
            "slice",
        ],
    );
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            sibling.to_str().unwrap(),
            "-b",
            "sibling",
        ],
    );
    std::fs::write(main.join("parent-sentinel"), "parent\n").unwrap();
    std::fs::write(sibling.join("sibling-sentinel"), "sibling\n").unwrap();

    let facts = crate::hands::git_facts(&worktree);
    let scope = GitScope {
        workspace: worktree.clone(),
        git_dir: facts.git_dir.clone().unwrap(),
        common_dir: facts.common_dir.clone().unwrap(),
    };
    let profile = dsh_workspace_write_profile(&worktree);
    let script = format!(
        "set -e\n\
         echo b > b.txt\n\
         git add b.txt\n\
         git commit -q -m boxed\n\
         git log -1 --format='%s %G?'\n\
         if echo x > '{parent}' 2>/dev/null; then echo PARENT_WRITABLE; else echo PARENT_READONLY; fi\n\
         if echo x > '{sibling_sentinel}' 2>/dev/null; then echo SIBLING_WRITABLE; else echo SIBLING_READONLY; fi\n\
         if echo x > '{sibling_index}' 2>/dev/null; then echo SIBLING_GIT_WRITABLE; else echo SIBLING_GIT_READONLY; fi\n\
         echo \"hooks=$(ls '{common}/hooks' | wc -l)\"\n\
         echo x > '{hook}' 2>/dev/null || true\n\
         if git config --local core.hooksPath /evil 2>/dev/null; then echo CONFIG_WRITABLE; else echo CONFIG_READONLY; fi\n",
        parent = main.join("parent-sentinel").display(),
        sibling_sentinel = sibling.join("sibling-sentinel").display(),
        sibling_index = main.join(".git/worktrees/sibling/HEAD").display(),
        hook = main.join(".git/hooks/post-checkout").display(),
        common = main.join(".git").display(),
    );
    let args = runner_args(&scope, &profile, &["bash", "-lc", &script]);
    let argv = runner_argv(&args).unwrap();
    let run = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&worktree)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "{stdout}\n{stderr}");

    // The commit is real, reached the host's shared store, and is unsigned.
    assert!(stdout.contains("boxed N"), "{stdout}");
    assert_eq!(git(&worktree, &["log", "-1", "--format=%s"]), "boxed");
    assert_eq!(
        git(&worktree, &["log", "-1", "--format=%an <%ae>"]),
        "Host Operator <host@example.invalid>"
    );

    // The scoped boundary held everywhere it promised. The hooks dir is
    // an empty tmpfs, so a write lands there and never on the host.
    assert!(stdout.contains("PARENT_READONLY"), "{stdout}");
    assert!(stdout.contains("SIBLING_READONLY"), "{stdout}");
    assert!(stdout.contains("SIBLING_GIT_READONLY"), "{stdout}");
    assert!(stdout.contains("hooks=0"), "{stdout}");
    assert!(stdout.contains("CONFIG_READONLY"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(main.join("parent-sentinel")).unwrap(),
        "parent\n"
    );
    assert_eq!(
        std::fs::read_to_string(sibling.join("sibling-sentinel")).unwrap(),
        "sibling\n"
    );
    assert!(!main.join(".git/hooks/post-checkout").exists());
    assert!(!std::fs::read_to_string(main.join(".git/config"))
        .unwrap()
        .contains("evil"));

    // An ordinary standalone repository still works under the runner: no
    // scoped bind is added and the workspace bind covers its `.git`.
    let standalone = dir.path().join("standalone");
    std::fs::create_dir_all(&standalone).unwrap();
    git(&standalone, &["init", "-q", "-b", "main"]);
    git(&standalone, &["config", "user.name", "Host Operator"]);
    git(
        &standalone,
        &["config", "user.email", "host@example.invalid"],
    );
    let facts = crate::hands::git_facts(&standalone);
    let scope = GitScope {
        workspace: standalone.clone(),
        git_dir: facts.git_dir.clone().unwrap(),
        common_dir: facts.common_dir.clone().unwrap(),
    };
    let profile = dsh_workspace_write_profile(&standalone);
    let script =
        "echo s > s.txt && git add s.txt && git commit -q -m standalone && git log -1 --format=%s";
    let argv = runner_argv(&runner_args(&scope, &profile, &["bash", "-lc", script])).unwrap();
    let run = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&standalone)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(String::from_utf8_lossy(&run.stdout).contains("standalone"));
}
