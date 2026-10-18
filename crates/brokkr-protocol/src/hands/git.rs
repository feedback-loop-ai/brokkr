//! What the host's Git says about a worktree, asked outside the box
//! (decision 0043 ruling 6). Moved out of `hands.rs`, frozen at its
//! baseline, when the box's Git directories became the sealed reach's
//! (decision 0065 slice two, U8a2c): Git is asked them to seal a reach and
//! to observe it again, never for where a box mounts them.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What the host knows about the worktree's git that the box must be
/// told, gathered OUTSIDE the box before it is built (decision 0043
/// ruling 6): where the two git directories are, and who the seat
/// commits as.
///
/// A repository has two identities, and a linked `git worktree` makes
/// the difference load-bearing: `--git-dir` names the per-worktree
/// directory (the one holding this worktree's `index` and `HEAD`), while
/// `--git-common-dir` names the shared one (the main checkout's `.git`,
/// holding `objects`, `refs` and `config`). A primary checkout has one
/// path for both.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitFacts {
    /// The per-worktree git directory, absolute. Inside the worktree for
    /// a primary checkout; under the shared `.git/worktrees/<name>` for
    /// a linked worktree. `None` when the workdir is not a repository.
    pub git_dir: Option<PathBuf>,
    /// The shared git directory, absolute — the main checkout's `.git`
    /// for a linked worktree. `None` when the workdir is not a repository.
    pub common_dir: Option<PathBuf>,
    /// `user.name` and `user.email` as the host resolves them, as the
    /// environment entries git reads them from.
    pub identity: Vec<(String, String)>,
}

/// Read the git facts from the host. Never errors: a workdir that is not
/// a repository simply carries none, and the box then has no git to
/// mask.
pub fn git_facts(workdir: &Path) -> GitFacts {
    let (git_dir, common_dir) = git_dirs(workdir);
    GitFacts {
        git_dir,
        common_dir,
        identity: git_identity(workdir),
    }
}

/// The two git directories Git names for `workdir`, neither where it is
/// not a repository: what a box's sealed reach is observed again against
/// (U8a2c).
pub(super) fn git_dirs(workdir: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    match Command::new("git")
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ])
        .current_dir(workdir)
        .output()
    {
        Ok(out) if out.status.success() => parse_git_dirs(&String::from_utf8_lossy(&out.stdout)),
        _ => (None, None),
    }
}

/// Who the seat commits as in `workdir`, as the host resolves it.
pub(super) fn git_identity(workdir: &Path) -> Vec<(String, String)> {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|text| !text.is_empty())
    };
    let mut identity = Vec::new();
    for (env, key) in [("NAME", "user.name"), ("EMAIL", "user.email")] {
        if let Some(value) = git(&["config", key]) {
            identity.push((format!("GIT_AUTHOR_{env}"), value.clone()));
            identity.push((format!("GIT_COMMITTER_{env}"), value));
        }
    }
    identity
}

/// The two git directories `git rev-parse --git-dir --git-common-dir`
/// printed: one path per line, in the order asked, and no trim — a path
/// may end in a space. A path that itself spans a line makes the output
/// ambiguous, and an ambiguous git directory is no git directory: both
/// stay `None` and the box has no git to bind.
pub(super) fn parse_git_dirs(reported: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    let mut lines = reported.lines();
    match (lines.next(), lines.next(), lines.next()) {
        (Some(git_dir), Some(common_dir), None)
            if !git_dir.is_empty() && !common_dir.is_empty() =>
        {
            (
                Some(PathBuf::from(git_dir)),
                Some(PathBuf::from(common_dir)),
            )
        }
        _ => (None, None),
    }
}
