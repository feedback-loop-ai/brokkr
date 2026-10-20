//! The shelves one compile composes across (#697 rulings 3 and 5): the
//! workspace's project shelf and its folder's own bundle, and the personal
//! shelf beside `host.json`. Each layer of a composition has one
//! [`Origin`], judged here by where its directory stands, and the origin
//! decides where its `extends` reads: a personal recipe reads its own
//! shelf, the project's, or the folder's bundle through the reserved name
//! `.brokkr`; nothing on a project shelf or in a folder bundle reads a
//! personal recipe, and naming one refuses ([`ShelfRefusal`]).

use std::fmt;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use super::{folder_bundle, resolved, Kind, Layout, PlacesError, FOLDER};
use crate::bundle::CompileError;

/// Where a composed layer stands (#697 ruling 5): the personal shelf, a
/// project shelf (a folder's `.brokkr/recipes` or a library's `recipes`),
/// the folder's own bundle, or any other library a layer's parent is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Personal,
    Project,
    Folder,
    Library,
}

impl Origin {
    /// The origin's one word, which the journal records and every surface
    /// prints.
    pub fn word(self) -> &'static str {
        match self {
            Origin::Personal => "personal",
            Origin::Project => "project",
            Origin::Folder => "folder",
            Origin::Library => "library",
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.word())
    }
}

impl serde::Serialize for Origin {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// One composed layer, leaf first, as a run's start records it and
/// `recipes show` prints it (#697 ruling 5): its origin and its directory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ComposedLayer {
    pub origin: Origin,
    pub path: String,
}

/// A layer naming what its origin may not compose from (#697 rulings 2
/// and 3). A caller matches the variant, never the text (decision 0071
/// ruling 8).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShelfRefusal {
    /// A name standing on both shelves: neither wins.
    #[error(
        "recipe '{name}' stands on both the project shelf, at {}, and the personal shelf, at {} \
         (#697): neither shelf wins, so rename or remove one of them",
        .project.display(),
        .personal.display()
    )]
    Shadowed {
        name: String,
        project: PathBuf,
        personal: PathBuf,
    },
    /// A project recipe naming a recipe only the personal shelf holds.
    #[error(
        "the project recipe at {} extends '{}', which stands only on the personal shelf, at {} \
         (#697): a project recipe never extends a personal one, since teammates and CI could not \
         compile it, so put '{}' on the project shelf or extend a recipe there",
        .0.layer.display(),
        .0.base,
        .0.personal.display(),
        .0.base
    )]
    ProjectNamesPersonal(Named),
    /// The folder's bundle naming a recipe only the personal shelf holds.
    #[error(
        "the folder's bundle at {} extends '{}', which stands only on the personal shelf, at {} \
         (#697): the folder's bundle never extends a personal recipe, since teammates and CI \
         could not compile it, so put '{}' on the project shelf or extend a recipe there",
        .0.layer.display(),
        .0.base,
        .0.personal.display(),
        .0.base
    )]
    FolderNamesPersonal(Named),
    /// A personal recipe extending `.brokkr` from a workspace with no
    /// folder.
    #[error(
        "the personal recipe at {} extends '{FOLDER}', the folder's own bundle, but this \
         workspace has no {FOLDER} folder (#697): run it from a folder repository, or extend a \
         recipe on a shelf",
        .layer.display()
    )]
    Unfoldered { layer: PathBuf },
}

/// A project layer, `layer`, and the personal recipe it names, `base`, at
/// `personal`.
#[derive(Debug, PartialEq, Eq)]
pub struct Named {
    pub layer: PathBuf,
    pub base: String,
    pub personal: PathBuf,
}

/// Where a shelf resolves ([`resolving_shelf`]).
#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedShelf<'a> {
    /// The nearest entry standing at the shelf or above it.
    pub at: &'a Path,
    /// The directory `at` resolves to, every link resolved.
    pub real: PathBuf,
    /// The shelf beneath `real`, the rest of its path still to be made.
    pub place: PathBuf,
}

/// Judge the `origin` shelf `shelf` before anything on it is read or
/// written (#697): the nearest entry standing at it or above it must
/// resolve to a directory ([`resolved`], the judgement the default map
/// takes too), so nothing standing at the shelf is an empty one, and a
/// link to nothing, a looping link, a file, or an entry whose metadata
/// cannot be read refuses ([`PlacesError::ShelfUnread`]), never read as an
/// empty shelf. A relative shelf nothing stands at or above resolves as
/// itself. The entry judged is the entry itself, never the target a
/// trailing separator reads through, since each ancestor is rebuilt from
/// its components. The part of the shelf beneath that entry, the current
/// directory for a relative one, holds no `..` (`climbs`). The one
/// judgement `--recipe`, `recipes add` and every compile's reads of a
/// shelf (`Shelves`) take.
pub fn resolving_shelf(origin: Origin, shelf: &Path) -> Result<ResolvedShelf<'_>, PlacesError> {
    for at in shelf.ancestors() {
        let unread = |cause| PlacesError::ShelfUnread {
            origin,
            shelf: shelf.to_path_buf(),
            at: at.to_path_buf(),
            cause,
        };
        match resolved(at) {
            Ok(None) => {}
            Ok(Some(real)) if real.is_dir() => {
                let rest = shelf
                    .strip_prefix(at)
                    .expect("an ancestor prefixes its path");
                climbs(origin, shelf, at, rest)?;
                // Collected from its parts, so an empty `rest` leaves no trailing `/`.
                let place = real.join(rest).components().collect();
                return Ok(ResolvedShelf { at, real, place });
            }
            Ok(Some(_)) => return Err(unread(std::io::ErrorKind::NotADirectory.into())),
            Err(cause) => return Err(unread(cause)),
        }
    }
    climbs(origin, shelf, Path::new("."), shelf)?;
    let (real, place) = (shelf.to_path_buf(), shelf.to_path_buf());
    Ok(ResolvedShelf {
        at: shelf,
        real,
        place,
    })
}

/// Refuse the `origin` shelf `shelf` where `rest`, its part beneath `at`,
/// the nearest entry standing at it or above it, holds a `..` (#726): the
/// first part of `rest` does not stand, so the kernel reports the shelf
/// absent, and a shelf read as absent there would be an empty one, past
/// the refusal of a name on both shelves ([`ShelfRefusal::Shadowed`]). A
/// `..` beneath a standing entry is in `at`, where the kernel resolves it.
fn climbs(origin: Origin, shelf: &Path, at: &Path, rest: &Path) -> Result<(), PlacesError> {
    match rest.components().any(|part| part == Component::ParentDir) {
        true => Err(PlacesError::ShelfClimbs {
            origin,
            shelf: shelf.to_path_buf(),
            at: at.to_path_buf(),
        }),
        false => Ok(()),
    }
}

/// The shelves one compile composes across: the project's, the folder's
/// own bundle where the workspace is a folder repository, and the personal
/// one where the machine places it. The default is no shelf at all: every
/// layer composes from its own library.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Shelves {
    project: Option<Shelf>,
    folder: Option<PathBuf>,
    personal: Option<Shelf>,
}

/// One shelf of a compile: its `origin`, the shelf as `spelled`, and where
/// it resolved when the compile began, if it could be read, which a
/// layer's origin is judged by. A shelf that cannot be read then is never
/// one a layer stands on, and every read of it judges it again
/// ([`Shelf::read`]), so it refuses there, never read as an empty one
/// (#697).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shelf {
    origin: Origin,
    spelled: PathBuf,
    real: Option<PathBuf>,
}

impl Shelf {
    fn of(origin: Origin, spelled: PathBuf) -> Shelf {
        let mut shelf = Shelf {
            origin,
            spelled,
            real: None,
        };
        shelf.real = shelf.read().ok();
        shelf
    }

    /// Where the shelf stands as a read finds it now ([`resolving_shelf`]):
    /// the directory it resolves to, or where nothing stands at it, its
    /// place beneath the nearest entry that stands, at which nothing does.
    /// One that cannot be read refuses, in `--recipe`'s words.
    fn read(&self) -> Result<PathBuf, CompileError> {
        match resolving_shelf(self.origin, &self.spelled) {
            Ok(found) => Ok(found.place),
            Err(refused) => Err(CompileError::Invalid(refused.to_string())),
        }
    }

    /// Where a read of `shelf`, if a compile has one, finds it.
    fn reading(shelf: &Option<Shelf>) -> Result<Option<PathBuf>, CompileError> {
        shelf.as_ref().map(Shelf::read).transpose()
    }

    /// Where `shelf`, if a compile has one, resolved when it began.
    fn stood(shelf: &Option<Shelf>) -> Option<&Path> {
        shelf.as_ref().and_then(|shelf| shelf.real.as_deref())
    }
}

/// Whether anything stands at `path`, judged as the entry itself. An entry
/// that cannot be read refuses, never read as absent.
fn stands(path: PathBuf) -> Result<(bool, PathBuf), CompileError> {
    match super::entry(&path) {
        Ok(found) => Ok((found.is_some(), path)),
        Err(source) => Err(CompileError::Invalid(
            PlacesError::Unreadable { path, source }.to_string(),
        )),
    }
}

impl Shelves {
    /// The shelves a compile from the workspace `layout` composes across:
    /// the `project` shelf the operator selected, else the workspace's own
    /// (#697 ruling 3), and the `personal` shelf this machine places, if
    /// any.
    pub(crate) fn of(
        layout: &Layout,
        project: Option<&Path>,
        personal: Option<PathBuf>,
    ) -> Shelves {
        let folder = (layout.kind() == Kind::Folder).then(|| layout.root().join(FOLDER));
        let project = project.map_or_else(|| layout.places().recipes, Path::to_path_buf);
        Shelves {
            project: Some(Shelf::of(Origin::Project, project)),
            folder,
            personal: personal.map(|shelf| Shelf::of(Origin::Personal, shelf)),
        }
    }

    /// The origin of the layer at `dir`, its canonical directory: the
    /// folder's bundle by the one judgement of a layout ([`folder_bundle`]),
    /// else the shelf its parent is, else its library.
    pub(crate) fn origin(&self, dir: &Path) -> Result<Origin, CompileError> {
        let judged =
            folder_bundle(dir).map_err(|refused| CompileError::Invalid(refused.to_string()));
        let parent = Some(dir.parent().unwrap_or(dir));
        Ok(match judged? {
            Some(_) => Origin::Folder,
            None if parent == Shelf::stood(&self.personal) => Origin::Personal,
            None if parent == Shelf::stood(&self.project) => Origin::Project,
            None => Origin::Library,
        })
    }

    /// The folder's own bundle, which a personal layer at `dir` names as
    /// `.brokkr`, refused where the workspace has no folder.
    pub(crate) fn folder(&self, dir: &Path) -> Result<PathBuf, CompileError> {
        let layer = dir.to_path_buf();
        let refused = || CompileError::Shelf(ShelfRefusal::Unfoldered { layer });
        self.folder.clone().ok_or_else(refused)
    }

    /// The library a personal layer at `dir` reads its base `base` from
    /// (#697 rulings 2 and 3): its own shelf, its parent, where the name
    /// stands there alone, else the project's where it stands there. A
    /// name with an entry on both refuses before anything at it is read
    /// ([`ShelfRefusal::Shadowed`]), and an entry, or a project shelf, that
    /// cannot be read refuses.
    pub(crate) fn personal_library(&self, dir: &Path, base: &str) -> Result<PathBuf, CompileError> {
        let personal = dir.parent().unwrap_or(dir);
        let (mine, at) = stands(personal.join(base))?;
        let Some(project) = &Shelf::reading(&self.project)? else {
            return Ok(personal.to_path_buf());
        };
        match (mine, stands(project.join(base))?) {
            (true, (true, theirs)) => Err(CompileError::Shelf(ShelfRefusal::Shadowed {
                name: base.to_string(),
                project: theirs,
                personal: at,
            })),
            (false, (true, _)) => Ok(project.clone()),
            (_, (false, _)) => Ok(personal.to_path_buf()),
        }
    }

    /// Refuse a project or folder layer at `dir` whose base `base`, which
    /// its own library does not hold, stands on the personal shelf (#697
    /// ruling 3): no project layer composes from it, and a personal shelf
    /// that cannot be read refuses. A personal or library layer refuses
    /// nothing here.
    pub(crate) fn refuse_personal(
        &self,
        origin: Origin,
        dir: &Path,
        base: &str,
    ) -> Result<(), CompileError> {
        let refusal = match origin {
            Origin::Project => ShelfRefusal::ProjectNamesPersonal,
            Origin::Folder => ShelfRefusal::FolderNamesPersonal,
            Origin::Personal | Origin::Library => return Ok(()),
        };
        let Some(shelf) = Shelf::reading(&self.personal)? else {
            return Ok(());
        };
        match stands(shelf.join(base))? {
            (true, personal) => Err(CompileError::Shelf(refusal(Named {
                layer: dir.to_path_buf(),
                base: base.to_string(),
                personal,
            }))),
            (false, _) => Ok(()),
        }
    }
}

#[cfg(test)]
mod shelves_tests;
