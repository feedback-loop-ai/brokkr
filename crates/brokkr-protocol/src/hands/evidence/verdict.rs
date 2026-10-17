//! The verdict on every mount a workspace box reaches, holds a writable
//! source in, or keeps its evidence on (decision 0065 slice two, U8a; CR2,
//! MB3), over the recorded mount table and the facts the edge observed
//! for it ([`super::observed`]). It makes no filesystem call: what it does
//! not hold as a fact refuses.

use std::collections::BTreeSet;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use super::observed::{Block, Contained, Facts, Root, Source};
use super::options::{dotted, path, whole, written};
use super::{Record, Unprotected};

/// How a judged mount's filesystem is judged.
enum Kind {
    /// One host.rs proves local, whose device and inode name one object.
    Local,
    Overlay,
    /// A stacking or remote filesystem, whose backing is never proved.
    Stacked,
    /// Any other, proved only by a block device of its own number.
    Unknown,
}

/// The overlay filesystem's names.
const OVERLAY: [&str; 2] = ["overlay", "overlayfs"];

/// Stacking and remote filesystems, whose backing a device and inode do
/// not prove apart from the root; every `fuse.` subtype is one too.
const STACKED: [&str; 12] = [
    "fuse",
    "fuseblk",
    "nfs",
    "nfs4",
    "cifs",
    "smb3",
    "9p",
    "virtiofs",
    "ceph",
    "glusterfs",
    "aufs",
    "unionfs",
];

/// How `record`'s filesystem is judged.
fn kind(record: &Record) -> Kind {
    let filesystem = record.filesystem.as_os_str();
    let named = |names: &[&str]| names.iter().any(|name| filesystem == *name);
    let fused = filesystem.as_bytes().starts_with(b"fuse.");
    match (record.local, named(&OVERLAY), named(&STACKED) | fused) {
        (true, _, _) => Kind::Local,
        (false, true, _) => Kind::Overlay,
        (false, false, true) => Kind::Stacked,
        (false, false, false) => Kind::Unknown,
    }
}

/// Every mount of `records` the verdict judges: (1) one whose point lies
/// at or under a writable source, which the box binds whole with every
/// mount below it; (2) the one each source lies on, as the edge observed
/// it (H4); (3) the root's own mount, and any at or inside the root. A
/// source whose mount was not observed, or names no record, refuses, and
/// so does a record whose point has a `.` or `..` component, which the
/// kernel never writes and no prefix places.
pub(super) fn judged<'r>(
    records: &'r [Record],
    root: &Root<'_>,
    sources: &[Source],
) -> Result<Vec<&'r Record>, Unprotected> {
    let dotted = records
        .iter()
        .find(|record| dotted(record.point.as_os_str().as_bytes()));
    if let Some(record) = dotted {
        return Err(Unprotected::Unparsed(record.point.clone()));
    }
    let holding = (sources.iter())
        .map(|source| contained(records, source))
        .collect::<Result<BTreeSet<u64>, _>>()?;
    let judged = records.iter().filter(|record| {
        let backs = (record.id == root.own) | record.point.starts_with(root.canonical);
        let under = (sources.iter()).any(|source| record.point.starts_with(&source.path));
        backs | under | holding.contains(&record.id)
    });
    Ok(judged.collect())
}

/// The record the mount `source` lies on is, by the id the edge observed.
fn contained(records: &[Record], source: &Source) -> Result<u64, Unprotected> {
    let at = || source.path.clone();
    match source.mount {
        Contained::Unobserved(kind) => Err(Unprotected::Unobserved(at(), kind)),
        Contained::Mount(id) if records.iter().any(|record| record.id == id) => Ok(id),
        Contained::Mount(_) | Contained::Unnamed => Err(Unprotected::Unmapped(at())),
    }
}

/// What the edge must observe for the verdict on `records`: the upper and
/// work directories each judged overlay names, and the source of each
/// judged mount of an unknown type. Nothing is wanted where the judged
/// set or a record's options refuse, which [`reached`] reports.
pub(in crate::hands) fn wanted(
    records: &[Record],
    root: &Root<'_>,
    sources: &[Source],
) -> (BTreeSet<PathBuf>, BTreeSet<PathBuf>) {
    let (mut layers, mut blocks) = (BTreeSet::new(), BTreeSet::new());
    for record in judged(records, root, sources).unwrap_or_default() {
        match kind(record) {
            Kind::Overlay => layers.extend(written(&record.supers).unwrap_or_default()),
            Kind::Unknown => blocks.extend(path(record.source.as_bytes())),
            Kind::Local | Kind::Stacked => {}
        }
    }
    (layers, blocks)
}

/// The verdict on every mount of `records` the box reaches, that holds a
/// writable source, or that backs the `root` ([`judged`]), by what backs
/// each ([`one`]), over the `facts` the edge observed. A same-device
/// alias was refused, wherever it is, by [`super::shown_elsewhere`].
pub(in crate::hands) fn reached(
    records: &[Record],
    root: &Root<'_>,
    facts: &Facts,
) -> Result<(), Unprotected> {
    for record in judged(records, root, &facts.sources)? {
        one(record, root, facts)?;
    }
    Ok(())
}

/// What backs one judged mount: its point, its root and every super
/// option must read strictly. A mount strictly inside the root refuses
/// whatever its type, so the root's region is one region of one
/// filesystem (H7). A local filesystem passes; an overlay is judged by
/// what it writes ([`overlay`]), except as the root's own mount, which
/// refuses like a stacking or remote one; an unknown one passes only where
/// the edge observed its source to be a block device of the mount's own
/// number (H5).
fn one(record: &Record, root: &Root<'_>, facts: &Facts) -> Result<(), Unprotected> {
    let at = || record.point.clone();
    let spelled = [&record.point, &record.root].map(|at| path(at.as_os_str().as_bytes()));
    if spelled.contains(&None) | !whole(&record.supers) {
        return Err(Unprotected::Unparsed(at()));
    }
    if (record.point != root.canonical) & record.point.starts_with(root.canonical) {
        return Err(Unprotected::Nested(at()));
    }
    match (kind(record), record.id == root.own) {
        (Kind::Local, _) => Ok(()),
        (Kind::Overlay, false) => overlay(record, root, facts),
        (Kind::Overlay, true) | (Kind::Stacked, _) => Err(Unprotected::Stacked(at())),
        (Kind::Unknown, _) => {
            let source = path(record.source.as_bytes());
            let block = source.and_then(|source| facts.blocks.get(&source));
            match block {
                Some(Block::Device(number)) if *number == record.dev => Ok(()),
                _ => Err(Unprotected::Stacked(at())),
            }
        }
    }
}

/// A judged overlay the root does not stand on, by what it writes: its
/// upper and work directories must each be observed and none be the root,
/// lie inside it or hold it. It only reads its lower and data layers: a
/// write through it copies the file up into its upper directory first, so
/// one inside the root, or one this namespace cannot see, is never
/// written.
fn overlay(record: &Record, root: &Root<'_>, facts: &Facts) -> Result<(), Unprotected> {
    let at = || record.point.clone();
    let written = written(&record.supers).ok_or_else(|| Unprotected::Unparsed(at()))?;
    for layer in &written {
        let (canonical, identity) = seen(record, layer, facts)?;
        if root.holders.contains(&identity) | overlapping(&canonical, root.canonical) {
            return Err(Unprotected::Overlaid(at()));
        }
    }
    Ok(())
}

/// The directory `layer` of the overlay `record` was observed to be,
/// refused by why it was not.
fn seen(
    record: &Record,
    layer: &Path,
    facts: &Facts,
) -> Result<(PathBuf, super::Identity), Unprotected> {
    let unfollowed = |kind| Unprotected::Unfollowed(record.point.clone(), layer.into(), kind);
    facts.layer(layer).map_err(unfollowed)
}

/// Whether one path is the other or lies inside it.
fn overlapping(one: &Path, other: &Path) -> bool {
    one.starts_with(other) | other.starts_with(one)
}
