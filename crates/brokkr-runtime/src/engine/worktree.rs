//! Repository observations shared by the carry, verify and ship decisions
//! (0093 ruling 6).
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// What git answered, before a caller decides what that answer permits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TreeReading {
    Clean,
    Dirty,
    Refused,
    Unrun,
}

/// Which untracked files a status reading counts: the ones the
/// repository's own `status.showUntrackedFiles` admits, or every one,
/// which is what a verified candidate is held to (decision 0097 ruling 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Untracked {
    AsConfigured,
    All,
}

pub(super) fn read_tree(repo: &Path) -> TreeReading {
    read_tree_counting(repo, Untracked::AsConfigured)
}

pub(super) fn read_tree_counting(repo: &Path, untracked: Untracked) -> TreeReading {
    let mut status = Command::new("git");
    status.args(["status", "--porcelain"]).current_dir(repo);
    if untracked == Untracked::All {
        status.arg("--untracked-files=all");
    }
    match status.output() {
        Ok(out) if !out.stdout.is_empty() => TreeReading::Dirty,
        Ok(out) if out.status.success() => TreeReading::Clean,
        Ok(_) => TreeReading::Refused,
        Err(_) => TreeReading::Unrun,
    }
}

/// The repository's observed HEAD, or nothing when there is no readable
/// git tree there. Public because `brokkr realms` reads out the same
/// fact the ship gate compares against — one reader, not two.
pub fn git_head(repo: &std::path::Path) -> Option<String> {
    rev_parse(repo, &["HEAD"])
}

/// HEAD and the tree it identifies, read in one answer: the commit that
/// names a verified candidate and the immutable content it binds. Nothing
/// where no commit answers: no repository, an empty one, or no git.
pub(super) fn git_candidate(repo: &Path) -> Option<(String, String)> {
    let both = rev_parse(repo, &["HEAD", "HEAD^{tree}"])?;
    let (head, tree) = both.split_once('\n')?;
    Some((head.to_string(), tree.to_string()))
}

fn rev_parse(repo: &Path, specs: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("rev-parse")
        .args(specs)
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub(super) fn git_dirty(repo: &Path) -> bool {
    matches!(read_tree(repo), TreeReading::Dirty | TreeReading::Unrun)
}

#[cfg(test)]
mod tests;
