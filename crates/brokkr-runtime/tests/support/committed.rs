//! A fixture repository with a head, beside `git.rs`'s, whose isolated
//! `git` it runs. Its own file, so a test binary that needs no head
//! includes none of it (fix round 0109a1r3).

use std::path::Path;

use super::git::{run, test_repository};

/// `dir` made a real git repository with one empty commit on its head, so a
/// run's protected review there is checked on a head it can read (decision
/// 0109 ruling 7, fix round 0109a1r3, H1): a review in a directory with no
/// commit fails closed.
pub(crate) fn committed_repository(dir: &Path) {
    test_repository(dir);
    let identity = [
        "-c",
        "user.name=Brokkr Test",
        "-c",
        "user.email=brokkr@test",
    ];
    let commit = [
        "-c",
        "commit.gpgSign=false",
        "commit",
        "-q",
        "--allow-empty",
    ];
    let args = [&identity[..], &commit, &["--no-verify", "-m", "base"]].concat();
    let out = run(dir, &args);
    assert!(
        out.status.success(),
        "git commit at {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}
