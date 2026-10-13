//! Where a workspace keeps what Brokkr reads and writes (decision 0083):
//! an adopter's `.brokkr/` folder, or a recipe library's own layout. The
//! CLI discovers the folder and refuses what it cannot read; this module
//! is the one derivation every reader shares once a workspace is known,
//! so a launch, a queued launch, the engine and the map's default cannot
//! disagree about a place (ruling 3).

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
/// launches read. Where their run-local state stands is [`Kind::state`]'s
/// to judge, because a workspace with no place for it refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    pub agents: PathBuf,
    pub adapters: PathBuf,
    /// The map a workspace adopts when none is named, if it is there.
    pub(crate) map: PathBuf,
}

impl Places {
    /// The places of `workspace`: its folder's when [`folder`] finds one,
    /// else the library layout relative to `workspace`. A root `.brokkr`
    /// that is not a directory refuses, never falling back to the
    /// library layout.
    pub fn of(workspace: &Path) -> Result<Places, PlacesError> {
        Ok(match folder(workspace)? {
            Some(folder) => Places::in_folder(&folder),
            None => Places {
                agents: workspace.join(DEFAULT_AGENTS_DIR),
                adapters: workspace.join(DEFAULT_ADAPTERS_DIR),
                map: workspace.join(DEFAULT_MAP_FILE),
            },
        })
    }

    /// The places of the folder at `folder`, already found.
    pub fn in_folder(folder: &Path) -> Places {
        Places {
            agents: folder.join(DEFAULT_AGENTS_DIR),
            adapters: folder.join(DEFAULT_ADAPTERS_DIR),
            map: folder.join(DEFAULT_MAP_FILE),
        }
    }
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
    pub fn of(root: &Path) -> Result<Kind, PlacesError> {
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
    pub fn state(
        self,
        cwd: &Path,
        root: &Path,
        escape: bool,
    ) -> Result<Option<PathBuf>, PlacesError> {
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
    for name in [BUNDLE, LIBRARY_STATE] {
        let path = root.join(name);
        if entry(&path).map_err(unreadable(&path))?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The repository root above `cwd`, an absolute path: the first directory
/// holding a `.git` entry, or `cwd` itself outside any repository. An
/// entry the walk cannot read refuses, never taken as absent. The walk is
/// over the path as it is named, as git's own is.
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
/// at dispatch by the CLI's own rule ([`Kind::state`]) at the root the
/// CLI's walk reaches: the journal a launch is handed is always named, so
/// the escape is taken, and a workspace whose folder has gone since it
/// was queued, with no library and no old layout standing, refuses rather
/// than writing a `.forge/` into a bare repository.
pub fn state_of(workspace: &Path) -> Result<Option<PathBuf>, PlacesError> {
    let cwd = std::path::absolute(workspace).map_err(unreadable(workspace))?;
    let root = root(&cwd)?;
    Kind::of(&root)?.state(&cwd, &root, true)
}

/// The one judgement of a workspace's root `.brokkr` entry, which the
/// CLI's discovery and every re-derivation at dispatch share: the folder
/// when the entry itself, never read through a link, is a directory and
/// `workspace` is no recipe library (a library keeps its own layout, a
/// `.brokkr` beside it included: the operator's ruling 1 of 2026-10-13 on
/// #634); `None` when nothing stands there; and a refusal for anything
/// else, or an entry that cannot be read.
pub fn folder(workspace: &Path) -> Result<Option<PathBuf>, PlacesError> {
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
pub fn library(root: &Path) -> Result<bool, PlacesError> {
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
