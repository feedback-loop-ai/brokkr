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

/// A linked worktree's administrative layout on disk, exactly as `git
/// worktree add` writes it: `<common>/worktrees/<name>` with a `gitdir`
/// back-pointer, and the worktree's own `.git` file naming it back. The
/// scope checks read the host, so a unit test has to build a host to be
/// worth anything. `git` itself builds the layout in the behavioral
/// proof; here it is written by hand so the argv tests stay cheap.
struct Layout {
    dir: tempfile::TempDir,
    scope: GitScope,
    mask: PathBuf,
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
        // Outside the workspace, as the driver stages it.
        let mask = dir.path().join("config-mask");
        std::fs::write(&mask, "").unwrap();
        Layout {
            scope: GitScope {
                workspace,
                git_dir,
                common_dir,
            },
            mask,
            dir,
        }
    }

    /// The back-pointer this worktree's administrative directory records.
    fn point_back_at(&self, target: &str) {
        std::fs::write(self.scope.git_dir.join("gitdir"), target).unwrap();
    }

    fn argv(&self, command: &[&str]) -> Result<Vec<String>, String> {
        let profile = dsh_workspace_write_profile(&self.scope.workspace);
        runner_argv(&runner_args(self, &profile, command))
    }
}

fn runner_args(layout: &Layout, profile: &[String], command: &[&str]) -> Vec<String> {
    runner_args_for(
        Path::new(BWRAP),
        &layout.scope,
        &layout.mask,
        profile,
        command,
    )
}

/// The runner argv with a chosen bubblewrap and mask, so the behavioral
/// test can exec the binary `require_bwrap` actually resolved rather than
/// the spelling the unit tests assert.
fn runner_args_for(
    bwrap: &Path,
    scope: &GitScope,
    mask: &Path,
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
        "--mask".to_string(),
        mask.to_string_lossy().into_owned(),
    ];
    args.extend(profile.iter().cloned());
    args.push("--".to_string());
    args.extend(command.iter().map(|part| part.to_string()));
    args
}

#[test]
fn the_runner_adds_the_scoped_git_binds_and_nothing_wider() {
    let layout = Layout::linked();
    let argv = layout.argv(&["bash", "-lc", "git add -A"]).unwrap();
    assert_eq!(argv[0], BWRAP);
    let text = argv.join(" ");
    let git = layout.scope.git_dir.display().to_string();
    let common = layout.scope.common_dir.display().to_string();
    let mask = layout.mask.display().to_string();

    // The per-worktree directory and the shared write set a commit needs.
    assert!(text.contains(&format!("--bind {git} {git}")), "{text}");
    for name in ["objects", "refs", "logs"] {
        assert!(
            text.contains(&format!("--bind-try {common}/{name} {common}/{name}")),
            "{name}: {text}"
        );
    }
    assert!(
        text.contains(&format!("--bind-try {common}/packed-refs")),
        "{text}"
    );

    // Hooks are an empty tmpfs, per-worktree and shared, and the shared
    // config is read-only.
    assert!(text.contains(&format!("--tmpfs {git}/hooks")), "{text}");
    assert!(text.contains(&format!("--tmpfs {common}/hooks")), "{text}");
    assert!(
        text.contains(&format!("--ro-bind-try {common}/config {common}/config")),
        "{text}"
    );

    // Both per-worktree config paths are masked with the staged empty
    // file, never with `--ro-bind-try` and never with a device node:
    // `-try` no-ops on the absent file a linked worktree normally has,
    // and the writable directory around it would let the box create one.
    assert!(
        text.contains(&format!("--ro-bind {mask} {git}/config.worktree")),
        "{text}"
    );
    assert!(
        text.contains(&format!("--ro-bind {mask} {git}/config")),
        "{text}"
    );
    assert!(!text.contains("/dev/null"), "{text}");
    assert!(
        !text.contains(&format!("--ro-bind-try {git}/config")),
        "{text}"
    );

    // The worktree's pointers back to the shared repository are read-only,
    // and hard: a `-try` would no-op on a missing source and leave the box
    // free to create the pointer it may not rewrite.
    assert!(
        text.contains(&format!("--ro-bind {git}/commondir {git}/commondir")),
        "{text}"
    );
    assert!(
        text.contains(&format!("--ro-bind {git}/gitdir {git}/gitdir")),
        "{text}"
    );

    // The whole shared `.git` is never writable, and no sibling is named.
    assert!(
        !text.contains(&format!("--bind {common} {common}")),
        "the whole common dir must not be mounted writable: {text}"
    );
    assert!(!text.contains("worktrees/sibling"), "{text}");

    // The profile and the command travel verbatim, separated by the bare `--`.
    assert!(text.contains("--ro-bind / / --dev /dev"));
    assert!(text.ends_with("-- bash -lc git add -A"), "{text}");
}

#[test]
fn a_read_only_profile_and_a_primary_checkout_get_no_scoped_binds() {
    let layout = Layout::linked();
    let common = layout.scope.common_dir.display().to_string();

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
    let argv = runner_argv(&runner_args(&layout, &read_only, &["true"])).unwrap();
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
        layout.scope.workspace.to_string_lossy().into_owned(),
        layout.scope.workspace.to_string_lossy().into_owned(),
    ]);
    let argv = runner_argv(&runner_args(&layout, &try_profile, &["true"])).unwrap();
    assert!(
        argv.join(" ")
            .contains(&format!("--bind-try {common}/objects")),
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
        layout.scope.workspace.to_string_lossy().into_owned(),
        layout.scope.workspace.to_string_lossy().into_owned(),
    ]);
    let argv = runner_argv(&runner_args(&layout, &mixed, &["true"])).unwrap();
    assert!(
        argv.join(" ")
            .contains(&format!("--bind-try {common}/objects")),
        "an unrelated bind does not end the scan"
    );

    // A primary checkout: the git directory sits under the workspace the
    // provider already made writable, so no scoped bind is added and no
    // layout check applies.
    let primary = GitScope {
        workspace: PathBuf::from("/repo"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    let profile = dsh_workspace_write_profile(&primary.workspace);
    let argv = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &primary,
        &layout.mask,
        &profile,
        &["true"],
    ))
    .unwrap();
    let text = argv.join(" ");
    assert!(!text.contains("--tmpfs /repo/.git/hooks"), "{text}");
    assert!(!text.contains("--bind-try /repo/.git/objects"), "{text}");
    assert!(text.contains("--bind /repo /repo"), "{text}");
}

#[test]
fn a_profile_that_does_not_root_at_the_read_only_host_root_is_refused() {
    // The shape guard has three arms, and each one alone refuses.
    let layout = Layout::linked();
    for bad in [
        ["--bind", "/", "/", "true"],
        ["--ro-bind", "/tmp", "/", "true"],
        ["--ro-bind", "/", "/tmp", "true"],
    ] {
        let profile: Vec<String> = bad.iter().map(|part| part.to_string()).collect();
        let refused = runner_argv(&runner_args(&layout, &profile, &["true"])).unwrap_err();
        assert!(refused.contains("--ro-bind / /"), "{bad:?}: {refused}");
    }
}

#[test]
fn a_bind_whose_source_is_not_its_destination_is_not_the_workspace_grant() {
    let layout = Layout::linked();
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
        layout.scope.workspace.to_string_lossy().into_owned(),
    ]);
    let argv = runner_argv(&runner_args(&layout, &profile, &["true"])).unwrap();
    assert!(
        !argv.join(" ").contains(
            &layout
                .scope
                .common_dir
                .join("objects")
                .display()
                .to_string()
        ),
        "a mismatched source/destination pair is not the workspace grant"
    );
}

/// Decision 0054's threat model: Git resolves both directories by
/// FOLLOWING the workspace's own `.git` file, which is a file the seat
/// can write. Every layout that is not the linked worktree Git itself
/// created is refused, and each refusal names its own cause.
#[test]
fn only_a_genuine_linked_worktree_is_served() {
    // The common directory already sits inside the writable root: nothing
    // to add, and never a refusal.
    let primary = GitScope {
        workspace: PathBuf::from("/repo"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    assert!(scope_refusal(&primary).is_none());

    // The layout git wrote is served.
    let layout = Layout::linked();
    assert!(scope_refusal(&layout.scope).is_none());
    // ... and so is the same back-pointer spelled relative.
    layout.point_back_at("../../../../wt/.git");
    assert!(scope_refusal(&layout.scope).is_none());

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
}

/// The redirect the review measured: a seat writes a `.git` file and a
/// fake administrative directory INSIDE its own workspace, names an
/// unrelated repository in `commondir`, and git then reports that
/// repository as the common directory. Nothing about that layout is a
/// linked worktree, and the scoped runner may never bind the victim's
/// object store and refs read-write on its word.
#[test]
fn a_workspace_local_git_directory_never_redirects_the_write_set() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("seat");
    let victim = dir.path().join("victim/.git");
    std::fs::create_dir_all(workspace.join("fake")).unwrap();
    std::fs::create_dir_all(&victim).unwrap();
    let scope = GitScope {
        workspace: workspace.clone(),
        git_dir: workspace.join("fake"),
        common_dir: victim.clone(),
    };
    let refused = scope_refusal(&scope).unwrap();
    assert!(refused.contains("administrative directories"), "{refused}");
    assert!(refused.contains("worktrees/<name>"), "{refused}");
    // A git directory with no parent at all is the same refusal, not a panic.
    let rootish = GitScope {
        git_dir: PathBuf::from("/"),
        ..scope.clone()
    };
    assert!(scope_refusal(&rootish)
        .unwrap()
        .contains("administrative directories"));

    // The runner is the last gate, and it refuses the same scope rather
    // than emitting a single bind for it.
    let profile = dsh_workspace_write_profile(&workspace);
    let mask = dir.path().join("mask");
    std::fs::write(&mask, "").unwrap();
    let refused = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &scope,
        &mask,
        &profile,
        &["true"],
    ))
    .unwrap_err();
    assert!(refused.contains("administrative directories"), "{refused}");
}

/// The topology check alone is not enough: a workspace `.git` file may
/// name a REAL administrative directory that belongs to another
/// worktree — of this repository or of another one. The back-pointer
/// lives outside the workspace, so it is the end of the pair that can be
/// trusted, and a directory without one is refused rather than excused.
#[test]
fn the_back_pointer_must_exist_and_name_this_seats_workspace() {
    let layout = Layout::linked();

    // Another worktree's `.git` file: the redirect.
    let elsewhere = layout.dir.path().join("sib/.git");
    layout.point_back_at(&format!("{}\n", elsewhere.display()));
    let refused = scope_refusal(&layout.scope).unwrap();
    assert!(refused.contains("records"), "{refused}");
    assert!(refused.contains("another worktree's metadata"), "{refused}");

    // A back-pointer with no parent directory at all lands in the same
    // refusal rather than in the subdirectory arm.
    layout.point_back_at("/");
    assert!(scope_refusal(&layout.scope)
        .unwrap()
        .contains("another worktree's metadata"));

    // An empty pointer is no pointer.
    layout.point_back_at("\n");
    let refused = scope_refusal(&layout.scope).unwrap();
    assert!(refused.contains("no `gitdir` back-pointer"), "{refused}");

    // And a missing one is refused too: git writes it for every worktree
    // it creates, so its absence is evidence, not silence.
    std::fs::remove_file(layout.scope.git_dir.join("gitdir")).unwrap();
    let refused = scope_refusal(&layout.scope).unwrap();
    assert!(refused.contains("no `gitdir` back-pointer"), "{refused}");
    assert!(
        refused.contains(&layout.scope.workspace.display().to_string()),
        "{refused}"
    );
}

/// The alias the review measured: a seat replaces its own `.git` with a
/// SYMLINK to another worktree's `.git` file. Git then resolves an
/// administrative directory Git itself created, so the topology check
/// holds — and a comparison of the back-pointer against
/// `<workspace>/.git` resolves the seat's OWN side through the symlink
/// to the victim's path, so both ends of the pair appear to agree. The
/// seat owns that path; the worktree ROOT the back-pointer names is the
/// end it cannot move, and that is what the check compares.
#[cfg(unix)]
#[test]
fn a_symlinked_workspace_git_cannot_borrow_another_worktrees_back_pointer() {
    let victim = Layout::linked();
    let seat = victim.dir.path().join("seat");
    std::fs::create_dir_all(&seat).unwrap();
    let victim_git_file = victim.scope.workspace.join(".git");
    std::os::unix::fs::symlink(&victim_git_file, seat.join(".git")).unwrap();
    // The alias really does resolve to the victim's `.git`: a comparison
    // against `<workspace>/.git` would have found the two ends agreeing.
    assert_eq!(
        std::fs::canonicalize(seat.join(".git")).unwrap(),
        std::fs::canonicalize(&victim_git_file).unwrap()
    );
    let scope = GitScope {
        workspace: seat.clone(),
        ..victim.scope.clone()
    };
    let refused = scope_refusal(&scope).unwrap();
    assert!(refused.contains("symbolic link"), "{refused}");
    assert!(refused.contains("linked `git worktree`"), "{refused}");

    // The runner is the last gate and refuses the same scope rather than
    // binding the victim's object store and refs read-write.
    let refused = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &scope,
        &victim.mask,
        &dsh_workspace_write_profile(&seat),
        &["true"],
    ))
    .unwrap_err();
    assert!(refused.contains("symbolic link"), "{refused}");

    // With the symlink gone the borrowed metadata is refused on
    // OWNERSHIP rather than on spelling: the back-pointer's parent is
    // the victim's worktree root, which is not this seat's workspace
    // however the seat spells its own `.git`. A plain COPY of the
    // victim's `.git` file — no symlink anywhere — lands here too.
    std::fs::remove_file(seat.join(".git")).unwrap();
    std::fs::copy(&victim_git_file, seat.join(".git")).unwrap();
    let refused = scope_refusal(&scope).unwrap();
    assert!(refused.contains("another worktree's metadata"), "{refused}");
    assert!(
        refused.contains(&victim.scope.workspace.display().to_string()),
        "{refused}"
    );
}

/// A seat started in a SUBDIRECTORY of its linked worktree resolves the
/// worktree's own git directory, whose back-pointer names the worktree
/// root rather than this workspace. Nothing is pointing at another
/// worktree, so saying so would misname the cause; the refusal names the
/// root to run from instead.
#[test]
fn a_subdirectory_of_a_linked_worktree_is_refused_by_its_own_name() {
    let layout = Layout::linked();
    let nested = layout.scope.workspace.join("crates/inner");
    std::fs::create_dir_all(&nested).unwrap();
    let scope = GitScope {
        workspace: nested.clone(),
        ..layout.scope.clone()
    };
    let refused = scope_refusal(&scope).unwrap();
    assert!(
        refused.contains("is a subdirectory of the linked worktree"),
        "{refused}"
    );
    assert!(
        refused.contains(&layout.scope.workspace.display().to_string()),
        "{refused}"
    );
    assert!(
        !refused.contains("another worktree's metadata"),
        "{refused}"
    );
}

#[test]
fn the_runner_refuses_a_scope_that_would_need_the_whole_shared_git() {
    let scope = GitScope {
        workspace: PathBuf::from("/repo/src"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    let profile = dsh_workspace_write_profile(&scope.workspace);
    let refused = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &scope,
        Path::new("/nonexistent/mask"),
        &profile,
        &["true"],
    ))
    .unwrap_err();
    assert!(
        refused.contains("shared repository's own git directory"),
        "{refused}"
    );
}

/// The mask is only a mask while its SOURCE is an empty regular file the
/// box cannot reach. Every way that can fail is refused rather than
/// mounted, because each one turns the mask into something else: an
/// unreadable path git calls fatal, a config the box can fill, or a
/// config with content nobody wrote.
#[test]
fn a_mask_that_is_not_an_unreachable_empty_regular_file_is_refused() {
    let layout = Layout::linked();
    let profile = dsh_workspace_write_profile(&layout.scope.workspace);
    let refuse = |mask: &Path| {
        runner_argv(&runner_args_for(
            Path::new(BWRAP),
            &layout.scope,
            mask,
            &profile,
            &["true"],
        ))
        .unwrap_err()
    };

    let missing = layout.dir.path().join("not-there");
    assert!(refuse(&missing).contains("cannot be read"), "missing mask");

    // A directory, or the `/dev/null` this fix replaced: bubblewrap binds
    // a source with MS_NODEV, so the box cannot open a device node there
    // and git calls an unreadable config file fatal.
    let directory = layout.dir.path().join("a-directory");
    std::fs::create_dir_all(&directory).unwrap();
    let refused = refuse(&directory);
    assert!(refused.contains("is not a regular file"), "{refused}");
    assert!(refused.contains("MS_NODEV"), "{refused}");
    if Path::new("/dev/null").exists() {
        let refused = refuse(Path::new("/dev/null"));
        assert!(refused.contains("is not a regular file"), "{refused}");
    }

    let filled = layout.dir.path().join("filled");
    std::fs::write(&filled, "[core]\n").unwrap();
    let refused = refuse(&filled);
    assert!(refused.contains("is not empty"), "{refused}");

    // Inside the seat's own writable workspace, a boxed command could fill
    // the very file the mask stands for.
    let inside = layout.scope.workspace.join("mask");
    std::fs::write(&inside, "").unwrap();
    let refused = refuse(&inside);
    assert!(
        refused.contains("inside the seat's own writable"),
        "{refused}"
    );
}

#[test]
fn the_runner_refuses_a_malformed_scope_or_profile() {
    let layout = Layout::linked();
    let profile = dsh_workspace_write_profile(&layout.scope.workspace);
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
        "/bin/bwrap"
    ]))
    .unwrap_err()
    .contains("--mask is required"));
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
    // The runner execs an absolute bubblewrap and mounts an absolute
    // mask; a relative one would be resolved by a working directory the
    // seat can move.
    for (flag, value) in [("--bwrap", "bwrap"), ("--mask", "mask")] {
        let refused = runner_argv(&s(&[
            "--workspace",
            "/w",
            "--git-dir",
            "/g",
            "--common-dir",
            "/c",
            "--bwrap",
            if flag == "--bwrap" {
                value
            } else {
                "/bin/bwrap"
            },
            "--mask",
            if flag == "--mask" { value } else { "/tmp/mask" },
        ]))
        .unwrap_err();
        assert!(
            refused.contains("is not an absolute path"),
            "{flag}: {refused}"
        );
        assert!(refused.contains(flag), "{flag}: {refused}");
    }

    // A profile with no command separator, and a profile that does not
    // stand for the boundary the runner knows.
    let mut no_separator = runner_args(&layout, &profile, &[]);
    no_separator.truncate(no_separator.len() - 1);
    assert!(runner_argv(&no_separator)
        .unwrap_err()
        .contains("no `--` before the command"));
    let bad_profile = vec!["--ro-bind".to_string(), "/tmp".to_string()];
    let bad = runner_args(&layout, &bad_profile, &["true"]);
    assert!(runner_argv(&bad).unwrap_err().contains("--ro-bind / /"));
}

#[test]
fn the_overlay_row_quotes_every_path_and_refuses_a_line_break() {
    let scope = GitScope {
        workspace: PathBuf::from("/work/it's a path"),
        git_dir: PathBuf::from("/main/.git/worktrees/wt"),
        common_dir: PathBuf::from("/main/.git"),
    };
    let mask = Path::new("/tmp/brokkr-dsh-config-mask-abc");
    let row = sandbox_row(
        "/opt/brokkr's bin/brokkr",
        Path::new("/opt/bwrap"),
        mask,
        &scope,
    )
    .unwrap();
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
    // The resolved bubblewrap and the staged mask are trusted paths too.
    assert!(row.contains("      - '--bwrap'\n"), "{row}");
    assert!(row.contains("      - '/opt/bwrap'\n"), "{row}");
    assert!(row.contains("      - '--mask'\n"), "{row}");
    assert!(
        row.contains("      - '/tmp/brokkr-dsh-config-mask-abc'\n"),
        "{row}"
    );
    assert!(
        row.contains(&format!("      - '{RUNNER_FAILURE_SIGNATURE}'\n")),
        "{row}"
    );
    assert!(row.contains("      - 'bwrap: '\n"), "{row}");

    let broken = GitScope {
        workspace: PathBuf::from("/work\nline"),
        ..scope.clone()
    };
    assert!(
        sandbox_row("brokkr", Path::new("/opt/bwrap"), mask, &broken)
            .unwrap_err()
            .contains("spans more than one line")
    );
    // The program itself is a scalar too, and gets the same refusal.
    assert!(
        sandbox_row("/opt/brokkr\nline", Path::new("/opt/bwrap"), mask, &scope)
            .unwrap_err()
            .contains("spans more than one line")
    );
    assert!(
        sandbox_row("/opt/brokkr\rline", Path::new("/opt/bwrap"), mask, &scope)
            .unwrap_err()
            .contains("spans more than one line")
    );
    // So are the bubblewrap and the mask.
    assert!(
        sandbox_row("brokkr", Path::new("/opt/bwrap\nline"), mask, &scope)
            .unwrap_err()
            .contains("spans more than one line")
    );
    assert!(sandbox_row(
        "brokkr",
        Path::new("/opt/bwrap"),
        Path::new("/tmp/mask\nline"),
        &scope
    )
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

/// The staged mask is an empty regular file the runner accepts, and the
/// one way staging can fail is a refusal rather than a panic.
#[test]
fn the_staged_mask_is_an_empty_regular_file() {
    let mask = stage_mask_file().unwrap();
    let meta = std::fs::metadata(mask.path()).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.len(), 0);
    // It is what the runner's own check accepts, from outside any workspace.
    assert!(mask_refusal(mask.path(), Path::new("/no/such/workspace")).is_none());

    let refused = stage_mask_file_in(|| {
        Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "no room",
        ))
    })
    .unwrap_err();
    assert!(
        refused.contains("could not stage the git config mask"),
        "{refused}"
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
///
/// A host that declares [`BOUNDARY_EVIDENCE_ENV`] fails instead of
/// skipping, so this proof cannot report `ok` without running.
#[cfg(target_os = "linux")]
#[test]
fn a_linked_worktree_commits_under_the_dsh_profile_and_the_boundary_holds() {
    use crate::hands::{boundary_evidence_required, skip_boundary_proof};
    let required = boundary_evidence_required();
    if std::env::var_os(crate::hands::HANDS_BOX_ENV).is_some() {
        skip_boundary_proof(required, "this environment is already a box");
        return;
    }
    let Ok(bwrap) = crate::hands::require_bwrap() else {
        skip_boundary_proof(required, "no bubblewrap on PATH");
        return;
    };
    // The runner refuses a relative `--bwrap`, so the test hands it the
    // resolved absolute path exactly as the driver does.
    let bwrap = std::fs::canonicalize(&bwrap).unwrap_or(bwrap);
    if !bwrap_usable(&bwrap) {
        skip_boundary_proof(
            required,
            "this environment cannot create a bubblewrap namespace",
        );
        return;
    }
    let Some(dir) = fixture_root() else {
        skip_boundary_proof(
            required,
            "no fixture root outside the profile's `/tmp` tmpfs",
        );
        return;
    };

    // The mask the driver stages, exactly as the driver stages it: in the
    // system temp directory, which dsh's own profile replaces with a fresh
    // tmpfs inside the box. Bubblewrap resolves a bind SOURCE against the
    // host root, so the box mounts the host's file and can reach it by no
    // other path.
    let config_mask = stage_mask_file().unwrap();

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
    // a per-worktree `config.worktree` is honoured by the host's git. This
    // is the class of repository the `/dev/null` mask broke outright: git
    // answered every command with `fatal: unknown error occurred while
    // reading the configuration files`.
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
    // The layout git itself wrote is the one layout the checks serve.
    assert!(scope_refusal(&scope).is_none());
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
         echo evil > '{worktree_plain_config}' 2>/dev/null || true\n\
         if echo evil > '{sibling_config}' 2>/dev/null; then echo SIBLING_CONFIG_WRITABLE; else echo SIBLING_CONFIG_READONLY; fi\n\
         echo evil > '{scratch}'\n\
         if mv '{scratch}' '{commondir}' 2>/dev/null; then echo COMMONDIR_REPLACED; else echo COMMONDIR_INTACT; fi\n\
         rm -f '{scratch}'\n\
         if echo x > '{commondir}' 2>/dev/null; then echo COMMONDIR_WRITABLE; else echo COMMONDIR_READONLY; fi\n\
         if rm -f '{commondir}' 2>/dev/null; then echo COMMONDIR_UNLINKED; else echo COMMONDIR_STAYS; fi\n\
         if echo x > '{gitdir}' 2>/dev/null; then echo GITDIR_WRITABLE; else echo GITDIR_READONLY; fi\n\
         if rm -f '{gitdir}' 2>/dev/null; then echo GITDIR_UNLINKED; else echo GITDIR_STAYS; fi\n",
        parent = main.join("parent-sentinel").display(),
        sibling_sentinel = sibling.join("sibling-sentinel").display(),
        sibling_index = main.join(".git/worktrees/sibling/HEAD").display(),
        hook = main.join(".git/hooks/post-checkout").display(),
        common = main.join(".git").display(),
        scratch = git_dir.join("probe-scratch").display(),
        commondir = commondir_file.display(),
        gitdir = gitdir_file.display(),
        worktree_config = git_dir.join("config.worktree").display(),
        worktree_plain_config = git_dir.join("config").display(),
        sibling_config = main.join(".git/worktrees/sibling/config.worktree").display(),
    );
    let args = runner_args_for(
        &bwrap,
        &scope,
        config_mask.path(),
        &profile,
        &["bash", "-lc", &script],
    );
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
    // The mask is readable inside the box: an unreadable one makes git
    // answer `fatal: unknown error occurred while reading the
    // configuration files` for every command, which is what a device node
    // did before this fix.
    assert!(!stderr.contains("unable to access"), "{stderr}");

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
    assert!(stdout.contains("GITDIR_STAYS"), "{stdout}");
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
    // Both masked mount points are at worst empty files on the host; the
    // box wrote nothing into either, and the staged source is still empty.
    for masked in ["config.worktree", "config"] {
        let seen = std::fs::read_to_string(git_dir.join(masked)).unwrap_or_default();
        assert!(seen.is_empty(), "{masked}: {seen}");
    }
    assert_eq!(std::fs::metadata(config_mask.path()).unwrap().len(), 0);
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
    // The sibling's own branch is still where the host left it. The shared
    // ref store IS in the write set — a commit cannot happen otherwise —
    // so this is the documented residual, not a mount the runner adds:
    // what the boundary promises for a sibling is its worktree directory
    // and its per-worktree metadata, both proved read-only above.
    assert_eq!(
        git(&sibling, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "sibling"
    );

    // The redirect a hostile seat would try, against the real host: a
    // `.git` file and a fake administrative directory written inside the
    // workspace, naming this repository as the common directory. Git
    // resolves it, and the runner refuses it.
    let hostile = dir.path().join("hostile");
    std::fs::create_dir_all(hostile.join("fake")).unwrap();
    std::fs::write(
        hostile.join(".git"),
        format!("gitdir: {}\n", hostile.join("fake").display()),
    )
    .unwrap();
    std::fs::write(hostile.join("fake/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        hostile.join("fake/commondir"),
        format!("{}\n", main.join(".git").display()),
    )
    .unwrap();
    std::fs::write(
        hostile.join("fake/gitdir"),
        format!("{}\n", hostile.join(".git").display()),
    )
    .unwrap();
    let hostile_facts = crate::hands::git_facts(&hostile);
    let hostile_scope = GitScope {
        workspace: hostile.clone(),
        git_dir: hostile_facts.git_dir.clone().unwrap(),
        common_dir: hostile_facts.common_dir.clone().unwrap(),
    };
    // Git really does report the victim repository here: the refusal is
    // load-bearing, not a check against something git would never say.
    assert_eq!(
        std::fs::canonicalize(&hostile_scope.common_dir).unwrap(),
        std::fs::canonicalize(main.join(".git")).unwrap()
    );
    let refused = runner_argv(&runner_args_for(
        &bwrap,
        &hostile_scope,
        config_mask.path(),
        &dsh_workspace_write_profile(&hostile),
        &["true"],
    ))
    .unwrap_err();
    assert!(refused.contains("administrative directories"), "{refused}");

    // The ALIAS variant, against the same real git: a seat whose own
    // `.git` is a symlink to this worktree's `.git` file. Git resolves a
    // REAL administrative directory it created itself, so the topology
    // check holds and the refusal has to come from ownership. Both
    // spellings of the borrowed pointer are measured — the symlink, and
    // a plain copy with no symlink anywhere.
    let alias = dir.path().join("alias seat");
    std::fs::create_dir_all(&alias).unwrap();
    std::os::unix::fs::symlink(worktree.join(".git"), alias.join(".git")).unwrap();
    let alias_facts = crate::hands::git_facts(&alias);
    let alias_scope = GitScope {
        workspace: alias.clone(),
        git_dir: alias_facts.git_dir.clone().unwrap(),
        common_dir: alias_facts.common_dir.clone().unwrap(),
    };
    // Git really does hand back the victim's metadata through the alias:
    // the refusal is load-bearing, not a check against something git
    // would never say.
    assert_eq!(
        std::fs::canonicalize(&alias_scope.git_dir).unwrap(),
        std::fs::canonicalize(&git_dir).unwrap()
    );
    assert_eq!(
        std::fs::canonicalize(&alias_scope.common_dir).unwrap(),
        std::fs::canonicalize(&common_dir).unwrap()
    );
    let alias_profile = dsh_workspace_write_profile(&alias);
    let refused = runner_argv(&runner_args_for(
        &bwrap,
        &alias_scope,
        config_mask.path(),
        &alias_profile,
        &["true"],
    ))
    .unwrap_err();
    assert!(refused.contains("symbolic link"), "{refused}");
    std::fs::remove_file(alias.join(".git")).unwrap();
    std::fs::copy(worktree.join(".git"), alias.join(".git")).unwrap();
    let refused = runner_argv(&runner_args_for(
        &bwrap,
        &alias_scope,
        config_mask.path(),
        &alias_profile,
        &["true"],
    ))
    .unwrap_err();
    assert!(refused.contains("another worktree's metadata"), "{refused}");
    assert!(
        refused.contains(&worktree.display().to_string()),
        "the refusal names the worktree that owns the metadata: {refused}"
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
        config_mask.path(),
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
