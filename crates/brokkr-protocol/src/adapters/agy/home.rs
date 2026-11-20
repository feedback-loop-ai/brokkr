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
//! The workspace's own servers cannot be shut out: the CLI reads
//! `.agents/mcp_config.json` and `.agents/plugins/` from the working
//! directory and each directory above it up to its git root, and nothing
//! above that root or above a directory in no repository (`iso-02`,
//! `iso-05`, `iso-06`); a workspace hook ran in a workspace nobody trusted
//! and approved its own command (`ambient-workspace-hook`). So a workspace
//! that carries any of them is refused before the HOME is made.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::LaunchError;

/// The operator's login, below a HOME: the one file a signed-in agy reads
/// (the record's Host and versions). Its content is never read here.
const STATE: &str = ".gemini/antigravity-cli";
const LOGIN: &str = "antigravity-oauth-token";

/// The name every engine HOME begins with, under the temporary root.
pub(super) const HOME_PREFIX: &str = "brokkr-agy-home-";

/// Where agy reads its user configuration, below a HOME.
const CONFIG: &str = ".gemini/config";

/// The operator's top-level entries never linked into the engine's HOME:
/// agy's own state and configuration, and the user's agent configuration.
const UNLINKED: [&str; 2] = [".gemini", ".agents"];

/// What a workspace may not carry, from the working directory up to its git
/// root: the configuration agy starts or runs from a workspace.
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
    pub(super) mcp: &'a str,
    pub(super) hooks: &'a str,
}

/// The engine's HOME for one launch: a directory of its own, mode 0700,
/// that goes with the value, on every path out of the turn. Its path is
/// UTF-8, so the child's environment names it exactly.
pub(super) struct Home {
    /// Held for its drop, which removes the HOME and all in it, the links
    /// themselves and never what they name.
    _dir: tempfile::TempDir,
    path: String,
}

impl Home {
    pub(super) fn path(&self) -> &str {
        &self.path
    }
}

/// The engine's HOME for a launch under `homes`, holding a link to the
/// operator's login, the engine's `written` configuration, and a link to
/// each other top-level entry of the operator's HOME whose name is UTF-8,
/// [`UNLINKED`] left out. With no login file, the launch is refused before
/// anything is made.
pub(super) fn made(homes: Homes<'_>, written: Written<'_>) -> Result<Home, LaunchError> {
    let operator = (homes.operator)
        .filter(|home| !home.as_os_str().is_empty())
        .ok_or(LaunchError::SignedOut)?;
    let login = operator.join(STATE).join(LOGIN);
    // Only whether it is there: what it holds is the operator's.
    std::fs::metadata(&login)
        .ok()
        .ok_or(LaunchError::SignedOut)?;
    // Owner-only from its creation: the default would let others read in.
    let dir = (tempfile::Builder::new().prefix(HOME_PREFIX))
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(homes.temp)
        .map_err(LaunchError::Unhomed)?;
    let path = dir.path().to_str().ok_or(LaunchError::Unnamed)?.to_string();
    furnished(dir.path(), operator, &login, written).map_err(LaunchError::Unhomed)?;
    Ok(Home { _dir: dir, path })
}

/// `home` furnished as [`made`] describes.
fn furnished(
    home: &Path,
    operator: &Path,
    login: &Path,
    written: Written<'_>,
) -> std::io::Result<()> {
    let (state, config) = (home.join(STATE), home.join(CONFIG));
    std::fs::create_dir_all(&state)?;
    std::os::unix::fs::symlink(login, state.join(LOGIN))?;
    std::fs::create_dir_all(&config)?;
    std::fs::write(config.join("mcp_config.json"), written.mcp)?;
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
/// configuration: any of [`WORKSPACE_CONFIGURATION`] in the working
/// directory or a directory above it up to its git root, inclusive, or in
/// the working directory alone where it is in no git repository (`iso-06`).
/// The first path found is named. This check has one home.
pub(super) fn unconfigured(workdir: &str) -> Result<(), LaunchError> {
    let workdir = if workdir.is_empty() { "." } else { workdir };
    let workdir = std::fs::canonicalize(workdir).map_err(LaunchError::Unrooted)?;
    let ancestors: Vec<&Path> = workdir.ancestors().collect();
    let root = ancestors
        .iter()
        .position(|dir| dir.join(".git").symlink_metadata().is_ok());
    let searched = &ancestors[..=root.unwrap_or(0)];
    let found = searched.iter().find_map(|dir| configured(dir));
    found.map_or(Ok(()), |path| Err(LaunchError::Configured(path)))
}

/// The first of [`WORKSPACE_CONFIGURATION`] present in `dir`, a link
/// included whatever it points at.
fn configured(dir: &Path) -> Option<PathBuf> {
    (WORKSPACE_CONFIGURATION.iter())
        .map(|relative| dir.join(relative))
        .find(|path| path.symlink_metadata().is_ok())
}
