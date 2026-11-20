//! The engine's HOME for one agy launch (#1200, G5), and the workspace
//! check that runs before it is made (G6).
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
//! directory and each directory above it up to its git root, and nothing
//! above that root or above a directory in no repository (`iso-02`,
//! `iso-05`, `iso-06`); a workspace hook ran in a workspace nobody trusted
//! and approved its own command (`ambient-workspace-hook`). The walk ends
//! at a `.git` file, whatever it says (`git-root-junk-dot-git-file`), or at
//! the root git names, and not at a `.git` directory git does not take for a
//! repository (`git-root-linked-worktree`, `git-root-nested-repository`,
//! `git-root-empty-dot-git-dir`). Where git names no root and a `.git`
//! entry stands above, where agy's walk ends is unknown, and the launch is
//! refused (K4). So a workspace that carries any of them is refused before
//! the HOME is made.
//!
//! The session tree holds the engine's records, its owner's name and its
//! lock, and one directory, `home`, that is the child's HOME: nothing of the
//! operator's shares a directory with them (K7).

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::LaunchError;
use crate::adapters::mcp;
use crate::hands::Session;

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

/// What a workspace may not carry, from the working directory up to where
/// agy's walk ends: the configuration agy starts or runs from a workspace.
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
    let session = Session::create_under(&parent, LABEL).map_err(LaunchError::Unsessioned)?;
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
        false => Err(LaunchError::Unowned(parent)),
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

/// Refuse a launch in `workdir` whose workspace carries agy's own
/// configuration: an entry at any of [`WORKSPACE_CONFIGURATION`], of any
/// type, in the working directory or a directory above it up to where
/// agy's walk ends, inclusive, or in the working directory alone where
/// nothing ends it (`iso-06`). The first path found is named, and a path
/// that cannot be read refuses, naming it. This check has one home.
pub(super) fn unconfigured(workdir: &str) -> Result<(), LaunchError> {
    unconfigured_with("git", workdir)
}

/// [`unconfigured`] asking `git` for the working directory's top level.
pub(super) fn unconfigured_with(git: &str, workdir: &str) -> Result<(), LaunchError> {
    let given = if workdir.is_empty() { "." } else { workdir };
    let workdir = std::fs::canonicalize(given)
        .map_err(|error| LaunchError::Unrooted(given.to_string(), error))?;
    let ancestors: Vec<&Path> = workdir.ancestors().collect();
    let end = walk_end(git, &ancestors)?;
    for dir in &ancestors[..=end] {
        if let Some(path) = configured(dir)? {
            return Err(LaunchError::Configured(path));
        }
    }
    Ok(())
}

/// Where agy's walk up `ancestors` ends, as an index into them (K4): at the
/// first `.git` file, or at the top level git names, asked once. Where git
/// names none, a `.git` entry of any other type below the first `.git` file
/// refuses, naming its directory: agy's walk ends somewhere git could not
/// say. With neither, the working directory is read alone (`iso-06`).
fn walk_end(git: &str, ancestors: &[&Path]) -> Result<usize, LaunchError> {
    let top = top_level(git, ancestors[0])?;
    for (at, dir) in ancestors.iter().enumerate() {
        let dot_git = entry(&dir.join(".git"))?;
        let found = (dot_git.as_ref()).map(std::fs::Metadata::is_file);
        if Some(*dir) == top.as_deref() || found == Some(true) {
            return Ok(at);
        }
        if top.is_none() && found.is_some() {
            return Err(LaunchError::Rootless(dir.to_path_buf()));
        }
    }
    Ok(0)
}

/// The top level git names for `workdir`, asked once, or none where git
/// names none, for whatever reason it gives; nothing of git's text is kept
/// but the path it names. A git that cannot be run refuses (J8): the walk
/// would not know where agy's ends.
fn top_level(git: &str, workdir: &Path) -> Result<Option<PathBuf>, LaunchError> {
    let mut asked = Command::new(git);
    asked
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(workdir);
    let out = crate::process::spawned::output(&mut asked).map_err(LaunchError::Unasked)?;
    // One path and its newline: the path may end in a space.
    let mut named = out.stdout;
    named.pop();
    Ok((out.status.success() && !named.is_empty())
        .then(|| PathBuf::from(OsString::from_vec(named))))
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
