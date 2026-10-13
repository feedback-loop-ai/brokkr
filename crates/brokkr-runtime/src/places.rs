//! Where a workspace keeps what Brokkr reads and writes (decision 0083):
//! an adopter's `.brokkr/` folder, or a recipe library's own layout. The
//! CLI discovers the folder and refuses what it cannot read; this module
//! is the one derivation every reader shares once a workspace is known,
//! so a launch, a queued launch, the engine and the map's default cannot
//! disagree about a place (ruling 3).

use std::path::{Path, PathBuf};

use brokkr_core::realms::DEFAULT_MAP_FILE;

use crate::bundle::{CompileError, DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};

/// The adopter's folder, at a repository's root (ruling 1).
pub const FOLDER: &str = ".brokkr";

/// The folder's runtime state, ignored by git and never pinned: the
/// fourth top-level name the bundle walk skips (ruling 1).
pub const STATE: &str = "state";

/// A recipe library's run-local directory, beside the operated
/// repository, as it has always been (ruling 3's last column).
pub const LIBRARY_STATE: &str = ".forge";

/// The secrets store's name in the run-local directory, where the engine
/// threads it and where the CLI's secrets verbs default (decision 0012).
pub const SECRETS: &str = "secrets.env";

/// The top-level names a bundle's manifest walk skips (decisions 0042,
/// 0065 and 0083): workspace declarations pinned into the run manifest,
/// the operator's capability definitions, and the folder's state.
pub(crate) const UNPINNED_TOP_LEVEL: [&str; 4] = [
    DEFAULT_MAP_FILE,
    "dialects",
    crate::capabilities::DEFINITIONS_DIR,
    STATE,
];

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
    /// The places of `workspace`: its folder's when `workspace/.brokkr`
    /// is a directory, judged by the entry itself and never through a
    /// link, else the library layout relative to `workspace`.
    pub fn of(workspace: &Path) -> Places {
        match folder(workspace) {
            Some(folder) => Places {
                agents: folder.join(DEFAULT_AGENTS_DIR),
                adapters: folder.join(DEFAULT_ADAPTERS_DIR),
                map: folder.join(DEFAULT_MAP_FILE),
                state: Some(folder.join(STATE)),
            },
            None => Places {
                agents: workspace.join(DEFAULT_AGENTS_DIR),
                adapters: workspace.join(DEFAULT_ADAPTERS_DIR),
                map: workspace.join(DEFAULT_MAP_FILE),
                state: None,
            },
        }
    }
}

/// `workspace/.brokkr` when that entry is a directory and `workspace` is
/// no recipe library: a library keeps its own layout, a `.brokkr` beside
/// it included (the operator's ruling 1 of 2026-10-13 on #634).
pub(crate) fn folder(workspace: &Path) -> Option<PathBuf> {
    let folder = workspace.join(FOLDER);
    let entry = std::fs::symlink_metadata(&folder);
    let found = entry.is_ok_and(|entry| entry.is_dir());
    (found && !library(workspace)).then_some(folder)
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
    match (dir.file_name(), dir.parent()) {
        (Some(name), Some(root)) if name == FOLDER => Err(CompileError::Invalid(format!(
            "{}/{FOLDER}/bundle.json extends '{base}' (decision 0083); a bundle in {FOLDER}/ \
             composes no base, because its library would be {}: remove extends",
            root.display(),
            root.display()
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod places_tests;
