//! What `brokkr init <dir>` admits before it writes anything (decision
//! 0083 ruling 5): `dir` is its repository's root, as discovery reads a
//! folder only there; the root is no recipe library, which reads none
//! (the operator's ruling 1 of 2026-10-13); it holds no old layout, each
//! refused with that shape's own refusal; and its folder, where one
//! stands, holds only what the state-only refusal's moves leave. These
//! stand in place of the file-by-file refusals `init` gave before.

use std::fmt;
use std::path::{Path, PathBuf};

use brokkr_runtime::places::{Kind, FOLDER, STATE};

use super::{unreadable, FolderError, Layout, IGNORE};

/// A root `init` does not write a folder under, in the operator's words.
#[derive(Debug)]
pub(crate) enum Refused {
    /// A `dir` below its repository's root, where no verb would read the
    /// folder.
    NotRoot { dir: PathBuf, root: PathBuf },
    /// A recipe library, which reads no folder.
    Library { root: PathBuf },
    /// A folder holding more than moved state: each entry it holds beside
    /// a `.gitignore` and `state/`.
    Occupied { root: PathBuf, held: Vec<String> },
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refused::NotRoot { dir, root } => write!(
                f,
                "{} is inside the repository {} but is not its root (decision 0083); a folder is \
                 read only at the root: run `brokkr init {}`",
                dir.display(),
                root.display(),
                root.display()
            ),
            Refused::Library { root } => write!(
                f,
                "{} is a recipe library, which keeps its own layout and reads no {FOLDER}/ folder \
                 (decision 0083; the operator's ruling 1 of 2026-10-13): init writes nothing here",
                root.display()
            ),
            Refused::Occupied { root, held } => write!(
                f,
                "{}/{FOLDER} already holds {} (decision 0083); init writes a folder only where \
                 none stands, or beside moved state ({IGNORE} and {STATE}/ alone): move or remove \
                 them",
                root.display(),
                held.join(", ")
            ),
        }
    }
}

/// The root `brokkr init <dir>` writes its folder under: `dir` itself,
/// walked as discovery walks ([`Layout::discover`]), so a `dir` that is
/// missing or unreadable, or a `.brokkr` below its root or one that is
/// not a directory, refuses in discovery's words first.
pub(crate) fn root(dir: &Path) -> Result<PathBuf, FolderError> {
    let layout = Layout::discover(dir)?;
    let (cwd, root) = (layout.0.cwd(), layout.0.root().to_path_buf());
    if cwd != root {
        let dir = cwd.to_path_buf();
        return Err(FolderError::Init(Refused::NotRoot { dir, root }));
    }
    layout.refuse_old()?;
    match layout.0.kind() {
        Kind::Library => Err(FolderError::Init(Refused::Library { root })),
        Kind::Bare => Ok(root),
        Kind::Folder => match held(&root.join(FOLDER))? {
            held if held.is_empty() => Ok(root),
            held => Err(FolderError::Init(Refused::Occupied { root, held })),
        },
    }
}

/// The names of what `folder` holds beside the moved state `init` writes
/// beside, a regular-file `.gitignore` and a directory `state`, sorted.
/// Each is judged as the entry itself and never through a link (ruling
/// 4), by the one stat ([`super::entry`]); an entry or a listing that
/// cannot be read refuses, and an entry gone between the listing and its
/// stat is held, so it refuses by name.
fn held(folder: &Path) -> Result<Vec<String>, FolderError> {
    let mut held = Vec::new();
    for listed in std::fs::read_dir(folder).map_err(unreadable(folder))? {
        let (path, name) = listed
            .map(|e| (e.path(), e.file_name()))
            .map_err(unreadable(folder))?;
        let moved = super::entry(&path)?.is_some_and(|found| {
            (name == IGNORE && found.is_file()) || (name == STATE && found.is_dir())
        });
        if !moved {
            held.push(name.to_string_lossy().into_owned());
        }
    }
    held.sort();
    Ok(held)
}
