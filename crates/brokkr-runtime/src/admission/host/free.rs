//! The free-space probe production admission measures a floor with,
//! moved whole from `host.rs` so the loader's file keeps its ceiling.

use std::io::{self, ErrorKind};
use std::path::Path;

/// The bytes free to an unprivileged writer on the filesystem holding
/// `path`, or that would hold it once made: that of its deepest component
/// that is there, since a seat's scratch tree is made when its attempt
/// starts. Only a component with nothing at it is passed over: one that
/// is a link that dangles or loops is there, and cannot be measured,
/// never measured on its parent's filesystem. The free-space probe
/// production admission measures with.
pub fn free_bytes(path: &Path) -> io::Result<u64> {
    let there = (path.ancestors()).find_map(|at| match std::fs::symlink_metadata(at) {
        Err(absent) if absent.kind() == ErrorKind::NotFound => None,
        reached => Some(reached.map(|_| at)),
    });
    let stat = rustix::fs::statvfs(there.unwrap_or(Ok(path))?)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}
