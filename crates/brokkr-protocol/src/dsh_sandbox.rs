//! The dsh harness sandbox runner (decision 0053).
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
//! `workspace-write` seat whose git metadata lies outside the
//! workspace, and the runner adds exactly the scoped binds a commit
//! needs. It never mounts the parent checkout or the whole shared
//! `.git`: the per-worktree directory — its `index`, `HEAD` and reflogs —
//! and the shared `objects`, `refs` and `logs` are writable; `hooks` is
//! an empty tmpfs; the per-worktree and shared `config`, the per-worktree
//! `config.worktree`, `commondir` and `gitdir` are masked or read-only,
//! so nothing a boxed command writes can become a program the host runs
//! on its next git invocation — nor can it point git at a `config` it
//! wrote in the workspace. Sibling worktrees, the parent checkout and
//! every credential path stay outside the write set.
//!
//! Two layouts the first slice bound are refused instead, because
//! serving them would widen the boundary past what a linked worktree
//! needs (decision 0053 addendum, 2026-09-09): a resolved git directory
//! that IS the shared repository (a primary checkout reached through a
//! subdirectory, a `--separate-git-dir` checkout, or a worktree `.git`
//! file redirected at the parent), which would need the whole shared
//! `.git` writable; and a per-worktree directory whose `gitdir` pointer
//! names a different worktree, which would let one seat write another's
//! metadata. Both refuse before the seat starts.
//!
//! The trusted paths travel in the runner's own argv, resolved by the
//! trusted driver BEFORE the seat starts, never re-resolved from a
//! workspace file the model can edit: the session workspace, the two
//! git directories, and the bubblewrap the driver probed. The runner
//! executes that absolute bubblewrap itself; it never searches `PATH`
//! for one, so the boundary is the binary the driver measured.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The stderr prefix dsh classifies as a runner failure, so a refusal
/// here reads "the command never ran", never "the command was denied".
pub const RUNNER_FAILURE_SIGNATURE: &str = "brokkr-dsh-sandbox-runner: ";

/// The bwrap-compatible verb the adapter names as dsh's runner.
pub const RUNNER_VERB: &str = "dsh-sandbox-runner";

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
    let (scope, bwrap, rest) = parse_scope(args)?;
    let (profile, command) = split_profile(rest)?;
    if profile.len() < 3 || profile[0] != "--ro-bind" || profile[1] != "/" || profile[2] != "/" {
        return Err(
            "the dsh sandbox profile does not begin with `--ro-bind / /`; refusing to guess \
             the boundary it stands for"
                .to_string(),
        );
    }
    let mut argv = vec![bwrap.to_string_lossy().into_owned()];
    argv.extend(profile.iter().cloned());
    if workspace_writable(profile, &scope.workspace) {
        argv.extend(scoped_git_binds(&scope)?);
    }
    argv.push("--".to_string());
    argv.extend(command.iter().cloned());
    Ok(argv)
}

/// The runner's own flags, and the profile that follows them. A flag
/// given twice, or without its path, is a malformed runner invocation,
/// not something to interpret generously. `--bwrap` must be absolute:
/// the runner executes the binary the driver probed, and a relative
/// spelling would be resolved by a working directory the seat can move.
fn parse_scope(args: &[String]) -> Result<(GitScope, PathBuf, &[String]), String> {
    let mut workspace: Option<PathBuf> = None;
    let mut git_dir: Option<PathBuf> = None;
    let mut common_dir: Option<PathBuf> = None;
    let mut bwrap: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let slot = match args[index].as_str() {
            "--workspace" => &mut workspace,
            "--git-dir" => &mut git_dir,
            "--common-dir" => &mut common_dir,
            "--bwrap" => &mut bwrap,
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
    if !bwrap.is_absolute() {
        return Err(format!(
            "dsh sandbox runner: --bwrap {} is not an absolute path; the runner executes the \
             bubblewrap the driver probed and never a `PATH` lookup",
            bwrap.display()
        ));
    }
    Ok((
        GitScope {
            workspace,
            git_dir,
            common_dir,
        },
        bwrap,
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

/// True when the profile binds the session workspace read-write, which
/// is what `workspace-write` means to the provider. A `read-only`
/// profile carries no such bind, and the runner then adds nothing.
fn workspace_writable(profile: &[String], workspace: &Path) -> bool {
    let mut index = 0;
    while index + 2 < profile.len() {
        if matches!(profile[index].as_str(), "--bind" | "--bind-try") {
            let source = Path::new(&profile[index + 1]);
            let destination = Path::new(&profile[index + 2]);
            if source == destination && same_path(source, workspace) {
                return true;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    false
}

/// Path identity for the bind check: resolved where both resolve, so a
/// symlinked spelling agrees with the provider's canonical workspace.
fn same_path(left: &Path, right: &Path) -> bool {
    let resolve = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    resolve(left) == resolve(right)
}

/// Why this scope cannot be served under the declared boundary, or
/// `None`. The driver calls it before the seat starts so an unsupported
/// layout refuses early, naming the remedy; the runner calls it again
/// because its argv is the last gate before bubblewrap.
///
/// Two layouts are refused rather than bound:
///
/// - the git directory IS the shared repository (`git_dir ==
///   common_dir` outside the workspace). A commit needs to create
///   `index.lock` and `HEAD.lock` in that directory, so serving it means
///   mounting the whole shared `.git` writable — every sibling worktree
///   and every submodule config and hook with it. A primary checkout
///   reached through a subdirectory, a `--separate-git-dir` checkout and
///   a worktree `.git` file redirected at the parent all resolve here.
/// - the per-worktree directory records a different worktree in its
///   `gitdir` pointer. Git follows a rewritten `.git` file without
///   checking the back-pointer, so without this check one seat could
///   bind and write a sibling's metadata.
///
/// A scope whose common directory already sits inside the workspace
/// needs nothing and is never refused: the provider's own bind covers
/// it.
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
    if let Some(recorded) = recorded_worktree(scope) {
        if !same_path(&recorded, &scope.workspace.join(".git")) {
            return Some(format!(
                "the per-worktree git directory {} records {} as its worktree, not this seat's \
                 workspace {}; refusing to bind another worktree's metadata",
                scope.git_dir.display(),
                recorded.display(),
                scope.workspace.display()
            ));
        }
    }
    None
}

/// The worktree `.git` file a linked worktree's `gitdir` pointer names,
/// when the host has one. `None` for a directory that carries no such
/// pointer: the caller then has nothing to compare and does not refuse
/// on this evidence alone.
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

/// The scoped write set, in mount order: later binds sit over earlier
/// ones, so a mask always follows the bind it hides. Refuses a scope
/// that [`scope_refusal`] rejects rather than mounting it wider.
fn scoped_git_binds(scope: &GitScope) -> Result<Vec<String>, String> {
    // A primary checkout keeps its whole git directory under the
    // workspace the provider already made writable: nothing to add.
    if scope.common_dir.starts_with(&scope.workspace) {
        return Ok(Vec::new());
    }
    if let Some(problem) = scope_refusal(scope) {
        return Err(problem);
    }
    let mut argv = Vec::new();
    let git = &scope.git_dir;
    let common = &scope.common_dir;
    // The per-worktree directory holds index, HEAD and its reflog. A
    // primary checkout keeps it under the workspace, where the provider
    // already made it writable.
    if !git.starts_with(&scope.workspace) {
        bind("--bind", &mut argv, git);
    }
    // Config, wherever git reads it, stays read-only: a hook path or a
    // signing program written here would be a program the host runs.
    bind("--ro-bind-try", &mut argv, &git.join("config"));
    // `config.worktree` is masked with an empty read-only file, not
    // `--ro-bind-try`: that flag no-ops when the host file is absent —
    // the normal state of a linked worktree — and the per-worktree
    // directory around it is writable, so a boxed command could CREATE
    // it. A repository that has run `git sparse-checkout` carries
    // `extensions.worktreeConfig`, so the host would then honour a
    // `core.hooksPath` the box wrote. Bubblewrap creates the mount point
    // for a missing destination, so the host gains an empty file and the
    // box gains no way to fill it.
    mask(&mut argv, &git.join("config.worktree"));
    // `commondir` and `gitdir` are the worktree's pointers back to the
    // shared repository and to its own `.git` file. Left writable, a
    // boxed command could redirect git at a `config` it wrote in the
    // workspace — a program the host then runs. Read-only: a write fails
    // EROFS, and unlinking or renaming over the mount point fails EBUSY.
    bind("--ro-bind-try", &mut argv, &git.join("commondir"));
    bind("--ro-bind-try", &mut argv, &git.join("gitdir"));
    tmpfs(&mut argv, &git.join("hooks"));
    // The shared store, refs and reflogs: the minimum a commit writes.
    // `--bind-try` keeps a directory git has not created yet from
    // refusing the run; the trusted driver pre-creates nothing.
    for name in ["objects", "refs", "logs"] {
        bind("--bind-try", &mut argv, &common.join(name));
    }
    bind("--bind-try", &mut argv, &common.join("packed-refs"));
    // The shared hooks are hidden and the shared config is read-only.
    // Nothing else under `.git` — sibling worktrees and submodule git
    // directories included — is bound.
    tmpfs(&mut argv, &common.join("hooks"));
    bind("--ro-bind-try", &mut argv, &common.join("config"));
    Ok(argv)
}

fn bind(flag: &str, argv: &mut Vec<String>, path: &Path) {
    let path = path.to_string_lossy().into_owned();
    argv.extend([flag.to_string(), path.clone(), path]);
}

/// Hide a path behind an empty read-only file, whether or not the host
/// file exists. Bubblewrap creates the mount point for a missing
/// destination under a writable parent, so the host gains an empty file
/// rather than a writable path.
fn mask(argv: &mut Vec<String>, path: &Path) {
    argv.extend([
        "--ro-bind".to_string(),
        "/dev/null".to_string(),
        path.to_string_lossy().into_owned(),
    ]);
}

fn tmpfs(argv: &mut Vec<String>, path: &Path) {
    argv.extend(["--tmpfs".to_string(), path.to_string_lossy().into_owned()]);
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
/// bubblewrap is a trusted path too: the runner executes it by absolute
/// path, so the boundary is the binary the driver probed, not whatever
/// `PATH` holds when the seat runs.
pub fn sandbox_row(program: &str, bwrap: &Path, scope: &GitScope) -> Result<String, String> {
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
