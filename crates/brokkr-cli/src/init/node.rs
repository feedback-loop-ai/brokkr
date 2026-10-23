//! Where a node-family stack's runner is installed, resolved once, at
//! `init` (#717). The verify box binds nothing a declaration does not
//! name, and its `PATH` reaches `~/.cargo/bin` and the system directories
//! only, so a runner a version manager installed under `$HOME` was not
//! found there ("npm: command not found"). `init` looks each binary up
//! on its own `PATH`, follows links to the real file, and records what
//! the box needs: the install, bound read-only on verify's hands in
//! `bundle.json`, and the directories `scripts/verify-seat.sh` puts at the
//! head of `PATH`. Nothing is re-resolved when a run starts.
//!
//! A binary is found where `PATH` lookup finds it: the first file of its
//! name, followed through links, that this user may execute. One whose
//! real file stands in the directory `PATH` found it in is bound alone,
//! as that one file. Otherwise, and for a volta shim (`~/.volta/bin/npm`
//! linking to `~/.volta/bin/volta-shim`, which runs the images under
//! `~/.volta/tools`), the install has a root: the nearest directory
//! holding both the real file and the `PATH` directory, starting from that
//! directory's parent when it is a `bin`, and from the directory itself
//! when it is not. Nothing is bound that holds more than the install
//! (#717): the root must be one of the [`Layout`]s `init` recognises, and
//! every entry it holds one that layout names, or it is refused, naming
//! the entry. Volta's root, bun's and pnpm's home are bound whole; a node
//! prefix, which cannot be told from a prefix other software shares, only
//! by its `bin` and `lib/node_modules`. A binary whose directory and file
//! both stand under a toolchain root every box already reads read-only
//! (`HOST_TOOLCHAIN_BINDS`) adds no bind. A root that would bind `$HOME`,
//! an ancestor of it, `~/.local` whole or `/` is refused before its
//! entries are read, and so is a binary not found: either way verify fails
//! closed with the sentence `init` printed.

use std::ffi::{OsStr, OsString};
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
    /// Every binary was found: the files and directories verify's box
    /// binds read-only (`~/`-spelled under `$HOME`, none under another),
    /// the directories put at the head of its `PATH`, and the
    /// `VOLTA_HOME` a volta shim reads, which the box's private `HOME`
    /// would otherwise hide.
    Found {
        binds: Vec<String>,
        path: Vec<PathBuf>,
        volta: Option<PathBuf>,
    },
    /// This binary was not on `PATH`.
    Missing(&'static str),
    /// This binary's install root is one `init` will not bind, and why.
    Unbindable(&'static str, PathBuf, Unbound),
}

/// Why an install's root is not bound.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unbound {
    /// `/`, `$HOME` or an ancestor of it, or `~/.local` whole.
    TooBroad,
    /// The root holds this entry, which its layout does not name, or a
    /// node prefix's `bin` does (spelled `bin/<name>`).
    Foreign(Layout, OsString),
    /// The root's entries, or a node prefix's `bin`, could not be listed,
    /// or the prefix has no `lib/node_modules` directory of its own.
    Unreadable,
}

/// An install layout `init` recognises, by the top-level entries each
/// tool's own installer documents. No layout names a dotfile, and none a
/// credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Layout {
    /// A node release's prefix, as its tarball unpacks: an nvm version
    /// directory, an fnm `installation`, a node tarball anywhere. Never
    /// bound whole: its `bin` and `lib/node_modules` are.
    NodePrefix,
    /// `VOLTA_HOME`: the shims, volta's tool images and its own state.
    Volta,
    /// `BUN_INSTALL`: the binary, its global installs and cache, and the
    /// zsh completion its installer writes.
    Bun,
    /// `PNPM_HOME`: the standalone binary, its global bins, store and
    /// global packages, and the node versions `pnpm env` installs.
    PnpmHome,
}

impl Layout {
    fn name(self) -> &'static str {
        match self {
            Layout::NodePrefix => "a node prefix",
            Layout::Volta => "volta's root",
            Layout::Bun => "bun's root",
            Layout::PnpmHome => "pnpm's home",
        }
    }

    fn entries(self) -> &'static [&'static str] {
        match self {
            Layout::NodePrefix => &[
                "bin",
                "include",
                "lib",
                "share",
                "CHANGELOG.md",
                "LICENSE",
                "README.md",
            ],
            Layout::Volta => &[
                "bin",
                "cache",
                "log",
                "tmp",
                "tools",
                "hooks.json",
                "layout.v1",
                "layout.v2",
                "layout.v3",
                "layout.v4",
            ],
            Layout::Bun => &["bin", "install", "_bun"],
            Layout::PnpmHome => &[
                "pnpm", "pnpx", "node", "npm", "npx", "global", "nodejs", "store",
            ],
        }
    }
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
            Reach::Unbindable(binary, root, unbound) => Some(format!(
                "{binary} is installed under {}, {} `brokkr init` will not bind \
                 into verify's hands; install it under a directory of its own, or \
                 fill in .brokkr/scripts/verify-seat.sh",
                root.display(),
                unbound.which()
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

impl Unbound {
    /// The refusal's clause between the root and why `init` will not
    /// bind it.
    fn which(&self) -> String {
        match self {
            Unbound::TooBroad => "which".to_string(),
            Unbound::Foreign(layout, entry) => format!(
                "which holds '{}', no part of {}, and which",
                entry.to_string_lossy(),
                layout.name()
            ),
            Unbound::Unreadable => "whose entries could not be read, and which".to_string(),
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
        if let Err((root, unbound)) = found.add(install, &home) {
            return Some(Reach::Unbindable(binary, root, unbound));
        }
    }
    Some(Reach::Found {
        binds: found.spelled(&home),
        path: found.path,
        volta: found.volta,
    })
}

/// One binary as `PATH` holds it: the directory it was found in, the
/// entry there of its name, and the file that entry resolves to, the
/// directory and file canonical.
struct Install {
    dir: PathBuf,
    entry: PathBuf,
    real: PathBuf,
}

/// The first `binary` on `path` that resolves to a file this user may
/// execute, as `PATH` lookup skips one that is not.
fn install(binary: &str, path: &OsStr) -> Option<Install> {
    std::env::split_paths(path).find_map(|dir| {
        let dir = dir.canonicalize().ok()?;
        let entry = dir.join(binary);
        let real = entry.canonicalize().ok()?;
        let runs = real.is_file() & runnable(&real);
        runs.then_some(Install { dir, entry, real })
    })
}

/// Whether this process may execute `path`, asked of the kernel with its
/// effective identity: `faccessat(AT_EACCESS)`, the check `execve` makes.
/// `mode & 0o111` asked whether anyone may, so an owner's mode-0641 file
/// shadowed a later install `PATH` lookup reaches. brokkr-protocol's
/// `adapters/composite.rs` asks the same question in
/// `effective_exec_access`, which is private to that module: sharing it
/// means exporting it through `adapters.rs`, which the operator holds
/// (2026-10-23, "keep the hold, go with node.rs").
fn runnable(path: &Path) -> bool {
    use rustix::fs::{accessat, Access, AtFlags, CWD};
    accessat(CWD, path, Access::EXEC_OK, AtFlags::EACCESS).is_ok()
}

/// What the found binaries add up to, in the order they were found.
#[derive(Default)]
struct Found {
    binds: Vec<PathBuf>,
    path: Vec<PathBuf>,
    volta: Option<PathBuf>,
}

impl Found {
    /// Add one install: its directory to `PATH`, and what it binds unless
    /// the box reads it already. `Err` is a root `init` will not bind, and
    /// why.
    fn add(&mut self, install: Install, home: &Path) -> Result<(), (PathBuf, Unbound)> {
        let (root, layout) = root(&install);
        if layout == Layout::Volta {
            self.volta = Some(root.clone());
        }
        if !self.path.contains(&install.dir) {
            self.path.push(install.dir.clone());
        }
        let paths = [install.dir.as_path(), install.real.as_path()];
        if paths.into_iter().all(read_by_every_box) {
            return Ok(());
        }
        // A file standing in its `PATH` directory is bound alone, and a
        // volta shim, which runs the images under its root, by its root.
        let standing = install.real.parent() == Some(install.dir.as_path());
        let bound = match standing & (layout != Layout::Volta) {
            true => vec![install.entry],
            false => {
                rooted(&root, layout, &install.dir, home).map_err(|unbound| (root, unbound))?
            }
        };
        for bind in bound {
            if !self.binds.contains(&bind) {
                self.binds.push(bind);
            }
        }
        Ok(())
    }

    /// The binds, in the order found, less any under another, each
    /// `~/`-spelled under `home`.
    fn spelled(&self, home: &Path) -> Vec<String> {
        let under = |bind: &PathBuf, other: &PathBuf| (other != bind) & bind.starts_with(other);
        let outer = |bind: &&PathBuf| !self.binds.iter().any(|other| under(bind, other));
        let spell = |bind: &PathBuf| match bind.strip_prefix(home) {
            Ok(rest) if home.is_absolute() => format!("~/{}", rest.display()),
            _ => bind.display().to_string(),
        };
        self.binds.iter().filter(outer).map(spell).collect()
    }
}

/// What a rooted install binds: its root whole where the root is a
/// tool's own home, and a node prefix's `bin` (the `PATH` directory) and
/// `lib/node_modules`; `Err` where the root is too broad or holds an
/// entry its layout does not name.
fn rooted(root: &Path, layout: Layout, bin: &Path, home: &Path) -> Result<Vec<PathBuf>, Unbound> {
    if too_broad(root, home) {
        return Err(Unbound::TooBroad);
    }
    holds_only(root, layout)?;
    match layout {
        Layout::NodePrefix => prefix(root, bin),
        Layout::Volta | Layout::Bun | Layout::PnpmHome => Ok(vec![root.to_path_buf()]),
    }
}

/// A node prefix's two binds: npm's global package directory,
/// `lib/node_modules`, which must be a directory of the prefix's own,
/// and `bin`, which must hold only a `node` file and links into it (npm,
/// npx, corepack, global packages' bins). The prefix itself is never
/// bound: its `share` or `include` may hold other software's files.
fn prefix(root: &Path, bin: &Path) -> Result<Vec<PathBuf>, Unbound> {
    let modules = root.join("lib/node_modules");
    let own = modules
        .canonicalize()
        .is_ok_and(|real| (real == modules) & real.is_dir());
    if !own {
        return Err(Unbound::Unreadable);
    }
    for name in listed(bin)? {
        let entry = bin.join(&name);
        let node = (name == "node") & entry.symlink_metadata().is_ok_and(|held| held.is_file());
        let linked = entry
            .canonicalize()
            .is_ok_and(|real| real.starts_with(&modules));
        if !(node | linked) {
            let spelled = bin.strip_prefix(root).unwrap_or(bin).join(name);
            return Err(Unbound::Foreign(Layout::NodePrefix, spelled.into()));
        }
    }
    Ok(vec![bin.to_path_buf(), modules])
}

/// The install's root, and the layout it must hold to be bound: the
/// nearest directory holding both the file the binary resolves to and
/// its directory's parent where that directory is a `bin` (a node prefix,
/// volta's or bun's root), or the directory itself where it is not
/// (pnpm's home).
fn root(install: &Install) -> (PathBuf, Layout) {
    let bin = install.dir.file_name() == Some(OsStr::new("bin"));
    let base = if bin {
        install.dir.parent().unwrap_or(&install.dir)
    } else {
        &install.dir
    };
    let holds = |ancestor: &&Path| install.real.starts_with(ancestor);
    let root = base.ancestors().find(holds).unwrap_or(Path::new("/"));
    let layout = match (install.real.file_name().and_then(OsStr::to_str), bin) {
        (Some("volta-shim"), _) => Layout::Volta,
        (_, false) => Layout::PnpmHome,
        (Some("bun"), true) => Layout::Bun,
        (_, true) => Layout::NodePrefix,
    };
    (root.to_path_buf(), layout)
}

/// `Ok` when every entry `root` holds is one `layout` names; else the
/// first, by name, that it does not, or that the entries could not be
/// read.
fn holds_only(root: &Path, layout: Layout) -> Result<(), Unbound> {
    let names = listed(root)?;
    let named = |name: &&OsString| layout.entries().iter().any(|entry| *name == entry);
    match names.iter().find(|name| !named(name)) {
        Some(entry) => Err(Unbound::Foreign(layout, entry.clone())),
        None => Ok(()),
    }
}

/// The names `dir` holds, sorted, or `Unreadable` where they could not be
/// listed.
fn listed(dir: &Path) -> Result<Vec<OsString>, Unbound> {
    let listed = std::fs::read_dir(dir).and_then(|entries| {
        entries
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<Vec<OsString>>>()
    });
    let mut names = listed.map_err(|_| Unbound::Unreadable)?;
    names.sort();
    Ok(names)
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
