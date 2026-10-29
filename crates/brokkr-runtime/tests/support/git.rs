//! A real git repository at a fixture's root, shared by the unit and
//! integration fixtures that mark a workspace root with `.git` (#908): a
//! bundle's walk asks git in the repository it stands in, so a bare `.git`
//! directory is a refusal to answer, not a marker.

use std::path::Path;

/// `dir` made a real git repository, creating the directory when needed.
pub(crate) fn test_repository(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let out = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .output()
        .expect("git init");
    assert!(
        out.status.success(),
        "git init -q at {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}
