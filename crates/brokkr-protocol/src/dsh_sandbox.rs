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
//! `.git`: the per-worktree directory, `objects`, `refs` and `logs` are
//! writable; the shared `hooks` are an empty tmpfs and the shared and
//! per-worktree `config` files are read-only, so nothing a boxed
//! command writes can become a program the host runs on its next git
//! invocation. Sibling worktrees, the parent checkout and every
//! credential path stay outside the write set.
//!
//! The three git paths travel in the runner's own argv, resolved by the
//! trusted driver through Git BEFORE the seat starts, never re-resolved
//! from a workspace file the model can edit.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The stderr prefix dsh classifies as a runner failure, so a refusal
/// here reads "the command never ran", never "the command was denied".
pub const RUNNER_FAILURE_SIGNATURE: &str = "brokkr-dsh-sandbox-runner: ";

/// The bwrap-compatible verb the adapter names as dsh's runner.
pub const RUNNER_VERB: &str = "dsh-sandbox-runner";

/// The three paths the driver resolved through Git before the seat
/// started, carried in the runner's own argv.
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
/// command. Only the profile prefix is read; the command is opaque.
pub fn runner_argv(args: &[String]) -> Result<Vec<String>, String> {
    let (scope, rest) = parse_scope(args)?;
    let (profile, command) = split_profile(rest)?;
    if profile.len() < 3 || profile[0] != "--ro-bind" || profile[1] != "/" || profile[2] != "/" {
        return Err(
            "the dsh sandbox profile does not begin with `--ro-bind / /`; refusing to guess \
             the boundary it stands for"
                .to_string(),
        );
    }
    let mut argv = vec!["bwrap".to_string()];
    argv.extend(profile.iter().cloned());
    if workspace_writable(profile, &scope.workspace) {
        argv.extend(scoped_git_binds(&scope));
    }
    argv.push("--".to_string());
    argv.extend(command.iter().cloned());
    Ok(argv)
}

/// The runner's own flags, and the profile that follows them. A flag
/// given twice, or without its path, is a malformed runner invocation,
/// not something to interpret generously.
fn parse_scope(args: &[String]) -> Result<(GitScope, &[String]), String> {
    let mut workspace: Option<PathBuf> = None;
    let mut git_dir: Option<PathBuf> = None;
    let mut common_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let slot = match args[index].as_str() {
            "--workspace" => &mut workspace,
            "--git-dir" => &mut git_dir,
            "--common-dir" => &mut common_dir,
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
    Ok((
        GitScope {
            workspace: required(workspace, "--workspace")?,
            git_dir: required(git_dir, "--git-dir")?,
            common_dir: required(common_dir, "--common-dir")?,
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

/// The scoped write set, in mount order: later binds sit over earlier
/// ones, so a mask always follows the bind it hides.
fn scoped_git_binds(scope: &GitScope) -> Vec<String> {
    // A primary checkout keeps its whole git directory under the
    // workspace the provider already made writable: nothing to add.
    if scope.common_dir.starts_with(&scope.workspace) {
        return Vec::new();
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
    bind("--ro-bind-try", &mut argv, &git.join("config.worktree"));
    tmpfs(&mut argv, &git.join("hooks"));
    // The shared store, refs and reflogs: the minimum a commit writes.
    // `--bind-try` keeps a directory git has not created yet from
    // refusing the run; the trusted driver pre-creates nothing.
    for name in ["objects", "refs", "logs"] {
        bind("--bind-try", &mut argv, &common.join(name));
    }
    bind("--bind-try", &mut argv, &common.join("packed-refs"));
    // The shared hooks are hidden and the shared config is read-only.
    // Nothing else under `.git` — sibling worktrees included — is bound.
    tmpfs(&mut argv, &common.join("hooks"));
    bind("--ro-bind-try", &mut argv, &common.join("config"));
    argv
}

fn bind(flag: &str, argv: &mut Vec<String>, path: &Path) {
    let path = path.to_string_lossy().into_owned();
    argv.extend([flag.to_string(), path.clone(), path]);
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
/// travels beside the seat's transcript and model rows.
pub fn sandbox_row(program: &str, scope: &GitScope) -> Result<String, String> {
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
