//! The personal shelf (#697): an operator's own named recipes, kept in the
//! machine's configuration directory beside `host.json`
//! ([`places::personal_shelf`]) and never in a repository. `recipes add
//! --personal` installs there, compiled against the workspace's agents and
//! adapters as every install is; `--recipe` reads the project's shelf, then
//! this one, and a name on both refuses ([`Unshelved::Shadowed`]);
//! `recipes list` and doctor show both, each recipe with its origin.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;
use brokkr_runtime::admission::HostError;
use brokkr_runtime::places::{self, PlacesError};

/// The personal shelf, from this process's `XDG_CONFIG_HOME` and `HOME`:
/// the CLI's edge, where every verb that reads the shelf asks.
pub(crate) fn shelf() -> Result<PathBuf, HostError> {
    places::personal_shelf(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// The personal shelf, where this machine places one. Where it places
/// none, no personal recipe can stand, so a reader has nothing to read.
pub(crate) fn placed() -> Option<PathBuf> {
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
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Origin::Project => "project",
            Origin::Personal => "personal",
        })
    }
}

/// Why `--recipe <name>` reads no shelf. A caller matches the variant,
/// never the text (decision 0071 ruling 8).
#[derive(Debug)]
pub(crate) enum Unshelved {
    /// The name stands on both shelves (#697 ruling 2): neither wins.
    Shadowed {
        name: String,
        project: PathBuf,
        personal: PathBuf,
    },
    /// An entry at the name that cannot be read, never taken as absent.
    Unreadable(PlacesError),
}

impl fmt::Display for Unshelved {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl std::error::Error for Unshelved {}

/// The shelf `--recipe` reads `recipe` from (#697 rulings 2 and 6): the
/// project's, `project`, else the personal one this machine places. No
/// recipe named reads `project`, as before.
pub(crate) fn holding(recipe: Option<&str>, project: PathBuf) -> Result<PathBuf, Unshelved> {
    match recipe {
        Some(name) => holding_in(name, project, placed().as_deref()),
        None => Ok(project),
    }
}

/// [`holding`] with the personal shelf given: `project` where the name
/// stands there, the personal shelf where it stands there alone, and
/// `project` where it stands on neither, whose launch refuses it as not
/// installed. Something standing at the name on both shelves refuses
/// ([`Unshelved::Shadowed`]), and so does an entry that cannot be read. A
/// name that is not one plain path component would not name a recipe
/// on the personal shelf, so it reads `project` alone, as it always did.
fn holding_in(name: &str, project: PathBuf, personal: Option<&Path>) -> Result<PathBuf, Unshelved> {
    let Some(personal) = personal.filter(|_| super::plain(Path::new(name))) else {
        return Ok(project);
    };
    let stands = |shelf: &Path| {
        let at = shelf.join(name);
        match places::entry(&at) {
            Ok(entry) => Ok((entry.is_some(), at)),
            Err(source) => Err(Unshelved::Unreadable(PlacesError::Unreadable {
                path: at,
                source,
            })),
        }
    };
    match (stands(&project)?, stands(personal)?) {
        ((true, project), (true, personal)) => Err(Unshelved::Shadowed {
            name: name.to_string(),
            project,
            personal,
        }),
        ((false, _), (true, _)) => Ok(personal.to_path_buf()),
        (_, (false, _)) => Ok(project),
    }
}

/// `recipes list`: the project's shelf `dir`, then the personal one where
/// this machine places it, each recipe with its origin (#697 ruling 6). A
/// personal shelf with nothing standing at it lists nothing.
pub(crate) fn list(workspace: &Path, dir: &Path, shelf: bool) -> Result<()> {
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
/// the repository: an install onto a shelf outside it copies no agent or
/// charter. The configuration directories made for the shelf are removed
/// again when the install refuses.
pub(crate) fn add(root: &Path, source: &str, name: &str, path: Option<&str>) -> Result<()> {
    let shelf = shelf()?;
    let made = Made::above(&shelf)?;
    super::add(root, &shelf, source, name, path)?;
    made.keep();
    Ok(())
}

/// The directories above the personal shelf an install made, deepest
/// first, removed when dropped unless kept, each only while it is empty.
struct Made(Vec<PathBuf>);

impl Made {
    /// Make the shelf's parent and every directory above it that nothing
    /// stands at, judged as the entry itself, so a link is never removed.
    fn above(shelf: &Path) -> std::io::Result<Made> {
        let parent = shelf.parent().unwrap_or(shelf);
        let missing = parent
            .ancestors()
            .take_while(|dir| matches!(places::entry(dir), Ok(None)))
            .map(Path::to_path_buf)
            .collect();
        std::fs::create_dir_all(parent)?;
        Ok(Made(missing))
    }

    fn keep(mut self) {
        self.0.clear();
    }
}

impl Drop for Made {
    fn drop(&mut self) {
        for dir in &self.0 {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[cfg(test)]
mod personal_tests;
