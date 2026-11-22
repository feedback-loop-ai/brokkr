//! The engine's HOME for one agy launch (#1200, G5), and the workspace
//! check that runs once it is made, last before agy starts (G6, M1).
//!
//! HOME is the only measured way to move agy's state, and the only thing
//! that shuts out the operator's own MCP servers, plugins and hooks: three
//! names the binary holds were tried and none moved it (the record's
//! `env-*` turns), and a scratch HOME started none of the user's or the
//! user's plugins' servers (`iso-02`, `iso-05`, `iso-06`). A command the
//! agent runs inherits that HOME, so every other top-level entry of the
//! operator's is linked in by name, as `child-env-linked-home` measured;
//! without that, `cargo` fails (`child-env-scratch-home`).
//!
//! Each HOME is a hands session (`hands/session.rs`) under one parent that
//! is agy's alone: its owner's pid in its name and its lock held for the
//! launch's life. A driver the engine's deadline kills drops nothing, so
//! the next launch reaps each HOME whose owner is dead before it makes its
//! own, and leaves a live owner's, and one it cannot judge, alone (J5).
//!
//! The workspace's own servers cannot be shut out: the CLI reads
//! `.agents/mcp_config.json` and `.agents/plugins/` from the working
//! directory and directories above it (`iso-02`, `iso-05`, `iso-06`); a
//! workspace hook ran in a workspace nobody trusted and approved its own
//! command (`ambient-workspace-hook`). Where that walk ends is agy's, and
//! the record measures it (`git-root-*`); no code here mirrors it. Every
//! directory above the working directory is checked instead (L1), so a
//! workspace that carries any of them anywhere above it is refused. The
//! check runs after the HOME is made, directly before agy starts (M1).
//!
//! The session tree holds the engine's records, its owner's name and its
//! lock, and one directory, `home`, that is the child's HOME: nothing of the
//! operator's shares a directory with them (K7).

use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::LaunchError;
use crate::adapters::mcp;
use crate::hands::{Session, SessionError};

/// The operator's login, below a HOME: the one file a signed-in agy reads
/// (the record's Host and versions). Its content is never read here.
const STATE: &str = ".gemini/antigravity-cli";
const LOGIN: &str = "antigravity-oauth-token";

/// The name of the one parent every engine HOME is made under, below the
/// temporary root, before the effective uid that owns it.
const PARENT_PREFIX: &str = "brokkr-agy-homes-";

/// The label of each HOME's session under that parent.
const LABEL: &str = "agy";

/// The one directory of a session's tree that is the child's HOME (K7).
const HOME_DIR: &str = "home";

/// The type and permission bits of a parent that is this user's alone, and
/// what they read: a directory nobody else may enter.
const PRIVATE_DIR: (u32, u32) = (0o170_077, 0o040_000);

/// Where agy reads its user configuration, below a HOME.
const CONFIG: &str = ".gemini/config";

/// The operator's top-level entries never linked into the engine's HOME:
/// agy's own state and configuration, and the user's agent configuration.
const UNLINKED: [&str; 2] = [".gemini", ".agents"];

/// What a workspace may not carry, in the working directory or any
/// directory above it: the configuration agy starts or runs from a
/// workspace.
const WORKSPACE_CONFIGURATION: [&str; 3] = [
    ".agents/mcp_config.json",
    ".agents/plugins",
    ".agents/hooks.json",
];

/// Where one launch runs: the engine's temporary root its HOME is made
/// under, and the operator's own HOME, where the driver has one.
#[derive(Debug, Clone, Copy)]
pub(super) struct Homes<'a> {
    pub(super) temp: &'a Path,
    pub(super) operator: Option<&'a Path>,
}

/// What the engine writes into its HOME's configuration: the MCP document
/// and the gate's hooks.
#[derive(Debug, Clone, Copy)]
pub(super) struct Written<'a> {
    pub(super) mcp: &'a Value,
    pub(super) hooks: &'a str,
}

/// The engine's HOME for one launch: the `home` directory, mode 0700, of a
/// session of its own, that goes with the value, on every path out of the
/// turn. Its path is UTF-8, so the child's environment names it exactly.
pub(super) struct Home {
    /// Held for its lock and its drop, which removes the HOME and all in
    /// it, the links themselves and never what they name.
    _session: Session,
    path: String,
}

impl Home {
    pub(super) fn path(&self) -> &str {
        &self.path
    }
}

/// The parent every engine HOME under `temp` is made in.
pub(super) fn parent(temp: &Path) -> PathBuf {
    let uid = rustix::process::geteuid().as_raw();
    temp.join(format!("{PARENT_PREFIX}{uid}"))
}

/// The engine's HOME for a launch under `homes`, holding a link to the
/// operator's login, the engine's `written` configuration, and a link to
/// each other top-level entry of the operator's HOME whose name is UTF-8,
/// [`UNLINKED`] left out. With no login file, the launch is refused before
/// anything is made. Each HOME a dead owner left beside it is reaped first.
pub(super) fn made(homes: Homes<'_>, written: Written<'_>) -> Result<Home, LaunchError> {
    let operator = (homes.operator)
        .filter(|home| !home.as_os_str().is_empty())
        .ok_or(LaunchError::SignedOut)?;
    let login = operator.join(STATE).join(LOGIN);
    // Only whether it is there: what it holds is the operator's.
    std::fs::metadata(&login)
        .ok()
        .ok_or(LaunchError::SignedOut)?;
    let parent = owned_parent(homes.temp)?;
    // Only removed where its owner is dead and holds no lock; what this
    // reaping kept or removed is no fact of this launch's.
    crate::hands::reap_dead_sessions_under(&parent);
    let session = Session::create_under(&parent, LABEL).map_err(unsessioned)?;
    let home = session.path().join(HOME_DIR);
    let path = home.to_str().ok_or(LaunchError::Unnamed)?.to_string();
    furnished(&home, operator, &login, written).map_err(LaunchError::Unhomed)?;
    Ok(Home {
        _session: session,
        path,
    })
}

/// The one parent under `temp`, made owner-only where it is missing, and
/// refused where it is not a directory of this user's that nobody else may
/// enter: another user's would hold the engine's HOME directories.
fn owned_parent(temp: &Path) -> Result<PathBuf, LaunchError> {
    let parent = parent(temp);
    match std::fs::DirBuilder::new().mode(0o700).create(&parent) {
        // Made before, by this user or by another: the read below judges.
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
            return Err(LaunchError::Unhomed(error))
        }
        _ => {}
    }
    let uid = rustix::process::geteuid().as_raw();
    let read = std::fs::symlink_metadata(&parent).ok();
    let shape = read.map(|meta| (meta.uid(), meta.mode() & PRIVATE_DIR.0));
    match shape == Some((uid, PRIVATE_DIR.1)) {
        true => Ok(parent),
        false => Err(LaunchError::Unowned),
    }
}

/// A session's error as the launch's refusal, its lock's path withheld: the
/// session is made under the temporary directory, whose path may not be
/// UTF-8 (#1200 V1).
pub(super) fn unsessioned(error: SessionError) -> LaunchError {
    match error {
        SessionError::Io(cause) => LaunchError::Unsessioned(cause),
        SessionError::Lock { cause, .. } => LaunchError::Unlocked(cause),
    }
}

/// `home` furnished as [`made`] describes, owner-only.
fn furnished(
    home: &Path,
    operator: &Path,
    login: &Path,
    written: Written<'_>,
) -> std::io::Result<()> {
    std::fs::create_dir(home)?;
    std::fs::set_permissions(home, std::fs::Permissions::from_mode(0o700))?;
    let (state, config) = (home.join(STATE), home.join(CONFIG));
    std::fs::create_dir_all(&state)?;
    std::os::unix::fs::symlink(login, state.join(LOGIN))?;
    std::fs::create_dir_all(&config)?;
    mcp::written(&config.join("mcp_config.json"), written.mcp)?;
    std::fs::write(config.join("hooks.json"), written.hooks)?;
    let names = std::fs::read_dir(operator)?
        .flatten()
        .map(|entry| entry.file_name());
    let linked = names.filter(|name| name.to_str().is_some_and(|name| !UNLINKED.contains(&name)));
    for name in linked {
        std::os::unix::fs::symlink(operator.join(&name), home.join(&name))?;
    }
    Ok(())
}

/// The seat's `workdir` resolved, before anything is made: the path the
/// child runs in, its `PWD` names (L2) and [`unconfigured`] checks. A
/// directory that cannot be resolved, named as the seat gave it, and a
/// resolved path that is not UTF-8 refuse.
pub(super) fn resolved(workdir: &str) -> Result<String, LaunchError> {
    let given = if workdir.is_empty() { "." } else { workdir };
    let resolved = std::fs::canonicalize(given)
        .map_err(|error| LaunchError::Unrooted(given.to_string(), error))?;
    texted(resolved)
}

/// Refuse a launch in `resolved` whose workspace carries agy's own
/// configuration: an entry at any of [`WORKSPACE_CONFIGURATION`], of any
/// type, in the resolved working directory or any directory above it, to
/// the filesystem root (L1). agy reads no directory but these, wherever its
/// walk ends, so the check keeps no notion of where it ends and asks git
/// nothing. The first path found is named, and a path that cannot be read
/// refuses, naming it. This check has one home, and runs once, last before
/// agy starts (M1): agy walks up `PWD` (`pwd-logical`, `pwd-physical`), and
/// its `PWD` is `resolved`.
pub(super) fn unconfigured(resolved: &str) -> Result<(), LaunchError> {
    for dir in Path::new(resolved).ancestors() {
        if let Some(path) = configured(dir)? {
            return Err(LaunchError::Configured(path));
        }
    }
    Ok(())
}

/// `resolved` as the text the shared spawn and the child's `PWD` take, or
/// refused where it is not UTF-8, naming nothing of it: a path rendered in
/// any form but its own bytes would carry a bound value standing in it past
/// the mask (#1200 U1).
fn texted(resolved: PathBuf) -> Result<String, LaunchError> {
    (resolved.into_os_string().into_string()).map_err(|_| LaunchError::Untexted)
}

/// The first of [`WORKSPACE_CONFIGURATION`] present in `dir`, whatever it
/// is, a link included whatever it points at.
fn configured(dir: &Path) -> Result<Option<PathBuf>, LaunchError> {
    for relative in WORKSPACE_CONFIGURATION {
        let path = dir.join(relative);
        if entry(&path)?.is_some() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// What stands at `path`, not followed, or nothing where nothing does; a
/// path that cannot be read is no absence.
fn entry(path: &Path) -> Result<Option<std::fs::Metadata>, LaunchError> {
    use std::io::ErrorKind::{NotADirectory, NotFound};
    match std::fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(error) if matches!(error.kind(), NotFound | NotADirectory) => Ok(None),
        Err(error) => Err(LaunchError::Unread(path.to_path_buf(), error)),
    }
}
