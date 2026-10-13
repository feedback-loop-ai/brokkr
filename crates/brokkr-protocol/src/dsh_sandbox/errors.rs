//! Why the dsh sandbox runner, the seat's git store and the runner's
//! launch row refuse (#353): each refusal is a variant, and its `Display`
//! is the text dsh, the seat's stderr and the launch refusal have always
//! carried, byte for byte. A host path is carried as a path, git's own
//! words and an I/O failure as their text.

use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

use super::oid_or_absent;

/// A flag the runner reads off its own argv, before the profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerFlag {
    Workspace,
    GitDir,
    CommonDir,
    Bwrap,
    Store,
    Trusted,
}

impl RunnerFlag {
    const ALL: [RunnerFlag; 6] = [
        RunnerFlag::Workspace,
        RunnerFlag::GitDir,
        RunnerFlag::CommonDir,
        RunnerFlag::Bwrap,
        RunnerFlag::Store,
        RunnerFlag::Trusted,
    ];

    /// The flag as the runner's argv spells it.
    pub(super) fn spelling(self) -> &'static str {
        match self {
            RunnerFlag::Workspace => "--workspace",
            RunnerFlag::GitDir => "--git-dir",
            RunnerFlag::CommonDir => "--common-dir",
            RunnerFlag::Bwrap => "--bwrap",
            RunnerFlag::Store => "--store",
            RunnerFlag::Trusted => "--trusted",
        }
    }

    /// The flag one argv token spells, or `None` where the runner's own
    /// flags end and the profile begins.
    pub(super) fn parse(token: &str) -> Option<RunnerFlag> {
        RunnerFlag::ALL
            .into_iter()
            .find(|flag| flag.spelling() == token)
    }
}

impl fmt::Display for RunnerFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spelling())
    }
}

/// Why the runner will not build bubblewrap's argv. dsh reads it behind
/// [`super::RUNNER_FAILURE_SIGNATURE`] as a command that never ran.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunnerRefusal {
    #[error("dsh sandbox runner: {0} needs a path after it")]
    NeedsPath(RunnerFlag),
    #[error("dsh sandbox runner: {0} given twice")]
    GivenTwice(RunnerFlag),
    #[error("dsh sandbox runner: {0} is required")]
    Required(RunnerFlag),
    #[error(
        "dsh sandbox runner: {flag} {} is not an absolute path; the runner uses the host paths \
         the driver staged and never a `PATH` or working-directory lookup",
        .path.display()
    )]
    NotAbsolute { flag: RunnerFlag, path: PathBuf },
    #[error("dsh sandbox runner: the profile has no `--` before the command")]
    NoSeparator,
    #[error(
        "the dsh sandbox profile does not begin with `--ro-bind / /`; refusing to guess the \
         boundary it stands for"
    )]
    NotRootedReadOnly,
    /// A bubblewrap option the runner does not read, as the profile spelt it.
    #[error(
        "the dsh sandbox profile carries {0}, which this runner does not read; it refuses rather \
         than guess how many arguments follow it, which host path the box could write through \
         it, or what further options it carries"
    )]
    UnreadOption(String),
    /// The profile ends inside this option's arguments.
    #[error("the dsh sandbox profile ends inside {0}'s arguments")]
    Truncated(String),
    #[error("{0}")]
    Scope(ScopeRefusal),
    #[error("{0}")]
    Staged(StagedRefusal),
}

/// Why a git scope cannot be served under the declared boundary
/// (decision 0054's threat model): each layout that is not the linked
/// worktree Git itself created, by its own cause.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScopeRefusal {
    #[error(
        "the seat's git directory {} is the shared repository's own git directory, so a commit \
         would need the whole shared .git writable; the scoped runner refuses that (a primary \
         checkout reached through a subdirectory, a `--separate-git-dir` checkout, or a `.git` \
         file redirected at the parent). Run the seat from the repository root, or from a \
         linked `git worktree`",
        .git_dir.display()
    )]
    SharedGitDir { git_dir: PathBuf },
    /// `administrative` is `<common>/worktrees`, the parent Git would have
    /// placed the git directory under.
    #[error(
        "the seat's git directory {} is not one of the linked-worktree administrative \
         directories {} that its shared repository owns, so Git reached it through a `.git` \
         file rather than through `git worktree`; the scoped runner refuses to bind a \
         repository a workspace file pointed it at. Run the seat from the repository root, or \
         from a linked `git worktree`",
        .git_dir.display(),
        .administrative.join("<name>").display()
    )]
    NotAdministrative {
        git_dir: PathBuf,
        administrative: PathBuf,
    },
    #[error(
        "this seat's workspace has a symbolic link at {}, and `git worktree` writes a plain file \
         there; the scoped runner refuses a `.git` that is a second name for another worktree's \
         metadata. Run the seat from a linked `git worktree` git itself created",
        .workspace_git.display()
    )]
    SymlinkedGit { workspace_git: PathBuf },
    #[error(
        "the per-worktree git directory {} carries no `gitdir` back-pointer, so nothing outside \
         this seat's workspace ties it to {}; the scoped runner refuses to write git metadata it \
         cannot tie to the seat. Run the seat from a linked `git worktree` git itself created",
        .git_dir.display(),
        .workspace.display()
    )]
    NoBackPointer {
        git_dir: PathBuf,
        workspace: PathBuf,
    },
    #[error(
        "this seat's workspace {} is a subdirectory of the linked worktree at {}, and the scoped \
         runner serves a worktree at its own root: the writable root dsh gives the seat would \
         not cover the rest of the worktree. Run the seat from {}",
        .workspace.display(),
        .root.display(),
        .root.display()
    )]
    Subdirectory { workspace: PathBuf, root: PathBuf },
    #[error(
        "the per-worktree git directory {} records {} as its worktree, not this seat's workspace \
         {}; refusing to bind another worktree's metadata",
        .git_dir.display(),
        .recorded.display(),
        .workspace.display()
    )]
    OtherWorktree {
        git_dir: PathBuf,
        recorded: PathBuf,
        workspace: PathBuf,
    },
}

/// Why the files the driver staged cannot be mounted read-only.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StagedRefusal {
    /// `root` is a path the box can write that holds the staged directory.
    #[error(
        "the files the dsh runner mounts read-only are staged in {}, which lies under {} — a \
         path this seat's box can write, so a boxed command could rewrite the very files the \
         mounts stand for",
        .trusted.display(),
        .root.display()
    )]
    Writable { trusted: PathBuf, root: PathBuf },
    #[error("the staged file {} cannot be read ({cause})", .path.display())]
    Unreadable { path: PathBuf, cause: String },
    #[error(
        "the staged file {} is not a regular file; bubblewrap mounts a bind source with \
         MS_NODEV, so a device node masks the path with something the box cannot open and git \
         calls an unreadable config file fatal",
        .path.display()
    )]
    NotAFile { path: PathBuf },
    #[error("the staged file {} is not empty ({bytes} bytes)", .path.display())]
    NotEmpty { path: PathBuf, bytes: u64 },
    #[error(
        "the staged file {} is empty, and git reads it as a path rather than as an absence",
        .path.display()
    )]
    Empty { path: PathBuf },
}

/// Why the seat's private git store cannot be staged or its commits
/// promoted.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StoreError {
    #[error(
        "the repository at {} stores its refs with git's `{backend}` backend, and the scoped dsh \
         runner builds the seat's private common directory by copying the `files` backend's ref \
         tree; it will not hand a seat a store whose branches are missing. Run the seat from a \
         standalone checkout, or convert the repository with `git refs migrate \
         --ref-format=files`",
        .common_dir.display()
    )]
    RefBackend {
        common_dir: PathBuf,
        backend: String,
    },
    #[error("dsh driver: could not stage the seat's private git store: {0}")]
    Staging(String),
    #[error(
        "the worktree at {} has no branch checked out, so there is no ref the seat owns and \
         nothing a commit could be promoted to; the scoped runner promotes exactly the branch a \
         worktree has checked out, because git refuses to check one branch out in two \
         worktrees. Run `git switch -c <branch>` in the worktree before the seat starts",
        .workspace.display()
    )]
    NoBranch { workspace: PathBuf },
    /// `other` is the main checkout or sibling worktree on the same branch.
    #[error(
        "the worktree at {} has {reference} checked out, and so does {}; the scoped runner \
         promotes the branch a worktree OWNS, and a branch two checkouts claim is owned by \
         neither. Run `git worktree repair`, or give this seat a branch of its own",
        .workspace.display(),
        .other.display()
    )]
    BranchTaken {
        workspace: PathBuf,
        reference: String,
        other: PathBuf,
    },
    #[error(
        "could not take the seat's private git store back from the box before reading it as a \
         repository: {0}"
    )]
    Reclaim(String),
    /// The branch stood at `baseline` when the seat started and stands at
    /// `now`; `None` is a branch with no commit.
    #[error(
        "{reference} moved on the host while the seat ran: it stood at {} when the seat started \
         and stands at {} now. The driver promotes the seat's commits onto the value the branch \
         had then, and will not overwrite work something else did meanwhile",
        oid_or_absent(.baseline),
        oid_or_absent(.now)
    )]
    Moved {
        reference: String,
        baseline: Option<String>,
        now: Option<String>,
    },
    #[error("`{program} {}` could not run: {cause}", .args.join(" "))]
    CouldNotRun {
        program: String,
        args: Vec<String>,
        cause: String,
    },
    #[error("`git {}` failed: {stderr}", .args.join(" "))]
    GitFailed { args: Vec<String>, stderr: String },
}

/// The driver's refusal that KEEPS a seat's private store, naming where
/// its commits are: `problem` is the failure that kept it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeptStore<P> {
    /// The reclaim succeeded, so the refusal says how to read the commits.
    #[error(
        "dsh driver: {problem}. The seat's commits are not lost: its private git store is kept \
         at {}, and `git --git-dir={} log {reference}` still reads them. Nothing removes that \
         directory afterwards; it is the operator's to delete once the commits are safe",
        .path.display(),
        .path.display()
    )]
    Reclaimed {
        problem: P,
        path: PathBuf,
        reference: String,
    },
    /// The reclaim failed with `cause`, so the refusal hands it over in
    /// words, ending in the `alternates` line the driver would have written.
    #[error(
        "dsh driver: {problem}. The seat's commits are not lost: its private git store is kept \
         at {}. But the driver could not take that directory back from the box ({cause}), so it \
         is NOT a repository the driver authored and this refusal will not invite git to read \
         it: a `commondir` the seat left redirects any `--git-dir` at a repository the seat \
         built, whose configuration names the commands git then runs. Take the directory back \
         by hand before reading it — delete every name in it except `objects`, `refs` and \
         `packed-refs`; remove `objects/info` too and recreate it with only `alternates`, then \
         write `{alternates}` into its `objects/info/alternates` and `ref: {reference}` into its \
         `HEAD` — and it is a repository again, with {reference} at the seat's commit. Nothing \
         removes that directory afterwards; it is the operator's to delete once the commits are \
         safe",
        .path.display()
    )]
    Unreclaimed {
        problem: P,
        path: PathBuf,
        reference: String,
        alternates: String,
        cause: String,
    },
}

/// Why the runner's bubblewrap or its `--patch` row cannot be handed to
/// dsh.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RowRefusal {
    #[error(
        "bubblewrap at {} cannot build the empty-root namespace on this host; the dsh \
         git-metadata runner cannot stand in for it",
        .0.display()
    )]
    Unusable(PathBuf),
    /// A path, as the row would have quoted it, that spans a line.
    #[error("dsh sandbox runner: path {0:?} spans more than one line")]
    Multiline(String),
}

/// The launch refusal is still text in `adapters.rs` (decision 0065 slice
/// two holds that file), so each of these converts at that boundary.
impl From<StoreError> for String {
    fn from(error: StoreError) -> Self {
        error.to_string()
    }
}

impl From<Box<KeptStore<StoreError>>> for String {
    fn from(kept: Box<KeptStore<StoreError>>) -> Self {
        kept.to_string()
    }
}

impl From<RowRefusal> for String {
    fn from(refusal: RowRefusal) -> Self {
        refusal.to_string()
    }
}

#[cfg(test)]
mod tests;
