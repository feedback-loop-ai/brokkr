//! The capability evidence a workspace box reads and never changes
//! (decision 0065 slice two, U8a; CR2, MB3). The engine publishes retained
//! responses at `artifacts/sha256/<hex>` in its run-local directory
//! (decision 0083), which it makes before launch and hands the box as a
//! typed fact, never an authored key: an adopter folder's `.brokkr/state`
//! need not lie under the worktree, a recipe library's `.forge` does.
//! Before a box is built that root is observed here: it is a directory of
//! its own, every published artifact is one regular file with one link,
//! and no writable or overlay mount the box would make is the root, holds
//! it or lies inside it, judged by device and inode on the canonical path.
//! The worktree alone may hold it, at the root's own spelling and through
//! directories of their own. Then `box_argv` mounts each directory between
//! the worktree and the root at its own path, so no write in the box
//! renames it, and the root read-only after every writable mount. A root
//! outside the worktree is mounted nowhere, so the box cannot reach it.
//! Each is a mount that must be made: a box bubblewrap cannot build
//! refuses, never runs unprotected.
//!
//! A pathname cannot see a mount, so on Linux the mount table is read once
//! as well, and one rule covers every mount type. Any other mount of the
//! root's device that shows the root, a directory holding it or anything
//! inside it, a single file included, at a place outside the root's own
//! path refuses, wherever it is mounted ([`shown_elsewhere`]). Then, once
//! every writable source is known, the edge observes what the verdict
//! needs ([`observed`]), and each mount in the box's reach (at or under a
//! source), each that holds a source, and each the root stands on (its
//! own, and any at or inside it) is judged by what backs it ([`reached`]).
//! The root stands on one region of one local filesystem (H7): its own
//! mount an overlay, or any stacking, remote or unproved type, refuses,
//! and so does anything mounted strictly inside it. An overlay in reach
//! that writes into the root, or names an upper or work directory this
//! namespace cannot observe, refuses, and so does any stacking, remote or
//! unknown filesystem in reach whose backing is not proved, and anything
//! that does not parse strictly ([`options`]). Right before the box's argv
//! is final, every directory and writable source observed is observed
//! again, and one that changed refuses.
//!
//! This protects the evidence from a workspace box. Keeping it out of a
//! server box is a separate proof, and every other managed writer is
//! U8a2's.

use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

/// The directory of published artifacts below the root, one file per
/// digest.
const DIGESTS: &str = "sha256";
/// CR2's managed-writer cause, which every refusal here, and the engine's
/// where it cannot make the root, begins with.
pub const UNPROTECTED_EVIDENCE: &str =
    "capability evidence root cannot be protected from every managed writer";
use UNPROTECTED_EVIDENCE as CAUSE;

/// Why a workspace box cannot be built with its evidence protected.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum Unprotected {
    #[error("{CAUSE}: '{}' is not a directory of its own", .0.display())]
    Directory(PathBuf),
    #[error("{CAUSE}: artifact '{}' is not a regular file with a single link", .0.display())]
    Artifact(PathBuf),
    #[error("{CAUSE}: the writable mount '{}' is the artifact root, holds it or lies inside it", .0.display())]
    Alias(PathBuf),
    #[error("{CAUSE}: '{}' cannot be observed: {}", .0.display(), .1)]
    Unobserved(PathBuf, std::io::ErrorKind),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the mount at '{}' shows the artifact root or part of it outside the root's own path", .0.display())]
    Mounted(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the mount table does not prove where '{}' is mounted", .0.display())]
    Unmapped(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the options of the mount at '{}' do not parse", .0.display())]
    Unparsed(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the overlay at '{}' writes into the artifact root, inside it or into a directory holding it", .0.display())]
    Overlaid(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the overlay at '{}' names the layer '{}', which cannot be observed: {}", .0.display(), .1.display(), .2)]
    Unfollowed(PathBuf, PathBuf, std::io::ErrorKind),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the mount at '{}' lies inside the artifact root", .0.display())]
    Nested(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: the mount at '{}' is a stacking, remote or unknown filesystem whose backing cannot be proved apart from the artifact root", .0.display())]
    Stacked(PathBuf),
    #[error("{CAUSE}: '{}' changed between its observation and the box's mounts", .0.display())]
    Changed(PathBuf),
}

impl From<Unprotected> for std::io::Error {
    fn from(unprotected: Unprotected) -> Self {
        std::io::Error::other(unprotected)
    }
}

/// A host directory by what it is, not what names it.
type Identity = (u64, u64);

/// What was observed of the artifact root before a box is built: the root
/// as the engine spells it, each directory from the worktree's child down
/// to the root's parent where the worktree holds it, the root's identity,
/// the identity of every directory that holds it on its canonical path,
/// the root's own included, to observe again before the argv is final,
/// the root and each pinned directory by the identity of its own entry and
/// each writable source excluded by its resolved one, and, on Linux, the
/// mount table as it stood with every writable source's canonical path.
#[derive(Debug)]
pub(super) struct Protected {
    root: PathBuf,
    chain: Option<Vec<PathBuf>>,
    identity: Identity,
    holders: BTreeSet<Identity>,
    fixed: Vec<Fixed>,
    excluded: Vec<(PathBuf, Option<Identity>)>,
    #[cfg(target_os = "linux")]
    mounts: Mounts,
}

/// What the mount table showed when the root was observed: every record,
/// the root's own mount and canonical path, and each writable source's
/// canonical path, the worktree's first, judged by [`reached`] once every
/// source is known.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct Mounts {
    records: Vec<Record>,
    own: u64,
    canonical: PathBuf,
    sources: Vec<PathBuf>,
}

#[cfg(target_os = "linux")]
use super::namespace::sources::host::Record;

/// What the edge observes for the mount verdict, kept apart from it.
#[cfg(target_os = "linux")]
mod observed;
/// The strict reading of a judged mount's options, layers and paths.
#[cfg(target_os = "linux")]
mod options;
/// The verdict on every mount the box reaches or the root stands on.
#[cfg(target_os = "linux")]
mod verdict;

#[cfg(all(test, target_os = "linux"))]
pub(super) use observed::{named, numbered, observed};
#[cfg(all(test, target_os = "linux"))]
pub(super) use observed::{Block, Contained, Facts, Root, Seen, Source};
#[cfg(all(test, target_os = "linux"))]
pub(super) use verdict::{reached, wanted};

#[cfg(test)]
thread_local! {
    /// A test's step at the start of [`Protected::confirm`], the one place
    /// inside `box_argv` where the world can change between the observation
    /// and its confirmation (L1): a test that changes it there fails where
    /// the production call is removed.
    pub(super) static CONFIRMING: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

/// A directory the box mounts at its own path, by its own entry's identity.
type Fixed = (PathBuf, Identity);

impl Protected {
    /// Observe the engine's artifact `root` beside the `workdir` the box
    /// binds read-write. Nothing is made here: a root that is absent,
    /// reached through a link or holding an artifact that is not one
    /// regular file refuses, and so does a worktree that is the root, lies
    /// inside it or holds it under another spelling, and, on Linux, a
    /// mount that shows it elsewhere ([`unaliased`]).
    pub(super) fn observe(root: &Path, workdir: &Path) -> Result<Protected, Unprotected> {
        let root = &std::path::absolute(root).map_err(unobserved(root))?;
        let own = directory(root)?;
        published(&root.join(DIGESTS))?;
        let canonical = std::fs::canonicalize(root).map_err(unobserved(root))?;
        let holders = (canonical.ancestors())
            .map(|holder| std::fs::metadata(holder).map(|found| identity(&found)))
            .collect::<std::io::Result<_>>()
            .map_err(unobserved(&canonical))?;
        // macOS has no bubblewrap box and no mount table this reads
        // (decision 0063): the pathname and identity checks stand there
        // alone, as before.
        #[cfg(target_os = "linux")]
        let mounts = unaliased(&canonical)?;
        let mut protected = Protected {
            root: root.to_path_buf(),
            chain: None,
            identity: own,
            holders,
            fixed: Vec::new(),
            excluded: Vec::new(),
            #[cfg(target_os = "linux")]
            mounts,
        };
        let worktree = std::fs::canonicalize(workdir).map_err(unobserved(workdir))?;
        #[cfg(target_os = "linux")]
        protected.mounts.sources.push(worktree.clone());
        if protected.inside(&worktree)? {
            return Err(Unprotected::Alias(workdir.to_path_buf()));
        }
        let pinned = match root.strip_prefix(workdir) {
            Ok(relative) => Some(chain(workdir, relative, root)?),
            Err(_) if protected.holds(&worktree)? => {
                return Err(Unprotected::Alias(workdir.to_path_buf()))
            }
            Err(_) => None,
        };
        protected.chain =
            (pinned.as_deref()).map(|pinned| pinned.iter().map(|(path, _)| path.clone()).collect());
        // Outermost first, the root last, so a confirm names the first.
        protected.fixed.extend(pinned.into_iter().flatten());
        protected.fixed.push((root.to_path_buf(), own));
        Ok(protected)
    }

    /// Judge, on Linux, every mount in the box's reach, holding a source or
    /// backing the root now that every writable source is known, over the
    /// facts the edge observes once for it ([`observed`], [`reached`]).
    /// Then observe again, right before the box's argv is final, the root,
    /// each pinned directory and each writable source excluded: an entry
    /// that is no longer the one observed, or a source absent then that
    /// resolves now, refuses (L1). bubblewrap resolves each pathname once
    /// more when it mounts it, a window this narrows and cannot close.
    pub(super) fn confirm(&self) -> Result<(), Unprotected> {
        #[cfg(test)]
        if let Some(step) = CONFIRMING.with_borrow_mut(Option::take) {
            step();
        }
        #[cfg(target_os = "linux")]
        {
            let records = &self.mounts.records;
            let root = observed::Root {
                canonical: &self.mounts.canonical,
                own: self.mounts.own,
                holders: &self.holders,
            };
            let facts = observed::observed(records, &root, &self.mounts.sources);
            verdict::reached(records, &root, &facts)?;
        }
        let fixed = (self.fixed.iter()).find(|(path, was)| {
            let found = std::fs::symlink_metadata(path).ok();
            found.map(|found| identity(&found)) != Some(*was)
        });
        let changed = fixed.map(|(path, _)| path).or_else(|| {
            let mut excluded = self.excluded.iter();
            let now = |host: &Path| {
                resolved(host)
                    .ok()
                    .map(|found| found.map(|(_, found)| found))
            };
            let moved = excluded.find(|(host, was)| now(host) != Some(*was));
            moved.map(|(host, _)| host)
        });
        changed.map_or(Ok(()), |path| Err(Unprotected::Changed(path.clone())))
    }

    /// Each directory between the worktree and a root it holds, which the
    /// box mounts at its own path before any declared bind; none where the
    /// worktree does not hold the root.
    pub(super) fn ancestors(&self) -> &[PathBuf] {
        self.chain.as_deref().unwrap_or_default()
    }

    /// The artifact root where the worktree holds it, which the box mounts
    /// read-only after every writable mount; `None` where no mount the box
    /// makes reaches it.
    pub(super) fn held(&self) -> Option<&Path> {
        self.chain.as_ref().map(|_| self.root.as_path())
    }

    /// Refuse a writable or overlay mount of `host` that is the root or one
    /// of its holders, or lies inside the root, by identity: a link or a
    /// second mount naming the same directory is the same directory. An
    /// absent `host` mounts nothing. Either is kept for [`Self::confirm`].
    pub(super) fn exclude(&mut self, host: &Path) -> Result<(), Unprotected> {
        let found = resolved(host)?;
        if let Some((canonical, found)) = &found {
            if self.holders.contains(found) || self.inside(canonical)? {
                return Err(Unprotected::Alias(host.to_path_buf()));
            }
            #[cfg(target_os = "linux")]
            self.mounts.sources.push(canonical.clone());
        }
        let found = found.map(|(_, found)| found);
        self.excluded.push((host.to_path_buf(), found));
        Ok(())
    }

    /// Whether the canonical directory `at` is the root or holds it.
    fn holds(&self, at: &Path) -> Result<bool, Unprotected> {
        let found = std::fs::metadata(at).map_err(unobserved(at))?;
        Ok(self.holders.contains(&identity(&found)))
    }

    /// Whether the canonical path `at` is the root or lies inside it.
    fn inside(&self, at: &Path) -> Result<bool, Unprotected> {
        for holder in at.ancestors() {
            let found = std::fs::metadata(holder).map_err(unobserved(holder))?;
            if identity(&found) == self.identity {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Each directory from `workdir`'s child down to the parent of `root`,
/// which lies `relative` below it: every one a directory of its own, so
/// the root's spelling inside the box is the directory the engine reads.
/// Each comes with its identity, which [`Protected::confirm`] observes
/// again.
fn chain(workdir: &Path, relative: &Path, root: &Path) -> Result<Vec<Fixed>, Unprotected> {
    let mut chain = Vec::new();
    let mut at = workdir.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(Unprotected::Directory(root.to_path_buf()));
        };
        at.push(name);
        chain.push((at.clone(), directory(&at)?));
    }
    // The root itself is mounted read-only, not pinned.
    chain.pop();
    Ok(chain)
}

/// The identity of the directory at `at`, refused where it is absent, a
/// link or another kind of file.
fn directory(at: &Path) -> Result<Identity, Unprotected> {
    let found = std::fs::symlink_metadata(at).map_err(unobserved(at))?;
    if !found.file_type().is_dir() {
        return Err(Unprotected::Directory(at.to_path_buf()));
    }
    Ok(identity(&found))
}

/// Every artifact published under `digests`, a directory of its own where
/// it is present, each one regular file with one link: a second link
/// would be a writable alias the read-only root does not cover. A root
/// that has published nothing yet has no `digests`.
fn published(digests: &Path) -> Result<(), Unprotected> {
    match std::fs::symlink_metadata(digests) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        _ => directory(digests)?,
    };
    for entry in std::fs::read_dir(digests).map_err(unobserved(digests))? {
        let path = entry.map_err(unobserved(digests))?.path();
        let found = std::fs::symlink_metadata(&path).map_err(unobserved(&path))?;
        if !found.file_type().is_file() || found.nlink() != 1 {
            return Err(Unprotected::Artifact(path));
        }
    }
    Ok(())
}

/// Where `host` resolves and the identity of what it names; `None` where
/// it is absent, so it mounts nothing.
fn resolved(host: &Path) -> Result<Option<(PathBuf, Identity)>, Unprotected> {
    let canonical = match std::fs::canonicalize(host) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        resolved => resolved.map_err(unobserved(host))?,
    };
    let found = std::fs::metadata(&canonical).map_err(unobserved(&canonical))?;
    Ok(Some((canonical, identity(&found))))
}

/// The mount table this namespace reads.
#[cfg(target_os = "linux")]
const MOUNTINFO: &str = "/proc/self/mountinfo";

/// Whether the mount table shows the root at `canonical`, or any part of
/// it, outside that path (H2), read once from [`MOUNTINFO`] and judged by
/// [`shown_elsewhere`].
#[cfg(target_os = "linux")]
fn unaliased(canonical: &Path) -> Result<Mounts, Unprotected> {
    unaliased_with(canonical, || std::fs::read(MOUNTINFO))
}

/// [`unaliased`] over the mount table `table` reads, the root's own mount
/// the one the kernel names for it ([`unaliased_on`]).
#[cfg(target_os = "linux")]
pub(super) fn unaliased_with(
    canonical: &Path,
    table: impl FnOnce() -> std::io::Result<Vec<u8>>,
) -> Result<Mounts, Unprotected> {
    unaliased_on(canonical, observed::contained(canonical), table)
}

/// [`unaliased_with`] where the kernel's answer for the root's own mount
/// is `contained`. A root it names none for (a kernel before statx's mount
/// id) is unproved, and one it cannot answer for, or a table that cannot
/// be read, is unobserved; each refuses, the error kind kept. The table is
/// kept for [`reached`].
#[cfg(target_os = "linux")]
pub(super) fn unaliased_on(
    canonical: &Path,
    contained: observed::Contained,
    table: impl FnOnce() -> std::io::Result<Vec<u8>>,
) -> Result<Mounts, Unprotected> {
    let own = match contained {
        observed::Contained::Mount(own) => own,
        observed::Contained::Unobserved(kind) => {
            return Err(Unprotected::Unobserved(canonical.to_path_buf(), kind))
        }
        observed::Contained::Unnamed => return Err(Unprotected::Unmapped(canonical.into())),
    };
    let table = table().map_err(unobserved(Path::new(MOUNTINFO)))?;
    Ok(Mounts {
        records: shown_elsewhere(&table, own, canonical)?,
        own,
        canonical: canonical.to_path_buf(),
        sources: Vec::new(),
    })
}

/// The mount table `table`'s verdict on the root at `canonical`, which
/// lies on the mount `mount`. Each part of the root a mount holds is
/// found: the part below the root's own mount, as that mount's root plus
/// the rest of the way, and the whole of each mount at or inside the
/// root. Every other mount of a part's device whose root holds the part or
/// lies inside it shows it somewhere ([`aliases`]), a file bind as a
/// directory's; one that shows it outside the root's own path refuses,
/// wherever it is mounted. The device is the table's, never the root's
/// own `stat`, which a btrfs subvolume reports apart from its mount's. A
/// table that does not parse, or names no mount `mount` holding
/// `canonical`, proves nothing and refuses, and so does a record of a
/// part's device whose root is not spelled strictly ([`strict`]). The
/// records parsed are returned.
#[cfg(target_os = "linux")]
pub(super) fn shown_elsewhere(
    table: &[u8],
    mount: u64,
    canonical: &Path,
) -> Result<Vec<Record>, Unprotected> {
    use super::namespace::sources::{host::aliases, host::records, LIMITS};
    let unmapped = Unprotected::Unmapped(canonical.to_path_buf());
    let Ok(records) = records(table, LIMITS.mounts) else {
        return Err(unmapped);
    };
    let own = records.iter().find(|record| record.id == mount);
    let held = own.and_then(|own| Some((own, canonical.strip_prefix(&own.point).ok()?)));
    let Some((own, rest)) = held else {
        return Err(unmapped);
    };
    let nested = (records.iter()).filter(|record| record.id != own.id);
    let nested = nested.filter(|record| record.point.starts_with(canonical));
    let parts = std::iter::once((own, own.root.join(rest)))
        .chain(nested.map(|record| (record, record.root.clone())));
    for (record, part) in parts {
        strict(&records, record.dev)?;
        let mut shown = aliases(&records, record.id, record.dev, &part);
        let outside = shown.find_map(|(point, rest)| {
            let place: PathBuf = point.components().chain(rest.components()).collect();
            (!place.starts_with(canonical)).then_some(place)
        });
        if let Some(place) = outside {
            return Err(Unprotected::Mounted(place));
        }
    }
    Ok(records)
}

/// Refuse, by its point, a record of the device `dev` whose root is not a
/// strict path ([`options::path`]): each such root places an alias, and a
/// `.` or `..` component, a NUL or a deleted mark would misplace it (H6).
#[cfg(target_os = "linux")]
fn strict(records: &[Record], dev: (u32, u32)) -> Result<(), Unprotected> {
    use std::os::unix::ffi::OsStrExt;
    let mut same = records.iter().filter(|record| record.dev == dev);
    let loose = same.find(|record| options::path(record.root.as_os_str().as_bytes()).is_none());
    loose.map_or(Ok(()), |record| {
        Err(Unprotected::Unparsed(record.point.clone()))
    })
}

fn identity(found: &std::fs::Metadata) -> Identity {
    (found.dev(), found.ino())
}

fn unobserved(at: &Path) -> impl Fn(std::io::Error) -> Unprotected + '_ {
    move |error| Unprotected::Unobserved(at.to_path_buf(), error.kind())
}
