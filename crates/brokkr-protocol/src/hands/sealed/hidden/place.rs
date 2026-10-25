//! Where the secrets store a reach seals lies (decision 0065 slice two,
//! U8h): its one canonical place, resolved once by the engine outside any
//! box and kept for the run (operator rulings 15, 17 and 18), sealed beside
//! the spelling the engine configured it at, and the box's check that the
//! sealed path still resolves to itself. Moved out of `hidden.rs`, which
//! the store's judgments outgrew.

// `readlink` of an entry that is no link is EINVAL.
use std::io::ErrorKind::{InvalidInput as NotLink, NotFound};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Unhidden;

/// The secrets store a reach seals: its canonical place, and the absolute
/// spelling the engine configured it at, which must still lead there
/// (operator ruling 17 of 2026-10-25).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::hands) struct Store {
    place: PathBuf,
    spelling: PathBuf,
}

impl Store {
    /// The store the engine keeps at `place`, configured at `spelling`, as
    /// it hands them: the place it resolved once for the run, never
    /// resolved again here (ruling 18).
    pub(in crate::hands) fn sealed((place, spelling): (&Path, &Path)) -> Store {
        let (place, spelling) = (place.to_path_buf(), spelling.to_path_buf());
        Store { place, spelling }
    }

    /// The store's canonical place.
    pub(in crate::hands) fn place(&self) -> &Path {
        &self.place
    }

    /// Refuse the store where its configured spelling, resolved again now,
    /// no longer leads to its sealed place: a route on the spelling changed
    /// since it was sealed, so the store the engine and `brokkr secrets`
    /// use now lies where no box would hide it (ruling 17); or where the
    /// spelling no longer resolves at all, by the error it is (ruling 18).
    pub(in crate::hands) fn led(&self) -> Result<(), Unhidden> {
        match store_place(&self.spelling) {
            Ok(now) if now == self.place => Ok(()),
            Ok(_) => Err(Unhidden::Strayed(self.spelling.clone(), self.place.clone())),
            Err(error) => Err(Unhidden::Unobserved(self.spelling.clone(), error.kind())),
        }
    }
}

/// The canonical place of the secrets store the engine spells at `store`,
/// which the engine resolves once for a run and keeps, to make the store
/// at, name it to a seat by and seal it in a box's reach by (rulings 15, 17
/// and 18): made absolute, and walked one entry at a time with every
/// symbolic link on it resolved, the final entry's included ([`walked`]).
/// A store not yet made, the final entry alone, is its place as spelled in
/// the folder it resolved, which the engine makes it in. Every folder on
/// the way must stand and be passed as Linux's lookup passes it, `..` too
/// (ruling 18; run 6's L1): a missing folder, a file, a folder it may not
/// search or more links than Linux follows is the error it is, and nothing
/// stands in for the place.
pub fn store_place(store: &Path) -> std::io::Result<PathBuf> {
    walked(&std::path::absolute(store)?, true, &mut 0)
}

/// The most symbolic links one walk follows, as Linux's own lookup does.
const LINKS: usize = 40;

/// The real place of the absolute `path`, walked one entry at a time, each
/// symbolic link followed and counted in `links`. Its final entry may be
/// missing only where `open`, and is then as spelled; any other missing
/// entry, an entry no lookup passes, or more links than Linux follows, is
/// the error it is.
fn walked(path: &Path, open: bool, links: &mut usize) -> std::io::Result<PathBuf> {
    let mut at = PathBuf::from("/");
    let mut parts = path.components().peekable();
    while let Some(part) = parts.next() {
        let last = open & parts.peek().is_none();
        let name = match part {
            Component::Normal(name) => name,
            Component::ParentDir => {
                std::fs::symlink_metadata(at.join(".."))?;
                at.pop();
                continue;
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => continue,
        };
        let next = at.join(name);
        at = match std::fs::read_link(&next) {
            Ok(_) if *links == LINKS => return Err(rustix::io::Errno::LOOP.into()),
            Ok(target) => {
                *links += 1;
                walked(&at.join(target), last, links)?
            }
            Err(error) if error.kind() == NotLink => next,
            Err(error) if (error.kind() == NotFound) & last => next,
            Err(error) => return Err(error),
        };
    }
    Ok(at)
}

/// Refuse the sealed `store` where it no longer resolves to itself: not
/// absolute and plain, or with a symbolic link at any entry on it that
/// stands, the final one's included (ruling 15). The entries below the
/// first missing one are absent, and so is the store.
pub(in crate::hands) fn unlinked(store: &Path) -> Result<(), Unhidden> {
    let rest = store.strip_prefix("/").ok();
    let mut parts = rest.into_iter().flat_map(Path::components);
    let plain = (rest.is_some()) & parts.all(|part| matches!(part, Component::Normal(_)));
    plain
        .then_some(())
        .ok_or_else(|| Unhidden::Unresolved(store.to_path_buf()))?;
    let mut entries: Vec<&Path> = store.ancestors().collect();
    entries.reverse();
    for at in entries.into_iter().skip(1) {
        match std::fs::symlink_metadata(at) {
            Ok(found) if found.is_symlink() => {
                return Err(Unhidden::Routed(store.to_path_buf(), at.to_path_buf()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == NotFound => break,
            Err(error) => return Err(Unhidden::Unobserved(store.to_path_buf(), error.kind())),
        }
    }
    Ok(())
}
