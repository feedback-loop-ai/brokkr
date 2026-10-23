//! Where a node-family stack's runner is installed, resolved once, at
//! `init` (#717). The verify box binds nothing a declaration does not
//! name, and its `PATH` reaches `~/.cargo/bin` and the system directories
//! only, so a runner a version manager installed under `$HOME` was not
//! found there ("npm: command not found"). `init` looks each binary up
//! on its own `PATH`, follows links to the real file, and records what
//! the box needs: the install's root, bound read-only on verify's hands in
//! `bundle.json`, and the directories `scripts/verify-seat.sh` puts at the
//! head of `PATH`. Nothing is re-resolved when a run starts.
//!
//! An install's root is the nearest directory holding both the directory
//! `PATH` found the binary in and the file it links to, starting from
//! that directory's parent: a volta shim (`~/.volta/bin/npm` linking to
//! `~/.volta/bin/volta-shim`) is rooted at `~/.volta`, which also holds
//! the images the shim runs; an nvm install at its version directory. A
//! binary whose directory and file both stand under a toolchain root
//! every box already reads read-only (`HOST_TOOLCHAIN_BINDS`) adds no
//! bind. A root that would bind `$HOME`, an ancestor of it, `~/.local`
//! whole or `/` is refused, and so is a binary not found: either way
//! verify fails closed with the sentence `init` printed.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use brokkr_protocol::hands::HOST_TOOLCHAIN_BINDS;

use super::{shell_literal, Tool};

/// The node-family runners the detection tables can lead a command with,
/// and whether each runs on the `node` it finds on `PATH`: npm, npx,
/// pnpm and yarn are node programs, and bun is its own runtime.
const RUNNERS: &[(&str, bool)] = &[
    ("npm", true),
    ("npx", true),
    ("pnpm", true),
    ("yarn", true),
    ("bun", false),
    ("bunx", false),
];

/// What `init` found of the node-family binaries a stack runs through.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Reach {
    /// Every binary was found: the install roots verify's box binds
    /// read-only (`~/`-spelled under `$HOME`), the directories put at the
    /// head of its `PATH`, and the `VOLTA_HOME` a volta shim reads, which
    /// the box's private `HOME` would otherwise hide.
    Found {
        binds: Vec<String>,
        path: Vec<PathBuf>,
        volta: Option<PathBuf>,
    },
    /// This binary was not on `PATH`.
    Missing(&'static str),
    /// This binary's install root is one `init` will not bind.
    Unbindable(&'static str, PathBuf),
}

impl Reach {
    /// The sentence `init` prints, and verify fails closed with, when the
    /// runner cannot be reached; none when it can.
    pub(super) fn refusal(&self) -> Option<String> {
        match self {
            Reach::Found { .. } => None,
            Reach::Missing(binary) => Some(format!(
                "{binary} was not found on PATH when `brokkr init` ran; install it, \
                 or fill in .brokkr/scripts/verify-seat.sh"
            )),
            Reach::Unbindable(binary, root) => Some(format!(
                "{binary} is installed under {}, which `brokkr init` will not bind \
                 into verify's hands; install it under a directory of its own, or \
                 fill in .brokkr/scripts/verify-seat.sh",
                root.display()
            )),
        }
    }

    /// The binds verify's hands declare for the runner.
    pub(super) fn binds(&self) -> Vec<serde_json::Value> {
        let binds: &[String] = match self {
            Reach::Found { binds, .. } => binds,
            Reach::Missing(_) | Reach::Unbindable(..) => &[],
        };
        let bind = |path: &String| serde_json::json!({"path": path, "mode": "ro"});
        binds.iter().map(bind).collect()
    }

    /// The lines `scripts/verify-seat.sh` runs before the suite: the
    /// `PATH` and `VOLTA_HOME` it found, or the fail-closed result.
    pub(super) fn script(&self) -> String {
        match self {
            Reach::Found { path, volta, .. } => {
                let path: Vec<_> = path.iter().map(|dir| dir.to_string_lossy()).collect();
                let mut lines =
                    format!("export PATH={}:\"$PATH\"\n", shell_literal(&path.join(":")));
                if let Some(volta) = volta {
                    let volta = shell_literal(&volta.to_string_lossy());
                    lines.push_str(&format!("export VOLTA_HOME={volta}\n"));
                }
                lines
            }
            Reach::Missing(_) | Reach::Unbindable(..) => format!(
                "printf '%s' {} > \"$notes\"\nwrite_result fail; exit 0\n",
                shell_literal(&self.refusal().unwrap_or_default())
            ),
        }
    }
}

/// Resolve the node-family binaries `runners` names, and `node` where one
/// of them runs on it, against `path`, with `home` the operator's `$HOME`.
/// `None` when no runner is node-family.
pub(super) fn reach(runners: &[Tool], path: &OsStr, home: &Path) -> Option<Reach> {
    let named = |name: &&str| runners.iter().any(|tool| tool.name == *name);
    let mut wanted: Vec<&'static str> = RUNNERS
        .iter()
        .map(|(name, _)| *name)
        .filter(named)
        .collect();
    if wanted.is_empty() {
        return None;
    }
    if RUNNERS.iter().any(|(name, node)| *node && named(name)) {
        wanted.push("node");
    }
    let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let mut found = Found::default();
    for binary in wanted {
        let Some(install) = install(binary, path) else {
            return Some(Reach::Missing(binary));
        };
        if let Err(root) = found.add(install, &home) {
            return Some(Reach::Unbindable(binary, root));
        }
    }
    Some(Reach::Found {
        binds: found.binds,
        path: found.path,
        volta: found.volta,
    })
}

/// One binary as `PATH` holds it: the directory it was found in and the
/// file it resolves to, both canonical.
struct Install {
    dir: PathBuf,
    real: PathBuf,
}

/// The first `binary` on `path` that resolves to a file.
fn install(binary: &str, path: &OsStr) -> Option<Install> {
    std::env::split_paths(path).find_map(|dir| {
        let dir = dir.canonicalize().ok()?;
        let real = dir.join(binary).canonicalize().ok()?;
        real.is_file().then_some(Install { dir, real })
    })
}

/// What the found binaries add up to, in the order they were found.
#[derive(Default)]
struct Found {
    binds: Vec<String>,
    path: Vec<PathBuf>,
    volta: Option<PathBuf>,
}

impl Found {
    /// Add one install: its directory to `PATH`, and its root as a bind
    /// unless the box reads it already. `Err` is a root too broad to bind.
    fn add(&mut self, install: Install, home: &Path) -> Result<(), PathBuf> {
        let root = root(&install);
        if install.real.file_name() == Some(OsStr::new("volta-shim")) {
            self.volta = Some(root.clone());
        }
        if !self.path.contains(&install.dir) {
            self.path.push(install.dir.clone());
        }
        let paths = [install.dir.as_path(), install.real.as_path()];
        if paths.into_iter().all(read_by_every_box) {
            return Ok(());
        }
        if too_broad(&root, home) {
            return Err(root);
        }
        let spelled = match root.strip_prefix(home) {
            Ok(rest) if home.is_absolute() => format!("~/{}", rest.display()),
            _ => root.display().to_string(),
        };
        if !self.binds.contains(&spelled) {
            self.binds.push(spelled);
        }
        Ok(())
    }
}

/// The nearest directory holding both the binary's directory's parent
/// and the file it resolves to.
fn root(install: &Install) -> PathBuf {
    let base = install.dir.parent().unwrap_or(&install.dir);
    let holds = |ancestor: &&Path| install.real.starts_with(ancestor);
    base.ancestors()
        .find(holds)
        .unwrap_or(Path::new("/"))
        .to_path_buf()
}

/// Under a root every box binds read-only at its own path.
fn read_by_every_box(path: &Path) -> bool {
    HOST_TOOLCHAIN_BINDS
        .iter()
        .any(|root| path.starts_with(root))
}

/// A root that would bind `/`, `$HOME` or an ancestor of it, or `~/.local`
/// whole: each holds far more than one install, credentials included.
fn too_broad(root: &Path, home: &Path) -> bool {
    root.parent().is_none() || home.starts_with(root) || root == home.join(".local")
}

#[cfg(test)]
mod tests;
