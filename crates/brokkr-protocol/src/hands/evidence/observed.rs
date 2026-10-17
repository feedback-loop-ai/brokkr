//! What the edge observes for the mount verdict (decision 0065 slice two,
//! U8a; H4, H5, SC4): once, before [`super::verdict::reached`] reads it,
//! the mount each writable source lies on, each layer a judged overlay
//! names, and the source of each judged mount of an unknown type. Every
//! failure is kept as the fact it is, its error kind whole, and the
//! verdict refuses on it; the verdict itself makes no filesystem call.

use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::{identity, Identity, Record};

/// The artifact root as the verdict reads it: its canonical path, the
/// mount the kernel names for it, and the identity of every directory on
/// that path that holds it, its own included.
pub(in crate::hands) struct Root<'r> {
    pub(in crate::hands) canonical: &'r Path,
    pub(in crate::hands) own: u64,
    pub(in crate::hands) holders: &'r BTreeSet<Identity>,
}

/// A writable source by its canonical path, and the mount it lies on as
/// observed.
#[derive(Debug, Clone)]
pub(in crate::hands) struct Source {
    pub(in crate::hands) path: PathBuf,
    pub(in crate::hands) mount: Contained,
}

/// The mount an object lies on, as the kernel answers for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Contained {
    /// The mount id the kernel names.
    Mount(u64),
    /// An answer that carries no mount id.
    Unnamed,
    /// No answer, and why.
    Unobserved(ErrorKind),
}

/// A layer as observed: the directory it resolves to, canonical, and its
/// identity, or why it does not resolve to one.
pub(in crate::hands) type Seen = Result<(PathBuf, Identity), ErrorKind>;

/// What a mount's source is, observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Block {
    /// A block device of this major and minor number.
    Device((u32, u32)),
    /// Something other than a block device.
    Other,
    /// Nothing observed, and why.
    Unobserved(ErrorKind),
}

/// Everything the verdict reads beside the table: each writable source,
/// and each layer and each source the edge observed for it.
#[derive(Debug, Default)]
pub(in crate::hands) struct Facts {
    pub(in crate::hands) sources: Vec<Source>,
    pub(in crate::hands) layers: BTreeMap<PathBuf, Seen>,
    pub(in crate::hands) blocks: BTreeMap<PathBuf, Block>,
}

impl Facts {
    /// What was observed of `layer`: a layer the edge never observed is
    /// one it never found.
    pub(super) fn layer(&self, layer: &Path) -> Seen {
        let seen = self.layers.get(layer).cloned();
        seen.unwrap_or(Err(ErrorKind::NotFound))
    }
}

/// Observe, for the verdict on `records` beside the `root`, the mount each
/// of the canonical writable `sources` lies on, then each layer and
/// source the judged mounts need ([`super::verdict::wanted`]).
pub(in crate::hands) fn observed(
    records: &[Record],
    root: &Root<'_>,
    sources: &[PathBuf],
) -> Facts {
    let sources: Vec<Source> = (sources.iter())
        .map(|path| Source {
            path: path.clone(),
            mount: contained(path),
        })
        .collect();
    let (layers, blocks) = super::verdict::wanted(records, root, &sources);
    let layers = layers.into_iter().map(|at| (seen(&at), at));
    let blocks = blocks.into_iter().map(|at| (block(&at), at));
    Facts {
        layers: layers.map(|(seen, at)| (at, seen)).collect(),
        blocks: blocks.map(|(block, at)| (at, block)).collect(),
        sources,
    }
}

/// The mount the object at `at` lies on, by its own entry, as `statx`
/// names it.
pub(super) fn contained(at: &Path) -> Contained {
    use rustix::fs::{statx, AtFlags, StatxFlags, CWD};
    match statx(CWD, at, AtFlags::SYMLINK_NOFOLLOW, StatxFlags::MNT_ID) {
        Err(errno) => Contained::Unobserved(kind(errno.into())),
        Ok(held) => named(held.stx_mask & StatxFlags::MNT_ID.bits(), held.stx_mnt_id),
    }
}

/// The mount a `statx` answer names: its id where the answer's `mask`
/// carries one, which every kernel since 5.8 sets, and none otherwise.
pub(in crate::hands) fn named(mask: u32, id: u64) -> Contained {
    (mask != 0)
        .then_some(id)
        .map_or(Contained::Unnamed, Contained::Mount)
}

/// The directory `layer` resolves to here, or why it does not resolve to
/// one.
fn seen(layer: &Path) -> Seen {
    let canonical = std::fs::canonicalize(layer).map_err(kind)?;
    let found = std::fs::metadata(&canonical).map_err(kind)?;
    match found.is_dir() {
        true => Ok((canonical, identity(&found))),
        false => Err(ErrorKind::NotADirectory),
    }
}

/// What `source` is, followed through any link as `/dev/mapper` names are
/// ([`numbered`]), or why it cannot be observed.
fn block(source: &Path) -> Block {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    match std::fs::metadata(source) {
        Err(error) => Block::Unobserved(kind(error)),
        Ok(found) => numbered(found.file_type().is_block_device(), found.rdev()),
    }
}

/// A source by what it is: a block device by its `number`, split into its
/// major and minor, or something else.
pub(in crate::hands) fn numbered(device: bool, number: u64) -> Block {
    match device {
        true => Block::Device((rustix::fs::major(number), rustix::fs::minor(number))),
        false => Block::Other,
    }
}

/// Why an observation failed, kept whole.
fn kind(error: std::io::Error) -> ErrorKind {
    error.kind()
}
