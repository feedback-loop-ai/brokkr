//! The retirement of a historical artifact root (decision 0105 ruling
//! 3(a)): a sealed `retired-<key>.json` beside the historical record it
//! retires, in the same worktree directory of the coordination root,
//! sealed the same way and under the same admission lock.
//!
//! The historical record is never deleted. Admission reads a historical
//! root with a readable retirement as no longer guarding evidence; a
//! retirement that cannot be read refuses with CR2's cause, never read as
//! absent and never as retired.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::engine::artifacts::hex;

/// What a retirement's record begins with.
const PREFIX: &str = "retired-";

/// What a retirement's record ends with.
const SUFFIX: &str = ".json";

/// Why a historical root was retired: its evidence was archived, or the
/// operator gave it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cause {
    Archived,
    Discarded,
}

/// One historical root's retirement, as the operator's verb sealed it
/// beside the historical record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retirement {
    /// The artifact root the historical record names.
    pub artifacts: PathBuf,
    /// Whether its evidence was archived or discarded.
    pub cause: Cause,
    /// The archive directory, when the cause is `archived`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<PathBuf>,
    /// The archive inventory's sha256, when the cause is `archived`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_sha256: Option<String>,
    /// The operator whose verb retired it.
    pub operator: String,
    /// The operator's reason, in their words.
    pub reason: String,
    /// When the verb retired it, RFC 3339.
    pub retired_at: String,
    /// The engine version that sealed the retirement.
    pub engine: String,
}

impl Retirement {
    /// What became of the evidence, in the operator's words: the archive
    /// it went to with the inventory's digest, or that it was discarded.
    /// Every surface prints this one derivation, escaping it its own way
    /// (decision 0105, M5).
    pub fn outcome(&self) -> String {
        match (&self.cause, &self.archive) {
            (Cause::Archived, Some(at)) => match &self.inventory_sha256 {
                Some(sha256) => format!("archived at {} (inventory {sha256})", at.display()),
                None => format!("archived at {}", at.display()),
            },
            (Cause::Archived, None) => "archived".to_string(),
            (Cause::Discarded, _) => "discarded".to_string(),
        }
    }

    /// The whole retirement in the operator's words: what became of the
    /// evidence, when, by whom and why. A surface escapes this text, and
    /// adds nothing of its own to the words.
    pub fn words(&self) -> String {
        format!(
            "{} on {} by {}: {}",
            self.outcome(),
            self.retired_at,
            self.operator,
            self.reason
        )
    }
}

/// The retirement's record name for the historical record's key `key`.
pub(crate) fn name(key: &str) -> String {
    format!("{PREFIX}{key}{SUFFIX}")
}

/// The key a retirement's record name `file` carries, where it is one.
pub(crate) fn named(file: &str) -> Option<&str> {
    let key = file.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    hex(key).then_some(key)
}
