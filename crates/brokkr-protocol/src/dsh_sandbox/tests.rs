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
/// back-pointer, and the worktree's own `.git` file naming it back —
/// beside the two directories the driver stages for one seat. The scope
/// checks read the host, so a unit test has to build a host to be worth
/// anything. `git` itself builds the layout in the behavioral proof; here
/// it is written by hand so the argv tests stay cheap.
struct Layout {
    dir: tempfile::TempDir,
    scope: GitScope,
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
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/slice\n").unwrap();
        // Outside the workspace and outside each other, as the driver
        // stages them.
        let store = dir.path().join("store");
        let trusted = dir.path().join("trusted");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::create_dir_all(&trusted).unwrap();
        std::fs::write(trusted.join(MASK_FILE), "").unwrap();
        std::fs::write(
            trusted.join(COMMONDIR_FILE),
            format!("{}\n", store.display()),
        )
        .unwrap();
        std::fs::write(
            trusted.join(ALTERNATES_FILE),
            format!("{}\n", common_dir.join("objects").display()),
        )
        .unwrap();
        Layout {
            scope: GitScope {
                workspace,
                git_dir,
                common_dir,
            },
            store,
            trusted,
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
        &layout.store,
        &layout.trusted,
        profile,
        command,
    )
}

/// The runner argv with chosen host paths, so the behavioral test can
/// exec the binary `require_bwrap` actually resolved and mount the store
/// the driver actually staged, rather than the spellings the unit tests
/// assert.
fn runner_args_for(
    bwrap: &Path,
    scope: &GitScope,
    store: &Path,
    trusted: &Path,
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
        "--store".to_string(),
        store.to_string_lossy().into_owned(),
        "--trusted".to_string(),
        trusted.to_string_lossy().into_owned(),
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
    let store = layout.store.display().to_string();
    let mask = layout.trusted.join(MASK_FILE).display().to_string();

    // The per-worktree directory — index, HEAD, its own reflog — and the
    // private common directory the seat's git is pointed at. Those are the
    // only two read-write mounts the runner adds.
    assert!(text.contains(&format!("--bind {git} {git}")), "{text}");
    assert!(text.contains(&format!("--bind {store} {store}")), "{text}");
    assert_eq!(
        argv.iter().filter(|token| *token == "--bind").count(),
        3,
        "the profile's workspace grant plus exactly two scoped ones: {text}"
    );

    // Nothing under the SHARED git directory is writable — not `objects`,
    // not `refs`, not `packed-refs`, not `logs`, and no sibling. The seat
    // reads them through the profile's own read-only root and through the
    // store's alternates.
    for name in ["objects", "refs", "logs", "packed-refs"] {
        assert!(
            !text.contains(&format!("--bind {common}/{name}")),
            "{name} must not be writable: {text}"
        );
        assert!(
            !text.contains(&format!("--bind-try {common}/{name}")),
            "{name} must not be writable: {text}"
        );
    }
    assert!(!text.contains("worktrees/sibling"), "{text}");
    assert!(
        !text.contains(&format!("--bind {common} {common}")),
        "the whole common dir must not be mounted writable: {text}"
    );

    // The `commondir` file is where the redirect lives: the box's git
    // reads the staged pointer, which names the private store, and cannot
    // write over it.
    assert!(
        text.contains(&format!(
            "--ro-bind {} {git}/commondir",
            layout.trusted.join(COMMONDIR_FILE).display()
        )),
        "{text}"
    );
    assert!(
        text.contains(&format!("--ro-bind {git}/gitdir {git}/gitdir")),
        "{text}"
    );
    // `HEAD` says which branch this worktree owns, and that is the ref the
    // driver promotes: read-only, so no seat can retarget the next one.
    assert!(
        text.contains(&format!("--ro-bind {git}/HEAD {git}/HEAD")),
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

    // Inside the store: the repository's own config and HEAD are mounted
    // read-only over the driver's copies, and the alternates line the
    // whole object store is read through cannot be repointed.
    assert!(
        text.contains(&format!("--ro-bind {common}/config {store}/config")),
        "{text}"
    );
    assert!(
        text.contains(&format!("--ro-bind {common}/HEAD {store}/HEAD")),
        "{text}"
    );
    // The store's own `config.worktree` is masked too: `<store>/config` is
    // a copy of the shared one, so it carries `extensions.worktreeConfig`
    // for a repository that has run `git sparse-checkout` — and the
    // trusted driver reads the store as a repository afterwards.
    assert!(
        text.contains(&format!("--ro-bind {mask} {store}/config.worktree")),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "--ro-bind {} {store}/objects/info/alternates",
            layout.trusted.join(ALTERNATES_FILE).display()
        )),
        "{text}"
    );

    // Hooks are an empty tmpfs on both hook paths git could use.
    assert!(text.contains(&format!("--tmpfs {git}/hooks")), "{text}");
    assert!(text.contains(&format!("--tmpfs {store}/hooks")), "{text}");

    // The profile and the command travel verbatim, separated by the bare `--`.
    assert!(text.contains("--ro-bind / / --dev /dev"));
    assert!(text.ends_with("-- bash -lc git add -A"), "{text}");
}

#[test]
fn a_read_only_profile_and_a_primary_checkout_get_no_scoped_binds() {
    let layout = Layout::linked();
    let store = layout.store.display().to_string();

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
        argv.join(" ").contains(&format!("--bind {store} {store}")),
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
        argv.join(" ").contains(&format!("--bind {store} {store}")),
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
        &layout.store,
        &layout.trusted,
        &profile,
        &["true"],
    ))
    .unwrap();
    let text = argv.join(" ");
    assert!(!text.contains("--tmpfs /repo/.git/hooks"), "{text}");
    assert!(!text.contains(&format!("--bind {store} {store}")), "{text}");
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
        !argv.join(" ").contains(&layout.store.display().to_string()),
        "a mismatched source/destination pair is not the workspace grant"
    );
}

/// Every bubblewrap option the runner steps over is one it knows the
/// arity of. A table that guessed would read an ARGUMENT as a flag, and
/// the answer to "can the box write the files this runner mounts
/// read-only?" would then be measured against the wrong tokens. Every
/// entry is exercised here, and an option outside the table refuses the
/// command rather than being skipped.
#[test]
fn every_known_bubblewrap_option_is_stepped_over_by_its_own_arity() {
    let none: &[&str] = &[];
    /// One row of the table under test: the option, how many arguments
    /// follow it, and which of them name a host path the box can write —
    /// each with the argument saying where it is mounted, when it is
    /// mounted anywhere.
    type Row = (&'static str, usize, &'static [(usize, Option<usize>)]);
    let cases: &[Row] = &[
        ("--help", 0, &[]),
        ("--version", 0, &[]),
        ("--level-prefix", 0, &[]),
        ("--unshare-all", 0, &[]),
        ("--share-net", 0, &[]),
        ("--unshare-user", 0, &[]),
        ("--unshare-user-try", 0, &[]),
        ("--unshare-ipc", 0, &[]),
        ("--unshare-pid", 0, &[]),
        ("--unshare-net", 0, &[]),
        ("--unshare-uts", 0, &[]),
        ("--unshare-cgroup", 0, &[]),
        ("--unshare-cgroup-try", 0, &[]),
        ("--clearenv", 0, &[]),
        ("--new-session", 0, &[]),
        ("--die-with-parent", 0, &[]),
        ("--as-pid-1", 0, &[]),
        ("--disable-userns", 0, &[]),
        ("--assert-userns-disabled", 0, &[]),
        ("--argv0", 1, &[]),
        ("--userns", 1, &[]),
        ("--userns2", 1, &[]),
        ("--pidns", 1, &[]),
        ("--uid", 1, &[]),
        ("--gid", 1, &[]),
        ("--hostname", 1, &[]),
        ("--chdir", 1, &[]),
        ("--unsetenv", 1, &[]),
        ("--lock-file", 1, &[]),
        ("--sync-fd", 1, &[]),
        ("--remount-ro", 1, &[]),
        ("--exec-label", 1, &[]),
        ("--file-label", 1, &[]),
        ("--proc", 1, &[]),
        ("--dev", 1, &[]),
        ("--tmpfs", 1, &[]),
        ("--mqueue", 1, &[]),
        ("--dir", 1, &[]),
        ("--seccomp", 1, &[]),
        ("--add-seccomp-fd", 1, &[]),
        ("--block-fd", 1, &[]),
        ("--userns-block-fd", 1, &[]),
        ("--info-fd", 1, &[]),
        ("--json-status-fd", 1, &[]),
        ("--cap-add", 1, &[]),
        ("--cap-drop", 1, &[]),
        ("--perms", 1, &[]),
        ("--size", 1, &[]),
        ("--overlay-src", 1, &[]),
        ("--tmp-overlay", 1, &[]),
        ("--ro-overlay", 1, &[]),
        ("--setenv", 2, &[]),
        ("--ro-bind", 2, &[]),
        ("--ro-bind-try", 2, &[]),
        ("--ro-bind-fd", 2, &[]),
        ("--file", 2, &[]),
        ("--bind-data", 2, &[]),
        ("--ro-bind-data", 2, &[]),
        ("--symlink", 2, &[]),
        ("--chmod", 2, &[]),
        ("--bind", 2, &[(0, Some(1))]),
        ("--bind-try", 2, &[(0, Some(1))]),
        ("--dev-bind", 2, &[(0, Some(1))]),
        ("--dev-bind-try", 2, &[(0, Some(1))]),
        ("--overlay", 3, &[(0, Some(2)), (1, None)]),
    ];
    for (flag, arity, writes) in cases {
        let shape = profile_flag(flag).unwrap_or_else(|| panic!("{flag} is not in the table"));
        assert_eq!(shape.arity, *arity, "{flag}");
        assert_eq!(shape.writes, *writes, "{flag}");
        // The whole option, followed by a bind of `/w`: the scan must land
        // on that bind, which it only does when it stepped over exactly
        // `arity` arguments.
        let mut profile: Vec<String> = vec![flag.to_string()];
        profile.extend((0..*arity).map(|slot| format!("/argument-{slot}")));
        profile.extend(["--bind".to_string(), "/w".to_string(), "/w".to_string()]);
        let binds = writable_binds(&profile).unwrap();
        assert_eq!(
            binds.last(),
            Some(&WritableBind {
                source: Path::new("/w"),
                destination: Some(Path::new("/w")),
            }),
            "{flag}"
        );
        assert_eq!(binds.len(), 1 + writes.len(), "{flag}");
    }
    assert!(profile_flag("--no-such-bubblewrap-option").is_none());
    assert!(profile_flag("/not-a-flag").is_none());
    // `--bind-fd FD DEST` is read-write, and its source is a file
    // descriptor rather than a path: the runner cannot measure whether
    // the staged files are reachable through it, so it refuses the
    // command instead of stepping over it.
    assert!(profile_flag("--bind-fd").is_none());
    // Descriptor-sourced options can hide writable binds from the scan.
    assert!(profile_flag("--args").is_none());
    let hidden_options = ["--args", "3", "--bind", "/w", "/w"].map(str::to_string);
    assert!(writable_binds(&hidden_options).is_err());

    // A read-write option the old scan did not know is now read as one,
    // and an option outside the table refuses rather than being skipped.
    let dev_bind = ["--dev-bind", "/src", "/dst"].map(str::to_string);
    assert_eq!(
        writable_binds(&dev_bind).unwrap(),
        vec![WritableBind {
            source: Path::new("/src"),
            destination: Some(Path::new("/dst")),
        }]
    );
    // `--overlay RWSRC WORKDIR DEST` writes two host paths: the upper
    // layer, reached through `DEST`, and the working directory, which is
    // mounted nowhere and would be invisible to a scan that read only
    // mounted sources.
    let overlay = ["--overlay", "/upper", "/work", "/dst"].map(str::to_string);
    assert_eq!(
        writable_binds(&overlay).unwrap(),
        vec![
            WritableBind {
                source: Path::new("/upper"),
                destination: Some(Path::new("/dst")),
            },
            WritableBind {
                source: Path::new("/work"),
                destination: None,
            },
        ]
    );
    let unknown = ["--future-option", "/src"].map(str::to_string);
    let refused = writable_binds(&unknown).unwrap_err();
    assert!(refused.contains("--future-option"), "{refused}");
    assert!(refused.contains("does not read"), "{refused}");
    // Truncated arguments are a malformed profile, not a shorter one.
    let truncated = ["--bind", "/src"].map(str::to_string);
    let refused = writable_binds(&truncated).unwrap_err();
    assert!(
        refused.contains("ends inside --bind's arguments"),
        "{refused}"
    );
    assert!(writable_binds(
        none.iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .as_slice()
    )
    .unwrap()
    .is_empty());

    // And the runner refuses the whole command on an unknown option,
    // before it composes a single bind.
    let layout = Layout::linked();
    let mut profile = dsh_workspace_write_profile(&layout.scope.workspace);
    profile.push("--future-option".to_string());
    let refused = runner_argv(&runner_args(&layout, &profile, &["true"])).unwrap_err();
    assert!(refused.contains("--future-option"), "{refused}");
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
/// linked worktree, and the scoped runner may never bind a directory
/// under the victim on its word.
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
    let layout = Layout::linked();
    let refused = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &scope,
        &layout.store,
        &layout.trusted,
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
    // binding the victim's metadata.
    let refused = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &scope,
        &victim.store,
        &victim.trusted,
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
    let layout = Layout::linked();
    let scope = GitScope {
        workspace: PathBuf::from("/repo/src"),
        git_dir: PathBuf::from("/repo/.git"),
        common_dir: PathBuf::from("/repo/.git"),
    };
    let profile = dsh_workspace_write_profile(&scope.workspace);
    let refused = runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &scope,
        &layout.store,
        &layout.trusted,
        &profile,
        &["true"],
    ))
    .unwrap_err();
    assert!(
        refused.contains("shared repository's own git directory"),
        "{refused}"
    );
}

/// The three staged files are only what they stand for while each is a
/// real file the box cannot write. Every way that can fail is refused
/// rather than mounted, because each one turns a mount into something
/// else: an unreadable path git calls fatal, a config the box can fill,
/// or a pointer with content nobody wrote.
#[test]
fn a_staged_file_that_is_not_what_the_runner_mounts_is_refused() {
    let layout = Layout::linked();
    let profile = dsh_workspace_write_profile(&layout.scope.workspace);
    let refuse = || runner_argv(&runner_args(&layout, &profile, &["true"])).unwrap_err();

    // The mask must exist. So must each pointer.
    for name in [MASK_FILE, COMMONDIR_FILE, ALTERNATES_FILE] {
        let path = layout.trusted.join(name);
        let kept = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let refused = refuse();
        assert!(refused.contains("cannot be read"), "{name}: {refused}");
        assert!(refused.contains(name), "{name}: {refused}");
        std::fs::write(&path, kept).unwrap();
    }
    assert!(layout.argv(&["true"]).is_ok());

    // A directory, or the `/dev/null` an earlier fix used: bubblewrap
    // binds a source with MS_NODEV, so the box cannot open a device node
    // there and git calls an unreadable config file fatal.
    let mask = layout.trusted.join(MASK_FILE);
    std::fs::remove_file(&mask).unwrap();
    std::fs::create_dir(&mask).unwrap();
    let refused = refuse();
    assert!(refused.contains("is not a regular file"), "{refused}");
    assert!(refused.contains("MS_NODEV"), "{refused}");
    std::fs::remove_dir(&mask).unwrap();

    // The mask stands for a config nobody wrote, so content is a refusal.
    std::fs::write(&mask, "[core]\n").unwrap();
    let refused = refuse();
    assert!(refused.contains("is not empty"), "{refused}");
    std::fs::write(&mask, "").unwrap();

    // The two pointers are the other way round: git reads each as a path,
    // and an empty one is a path it cannot follow.
    for name in [COMMONDIR_FILE, ALTERNATES_FILE] {
        let path = layout.trusted.join(name);
        let kept = std::fs::read(&path).unwrap();
        std::fs::write(&path, "").unwrap();
        let refused = refuse();
        assert!(refused.contains("is empty"), "{name}: {refused}");
        std::fs::write(&path, kept).unwrap();
    }
    assert!(layout.argv(&["true"]).is_ok());
}

/// The staged files are only unreachable while the profile keeps them so.
/// dsh 0.1.2-rc.1 replaces `/tmp` with a fresh tmpfs, which is why the
/// host's temporary directory is where the driver stages them — but
/// `writableRoots` is documented as the workspace PLUS the platform temp
/// areas, so a later provider that binds one of them read-write would
/// hand the box a path to a mount's source. The runner reads the profile
/// it was actually given rather than trusting that shape: any read-write
/// bind covering the staged directory refuses the command, and the
/// scoped write set the runner adds itself refuses it too.
#[test]
fn staged_files_the_profile_would_let_the_box_write_are_refused() {
    let layout = Layout::linked();
    let plain = dsh_workspace_write_profile(&layout.scope.workspace);
    let refuse_with = |profile: &[String], trusted: &Path| {
        runner_argv(&runner_args_for(
            Path::new(BWRAP),
            &layout.scope,
            &layout.store,
            trusted,
            profile,
            &["true"],
        ))
    };

    // The profile dsh composes today leaves them alone.
    assert!(refuse_with(&plain, &layout.trusted).is_ok());

    // A profile that also binds the directory they were staged in — the
    // temp-area case — makes them files the box can rewrite, at every
    // spelling of a read-write bind and wherever it is mounted.
    for (flag, destination) in [
        ("--bind", layout.dir.path().display().to_string()),
        ("--bind-try", layout.dir.path().display().to_string()),
        ("--dev-bind", layout.dir.path().display().to_string()),
        ("--bind", "/elsewhere".to_string()),
    ] {
        let mut profile = plain.clone();
        profile.extend([
            flag.to_string(),
            layout.dir.path().display().to_string(),
            destination.clone(),
        ]);
        let refused = refuse_with(&profile, &layout.trusted).unwrap_err();
        assert!(refused.contains("this seat's box can write"), "{refused}");
        assert!(
            refused.contains(&layout.dir.path().display().to_string()),
            "{flag} {destination}: {refused}"
        );
    }

    // An overlay's WORKING directory is a host path the box fills too,
    // even though nothing is mounted at it, so staging under one is the
    // same refusal as staging under a bind's source.
    let mut overlay = plain.clone();
    overlay.extend([
        "--overlay".to_string(),
        "/upper".to_string(),
        layout.dir.path().display().to_string(),
        "/dst".to_string(),
    ]);
    let refused = refuse_with(&overlay, &layout.trusted).unwrap_err();
    assert!(refused.contains("this seat's box can write"), "{refused}");
    assert!(
        refused.contains(&layout.dir.path().display().to_string()),
        "{refused}"
    );

    // A read-only bind of the same directory is not a way to write it.
    let mut read_only = plain.clone();
    read_only.extend([
        "--ro-bind".to_string(),
        layout.dir.path().display().to_string(),
        layout.dir.path().display().to_string(),
    ]);
    assert!(refuse_with(&read_only, &layout.trusted).is_ok());

    // The runner's OWN write set counts as well: the per-worktree git
    // directory and the private store are read-write for the commit, so
    // files staged under either are files the box can rewrite.
    for root in [&layout.scope.git_dir, &layout.store] {
        let inside = root.join("trusted");
        std::fs::create_dir_all(&inside).unwrap();
        for name in [MASK_FILE, COMMONDIR_FILE, ALTERNATES_FILE] {
            std::fs::write(inside.join(name), "x").unwrap();
        }
        let refused = refuse_with(&plain, &inside).unwrap_err();
        assert!(refused.contains("this seat's box can write"), "{refused}");
        assert!(refused.contains(&root.display().to_string()), "{refused}");
    }
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
    let head = ["--workspace", "/w", "--git-dir", "/g", "--common-dir", "/c"];

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
    assert!(runner_argv(&s(&head))
        .unwrap_err()
        .contains("--bwrap is required"));
    assert!(
        runner_argv(&s(&[&head[..], &["--bwrap", "/bin/bwrap"][..]].concat()))
            .unwrap_err()
            .contains("--store is required")
    );
    assert!(runner_argv(&s(&[
        &head[..],
        &["--bwrap", "/bin/bwrap", "--store", "/tmp/store"][..]
    ]
    .concat()))
    .unwrap_err()
    .contains("--trusted is required"));
    assert!(runner_argv(&s(&[
        &head[..],
        &["--bwrap", "/bin/bwrap", "--bwrap", "/other/bwrap"][..]
    ]
    .concat()))
    .unwrap_err()
    .contains("--bwrap given twice"));

    // The runner execs an absolute bubblewrap and mounts absolute staged
    // directories; a relative one would be resolved by a working
    // directory the seat can move.
    for flag in ["--bwrap", "--store", "--trusted"] {
        let pick = |name: &str, absolute: &'static str| {
            if name == flag {
                "relative"
            } else {
                absolute
            }
        };
        let refused = runner_argv(&s(&[
            &head[..],
            &[
                "--bwrap",
                pick("--bwrap", "/bin/bwrap"),
                "--store",
                pick("--store", "/tmp/store"),
                "--trusted",
                pick("--trusted", "/tmp/trusted"),
            ][..],
        ]
        .concat()))
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
    // A workspace whose real path carries an apostrophe, so the quoting
    // under test is exercised against a path a host really can hold.
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("it's a path");
    let common_dir = dir.path().join("main/.git");
    let git_dir = common_dir.join("worktrees/wt");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/slice\n").unwrap();
    let scope = GitScope {
        workspace: workspace.clone(),
        git_dir,
        common_dir,
    };
    let staged = stage_seat_store(&scope).unwrap();
    let row = sandbox_row(
        "/opt/brokkr's bin/brokkr",
        Path::new("/opt/bwrap"),
        &staged,
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
    assert!(
        row.contains(&format!(
            "      - '{}'\n",
            workspace.display().to_string().replace('\'', "''")
        )),
        "{row}"
    );
    assert!(row.contains("it''s a path"), "{row}");
    assert!(row.contains("      - '--git-dir'\n"), "{row}");
    assert!(row.contains("      - '--common-dir'\n"), "{row}");
    // The resolved bubblewrap and the two staged directories are trusted
    // paths too.
    assert!(row.contains("      - '--bwrap'\n"), "{row}");
    assert!(row.contains("      - '/opt/bwrap'\n"), "{row}");
    assert!(row.contains("      - '--store'\n"), "{row}");
    assert!(
        row.contains(&format!("      - '{}'\n", staged.store_path().display())),
        "{row}"
    );
    assert!(row.contains("      - '--trusted'\n"), "{row}");
    assert!(
        row.contains(&format!("      - '{}'\n", staged.trusted_path().display())),
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
        sandbox_row("brokkr", Path::new("/opt/bwrap"), &staged, &broken)
            .unwrap_err()
            .contains("spans more than one line")
    );
    // The program itself is a scalar too, and gets the same refusal.
    assert!(sandbox_row(
        "/opt/brokkr\nline",
        Path::new("/opt/bwrap"),
        &staged,
        &scope
    )
    .unwrap_err()
    .contains("spans more than one line"));
    assert!(sandbox_row(
        "/opt/brokkr\rline",
        Path::new("/opt/bwrap"),
        &staged,
        &scope
    )
    .unwrap_err()
    .contains("spans more than one line"));
    // So is the bubblewrap.
    assert!(
        sandbox_row("brokkr", Path::new("/opt/bwrap\nline"), &staged, &scope)
            .unwrap_err()
            .contains("spans more than one line")
    );
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

/// The staged store is a real git common directory the seat can write
/// and the driver can read back, and the ways staging can fail are
/// refusals rather than panics.
#[test]
fn the_staged_store_is_a_private_common_directory() {
    let layout = Layout::linked();
    // A shared directory with something in every name the staging copies.
    let common = &layout.scope.common_dir;
    std::fs::create_dir_all(common.join("refs/heads")).unwrap();
    std::fs::create_dir_all(common.join("info")).unwrap();
    std::fs::write(common.join("refs/heads/slice"), "a".repeat(40)).unwrap();
    std::fs::write(common.join("packed-refs"), "# pack-refs with: peeled\n").unwrap();
    std::fs::write(common.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(common.join("config"), "[core]\n").unwrap();
    std::fs::write(common.join("info/exclude"), "target\n").unwrap();

    let staged = stage_seat_store(&layout.scope).unwrap();
    assert_eq!(staged.reference(), "refs/heads/slice");
    let store = staged.store_path();
    assert_eq!(
        std::fs::read_to_string(store.join("refs/heads/slice")).unwrap(),
        "a".repeat(40)
    );
    assert_eq!(
        std::fs::read_to_string(store.join("packed-refs")).unwrap(),
        "# pack-refs with: peeled\n"
    );
    assert_eq!(
        std::fs::read_to_string(store.join("info/exclude")).unwrap(),
        "target\n"
    );
    assert_eq!(
        std::fs::read_to_string(store.join("HEAD")).unwrap(),
        "ref: refs/heads/main\n"
    );
    assert_eq!(
        std::fs::read_to_string(store.join("config")).unwrap(),
        "[core]\n"
    );
    // A name the shared directory does not carry is not a failure.
    assert!(!store.join("shallow").exists());
    // The store reads the host's objects and writes none of them.
    let alternates = format!("{}\n", common.join("objects").display());
    assert_eq!(
        std::fs::read_to_string(store.join("objects/info/alternates")).unwrap(),
        alternates
    );
    assert!(store.join("logs").is_dir());

    // The trusted directory: the empty mask, the commondir pointing at
    // the store, and the same alternates line the runner mounts.
    let trusted = staged.trusted_path();
    assert_eq!(
        std::fs::read_to_string(trusted.join(MASK_FILE)).unwrap(),
        ""
    );
    assert_eq!(
        std::fs::read_to_string(trusted.join(COMMONDIR_FILE)).unwrap(),
        format!("{}\n", store.display())
    );
    assert_eq!(
        std::fs::read_to_string(trusted.join(ALTERNATES_FILE)).unwrap(),
        alternates
    );
    // And it is what the runner's own check accepts.
    let profile = dsh_workspace_write_profile(&layout.scope.workspace);
    assert!(runner_argv(&runner_args_for(
        Path::new(BWRAP),
        &layout.scope,
        store,
        trusted,
        &profile,
        &["true"],
    ))
    .is_ok());
}

/// The two ways staging cannot start: a host that will not give the
/// driver a directory, and a worktree with no branch to promote to.
#[test]
fn staging_refuses_a_host_or_a_worktree_that_cannot_carry_a_seat() {
    let layout = Layout::linked();
    let full = || {
        Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "no room",
        ))
    };
    let refused = stage_seat_store_with("git", &layout.scope, full).unwrap_err();
    assert!(refused.contains("could not stage"), "{refused}");

    // A `git` that cannot be run at all is a failure and never an answer:
    // the staging asks git which ref backend the repository uses and where
    // its branch stands, and reading "no answer" as "the ordinary one"
    // would start a seat on questions nobody answered.
    let refused =
        stage_seat_store_with("brokkr-no-such-git", &layout.scope, tempfile::tempdir).unwrap_err();
    assert!(refused.contains("could not run"), "{refused}");
    // The SECOND directory is staged too, and fails the same way.
    let mut made = 0;
    let refused = stage_seat_store_with("git", &layout.scope, || {
        made += 1;
        if made == 1 {
            tempfile::tempdir()
        } else {
            full()
        }
    })
    .unwrap_err();
    assert!(refused.contains("could not stage"), "{refused}");
    // And so does writing into them.
    let mut made = 0;
    let refused = stage_seat_store_with("git", &layout.scope, || {
        made += 1;
        let dir = tempfile::tempdir()?;
        if made == 2 {
            std::fs::remove_dir_all(dir.path())?;
        }
        Ok(dir)
    })
    .unwrap_err();
    assert!(refused.contains("could not stage"), "{refused}");

    // A shared directory with no `worktrees` tree at all is not a
    // conflict, and not a panic either.
    let bare = Layout::linked();
    let alone = GitScope {
        common_dir: bare.dir.path().join("elsewhere/.git"),
        ..bare.scope.clone()
    };
    std::fs::create_dir_all(&alone.common_dir).unwrap();
    assert!(stage_seat_store(&alone).is_ok());

    // A branch two checkouts claim is owned by neither. Git will not
    // create that state, but a seat CAN write `<git_dir>/HEAD` — which is
    // why the runner mounts it read-only and why the driver checks the
    // other checkouts' `HEAD`s, which lie outside every seat's write set.
    std::fs::write(
        layout.scope.common_dir.join("HEAD"),
        "ref: refs/heads/slice\n",
    )
    .unwrap();
    let refused = stage_seat_store(&layout.scope).unwrap_err();
    assert!(refused.contains("and so does"), "{refused}");
    assert!(refused.contains("owned by neither"), "{refused}");
    // A sibling worktree claiming it is the same refusal.
    std::fs::write(
        layout.scope.common_dir.join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .unwrap();
    let sibling = layout.scope.common_dir.join("worktrees/sibling");
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join("HEAD"), "ref: refs/heads/slice\n").unwrap();
    let refused = stage_seat_store(&layout.scope).unwrap_err();
    assert!(
        refused.contains(&sibling.display().to_string()),
        "{refused}"
    );
    std::fs::write(sibling.join("HEAD"), "ref: refs/heads/sibling\n").unwrap();
    assert!(stage_seat_store(&layout.scope).is_ok());

    // A detached HEAD owns no ref, so there is nothing a commit could be
    // promoted to, and the seat is told so before it starts.
    std::fs::write(layout.scope.git_dir.join("HEAD"), "a".repeat(40)).unwrap();
    let refused = stage_seat_store(&layout.scope).unwrap_err();
    assert!(refused.contains("no branch checked out"), "{refused}");
    assert!(refused.contains("git switch"), "{refused}");
    // So does a HEAD naming something that is not a branch, and a missing
    // one.
    std::fs::write(layout.scope.git_dir.join("HEAD"), "ref: refs/tags/v1\n").unwrap();
    assert!(stage_seat_store(&layout.scope)
        .unwrap_err()
        .contains("no branch checked out"));
    std::fs::remove_file(layout.scope.git_dir.join("HEAD")).unwrap();
    assert!(stage_seat_store(&layout.scope)
        .unwrap_err()
        .contains("no branch checked out"));
}

/// The copy that builds the private ref store: trees, files, absent
/// names, and a destination the host will not take.
#[test]
fn the_private_store_copy_carries_trees_files_and_absences() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    std::fs::create_dir_all(from.join("heads/topic")).unwrap();
    std::fs::write(from.join("heads/topic/one"), "one").unwrap();
    std::fs::write(from.join("plain"), "plain").unwrap();

    let to = dir.path().join("to");
    copy_tree(&from, &to).unwrap();
    assert_eq!(
        std::fs::read_to_string(to.join("heads/topic/one")).unwrap(),
        "one"
    );
    assert_eq!(std::fs::read_to_string(to.join("plain")).unwrap(), "plain");

    // A name the source does not carry is nothing to copy, not a failure.
    copy_tree(&from.join("absent"), &to.join("absent")).unwrap();
    assert!(!to.join("absent").exists());

    // A destination the host will not take is the error that travels.
    let failed = copy_tree(&from.join("plain"), &dir.path().join("no/such/dir/plain")).unwrap_err();
    assert_eq!(failed.kind(), std::io::ErrorKind::NotFound);
}

/// The store the box held is not the store the driver reads.
///
/// A seat owns its private common directory for its whole life, and the
/// promotion afterwards hands that directory to git as a repository,
/// outside every box. Git resolves a git directory's COMMON directory
/// from `<dir>/commondir` however that directory was named, `--git-dir`
/// included — measured here against a real git, because the reclaim is
/// only worth anything if git really does follow it — and reads the
/// repository configuration, the ref store and the object store from
/// wherever it lands. So the driver keeps the objects and the refs and
/// writes everything else afresh.
#[test]
fn the_store_the_driver_reads_is_the_one_the_driver_authored() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());
    let staged = stage_seat_store(&repo.scope).unwrap();
    let store = staged.store_path().to_path_buf();
    std::fs::write(repo.worktree.join("b.txt"), "b\n").unwrap();
    repo.git_with_common(&staged, &["add", "b.txt"]);
    repo.git_with_common(&staged, &["commit", "-q", "-m", "boxed"]);
    let committed = repo.git_with_common(&staged, &["rev-parse", "HEAD"]);

    // Everything a seat can leave in a directory it owns. `commondir` is
    // the one that matters: it redirects every other answer at once.
    let evil = dir.path().join("evil common");
    std::fs::create_dir_all(evil.join("objects/info")).unwrap();
    std::fs::create_dir_all(evil.join("refs/heads")).unwrap();
    std::fs::write(
        evil.join("config"),
        "[core]\n\trepositoryformatversion = 0\n\tbare = true\n\thooksPath = /evil-hooks\n",
    )
    .unwrap();
    std::fs::write(evil.join("HEAD"), "ref: refs/heads/slice\n").unwrap();
    std::fs::write(evil.join("refs/heads/slice"), format!("{}\n", repo.base)).unwrap();
    std::fs::write(
        evil.join("objects/info/alternates"),
        format!("{}\n", repo.main.join(".git/objects").display()),
    )
    .unwrap();
    std::fs::write(store.join("commondir"), format!("{}\n", evil.display())).unwrap();
    std::fs::write(store.join("shallow"), format!("{}\n", repo.base)).unwrap();
    std::fs::create_dir_all(store.join("info")).unwrap();
    std::fs::write(store.join("info/grafts"), format!("{}\n", repo.base)).unwrap();
    std::fs::create_dir_all(store.join("hooks")).unwrap();
    std::fs::write(store.join("hooks/post-receive"), "#!/bin/sh\n").unwrap();
    std::fs::write(
        store.join("config.worktree"),
        "[core]\n\thooksPath = /evil\n",
    )
    .unwrap();
    std::fs::write(store.join("surprise"), "a name nobody enumerated\n").unwrap();
    std::fs::write(store.join("objects/info/grafts"), "").unwrap();
    // A symlink at a KEPT name: a second name for a directory this store
    // does not own. Following one is how a reclaim would come to delete
    // the shared repository's own files.
    let shared_packed = repo.main.join(".git/packed-refs");
    let packed_before = std::fs::read_to_string(&shared_packed).unwrap();
    std::fs::remove_file(store.join("packed-refs")).unwrap();
    std::os::unix::fs::symlink(&shared_packed, store.join("packed-refs")).unwrap();

    // The hole is real, against a real git: with the `commondir` the box
    // wrote, a trusted `git --git-dir=<store>` reads the seat's common
    // directory, the seat's configuration and the seat's value for the
    // branch the driver is about to promote.
    let store_git = |args: &[&str]| {
        let mut argv = vec!["--git-dir", store.to_str().unwrap()];
        argv.extend_from_slice(args);
        repo.git(&repo.main, &argv)
    };
    assert_eq!(
        store_git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]),
        evil.display().to_string()
    );
    assert_eq!(
        store_git(&["config", "--get", "core.hooksPath"]),
        "/evil-hooks"
    );
    assert_eq!(
        store_git(&["rev-parse", "--verify", "refs/heads/slice"]),
        repo.base
    );

    staged.reclaim().unwrap();

    // Nothing the box wrote survives, whether the driver had a name for
    // it or not.
    for gone in [
        "commondir",
        "shallow",
        "info",
        "hooks",
        "config.worktree",
        "surprise",
        "logs",
        "packed-refs",
        "objects/info/grafts",
    ] {
        assert!(!store.join(gone).exists(), "{gone} survived the reclaim");
    }
    // And the symlinked name was unlinked, not walked into.
    assert_eq!(
        std::fs::read_to_string(&shared_packed).unwrap(),
        packed_before,
        "the reclaim followed a symlink into the shared repository"
    );
    // What the driver reads is what the driver wrote.
    assert_eq!(
        std::fs::read_to_string(store.join("HEAD")).unwrap(),
        "ref: refs/heads/slice\n"
    );
    assert_eq!(
        std::fs::read_to_string(store.join("config")).unwrap(),
        std::fs::read_to_string(repo.main.join(".git/config")).unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(store.join("objects/info/alternates")).unwrap(),
        alternates_line(&repo.scope.common_dir)
    );
    // Git agrees: the store is its own common directory again, carries no
    // configuration the box chose, and answers with the seat's commit.
    assert_eq!(
        store_git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]),
        store.display().to_string()
    );
    assert_eq!(store_git(&["config", "--get", "core.hooksPath"]), "");
    assert_eq!(
        store_git(&["rev-parse", "--verify", "refs/heads/slice"]),
        committed
    );

    // A name that is not there is nothing to remove, so a second reclaim
    // is not a failure.
    remove(&store.join("never-existed")).unwrap();
    staged.reclaim().unwrap();

    // And the promotion that follows moves the seat's commit, not the
    // value the box aimed it at.
    let promotion = promote_seat_commits(staged, &repo.scope).unwrap().unwrap();
    assert_eq!(promotion.to, committed);
    assert_eq!(repo.host_ref("refs/heads/slice"), committed);
    // A `shallow` the seat wrote does not become the shared repository's
    // history boundary.
    assert!(!repo.main.join(".git/shallow").exists());
    repo.git(&repo.main, &["fsck", "--no-progress", "--no-dangling"]);
}

/// A seat owns its private store read-write for its whole life, and that
/// includes taking access AWAY: `chmod 0500` on a directory it created
/// inside the store — or on the store itself — makes the driver's own
/// `remove_dir_all` fail with `EACCES`, because both run as the same
/// uid and an unlink WRITES the directory holding the name. A reclaim
/// that stopped at the first such name would leave the rest of what the
/// box wrote, `commondir` among it. So the driver takes the access back
/// before it sweeps.
#[test]
fn a_store_the_box_locked_is_still_reclaimed() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());
    let staged = stage_seat_store(&repo.scope).unwrap();
    let store = staged.store_path().to_path_buf();
    std::fs::write(repo.worktree.join("b.txt"), "b\n").unwrap();
    repo.git_with_common(&staged, &["add", "b.txt"]);
    repo.git_with_common(&staged, &["commit", "-q", "-m", "boxed"]);
    let committed = repo.git_with_common(&staged, &["rev-parse", "HEAD"]);

    // The redirection the reclaim exists for, whose configuration names
    // a command git would run for a human reading this store.
    let evil = dir.path().join("evil common");
    std::fs::create_dir_all(evil.join("refs/heads")).unwrap();
    std::fs::write(
        evil.join("config"),
        "[core]\n\trepositoryformatversion = 0\n\tbare = true\n\thooksPath = /evil-hooks\n",
    )
    .unwrap();
    std::fs::write(evil.join("HEAD"), "ref: refs/heads/slice\n").unwrap();
    std::fs::write(evil.join("refs/heads/slice"), format!("{}\n", repo.base)).unwrap();
    std::fs::write(store.join("commondir"), format!("{}\n", evil.display())).unwrap();

    // And the lock beside it: a tree the driver cannot unlink out of,
    // locked one level down as well so the repair is not only at the
    // top, plus the store's own directory — which is where the unlink of
    // `commondir` itself has to happen.
    let locked = store.join("locked");
    std::fs::create_dir_all(locked.join("deeper")).unwrap();
    std::fs::write(locked.join("deeper/held"), "x\n").unwrap();
    let lock = |path: &Path| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o500)).unwrap();
    };
    lock(&locked.join("deeper"));
    lock(&locked);
    lock(&store);
    assert!(
        std::fs::remove_dir_all(&locked).is_err(),
        "the lock does not hold on this filesystem, so this proof measures nothing"
    );

    staged.reclaim().unwrap();

    assert!(!locked.exists(), "a locked tree survived the reclaim");
    assert!(
        !store.join("commondir").exists(),
        "the redirection survived the reclaim"
    );
    // Git agrees: the store is its own common directory again, carries
    // no configuration the box chose, and answers with the seat's commit.
    let store_git = |args: &[&str]| {
        let mut argv = vec!["--git-dir", store.to_str().unwrap()];
        argv.extend_from_slice(args);
        repo.git(&repo.main, &argv)
    };
    assert_eq!(
        store_git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]),
        store.display().to_string()
    );
    assert_eq!(store_git(&["config", "--get", "core.hooksPath"]), "");
    assert_eq!(
        store_git(&["rev-parse", "--verify", "refs/heads/slice"]),
        committed
    );
}

/// The refusal that KEEPS a store names where the commits are — and
/// offers a command to read them only when the store is a repository the
/// DRIVER authored. A reclaim that could not finish leaves whatever the
/// box wrote in place, and an operator who pasted `git --git-dir=<store>`
/// out of the refusal would let a surviving `commondir` choose the
/// repository, its configuration, and the commands that configuration
/// runs. So that message names the path, says what the directory is, and
/// hands the reclaim over in words instead.
#[test]
fn a_store_the_driver_could_not_take_back_is_not_offered_to_git() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());

    // A store the driver did take back: read it with this command.
    let staged = stage_seat_store(&repo.scope).unwrap();
    let store = staged.store_path().to_path_buf();
    let kept = keep_store(staged, "the promotion refused");
    assert!(kept.contains("the promotion refused"), "{kept}");
    assert!(
        kept.contains(&format!(
            "git --git-dir={} log refs/heads/slice",
            store.display()
        )),
        "{kept}"
    );
    std::fs::remove_dir_all(&store).unwrap();

    // A store it could not: the same path, because the commits are still
    // the operator's to recover, and no command against it.
    let staged = stage_seat_store(&repo.scope).unwrap();
    let store = staged.store_path().to_path_buf();
    std::fs::remove_dir_all(&store).unwrap();
    let kept = keep_store(staged, "the promotion refused");
    assert!(kept.contains(&store.display().to_string()), "{kept}");
    // Not one command an operator could paste points git at this
    // directory: `--git-dir` appears only as the thing a surviving
    // `commondir` would redirect, never with a path bound to it.
    assert!(!kept.contains("--git-dir="), "{kept}");
    assert!(
        kept.contains("NOT a repository the driver authored"),
        "{kept}"
    );
    // And the reclaim in words is the reclaim the driver would have run.
    assert!(
        kept.contains(alternates_line(&repo.scope.common_dir).trim_end()),
        "{kept}"
    );
    assert!(kept.contains("ref: refs/heads/slice"), "{kept}");
}

/// A real repository, a real linked worktree, and a real private store:
/// the promotion moves the ONE branch the worktree owns, leaves every
/// other ref where the host had it, and says so.
///
/// This proof needs `git` and nothing else — no namespace, so no skip:
/// an ordinary temporary directory is a host it can read back.
#[test]
fn a_promotion_moves_the_owned_branch_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());
    let staged = stage_seat_store(&repo.scope).unwrap();

    // Nothing committed: nothing to promote, and the host is untouched.
    assert_eq!(
        promote_seat_commits(staged, &repo.scope).unwrap(),
        None,
        "an untouched store promotes nothing"
    );

    // The seat commits inside the private store, and moves a sibling's
    // branch and a tag while it is there.
    let staged = stage_seat_store(&repo.scope).unwrap();
    let store = staged.store_path().to_path_buf();
    // The anchor the objects travel through is this SEAT's, not one every
    // seat of the repository shares.
    let anchor = staged.promotion_ref.clone();
    assert!(anchor.starts_with(PROMOTION_REF_PREFIX), "{anchor}");
    assert_ne!(anchor, PROMOTION_REF_PREFIX, "{anchor}");
    let seat = |args: &[&str]| repo.git_with_common(&staged, args);
    std::fs::write(repo.worktree.join("b.txt"), "b\n").unwrap();
    seat(&["add", "b.txt"]);
    seat(&["commit", "-q", "-m", "boxed"]);
    let committed = seat(&["rev-parse", "HEAD"]);
    seat(&["update-ref", "refs/heads/sibling", &committed]);
    seat(&["tag", "-f", "v1", &committed]);
    seat(&["update-ref", "refs/heads/brand-new", &committed]);

    // Before the promotion, the host has none of it.
    assert_eq!(repo.host_ref("refs/heads/slice"), repo.base);
    assert_eq!(repo.host_ref("refs/heads/sibling"), repo.base);

    let promotion = promote_seat_commits(staged, &repo.scope).unwrap().unwrap();
    assert_eq!(promotion.reference, "refs/heads/slice");
    assert_eq!(promotion.from.as_deref(), Some(repo.base.as_str()));
    assert_eq!(promotion.to, committed);
    assert!(promotion.summary().contains("refs/heads/slice"), "summary");
    assert!(promotion.summary().contains(&committed), "summary");

    // The owned branch moved; every other ref the seat touched did not,
    // and the temporary namespace the objects travelled through is gone.
    assert_eq!(repo.host_ref("refs/heads/slice"), committed);
    assert_eq!(repo.host_ref("refs/heads/sibling"), repo.base);
    assert_eq!(repo.host_ref("refs/tags/v1"), repo.base);
    assert_eq!(
        repo.git(
            &repo.main,
            &["rev-parse", "--verify", "--quiet", "refs/heads/brand-new"]
        ),
        ""
    );
    assert_eq!(
        repo.git(&repo.main, &["rev-parse", "--verify", "--quiet", &anchor]),
        ""
    );
    // The worktree reads its own commit back, and the repository is whole.
    assert_eq!(
        repo.git(&repo.worktree, &["log", "-1", "--format=%s"]),
        "boxed"
    );
    repo.git(&repo.main, &["fsck", "--no-progress", "--no-dangling"]);
    assert!(!store.exists(), "a promoted store is discarded");
}

/// Every way a promotion can fail keeps the seat's commits and says
/// where they are, rather than discarding the store it could not move.
#[test]
fn a_promotion_that_cannot_happen_keeps_the_store_and_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());

    // A `git` that cannot be run at all — read as a failure and never as
    // "this store has no such ref", which would discard the seat's work.
    let staged = stage_seat_store(&repo.scope).unwrap();
    let commit = |staged: &SeatGitStore| {
        std::fs::write(repo.worktree.join("b.txt"), "b\n").unwrap();
        repo.git_with_common(staged, &["add", "b.txt"]);
        repo.git_with_common(staged, &["commit", "-q", "-m", "boxed"])
    };
    commit(&staged);
    let store = staged.store_path().to_path_buf();
    let refused = promote_seat_commits_with("brokkr-no-such-git", staged, &repo.scope).unwrap_err();
    assert!(refused.contains("could not run"), "{refused}");
    assert!(refused.contains(&store.display().to_string()), "{refused}");
    assert!(store.exists(), "the store is kept, not discarded");
    std::fs::remove_dir_all(&store).unwrap();

    // A `git` that refuses one step. Each of the three is the one that
    // travels, and each keeps the store.
    for needle in ["fetch", "update-ref -m", "update-ref -d"] {
        let staged = stage_seat_store(&repo.scope).unwrap();
        commit(&staged);
        let store = staged.store_path().to_path_buf();
        let anchor = staged.promotion_ref.clone();
        let shim = fake_git(dir.path(), needle);
        let refused =
            promote_seat_commits_with(&shim.to_string_lossy(), staged, &repo.scope).unwrap_err();
        assert!(refused.contains("fake git refusing"), "{needle}: {refused}");
        assert!(refused.contains(&store.display().to_string()), "{refused}");
        assert!(store.exists(), "{needle}: the store is kept");
        std::fs::remove_dir_all(&store).unwrap();
        repo.git(&repo.main, &["update-ref", "-d", &anchor, "--no-deref"]);
        repo.git(&repo.main, &["update-ref", "refs/heads/slice", &repo.base]);
    }

    // A store the driver cannot take back from the box is refused BEFORE
    // any git reads it: the whole point of the reclaim is that no trusted
    // git touches a store the driver has not re-authored.
    let staged = stage_seat_store(&repo.scope).unwrap();
    commit(&staged);
    let store = staged.store_path().to_path_buf();
    std::fs::remove_dir_all(&store).unwrap();
    let refused = promote_seat_commits(staged, &repo.scope).unwrap_err();
    assert!(
        refused.contains("take the seat's private git store back"),
        "{refused}"
    );

    // A store whose branch the seat deleted promotes nothing rather than
    // failing: there is no commit to move to. The ref is deleted through
    // git, so a copy that arrived PACKED goes with the loose one.
    let staged = stage_seat_store(&repo.scope).unwrap();
    Repo::run(
        staged.store_path(),
        &[
            "--git-dir",
            &staged.store_path().to_string_lossy(),
            "update-ref",
            "-d",
            "refs/heads/slice",
        ],
    );
    assert_eq!(
        repo.git(
            staged.store_path(),
            &[
                "--git-dir",
                &staged.store_path().to_string_lossy(),
                "rev-parse",
                "--verify",
                "--quiet",
                "refs/heads/slice",
            ],
        ),
        "",
        "the store really has no such ref"
    );
    assert_eq!(promote_seat_commits(staged, &repo.scope).unwrap(), None);
}

/// The branch is the seat's for the seat's WHOLE life, not for the
/// instant the promotion looks at it. A host that moved the branch while
/// the seat ran — an operator, a fetch, a push — is not overwritten: the
/// promotion compares the host against the baseline the driver recorded
/// before the seat started, refuses, and keeps the store its commits are
/// in.
#[test]
fn a_branch_the_host_moved_while_the_seat_ran_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());
    let staged = stage_seat_store(&repo.scope).unwrap();
    assert_eq!(
        staged.baseline(),
        Some(repo.base.as_str()),
        "the baseline is where the host's branch stood at staging"
    );
    let store = staged.store_path().to_path_buf();

    // The seat commits inside its private store, exactly as it would in
    // the box.
    std::fs::write(repo.worktree.join("b.txt"), "b\n").unwrap();
    repo.git_with_common(&staged, &["add", "b.txt"]);
    repo.git_with_common(&staged, &["commit", "-q", "-m", "boxed"]);
    let committed = repo.git_with_common(&staged, &["rev-parse", "HEAD"]);

    // Meanwhile, on the host, the branch moves.
    let tree = repo.git(&repo.main, &["rev-parse", "HEAD^{tree}"]);
    let concurrent = Repo::run(
        &repo.main,
        &["commit-tree", &tree, "-p", &repo.base, "-m", "host work"],
    );
    Repo::run(
        &repo.main,
        &["update-ref", "refs/heads/slice", &concurrent, &repo.base],
    );

    let refused = promote_seat_commits(staged, &repo.scope).unwrap_err();
    assert!(refused.contains("moved on the host"), "{refused}");
    assert!(refused.contains(&repo.base), "{refused}");
    assert!(refused.contains(&concurrent), "{refused}");
    assert!(refused.contains(&store.display().to_string()), "{refused}");
    // The host keeps what the host wrote, and the seat keeps its commits.
    assert_eq!(repo.host_ref("refs/heads/slice"), concurrent);
    assert!(store.exists(), "the store is kept, not discarded");
    assert_eq!(
        Repo::run(
            &store,
            &[
                "--git-dir",
                &store.to_string_lossy(),
                "rev-parse",
                "refs/heads/slice",
            ],
        ),
        committed,
        "the seat's commit is still readable where the refusal says it is"
    );
    std::fs::remove_dir_all(&store).unwrap();

    // The baseline is per SEAT, not per repository: the next seat starts
    // from where the host stands now, and promotes onto that.
    let next = stage_seat_store(&repo.scope).unwrap();
    assert_eq!(next.baseline(), Some(concurrent.as_str()));
}

/// A worktree of a BARE parent is a layout git itself serves: a bare
/// repository has no working tree, so its `HEAD` is the branch a clone
/// would follow rather than a second claim on the branch. A real
/// checkout on the same branch still refuses, and says which one.
#[test]
fn a_bare_parent_is_not_a_second_checkout_on_the_branch() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());

    let bare = dir.path().join("bare parent.git");
    Repo::run(
        dir.path(),
        &[
            "clone",
            "--bare",
            "-q",
            repo.main.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    let worktree = dir.path().join("bare wt");
    Repo::run(
        &bare,
        &["worktree", "add", "-q", worktree.to_str().unwrap(), "main"],
    );
    let facts = crate::hands::git_facts(&worktree);
    let scope = GitScope {
        workspace: worktree.clone(),
        git_dir: facts.git_dir.clone().unwrap(),
        common_dir: facts.common_dir.clone().unwrap(),
    };
    // Git wrote the same branch into both `HEAD`s, which is why reading
    // them is not enough to tell a checkout from a bare repository.
    assert_eq!(
        head_branch(&scope.common_dir).as_deref(),
        Some("refs/heads/main")
    );
    assert_eq!(
        head_branch(&scope.git_dir).as_deref(),
        Some("refs/heads/main")
    );
    assert!(scope_refusal(&scope).is_none());
    let staged = stage_seat_store(&scope).unwrap();
    assert_eq!(staged.reference(), "refs/heads/main");

    // The same shape on a repository that is NOT bare is the conflict
    // this check exists for, and it names the other checkout.
    std::fs::write(
        repo.scope.common_dir.join("HEAD"),
        "ref: refs/heads/slice\n",
    )
    .unwrap();
    let refused = stage_seat_store(&repo.scope).unwrap_err();
    assert!(refused.contains("owned by neither"), "{refused}");
    assert!(
        refused.contains(&repo.scope.common_dir.display().to_string()),
        "{refused}"
    );
}

/// The private store reproduces the `files` ref backend, and only that
/// one. A `reftable` repository keeps its refs in `<common>/reftable`
/// and writes the placeholder `ref: refs/heads/.invalid` into every
/// `HEAD` file (measured on git 2.51), so copying `refs` and
/// `packed-refs` would hand the seat a store with no branch at all — and
/// a commit on an unborn branch is a root commit the driver would then be
/// asked to promote. The seat is refused before it starts instead.
#[test]
fn a_ref_backend_the_private_store_cannot_reproduce_refuses_at_staging() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::linked(dir.path());
    // No `extensions.refstorage` at all is the `files` backend.
    assert!(stage_seat_store(&repo.scope).is_ok());
    Repo::run(&repo.main, &["config", "core.repositoryformatversion", "1"]);
    Repo::run(&repo.main, &["config", "extensions.refstorage", "files"]);
    assert!(stage_seat_store(&repo.scope).is_ok());

    Repo::run(&repo.main, &["config", "extensions.refstorage", "reftable"]);
    let refused = stage_seat_store(&repo.scope).unwrap_err();
    assert!(refused.contains("reftable"), "{refused}");
    assert!(refused.contains("git refs migrate"), "{refused}");
    assert!(
        refused.contains(&repo.scope.common_dir.display().to_string()),
        "{refused}"
    );
}

/// A `git` that refuses exactly one step and delegates the rest, so the
/// arm that carries each failure is the one under test.
fn fake_git(dir: &Path, needle: &str) -> PathBuf {
    let path = dir.join(format!("fake-git-{}", needle.replace([' ', '-'], "")));
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *'{needle}'*) echo \"fake git refusing: $*\" >&2; exit 1;;\nesac\nexec git \"$@\"\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// A real repository with a linked worktree and a sibling, built by git
/// itself so the layout under test is the one git writes.
struct Repo {
    main: PathBuf,
    worktree: PathBuf,
    sibling: PathBuf,
    scope: GitScope,
    base: String,
}

impl Repo {
    fn linked(root: &Path) -> Self {
        // A nested path with a space, so the argv carries what a host path
        // really can carry.
        let main = root.join("main repo");
        std::fs::create_dir_all(&main).unwrap();
        let git = |cwd: &Path, args: &[&str]| Repo::run(cwd, args);
        git(&main, &["init", "-q", "-b", "main"]);
        git(&main, &["config", "user.name", "Host Operator"]);
        git(&main, &["config", "user.email", "host@example.invalid"]);
        git(&main, &["config", "commit.gpgsign", "true"]);
        git(&main, &["config", "gpg.program", "/nonexistent/gpg"]);
        // A repository that has run sparse-checkout carries this extension,
        // so a per-worktree `config.worktree` is honoured by the host's git.
        git(&main, &["config", "extensions.worktreeConfig", "true"]);
        std::fs::write(main.join("a.txt"), "a\n").unwrap();
        git(&main, &["add", "-A"]);
        git(&main, &["commit", "-q", "--no-gpg-sign", "-m", "base"]);

        let worktree = root.join("wt with space");
        let sibling = root.join("sibling");
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
        git(&main, &["tag", "v1"]);
        // Pack every ref, so the sibling's branch is a PACKED ref rather
        // than a loose file, and add one loose ref beside it: both
        // spellings have to survive the seat.
        git(&main, &["pack-refs", "--all"]);
        let base = git(&main, &["rev-parse", "HEAD"]);
        git(&main, &["update-ref", "refs/heads/loose-sibling", &base]);

        let facts = crate::hands::git_facts(&worktree);
        let scope = GitScope {
            workspace: worktree.clone(),
            git_dir: facts.git_dir.clone().unwrap(),
            common_dir: facts.common_dir.clone().unwrap(),
        };
        Repo {
            main,
            worktree,
            sibling,
            scope,
            base,
        }
    }

    fn run(cwd: &Path, args: &[&str]) -> String {
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
    }

    fn git(&self, cwd: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
            .env("GIT_CONFIG_VALUE_0", "false")
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// One git command in the worktree with the PRIVATE store as its
    /// common directory. The runner makes that redirect by mounting a
    /// `commondir` naming the store over the worktree's own; here the
    /// same file is written and put back, so the promotion is proved on a
    /// host that cannot open a namespace as well as on one that can.
    fn git_with_common(&self, staged: &SeatGitStore, args: &[&str]) -> String {
        let commondir = self.scope.git_dir.join("commondir");
        let real = std::fs::read(&commondir).unwrap();
        std::fs::write(&commondir, format!("{}\n", staged.store_path().display())).unwrap();
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.worktree)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
            .env("GIT_CONFIG_VALUE_0", "false")
            .env("GIT_AUTHOR_NAME", "Host Operator")
            .env("GIT_AUTHOR_EMAIL", "host@example.invalid")
            .env("GIT_COMMITTER_NAME", "Host Operator")
            .env("GIT_COMMITTER_EMAIL", "host@example.invalid")
            .output()
            .unwrap();
        std::fs::write(&commondir, real).unwrap();
        assert!(
            out.status.success(),
            "git {args:?} in the private store: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn host_ref(&self, reference: &str) -> String {
        self.git(&self.main, &["rev-parse", "--verify", "--quiet", reference])
    }

    /// Every byte of the shared store a seat must not be able to move.
    fn shared_state(&self) -> Vec<(String, String)> {
        let common = self.main.join(".git");
        let mut state = vec![
            (
                "packed-refs".to_string(),
                std::fs::read_to_string(common.join("packed-refs")).unwrap_or_default(),
            ),
            (
                "sibling-head".to_string(),
                std::fs::read_to_string(common.join("worktrees/sibling/HEAD")).unwrap_or_default(),
            ),
            (
                "config".to_string(),
                std::fs::read_to_string(common.join("config")).unwrap_or_default(),
            ),
        ];
        for reference in [
            "refs/heads/sibling",
            "refs/heads/loose-sibling",
            "refs/heads/main",
            "refs/tags/v1",
        ] {
            state.push((reference.to_string(), self.host_ref(reference)));
        }
        let mut objects: Vec<String> = walk(&common.join("objects"));
        objects.sort();
        state.push(("objects".to_string(), objects.join("\n")));
        state
    }
}

/// Every file under one directory, relative and sorted by the caller.
fn walk(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path.display().to_string());
        }
    }
    found
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
/// provider's profile confines the worktree; the runner adds the
/// per-worktree directory and a PRIVATE common directory, so a linked
/// worktree commits while the shared repository stays read-only —
/// objects, refs, packed refs, reflogs, hooks, config, every sibling
/// worktree. The assertions read the HOST back: a marker printed inside
/// the box would only prove what the box believes, not what landed on
/// the host.
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

    let repo = Repo::linked(dir.path());
    let main = repo.main.clone();
    let common = main.join(".git");
    let worktree = repo.worktree.clone();
    let sibling = repo.sibling.clone();
    std::fs::write(main.join("parent-sentinel"), "parent\n").unwrap();
    std::fs::write(sibling.join("sibling-sentinel"), "sibling\n").unwrap();
    // A hook the HOST would run. If the seat's commit succeeds, the host's
    // hooks were never on the seat's hook path.
    std::fs::write(
        common.join("hooks/pre-commit"),
        "#!/bin/sh\necho HOST_HOOK_RAN\nexit 1\n",
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            common.join("hooks/pre-commit"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }

    // The layout git itself wrote is the one layout the checks serve.
    assert!(scope_refusal(&repo.scope).is_none());
    let git_dir = repo.scope.git_dir.clone();
    let commondir_file = git_dir.join("commondir");
    let gitdir_file = git_dir.join("gitdir");
    let head_file = git_dir.join("HEAD");
    let commondir_before = std::fs::read_to_string(&commondir_file).unwrap();
    let gitdir_before = std::fs::read_to_string(&gitdir_file).unwrap();
    let head_before = std::fs::read_to_string(&head_file).unwrap();
    // Every object the repository held before any seat ran. A promotion
    // ADDS objects; nothing a seat does may take one away.
    let objects_at_start: Vec<String> = repo
        .shared_state()
        .into_iter()
        .filter(|(name, _)| name == "objects")
        .flat_map(|(_, files)| files.lines().map(str::to_string).collect::<Vec<_>>())
        .collect();

    // Two boxed sessions in the same worktree, the way implement, verify
    // and review reuse one: the second is the "later malicious session"
    // that finds a repository a promotion has already touched.
    let mut committed = String::new();
    for session in ["first", "second"] {
        // What the shared repository holds going INTO this session. The
        // second session opens on a repository a promotion has already
        // written to, which is the "later malicious seat" case.
        let shared_before = repo.shared_state();
        // The first session's store is rooted in `/tmp` on purpose: dsh's
        // own profile replaces `/tmp` with a fresh tmpfs, and the runner's
        // bind comes AFTER it in the argv, so this is what proves the
        // seat's writes reach the HOST's store rather than a tmpfs that
        // dies with the box. The second uses the host's temporary
        // directory wherever that is, which on a machine with `TMPDIR`
        // set is not `/tmp` at all.
        let staged = if session == "first" {
            stage_seat_store_with("git", &repo.scope, || {
                tempfile::Builder::new()
                    .prefix("brokkr-dsh-git-")
                    .tempdir_in("/tmp")
            })
            .unwrap()
        } else {
            stage_seat_store(&repo.scope).unwrap()
        };
        assert!(
            session != "first" || staged.store_path().starts_with("/tmp"),
            "the first session's store must sit under the tmpfs the profile creates"
        );
        let store = staged.store_path().to_path_buf();
        let trusted = staged.trusted_path().to_path_buf();
        let script = boxed_script(&repo, &store, &trusted, session);
        let argv = runner_argv(&runner_args_for(
            &bwrap,
            &repo.scope,
            &store,
            &trusted,
            &dsh_workspace_write_profile(&worktree),
            &["bash", "-lc", &script],
        ))
        .unwrap();
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
        assert!(run.status.success(), "{session}: {stdout}\n{stderr}");
        // The mask is readable inside the box: an unreadable one makes git
        // answer `fatal: unknown error occurred while reading the
        // configuration files` for every command, which is what a device
        // node did before this fix.
        assert!(!stderr.contains("unable to access"), "{session}: {stderr}");
        assert!(!stdout.contains("HOST_HOOK_RAN"), "{session}: {stdout}");

        // The commit is real and unsigned, and it is in the PRIVATE store:
        // the host's branch has not moved yet.
        assert!(stdout.contains("boxed N"), "{session}: {stdout}");
        assert!(
            stdout.contains("author=Host Operator <host@example.invalid>"),
            "{session}: {stdout}"
        );
        assert_eq!(
            repo.host_ref("refs/heads/slice"),
            if session == "first" {
                repo.base.clone()
            } else {
                committed.clone()
            },
            "{session}: the shared branch moves only in the promotion"
        );

        // Everything the boundary promises, measured inside the box.
        for expected in [
            "PARENT_READONLY",
            "SIBLING_READONLY",
            "SIBLING_GIT_READONLY",
            "SIBLING_CONFIG_READONLY",
            "SIBLING_REF_UNTOUCHED_ON_HOST",
            "HOST_REF_READONLY",
            "HOST_PACKED_READONLY",
            "HOST_LOCK_REFUSED",
            "HOST_REFS_DIR_READONLY",
            "OBJECTS_READONLY",
            "OBJECTS_INTACT",
            "ALTERNATES_READONLY",
            "STORE_ALTERNATES_READONLY",
            "LOGS_READONLY",
            "CONFIG_READONLY",
            "WORKTREE_CONFIG_READONLY",
            "STORE_WORKTREE_CONFIG_READONLY",
            "HOOK_REFUSED",
            "hooks=0",
            "COMMONDIR_INTACT",
            "COMMONDIR_READONLY",
            "COMMONDIR_STAYS",
            "GITDIR_READONLY",
            "GITDIR_STAYS",
            "HEAD_READONLY",
            "HEAD_STAYS",
            "HEAD_HELD",
            "HEAD_KEPT",
            "TRUSTED_UNREACHABLE",
            "STORE_COMMONDIR_PLANTED",
            "STORE_SHALLOW_PLANTED",
            "STORE_GRAFTS_PLANTED",
        ] {
            assert!(stdout.contains(expected), "{session}: {expected}\n{stdout}");
        }
        assert!(!stdout.contains("WORKTREE_CONFIG_WRITABLE"), "{stdout}");
        // The store the trusted driver reads afterwards carries no
        // configuration the box chose: the mount point bubblewrap made
        // for the mask is an empty file, and it stays one.
        assert!(
            std::fs::read_to_string(store.join("config.worktree"))
                .unwrap_or_default()
                .is_empty(),
            "{session}: the store's worktree config is the seat's to fill"
        );
        // The box's git really did follow the private store, so every
        // shared write above went there rather than nowhere.
        assert!(
            stdout.contains(&format!("common={}", store.display())),
            "{session}: {stdout}"
        );
        // The seat's own branch moved INSIDE the store, and so did the
        // sibling's — which is exactly why nothing but the owned ref is
        // promoted out of it.
        assert!(
            stdout.contains("STORE_SIBLING_MOVED"),
            "{session}: {stdout}"
        );

        // The shared repository is byte-for-byte what the host left.
        assert_eq!(repo.shared_state(), shared_before, "{session}");
        for masked in ["config.worktree", "config"] {
            let seen = std::fs::read_to_string(git_dir.join(masked)).unwrap_or_default();
            assert!(seen.is_empty(), "{session} {masked}: {seen}");
        }
        assert_eq!(
            std::fs::read_to_string(trusted.join(MASK_FILE)).unwrap(),
            ""
        );
        assert_eq!(
            std::fs::read_to_string(&commondir_file).unwrap(),
            commondir_before
        );
        assert_eq!(
            std::fs::read_to_string(&gitdir_file).unwrap(),
            gitdir_before
        );
        // The branch this worktree owns is still the one the HOST wrote,
        // so the next seat's promotion cannot have been retargeted.
        assert_eq!(std::fs::read_to_string(&head_file).unwrap(), head_before);
        assert_eq!(
            std::fs::read_to_string(main.join("parent-sentinel")).unwrap(),
            "parent\n"
        );
        assert_eq!(
            std::fs::read_to_string(sibling.join("sibling-sentinel")).unwrap(),
            "sibling\n"
        );
        // The host still resolves the real common directory: nothing the
        // box wrote redirected it.
        assert_eq!(
            repo.git(
                &worktree,
                &["rev-parse", "--path-format=absolute", "--git-common-dir"]
            ),
            common.display().to_string()
        );

        // The store the box just held really can steer a trusted git: the
        // `commondir` the seat wrote makes `git --git-dir=<store>` report
        // the SEAT's directory as the common one, read the SEAT's
        // configuration out of it, and answer with the SEAT's value for
        // the branch about to be promoted. The reclaim inside the
        // promotion is what stands between that and the shared
        // repository.
        let store_git = |args: &[&str]| {
            let mut argv = vec!["--git-dir", store.to_str().unwrap()];
            argv.extend_from_slice(args);
            repo.git(&main, &argv)
        };
        assert_eq!(
            store_git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]),
            worktree.join("evil").display().to_string(),
            "{session}: the box could not plant a commondir, so this proves nothing"
        );
        assert_eq!(
            store_git(&["config", "--get", "core.hooksPath"]),
            "/evil-hooks",
            "{session}"
        );

        // And now the driver promotes the one ref the worktree owns. The
        // seat aimed `<store>/commondir` at a common directory whose
        // `refs/heads/slice` is the branch's CURRENT host value, so a
        // promotion that read the store the box left would find nothing
        // to move and answer `None` — this `unwrap` is the assertion.
        let promotion = promote_seat_commits(staged, &repo.scope).unwrap().unwrap();
        assert_eq!(promotion.reference, "refs/heads/slice");
        committed = promotion.to.clone();
        assert_eq!(repo.host_ref("refs/heads/slice"), committed);
        assert_eq!(repo.git(&worktree, &["log", "-1", "--format=%s"]), "boxed");
        // The sibling's branch, the tag and the packed refs are still the
        // host's, after a promotion as well as after the box.
        let mut expected = shared_before.clone();
        expected.retain(|(name, _)| name != "objects");
        let mut seen = repo.shared_state();
        seen.retain(|(name, _)| name != "objects");
        assert_eq!(seen, expected, "{session}: after the promotion");
        // A promotion adds objects and never removes one: the store the
        // repository had before any seat ran is still whole.
        let now = repo
            .shared_state()
            .into_iter()
            .find(|(name, _)| name == "objects")
            .map(|(_, files)| files)
            .unwrap_or_default();
        for object in &objects_at_start {
            assert!(now.contains(object), "{session}: {object} was destroyed");
        }
        // A history boundary the seat wrote into its own store is not the
        // shared repository's: `shallow` there would truncate the parent's
        // history for every worktree.
        assert!(!common.join("shallow").exists(), "{session}");
        repo.git(&main, &["fsck", "--no-progress", "--no-dangling"]);
    }

    // The redirect a hostile seat would try, against the real host: a
    // `.git` file and a fake administrative directory written inside the
    // workspace, naming this repository as the common directory. Git
    // resolves it, and the runner refuses it.
    let staged = stage_seat_store(&repo.scope).unwrap();
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
        format!("{}\n", common.display()),
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
        std::fs::canonicalize(&common).unwrap()
    );
    let refuse = |scope: &GitScope, workspace: &Path| {
        runner_argv(&runner_args_for(
            &bwrap,
            scope,
            staged.store_path(),
            staged.trusted_path(),
            &dsh_workspace_write_profile(workspace),
            &["true"],
        ))
        .unwrap_err()
    };
    assert!(refuse(&hostile_scope, &hostile).contains("administrative directories"));

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
    // Git really does hand back the victim's metadata through the alias.
    assert_eq!(
        std::fs::canonicalize(&alias_scope.git_dir).unwrap(),
        std::fs::canonicalize(&git_dir).unwrap()
    );
    assert_eq!(
        std::fs::canonicalize(&alias_scope.common_dir).unwrap(),
        std::fs::canonicalize(&common).unwrap()
    );
    assert!(refuse(&alias_scope, &alias).contains("symbolic link"));
    std::fs::remove_file(alias.join(".git")).unwrap();
    std::fs::copy(worktree.join(".git"), alias.join(".git")).unwrap();
    let refused = refuse(&alias_scope, &alias);
    assert!(refused.contains("another worktree's metadata"), "{refused}");
    assert!(
        refused.contains(&worktree.display().to_string()),
        "the refusal names the worktree that owns the metadata: {refused}"
    );

    // An ordinary standalone repository still works under the runner: no
    // scoped bind is added and the workspace bind covers its `.git`.
    let standalone = dir.path().join("standalone");
    std::fs::create_dir_all(&standalone).unwrap();
    Repo::run(&standalone, &["init", "-q", "-b", "main"]);
    Repo::run(&standalone, &["config", "user.name", "Host Operator"]);
    Repo::run(
        &standalone,
        &["config", "user.email", "host@example.invalid"],
    );
    let facts = crate::hands::git_facts(&standalone);
    let scope = GitScope {
        workspace: standalone.clone(),
        git_dir: facts.git_dir.clone().unwrap(),
        common_dir: facts.common_dir.clone().unwrap(),
    };
    let script =
        "echo s > s.txt && git add s.txt && git commit -q -m standalone && git log -1 --format=%s";
    let argv = runner_argv(&runner_args_for(
        &bwrap,
        &scope,
        staged.store_path(),
        staged.trusted_path(),
        &dsh_workspace_write_profile(&standalone),
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
    // A standalone repository's own commit really did land on its branch:
    // the runner added nothing, and the provider's workspace bind was
    // enough.
    assert_eq!(
        Repo::run(&standalone, &["log", "-1", "--format=%s"]),
        "standalone"
    );
}

/// What the boxed seat does: commit its own work, then try every way a
/// hostile seat could reach the shared repository. Each answer is a word
/// the host reads back, and every claim is re-checked against the host's
/// own bytes afterwards.
#[cfg(target_os = "linux")]
fn boxed_script(repo: &Repo, store: &Path, trusted: &Path, session: &str) -> String {
    let common = repo.main.join(".git");
    let git_dir = repo.scope.git_dir.clone();
    format!(
        "set -e\n\
         echo {session} > b.txt\n\
         git add b.txt\n\
         git commit -q -m boxed\n\
         git log -1 --format='%s %G?'\n\
         echo \"author=$(git log -1 --format='%an <%ae>')\"\n\
         echo \"common=$(git rev-parse --path-format=absolute --git-common-dir)\"\n\
         # the seat's own branch and a sibling's both move — inside the store\n\
         git update-ref refs/heads/sibling \"$(git rev-parse HEAD)\"\n\
         git branch -f loose-sibling HEAD\n\
         git tag -f v1 HEAD\n\
         git update-ref refs/remotes/origin/main HEAD\n\
         git pack-refs --all\n\
         if [ \"$(git rev-parse refs/heads/sibling)\" = \"$(git rev-parse HEAD)\" ]; then echo STORE_SIBLING_MOVED; fi\n\
         if [ \"$(cat '{common}/packed-refs' | grep -c ' refs/heads/sibling$')\" = 1 ]; then echo SIBLING_REF_UNTOUCHED_ON_HOST; fi\n\
         # the parent checkout, the sibling worktree and its metadata\n\
         if echo x > '{parent}' 2>/dev/null; then echo PARENT_WRITABLE; else echo PARENT_READONLY; fi\n\
         if echo x > '{sibling_sentinel}' 2>/dev/null; then echo SIBLING_WRITABLE; else echo SIBLING_READONLY; fi\n\
         if echo x > '{common}/worktrees/sibling/HEAD' 2>/dev/null; then echo SIBLING_GIT_WRITABLE; else echo SIBLING_GIT_READONLY; fi\n\
         if echo x > '{common}/worktrees/sibling/config.worktree' 2>/dev/null; then echo SIBLING_CONFIG_WRITABLE; else echo SIBLING_CONFIG_READONLY; fi\n\
         # the shared ref store, at every spelling git uses\n\
         if echo x > '{common}/refs/heads/loose-sibling' 2>/dev/null; then echo HOST_REF_WRITABLE; else echo HOST_REF_READONLY; fi\n\
         if echo x > '{common}/packed-refs' 2>/dev/null; then echo HOST_PACKED_WRITABLE; else echo HOST_PACKED_READONLY; fi\n\
         if echo x > '{common}/refs/heads/sibling.lock' 2>/dev/null; then echo HOST_LOCK_TAKEN; else echo HOST_LOCK_REFUSED; fi\n\
         if mkdir '{common}/refs/heads/evil' 2>/dev/null; then echo HOST_REFS_DIR_WRITABLE; else echo HOST_REFS_DIR_READONLY; fi\n\
         if echo x > '{common}/logs/HEAD' 2>/dev/null; then echo LOGS_WRITABLE; else echo LOGS_READONLY; fi\n\
         # the shared object store, and the alternates that point at it\n\
         if echo x > '{common}/objects/info/packs' 2>/dev/null; then echo OBJECTS_WRITABLE; else echo OBJECTS_READONLY; fi\n\
         rm -rf '{common}/objects/pack' 2>/dev/null || true\n\
         if [ -d '{common}/objects' ]; then echo OBJECTS_INTACT; fi\n\
         if echo /evil > '{common}/objects/info/alternates' 2>/dev/null; then echo ALTERNATES_WRITABLE; else echo ALTERNATES_READONLY; fi\n\
         if echo /evil > '{store}/objects/info/alternates' 2>/dev/null; then echo STORE_ALTERNATES_WRITABLE; else echo STORE_ALTERNATES_READONLY; fi\n\
         # hooks and config, on both paths git would read them from\n\
         if echo x > '{common}/hooks/post-commit' 2>/dev/null; then echo HOOK_PLANTED; else echo HOOK_REFUSED; fi\n\
         echo \"hooks=$(ls '{store}/hooks' | wc -l)\"\n\
         if git config --local core.hooksPath /evil 2>/dev/null; then echo CONFIG_WRITABLE; else echo CONFIG_READONLY; fi\n\
         if git config --worktree core.hooksPath /evil 2>/dev/null; then echo WORKTREE_CONFIG_WRITABLE; else echo WORKTREE_CONFIG_READONLY; fi\n\
         # the store's own worktree config, which the TRUSTED driver reads\n\
         # afterwards: `<store>/config` is a copy of the shared one, so it\n\
         # carries `extensions.worktreeConfig` where the repository has it\n\
         if echo x > '{store}/config.worktree' 2>/dev/null; then echo STORE_WORKTREE_CONFIG_WRITABLE; else echo STORE_WORKTREE_CONFIG_READONLY; fi\n\
         echo evil > '{git_dir}/config.worktree' 2>/dev/null || true\n\
         echo evil > '{git_dir}/config' 2>/dev/null || true\n\
         # the pointers the box must not move, and the staged files behind them\n\
         echo evil > '{git_dir}/probe-scratch'\n\
         if mv '{git_dir}/probe-scratch' '{commondir}' 2>/dev/null; then echo COMMONDIR_REPLACED; else echo COMMONDIR_INTACT; fi\n\
         rm -f '{git_dir}/probe-scratch'\n\
         if echo x > '{commondir}' 2>/dev/null; then echo COMMONDIR_WRITABLE; else echo COMMONDIR_READONLY; fi\n\
         if rm -f '{commondir}' 2>/dev/null; then echo COMMONDIR_UNLINKED; else echo COMMONDIR_STAYS; fi\n\
         if echo x > '{gitdir}' 2>/dev/null; then echo GITDIR_WRITABLE; else echo GITDIR_READONLY; fi\n\
         if rm -f '{gitdir}' 2>/dev/null; then echo GITDIR_UNLINKED; else echo GITDIR_STAYS; fi\n\
         # HEAD names the branch this worktree owns, and that is the ref the\n\
         # driver promotes: retargeting it would aim the NEXT seat's honest\n\
         # commits at a branch this one chose.\n\
         if printf 'ref: refs/heads/sibling\\n' > '{head}' 2>/dev/null; then echo HEAD_WRITABLE; else echo HEAD_READONLY; fi\n\
         if git symbolic-ref HEAD refs/heads/sibling 2>/dev/null; then echo HEAD_RETARGETED; else echo HEAD_STAYS; fi\n\
         if git checkout -q sibling 2>/dev/null; then echo HEAD_SWITCHED; else echo HEAD_HELD; fi\n\
         if rm -f '{head}' 2>/dev/null; then echo HEAD_UNLINKED; else echo HEAD_KEPT; fi\n\
         echo evil > '{trusted}/{mask}' 2>/dev/null || true\n\
         echo evil > '{trusted}/{pointer}' 2>/dev/null || true\n\
         if [ ! -s '{trusted}/{mask}' ]; then echo TRUSTED_UNREACHABLE; fi\n\
         # LAST, because it redirects this seat's own git as well: the\n\
         # private store is the seat's to write for its whole life, and the\n\
         # TRUSTED driver reads that directory as a repository afterwards.\n\
         # A `commondir` inside it steers `--git-dir` at a common directory\n\
         # the seat built in its own workspace — config, HEAD, refs and\n\
         # alternates all the seat's.\n\
         git rev-parse 'HEAD^' > '{workspace}/parent-oid'\n\
         mkdir -p '{workspace}/evil/objects/info' '{workspace}/evil/refs/heads'\n\
         printf '[core]\\n\\trepositoryformatversion = 0\\n\\tbare = true\\n\\thooksPath = /evil-hooks\\n' > '{workspace}/evil/config'\n\
         printf 'ref: refs/heads/slice\\n' > '{workspace}/evil/HEAD'\n\
         cp '{workspace}/parent-oid' '{workspace}/evil/refs/heads/slice'\n\
         echo '{common}/objects' > '{workspace}/evil/objects/info/alternates'\n\
         if echo '{workspace}/evil' > '{store}/commondir'; then echo STORE_COMMONDIR_PLANTED; fi\n\
         if cp '{workspace}/parent-oid' '{store}/shallow'; then echo STORE_SHALLOW_PLANTED; fi\n\
         mkdir -p '{store}/info'\n\
         if cp '{workspace}/parent-oid' '{store}/info/grafts'; then echo STORE_GRAFTS_PLANTED; fi\n",
        session = session,
        workspace = repo.worktree.display(),
        parent = repo.main.join("parent-sentinel").display(),
        sibling_sentinel = repo.sibling.join("sibling-sentinel").display(),
        common = common.display(),
        store = store.display(),
        trusted = trusted.display(),
        mask = MASK_FILE,
        pointer = COMMONDIR_FILE,
        git_dir = git_dir.display(),
        commondir = git_dir.join("commondir").display(),
        gitdir = git_dir.join("gitdir").display(),
        head = git_dir.join("HEAD").display(),
    )
}
