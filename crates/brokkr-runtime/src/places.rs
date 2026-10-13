//! Where a workspace keeps what Brokkr reads and writes (decision 0083):
//! an adopter's `.brokkr/` folder, or a recipe library's own layout. This
//! module is the one judgement of a workspace's layout ([`Layout`]): the
//! walk to the repository's root, the nested folder's refusal, the root's
//! `.brokkr` entry, the library mark, the old layouts, the no-place rule
//! and the map's presence. The CLI's verbs, a launch, a queued launch,
//! the engine and the map's default all take it from the same workspace,
//! so none can admit what another refuses (ruling 3).

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

/// Whether the walk of the layer at `dir`, its canonical directory, skips
/// the top-level `name`: one of [`UNPINNED_TOP_LEVEL`], or a folder
/// bundle's [`STATE`] (decision 0083). Every other bundle pins a
/// top-level `state/` as it always did.
pub(crate) fn unpinned_top_level(dir: &Path, name: &str) -> bool {
    UNPINNED_TOP_LEVEL.contains(&name) || name == STATE && folder_bundle(dir).is_some()
}

/// The repository root of a folder bundle: `dir`, a layer's canonical
/// directory, is named `.brokkr` (ruling 1).
fn folder_bundle(dir: &Path) -> Option<&Path> {
    dir.parent()
        .filter(|_| dir.file_name() == Some(FOLDER.as_ref()))
}

/// The agent library, the adapters and the default map a workspace's
/// launches read. Where their run-local state stands is
/// [`Layout::state`]'s to judge, because a workspace with no place for it
/// refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    pub agents: PathBuf,
    pub adapters: PathBuf,
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

    /// The layout of `root` itself, judged with no walk: what `init`
    /// asks of the directory it writes into.
    pub fn of_root(root: &Path) -> Result<Layout, PlacesError> {
        let root = root.to_path_buf();
        Layout::at(root.clone(), root.clone(), &root)
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
    /// whether a journal stands both in `.forge/` and in the folder.
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
/// regular file, or a link that resolves to one (ruling 4: a linked entry
/// is the entry), is the map; anything else refuses with its path.
pub(crate) fn default_map(workspace: &Path) -> Result<Option<PathBuf>, PlacesError> {
    let map = Places::of(workspace)?.map;
    let absent =
        std::fs::symlink_metadata(&map).is_err_and(|error| error.kind() == ErrorKind::NotFound);
    if absent {
        return Ok(None);
    }
    match std::fs::metadata(&map) {
        Ok(target) if target.is_file() => Ok(Some(map)),
        Ok(_) => Err(PlacesError::MapNotAFile { path: map }),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            Err(PlacesError::MapDangling { path: map })
        }
        Err(source) => Err(PlacesError::Unreadable { path: map, source }),
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
    let recipes = root.join("recipes");
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

/// A bundle whose directory is a `.brokkr` folder composes no base
/// (ruling 1): its library would be the repository root, a directory
/// the project owns. Refused before any base is read. `dir` is the
/// layer's canonical directory.
pub(crate) fn refuse_folder_base(dir: &Path, base: &str) -> Result<(), CompileError> {
    match folder_bundle(dir) {
        Some(root) => Err(CompileError::Invalid(format!(
            "{}/{FOLDER}/bundle.json extends '{base}' (decision 0083); a bundle in {FOLDER}/ \
             composes no base, because its library would be {}: remove extends",
            root.display(),
            root.display()
        ))),
        None => Ok(()),
    }
}

#[cfg(test)]
pub(crate) mod places_tests;
