//! The shelves one compile composes across (#697 rulings 3 and 5): the
//! workspace's project shelf and its folder's own bundle, and the personal
//! shelf beside `host.json`. Each layer of a composition has one
//! [`Origin`], judged here by where its directory stands, and the origin
//! decides where its `extends` reads: a personal recipe reads its own
//! shelf, the project's, or the folder's bundle through the reserved name
//! `.brokkr`; nothing on a project shelf or in a folder bundle reads a
//! personal recipe, and naming one refuses ([`ShelfRefusal`]).

use std::fmt;
use std::path::{Path, PathBuf};

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

/// The shelves one compile composes across, each where it resolves: the
/// project's, the folder's own bundle where the workspace is a folder
/// repository, and the personal one where the machine places it. A shelf
/// nothing stands at is none. One that does not resolve is kept as
/// spelled, so a layer is never judged to stand on it and every entry read
/// on it is judged as spelled, refusing where it cannot be read. The
/// default is no shelf at all: every layer composes from its own library.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Shelves {
    project: Option<PathBuf>,
    folder: Option<PathBuf>,
    personal: Option<PathBuf>,
}

/// Where `shelf` resolves, or `shelf` itself where it does not.
fn real(shelf: PathBuf) -> Option<PathBuf> {
    let found = resolved(&shelf);
    found.unwrap_or(Some(shelf))
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
    /// The shelves a compile from the workspace `layout` composes across,
    /// with the `personal` shelf this machine places, if any.
    pub(crate) fn of(layout: &Layout, personal: Option<PathBuf>) -> Shelves {
        let folder = (layout.kind() == Kind::Folder).then(|| layout.root().join(FOLDER));
        Shelves {
            project: real(layout.places().recipes),
            folder,
            personal: personal.and_then(real),
        }
    }

    /// The origin of the layer at `dir`, its canonical directory: the
    /// folder's bundle by the one judgement of a layout ([`folder_bundle`]),
    /// else the shelf its parent is, else its library.
    pub(crate) fn origin(&self, dir: &Path) -> Result<Origin, CompileError> {
        let judged =
            folder_bundle(dir).map_err(|refused| CompileError::Invalid(refused.to_string()));
        let parent = Some(dir.parent().unwrap_or(dir).to_path_buf());
        Ok(match judged? {
            Some(_) => Origin::Folder,
            None if parent == self.personal => Origin::Personal,
            None if parent == self.project => Origin::Project,
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
    /// ([`ShelfRefusal::Shadowed`]), and an entry that cannot be read
    /// refuses.
    pub(crate) fn personal_library(&self, dir: &Path, base: &str) -> Result<PathBuf, CompileError> {
        let personal = dir.parent().unwrap_or(dir);
        let (mine, at) = stands(personal.join(base))?;
        let Some(project) = &self.project else {
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
    /// ruling 3): no project layer composes from it. A personal or library
    /// layer refuses nothing here.
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
        let Some(shelf) = &self.personal else {
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
