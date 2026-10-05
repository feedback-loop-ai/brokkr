//! The text of every file git tracks, for the test binaries that read the
//! whole tree. Each includes this through `#[path]` beside `tracked.rs` as
//! `tracked_files`, so the reading rule has one home: a file that is tracked
//! but cannot be read fails the check, and one that is tracked and deleted
//! from the working tree is not there to hold anything.

use std::path::Path;

/// Each tracked path under `root` with its text, invalid UTF-8 replaced.
pub(crate) fn texts(root: &Path) -> impl Iterator<Item = (String, String)> + '_ {
    super::tracked_files::tracked(root, &[])
        .into_iter()
        .filter_map(|path| match std::fs::read(root.join(&path)) {
            Ok(bytes) => Some((path, String::from_utf8_lossy(&bytes).into_owned())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("{path} is tracked and cannot be read: {error}"),
        })
}
