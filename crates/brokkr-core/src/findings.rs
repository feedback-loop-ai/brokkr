//! A gate's findings and its stable finding key (decision 0109 ruling 3).
//!
//! A seat reports each finding with a stable `id`; the engine stamps the
//! site digest of the compiled site that reported it, and `(site, id)` is
//! the key. `file`, `line`, `summary`, `severity` and `security` are the
//! record, not the identity, so a moved line, a reworded summary and a
//! moved file are the same finding. [`admit`] parses a seat's `findings`
//! once, at the result edge, refuses what ruling 3 refuses, and writes the
//! stamped records back; nothing here reads a clock, a file or a process.

use std::collections::BTreeSet;
use std::fmt;
use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::policy::{is_identifier, Severity};

/// Ruling 3's bound on a finding id, in bytes.
pub const ID_MAX_BYTES: usize = 64;

/// The bound on a finding's repository-relative file, as v7 states it.
const FILE_MAX_BYTES: usize = 4096;

/// Why a result's findings are refused. Each is the seat's fault and is
/// never repaired (decision 0001); no variant echoes a value the seat
/// wrote except an id that already passed ruling 3's pattern.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdmissionRefusal {
    #[error("the result's findings are not an array of finding records")]
    NotAnArray,
    #[error(
        "finding {index} carries a site: the engine stamps each finding's site \
         (decision 0109 ruling 3)"
    )]
    SeatSite { index: usize },
    #[error(
        "finding {index} has no id matching ^[a-z0-9][a-z0-9._-]*$ of at most \
         {ID_MAX_BYTES} bytes (decision 0109 ruling 3)"
    )]
    Id { index: usize },
    #[error(
        "finding {index} is not a finding record: a repository-relative file, an \
         optional positive line, a non-empty summary, a severity and a security flag, \
         and nothing else"
    )]
    Malformed { index: usize },
    #[error("finding '{id}' is reported twice by one site (decision 0109 ruling 3)")]
    Duplicate { id: FindingId },
    #[error("findings were reported at a site the engine cannot stamp")]
    Unsited,
}

/// Why a value is not one of this module's typed words.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WordError {
    #[error("not a finding id: ^[a-z0-9][a-z0-9._-]*$, at most {ID_MAX_BYTES} bytes")]
    Id,
    #[error("not a site digest: 64 lowercase hex characters")]
    Site,
    #[error("not a repository-relative file")]
    File,
    #[error("not a non-empty summary")]
    Summary,
}

/// A seat's stable identifier for one finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FindingId(String);

impl TryFrom<String> for FindingId {
    type Error = WordError;

    fn try_from(id: String) -> Result<FindingId, WordError> {
        if id.len() <= ID_MAX_BYTES && is_identifier(&id) {
            Ok(FindingId(id))
        } else {
            Err(WordError::Id)
        }
    }
}

/// The 64-hex digest of one compiled structural site, the engine's.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SiteDigest(String);

impl TryFrom<String> for SiteDigest {
    type Error = WordError;

    fn try_from(site: String) -> Result<SiteDigest, WordError> {
        if is_sha256(&site) {
            Ok(SiteDigest(site))
        } else {
            Err(WordError::Site)
        }
    }
}

/// A repository-relative path: non-empty, never absolute, and with no
/// `..` component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RepoFile(String);

impl TryFrom<String> for RepoFile {
    type Error = WordError;

    fn try_from(file: String) -> Result<RepoFile, WordError> {
        let relative = !file.is_empty()
            && file.len() <= FILE_MAX_BYTES
            && !file.starts_with('/')
            && file.split('/').all(|part| part != "..");
        if relative {
            Ok(RepoFile(file))
        } else {
            Err(WordError::File)
        }
    }
}

/// A finding's non-empty summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Summary(String);

impl TryFrom<String> for Summary {
    type Error = WordError;

    fn try_from(summary: String) -> Result<Summary, WordError> {
        if summary.is_empty() {
            Err(WordError::Summary)
        } else {
            Ok(Summary(summary))
        }
    }
}

macro_rules! word_text {
    ($($word:ident),*) => {$(
        impl From<$word> for String {
            fn from(word: $word) -> String {
                word.0
            }
        }

        impl $word {
            /// The word as it is written.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $word {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    )*};
}

word_text!(FindingId, SiteDigest, RepoFile, Summary);

/// A finding's identity: the site that reported it and its id. Both are
/// required, so no key exists without a site and an id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingKey {
    site: SiteDigest,
    id: FindingId,
}

impl FindingKey {
    /// The site that reported the finding.
    pub fn site(&self) -> &SiteDigest {
        &self.site
    }

    /// The id its seat gave it.
    pub fn id(&self) -> &FindingId {
        &self.id
    }
}

/// What a seat writes for one finding: everything but the site.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reported {
    id: FindingId,
    file: RepoFile,
    #[serde(default)]
    line: Option<NonZeroU64>,
    summary: Summary,
    severity: Severity,
    security: bool,
}

/// One finding as the engine journals it: the seat's report with the
/// site the engine stamped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    site: SiteDigest,
    id: FindingId,
    file: RepoFile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line: Option<NonZeroU64>,
    summary: Summary,
    severity: Severity,
    security: bool,
}

impl Finding {
    /// The finding's key, `(site, id)`.
    pub fn key(&self) -> FindingKey {
        FindingKey {
            site: self.site.clone(),
            id: self.id.clone(),
        }
    }

    pub fn file(&self) -> &RepoFile {
        &self.file
    }

    pub fn line(&self) -> Option<NonZeroU64> {
        self.line
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn security(&self) -> bool {
        self.security
    }

    fn stamped(reported: Reported, site: &SiteDigest) -> Finding {
        Finding {
            site: site.clone(),
            id: reported.id,
            file: reported.file,
            line: reported.line,
            summary: reported.summary,
            severity: reported.severity,
            security: reported.security,
        }
    }
}

/// Read one seat-written finding, refusing a seat-supplied site, an id
/// outside ruling 3's pattern, and any other malformed record, in that
/// order, each naming the finding's position only.
fn reported(index: usize, item: &Value) -> Result<Reported, AdmissionRefusal> {
    let object = item
        .as_object()
        .ok_or(AdmissionRefusal::Malformed { index })?;
    if object.contains_key("site") {
        return Err(AdmissionRefusal::SeatSite { index });
    }
    let id = object.get("id").and_then(Value::as_str);
    if id.is_none_or(|id| FindingId::try_from(id.to_string()).is_err()) {
        return Err(AdmissionRefusal::Id { index });
    }
    Reported::deserialize(item).map_err(|_| AdmissionRefusal::Malformed { index })
}

/// Read and stamp the findings a seat wrote at `site`: every finding's
/// site is the one site that reported it, and an id twice in one
/// verdict is refused.
pub fn stamp(findings: &Value, site: &SiteDigest) -> Result<Vec<Finding>, AdmissionRefusal> {
    let items = findings.as_array().ok_or(AdmissionRefusal::NotAnArray)?;
    let mut keys = BTreeSet::new();
    let mut stamped = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let finding = Finding::stamped(reported(index, item)?, site);
        if !keys.insert(finding.key()) {
            return Err(AdmissionRefusal::Duplicate { id: finding.id });
        }
        stamped.push(finding);
    }
    Ok(stamped)
}

/// Admit a seat's result at the site whose digest is `site`: its
/// `findings`, if it wrote any, are read, refused or stamped in place
/// (decision 0109 ruling 3). A result with findings and no site to stamp
/// is refused rather than journaled unkeyed.
pub fn admit(result: &mut Value, site: Option<&str>) -> Result<(), AdmissionRefusal> {
    let Some(findings) = result.get("findings") else {
        return Ok(());
    };
    let site = site
        .and_then(|site| SiteDigest::try_from(site.to_string()).ok())
        .ok_or(AdmissionRefusal::Unsited)?;
    let stamped = stamp(findings, &site)?;
    result["findings"] = serde_json::to_value(stamped).expect("a finding serializes");
    Ok(())
}

/// The stamped findings a journaled result carries, read back. A result
/// with no `findings` reads as the empty set (decision 0109 ruling 6), and
/// one whose findings do not read is `None`, never a guessed key.
pub fn recorded(result: &Value) -> Option<Vec<Finding>> {
    match result.get("findings") {
        None => Some(Vec::new()),
        Some(findings) => Vec::<Finding>::deserialize(findings).ok(),
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests;
