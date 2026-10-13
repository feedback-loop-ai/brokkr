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
}

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

/// The agent library, the adapters, the default map and the run-local
/// directory a workspace's launches read and write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    pub agents: PathBuf,
    pub adapters: PathBuf,
    /// The map a workspace adopts when none is named, if it is there.
    pub(crate) map: PathBuf,
    /// The folder's `state/`; `None` in a library, whose run-local
    /// directory is the operated repository's `.forge/` ([`run_local`]).
    pub state: Option<PathBuf>,
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
                state: None,
            },
        })
    }

    /// The places of the folder at `folder`, already found.
    pub fn in_folder(folder: &Path) -> Places {
        Places {
            agents: folder.join(DEFAULT_AGENTS_DIR),
            adapters: folder.join(DEFAULT_ADAPTERS_DIR),
            map: folder.join(DEFAULT_MAP_FILE),
            state: Some(folder.join(STATE)),
        }
    }
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
    let unreadable = |source| PlacesError::Unreadable {
        path: folder.clone(),
        source,
    };
    match entry(&folder).map_err(unreadable)? {
        Some(found) if !found.is_dir() => Err(PlacesError::NotADirectory {
            root: workspace.to_path_buf(),
        }),
        found => Ok(found.filter(|_| !library(workspace)).map(|_| folder)),
    }
}

/// What stands at `path`, judged by the entry itself and never through a
/// link: `None` when nothing does, or nothing can because a parent is a
/// file.
pub fn entry(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    let absent = |kind| matches!(kind, ErrorKind::NotFound | ErrorKind::NotADirectory);
    match std::fs::symlink_metadata(path) {
        Ok(found) => Ok(Some(found)),
        Err(error) if absent(error.kind()) => Ok(None),
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
/// 0078) and `init` never wrote one.
pub fn library(root: &Path) -> bool {
    let Ok(recipes) = std::fs::read_dir(root.join("recipes")) else {
        return false;
    };
    recipes
        .flatten()
        .any(|recipe| recipe.path().join("bundle.json").is_file())
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
mod places_tests;
