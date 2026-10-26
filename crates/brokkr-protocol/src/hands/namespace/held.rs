//! A box's sources mounted from the handles observed, never their names
//! (decision 0065 slice two, U6c5a and U8k; MB3): each recorded mount whose
//! source is held is rewritten to a descriptor mount as writable as the
//! named one, and an overlay's lower layer to the descriptor's own name,
//! so the box holds the object observed, whatever its path names by the
//! time bubblewrap runs.

use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};

use super::super::sealed::Drifted;
use super::{Bind, Layer, Mount, Namespace};
use crate::broker::Refusal;

/// What one recorded mount becomes once the box's handles are in.
enum Placed<'h> {
    /// Mounted by its name, as recorded: no source of the box's.
    Kept,
    /// Mounted from this handle.
    Held(&'h OwnedFd),
    /// Not mounted: an optional source that stood nowhere.
    Dropped,
}

/// The descriptors a rewrite mounts: each number the argv names, and the
/// duplicate made for each source mounted again, by its host path.
type Mounted = (Vec<RawFd>, Vec<(PathBuf, OwnedFd)>);

impl Mount {
    /// The flag mounting a source from a descriptor, read-only exactly
    /// where the named flag is.
    pub(super) const fn descriptor(self) -> &'static str {
        match self {
            Mount::Bind | Mount::BindTry => "--bind-fd",
            Mount::RoBind | Mount::RoBindTry => "--ro-bind-fd",
        }
    }
}

impl Namespace {
    /// Mount every host source from its observed handle (`--ro-bind-fd`),
    /// not its name, dropping an optional source with none: the box holds
    /// the object observed, whatever its path names later (MB3). bubblewrap
    /// closes each descriptor it mounts, so a source mounted twice (a
    /// system source and its alias) gets a duplicate, returned for the box
    /// to hold; one that cannot be made leaves identity unprotected.
    pub(in crate::hands) fn backed(
        &mut self,
        handles: &[(PathBuf, OwnedFd)],
    ) -> Result<Vec<(PathBuf, OwnedFd)>, Refusal> {
        let placed = |host: &Path| match handles.iter().find(|(held, _)| held == host) {
            Some((_, fd)) => Placed::Held(fd),
            None => Placed::Dropped,
        };
        let mounted = self.descriptors(placed).or(Err(Refusal::Identity))?;
        Ok(mounted.1)
    }

    /// Mount each of a workspace box's sealed `sources` from the handle
    /// opened at its last observation (U8k), `--bind-fd` where it is
    /// writable and `--ro-bind-fd` where it is not, and an overlay's lower
    /// layer as [`Bind::held`] says; a bind sealed absent is not mounted at
    /// all, and an overlay over a source sealed absent refuses, as its
    /// source cannot be opened. Every other mount keeps its name. The
    /// handles the argv names are returned, each source mounted again by a
    /// duplicate, for the box to carry into bubblewrap and nothing else: a
    /// handle the argv does not mount would reach the box's command open.
    pub(in crate::hands) fn held(
        &mut self,
        sources: Vec<(PathBuf, Option<OwnedFd>)>,
    ) -> std::io::Result<Vec<(PathBuf, OwnedFd)>> {
        let placed = |host: &Path| match sources.iter().find(|(source, _)| source == host) {
            Some((_, Some(fd))) => Placed::Held(fd),
            Some((_, None)) => Placed::Dropped,
            None => Placed::Kept,
        };
        let (used, mut held) = self.descriptors(placed)?;
        let opened = sources
            .into_iter()
            .filter_map(|(path, fd)| Some((path, fd?)));
        held.extend(opened.filter(|(_, fd)| used.contains(&fd.as_raw_fd())));
        Ok(held)
    }

    /// Rewrite each recorded mount as `placed` decides for its host source,
    /// in place: the box's mount order is its boundary.
    fn descriptors<'h>(
        &mut self,
        placed: impl Fn(&Path) -> Placed<'h>,
    ) -> std::io::Result<Mounted> {
        let mut argv = Vec::with_capacity(self.argv.len());
        let (mut from, mut used, mut duplicates) = (0, Vec::new(), Vec::new());
        for bind in std::mem::take(&mut self.binds) {
            argv.extend_from_slice(&self.argv[from..bind.at]);
            from = bind.at + bind.span();
            let fd = match (placed(&bind.host), bind.layer) {
                (Placed::Kept, _) => {
                    argv.extend_from_slice(&self.argv[bind.at..from]);
                    continue;
                }
                (Placed::Dropped, Layer::Mounted(_)) => continue,
                (Placed::Dropped, Layer::Lower) => {
                    let kind = std::io::ErrorKind::NotFound;
                    return Err(Drifted::Unopened(bind.host, kind).into());
                }
                (Placed::Held(fd), _) => fd,
            };
            let mut number = fd.as_raw_fd();
            if used.contains(&number) {
                let duplicate = fd.try_clone()?;
                number = duplicate.as_raw_fd();
                duplicates.push((bind.host.clone(), duplicate));
            }
            used.push(number);
            argv.extend(bind.held(number));
        }
        argv.extend_from_slice(&self.argv[from..]);
        self.argv = argv;
        Ok((used, duplicates))
    }
}

impl Bind {
    /// How many words of the argv name this source: a bind's flag, source
    /// and target, or an overlay's `--overlay-src` and its lower layer.
    fn span(&self) -> usize {
        match self.layer {
            Layer::Mounted(_) => 3,
            Layer::Lower => 2,
        }
    }

    /// The words mounting this source from the descriptor `number`
    /// instead: a bind as writable as the named one; an overlay's lower
    /// layer read through the descriptor's name in bubblewrap's own
    /// process, which bubblewrap resolves to the object the descriptor
    /// holds, and that descriptor mounted read-only beneath the overlay at
    /// its target first: bubblewrap closes only a descriptor it mounts, and
    /// one it only read would reach the box's command open, a handle on a
    /// host directory outside the box (U8k).
    fn held(&self, number: RawFd) -> Vec<String> {
        let (mount, lower) = match self.layer {
            Layer::Mounted(mount) => (mount, None),
            Layer::Lower => {
                let named = format!("/proc/self/fd/{number}");
                (Mount::RoBind, Some(["--overlay-src".to_string(), named]))
            }
        };
        let mut words = vec![
            mount.descriptor().to_string(),
            number.to_string(),
            self.target.clone(),
        ];
        words.extend(lower.into_iter().flatten());
        words
    }
}
