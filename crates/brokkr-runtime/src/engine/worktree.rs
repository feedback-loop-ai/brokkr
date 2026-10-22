//! Repository observations shared by the carry and ship decisions (0093 ruling 6).
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

pub(super) fn read_tree(repo: &Path) -> TreeReading {
    match Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
    {
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
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
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
