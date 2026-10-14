//! The links an old layout's moves would carry or look through (decision
//! 0083 ruling 4): a link under `.forge/` that would name another path or
//! none once moved into `.brokkr/state/`, and a linked `.forge` whose
//! target lies inside the repository, where git does not look, each
//! refused by name before a move is listed (the fifth landing's review).

use std::fmt;
use std::path::{Component, Path, PathBuf};

use brokkr_runtime::places::{FOLDER, LIBRARY_STATE, STATE};

use super::{unreadable, FolderError, IGNORE};

/// A link no listed move could carry or look through, with its target.
#[derive(Debug)]
pub(crate) enum LinkRefusal {
    /// A link under `.forge/` the moves would leave naming another path or
    /// none ([`stays`]).
    Escaping { path: PathBuf, target: PathBuf },
    /// A `.forge` that is a link to a directory inside the repository:
    /// git does not look through a link, so `git ls-files -- .forge` names
    /// none of the entries it tracks there, and an `mv` of one would leave
    /// git a deletion.
    InRepository { path: PathBuf, target: PathBuf },
}

impl fmt::Display for LinkRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkRefusal::Escaping { path, target } => write!(
                f,
                "{} is a link to {}, which a listed move into {FOLDER}/{STATE} would leave naming \
                 another path or none, so no moves are listed (decision 0083); nothing was \
                 opened: put what it names in its place, or remove it, then run again",
                path.display(),
                target.display()
            ),
            LinkRefusal::InRepository { path, target } => write!(
                f,
                "{} is a link to {}, inside the repository, and git does not look through a \
                 link, so the moves cannot tell which entries git tracks there; no moves are \
                 listed (decision 0083); nothing was opened: move its entries into \
                 {FOLDER}/{STATE} yourself, by `git mv` for each git tracks, then remove the \
                 link and run again",
                path.display(),
                target.display()
            ),
        }
    }
}

/// Every link the moves would carry out of the `.forge` at `forge`, the
/// root's, at any depth below `names`, its entries, but the ignore file,
/// which the list removes and never moves: one that would name another
/// path or none once moved refuses by name ([`stays`]). An entry or a
/// directory that cannot be read refuses, never skipped.
pub(super) fn links_stay(root: &Path, forge: &Path, names: &[String]) -> Result<(), FolderError> {
    let real = |path: &Path| std::fs::canonicalize(path).map_err(unreadable(path));
    let places = [real(root)?.join(LIBRARY_STATE), real(forge)?];
    let moved = names.iter().filter(|name| *name != IGNORE);
    let mut left: Vec<PathBuf> = moved.map(PathBuf::from).collect();
    while let Some(inner) = left.pop() {
        let path = forge.join(&inner);
        let found = std::fs::symlink_metadata(&path).map_err(unreadable(&path))?;
        if found.is_symlink() {
            stays(&places, &path, &inner)?;
        } else if found.is_dir() {
            for listed in std::fs::read_dir(&path).map_err(unreadable(&path))? {
                left.push(inner.join(listed.map_err(unreadable(&path))?.file_name()));
            }
        }
    }
    Ok(())
}

/// Whether the link at `path`, `inner` under `.forge/`, names the same
/// thing once moved to `.brokkr/state/<inner>`: a relative target must
/// resolve inside `.forge`, to no ignore file the list removes
/// ([`escapes`]), and an absolute one must name no path under `.forge`,
/// by the root's name for it or its real directory (`places`), whose
/// entries the list moves away. Anything else refuses by name.
fn stays(places: &[PathBuf], path: &Path, inner: &Path) -> Result<(), FolderError> {
    let target = std::fs::read_link(path).map_err(unreadable(path))?;
    let leaves = match target.is_absolute() {
        true => places.iter().any(|place| target.starts_with(place)),
        false => escapes(inner, &target),
    };
    match leaves {
        true => Err(FolderError::Link(LinkRefusal::Escaping {
            path: path.to_path_buf(),
            target,
        })),
        false => Ok(()),
    }
}

/// Whether `target`, a relative link's, read lexically from `inner`'s
/// directory under `.forge/`, leads above `.forge` or to its ignore file:
/// every entry moves into `.brokkr/state/` one level deeper, so such a
/// target would name another path once moved, or none.
fn escapes(inner: &Path, target: &Path) -> bool {
    let mut resolved: Vec<Component<'_>> = inner
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .collect();
    let mut above = false;
    for part in target.components() {
        match part {
            Component::ParentDir => above |= resolved.pop().is_none(),
            Component::Normal(_) => resolved.push(part),
            // A leading `.`; a relative target holds no root or prefix.
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    above || resolved == [Component::Normal(IGNORE.as_ref())]
}

/// Refuse a `.forge` that is a link to a directory inside `repository`,
/// the root of the repository `root` stands in: git does not look through
/// a link, so what it tracks there no `git ls-files -- .forge` names. A
/// target or a root that cannot be resolved refuses.
pub(super) fn outside(root: &Path, repository: &Path) -> Result<(), FolderError> {
    let path = root.join(LIBRARY_STATE);
    let target = std::fs::canonicalize(&path).map_err(unreadable(&path))?;
    let real = std::fs::canonicalize(repository).map_err(unreadable(repository))?;
    match target.starts_with(&real) {
        true => Err(FolderError::Link(LinkRefusal::InRepository {
            path,
            target,
        })),
        false => Ok(()),
    }
}
