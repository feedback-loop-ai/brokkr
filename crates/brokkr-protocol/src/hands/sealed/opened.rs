//! Each sealed source opened once, at the box's last observation (decision
//! 0065 slice two, U8k; SI1, MB3): a handle on the very object the reach
//! sealed, its device and inode as the handle itself reports them, which
//! the box mounts from, so bubblewrap never resolves the source's path
//! again and an object put at that path afterwards is not the one mounted.
//! On Linux the handle is a path handle, which reads nothing of its object.
//!
//! What the box mounts from inside a sealed source, Git's `config`, a
//! folder it pins or the artifact root it holds read-only ([`Beneath`]),
//! is opened the same way from the handle of the source that holds it,
//! never from a path resolved again: on Linux one `openat2` beneath that
//! handle that follows no link (Linux 5.6), and held to what the box
//! observed there, or refused.

use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use super::{Drifted, Sealed, Source};

#[cfg(test)]
thread_local! {
    /// A test's step once [`Sealed::opened`] holds every handle: the window
    /// between the box's last observation and bubblewrap's mounts, where a
    /// box mounting a source by its path would mount whatever stands there.
    pub(in crate::hands) static OPENED: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

/// A source a box mounts from inside a sealed one (U8k): `host`, as the
/// box's mount names it, lies `rest` below the sealed source `within`,
/// and its handle must be what `expected` says.
#[derive(Debug)]
pub(in crate::hands) struct Beneath {
    pub(in crate::hands) host: PathBuf,
    pub(in crate::hands) within: PathBuf,
    pub(in crate::hands) rest: PathBuf,
    pub(in crate::hands) expected: Expected,
}

/// What a handle opened beneath a sealed source must hold.
#[derive(Debug, Clone, Copy)]
pub(in crate::hands) enum Expected {
    /// The object the box's observation saw there, by device and inode;
    /// none where it saw nothing.
    Seen(Option<(u64, u64)>),
    /// One regular file, which the box mounts read-only over itself: Git's
    /// `config` (decision 0043 ruling 6).
    File,
}

impl Sealed {
    /// A handle on each source sealed, by its canonical path, in sealed
    /// order: none for a source sealed where nothing stood, which the box
    /// does not mount. A source that cannot be opened now, or whose handle
    /// is not the object sealed, refuses with MB3's identity cause.
    pub(in crate::hands) fn opened(&self) -> Result<Vec<(PathBuf, Option<OwnedFd>)>, Drifted> {
        let sources = self.sources().into_iter();
        let held = sources.map(|source| Ok((source.path.clone(), held(source)?)));
        let held = held.collect::<Result<Vec<_>, Drifted>>()?;
        #[cfg(test)]
        if let Some(step) = OPENED.with_borrow_mut(Option::take) {
            step();
        }
        Ok(held)
    }
}

/// A handle on each of `beneath`, by its host path, opened from the handle
/// `opened` holds for the sealed source it lies in: refused, by its host
/// path, where that source has none or it does not open beneath it, and
/// where it is not what the box expects there.
pub(in crate::hands) fn beneath(
    opened: &[(PathBuf, Option<OwnedFd>)],
    beneath: Vec<Beneath>,
) -> Result<Vec<(PathBuf, Option<OwnedFd>)>, Drifted> {
    let mut held = Vec::with_capacity(beneath.len());
    for inner in beneath {
        let within = opened.iter().find(|(path, _)| *path == inner.within);
        let within = within.and_then(|(_, fd)| fd.as_ref());
        let within = within.ok_or(std::io::Error::from(std::io::ErrorKind::NotFound));
        let opened = within.and_then(|within| below(within, &inner.rest));
        let unopened = |error: std::io::Error| Drifted::Unopened(inner.host.clone(), error.kind());
        let (fd, now, file) = opened.and_then(identified).map_err(unopened)?;
        let fits = match inner.expected {
            Expected::Seen(seen) => (seen == Some(now)).then_some(()),
            Expected::File => file.then_some(()),
        };
        fits.ok_or_else(|| inner.drifted())?;
        held.push((inner.host, Some(fd)));
    }
    Ok(held)
}

impl Beneath {
    /// Why its handle is refused: not the object observed, or not one
    /// regular file.
    fn drifted(&self) -> Drifted {
        match self.expected {
            Expected::Seen(_) => Drifted::Moved(self.host.clone()),
            Expected::File => Drifted::Unheld(self.host.clone()),
        }
    }
}

/// `rest` opened beneath the directory `within` holds, no link followed
/// and nothing above `within` reached.
#[cfg(target_os = "linux")]
fn below(within: &OwnedFd, rest: &Path) -> std::io::Result<OwnedFd> {
    use rustix::fs::{Mode, OFlags, ResolveFlags};
    let flags = OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let resolve = ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS;
    let opened = rustix::fs::openat2(within, rest, flags, Mode::empty(), resolve);
    opened.map_err(std::io::Error::from)
}

/// Off Linux no bubblewrap runs and nothing is mounted from the handle,
/// which is opened read-only, its last entry no link.
#[cfg(not(target_os = "linux"))]
fn below(within: &OwnedFd, rest: &Path) -> std::io::Result<OwnedFd> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let opened = rustix::fs::openat(within, rest, flags, Mode::empty());
    opened.map_err(std::io::Error::from)
}

/// A handle on `source`'s object where one was sealed: refused where its
/// path no longer opens, or opens another object.
fn held(source: &Source) -> Result<Option<OwnedFd>, Drifted> {
    let Some(sealed) = source.identity else {
        return Ok(None);
    };
    let path = &source.path;
    let unopened = |error: std::io::Error| Drifted::Unopened(path.clone(), error.kind());
    let (fd, now, _) = handle(path).and_then(identified).map_err(unopened)?;
    (now == sealed)
        .then_some(Some(fd))
        .ok_or_else(|| Drifted::Moved(path.clone()))
}

/// `path` opened.
fn handle(path: &Path) -> std::io::Result<OwnedFd> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    options.custom_flags(libc::O_PATH);
    options.open(path).map(OwnedFd::from)
}

/// `fd` beside the device and inode of the object it holds, and whether
/// that is one regular file.
fn identified(fd: OwnedFd) -> std::io::Result<(OwnedFd, (u64, u64), bool)> {
    let file = std::fs::File::from(fd);
    let found = file.metadata();
    found.map(|found| (file.into(), (found.dev(), found.ino()), found.is_file()))
}
