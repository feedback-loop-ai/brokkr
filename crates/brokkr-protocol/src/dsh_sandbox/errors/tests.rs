//! The one place each refusal's bytes are pinned (#353): dsh reads the
//! runner's behind its failure signature, and the driver's reach the
//! seat's stderr and its launch refusal.

use super::*;

fn path(spelled: &str) -> PathBuf {
    PathBuf::from(spelled)
}

#[test]
fn every_runner_refusal_reads_as_it_always_has() {
    let spellings = RunnerFlag::ALL.map(|flag| flag.to_string());
    let expected = [
        "--workspace",
        "--git-dir",
        "--common-dir",
        "--bwrap",
        "--store",
        "--trusted",
    ];
    assert_eq!(spellings, expected);
    let scope = ScopeRefusal::SharedGitDir {
        git_dir: path("/g"),
    };
    let staged = StagedRefusal::Empty { path: path("/t/c") };
    let pins = [
        (
            RunnerRefusal::NeedsPath(RunnerFlag::Workspace),
            "dsh sandbox runner: --workspace needs a path after it".to_string(),
        ),
        (
            RunnerRefusal::GivenTwice(RunnerFlag::Bwrap),
            "dsh sandbox runner: --bwrap given twice".into(),
        ),
        (
            RunnerRefusal::Required(RunnerFlag::Trusted),
            "dsh sandbox runner: --trusted is required".into(),
        ),
        (
            RunnerRefusal::NotAbsolute {
                flag: RunnerFlag::Store,
                path: path("rel"),
            },
            "dsh sandbox runner: --store rel is not an absolute path; the runner uses the host \
             paths the driver staged and never a `PATH` or working-directory lookup"
                .into(),
        ),
        (
            RunnerRefusal::NoSeparator,
            "dsh sandbox runner: the profile has no `--` before the command".into(),
        ),
        (
            RunnerRefusal::NotRootedReadOnly,
            "the dsh sandbox profile does not begin with `--ro-bind / /`; refusing to guess the \
             boundary it stands for"
                .into(),
        ),
        (
            RunnerRefusal::UnreadOption("--future".into()),
            "the dsh sandbox profile carries --future, which this runner does not read; it \
             refuses rather than guess how many arguments follow it, which host path the box \
             could write through it, or what further options it carries"
                .into(),
        ),
        (
            RunnerRefusal::Truncated("--bind".into()),
            "the dsh sandbox profile ends inside --bind's arguments".into(),
        ),
        // A scope or staged refusal prints as itself, with nothing added.
        (RunnerRefusal::Scope(scope.clone()), scope.to_string()),
        (RunnerRefusal::Staged(staged.clone()), staged.to_string()),
    ];
    for (refusal, text) in pins {
        assert_eq!(refusal.to_string(), text);
    }
}

#[test]
fn every_scope_refusal_reads_as_it_always_has() {
    let (git_dir, workspace) = (path("/c/wt-admin"), path("/w"));
    let pins = [
        (
            ScopeRefusal::SharedGitDir {
                git_dir: git_dir.clone(),
            },
            "the seat's git directory /c/wt-admin is the shared repository's own git directory, \
             so a commit would need the whole shared .git writable; the scoped runner refuses \
             that (a primary checkout reached through a subdirectory, a `--separate-git-dir` \
             checkout, or a `.git` file redirected at the parent). Run the seat from the \
             repository root, or from a linked `git worktree`",
        ),
        (
            ScopeRefusal::NotAdministrative {
                git_dir: git_dir.clone(),
                administrative: path("/c/worktrees"),
            },
            "the seat's git directory /c/wt-admin is not one of the linked-worktree \
             administrative directories /c/worktrees/<name> that its shared repository owns, so \
             Git reached it through a `.git` file rather than through `git worktree`; the scoped \
             runner refuses to bind a repository a workspace file pointed it at. Run the seat \
             from the repository root, or from a linked `git worktree`",
        ),
        (
            ScopeRefusal::SymlinkedGit {
                workspace_git: path("/w/.git"),
            },
            "this seat's workspace has a symbolic link at /w/.git, and `git worktree` writes a \
             plain file there; the scoped runner refuses a `.git` that is a second name for \
             another worktree's metadata. Run the seat from a linked `git worktree` git itself \
             created",
        ),
        (
            ScopeRefusal::NoBackPointer {
                git_dir: git_dir.clone(),
                workspace: workspace.clone(),
            },
            "the per-worktree git directory /c/wt-admin carries no `gitdir` back-pointer, so \
             nothing outside this seat's workspace ties it to /w; the scoped runner refuses to \
             write git metadata it cannot tie to the seat. Run the seat from a linked `git \
             worktree` git itself created",
        ),
        (
            ScopeRefusal::Subdirectory {
                workspace: path("/w/inner"),
                root: workspace.clone(),
            },
            "this seat's workspace /w/inner is a subdirectory of the linked worktree at /w, and \
             the scoped runner serves a worktree at its own root: the writable root dsh gives \
             the seat would not cover the rest of the worktree. Run the seat from /w",
        ),
        (
            ScopeRefusal::OtherWorktree {
                git_dir,
                recorded: path("/v/.git"),
                workspace,
            },
            "the per-worktree git directory /c/wt-admin records /v/.git as its worktree, not \
             this seat's workspace /w; refusing to bind another worktree's metadata",
        ),
    ];
    for (refusal, text) in pins {
        assert_eq!(refusal.to_string(), text);
    }
}

#[test]
fn every_staged_refusal_reads_as_it_always_has() {
    let pins = [
        (
            StagedRefusal::Writable {
                trusted: path("/tmp/t"),
                root: path("/tmp"),
            },
            "the files the dsh runner mounts read-only are staged in /tmp/t, which lies under \
             /tmp — a path this seat's box can write, so a boxed command could rewrite the very \
             files the mounts stand for",
        ),
        (
            StagedRefusal::Unreadable {
                path: path("/t/mask"),
                cause: "No such file or directory (os error 2)".into(),
            },
            "the staged file /t/mask cannot be read (No such file or directory (os error 2))",
        ),
        (
            StagedRefusal::NotAFile {
                path: path("/t/mask"),
            },
            "the staged file /t/mask is not a regular file; bubblewrap mounts a bind source with \
             MS_NODEV, so a device node masks the path with something the box cannot open and \
             git calls an unreadable config file fatal",
        ),
        (
            StagedRefusal::NotEmpty {
                path: path("/t/mask"),
                bytes: 7,
            },
            "the staged file /t/mask is not empty (7 bytes)",
        ),
        (
            StagedRefusal::Empty {
                path: path("/t/commondir"),
            },
            "the staged file /t/commondir is empty, and git reads it as a path rather than as an \
             absence",
        ),
    ];
    for (refusal, text) in pins {
        assert_eq!(refusal.to_string(), text);
    }
}

#[test]
fn every_store_error_reads_as_it_always_has() {
    let (workspace, reference) = (path("/w"), "refs/heads/slice".to_string());
    let args = ["--git-dir", "/c", "update-ref", "-d", "x"].map(String::from);
    let pins = [
        (
            StoreError::RefBackend {
                common_dir: path("/c"),
                backend: "reftable".into(),
            },
            "the repository at /c stores its refs with git's `reftable` backend, and the scoped \
             dsh runner builds the seat's private common directory by copying the `files` \
             backend's ref tree; it will not hand a seat a store whose branches are missing. Run \
             the seat from a standalone checkout, or convert the repository with `git refs \
             migrate --ref-format=files`",
        ),
        (
            StoreError::Staging("no room".into()),
            "dsh driver: could not stage the seat's private git store: no room",
        ),
        (
            StoreError::NoBranch {
                workspace: workspace.clone(),
            },
            "the worktree at /w has no branch checked out, so there is no ref the seat owns and \
             nothing a commit could be promoted to; the scoped runner promotes exactly the branch \
             a worktree has checked out, because git refuses to check one branch out in two \
             worktrees. Run `git switch -c <branch>` in the worktree before the seat starts",
        ),
        (
            StoreError::BranchTaken {
                workspace,
                reference: reference.clone(),
                other: path("/c"),
            },
            "the worktree at /w has refs/heads/slice checked out, and so does /c; the scoped \
             runner promotes the branch a worktree OWNS, and a branch two checkouts claim is \
             owned by neither. Run `git worktree repair`, or give this seat a branch of its own",
        ),
        (
            StoreError::Reclaim("EACCES".into()),
            "could not take the seat's private git store back from the box before reading it as \
             a repository: EACCES",
        ),
        (
            StoreError::Moved {
                reference,
                baseline: None,
                now: Some("abc".into()),
            },
            "refs/heads/slice moved on the host while the seat ran: it stood at no commit when \
             the seat started and stands at abc now. The driver promotes the seat's commits onto \
             the value the branch had then, and will not overwrite work something else did \
             meanwhile",
        ),
        (
            StoreError::CouldNotRun {
                program: "git".into(),
                args: args.to_vec(),
                cause: "ENOENT".into(),
            },
            "`git --git-dir /c update-ref -d x` could not run: ENOENT",
        ),
        (
            StoreError::GitFailed {
                args: args.to_vec(),
                stderr: "fatal: no".into(),
            },
            "`git --git-dir /c update-ref -d x` failed: fatal: no",
        ),
    ];
    for (error, text) in pins {
        assert_eq!(error.to_string(), text);
    }
}

#[test]
fn every_kept_store_and_row_refusal_reads_as_it_always_has() {
    let reclaimed = KeptStore::Reclaimed {
        problem: "the promotion refused",
        path: path("/s"),
        reference: "refs/heads/slice".into(),
    };
    assert_eq!(
        reclaimed.to_string(),
        "dsh driver: the promotion refused. The seat's commits are not lost: its private git \
         store is kept at /s, and `git --git-dir=/s log refs/heads/slice` still reads them. \
         Nothing removes that directory afterwards; it is the operator's to delete once the \
         commits are safe"
    );
    let unreclaimed = KeptStore::Unreclaimed {
        problem: "the promotion refused",
        path: path("/s"),
        reference: "refs/heads/slice".into(),
        alternates: "/c/objects".into(),
        cause: "EACCES".into(),
    };
    assert_eq!(
        unreclaimed.to_string(),
        "dsh driver: the promotion refused. The seat's commits are not lost: its private git \
         store is kept at /s. But the driver could not take that directory back from the box \
         (EACCES), so it is NOT a repository the driver authored and this refusal will not \
         invite git to read it: a `commondir` the seat left redirects any `--git-dir` at a \
         repository the seat built, whose configuration names the commands git then runs. Take \
         the directory back by hand before reading it — delete every name in it except \
         `objects`, `refs` and `packed-refs`; remove `objects/info` too and recreate it with \
         only `alternates`, then write `/c/objects` into its `objects/info/alternates` and \
         `ref: refs/heads/slice` into its `HEAD` — and it is a repository again, with \
         refs/heads/slice at the seat's commit. Nothing removes that directory afterwards; it \
         is the operator's to delete once the commits are safe"
    );
    assert_eq!(
        RowRefusal::Unusable(path("/opt/bwrap")).to_string(),
        "bubblewrap at /opt/bwrap cannot build the empty-root namespace on this host; the dsh \
         git-metadata runner cannot stand in for it"
    );
    assert_eq!(
        RowRefusal::Multiline("/a\nb".into()).to_string(),
        "dsh sandbox runner: path \"/a\\nb\" spans more than one line"
    );
    // The driver's launch refusal is still text: each converts to its own
    // bytes and nothing else.
    let staging = StoreError::Staging("no room".into());
    assert_eq!(String::from(staging.clone()), staging.to_string());
    let kept = Box::new(KeptStore::Reclaimed {
        problem: staging,
        path: path("/s"),
        reference: "refs/heads/slice".into(),
    });
    assert_eq!(String::from(kept.clone()), kept.to_string());
    let row = RowRefusal::Unusable(path("/b"));
    assert_eq!(String::from(row.clone()), row.to_string());
}
