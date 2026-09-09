//! The dsh harness sandbox runner (decision 0054).
//!
//! A dsh seat is confined by dsh's OWN sandbox
//! (`@deepseek-ai/dsh-sandbox-local`): a read-only host root plus a
//! writable session workspace, and nothing else. That boundary cannot
//! express an extra writable root — `SandboxExecutionPolicy` is
//! `{mode, workspaceRoot}` and `bwrapProfileArgs` binds only
//! `workspaceRoot` — so a seat in a linked `git worktree` cannot write
//! the metadata git keeps outside the worktree, and `git add` /
//! `git commit` fails to create
//! `$GIT_COMMON_DIR/worktrees/<name>/index.lock`.
//!
//! The provider's supported extension point is its `runnerCommand`: a
//! bwrap-compatible runner that receives the provider's profile
//! arguments and may refine them before executing bubblewrap. `brokkr
//! driver dsh` points that runner at [`RUNNER_VERB`] for a
//! `workspace-write` seat whose git metadata lies outside the workspace.
//!
//! **The shared repository is never writable.** What the seat gets is
//! its own per-worktree directory — `index`, `HEAD`, its reflog — and a
//! PRIVATE common directory the driver staged: a copy of the shared
//! refs, an `objects` directory whose `info/alternates` names the real
//! object store read-only, and empty reflogs. The box's git follows it
//! because the worktree's `commondir` file, the one file git reads to
//! find the shared directory, is masked with a read-only bind naming
//! that private store. So a commit in the seat writes its objects and
//! moves its branch inside the private store, and the shared
//! `objects`, `refs`, `packed-refs`, `logs` and every sibling worktree
//! stay outside the write set entirely — read-only under the profile's
//! own `--ro-bind / /`.
//!
//! Afterwards the trusted driver PROMOTES exactly one ref: the branch
//! the worktree had checked out when the seat started. Git refuses to
//! check one branch out in two worktrees, so that ref is this seat's
//! and no sibling's; the promotion fetches the seat's objects into the
//! shared store through git's own local transport and moves the branch
//! with a compare-and-swap against the value the host still holds. A
//! seat that moved a sibling's branch, a tag, a remote-tracking ref or
//! anything else moved it only inside a private store that is then
//! discarded.
//!
//! Only ONE layout is served: a genuine linked worktree, whose
//! administrative directory Git itself would have placed at
//! `<common>/worktrees/<name>` and whose `gitdir` back-pointer names a
//! `.git` in this seat's own workspace directory — the directory, not
//! the `.git` path, because that path is one the seat can replace with a
//! symlink to somebody else's. Everything else refuses before the seat starts
//! (decision 0054's threat model). Git resolves `--git-dir` and
//! `--git-common-dir` by FOLLOWING a `.git` file, and a `.git` file
//! inside the workspace is a file the model can write: without the
//! topology and back-pointer checks a seat could hand the driver a
//! `commondir` naming an unrelated repository and have the runner bind
//! that repository's per-worktree metadata.
//!
//! The trusted paths travel in the runner's own argv, resolved by the
//! trusted driver BEFORE the seat starts, never re-resolved from a
//! workspace file the model can edit: the session workspace, the two git
//! directories, the bubblewrap the driver probed, the private store, and
//! the staged directory holding the three files the runner mounts
//! read-only. The runner executes that absolute bubblewrap itself; it
//! never searches `PATH` for one, so the boundary is the binary the
//! driver measured. It reads the profile it was handed to decide whether
//! those staged files are reachable, and refuses every bubblewrap option
//! it does not know rather than guessing how many arguments follow it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The stderr prefix dsh classifies as a runner failure, so a refusal
/// here reads "the command never ran", never "the command was denied".
pub const RUNNER_FAILURE_SIGNATURE: &str = "brokkr-dsh-sandbox-runner: ";

/// The bwrap-compatible verb the adapter names as dsh's runner.
pub const RUNNER_VERB: &str = "dsh-sandbox-runner";

/// The empty file that stands in for the per-worktree `config` and
/// `config.worktree`.
const MASK_FILE: &str = "config-mask";
/// The `commondir` the box's git reads: the private store's path, so
/// every shared write the seat makes lands there.
const COMMONDIR_FILE: &str = "commondir";
/// The private store's `objects/info/alternates`: the real object store,
/// read-only, so the seat reads the whole repository and writes none of it.
const ALTERNATES_FILE: &str = "alternates";

/// Where the driver parks a seat's commits while it moves the branch.
/// Not a branch, so `git fetch` has no checked-out ref to refuse.
const PROMOTION_REF: &str = "refs/brokkr/dsh-promotion";

/// The reflog message the promoted branch carries, so the host's
/// `git reflog` says who moved it.
const PROMOTION_MESSAGE: &str = "brokkr: the dsh seat's commits, promoted by the driver";

/// The three git paths the driver resolved through Git before the seat
/// started, carried in the runner's own argv beside the bubblewrap the
/// driver probed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitScope {
    /// The dsh session workspace — the seat's writable root.
    pub workspace: PathBuf,
    /// The per-worktree git directory (`git rev-parse --git-dir`).
    pub git_dir: PathBuf,
    /// The shared git directory (`git rev-parse --git-common-dir`).
    pub common_dir: PathBuf,
}

/// Build the bubblewrap argv from the runner's argv: the runner's own
/// scope flags, then the profile the dsh provider composed, then the
/// command. Only the profile prefix is read; the command is opaque. The
/// program is the absolute bubblewrap named by `--bwrap`, never a `PATH`
/// lookup.
pub fn runner_argv(args: &[String]) -> Result<Vec<String>, String> {
    let (scope, paths, rest) = parse_scope(args)?;
    let (profile, command) = split_profile(rest)?;
    if profile.len() < 3 || profile[0] != "--ro-bind" || profile[1] != "/" || profile[2] != "/" {
        return Err(
            "the dsh sandbox profile does not begin with `--ro-bind / /`; refusing to guess \
             the boundary it stands for"
                .to_string(),
        );
    }
    let binds = writable_binds(profile)?;
    let mut argv = vec![paths.bwrap.to_string_lossy().into_owned()];
    argv.extend(profile.iter().cloned());
    if workspace_writable(&binds, &scope.workspace) {
        argv.extend(scoped_git_binds(&scope, &paths, &binds)?);
    }
    argv.push("--".to_string());
    argv.extend(command.iter().cloned());
    Ok(argv)
}

/// The host paths the runner needs beside the git scope: the bubblewrap
/// the driver probed, the private common directory the seat writes, and
/// the staged directory holding the files the runner mounts read-only.
struct RunnerPaths {
    bwrap: PathBuf,
    store: PathBuf,
    trusted: PathBuf,
}

impl RunnerPaths {
    fn mask(&self) -> PathBuf {
        self.trusted.join(MASK_FILE)
    }

    fn commondir(&self) -> PathBuf {
        self.trusted.join(COMMONDIR_FILE)
    }

    fn alternates(&self) -> PathBuf {
        self.trusted.join(ALTERNATES_FILE)
    }
}

/// The runner's own flags, and the profile that follows them. A flag
/// given twice, or without its path, is a malformed runner invocation,
/// not something to interpret generously. `--bwrap`, `--store` and
/// `--trusted` must be absolute: the runner executes the binary and
/// mounts the directories the driver staged, and a relative spelling
/// would be resolved by a working directory the seat can move.
fn parse_scope(args: &[String]) -> Result<(GitScope, RunnerPaths, &[String]), String> {
    let mut workspace: Option<PathBuf> = None;
    let mut git_dir: Option<PathBuf> = None;
    let mut common_dir: Option<PathBuf> = None;
    let mut bwrap: Option<PathBuf> = None;
    let mut store: Option<PathBuf> = None;
    let mut trusted: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let slot = match args[index].as_str() {
            "--workspace" => &mut workspace,
            "--git-dir" => &mut git_dir,
            "--common-dir" => &mut common_dir,
            "--bwrap" => &mut bwrap,
            "--store" => &mut store,
            "--trusted" => &mut trusted,
            _ => break,
        };
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("dsh sandbox runner: {} needs a path after it", args[index]))?;
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err(format!("dsh sandbox runner: {} given twice", args[index]));
        }
        index += 2;
    }
    let required = |value: Option<PathBuf>, flag: &str| {
        value.ok_or_else(|| format!("dsh sandbox runner: {flag} is required"))
    };
    let workspace = required(workspace, "--workspace")?;
    let git_dir = required(git_dir, "--git-dir")?;
    let common_dir = required(common_dir, "--common-dir")?;
    let bwrap = required(bwrap, "--bwrap")?;
    let store = required(store, "--store")?;
    let trusted = required(trusted, "--trusted")?;
    for (flag, path) in [
        ("--bwrap", &bwrap),
        ("--store", &store),
        ("--trusted", &trusted),
    ] {
        if !path.is_absolute() {
            return Err(format!(
                "dsh sandbox runner: {flag} {} is not an absolute path; the runner uses the \
                 host paths the driver staged and never a `PATH` or working-directory lookup",
                path.display()
            ));
        }
    }
    Ok((
        GitScope {
            workspace,
            git_dir,
            common_dir,
        },
        RunnerPaths {
            bwrap,
            store,
            trusted,
        },
        &args[index..],
    ))
}

/// The profile before the bare `--`, and the command after it.
fn split_profile(rest: &[String]) -> Result<(&[String], &[String]), String> {
    let split = rest.iter().position(|token| token == "--").ok_or_else(|| {
        "dsh sandbox runner: the profile has no `--` before the command".to_string()
    })?;
    Ok((&rest[..split], &rest[split + 1..]))
}

/// One bubblewrap option: how many arguments follow it, and — when the
/// option grants the box WRITE access to a host path — which of those
/// arguments is the host source and which is the destination.
struct ProfileFlag {
    arity: usize,
    write: Option<(usize, usize)>,
}

/// Bubblewrap 0.11's options, by name. The table is exhaustive on
/// purpose: an option the runner does not know is refused rather than
/// stepped over, because stepping over it means guessing how many
/// arguments follow — and a wrong guess reads an ARGUMENT as a flag, or
/// a flag as an argument, and the answer to "can the box write the files
/// the runner mounts read-only?" is then measured against the wrong
/// tokens. Only `--bind`, `--bind-try`, `--dev-bind`, `--dev-bind-try`
/// and `--overlay` name a host path the box can write; `--tmpfs`,
/// `--dir`, `--dev`, `--proc`, `--mqueue`, `--tmp-overlay` and
/// `--ro-overlay` make writable mount points with no host source behind
/// them, and every `--ro-` form is read-only by name.
fn profile_flag(flag: &str) -> Option<ProfileFlag> {
    let plain = |arity| Some(ProfileFlag { arity, write: None });
    match flag {
        "--help"
        | "--version"
        | "--level-prefix"
        | "--unshare-all"
        | "--share-net"
        | "--unshare-user"
        | "--unshare-user-try"
        | "--unshare-ipc"
        | "--unshare-pid"
        | "--unshare-net"
        | "--unshare-uts"
        | "--unshare-cgroup"
        | "--unshare-cgroup-try"
        | "--clearenv"
        | "--new-session"
        | "--die-with-parent"
        | "--as-pid-1"
        | "--disable-userns"
        | "--assert-userns-disabled" => plain(0),
        "--args" | "--argv0" | "--userns" | "--userns2" | "--pidns" | "--uid" | "--gid"
        | "--hostname" | "--chdir" | "--unsetenv" | "--lock-file" | "--sync-fd"
        | "--remount-ro" | "--exec-label" | "--file-label" | "--proc" | "--dev" | "--tmpfs"
        | "--mqueue" | "--dir" | "--seccomp" | "--add-seccomp-fd" | "--block-fd"
        | "--userns-block-fd" | "--info-fd" | "--json-status-fd" | "--cap-add" | "--cap-drop"
        | "--perms" | "--size" | "--overlay-src" | "--tmp-overlay" | "--ro-overlay" => plain(1),
        "--setenv" | "--ro-bind" | "--ro-bind-try" | "--bind-fd" | "--ro-bind-fd" | "--file"
        | "--bind-data" | "--ro-bind-data" | "--symlink" | "--chmod" => plain(2),
        "--bind" | "--bind-try" | "--dev-bind" | "--dev-bind-try" => Some(ProfileFlag {
            arity: 2,
            write: Some((0, 1)),
        }),
        "--overlay" => Some(ProfileFlag {
            arity: 3,
            write: Some((0, 2)),
        }),
        _ => None,
    }
}

/// Every read-write bind in the profile the runner was handed, as
/// (source, destination). Two questions are answered from this one
/// reading of the argv it was actually given, rather than from what a
/// dsh version's `bwrapProfileArgs` is expected to emit: which grant
/// makes this a `workspace-write` seat, and which host paths the box can
/// WRITE — every source here, because a read-write bind lets the box
/// write the host file under it through the destination it is mounted
/// at.
fn writable_binds(profile: &[String]) -> Result<Vec<(&Path, &Path)>, String> {
    let mut binds = Vec::new();
    let mut index = 0;
    while index < profile.len() {
        let flag = &profile[index];
        let Some(shape) = profile_flag(flag) else {
            return Err(format!(
                "the dsh sandbox profile carries {flag}, which this runner does not know; it \
                 refuses rather than guess how many arguments follow it and whether they name \
                 a host path the box could write"
            ));
        };
        if index + shape.arity >= profile.len() {
            return Err(format!(
                "the dsh sandbox profile ends inside {flag}'s arguments"
            ));
        }
        if let Some((source, destination)) = shape.write {
            binds.push((
                Path::new(&profile[index + 1 + source]),
                Path::new(&profile[index + 1 + destination]),
            ));
        }
        index += 1 + shape.arity;
    }
    Ok(binds)
}

/// True when the profile binds the session workspace read-write at its
/// own path, which is what `workspace-write` means to the provider. A
/// `read-only` profile carries no such bind, and the runner then adds
/// nothing.
fn workspace_writable(binds: &[(&Path, &Path)], workspace: &Path) -> bool {
    binds
        .iter()
        .any(|(source, destination)| source == destination && same_path(source, workspace))
}

/// Path identity for the bind check: resolved where both resolve, so a
/// symlinked spelling agrees with the provider's canonical workspace.
fn same_path(left: &Path, right: &Path) -> bool {
    resolve(left) == resolve(right)
}

/// A path with every symlink and `..` component resolved where the host
/// can resolve it, so a symlinked alias of a repository cannot pass a
/// comparison the real path would fail. A path that does not exist keeps
/// its literal spelling and therefore never equals a real one.
fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// True when `inner` lies at or under `outer`, both resolved.
fn within(inner: &Path, outer: &Path) -> bool {
    resolve(inner).starts_with(resolve(outer))
}

/// Why this scope cannot be served under the declared boundary, or
/// `None`. The driver calls it before the seat starts so an unsupported
/// layout refuses early, naming the remedy; the runner calls it again on
/// every command, because its argv is the last gate before bubblewrap.
///
/// A scope whose common directory already sits inside the workspace
/// needs nothing and is never refused: the provider's own bind covers
/// it. Every other scope must be a genuine linked worktree, proved
/// against the host and not against anything the model could have
/// written, because Git resolves both directories by FOLLOWING the
/// workspace's own `.git` file:
///
/// 1. **The git directory is not the shared repository.** A commit needs
///    `index.lock` and `HEAD.lock` inside it, so serving `git_dir ==
///    common_dir` means mounting the whole shared `.git` writable — every
///    sibling worktree and every submodule config and hook with it. A
///    primary checkout reached through a subdirectory, a
///    `--separate-git-dir` checkout and a worktree `.git` file redirected
///    at the parent all resolve here.
/// 2. **The git directory sits where Git itself would have put it**, at
///    `<common_dir>/worktrees/<name>`. Without this, a seat that writes
///    `<workspace>/.git` -> `gitdir: <workspace>/fake` and fills
///    `<workspace>/fake/commondir` with an unrelated repository's path
///    makes Git report that repository as the common directory, and the
///    runner would then bind a directory under it read-write.
/// 3. **The `gitdir` back-pointer exists and names a `.git` INSIDE this
///    seat's workspace directory.** The topology check alone is not
///    enough: a workspace `.git` file may name a REAL administrative
///    directory belonging to another worktree of another repository. The
///    back-pointer lives inside `git_dir`, outside the workspace and
///    therefore outside anything a seat can write, so it is the
///    trustworthy end of the pair. A directory with no back-pointer is
///    refused, not excused.
///
///    What is compared is the back-pointer's PARENT against the
///    workspace, never the back-pointer against `<workspace>/.git`.
///    `<workspace>/.git` is a path the seat OWNS, and the comparison
///    resolves symlinks: a seat that replaces its own `.git` with a
///    symlink to another worktree's `.git` file makes both sides resolve
///    to the victim's path, and the two ends of the pair then agree
///    about a repository this seat does not own. The parent is the one
///    end no workspace write can move — the victim's worktree root is
///    not this seat's workspace however the seat spells it. A symlink at
///    `<workspace>/.git` is refused outright as well: `git worktree`
///    never writes one, so its presence is evidence rather than a
///    spelling.
pub fn scope_refusal(scope: &GitScope) -> Option<String> {
    if scope.common_dir.starts_with(&scope.workspace) {
        return None;
    }
    if same_path(&scope.git_dir, &scope.common_dir) {
        return Some(format!(
            "the seat's git directory {} is the shared repository's own git directory, so a \
             commit would need the whole shared .git writable; the scoped runner refuses that \
             (a primary checkout reached through a subdirectory, a `--separate-git-dir` \
             checkout, or a `.git` file redirected at the parent). Run the seat from the \
             repository root, or from a linked `git worktree`",
            scope.git_dir.display()
        ));
    }
    let administrative = scope.common_dir.join("worktrees");
    if !scope
        .git_dir
        .parent()
        .is_some_and(|parent| same_path(parent, &administrative))
    {
        return Some(format!(
            "the seat's git directory {} is not one of the linked-worktree administrative \
             directories {} that its shared repository owns, so Git reached it through a \
             `.git` file rather than through `git worktree`; the scoped runner refuses to \
             bind a repository a workspace file pointed it at. Run the seat from the \
             repository root, or from a linked `git worktree`",
            scope.git_dir.display(),
            administrative.join("<name>").display()
        ));
    }
    // A seat can write its own `.git`, and `git worktree` writes a plain
    // file there. A SYMLINK is a second name for a `.git` this seat does
    // not own, and it exists only to make a resolving comparison agree
    // with the wrong repository, so it is refused before anything is
    // compared.
    let workspace_git = scope.workspace.join(".git");
    if std::fs::symlink_metadata(&workspace_git).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Some(format!(
            "this seat's workspace has a symbolic link at {}, and `git worktree` writes a \
             plain file there; the scoped runner refuses a `.git` that is a second name for \
             another worktree's metadata. Run the seat from a linked `git worktree` git \
             itself created",
            workspace_git.display()
        ));
    }
    let Some(recorded) = recorded_worktree(scope) else {
        return Some(format!(
            "the per-worktree git directory {} carries no `gitdir` back-pointer, so nothing \
             outside this seat's workspace ties it to {}; the scoped runner refuses to write \
             git metadata it cannot tie to the seat. Run the seat from a linked `git \
             worktree` git itself created",
            scope.git_dir.display(),
            scope.workspace.display()
        ));
    };
    // The DIRECTORY the back-pointer names is what must be this seat's
    // workspace. Comparing the pointer itself against
    // `<workspace>/.git` would compare against a path the seat owns, and
    // a symlink there would make both sides resolve to the victim's
    // `.git`; the worktree root the pointer names is the end no
    // workspace write can move.
    let recorded_root = recorded.parent();
    if recorded_root.is_some_and(|root| same_path(root, &scope.workspace)) {
        return None;
    }
    // The back-pointer names a real worktree that CONTAINS this seat's
    // workspace: the seat was started in a subdirectory of its worktree.
    // Nothing is pointing at another worktree here, and saying so would
    // misname the cause; the remedy is the worktree's own root.
    if let Some(root) = recorded_root {
        if within(&scope.workspace, root) {
            return Some(format!(
                "this seat's workspace {} is a subdirectory of the linked worktree at {}, and \
                 the scoped runner serves a worktree at its own root: the writable root dsh \
                 gives the seat would not cover the rest of the worktree. Run the seat from {}",
                scope.workspace.display(),
                root.display(),
                root.display()
            ));
        }
    }
    Some(format!(
        "the per-worktree git directory {} records {} as its worktree, not this seat's \
         workspace {}; refusing to bind another worktree's metadata",
        scope.git_dir.display(),
        recorded.display(),
        scope.workspace.display()
    ))
}

/// The worktree `.git` file a linked worktree's `gitdir` pointer names.
/// `None` when the host has no readable, non-empty pointer — which for a
/// directory Git itself created never happens, and which the caller
/// therefore treats as a refusal rather than as an excuse.
fn recorded_worktree(scope: &GitScope) -> Option<PathBuf> {
    let recorded = std::fs::read_to_string(scope.git_dir.join("gitdir")).ok()?;
    let recorded = recorded.trim();
    if recorded.is_empty() {
        return None;
    }
    let recorded = Path::new(recorded);
    Some(if recorded.is_absolute() {
        recorded.to_path_buf()
    } else {
        scope.git_dir.join(recorded)
    })
}

/// Why the files the driver staged cannot be mounted read-only, or
/// `None`. All three stand for something git READS and the box must not
/// choose: the per-worktree config, the `commondir` that says which
/// common directory the seat's git writes, and the private store's
/// `alternates`. A bind is only as read-only as its source is
/// unreachable, so the directory holding them must lie outside every
/// path the box can write — the source of every read-write bind in the
/// profile the runner was handed, plus the read-write set this runner is
/// about to add itself. The seat's own workspace is one of those profile
/// sources, so it needs no separate arm.
fn trusted_refusal(
    paths: &RunnerPaths,
    scope: &GitScope,
    binds: &[(&Path, &Path)],
) -> Option<String> {
    if let Some(root) = binds
        .iter()
        .map(|(source, _)| *source)
        .chain([scope.git_dir.as_path(), paths.store.as_path()])
        .find(|root| within(&paths.trusted, root))
    {
        return Some(format!(
            "the files the dsh runner mounts read-only are staged in {}, which lies under {} — \
             a path this seat's box can write, so a boxed command could rewrite the very files \
             the mounts stand for",
            paths.trusted.display(),
            root.display()
        ));
    }
    [
        (paths.mask(), true),
        (paths.commondir(), false),
        (paths.alternates(), false),
    ]
    .iter()
    .find_map(|(path, empty)| staged_file_refusal(path, *empty))
}

/// Why one staged file is not what the runner mounts, or `None`. The
/// mask must be EMPTY, because it stands for a config nobody wrote; the
/// two pointers must not be, because git reads each as a path.
fn staged_file_refusal(path: &Path, empty: bool) -> Option<String> {
    let problem = match std::fs::symlink_metadata(path) {
        Err(error) => format!("cannot be read ({error})"),
        Ok(meta) if !meta.file_type().is_file() => {
            // `/dev/null` is the obvious wrong answer for the mask:
            // bubblewrap mounts every bind source with `MS_NODEV`, so a
            // bound character device cannot be opened inside the box and
            // git calls an unreadable config file a fatal error.
            "is not a regular file; bubblewrap mounts a bind source with MS_NODEV, so a device \
             node masks the path with something the box cannot open and git calls an \
             unreadable config file fatal"
                .to_string()
        }
        Ok(meta) if empty && meta.len() != 0 => format!("is not empty ({} bytes)", meta.len()),
        Ok(meta) if !empty && meta.len() == 0 => {
            "is empty, and git reads it as a path rather than as an absence".to_string()
        }
        Ok(_) => return None,
    };
    Some(format!("the staged file {} {problem}", path.display()))
}

/// The scoped write set, in mount order: later binds sit over earlier
/// ones, so a mask always follows the bind it hides. Refuses a scope
/// that [`scope_refusal`] rejects rather than mounting it wider.
///
/// Nothing under the SHARED git directory is here except this worktree's
/// own administrative directory. The shared `objects`, `refs`,
/// `packed-refs` and `logs` are reached through the private store the
/// driver staged, and reached for READING through its `alternates`; the
/// seat's writes land in the store and the driver promotes one ref out of
/// it afterwards.
fn scoped_git_binds(
    scope: &GitScope,
    paths: &RunnerPaths,
    binds: &[(&Path, &Path)],
) -> Result<Vec<String>, String> {
    // A primary checkout keeps its whole git directory under the
    // workspace the provider already made writable: nothing to add.
    if scope.common_dir.starts_with(&scope.workspace) {
        return Ok(Vec::new());
    }
    if let Some(problem) = scope_refusal(scope) {
        return Err(problem);
    }
    if let Some(problem) = trusted_refusal(paths, scope, binds) {
        return Err(problem);
    }
    let mut argv = Vec::new();
    let git = &scope.git_dir;
    let common = &scope.common_dir;
    let store = &paths.store;
    // The per-worktree directory holds index, HEAD and its reflog. Every
    // served scope keeps it outside the workspace (`scope_refusal` proves
    // it is `<common>/worktrees/<name>`), so it always needs the bind.
    bind("--bind", &mut argv, git);
    // Config, wherever git reads it, is masked rather than bound
    // read-only-try: `--ro-bind-try` no-ops when the host file is absent —
    // the normal state of a linked worktree's per-worktree directory —
    // and the writable directory around it would then let a boxed command
    // CREATE one. A repository that has run `git sparse-checkout` carries
    // `extensions.worktreeConfig`, so the host would honour a
    // `core.hooksPath` written into `config.worktree`. Bubblewrap creates
    // the mount point for a missing destination, so the host gains an
    // empty file and the box gains no way to fill it.
    over(&mut argv, &paths.mask(), &git.join("config"));
    over(&mut argv, &paths.mask(), &git.join("config.worktree"));
    // `commondir` is the file git reads to find the shared directory, and
    // this is where the whole boundary turns: the box's git is handed the
    // PRIVATE store instead. Read-only and hard, so a write is `EROFS`
    // and unlink or rename over the mount point is `EBUSY`; the host's own
    // file is untouched, so a host `git` in the same worktree still reads
    // the real shared directory.
    over(&mut argv, &paths.commondir(), &git.join("commondir"));
    // The worktree's pointer back to its own `.git` file stays what the
    // host wrote: left writable, a boxed command could point the next
    // seat's resolution somewhere else.
    bind("--ro-bind", &mut argv, &git.join("gitdir"));
    // So does the worktree's `HEAD`, and for the same reason one level
    // up: `HEAD` is what says which branch this worktree OWNS, and it is
    // the ref the driver promotes. Left writable, a seat could point it
    // at a sibling's branch and have the NEXT seat's honest commits
    // promoted onto it. A commit does not need to write `HEAD` — it moves
    // the branch `HEAD` names — so the work path is untouched; switching
    // branches or rebasing inside a seat is not served, and says so with
    // a lock failure rather than by quietly widening the write set.
    bind("--ro-bind", &mut argv, &git.join("HEAD"));
    tmpfs(&mut argv, &git.join("hooks"));
    // The private common directory: the seat's objects, refs and reflogs.
    bind("--bind", &mut argv, store);
    // Read-only inside the store, because each is something git reads and
    // the seat must not choose: the repository's config (so `git config
    // --local` cannot plant a `core.hooksPath`), the shared HEAD, and the
    // alternates line that makes the real object store readable.
    over(&mut argv, &common.join("config"), &store.join("config"));
    over(&mut argv, &common.join("HEAD"), &store.join("HEAD"));
    over(
        &mut argv,
        &paths.alternates(),
        &store.join("objects/info/alternates"),
    );
    // The hooks the seat's git would run are an empty tmpfs, per-worktree
    // and shared: the host's hooks are never on the seat's hook path, and
    // anything the seat writes there dies with the box.
    tmpfs(&mut argv, &store.join("hooks"));
    Ok(argv)
}

fn bind(flag: &str, argv: &mut Vec<String>, path: &Path) {
    let path = path.to_string_lossy().into_owned();
    argv.extend([flag.to_string(), path.clone(), path]);
}

/// Mount one staged host file read-only over a path in the box, whether
/// or not the host file exists. Bubblewrap creates the mount point for a
/// missing destination under a writable parent, so the host gains an
/// empty file rather than a writable path.
///
/// The source is always a REGULAR file, never `/dev/null`: bubblewrap
/// mounts every bind source with `MS_NODEV`, so a bound character device
/// cannot be opened inside the box, and git answers an unreadable
/// configuration file with `fatal: unknown error occurred while reading
/// the configuration files` — which would break every git command in the
/// seat, in exactly the repositories (`extensions.worktreeConfig`) the
/// mask exists for. Measured on Linux 6.17 with bubblewrap 0.11.
fn over(argv: &mut Vec<String>, source: &Path, path: &Path) {
    argv.extend([
        "--ro-bind".to_string(),
        source.to_string_lossy().into_owned(),
        path.to_string_lossy().into_owned(),
    ]);
}

fn tmpfs(argv: &mut Vec<String>, path: &Path) {
    argv.extend(["--tmpfs".to_string(), path.to_string_lossy().into_owned()]);
}

/// Everything the driver stages on the host for one seat's life: the
/// private common directory the seat's git writes, the directory of
/// read-only files the runner mounts, and the ONE ref this worktree
/// owns. The driver holds it while `dsh` runs and hands it to
/// [`promote_seat_commits`] afterwards; dropping it before then unlinks
/// the store every command in the seat was writing into.
#[derive(Debug)]
pub struct SeatGitStore {
    store: tempfile::TempDir,
    trusted: tempfile::TempDir,
    reference: String,
}

impl SeatGitStore {
    /// The private common directory, bound read-write for the seat.
    pub fn store_path(&self) -> &Path {
        self.store.path()
    }

    /// The staged read-only files, named to the runner and reachable
    /// from no path inside the box.
    pub fn trusted_path(&self) -> &Path {
        self.trusted.path()
    }

    /// The branch this worktree had checked out when the seat started —
    /// the only ref a promotion may move.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Keep the private store rather than deleting it, and answer where
    /// it landed. A promotion that failed has the seat's commits in
    /// there and nowhere else, so the operator is told the path instead
    /// of losing the work.
    fn kept(self) -> PathBuf {
        self.store.keep()
    }
}

/// What a promotion moved, for the line the driver leaves in the seat's
/// stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promotion {
    /// The branch the worktree owns.
    pub reference: String,
    /// Where the host's branch stood before, or `None` for a branch with
    /// no commit yet.
    pub from: Option<String>,
    /// The commit the seat left.
    pub to: String,
}

impl Promotion {
    /// The one line the driver appends to a seat's stderr, so the
    /// journal's tail says which ref moved and between which commits.
    pub fn summary(&self) -> String {
        format!(
            "brokkr dsh: promoted the seat's commits to {} ({} -> {})",
            self.reference,
            self.from.as_deref().unwrap_or("no commit"),
            self.to
        )
    }
}

/// Stage the private common directory and the read-only files one seat
/// needs, or say why this worktree cannot be served.
pub fn stage_seat_store(scope: &GitScope) -> Result<SeatGitStore, String> {
    stage_seat_store_in(scope, || {
        tempfile::Builder::new().prefix("brokkr-dsh-git-").tempdir()
    })
}

/// The staging over an injected directory maker, so the ways it can fail
/// on a full or read-only disk are reachable from a test.
fn stage_seat_store_in(
    scope: &GitScope,
    mut make: impl FnMut() -> std::io::Result<tempfile::TempDir>,
) -> Result<SeatGitStore, String> {
    let reference = checked_out_branch(scope)?;
    let store = make().map_err(staging_failed)?;
    let trusted = make().map_err(staging_failed)?;
    stage_files(scope, store.path(), trusted.path()).map_err(staging_failed)?;
    Ok(SeatGitStore {
        store,
        trusted,
        reference,
    })
}

fn staging_failed(error: std::io::Error) -> String {
    format!("dsh driver: could not stage the seat's private git store: {error}")
}

/// The branch this worktree has checked out, read from the host's
/// `<git_dir>/HEAD` before the seat starts. It is the ONE ref the seat
/// owns, and the runner mounts that `HEAD` read-only so it stays a value
/// only the host ever wrote — otherwise a seat could point it at a
/// sibling's branch and have the NEXT seat's commits promoted there.
/// A worktree with a detached HEAD owns no ref, and is refused before an
/// implementation is spent rather than after it.
fn checked_out_branch(scope: &GitScope) -> Result<String, String> {
    let Some(reference) = head_branch(&scope.git_dir) else {
        return Err(format!(
            "the worktree at {} has no branch checked out, so there is no ref the seat owns \
             and nothing a commit could be promoted to; the scoped runner promotes exactly \
             the branch a worktree has checked out, because git refuses to check one branch \
             out in two worktrees. Run `git switch -c <branch>` in the worktree before the \
             seat starts",
            scope.workspace.display()
        ));
    };
    // That git refuses one branch in two worktrees is the whole reason
    // this ref can be called this seat's, so it is VERIFIED rather than
    // assumed. The main checkout's `HEAD` and every sibling's lie outside
    // the seat's write set, so what they say is the host's answer.
    if let Some(other) = other_checkout_on(scope, &reference) {
        return Err(format!(
            "the worktree at {} has {reference} checked out, and so does {}; the scoped \
             runner promotes the branch a worktree OWNS, and a branch two checkouts claim is \
             owned by neither. Run `git worktree repair`, or give this seat a branch of its own",
            scope.workspace.display(),
            other.display()
        ));
    }
    Ok(reference)
}

/// The branch one git directory's `HEAD` names. `None` for a detached
/// HEAD, an unreadable file, or a symbolic ref outside `refs/heads/`.
fn head_branch(git_dir: &Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let reference = head.trim().strip_prefix("ref: ")?;
    reference
        .starts_with("refs/heads/")
        .then(|| reference.to_string())
}

/// The main checkout or the sibling worktree that also has this branch
/// checked out, if any. Both are read from the shared directory, which no
/// seat can write.
fn other_checkout_on(scope: &GitScope, reference: &str) -> Option<PathBuf> {
    let mut others = vec![scope.common_dir.clone()];
    if let Ok(entries) = std::fs::read_dir(scope.common_dir.join("worktrees")) {
        others.extend(entries.flatten().map(|entry| entry.path()));
    }
    others.into_iter().find(|other| {
        !same_path(other, &scope.git_dir) && head_branch(other).as_deref() == Some(reference)
    })
}

/// Write the private common directory and the three read-only files.
///
/// The private directory is a real git common directory: the shared
/// refs copied so every branch, tag and remote-tracking ref reads back
/// exactly as the host has it, an empty `logs` for the seat's own
/// reflogs, and an `objects` whose `info/alternates` names the host's
/// object store — so the seat READS every object the repository has and
/// WRITES none of them. `HEAD`, `config`, `info` and `shallow` are
/// copied because the host-side promotion reads this directory as a
/// repository; the runner then mounts the real `config` and `HEAD` over
/// their copies, so the box cannot choose either.
fn stage_files(scope: &GitScope, store: &Path, trusted: &Path) -> std::io::Result<()> {
    let commondir = format!("{}\n", store.display());
    let alternates = format!("{}\n", scope.common_dir.join("objects").display());
    std::fs::write(trusted.join(MASK_FILE), "")?;
    std::fs::write(trusted.join(COMMONDIR_FILE), commondir)?;
    std::fs::write(trusted.join(ALTERNATES_FILE), &alternates)?;
    std::fs::create_dir_all(store.join("objects/info"))?;
    std::fs::create_dir_all(store.join("logs"))?;
    std::fs::create_dir_all(store.join("refs"))?;
    std::fs::write(store.join("objects/info/alternates"), &alternates)?;
    for name in ["refs", "packed-refs", "HEAD", "config", "info", "shallow"] {
        copy_tree(&scope.common_dir.join(name), &store.join(name))?;
    }
    Ok(())
}

/// Copy one name out of the shared git directory into the private one,
/// file or directory tree. A name the shared directory does not carry is
/// not a failure: a repository with every ref packed has no loose
/// `refs/heads`, and one that is not shallow has no `shallow`.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    match std::fs::read_dir(from) {
        Ok(entries) => {
            std::fs::create_dir_all(to)?;
            for entry in entries {
                let entry = entry?;
                copy_tree(&entry.path(), &to.join(entry.file_name()))?;
            }
            Ok(())
        }
        Err(_) if !from.exists() => Ok(()),
        // Not a directory, or a directory the driver may not read; `copy`
        // is the one that says which, and its error is the one that
        // travels.
        Err(_) => std::fs::copy(from, to).map(drop),
    }
}

/// Move the seat's work out of the private store and into the shared
/// repository: the ONE branch the worktree owns, and only it.
///
/// The objects travel through git's own local transport into a ref
/// namespace no worktree can have checked out, and the branch then moves
/// with a compare-and-swap against the value the host still holds, so a
/// branch something else moved while the seat ran refuses rather than
/// being overwritten. `None` means there was nothing to promote — the
/// seat committed nothing, or left its branch where the host had it.
/// Everything else the seat wrote — a sibling's branch, a tag, a
/// remote-tracking ref, an object nothing reaches — stays in the private
/// store and is discarded with it.
pub fn promote_seat_commits(
    store: SeatGitStore,
    scope: &GitScope,
) -> Result<Option<Promotion>, String> {
    promote_seat_commits_with("git", store, scope)
}

/// The promotion over an injected `git`, so each way it can fail is
/// reachable from a test without breaking a real repository.
fn promote_seat_commits_with(
    program: &str,
    store: SeatGitStore,
    scope: &GitScope,
) -> Result<Option<Promotion>, String> {
    match promote(program, &store, scope) {
        Ok(promotion) => Ok(promotion),
        Err(problem) => Err(format!(
            "dsh driver: {problem}. The seat's commits are not lost: its private git store is \
             kept at {}, and `git --git-dir=<store> log {}` still reads them",
            store.kept().display(),
            PROMOTION_REF
        )),
    }
}

fn promote(
    program: &str,
    store: &SeatGitStore,
    scope: &GitScope,
) -> Result<Option<Promotion>, String> {
    let private = store.store.path().to_string_lossy().into_owned();
    let common = scope.common_dir.to_string_lossy().into_owned();
    let reference = store.reference.clone();
    let Some(to) = ref_value(program, &private, &reference)? else {
        return Ok(None);
    };
    let from = ref_value(program, &common, &reference)?;
    if from.as_deref() == Some(to.as_str()) {
        return Ok(None);
    }
    let refspec = format!("+{reference}:{PROMOTION_REF}");
    git(
        program,
        &[
            "--git-dir",
            &common,
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "--quiet",
            &private,
            &refspec,
        ],
    )?;
    git(
        program,
        &[
            "--git-dir",
            &common,
            "update-ref",
            "-m",
            PROMOTION_MESSAGE,
            &reference,
            &to,
            from.as_deref().unwrap_or(""),
        ],
    )?;
    git(
        program,
        &["--git-dir", &common, "update-ref", "-d", PROMOTION_REF],
    )?;
    Ok(Some(Promotion {
        reference,
        from,
        to,
    }))
}

/// The commit one ref names in one git directory, or `None` when that
/// directory has no such ref — an unborn branch on the host, or a branch
/// the seat deleted in its private store. `rev-parse --verify --quiet`
/// answers a missing ref with a non-zero exit and no output, which is an
/// ANSWER; a git that could not be run at all is a failure, and travels
/// as one rather than being read as "no such ref".
fn ref_value(program: &str, git_dir: &str, reference: &str) -> Result<Option<String>, String> {
    let peeled = format!("{reference}^{{commit}}");
    let run = run_git(
        program,
        &[
            "--git-dir",
            git_dir,
            "rev-parse",
            "--verify",
            "--quiet",
            &peeled,
        ],
    )?;
    Ok(run.ok.then_some(run.stdout))
}

/// What one git run said.
struct GitRun {
    ok: bool,
    stdout: String,
    stderr: String,
}

/// Run one git command outside every box.
fn run_git(program: &str, args: &[&str]) -> Result<GitRun, String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("`{program} {}` could not run: {error}", args.join(" ")))?;
    Ok(GitRun {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
    })
}

/// One git command that must succeed: its trimmed stdout, or the refusal
/// its own stderr names.
fn git(program: &str, args: &[&str]) -> Result<String, String> {
    let run = run_git(program, args)?;
    if !run.ok {
        return Err(format!("`git {}` failed: {}", args.join(" "), run.stderr));
    }
    Ok(run.stdout)
}

/// True when this bubblewrap can build the empty-root profile here. The
/// probe is the provider's own: a read-only root around `true`.
pub fn bwrap_usable(bwrap: &Path) -> bool {
    Command::new(bwrap)
        .args(["--ro-bind", "/", "/", "--", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Refuse a bubblewrap that is present but cannot build the namespace.
pub fn require_usable_bwrap(bwrap: &Path) -> Result<(), String> {
    if bwrap_usable(bwrap) {
        return Ok(());
    }
    Err(format!(
        "bubblewrap at {} cannot build the empty-root namespace on this host; the dsh \
         git-metadata runner cannot stand in for it",
        bwrap.display()
    ))
}

/// The `--patch` row that points dsh's sandbox provider at the runner.
/// One patch file is the launcher's only override channel, so this row
/// travels beside the seat's transcript and model rows. The resolved
/// bubblewrap and the two staged directories are trusted paths too: the
/// runner executes and mounts exactly what the driver staged, so the
/// boundary is what the driver measured rather than whatever the seat's
/// environment holds when a command runs.
pub fn sandbox_row(
    program: &str,
    bwrap: &Path,
    staged: &SeatGitStore,
    scope: &GitScope,
) -> Result<String, String> {
    let mut row = format!(
        "# Written by `brokkr driver dsh` for one seat: the sandbox provider\n\
         # runs every command through this bwrap-compatible runner, which adds\n\
         # the scoped binds a linked worktree's commit needs and nothing else.\n\
         - id: sandbox\n\
         \x20 config:\n\
         \x20   runnerCommand:\n\
         \x20     - '{}'\n\
         \x20     - '{RUNNER_VERB}'\n",
        yaml_scalar(program)?
    );
    for (flag, path) in [
        ("--workspace", scope.workspace.as_path()),
        ("--git-dir", scope.git_dir.as_path()),
        ("--common-dir", scope.common_dir.as_path()),
        ("--bwrap", bwrap),
        ("--store", staged.store_path()),
        ("--trusted", staged.trusted_path()),
    ] {
        row.push_str(&format!(
            "\x20     - '{flag}'\n\x20     - '{}'\n",
            yaml_scalar(&path.to_string_lossy())?
        ));
    }
    row.push_str(&format!(
        "\x20   runnerFailureSignatures:\n\
         \x20     - '{RUNNER_FAILURE_SIGNATURE}'\n\
         \x20     - 'bwrap: '\n"
    ));
    Ok(row)
}

/// A single-quoted YAML scalar with its quotes doubled. A path that
/// spans a line is refused rather than written, the same discipline the
/// transcript and settings rows use.
fn yaml_scalar(value: &str) -> Result<String, String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!(
            "dsh sandbox runner: path {value:?} spans more than one line"
        ));
    }
    Ok(value.replace('\'', "''"))
}

#[cfg(test)]
mod tests;
