//! One removal walk (decision 0104 ruling 8): the hands reaper and, from
//! slice 5c2, the sweep share this home. It descends by descriptor, each
//! directory opened no-follow from the one above, follows no link, crosses
//! no filesystem, makes each directory owner-writable through a descriptor
//! before removing its entries, counts every inode's allocated bytes once,
//! and removes the tree's own root last.
//!
//! The root's parent is opened as a directory and the root by its final
//! name, so the root's own name is never resolved through a link: POSIX
//! resolves `<link>/` and `<link>/..` through the link when the whole path
//! is opened, and both are refused by spelling here. The listing form
//! removes nothing and never changes a mode; a directory it cannot read
//! makes its count a lower bound that names the first unreadable path.

use std::collections::HashSet;
use std::ffi::{CString, OsStr};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use rustix::fs::{
    chmodat, fstat, openat, statat, unlinkat, AtFlags, Dir, FileType, Mode, OFlags, Stat, CWD,
};
use rustix::io::Errno;
use thiserror::Error;

/// A removal refused. The first error the walk met is returned with the
/// path that met it; a tree that was already absent is not an error.
#[derive(Debug, Error)]
pub enum TreeError {
    /// A descriptor, stat, read, write or unlink the walk could not do.
    #[error("removing {}: {cause}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    /// An entry the tree's own filesystem does not hold. A tree is removed
    /// whole or not at all, so the walk refuses rather than leaving a
    /// mount point's contents behind.
    #[error("{} {}", path.display(), foreign(*device, *tree))]
    Foreign {
        path: PathBuf,
        device: u64,
        tree: u64,
    },
}

/// Two removal errors are one when they name the same path and the same
/// cause: an errno for an io failure, the same two devices for a crossing.
/// The walk's tests match variants, and a reaper's kept tree compares equal
/// across two probes.
impl PartialEq for TreeError {
    fn eq(&self, other: &TreeError) -> bool {
        match (self, other) {
            (
                TreeError::Io { path: one, cause },
                TreeError::Io {
                    path: other,
                    cause: other_cause,
                },
            ) => one == other && cause.raw_os_error() == other_cause.raw_os_error(),
            (
                TreeError::Foreign { path, device, tree },
                TreeError::Foreign {
                    path: other,
                    device: other_device,
                    tree: other_tree,
                },
            ) => path == other && device == other_device && tree == other_tree,
            (TreeError::Io { .. }, TreeError::Foreign { .. })
            | (TreeError::Foreign { .. }, TreeError::Io { .. }) => false,
        }
    }
}

impl Eq for TreeError {}

impl TreeError {
    /// The refusal's cause alone, without its path: the reaper's kept line
    /// already names the tree, through `safe`, so the reason must not name
    /// a path a seat wrote inside the tree a second time.
    pub(crate) fn cause(&self) -> String {
        match self {
            TreeError::Io { cause, .. } => cause.to_string(),
            TreeError::Foreign { device, tree, .. } => foreign(*device, *tree),
        }
    }
}

/// The cause after a `Foreign` path: one home for the words `Display` and
/// [`TreeError::cause`] use.
fn foreign(device: u64, tree: u64) -> String {
    format!(
        "is on another filesystem (device {device}, not {tree}); a tree is removed on its own \
         filesystem"
    )
}

/// What a listing counted: the bytes, and the first directory it could not
/// read. When `unreadable` names a path, `bytes` is a lower bound: that
/// directory's own blocks are counted, and nothing below it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counted {
    pub bytes: u64,
    pub unreadable: Option<PathBuf>,
}

/// Remove `tree` whole and return the bytes it freed, or `None` when it was
/// already absent. A partial removal returns the first error and leaves the
/// rest, so the caller records nothing and a later pass tries again.
pub fn remove_tree(tree: &Path) -> Result<Option<u64>, TreeError> {
    walk(tree, true, same_device).map(|counted| counted.map(|counted| counted.bytes))
}

/// What [`remove_tree`] would free, counting the same inodes and removing
/// nothing: the listing form a `--dry-run` paints. A directory the count
/// cannot read makes the figure a lower bound rather than an error; one the
/// walk cannot stat is still the walk's refusal.
pub fn count_tree(tree: &Path) -> Result<Option<Counted>, TreeError> {
    walk(tree, false, same_device)
}

/// The device a stat reports, as a u64 on either supported host: rustix's
/// `st_dev` is a u64 on Linux and an i32 on macOS.
#[cfg(target_os = "linux")]
fn stat_device(stat: &Stat) -> u64 {
    stat.st_dev
}

#[cfg(not(target_os = "linux"))]
fn stat_device(stat: &Stat) -> u64 {
    stat.st_dev as u64
}

/// The device a real filesystem reports, unchanged.
fn same_device(_: &Path, device: u64) -> u64 {
    device
}

/// The directory open every descent uses: never through a link.
fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

/// Opening the root's parent resolves a caller's path as any open does; the
/// root's own final name is the component this walk refuses to resolve
/// through a link, and it is opened with [`dir_flags`] from this descriptor.
fn parent_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC
}

/// The root's parent and final name. A spelling whose final component is a
/// separator, `.` or `..`, or that has no parent, is refused before anything
/// opens: POSIX resolves `<link>/` and `<link>/..` through the link, and a
/// name is the only thing the walk can open no-follow.
fn root_name(tree: &Path) -> Result<(&Path, &OsStr), TreeError> {
    let bytes = tree.as_os_str().as_bytes();
    let last = bytes.rsplit(|byte| *byte == b'/').next().unwrap_or(b"");
    let name = match last {
        [] | [b'.'] | [b'.', b'.'] => return Err(io_error(tree, Errno::INVAL)),
        _ => OsStr::from_bytes(last),
    };
    let parent = match tree.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => return Err(io_error(tree, Errno::INVAL)),
    };
    Ok((parent, name))
}

/// The root's parent as a directory, or `None` when it is already absent so
/// the root is too.
fn open_parent(parent: &Path, tree: &Path) -> Result<Option<OwnedFd>, TreeError> {
    match openat(CWD, parent, parent_flags(), Mode::empty()) {
        Ok(parent) => Ok(Some(parent)),
        Err(Errno::NOENT) => Ok(None),
        Err(cause) => Err(io_error(tree, cause)),
    }
}

/// The root a walk opened, with the parent descriptor and name it was
/// opened from: the root is unlinked from the same parent, never by path.
struct Opened<'a> {
    root: OwnedFd,
    stat: Stat,
    parent: OwnedFd,
    name: &'a OsStr,
}

/// What opening the root found.
enum Root<'a> {
    Opened(Opened<'a>),
    Absent,
    /// A listing's root it can stat but not read: its own blocks are
    /// counted and the count is a lower bound naming it.
    Unreadable(Stat),
}

/// Open the root from its parent, never by its whole path. A removal
/// unseals the root before opening it, so an overlay's 0200 directory is
/// reached; a listing never changes a mode.
fn open_root<'a>(tree: &'a Path, remove: bool) -> Result<Root<'a>, TreeError> {
    let (parent, name) = root_name(tree)?;
    let parent = match open_parent(parent, tree)? {
        Some(parent) => parent,
        None => return Ok(Root::Absent),
    };
    if remove {
        unseal_root(&parent, name, tree)?;
    }
    open_final(parent, name, tree, remove)
}

/// Unseal the root before opening it for removal, refusing an entry that is
/// not a directory. An absent root is left to the open, which answers
/// [`Root::Absent`].
fn unseal_root(parent: &OwnedFd, name: &OsStr, tree: &Path) -> Result<(), TreeError> {
    match statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::Directory => {
            unseal_at(parent, name, tree)
        }
        Ok(_) => Err(io_error(tree, Errno::NOTDIR)),
        Err(Errno::NOENT) => Ok(()),
        Err(cause) => Err(io_error(tree, cause)),
    }
}

/// Open the root by its final name from its parent's descriptor. A listing
/// that is refused the read still counts the root's own blocks and names it
/// as the first unreadable path.
fn open_final<'a>(
    parent: OwnedFd,
    name: &'a OsStr,
    tree: &Path,
    remove: bool,
) -> Result<Root<'a>, TreeError> {
    match openat(&parent, name, dir_flags(), Mode::empty()) {
        Ok(root) => Ok(Root::Opened(Opened {
            stat: io(tree, fstat(&root))?,
            root,
            parent,
            name,
        })),
        Err(Errno::NOENT) => Ok(Root::Absent),
        Err(cause) => match (remove, cause) {
            (false, Errno::ACCESS) => Ok(Root::Unreadable(stat_root(&parent, name, tree)?)),
            _ => Err(io_error(tree, link_errno(cause))),
        },
    }
}

/// The errno a refused open names. macOS answers `ELOOP` for an
/// `O_NOFOLLOW | O_DIRECTORY` open of a link where Linux answers `ENOTDIR`;
/// the listing's line must not depend on the host, so both are the walk's
/// same refusal. The removal answers `ENOTDIR` from its own stat first.
fn link_errno(cause: Errno) -> Errno {
    match cause {
        Errno::LOOP | Errno::NOTDIR => Errno::NOTDIR,
        _ => cause,
    }
}

/// The root's stat, named in whatever refuses it: a listing refused the
/// open still counts the root's own blocks.
fn stat_root(parent: &OwnedFd, name: &OsStr, tree: &Path) -> Result<Stat, TreeError> {
    io(tree, statat(parent, name, AtFlags::SYMLINK_NOFOLLOW))
}

/// One walk's own state.
struct Walk {
    remove: bool,
    bytes: u64,
    seen: HashSet<(u64, u64)>,
    unreadable: Option<PathBuf>,
}

impl Walk {
    fn new(remove: bool) -> Walk {
        Walk {
            remove,
            bytes: 0,
            seen: HashSet::new(),
            unreadable: None,
        }
    }

    /// Count `stat`'s allocated bytes once, keyed by its inode: a hard link
    /// names one inode and frees its blocks once.
    fn count(&mut self, stat: &Stat) {
        if self.seen.insert((stat_device(stat), stat.st_ino)) {
            self.bytes += stat.st_blocks as u64 * 512;
        }
    }

    /// Remember the first directory the listing could not read. A removal
    /// never reaches this: what it cannot read is the walk's error.
    fn unreadable(&mut self, path: &Path) {
        self.unreadable.get_or_insert_with(|| path.to_path_buf());
    }

    fn finish(self) -> Counted {
        Counted {
            bytes: self.bytes,
            unreadable: self.unreadable,
        }
    }
}

/// Walk `tree`, removing it when `remove` is set, and report what the walk
/// counted. `device_of` is the seam the crossing test drives: it may report
/// another device for a planted path, which a real host would show only
/// where a mount sits inside the tree.
fn walk(
    tree: &Path,
    remove: bool,
    device_of: impl Fn(&Path, u64) -> u64,
) -> Result<Option<Counted>, TreeError> {
    let opened = match open_root(tree, remove)? {
        Root::Absent => return Ok(None),
        Root::Unreadable(stat) => {
            let mut walk = Walk::new(false);
            walk.count(&stat);
            walk.unreadable(tree);
            return Ok(Some(walk.finish()));
        }
        Root::Opened(opened) => opened,
    };
    let tree_device = stat_device(&opened.stat);
    let mut walk = Walk::new(remove);
    walk.count(&opened.stat);
    descend(&opened.root, tree, tree_device, &mut walk, &device_of)?;
    if remove {
        io(
            tree,
            unlinkat(&opened.parent, opened.name, AtFlags::REMOVEDIR),
        )?;
    }
    Ok(Some(walk.finish()))
}

/// Walk one directory's entries, deepest first, removing each as it goes
/// when the walk removes. The directory is opened by its parent, never by a
/// path, so no entry can be swapped for a link between the read and the
/// descent.
fn descend(
    dir: &OwnedFd,
    path: &Path,
    tree_device: u64,
    walk: &mut Walk,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<(), TreeError> {
    let names = match entry_names(dir) {
        Ok(names) => names,
        Err(_) if !walk.remove => {
            walk.unreadable(path);
            return Ok(());
        }
        Err(cause) => return Err(io_error(path, cause)),
    };
    for name in names {
        if matches!(name.to_bytes(), b"." | b"..") {
            continue;
        }
        descend_entry(dir, path, &name, tree_device, walk, device_of)?;
    }
    Ok(())
}

/// One entry of `dir`: counted, refused when it crosses a filesystem, and
/// removed when the walk removes. Its path is the directory's and its name,
/// never a path the kernel resolves through a link.
fn descend_entry(
    dir: &OwnedFd,
    path: &Path,
    name: &CString,
    tree_device: u64,
    walk: &mut Walk,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<(), TreeError> {
    let child = path.join(OsStr::from_bytes(name.to_bytes()));
    let stat = io(&child, statat(dir, name, AtFlags::SYMLINK_NOFOLLOW))?;
    let device = device_of(&child, stat_device(&stat));
    if device != tree_device {
        return Err(TreeError::Foreign {
            path: child,
            device,
            tree: tree_device,
        });
    }
    walk.count(&stat);
    if FileType::from_raw_mode(stat.st_mode) == FileType::Directory {
        descend_directory(dir, &child, name, tree_device, walk, device_of)
    } else if walk.remove {
        io(&child, unlinkat(dir, name, AtFlags::empty()))
    } else {
        Ok(())
    }
}

/// Enter one directory entry and remove it last when the walk removes. A
/// listing never changes a mode: an open the kernel refuses with `EACCES` is
/// that entry's own blocks counted (already, above) and the count's lower
/// bound, whether the mode is 0000 or another owner's. That is the one rule
/// the root's unreadable answer uses too.
fn descend_directory(
    dir: &OwnedFd,
    child: &Path,
    name: &CString,
    tree_device: u64,
    walk: &mut Walk,
    device_of: &impl Fn(&Path, u64) -> u64,
) -> Result<(), TreeError> {
    if walk.remove {
        unseal_at(dir, name, child)?;
    }
    let opened = openat(dir, name, dir_flags(), Mode::empty());
    if !walk.remove && matches!(&opened, Err(Errno::ACCESS)) {
        walk.unreadable(child);
        return Ok(());
    }
    let opened = io(child, opened)?;
    descend(&opened, child, tree_device, walk, device_of)?;
    if walk.remove {
        io(child, unlinkat(dir, name, AtFlags::REMOVEDIR))?;
    }
    Ok(())
}

/// Every name in `dir`, or the errno the listing met. The whole listing is
/// one result, so an entry the iterator refuses is the directory's error
/// rather than dropped the way `flatten` drops it.
fn entry_names(dir: &OwnedFd) -> Result<Vec<CString>, Errno> {
    Dir::read_from(dir)?
        .map(|entry| entry.map(|entry| entry.file_name().to_owned()))
        .collect()
}

/// Give the owner read, write and search on `name` in `dir` before the walk
/// opens and reads it: hands overlays leave directories with no bit for
/// their owner. The mode is exactly 0700: the directory is about to be
/// removed, so no group or other bit is worth keeping, and a planted mode
/// cannot put a bit on whatever the name became. A mode SQLite could not
/// open first (0200 has no read) is changed from its parent, so the walk
/// never has to open what it cannot yet read. The mode is set through a
/// descriptor opened no-follow, never by the name: a writer who swaps the
/// name for a link between the stat and the change cannot reach the link's
/// target. The operator's 2026-09-26 ruling covers the tree, and this makes
/// the change itself not follow.
fn unseal_at<P: rustix::path::Arg>(dir: impl AsFd, name: P, path: &Path) -> Result<(), TreeError> {
    set_dir_mode(dir, name, path)
}

/// What an unseal writes: owner read, write and search, and nothing else.
const UNSEALED: Mode = Mode::from_bits_truncate(0o700);

/// Linux opens the directory itself with `O_PATH`, needing no read bit, and
/// changes it through the descriptor's `/proc/self/fd` name: that names the
/// inode the no-follow open reached, so no later swap can redirect it.
#[cfg(target_os = "linux")]
fn set_dir_mode<P: rustix::path::Arg>(
    dir: impl AsFd,
    name: P,
    path: &Path,
) -> Result<(), TreeError> {
    let opened = io(
        path,
        openat(
            dir,
            name,
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ),
    )?;
    let named = format!("/proc/self/fd/{}", opened.as_raw_fd());
    io(path, chmodat(CWD, named, UNSEALED, AtFlags::empty()))
}

/// macOS's `fchmodat` takes `AT_SYMLINK_NOFOLLOW`, so the final component
/// is changed without following it and the name needs no second open.
#[cfg(not(target_os = "linux"))]
fn set_dir_mode<P: rustix::path::Arg>(
    dir: impl AsFd,
    name: P,
    path: &Path,
) -> Result<(), TreeError> {
    io(
        path,
        chmodat(dir, name, UNSEALED, AtFlags::SYMLINK_NOFOLLOW),
    )
}

/// Run one filesystem call and name `path` in whatever it refuses. One
/// helper rather than a `map_err` closure at each call keeps the error
/// arm one function, and the first refusal covers it.
fn io<T>(path: &Path, result: Result<T, Errno>) -> Result<T, TreeError> {
    match result {
        Ok(value) => Ok(value),
        Err(cause) => Err(io_error(path, cause)),
    }
}

/// A rustix errno as the walk's typed error, naming the path it met.
fn io_error(path: &Path, cause: Errno) -> TreeError {
    TreeError::Io {
        path: path.to_path_buf(),
        cause: std::io::Error::from_raw_os_error(cause.raw_os_error()),
    }
}

#[cfg(test)]
mod tests;
