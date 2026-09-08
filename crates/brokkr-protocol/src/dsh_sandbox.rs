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
//! `workspace-write` seat whose git metadata lies outside the
//! workspace, and the runner adds exactly the scoped binds a commit
//! needs. It never mounts the parent checkout or the whole shared
//! `.git`: the per-worktree directory — its `index`, `HEAD` and reflogs —
//! and the shared `objects`, `refs` and `logs` are writable; `hooks` is
//! an empty tmpfs; the per-worktree and shared `config`, the per-worktree
//! `config.worktree`, `commondir` and `gitdir` are masked or read-only,
//! so nothing a boxed command writes can become a program the host runs
//! on its next git invocation — nor can it point git at a `config` it
//! wrote in the workspace. Every sibling worktree DIRECTORY and its own
//! per-worktree metadata, the parent checkout and every credential path
//! stay outside the write set. The shared ref store does not: a commit
//! must write it, so a seat can move a branch a sibling has checked out.
//! That residual is inherent to sharing a ref store through a kernel
//! bind and is recorded in decision 0054's consequences.
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
//! that repository's object store and refs read-write.
//!
//! The trusted paths travel in the runner's own argv, resolved by the
//! trusted driver BEFORE the seat starts, never re-resolved from a
//! workspace file the model can edit: the session workspace, the two
//! git directories, the bubblewrap the driver probed, and the empty file
//! the driver staged as a config mask. The runner executes that absolute
//! bubblewrap itself; it never searches `PATH` for one, so the boundary
//! is the binary the driver measured.

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
    let (scope, paths, rest) = parse_scope(args)?;
    let (profile, command) = split_profile(rest)?;
    if profile.len() < 3 || profile[0] != "--ro-bind" || profile[1] != "/" || profile[2] != "/" {
        return Err(
            "the dsh sandbox profile does not begin with `--ro-bind / /`; refusing to guess \
             the boundary it stands for"
                .to_string(),
        );
    }
    let mut argv = vec![paths.bwrap.to_string_lossy().into_owned()];
    argv.extend(profile.iter().cloned());
    if workspace_writable(profile, &scope.workspace) {
        argv.extend(scoped_git_binds(&scope, &paths.mask)?);
    }
    argv.push("--".to_string());
    argv.extend(command.iter().cloned());
    Ok(argv)
}

/// The two host paths the runner needs beside the git scope: the
/// bubblewrap the driver probed and the empty file it staged as a
/// config mask.
struct RunnerPaths {
    bwrap: PathBuf,
    mask: PathBuf,
}

/// The runner's own flags, and the profile that follows them. A flag
/// given twice, or without its path, is a malformed runner invocation,
/// not something to interpret generously. `--bwrap` and `--mask` must be
/// absolute: the runner executes the binary and mounts the file the
/// driver staged, and a relative spelling would be resolved by a working
/// directory the seat can move.
fn parse_scope(args: &[String]) -> Result<(GitScope, RunnerPaths, &[String]), String> {
    let mut workspace: Option<PathBuf> = None;
    let mut git_dir: Option<PathBuf> = None;
    let mut common_dir: Option<PathBuf> = None;
    let mut bwrap: Option<PathBuf> = None;
    let mut mask: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let slot = match args[index].as_str() {
            "--workspace" => &mut workspace,
            "--git-dir" => &mut git_dir,
            "--common-dir" => &mut common_dir,
            "--bwrap" => &mut bwrap,
            "--mask" => &mut mask,
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
    let mask = required(mask, "--mask")?;
    for (flag, path) in [("--bwrap", &bwrap), ("--mask", &mask)] {
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
        RunnerPaths { bwrap, mask },
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
///    runner would bind its objects, refs and reflogs read-write.
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

/// Why the staged mask file cannot stand in for the per-worktree
/// `config` and `config.worktree`, or `None`. It must be a real, empty,
/// regular file the seat cannot reach: a bind is only as read-only as
/// its SOURCE is unreachable, and a mask source inside the writable
/// workspace would let the box fill the file it is being masked with.
fn mask_refusal(mask: &Path, workspace: &Path) -> Option<String> {
    let problem = match std::fs::symlink_metadata(mask) {
        Err(error) => format!("cannot be read ({error})"),
        Ok(meta) if !meta.file_type().is_file() => {
            // `/dev/null` is the obvious wrong answer: bubblewrap mounts
            // every bind source with `MS_NODEV`, so a bound character
            // device cannot be opened inside the box and git calls an
            // unreadable config file a fatal error.
            "is not a regular file; bubblewrap mounts a bind source with MS_NODEV, so a device \
             node masks the path with something the box cannot open and git calls an \
             unreadable config file fatal"
                .to_string()
        }
        Ok(meta) if meta.len() != 0 => format!("is not empty ({} bytes)", meta.len()),
        Ok(_) if within(mask, workspace) => {
            "lies inside the seat's own writable workspace, where a boxed command could fill \
             the very file the mask stands for"
                .to_string()
        }
        Ok(_) => return None,
    };
    Some(format!("the config mask {} {problem}", mask.display()))
}

/// The scoped write set, in mount order: later binds sit over earlier
/// ones, so a mask always follows the bind it hides. Refuses a scope
/// that [`scope_refusal`] rejects rather than mounting it wider.
fn scoped_git_binds(scope: &GitScope, mask_source: &Path) -> Result<Vec<String>, String> {
    // A primary checkout keeps its whole git directory under the
    // workspace the provider already made writable: nothing to add.
    if scope.common_dir.starts_with(&scope.workspace) {
        return Ok(Vec::new());
    }
    if let Some(problem) = scope_refusal(scope) {
        return Err(problem);
    }
    if let Some(problem) = mask_refusal(mask_source, &scope.workspace) {
        return Err(problem);
    }
    let mut argv = Vec::new();
    let git = &scope.git_dir;
    let common = &scope.common_dir;
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
    mask(&mut argv, mask_source, &git.join("config"));
    mask(&mut argv, mask_source, &git.join("config.worktree"));
    // `commondir` and `gitdir` are the worktree's pointers back to the
    // shared repository and to its own `.git` file. Left writable, a
    // boxed command could redirect git at a `config` it wrote in the
    // workspace — a program the host then runs. Read-only: a write fails
    // EROFS, and unlinking or renaming over the mount point fails EBUSY.
    // These are hard binds, not `-try`: git writes both when it creates a
    // linked worktree, and a `-try` that no-oped on a missing source
    // would leave the box free to CREATE the pointer it may not rewrite.
    bind("--ro-bind", &mut argv, &git.join("commondir"));
    bind("--ro-bind", &mut argv, &git.join("gitdir"));
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

/// Hide a path behind the empty read-only file the driver staged,
/// whether or not the host file exists. Bubblewrap creates the mount
/// point for a missing destination under a writable parent, so the host
/// gains an empty file rather than a writable path.
///
/// The source is a REGULAR file, never `/dev/null`: bubblewrap mounts
/// every bind source with `MS_NODEV`, so a bound character device cannot
/// be opened inside the box, and git answers an unreadable configuration
/// file with `fatal: unknown error occurred while reading the
/// configuration files` — which would break every git command in the
/// seat, in exactly the repositories (`extensions.worktreeConfig`) the
/// mask exists for. Measured on Linux 6.17 with bubblewrap 0.11.
fn mask(argv: &mut Vec<String>, source: &Path, path: &Path) {
    argv.extend([
        "--ro-bind".to_string(),
        source.to_string_lossy().into_owned(),
        path.to_string_lossy().into_owned(),
    ]);
}

/// Stage the empty regular file the runner masks a linked worktree's
/// per-worktree `config` and `config.worktree` with. The driver holds it
/// for the seat's whole life and names it in the runner's argv, so every
/// command in the seat masks with a file no boxed command can reach: it
/// lives outside the session workspace, and the seat's own `/tmp` is a
/// fresh tmpfs of dsh's making.
pub fn stage_mask_file() -> Result<tempfile::NamedTempFile, String> {
    stage_mask_file_in(|| {
        tempfile::Builder::new()
            .prefix("brokkr-dsh-config-mask-")
            .tempfile()
    })
}

/// The mask over an injected file, so the one way staging can fail is
/// reachable from a test without a full disk.
fn stage_mask_file_in(
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<tempfile::NamedTempFile, String> {
    let file = create().map_err(|error| {
        format!("dsh driver: could not stage the git config mask for the seat: {error}")
    })?;
    // The read-only bind is what actually refuses the write; this only
    // makes the host file say the same thing, and a host that will not
    // set the mode changes nothing about the boundary.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o444));
    }
    Ok(file)
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
/// bubblewrap and the staged config mask are trusted paths too: the
/// runner executes and mounts exactly what the driver staged, so the
/// boundary is what the driver measured rather than whatever the seat's
/// environment holds when a command runs.
pub fn sandbox_row(
    program: &str,
    bwrap: &Path,
    mask: &Path,
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
        ("--mask", mask),
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
