//! One removal walk (decision 0104 ruling 8): the hands reaper and, from
//! slice 5c2, the sweep share this home. It descends by descriptor, each
//! directory opened no-follow from the one above, follows no link, crosses
//! no filesystem, makes each directory owner-writable before removing its
//! entries, counts every inode's allocated bytes once, and removes the
//! tree's own root last.

use std::collections::HashSet;
use std::ffi::{CString, OsStr};
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
    #[error(
        "{} is on another filesystem (device {device}, not {tree}); a tree is removed on its \
         own filesystem",
        path.display()
    )]
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

/// Remove `tree` whole and return the bytes it freed, or `None` when it was
/// already absent. A partial removal returns the first error and leaves the
/// rest, so the caller records nothing and a later pass tries again.
pub fn remove_tree(tree: &Path) -> Result<Option<u64>, TreeError> {
    walk(tree, true, same_device)
}

/// The bytes [`remove_tree`] would free, counting the same inodes and
/// removing nothing: the listing form a `--dry-run` paints.
pub fn count_tree(tree: &Path) -> Result<Option<u64>, TreeError> {
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

/// One walk's own state.
struct Walk {
    remove: bool,
    bytes: u64,
    seen: HashSet<(u64, u64)>,
}

impl Walk {
    /// Count `stat`'s allocated bytes once, keyed by its inode: a hard link
    /// names one inode and frees its blocks once.
    fn count(&mut self, stat: &Stat) {
        if self.seen.insert((stat_device(stat), stat.st_ino)) {
            self.bytes += stat.st_blocks as u64 * 512;
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
) -> Result<Option<u64>, TreeError> {
    if remove {
        match statat(CWD, tree, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::Directory => {
                unseal_at(CWD, tree, tree, stat.st_mode)?;
            }
            Ok(_) => return Err(io_error(tree, Errno::NOTDIR)),
            Err(Errno::NOENT) => return Ok(None),
            Err(cause) => return Err(io_error(tree, cause)),
        }
    }
    let root = match openat(CWD, tree, dir_flags(), Mode::empty()) {
        Ok(root) => root,
        Err(Errno::NOENT) => return Ok(None),
        Err(cause) => return Err(io_error(tree, cause)),
    };
    let stat = io(tree, fstat(&root))?;
    let tree_device = stat_device(&stat);
    let mut walk = Walk {
        remove,
        bytes: 0,
        seen: HashSet::new(),
    };
    walk.count(&stat);
    descend(&root, tree, tree_device, &mut walk, &device_of)?;
    if remove {
        io(tree, unlinkat(CWD, tree, AtFlags::REMOVEDIR))?;
    }
    Ok(Some(walk.bytes))
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
    let entries = io(path, Dir::read_from(dir))?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            (!matches!(name.to_bytes(), b"." | b"..")).then(|| name.to_owned())
        })
        .collect::<Vec<CString>>();
    for name in entries {
        let child = path.join(OsStr::from_bytes(name.to_bytes()));
        let stat = io(&child, statat(dir, &name, AtFlags::SYMLINK_NOFOLLOW))?;
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
            if walk.remove {
                unseal_at(dir, &name, &child, stat.st_mode)?;
            }
            let opened = io(&child, openat(dir, &name, dir_flags(), Mode::empty()))?;
            descend(&opened, &child, tree_device, walk, device_of)?;
            if walk.remove {
                io(&child, unlinkat(dir, &name, AtFlags::REMOVEDIR))?;
            }
        } else if walk.remove {
            io(&child, unlinkat(dir, &name, AtFlags::empty()))?;
        }
    }
    Ok(())
}

/// Give the owner read, write and search on `name` in `dir` before the walk
/// opens and reads it: hands overlays leave directories with no bit for
/// their owner. The other bits stay as they were. A mode SQLite could not
/// open first (0200 has no read) is changed from its parent, so the walk
/// never has to open what it cannot yet read.
fn unseal_at<P: rustix::path::Arg>(
    dir: impl AsFd,
    name: P,
    path: &Path,
    mode: u32,
) -> Result<(), TreeError> {
    let mode = Mode::from_bits_truncate((mode & 0o7777) | 0o700);
    io(path, chmodat(dir, name, mode, AtFlags::empty()))
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
