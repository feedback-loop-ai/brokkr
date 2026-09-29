//! The workspace root and a reader for files under it, for the test
//! binaries that judge the repository's own files. Each includes this
//! through `#[path]`, so the helpers have one home.

use std::path::PathBuf;

/// The workspace root: two levels above this crate's manifest.
pub(crate) fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// A file under the workspace root, read whole; a missing file panics
/// with its path.
pub(crate) fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}
