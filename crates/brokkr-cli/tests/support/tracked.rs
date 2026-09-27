//! The files git tracks, for the test binaries that read the repository's
//! own tree. Each includes this through `#[path]`, so the listing has one
//! home.

use std::path::Path;

/// Every tracked path under `root` that `pathspec` matches (all of them
/// when it is empty), as `git ls-files` lists it.
pub(crate) fn tracked(root: &Path, pathspec: &[&str]) -> Vec<String> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["ls-files", "-z", "--"])
        .args(pathspec)
        .output()
        .expect("git ls-files");
    assert!(output.status.success(), "git ls-files failed: {output:?}");
    String::from_utf8(output.stdout)
        .expect("UTF-8 paths")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}
