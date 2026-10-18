//! The personal shelf (#697): an operator's own named recipes, kept in the
//! machine's configuration directory beside `host.json`
//! ([`places::personal_shelf`]) and never in a repository. `recipes add
//! --personal` installs there, compiled against the workspace's agents and
//! adapters as every install is; `--recipe` reads the project's shelf, then
//! this one, and a name on both refuses ([`Unshelved::Shadowed`]);
//! `recipes list` and doctor show both, each recipe with its origin.

use std::fmt;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use anyhow::Result;
use brokkr_runtime::admission::HostError;
use brokkr_runtime::places::{self, PlacesError};

/// A shelf's path, normalised once where it enters the CLI (#697; decision
/// 0071 ruling 3): rebuilt from its components, so no trailing separator,
/// `.` component or doubled separator is left, and every spelling of one
/// shelf is judged, and read, at the same entry. A `..` stays as spelled,
/// judged like any other entry ([`resolving`]), since folding it lexically
/// is not what a link resolves to. The one constructor normalises, so no
/// spelling of a shelf reaches a judgement or a read raw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShelfPath(PathBuf);

impl ShelfPath {
    /// The shelf `spelled` names, normalised. A spelling of `.` alone is
    /// the current directory, and an empty one stays empty.
    pub(crate) fn new(spelled: impl AsRef<Path>) -> ShelfPath {
        let spelled = spelled.as_ref();
        let parts = spelled
            .components()
            .filter(|part| *part != Component::CurDir);
        let path: PathBuf = parts.collect();
        match path.as_os_str().is_empty() && !spelled.as_os_str().is_empty() {
            true => ShelfPath(PathBuf::from(".")),
            false => ShelfPath(path),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn into_path(self) -> PathBuf {
        self.0
    }
}

/// The personal shelf, from this process's `XDG_CONFIG_HOME` and `HOME`:
/// the CLI's edge, where every verb that reads the shelf asks.
pub(crate) fn shelf() -> Result<ShelfPath, HostError> {
    let placed = places::personal_shelf(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    );
    placed.map(ShelfPath::new)
}

/// The personal shelf, where this machine places one. Where it places
/// none, no personal recipe can stand, so a reader has nothing to read.
pub(crate) fn placed() -> Option<ShelfPath> {
    shelf().ok()
}

/// The shelf a recipe stands on: the project's, a folder's `.brokkr/recipes`
/// or a library's `recipes`, or the personal one (#697).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    Project,
    Personal,
}

impl Origin {
    /// What doctor calls a recipe on this shelf, and the shelf, and the
    /// issue its refusal cites.
    pub(crate) fn doctor(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Origin::Project => ("recipe", "shelf", "#696"),
            Origin::Personal => ("personal recipe", "personal shelf", "#697"),
        }
    }

    /// Where an install onto this origin's shelf `dir`, an absolute path,
    /// resolves ([`resolving`]), a shelf that cannot be read refusing. A
    /// project library is refused anywhere in a folder repository but its
    /// shelf ([`super::refuse_folder`]); the personal shelf is judged by
    /// no repository, folder or library mark, its own or an ancestor's
    /// (#697 ruling 1).
    pub(super) fn place(self, dir: &ShelfPath) -> Result<PathBuf> {
        let resolved = resolving(self, dir)?;
        match self {
            Origin::Project => super::refuse_folder(resolved),
            Origin::Personal => Ok(resolved.place),
        }
    }
}

/// Where a shelf resolves ([`resolving`]).
pub(super) struct Resolved<'a> {
    /// The nearest entry standing at the shelf or above it.
    pub(super) at: &'a Path,
    /// The directory `at` resolves to, every link resolved.
    pub(super) real: PathBuf,
    /// The shelf beneath `real`, the rest of its path still to be made.
    pub(super) place: PathBuf,
}

/// Judge the `origin` shelf `shelf` before anything on it is read or
/// written (#697): the nearest entry standing at it or above it must
/// resolve to a directory ([`places::resolved`], the judgement the default
/// map takes too), so nothing standing at the shelf is an empty one, and
/// a link to nothing, a looping link, a file, or an entry whose metadata
/// cannot be read refuses ([`Unshelved::Unread`]), never read as an empty
/// shelf. A relative shelf nothing stands at or above resolves as itself.
/// The shelf comes normalised ([`ShelfPath`]), so the entry judged is the
/// entry itself, never the target a trailing separator reads through. The
/// part of the shelf beneath that entry, the current directory for a
/// relative one, holds no `..` ([`climbs`]).
pub(super) fn resolving(origin: Origin, shelf: &ShelfPath) -> Result<Resolved<'_>, Unshelved> {
    let shelf = shelf.path();
    for at in shelf.ancestors() {
        let unread = |cause| Unshelved::Unread {
            origin,
            shelf: shelf.to_path_buf(),
            at: at.to_path_buf(),
            cause,
        };
        match places::resolved(at) {
            Ok(None) => {}
            Ok(Some(real)) if real.is_dir() => {
                let rest = shelf
                    .strip_prefix(at)
                    .expect("an ancestor prefixes its path");
                climbs(origin, shelf, at, rest)?;
                // Collected from its parts, so an empty `rest` leaves no trailing `/`.
                let place = real.join(rest).components().collect();
                return Ok(Resolved { at, real, place });
            }
            Ok(Some(_)) => return Err(unread(ErrorKind::NotADirectory.into())),
            Err(cause) => return Err(unread(cause)),
        }
    }
    climbs(origin, shelf, Path::new("."), shelf)?;
    let (real, place) = (shelf.to_path_buf(), shelf.to_path_buf());
    Ok(Resolved {
        at: shelf,
        real,
        place,
    })
}

/// Refuse the `origin` shelf `shelf` where `rest`, its part beneath `at`,
/// the nearest entry standing at it or above it, holds a `..` (#726): the
/// first part of `rest` does not stand, so the kernel reports the shelf
/// absent, and a shelf read as absent there would be an empty one, past
/// the refusal of a name on both shelves ([`Unshelved::Shadowed`]). A `..`
/// beneath a standing entry is in `at`, where the kernel resolves it.
fn climbs(origin: Origin, shelf: &Path, at: &Path, rest: &Path) -> Result<(), Unshelved> {
    match rest.components().any(|part| part == Component::ParentDir) {
        true => Err(Unshelved::Climbing {
            origin,
            shelf: shelf.to_path_buf(),
            at: at.to_path_buf(),
        }),
        false => Ok(()),
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Origin::Project => "project",
            Origin::Personal => "personal",
        })
    }
}

/// Why `--recipe <name>` reads no shelf, or `recipes add --personal`
/// installs onto none. A caller matches the variant, never the text
/// (decision 0071 ruling 8).
#[derive(Debug)]
pub(crate) enum Unshelved {
    /// The name is not one plain recipe name: `--recipe` names a recipe,
    /// never a path, so no shelf is read for it (#697 ruling 2).
    Unplain { name: String },
    /// The name stands on both shelves (#697 ruling 2): neither wins.
    Shadowed {
        name: String,
        project: PathBuf,
        personal: PathBuf,
    },
    /// An entry at the name that cannot be read, never taken as absent.
    Unreadable(PlacesError),
    /// A shelf that cannot be read ([`resolving`]): what stands `at` it,
    /// or above it, does not resolve to a directory, for its `cause`.
    Unread {
        origin: Origin,
        shelf: PathBuf,
        at: PathBuf,
        cause: std::io::Error,
    },
    /// A shelf spelled through `..` beneath `at`, the nearest entry of it
    /// that stands ([`climbs`]).
    Climbing {
        origin: Origin,
        shelf: PathBuf,
        at: PathBuf,
    },
    /// An install of a name the project shelf holds (#726), which
    /// resolution would refuse ([`Unshelved::Shadowed`]).
    Shadowing {
        name: String,
        project: PathBuf,
        personal: PathBuf,
    },
    /// A directory above the personal `shelf`, `at`, that the install
    /// could not make for its `cause`, every one it made removed ([`Made`]).
    Unmade {
        shelf: PathBuf,
        at: PathBuf,
        cause: std::io::Error,
    },
}

impl fmt::Display for Unshelved {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unshelved::Unplain { name } => write!(
                formatter,
                "--recipe '{name}' is not one plain recipe name, so it would not name one recipe \
                 on a shelf (#697): name the recipe as it stands on its shelf, or give --bundle \
                 <dir> for a bundle elsewhere; nothing was run"
            ),
            Unshelved::Shadowed {
                name,
                project,
                personal,
            } => write!(
                formatter,
                "recipe '{name}' stands on both the project shelf, at {}, and the personal shelf, \
                 at {} (#697): neither shelf wins, so rename or remove one of them",
                project.display(),
                personal.display()
            ),
            Unshelved::Unreadable(refused) => fmt::Display::fmt(refused, formatter),
            Unshelved::Unread {
                origin,
                shelf,
                at,
                cause,
            } => write!(
                formatter,
                "the {origin} shelf {} cannot be read (#697): {}: {cause}; a shelf that cannot \
                 be read is never read as an empty one, so mend or remove it; nothing was run \
                 or installed",
                shelf.display(),
                at.display()
            ),
            Unshelved::Climbing { origin, shelf, at } => write!(
                formatter,
                "the {origin} shelf {} climbs by '..' out of a directory that does not stand \
                 beneath {} (#726): a shelf spelled through what does not exist is never read as \
                 an empty one, so spell it without the '..'; nothing was run or installed",
                shelf.display(),
                at.display()
            ),
            Unshelved::Shadowing {
                name,
                project,
                personal,
            } => write!(
                formatter,
                "recipe '{name}' stands on the project shelf, at {}, so installing it onto the \
                 personal shelf, at {}, would stand it on both (#726): neither shelf would win, \
                 so rename or remove one of them; nothing was installed",
                project.display(),
                personal.display()
            ),
            Unshelved::Unmade { shelf, at, cause } => write!(
                formatter,
                "the personal shelf {} cannot be made (#726): {}: {cause}; every directory made \
                 for it was removed again, so nothing was installed",
                shelf.display(),
                at.display()
            ),
        }
    }
}

impl std::error::Error for Unshelved {}

/// The shelf `--recipe` reads `recipe` from (#697 rulings 2 and 6): the
/// project's, `project`, else the personal one this machine places. No
/// recipe named reads `project`, as before. A name that is not one plain
/// recipe name, judged by its spelling, refuses before either shelf is
/// read ([`Unshelved::Unplain`]): `run`, `resume`, `rerun`, `queue add`
/// and `recipes show` all ask here.
pub(crate) fn holding(recipe: Option<&str>, project: ShelfPath) -> Result<PathBuf, Unshelved> {
    match recipe {
        Some(name) if !super::plain(Path::new(name)) => Err(Unshelved::Unplain {
            name: name.to_string(),
        }),
        Some(name) => holding_in(name, project, placed().as_ref()),
        None => Ok(project.into_path()),
    }
}

/// [`holding`] with the personal shelf given, for a plain `name`:
/// `project` where the name stands there, the personal shelf where it
/// stands there alone, and `project` where it stands on neither, whose
/// launch refuses it as not installed. Something standing at the name on
/// both shelves refuses ([`Unshelved::Shadowed`]), and so does an entry
/// that cannot be read, and, before either is read, a shelf that cannot
/// be ([`resolving`]).
fn holding_in(
    name: &str,
    project: ShelfPath,
    personal: Option<&ShelfPath>,
) -> Result<PathBuf, Unshelved> {
    resolving(Origin::Project, &project)?;
    let Some(personal) = personal else {
        return Ok(project.into_path());
    };
    resolving(Origin::Personal, personal)?;
    let (project, personal) = (project.path(), personal.path());
    match (stands(project, name)?, stands(personal, name)?) {
        ((true, project), (true, personal)) => Err(Unshelved::Shadowed {
            name: name.to_string(),
            project,
            personal,
        }),
        ((false, _), (true, _)) => Ok(personal.to_path_buf()),
        (_, (false, _)) => Ok(project.to_path_buf()),
    }
}

/// Whether anything stands at `name` on `shelf`, judged as the entry
/// itself, and its path. An entry that cannot be read refuses, never read
/// as absent.
fn stands(shelf: &Path, name: &str) -> Result<(bool, PathBuf), Unshelved> {
    let at = shelf.join(name);
    match places::entry(&at) {
        Ok(entry) => Ok((entry.is_some(), at)),
        Err(source) => Err(Unshelved::Unreadable(PlacesError::Unreadable {
            path: at,
            source,
        })),
    }
}

/// `recipes list`: the project's shelf `dir`, then the personal one where
/// this machine places it, each recipe with its origin (#697 ruling 6). A
/// personal shelf with nothing standing at it lists nothing.
pub(crate) fn list(workspace: &Path, dir: &ShelfPath, shelf: bool) -> Result<()> {
    super::list(workspace, dir, shelf, Origin::Project)?;
    match placed() {
        Some(personal) => super::list(workspace, &personal, true, Origin::Personal),
        None => Ok(()),
    }
}

/// `recipes add --personal` (#697 ruling 6): install onto the personal
/// shelf, compiled against the agents and adapters of the repository at
/// `root` (ruling 4), as every install is, and refused as `host.json`'s
/// lookup is where this machine places no shelf. Nothing is copied into
/// the repository: a personal install copies no agent or charter, and
/// judges no repository, folder or library mark at its shelf
/// ([`Origin::place`]). The shelf is judged before anything is made, a
/// recipe the `project` shelf holds is refused before any is copied
/// ([`Onto::unshadowing`]), and the configuration directories made for
/// the shelf are removed again when the install refuses (#726).
pub(crate) fn add(
    root: &Path,
    project: &ShelfPath,
    source: &str,
    name: &str,
    path: Option<&str>,
) -> Result<()> {
    let shelf = shelf()?;
    resolving(Origin::Personal, &shelf)?;
    let made = Made::above(shelf.path())?;
    super::add(root, &shelf, Onto::Personal { project }, source, name, path)?;
    made.keep();
    Ok(())
}

/// The shelf an install goes onto: the project's, or the personal one,
/// which carries the `project` shelf, the one `--recipe` reads first
/// ([`holding`]), whose recipes it may not install (#726).
#[derive(Clone, Copy)]
pub(crate) enum Onto<'a> {
    Project,
    Personal { project: &'a ShelfPath },
}

impl Onto<'_> {
    /// The origin of the shelf the install goes onto.
    pub(super) fn origin(self) -> Origin {
        match self {
            Onto::Project => Origin::Project,
            Onto::Personal { .. } => Origin::Personal,
        }
    }

    /// Refuse a personal install onto `dir` whose plan, the recipe and
    /// each base its `extends` chain copies, holds a name the project
    /// shelf holds (#726), since resolution would refuse the name on both
    /// ([`Unshelved::Shadowed`]). The project shelf is judged as
    /// resolution judges it, so one that cannot be read refuses
    /// ([`resolving`], [`stands`]). A project install refuses nothing.
    pub(super) fn unshadowing(
        self,
        planned: &[(String, PathBuf)],
        dir: &Path,
    ) -> Result<(), Unshelved> {
        let Onto::Personal { project } = self else {
            return Ok(());
        };
        resolving(Origin::Project, project)?;
        for (name, _) in planned {
            if let (true, project) = stands(project.path(), name)? {
                let (name, personal) = (name.clone(), dir.join(name));
                return Err(Unshelved::Shadowing {
                    name,
                    project,
                    personal,
                });
            }
        }
        Ok(())
    }
}

/// The directories above the personal shelf an install made, in the order
/// made, removed deepest first when dropped unless kept, each only while
/// it is empty.
struct Made(Vec<PathBuf>);

impl Made {
    /// Make the shelf's parent and every directory above it that nothing
    /// stands at, judged as the entry itself, so a link is never removed,
    /// one at a time from the outermost (#726): the guard stands before
    /// the first is made and holds each once made, so a failure at any
    /// depth removes every one made and refuses naming the shelf
    /// ([`Unshelved::Unmade`]). An entry whose presence cannot be read is
    /// not judged to stand, so its making refuses by its own cause.
    fn above(shelf: &Path) -> Result<Made, Unshelved> {
        let parent = shelf.parent().unwrap_or(shelf);
        let missing: Vec<&Path> = parent
            .ancestors()
            .take_while(|dir| !matches!(places::entry(dir), Ok(Some(_))))
            .collect();
        let mut made = Made(Vec::new());
        for dir in missing.into_iter().rev() {
            let unmade = |cause| Unshelved::Unmade {
                shelf: shelf.to_path_buf(),
                at: dir.to_path_buf(),
                cause,
            };
            std::fs::create_dir(dir).map_err(unmade)?;
            made.0.push(dir.to_path_buf());
        }
        Ok(made)
    }

    fn keep(mut self) {
        self.0.clear();
    }
}

impl Drop for Made {
    fn drop(&mut self) {
        for dir in self.0.iter().rev() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[cfg(test)]
mod personal_tests;
