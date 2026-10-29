//! Where a workspace keeps what Brokkr reads and writes (decision 0083):
//! an adopter's `.brokkr/` folder, or a recipe library's own layout. This
//! module is the one judgement of a workspace's layout ([`Layout`]): the
//! walk to the repository's root, the nested folder's refusal, the root's
//! `.brokkr` entry, the library mark, the old layouts, the no-place rule
//! and the map's presence. The CLI's verbs, a launch, a queued launch,
//! the engine and the map's default all take it from the same workspace,
//! so none can admit what another refuses (ruling 3).

use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use brokkr_core::realms::DEFAULT_MAP_FILE;
use thiserror::Error;

use crate::bundle::{CompileError, DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};

/// The adopter's folder, at a repository's root (ruling 1).
pub const FOLDER: &str = ".brokkr";

/// The folder's runtime state, ignored by git and never pinned: the name
/// a folder bundle's walk skips beside [`UNPINNED_TOP_LEVEL`] (ruling 1).
pub const STATE: &str = "state";

/// A place a workspace's readers cannot take, refused by name with its
/// path rather than read as the library layout or as no map (decision
/// 0083; the operator's ruling of 2026-09-26: fail closed on what cannot
/// be read).
#[derive(Debug, Error)]
pub enum PlacesError {
    /// A `.brokkr` entry the walk met below the repository's root.
    #[error(
        "{}/{FOLDER} is not at this repository's root {} (decision 0083); Brokkr reads only \
         {}/{FOLDER}: move or remove it",
        .path.display(),
        .root.display(),
        .root.display()
    )]
    Nested { path: PathBuf, root: PathBuf },
    /// A root `.brokkr` that is not a directory, a link included.
    #[error(
        "{}/{FOLDER} is not a directory (decision 0083); Brokkr's folder must be one: move or \
         remove it",
        .root.display()
    )]
    NotADirectory { root: PathBuf },
    /// The default map's place holds something that is not a file.
    #[error(
        "{} is not a file (decision 0083): the default realms map is a file, or a link to one; \
         move or remove it",
        .path.display()
    )]
    MapNotAFile { path: PathBuf },
    /// The default map's place is a link to nothing.
    #[error(
        "{} is a link to nothing (decision 0083 ruling 4: a linked entry is the entry); mend or \
         remove the link",
        .path.display()
    )]
    MapDangling { path: PathBuf },
    /// An entry that cannot be read at all.
    #[error("reading {}: {source}", .path.display())]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A shelf that cannot be read ([`resolving_shelf`], #697): what stands
    /// `at` it, or above it, does not resolve to a directory, for its
    /// `cause`.
    #[error(
        "the {origin} shelf {} cannot be read (#697): {}: {cause}; a shelf that cannot be read \
         is never read as an empty one, so mend or remove it; nothing was run or installed",
        .shelf.display(),
        .at.display()
    )]
    ShelfUnread {
        origin: Origin,
        shelf: PathBuf,
        at: PathBuf,
        cause: std::io::Error,
    },
    /// A shelf spelled through `..` beneath `at`, the nearest entry of it
    /// that stands ([`resolving_shelf`], #726).
    #[error(
        "the {origin} shelf {} climbs by '..' out of a directory that does not stand beneath {} \
         (#726): a shelf spelled through what does not exist is never read as an empty one, so \
         spell it without the '..'; nothing was run or installed",
        .shelf.display(),
        .at.display()
    )]
    ShelfClimbs {
        origin: Origin,
        shelf: PathBuf,
        at: PathBuf,
    },
    /// A state fact with no place: no folder, no library, and no old
    /// layout's own entry for the `--db` escape (ruling 2's step 4).
    #[error(
        "no {FOLDER}/ folder from {} up to {}, and {} is not a recipe library (decision 0083); \
         nothing was created: run `brokkr init .` from {}",
        .cwd.display(),
        .root.display(),
        .root.display(),
        .root.display()
    )]
    NoPlace { cwd: PathBuf, root: PathBuf },
}

/// The refusal of the entry at `path`, which cannot be read.
fn unreadable(path: &Path) -> impl FnOnce(std::io::Error) -> PlacesError + '_ {
    move |source| PlacesError::Unreadable {
        path: path.to_path_buf(),
        source,
    }
}

/// The old root layout's signature, `init`'s bundle, and a library's
/// mark under each `recipes/<name>/`.
pub const BUNDLE: &str = "bundle.json";

/// The named recipes' directory: a library's at its root (decision 0078),
/// and the folder's shelf in `.brokkr/` (#696), which a folder bundle's
/// walk skips beside [`STATE`] and composes its base from.
pub const RECIPES: &str = "recipes";

/// The personal shelf (#697): the operator's own named recipes, `recipes`
/// in the machine's configuration directory beside `host.json`
/// (`$XDG_CONFIG_HOME/brokkr`, else `$HOME/.config/brokkr`), never in a
/// repository, so no library mark is judged there. Placed by the one root
/// `host.json` is, and refused as its lookup is where neither names an
/// absolute directory.
pub fn personal_shelf(
    xdg: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, crate::admission::HostError> {
    Ok(crate::admission::config_dir(xdg, home)?.join(RECIPES))
}

/// [`personal_shelf`] as this process's environment places it: the one
/// read of `XDG_CONFIG_HOME` and `HOME` that the CLI's verbs and every
/// compile's shelves take (#697).
pub fn personal_shelf_here() -> Result<PathBuf, crate::admission::HostError> {
    personal_shelf(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

mod shelves;
pub(crate) use shelves::Shelves;
pub use shelves::{resolving_shelf, ComposedLayer, Named, Origin, ResolvedShelf, ShelfRefusal};

/// A recipe library's run-local directory, beside the operated
/// repository, as it has always been (ruling 3's last column).
pub const LIBRARY_STATE: &str = ".forge";

/// The secrets store's name in the run-local directory, where the engine
/// threads it and where the CLI's secrets verbs default (decision 0012).
pub const SECRETS: &str = "secrets.env";

/// The journal's name in the run-local directory, which a command opens
/// when neither a map nor `--db` names one (ruling 2).
///
/// It is a fallback, not the steady state. Same-realm parallel burns
/// belong in the realm's one `journal` from `realms.json`, which several
/// `brokkr` processes driving different runs can share safely — measured
/// and fenced at the store layer, see `brokkr_store`'s module doc.
/// Reaching for a journal per worktree to keep parallel burns apart is
/// emergency isolation: it buys nothing the shared journal does not
/// already give, and it scatters one realm's history across files that
/// no single reader can fold together.
pub const JOURNAL: &str = "forge.db";

/// The top-level names every bundle's manifest walk skips (decisions 0042
/// and 0065): workspace declarations pinned into the run manifest, and
/// the operator's capability definitions.
pub(crate) const UNPINNED_TOP_LEVEL: [&str; 3] = [
    DEFAULT_MAP_FILE,
    "dialects",
    crate::capabilities::DEFINITIONS_DIR,
];

/// Git's own metadata directory: never bundle bytes, whatever git says
/// about it (#908).
pub(crate) const GIT: &str = ".git";

/// Why the walk of a layer skips a top-level name: one of the fixed
/// answers, or git's own verdict on the tree (#908). A closed set matched
/// exhaustively, so a new skip cannot be silently read as operator
/// configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Skip {
    /// One of [`UNPINNED_TOP_LEVEL`].
    Workspace,
    /// A folder bundle's [`STATE`].
    State,
    /// A folder bundle's shelf, [`RECIPES`].
    Shelf,
    /// [`GIT`].
    GitMetadata,
    /// A tree git reports ignored for the bundle's repository.
    GitIgnored,
}

/// A git question the bundle's walk could not get answered (#908):
/// refused by name rather than read as "pin everything" or "skip
/// everything".
#[derive(Debug, Error)]
pub(crate) enum SkipError {
    #[error(
        "asking git which trees the bundle at {} ignores failed ({detail}); a bundle's walk \
         skips a git-ignored tree only when git says so, never by guessing",
        .bundle.display()
    )]
    Git { bundle: PathBuf, detail: String },
}

/// Why the walk of the layer at `dir`, its canonical directory, skips the
/// top-level `name`, or `None` when it pins it. [`Skip`] has one home
/// here: one of [`UNPINNED_TOP_LEVEL`], a folder bundle's [`STATE`]
/// (decision 0083) or shelf [`RECIPES`] (#696), git's metadata [`GIT`],
/// or a tree git reports ignored for the bundle's repository (#908).
///
/// A scaffold may also be the workspace brokkr is invoked from (decision
/// 0042): its realm map and dialect library are workspace declarations
/// pinned into the RUN manifest, never bundle files, and an unconsulted
/// capability definition is no second source of bundle identity (decision
/// 0065). A bundle outside any git work tree skips only [`GIT`], and a
/// layer whose layout cannot be read pins its `state/` and `recipes/` as
/// every other bundle always did: pinning more never lets a file change
/// unseen. The bundle's file walk, its pinned-script lookup and its skip
/// reason all ask this one function, so they cannot drift.
pub(crate) fn unpinned_reason(dir: &Path, name: &str) -> Result<Option<Skip>, SkipError> {
    if [STATE, RECIPES].contains(&name) && matches!(folder_bundle(dir), Ok(Some(_))) {
        return Ok(Some(match name {
            STATE => Skip::State,
            _ => Skip::Shelf,
        }));
    }
    if UNPINNED_TOP_LEVEL.contains(&name) {
        return Ok(Some(Skip::Workspace));
    }
    if name == GIT {
        return Ok(Some(Skip::GitMetadata));
    }
    Ok(git_ignored(dir, name)?.then_some(Skip::GitIgnored))
}

/// Whether the walk skips the top-level `name` ([`unpinned_reason`]).
pub(crate) fn unpinned_top_level(dir: &Path, name: &str) -> Result<bool, SkipError> {
    Ok(unpinned_reason(dir, name)?.is_some())
}

/// What a skipped top-level name holds, in the words of the refusal of a
/// path under it.
pub(crate) fn unpinned_holds(skip: Skip) -> &'static str {
    match skip {
        Skip::Workspace => "operator configuration",
        Skip::State => "the folder's runtime state, which a run writes",
        Skip::Shelf => "the folder's shelf, whose named recipes are bundles of their own (#696)",
        Skip::GitMetadata => "git's own metadata, which no bundle pins",
        Skip::GitIgnored => {
            "a tree git ignores in the bundle's repository, whose bytes are not part of the \
             bundle's identity"
        }
    }
}

/// Git's own answer to whether the top-level `name` under `dir` is
/// ignored (#908): `git check-ignore` for the repository the bundle
/// stands in, so `.gitignore`, `.git/info/exclude` and a global exclude
/// rule alike are git's rules, never a copy of them.
///
/// A bundle outside a git work tree answers "not ignored": it skips only
/// [`GIT`]. A git that cannot be asked refuses by name; it is never read
/// as "nothing is ignored" or "everything is".
fn git_ignored(dir: &Path, name: &str) -> Result<bool, SkipError> {
    // A directory that is no longer there has no repository to ask: the
    // read that follows names it missing, as it always did, rather than
    // asking git about a bundle that does not stand.
    if !dir.exists() {
        return Ok(false);
    }
    // Run git in the bundle, as the house's other git calls do. A bundle
    // whose directory cannot be entered is a spawn failure, refused by
    // name rather than read as "nothing is ignored".
    let run = |args: &[&str]| -> Result<std::process::Output, SkipError> {
        std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(|error| SkipError::Git {
                bundle: dir.to_path_buf(),
                detail: error.to_string(),
            })
    };
    // Git's own test for the question, and the one way to tell "not in a
    // work tree" from a repository that could not be read: a non-zero
    // answer here is git saying the bundle stands in no work tree, which
    // is the case the rule pins as it always did.
    let inside = run(&["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() || String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Ok(false);
    }
    let checked = run(&["check-ignore", "--quiet", "--", name])?;
    match checked.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(SkipError::Git {
            bundle: dir.to_path_buf(),
            detail: format!("git check-ignore answered {}", checked.status),
        }),
    }
}

/// The repository root of a folder bundle, by the one judgement of a
/// layout (ruling 1): `dir`, a layer's canonical directory, is
/// `<root>/.brokkr`, where `<root>` is its repository's root ([`root`])
/// and holds the folder ([`Kind::Folder`]). Every other directory named
/// `.brokkr`, a library's recipe or one below the root, is an ordinary
/// bundle, never one by its name alone. `recipes add` asks it of every
/// recipe it plans, so it never copies a folder bundle as one (#696).
pub fn folder_bundle(dir: &Path) -> Result<Option<&Path>, PlacesError> {
    let named = dir.file_name() == Some(FOLDER.as_ref());
    let Some(parent) = dir.parent().filter(|_| named) else {
        return Ok(None);
    };
    let folder = root(parent)? == parent && Kind::of(parent)? == Kind::Folder;
    Ok(folder.then_some(parent))
}

/// The agent library, the adapters, the named recipes and the default map
/// a workspace's launches read. Where their run-local state stands is
/// [`Layout::state`]'s to judge, because a workspace with no place for it
/// refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    pub agents: PathBuf,
    pub adapters: PathBuf,
    /// The named recipes: the folder's shelf, `.brokkr/recipes` (#696),
    /// else the library's `recipes`.
    pub recipes: PathBuf,
    /// The map a workspace adopts when none is named, if it is there.
    pub(crate) map: PathBuf,
}

impl Places {
    /// The places of `workspace`, by the walk every reader takes
    /// ([`Layout::discover`]): the folder's at the root the walk reaches,
    /// else the library layout relative to `workspace`. A nested
    /// `.brokkr`, and a root `.brokkr` that is not a directory, refuse,
    /// never falling back to the library layout.
    pub fn of(workspace: &Path) -> Result<Places, PlacesError> {
        Ok(Layout::discover(workspace)?.places())
    }

    /// The places under `dir`: a folder, or a library's workspace.
    fn under(dir: &Path) -> Places {
        Places {
            agents: dir.join(DEFAULT_AGENTS_DIR),
            adapters: dir.join(DEFAULT_ADAPTERS_DIR),
            recipes: dir.join(RECIPES),
            map: dir.join(DEFAULT_MAP_FILE),
        }
    }
}

/// A workspace's layout (ruling 2): where the walk from it began, the
/// repository root the walk reached, the workspace as it was handed, and
/// what the root holds. The one judgement the CLI's verbs and every
/// dispatch share, so a queued launch is never admitted where `brokkr
/// run` refuses.
#[derive(Debug, Clone)]
pub struct Layout {
    cwd: PathBuf,
    root: PathBuf,
    workspace: PathBuf,
    kind: Kind,
}

impl Layout {
    /// Walk up from `workspace` to the repository root ([`root`]); refuse
    /// a `.brokkr` below the root, and a root `.brokkr` that is not a
    /// directory, before anything is opened. The walk is over the real
    /// directory's ancestors, every link resolved, as the CLI's working
    /// directory names them: a queued workspace that has since become a
    /// link is judged where it now stands, never along the chain its old
    /// name spells. A workspace that cannot be resolved refuses by name.
    pub fn discover(workspace: &Path) -> Result<Layout, PlacesError> {
        let cwd = std::path::absolute(workspace)
            .and_then(std::fs::canonicalize)
            .map_err(unreadable(workspace))?;
        let root = root(&cwd)?;
        for dir in cwd.ancestors().take_while(|dir| *dir != root) {
            if stands(dir, FOLDER)? {
                let path = dir.to_path_buf();
                return Err(PlacesError::Nested { path, root });
            }
        }
        Layout::at(cwd, root, workspace)
    }

    fn at(cwd: PathBuf, root: PathBuf, workspace: &Path) -> Result<Layout, PlacesError> {
        // A library outranks a folder beside it, so its journal never
        // moves to a fresh one under the folder.
        let kind = Kind::of(&root)?;
        Ok(Layout {
            cwd,
            root,
            workspace: workspace.to_path_buf(),
            kind,
        })
    }

    /// What the root holds.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The real directory the walk began in, every link resolved.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// The repository root the walk reached.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The tree a workspace's places resolve against: the root, where the
    /// folder is, else the workspace as handed, which a library's facts
    /// still resolve against exactly as they always did.
    pub fn workspace(&self) -> PathBuf {
        match self.kind {
            Kind::Folder => self.root.clone(),
            Kind::Library | Kind::Bare => self.workspace.clone(),
        }
    }

    /// The agent library, the adapters and the default map: the folder's,
    /// else the library layout relative to the workspace as handed.
    pub fn places(&self) -> Places {
        match self.kind {
            Kind::Folder => Places::under(&self.root.join(FOLDER)),
            Kind::Library | Kind::Bare => Places::under(&self.workspace),
        }
    }

    /// The run-local state directory, by `Kind::state`'s one rule.
    /// `escape` is a journal named outright, `--db` or a dispatch's.
    pub fn state(&self, escape: bool) -> Result<Option<PathBuf>, PlacesError> {
        self.kind.state(&self.cwd, &self.root, escape)
    }

    /// The old shape the root holds, if any (ruling 4). A recipe library
    /// keeps its layout and holds none (the operator's ruling 1 of
    /// 2026-10-13).
    pub fn old(&self) -> Result<Option<Old>, PlacesError> {
        let bundle = stands(&self.root, BUNDLE)?;
        let forge = stands(&self.root, LIBRARY_STATE)?;
        Ok(match self.kind {
            Kind::Library => None,
            Kind::Bare if bundle => Some(Old::Root),
            Kind::Bare if forge => Some(Old::StateOnly),
            Kind::Bare => None,
            Kind::Folder => self.half_moved(bundle, forge)?,
        })
    }

    /// The half-moved shape: each Brokkr entry left beside the folder.
    fn half_moved(&self, bundle: bool, forge: bool) -> Result<Option<Old>, PlacesError> {
        let left: Vec<PathBuf> = [(bundle, BUNDLE), (forge, LIBRARY_STATE)]
            .into_iter()
            .filter(|(stands, _)| *stands)
            .map(|(_, name)| self.root.join(name))
            .collect();
        let both_journals = stands(&self.root, &format!("{LIBRARY_STATE}/{JOURNAL}"))?
            && stands(&self.root, &format!("{FOLDER}/{STATE}/{JOURNAL}"))?;
        Ok((!left.is_empty()).then_some(Old::HalfMoved {
            left,
            both_journals,
        }))
    }
}

/// The old shapes of ruling 4 a root can hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Old {
    /// No folder, and a root `bundle.json`: `init`'s old output.
    Root,
    /// No folder, no root `bundle.json`, and a root `.forge` entry.
    StateOnly,
    /// A folder, and Brokkr entries still at its root: each one left, and
    /// whether an entry stands at the journal's name both in `.forge/` and
    /// in the folder's state, judged as the entry itself, so a link there
    /// counts whatever it names, a link to nothing included.
    HalfMoved {
        left: Vec<PathBuf>,
        both_journals: bool,
    },
}

/// What a repository root holds, as ruling 2 tells it: the one judgement
/// the CLI's discovery and every re-derivation at dispatch share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `<root>/.brokkr` is a directory, and the root is no library.
    Folder,
    /// A `recipes/<name>/bundle.json` at the root, which keeps its own
    /// layout whatever else stands there (the operator's ruling 1).
    Library,
    /// Neither.
    Bare,
}

impl Kind {
    /// What `root` holds. A root `.brokkr` that is not a directory, and a
    /// `.brokkr` or a library mark that cannot be read, refuse by name.
    fn of(root: &Path) -> Result<Kind, PlacesError> {
        Ok(match folder(root)? {
            Some(_) => Kind::Folder,
            None if library(root)? => Kind::Library,
            None => Kind::Bare,
        })
    }

    /// The run-local state directory of `root`, which holds `self`: the
    /// folder's `state/`, `None` for a library's own `.forge/`, or no
    /// place at all (ruling 2's step 4). `escape` is a journal named
    /// outright, `--db` or a dispatch's: with no folder and no library,
    /// the state then takes the library column where an old layout's own
    /// entry stands at the root, and nowhere else. `cwd` is where the
    /// walk to `root` began, for the refusal's words.
    fn state(self, cwd: &Path, root: &Path, escape: bool) -> Result<Option<PathBuf>, PlacesError> {
        match self {
            Kind::Folder => Ok(Some(root.join(FOLDER).join(STATE))),
            Kind::Library => Ok(None),
            Kind::Bare if escape && holds_old(root)? => Ok(None),
            Kind::Bare => Err(PlacesError::NoPlace {
                cwd: cwd.to_path_buf(),
                root: root.to_path_buf(),
            }),
        }
    }
}

/// Whether `root` holds a root layout's `bundle.json` or a state-only
/// layout's `.forge`, link or not: what the `--db` escape reads as that
/// layout's own.
fn holds_old(root: &Path) -> Result<bool, PlacesError> {
    Ok(stands(root, BUNDLE)? || stands(root, LIBRARY_STATE)?)
}

/// Whether anything stands at `name` under `dir`, link or not. An entry
/// that cannot be read refuses, never taken as absent.
fn stands(dir: &Path, name: &str) -> Result<bool, PlacesError> {
    let path = dir.join(name);
    let found = entry(&path).map_err(unreadable(&path))?;
    Ok(found.is_some())
}

/// The repository root above `cwd`, an absolute path: the first directory
/// holding a `.git` entry, or `cwd` itself outside any repository. An
/// entry the walk cannot read refuses, never taken as absent. The walk is
/// over the path as it is named, which [`Layout::discover`] resolves
/// first.
pub fn root(cwd: &Path) -> Result<PathBuf, PlacesError> {
    for dir in cwd.ancestors() {
        let git = dir.join(".git");
        if entry(&git).map_err(unreadable(&git))?.is_some() {
            return Ok(dir.to_path_buf());
        }
    }
    Ok(cwd.to_path_buf())
}

/// The run-local state a launch from `workspace` writes in, judged again
/// at dispatch by the CLI's own walk ([`Layout::discover`]) and rule
/// ([`Layout::state`]): a `.brokkr` made below the root since the launch
/// was queued refuses as `brokkr run` would; the journal a launch is
/// handed is always named, so the escape is taken; and a workspace whose
/// folder has gone since it was queued, with no library and no old layout
/// standing, refuses rather than writing a `.forge/` into a bare
/// repository.
pub fn state_of(workspace: &Path) -> Result<Option<PathBuf>, PlacesError> {
    Layout::discover(workspace)?.state(true)
}

/// The one judgement of a root's `.brokkr` entry, which every walk's
/// [`Kind`] takes: the folder
/// when the entry itself, never read through a link, is a directory and
/// `workspace` is no recipe library (a library keeps its own layout, a
/// `.brokkr` beside it included: the operator's ruling 1 of 2026-10-13 on
/// #634); `None` when nothing stands there; and a refusal for anything
/// else, or an entry that cannot be read.
fn folder(workspace: &Path) -> Result<Option<PathBuf>, PlacesError> {
    let folder = workspace.join(FOLDER);
    match entry(&folder).map_err(unreadable(&folder))? {
        Some(found) if !found.is_dir() => Err(PlacesError::NotADirectory {
            root: workspace.to_path_buf(),
        }),
        Some(_) if library(workspace)? => Ok(None),
        found => Ok(found.map(|_| folder)),
    }
}

/// Whether an error says nothing stands at a path: nothing does, or
/// nothing can because a parent is a file.
fn absent(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory)
}

/// What stands at `path`, judged by the entry itself and never through a
/// link: `None` when nothing does, or nothing can because a parent is a
/// file.
pub fn entry(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(found) => Ok(Some(found)),
        Err(error) if absent(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The map `workspace` adopts when none is named, if one is there: the
/// one presence rule for a launch, a queued launch and every read of the
/// default (decision 0083 ruling 2). Nothing at its place is no map; a
/// regular file, or a link that resolves to one ([`resolved`]), is the
/// map; anything else refuses with its path.
pub(crate) fn default_map(workspace: &Path) -> Result<Option<PathBuf>, PlacesError> {
    let map = Places::of(workspace)?.map;
    match resolved(&map) {
        Ok(None) => Ok(None),
        Ok(Some(target)) if target.is_file() => Ok(Some(map)),
        Ok(Some(_)) => Err(PlacesError::MapNotAFile { path: map }),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            Err(PlacesError::MapDangling { path: map })
        }
        Err(source) => Err(PlacesError::Unreadable { path: map, source }),
    }
}

/// Where the entry at `path` resolves (ruling 4: a linked entry is the
/// entry): `None` when nothing stands there, else the real path it, or
/// the end of its links, names. A link to nothing fails with `NotFound`,
/// and a looping link, or a path through a file, with its own error, so
/// none is read as absent: the one absent-versus-unreadable judgement the
/// default map ([`default_map`]) and a recipe shelf (#697) take. The path
/// is rebuilt from its components first, so the entry judged is the entry
/// itself: a trailing separator would read through a link to its target,
/// and a link to nothing would read as nothing there.
pub fn resolved(path: &Path) -> std::io::Result<Option<PathBuf>> {
    let entry: PathBuf = path.components().collect();
    match std::fs::symlink_metadata(&entry) {
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        _ => std::fs::canonicalize(&entry).map(Some),
    }
}

/// A recipe library's one mark (ruling 4): a `recipes/<name>/bundle.json`
/// at `root`, because a library's bundles live under `recipes/` (decision
/// 0078) and `init` never wrote one. Nothing at `recipes` is no library;
/// a `recipes/` that cannot be listed, an entry of it that cannot be
/// read, or a `bundle.json` whose presence cannot be read refuses by name
/// rather than choosing a layout (the operator's ruling of 2026-09-26:
/// fail closed on what cannot be read).
fn library(root: &Path) -> Result<bool, PlacesError> {
    let recipes = root.join(RECIPES);
    let listed = match std::fs::read_dir(&recipes) {
        Err(error) if absent(&error) => return Ok(false),
        listed => listed.map_err(unreadable(&recipes))?,
    };
    for recipe in listed {
        let bundle = recipe.map_err(unreadable(&recipes))?.path().join(BUNDLE);
        if marks(&bundle)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether a library's mark stands at `path`: a file, or a link that
/// resolves to one (ruling 4: a linked entry is the entry). Nothing there
/// is no mark; a link to nothing, or an entry that cannot be read,
/// refuses with its path.
fn marks(path: &Path) -> Result<bool, PlacesError> {
    match std::fs::metadata(path) {
        Ok(found) => Ok(found.is_file()),
        Err(error) if absent(&error) && entry(path).map_err(unreadable(path))?.is_none() => {
            Ok(false)
        }
        Err(source) => Err(unreadable(path)(source)),
    }
}

/// The directory a run's results, scratch and secrets stand in: the
/// folder's `state/` when it was handed one, else `workdir/.forge`.
pub fn run_local(state: Option<&Path>, workdir: &Path) -> PathBuf {
    state.map_or_else(|| workdir.join(LIBRARY_STATE), Path::to_path_buf)
}

/// The library the layer at `dir`, its canonical directory, composes its
/// base from (decision 0017): a folder bundle's ([`folder_bundle`]) is
/// its shelf, `.brokkr/recipes`, never the repository root, a directory
/// the project owns (#696, amending decision 0083 ruling 1); every other
/// layer's is its parent, what a recipe library already is. Refused
/// before any base is read where the layer's layout cannot be read. `pop`
/// is total: a directory holding a bundle.json is never the filesystem
/// root, and if it somehow were, the base is "not a recipe" in it anyway.
pub(crate) fn base_library(dir: &Path) -> Result<PathBuf, CompileError> {
    let judged = folder_bundle(dir).map_err(|refused| CompileError::Invalid(refused.to_string()));
    let mut library = dir.to_path_buf();
    match judged? {
        Some(_) => library.push(RECIPES),
        None => {
            library.pop();
        }
    }
    Ok(library)
}

#[cfg(test)]
pub(crate) mod places_tests;
