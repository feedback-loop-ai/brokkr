//! What a verdict covered (decision 0109 ruling 7), engine-written. The
//! `head` arm has two field sets: a gate with its own head record names
//! the commit it read, and the protected phase names the SHA-256 of its
//! decide-time head string, a digest-pinned projection of
//! `reviewed_heads` rather than a third head authority. Exactly one is
//! present, beside `dirty`, and no `base` is recorded. The `artifacts`
//! arm v7 admits has no writer until slice 0109c, so it has no type here.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{is_lower_hex, is_sha256};
use crate::canonical::sha256_bytes;

/// The head a verdict covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoveredHead {
    /// The commit a gate with its own head record read.
    Commit(String),
    /// The SHA-256 of the protected phase's decide-time head string.
    Digest(String),
}

/// One `covered` record of the `head` arm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Record", into = "Record")]
pub struct Covered {
    head: CoveredHead,
    dirty: bool,
}

/// Why a covered record does not read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoveredError {
    #[error("a covered head record names exactly one of a commit head or a SHA-256 head_digest")]
    Head,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Head,
}

/// The record as v7 writes it.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    head_digest: Option<String>,
    dirty: bool,
}

impl TryFrom<Record> for Covered {
    type Error = CoveredError;

    fn try_from(record: Record) -> Result<Covered, CoveredError> {
        let head = match (record.head, record.head_digest) {
            (Some(commit), None) if is_commit(&commit) => CoveredHead::Commit(commit),
            (None, Some(digest)) if is_sha256(&digest) => CoveredHead::Digest(digest),
            _ => return Err(CoveredError::Head),
        };
        Ok(Covered {
            head,
            dirty: record.dirty,
        })
    }
}

impl From<Covered> for Record {
    fn from(covered: Covered) -> Record {
        let (head, head_digest) = match covered.head {
            CoveredHead::Commit(commit) => (Some(commit), None),
            CoveredHead::Digest(digest) => (None, Some(digest)),
        };
        Record {
            kind: Kind::Head,
            head,
            head_digest,
            dirty: covered.dirty,
        }
    }
}

impl Covered {
    /// The record of a gate that read the commit `head`; `None` when
    /// `head` is not a commit id, so no record names what is not one.
    pub fn commit(head: &str, dirty: bool) -> Option<Covered> {
        is_commit(head).then(|| Covered {
            head: CoveredHead::Commit(head.to_string()),
            dirty,
        })
    }

    /// The protected phase's record: the SHA-256 of its decide-time head
    /// string, `head`.
    pub fn digest_of(head: &str, dirty: bool) -> Covered {
        Covered {
            head: CoveredHead::Digest(sha256_bytes(head.as_bytes())),
            dirty,
        }
    }

    pub fn head(&self) -> &CoveredHead {
        &self.head
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Whether this record names `head`: the same commit, or the SHA-256
    /// of the same head string.
    pub fn projects(&self, head: &str) -> bool {
        match &self.head {
            CoveredHead::Commit(commit) => commit == head,
            CoveredHead::Digest(digest) => *digest == sha256_bytes(head.as_bytes()),
        }
    }
}

/// A SHA-1 or SHA-256 commit id, lowercase hex.
fn is_commit(head: &str) -> bool {
    matches!(head.len(), 40 | 64) && is_lower_hex(head)
}

#[cfg(test)]
mod tests;
