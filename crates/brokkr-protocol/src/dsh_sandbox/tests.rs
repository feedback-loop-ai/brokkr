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

/// The absolute bubblewrap the driver probed and the runner must exec.
const BWRAP: &str = "/usr/bin/bwrap";

fn linked_scope() -> GitScope {
    GitScope {
        workspace: PathBuf::from("/work/wt"),
        git_dir: PathBuf::from("/main/.git/worktrees/wt"),
        common_dir: PathBuf::from("/main/.git"),
    }
}

fn runner_args(scope: &GitScope, profile: &[String], command: &[&str]) -> Vec<String> {
    runner_args_for(Path::new(BWRAP), scope, profile, command)
}

/// The runner argv with a chosen bubblewrap, so the behavioral test can
/// exec the binary `require_bwrap` actually resolved rather than the
/// spelling the unit tests assert.
fn runner_args_for(
    bwrap: &Path,
    scope: &GitScope,
    profile: &[String],
    command: &[&str],
) -> Vec<String> {
    let mut args = vec![
        "--workspace".to_string(),
        scope.workspace.to_string_lossy().into_owned(),
        "--git-dir".to_string(),
        scope.git_dir.to_string_lossy().into_owned(),
        "--common-dir".to_string(),
        scope.common_dir.to_string_lossy().into_owned(),
        "--bwrap".to_string(),
        bwrap.to_string_lossy().into_owned(),
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
    assert_eq!(argv[0], BWRAP);
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
    // `config.worktree` is masked, not ro-bound-try: the per-worktree file
    // is normally absent, and the writable directory around it would let a
    // boxed command create one the host then reads.
    assert!(text.contains("--ro-bind /dev/null /main/.git/worktrees/wt/config.worktree"));
    assert!(!text.contains("--ro-bind-try /main/.git/worktrees/wt/config.worktree"));

    // The worktree's pointers back to the shared repository are read-only:
    // a boxed rewrite would redirect git at a config it wrote.
    assert!(text.contains(
        "--ro-bind-try /main/.git/worktrees/wt/commondir /main/.git/worktrees/wt/commondir"
    ));
    assert!(text
        .contains("--ro-bind-try /main/.git/worktrees/wt/gitdir /main/.git/worktrees/wt/gitdir"));

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
            BWRAP,
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
fn a_profile_that_does_not_root_at_the_read_only_host_root_is_refused() {
    // The shape guard has three arms, and each one alone refuses.
    let scope = linked_scope();
    for bad in [
        ["--bind", "/", "/", "true"],
        ["--ro-bind", "/tmp", "/", "true"],
        ["--ro-bind", "/", "/tmp", "true"],
    ] {
        let profile: Vec<String> = bad.iter().map(|part| part.to_string()).collect();
        let refused = runner_argv(&runner_args(&scope, &profile, &["true"])).unwrap_err();
        assert!(refused.contains("--ro-bind / /"), "{bad:?}: {refused}");
    }
}

#[test]
fn a_bind_whose_source_is_not_its_destination_is_not_the_workspace_grant() {
    let scope = linked_scope();
    let mut profile: Vec<String> = ["--ro-bind", "/", "/", "--dev", "/dev"]
        .into_iter()
        .map(str::to_string)
        .collect();
    // The destination names the workspace, but the source does not: this is
    // not the `--bind <workspace> <workspace>` grant `workspace-write`
    // composes, so no scoped bind may ride on it.
    profile.extend([
        "--bind".to_string(),
        "/src".to_string(),
        scope.workspace.to_string_lossy().into_owned(),
    ]);
    let argv = runner_argv(&runner_args(&scope, &profile, &["true"])).unwrap();
    assert!(
        !argv.join(" ").contains("--bind-try /main/.git/objects"),
        "a mismatched source/destination pair is not the workspace grant"
    );
}

/// Decision 0053 addendum: the two layouts the runner refuses instead of
/// widening the boundary, and the layouts it serves.
#[test]
fn a_shared_git_directory_and_a_redirected_worktree_are_refused() {
    // The common directory already sits inside the writable root: nothing
    // to add, and never a refusal.
    let primary = GitScope {
        workspace: PathBuf::from("/repo"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    assert!(scope_refusal(&primary).is_none());

    // The git directory IS the shared repository: a subdirectory of a
    // primary checkout, a `--separate-git-dir` checkout, or a `.git` file
    // redirected at the parent. Serving it would need the whole shared
    // `.git` writable.
    let subdir = GitScope {
        workspace: PathBuf::from("/repo/src"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    let refused = scope_refusal(&subdir).unwrap();
    assert!(
        refused.contains("shared repository's own git directory"),
        "{refused}"
    );
    assert!(refused.contains("linked `git worktree`"), "{refused}");

    // A per-worktree directory whose `gitdir` pointer names another
    // worktree is a redirect: git follows a rewritten `.git` file without
    // checking the back-pointer, so this is refused rather than bound.
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("wt");
    let git_dir = dir.path().join("main/.git/worktrees/wt");
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join(".git"),
        format!("gitdir: {}\n", git_dir.display()),
    )
    .unwrap();
    let scope = GitScope {
        workspace: workspace.clone(),
        git_dir: git_dir.clone(),
        common_dir: dir.path().join("main/.git"),
    };
    // No pointer file: nothing to compare, and no refusal on this evidence.
    assert!(scope_refusal(&scope).is_none());
    // An empty pointer says nothing either.
    std::fs::write(git_dir.join("gitdir"), "\n").unwrap();
    assert!(scope_refusal(&scope).is_none());
    // This workspace's own `.git` file, spelled absolute and relative.
    std::fs::write(
        git_dir.join("gitdir"),
        format!("{}\n", workspace.join(".git").display()),
    )
    .unwrap();
    assert!(scope_refusal(&scope).is_none());
    std::fs::write(git_dir.join("gitdir"), "../../../../wt/.git").unwrap();
    assert!(scope_refusal(&scope).is_none());
    // Another worktree's `.git` file is the redirect.
    std::fs::write(
        git_dir.join("gitdir"),
        format!("{}\n", dir.path().join("sib/.git").display()),
    )
    .unwrap();
    let refused = scope_refusal(&scope).unwrap();
    assert!(refused.contains("records"), "{refused}");
    assert!(refused.contains("another worktree's metadata"), "{refused}");
}

#[test]
fn the_runner_refuses_a_scope_that_would_need_the_whole_shared_git() {
    let scope = GitScope {
        workspace: PathBuf::from("/repo/src"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    let profile = dsh_workspace_write_profile(&scope.workspace);
    let refused = runner_argv(&runner_args(&scope, &profile, &["true"])).unwrap_err();
    assert!(
        refused.contains("shared repository's own git directory"),
        "{refused}"
    );
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
    assert!(runner_argv(&s(&[
        "--workspace",
        "/w",
        "--git-dir",
        "/g",
        "--common-dir",
        "/c"
    ]))
    .unwrap_err()
    .contains("--bwrap is required"));
    assert!(runner_argv(&s(&[
        "--workspace",
        "/w",
        "--git-dir",
        "/g",
        "--common-dir",
        "/c",
        "--bwrap",
        "/bin/bwrap",
        "--bwrap",
        "/other/bwrap"
    ]))
    .unwrap_err()
    .contains("--bwrap given twice"));
    // The runner execs an absolute bubblewrap; a relative one would be
    // resolved by a working directory the seat can move.
    assert!(runner_argv(&s(&[
        "--workspace",
        "/w",
        "--git-dir",
        "/g",
        "--common-dir",
        "/c",
        "--bwrap",
        "bwrap"
    ]))
    .unwrap_err()
    .contains("is not an absolute path"));

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
    let row = sandbox_row("/opt/brokkr's bin/brokkr", Path::new("/opt/bwrap"), &scope).unwrap();
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
    // The resolved bubblewrap is the runner's fourth trusted path.
    assert!(row.contains("      - '--bwrap'\n"), "{row}");
    assert!(row.contains("      - '/opt/bwrap'\n"), "{row}");
    assert!(
        row.contains(&format!("      - '{RUNNER_FAILURE_SIGNATURE}'\n")),
        "{row}"
    );
    assert!(row.contains("      - 'bwrap: '\n"), "{row}");

    let broken = GitScope {
        workspace: PathBuf::from("/work\nline"),
        ..scope.clone()
    };
    assert!(sandbox_row("brokkr", Path::new("/opt/bwrap"), &broken)
        .unwrap_err()
        .contains("spans more than one line"));
    // The program itself is a scalar too, and gets the same refusal.
    assert!(
        sandbox_row("/opt/brokkr\nline", Path::new("/opt/bwrap"), &scope)
            .unwrap_err()
            .contains("spans more than one line")
    );
    assert!(
        sandbox_row("/opt/brokkr\rline", Path::new("/opt/bwrap"), &scope)
            .unwrap_err()
            .contains("spans more than one line")
    );
    // So is the bubblewrap path.
    assert!(sandbox_row("brokkr", Path::new("/opt/bwrap\nline"), &scope)
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

/// A fixture root dsh's own profile cannot shadow. The workspace-write
/// profile mounts a fresh tmpfs over `/tmp`, so a fixture under `/tmp`
/// is not the host directory the assertions read back: the box would
/// write to the tmpfs, the host paths would stay untouched, and the
/// test would pass for a reason that has nothing to do with the
/// boundary. The cargo target directory (beside this test binary) is
/// outside that tmpfs and outside the session workspace. `None` when
/// this binary has no target directory outside `/tmp` to root the
/// fixture in.
fn fixture_root() -> Option<tempfile::TempDir> {
    let base = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if base.starts_with("/tmp") {
        return None;
    }
    tempfile::Builder::new()
        .prefix("brokkr-dsh-sandbox-")
        .tempdir_in(base)
        .ok()
}

/// The boundary, for real: Linux with bubblewrap, which is where dsh's
/// own sandbox runs and the only place this runner is claimed. The
/// provider's profile confines the worktree; the runner widens it by
/// exactly the scoped git metadata, so a linked worktree commits, the
/// parent and sibling stay unreachable, and hooks, config and the
/// worktree's `commondir`/`gitdir` pointers cannot be written. The
/// assertions read the HOST back: a marker printed inside the box would
/// only prove what the box believes, not what landed on the host.
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
    // The runner refuses a relative `--bwrap`, so the test hands it the
    // resolved absolute path exactly as the driver does.
    let bwrap = std::fs::canonicalize(&bwrap).unwrap_or(bwrap);
    if !bwrap_usable(&bwrap) {
        eprintln!("skipped: this environment cannot create a bubblewrap namespace");
        return;
    }
    let Some(dir) = fixture_root() else {
        eprintln!("skipped: no fixture root outside the profile's `/tmp` tmpfs");
        return;
    };

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
    // A repository that has run sparse-checkout carries this extension, so
    // a per-worktree `config.worktree` is honoured by the host's git.
    git(&main, &["config", "extensions.worktreeConfig", "true"]);
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
    let git_dir = facts.git_dir.clone().unwrap();
    let common_dir = facts.common_dir.clone().unwrap();
    let scope = GitScope {
        workspace: worktree.clone(),
        git_dir: git_dir.clone(),
        common_dir: common_dir.clone(),
    };
    // Read the pointers the box must not move BEFORE the box runs, so a
    // rewrite is caught by comparing host bytes, not by trusting a
    // marker.
    let commondir_file = git_dir.join("commondir");
    let gitdir_file = git_dir.join("gitdir");
    let commondir_before = std::fs::read_to_string(&commondir_file).unwrap();
    let gitdir_before = std::fs::read_to_string(&gitdir_file).unwrap();

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
         if git config --local core.hooksPath /evil 2>/dev/null; then echo CONFIG_WRITABLE; else echo CONFIG_READONLY; fi\n\
         if git config --worktree core.hooksPath /evil 2>/dev/null; then echo WORKTREE_CONFIG_WRITABLE; else echo WORKTREE_CONFIG_READONLY; fi\n\
         echo evil > '{worktree_config}' 2>/dev/null || true\n\
         if echo evil > '{sibling_config}' 2>/dev/null; then echo SIBLING_CONFIG_WRITABLE; else echo SIBLING_CONFIG_READONLY; fi\n\
         echo evil > '{scratch}'\n\
         if mv '{scratch}' '{commondir}' 2>/dev/null; then echo COMMONDIR_REPLACED; else echo COMMONDIR_INTACT; fi\n\
         rm -f '{scratch}'\n\
         if echo x > '{commondir}' 2>/dev/null; then echo COMMONDIR_WRITABLE; else echo COMMONDIR_READONLY; fi\n\
         if rm -f '{commondir}' 2>/dev/null; then echo COMMONDIR_UNLINKED; else echo COMMONDIR_STAYS; fi\n\
         if echo x > '{gitdir}' 2>/dev/null; then echo GITDIR_WRITABLE; else echo GITDIR_READONLY; fi\n",
        parent = main.join("parent-sentinel").display(),
        sibling_sentinel = sibling.join("sibling-sentinel").display(),
        sibling_index = main.join(".git/worktrees/sibling/HEAD").display(),
        hook = main.join(".git/hooks/post-checkout").display(),
        common = main.join(".git").display(),
        scratch = git_dir.join("probe-scratch").display(),
        commondir = commondir_file.display(),
        gitdir = gitdir_file.display(),
        worktree_config = git_dir.join("config.worktree").display(),
        sibling_config = main.join(".git/worktrees/sibling/config.worktree").display(),
    );
    let args = runner_args_for(&bwrap, &scope, &profile, &["bash", "-lc", &script]);
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
    // The per-worktree config the host honours is masked, not merely
    // ro-bound-try: the box can neither write it nor create it, and a
    // sibling's is outside the write set entirely.
    assert!(stdout.contains("WORKTREE_CONFIG_READONLY"), "{stdout}");
    assert!(stdout.contains("SIBLING_CONFIG_READONLY"), "{stdout}");
    assert!(!stdout.contains("WORKTREE_CONFIG_WRITABLE"), "{stdout}");
    assert!(stdout.contains("COMMONDIR_INTACT"), "{stdout}");
    assert!(stdout.contains("COMMONDIR_READONLY"), "{stdout}");
    assert!(stdout.contains("COMMONDIR_STAYS"), "{stdout}");
    assert!(stdout.contains("GITDIR_READONLY"), "{stdout}");
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
    // The mask's mount point is at worst an empty file on the host; the
    // box wrote nothing into it.
    let masked = std::fs::read_to_string(git_dir.join("config.worktree")).unwrap_or_default();
    assert!(!masked.contains("evil"), "{masked}");
    assert!(
        !std::fs::read_to_string(main.join(".git/worktrees/sibling/config.worktree"))
            .unwrap_or_default()
            .contains("evil")
    );
    // The worktree's pointers are byte-for-byte what the host wrote, and
    // the host's git still resolves the real common directory: nothing
    // the box wrote redirected it at a config in the workspace.
    assert_eq!(
        std::fs::read_to_string(&commondir_file).unwrap(),
        commondir_before
    );
    assert_eq!(
        std::fs::read_to_string(&gitdir_file).unwrap(),
        gitdir_before
    );
    assert_eq!(
        git(
            &worktree,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"]
        ),
        common_dir.to_string_lossy()
    );

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
    let argv = runner_argv(&runner_args_for(
        &bwrap,
        &scope,
        &profile,
        &["bash", "-lc", script],
    ))
    .unwrap();
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
