//! The workspace root and a reader for files under it, for the test
//! files that judge the repository's own files. `tests/it.rs` declares
//! this once, so the helpers have one home.

use std::path::PathBuf;

/// The one checked entry point a workflow's or a script's one-binary test
/// command runs through, from the workspace root (#543).
pub(crate) const GUARD: &str = "scripts/run-it-tests.sh";

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
