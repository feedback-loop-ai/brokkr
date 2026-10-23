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
//! name, followed through links, that this user may execute. Nothing is
//! bound that holds more than the install (#717), and no root is found by
//! climbing to an ancestor. A tool-managed home is one only where the tool
//! itself names it ([`Homes`]): `$VOLTA_HOME` or `~/.volta`, `$BUN_INSTALL`
//! or `~/.bun`, `$PNPM_HOME` or pnpm's default data directory. It is bound
//! whole where `PATH` found the binary in the home's own runner directory
//! (its `bin`, or pnpm's home itself), the real file stands inside it, and
//! every entry it holds is one its [`Layout`] names; a volta shim
//! (`~/.volta/bin/npm` linking to `~/.volta/bin/volta-shim`, which runs the
//! images under `~/.volta/tools`) is bound so even standing in its `bin`.
//! Otherwise a binary whose real file stands in its `PATH` directory is
//! bound alone, as that one file; one found in a `bin` whose parent holds
//! its real file is a node prefix, which cannot be told from a prefix other
//! software shares, and is bound only by its `bin` and `lib/node_modules`;
//! and anything else is refused, naming the file it resolves to. A binary
//! whose directory and file both stand under a toolchain root every box
//! already reads read-only (`HOST_TOOLCHAIN_BINDS`) adds no bind. A root
//! that would bind `$HOME`, an ancestor of it, `~/.local` whole or `/` is
//! refused before its entries are read, even where a tool's variable names
//! it, and so is a binary not found, or one whose path the verifier's
//! `PATH` or `bundle.json` cannot carry exactly: each way verify fails
//! closed with the sentence `init` printed.
//!
//! The residual, at the operator's bar ("a tool-managed home is the
//! install; its named entries' contents are the tool's own"): only a
//! home's top-level names are read, so a home the tool's own variable
//! names is trusted below them. `BUN_INSTALL=/srv/t`, where `/srv/t` holds
//! only `bin` and `install`, binds `/srv/t` whole, whatever `install`
//! holds.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

use brokkr_protocol::hands::HOST_TOOLCHAIN_BINDS;

use super::{shell_literal, Tool};

/// The node-family runners the detection tables can lead a command with,
/// and whether each runs on the `node` it finds on `PATH`: npm, npx,
/// pnpm and yarn are node programs, and bun is its own runtime.
const FAMILY: &[(&str, bool)] = &[
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
    /// The binary resolves to this file, outside the home or prefix it
    /// was found in.
    Outside(PathBuf),
    /// The binary resolves to this file, and the `PATH` directory it was
    /// found in is no home its tool names, nor a node prefix's `bin`.
    Unowned(PathBuf),
    /// The path cannot be written into the verifier's `PATH` or
    /// `bundle.json` exactly.
    Uncarried(Flaw),
}

/// What keeps a resolved path from being written exactly where `init`
/// writes it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Flaw {
    /// A `:` in a directory exported on `PATH`, which splits it there.
    Colon,
    /// A newline, which the verifier's lines cannot carry.
    Newline,
    /// A byte that is not UTF-8, which `bundle.json` cannot carry.
    NotUtf8,
}

/// An install layout `init` recognises, by the top-level entries each
/// tool's own installer documents. No layout names a dotfile, and none a
/// credential. A tool's home is held to its layout only where [`Homes`]
/// identifies it, never by the shape of a directory.
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
            Unbound::Outside(real) => {
                format!("which it resolves out of, to {}, and which", real.display())
            }
            Unbound::Unowned(real) => format!(
                "where it resolves to {}, in no home its tool names and no node prefix, \
                 and which",
                real.display()
            ),
            Unbound::Uncarried(Flaw::Colon) => {
                "whose ':' would split the verifier's PATH, and which".to_string()
            }
            Unbound::Uncarried(Flaw::Newline) => {
                "whose newline the verifier's lines cannot carry, and which".to_string()
            }
            Unbound::Uncarried(Flaw::NotUtf8) => {
                "which is not UTF-8, as bundle.json must be, and which".to_string()
            }
        }
    }
}

/// The homes the tools themselves name, canonical, beside the operator's
/// `$HOME`.
pub(super) struct Homes {
    home: PathBuf,
    tools: Vec<(Layout, PathBuf)>,
}

impl Homes {
    /// Each tool's home as the tool names it, read from `var`, `init`'s
    /// environment, and `os`: `$VOLTA_HOME`, else `~/.volta`;
    /// `$BUN_INSTALL`, else `~/.bun`; `$PNPM_HOME`, else on macOS
    /// `~/Library/pnpm`, and elsewhere `$XDG_DATA_HOME/pnpm` or
    /// `~/.local/share/pnpm`. An empty variable is unset; one naming a
    /// relative path names no home, nor does a home that does not exist.
    pub(super) fn named(home: &Path, os: &str, var: impl Fn(&str) -> Option<OsString>) -> Homes {
        let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
        let under = |rest: &str| home.is_absolute().then(|| home.join(rest));
        let named = |name: &str, default: Option<PathBuf>| match var(name)
            .filter(|value| !value.is_empty())
        {
            Some(set) => Some(PathBuf::from(set)).filter(|set| set.is_absolute()),
            None => default,
        };
        let pnpm = match os == "macos" {
            true => under("Library/pnpm"),
            false => named("XDG_DATA_HOME", under(".local/share")).map(|data| data.join("pnpm")),
        };
        let named = [
            (Layout::Volta, named("VOLTA_HOME", under(".volta"))),
            (Layout::Bun, named("BUN_INSTALL", under(".bun"))),
            (Layout::PnpmHome, named("PNPM_HOME", pnpm)),
        ];
        let real =
            |(layout, root): (Layout, Option<PathBuf>)| Some((layout, root?.canonicalize().ok()?));
        let tools = named.into_iter().filter_map(real).collect();
        Homes { home, tools }
    }

    /// The home whose own runner directory is `dir`: `bin` under volta's
    /// and bun's, pnpm's home itself.
    fn serving(&self, dir: &Path) -> Option<(Layout, PathBuf)> {
        let runners = |layout: Layout, root: &Path| match layout {
            Layout::PnpmHome => root.to_path_buf(),
            Layout::Volta | Layout::Bun | Layout::NodePrefix => root.join("bin"),
        };
        let serves = |(layout, root): &&(Layout, PathBuf)| runners(*layout, root) == dir;
        self.tools.iter().find(serves).cloned()
    }
}

/// Resolve the node-family binaries `runners` names, and `node` where one
/// of them runs on it, against `path`, with `homes` the operator's `$HOME`
/// and the tools' own. `None` when no runner is node-family.
pub(super) fn reach(runners: &[Tool], path: &OsStr, homes: &Homes) -> Option<Reach> {
    let named = |name: &&str| runners.iter().any(|tool| tool.name == *name);
    let mut wanted: Vec<&'static str> =
        FAMILY.iter().map(|(name, _)| *name).filter(named).collect();
    if wanted.is_empty() {
        return None;
    }
    if FAMILY.iter().any(|(name, node)| *node && named(name)) {
        wanted.push("node");
    }
    let mut found = Found::default();
    for binary in wanted {
        let Some(install) = install(binary, path) else {
            return Some(Reach::Missing(binary));
        };
        if let Err((root, unbound)) = found.add(install, homes) {
            return Some(Reach::Unbindable(binary, root, unbound));
        }
    }
    Some(Reach::Found {
        binds: found.spelled(&homes.home),
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
    /// the box reads it already. `Err` is a root or path `init` will not
    /// bind, and why.
    fn add(&mut self, install: Install, homes: &Homes) -> Result<(), (PathBuf, Unbound)> {
        // Every bind is the `PATH` directory, an entry in it, or a root
        // above it (with `lib/node_modules`), so a newline or a byte not
        // UTF-8 any bind holds, the directory holds first, and binds are
        // not checked again: a_path_the_verifier_cannot_carry_is_refused
        // plants the flaw in a root's own name.
        carried(&install.dir)?;
        if !self.path.contains(&install.dir) {
            self.path.push(install.dir.clone());
        }
        let owner = owner(&install, homes);
        if let Owner::Rooted(Layout::Volta, root) = &owner {
            self.volta = Some(root.clone());
        }
        let paths = [install.dir.as_path(), install.real.as_path()];
        if paths.into_iter().all(read_by_every_box) {
            return Ok(());
        }
        for bind in owner.binds(install, &homes.home)? {
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

/// Whose an install is, by where `PATH` found it.
enum Owner {
    /// A root of this layout: a home its tool names, whose runner
    /// directory `PATH` found it in, or the node prefix above the `bin` it
    /// was found in.
    Rooted(Layout, PathBuf),
    /// No root: its real file stands in the `PATH` directory.
    Standing,
    /// None of these.
    Unowned,
}

/// Whose `install` is. A volta shim is its home's even standing in the
/// home's `bin`, as it runs the images under the home; any other file
/// standing in its `PATH` directory is its own; then a home whose runner
/// directory found it, and a `bin`'s parent. No ancestor is climbed to.
fn owner(install: &Install, homes: &Homes) -> Owner {
    let standing = install.real.parent() == Some(install.dir.as_path());
    let bin = install.dir.file_name() == Some(OsStr::new("bin"));
    match (homes.serving(&install.dir), standing) {
        (Some((Layout::Volta, root)), _) => Owner::Rooted(Layout::Volta, root),
        (_, true) => Owner::Standing,
        (Some((layout, root)), false) => Owner::Rooted(layout, root),
        (None, false) => match install.dir.parent().filter(|_| bin) {
            Some(prefix) => Owner::Rooted(Layout::NodePrefix, prefix.to_path_buf()),
            None => Owner::Unowned,
        },
    }
}

impl Owner {
    /// What the install binds: the entry alone where it stands, else its
    /// root as [`rooted`] rules; `Err` naming the root, or the `PATH`
    /// directory where there is none, and why it is not bound.
    fn binds(self, install: Install, home: &Path) -> Result<Vec<PathBuf>, (PathBuf, Unbound)> {
        match self {
            Owner::Standing => Ok(vec![install.entry]),
            Owner::Rooted(layout, root) => {
                rooted(&root, layout, &install, home).map_err(|unbound| (root, unbound))
            }
            Owner::Unowned => Err((install.dir, Unbound::Unowned(install.real))),
        }
    }
}

/// What a rooted install binds: its root whole where the root is a
/// tool's own home, and a node prefix's `bin` (the `PATH` directory) and
/// `lib/node_modules`; `Err` where the root is too broad, does not hold
/// the real file, or holds an entry its layout does not name.
fn rooted(
    root: &Path,
    layout: Layout,
    install: &Install,
    home: &Path,
) -> Result<Vec<PathBuf>, Unbound> {
    if too_broad(root, home) {
        return Err(Unbound::TooBroad);
    }
    if !install.real.starts_with(root) {
        return Err(Unbound::Outside(install.real.clone()));
    }
    holds_only(root, layout)?;
    match layout {
        Layout::NodePrefix => prefix(root, &install.dir),
        Layout::Volta | Layout::Bun | Layout::PnpmHome => Ok(vec![root.to_path_buf()]),
    }
}

/// `Ok` where `dir`, a directory the verifier exports on `PATH`, is
/// written exactly into its lines and `bundle.json`; else `Err` naming it
/// and its flaw.
fn carried(dir: &Path) -> Result<(), (PathBuf, Unbound)> {
    let flaw = match dir.to_str() {
        None => Some(Flaw::NotUtf8),
        Some(text) if text.contains('\n') => Some(Flaw::Newline),
        Some(text) if text.contains(':') => Some(Flaw::Colon),
        Some(_) => None,
    };
    match flaw {
        Some(flaw) => Err((dir.to_path_buf(), Unbound::Uncarried(flaw))),
        None => Ok(()),
    }
}

/// A node prefix's two binds: npm's global package directory,
/// `lib/node_modules`, which must be a directory of the prefix's own,
/// and `bin`, which must hold only a `node` file and links into it (npm,
/// npx, corepack, global packages' bins) that the box, binding those two
/// alone, resolves as the host does. The prefix itself is never bound:
/// its `share` or `include` may hold other software's files.
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
            .is_ok_and(|real| real.starts_with(&modules))
            & resolves_within(&entry, [bin, &modules]);
        if !(node | linked) {
            let spelled = bin.strip_prefix(root).unwrap_or(bin).join(name);
            return Err(Unbound::Foreign(Layout::NodePrefix, spelled.into()));
        }
    }
    Ok(vec![bin.to_path_buf(), modules])
}

/// The links the kernel follows before it gives up (Linux's
/// `MAXSYMLINKS`); a chain longer, or a cycle, is not followed.
const LINKS: usize = 40;

/// Whether `entry`, absolute, resolves as the kernel walks it, a component
/// and a link at a time, through nothing but `binds` and the directories
/// above them, the only paths a box binding those alone holds. A chain
/// whose final file lies under `binds` may pass through a link elsewhere,
/// another `PATH` directory's, which the box does not hold (#717).
fn resolves_within(entry: &Path, binds: [&Path; 2]) -> bool {
    let held = |at: &Path| {
        binds
            .iter()
            .any(|bind| at.starts_with(bind) | bind.starts_with(at))
    };
    let steps = |path: &Path| -> Vec<PathBuf> {
        let parts = path.components().filter(|part| *part != Component::CurDir);
        parts
            .rev()
            .map(|part| PathBuf::from(part.as_os_str()))
            .collect()
    };
    let mut rest = steps(entry);
    let (mut at, mut links) = (PathBuf::new(), 0);
    while let Some(step) = rest.pop() {
        if step.as_os_str() == ".." {
            at.pop();
            continue;
        }
        at.push(step);
        if !held(&at) | (links > LINKS) {
            return false;
        }
        if let Ok(text) = at.read_link() {
            links += 1;
            at.pop();
            rest.extend(steps(&text));
        }
    }
    true
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
