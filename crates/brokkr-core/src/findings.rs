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

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::policy::{is_identifier, Severity};

/// Ruling 3's bound on a finding id, in bytes.
const ID_MAX_BYTES: usize = 64;

/// The bound on a finding's repository-relative file, in characters, as
/// v7's `maxLength` counts them.
const FILE_MAX_CHARS: usize = 4096;

/// The gate report fields only the engine writes (decision 0109 ruling
/// 1): it knows the head and the delta, and a seat does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineWritten {
    Covered,
    FollowUps,
    /// The head a protected effect's check accepted (fix round 0109a1r3).
    CheckedHead,
}

impl EngineWritten {
    const ALL: [EngineWritten; 3] = [
        EngineWritten::Covered,
        EngineWritten::FollowUps,
        EngineWritten::CheckedHead,
    ];

    /// The result field it is.
    pub const fn field(self) -> &'static str {
        match self {
            EngineWritten::Covered => "covered",
            EngineWritten::FollowUps => "follow_ups",
            EngineWritten::CheckedHead => "checked_head",
        }
    }
}

impl fmt::Display for EngineWritten {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.field())
    }
}

/// Why a seat's result is refused at admission. Each is the seat's fault
/// and is never repaired (decision 0001); no variant echoes a value the
/// seat wrote except an id that already passed ruling 3's pattern.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdmissionRefusal {
    #[error("the result carries '{0}', which only the engine writes (decision 0109 ruling 1)")]
    EngineWritten(EngineWritten),
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
    #[error(
        "the result carries 'findings' from a work site: only a gate reports findings \
         (decision 0109 ruling 1)"
    )]
    WorkSite,
}

/// The compiled site a seat's result was written at, as admission reads
/// it (decision 0109 ruling 1): a gate's, by its site digest, which may
/// report findings, or a work site's, which may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reporter<'a> {
    Gate(&'a str),
    Work,
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

/// Why a number is not a [`Line`]; a seat's bad line is refused as
/// [`AdmissionRefusal::Malformed`], so the reason stays inside the module.
#[derive(Debug, Error)]
#[error("not a positive line number")]
struct NotALine;

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

/// A repository-relative path: non-empty, at most 4096 characters, never
/// absolute, and with no `..` component, exactly what v7's `repo_path`
/// admits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RepoFile(String);

impl TryFrom<String> for RepoFile {
    type Error = WordError;

    fn try_from(file: String) -> Result<RepoFile, WordError> {
        let relative = !file.is_empty()
            && file.chars().count() <= FILE_MAX_CHARS
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

/// A finding's positive, one-based line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
struct Line(u64);

impl TryFrom<u64> for Line {
    type Error = NotALine;

    fn try_from(line: u64) -> Result<Line, NotALine> {
        if line == 0 {
            Err(NotALine)
        } else {
            Ok(Line(line))
        }
    }
}

impl From<Line> for u64 {
    fn from(line: Line) -> u64 {
        line.0
    }
}

macro_rules! word_text {
    ($($word:ident),*) => {$(
        impl From<$word> for String {
            fn from(word: $word) -> String {
                word.0
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

/// A `line` that is present is a positive integer: `null` is refused, and
/// only an absent line reads as a whole-file finding.
fn present_line<'de, D: Deserializer<'de>>(line: D) -> Result<Option<Line>, D::Error> {
    Line::deserialize(line).map(Some)
}

/// What a seat writes for one finding: everything but the site.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reported {
    id: FindingId,
    file: RepoFile,
    #[serde(default, deserialize_with = "present_line")]
    line: Option<Line>,
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
    #[serde(
        default,
        deserialize_with = "present_line",
        skip_serializing_if = "Option::is_none"
    )]
    line: Option<Line>,
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

    /// The one-based line it was reported at, when the seat named one.
    pub fn line(&self) -> Option<u64> {
        self.line.map(u64::from)
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
fn stamp(findings: &Value, site: &SiteDigest) -> Result<Vec<Finding>, AdmissionRefusal> {
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

/// Admit a seat's result written at `reporter`: a result that writes a
/// field only the engine writes is refused (ruling 1), and so is one
/// carrying `findings` from a work site; a gate's `findings` are read,
/// refused or stamped (ruling 3) and handed back typed, taken out of
/// `result`, so they stay typed until [`journal`] writes them at the
/// journal's boundary. `None` is a result that reported no `findings`.
/// A result with findings and no site to stamp is refused rather than
/// journaled unkeyed.
pub fn admit(
    result: &mut Value,
    reporter: Option<Reporter<'_>>,
) -> Result<Option<Vec<Finding>>, AdmissionRefusal> {
    if let Some(written) = EngineWritten::ALL
        .into_iter()
        .find(|written| result.get(written.field()).is_some())
    {
        return Err(AdmissionRefusal::EngineWritten(written));
    }
    let Some(findings) = result.get("findings") else {
        return Ok(None);
    };
    let site = match reporter {
        Some(Reporter::Work) => return Err(AdmissionRefusal::WorkSite),
        Some(Reporter::Gate(site)) => SiteDigest::try_from(site.to_string()).ok(),
        None => None,
    }
    .ok_or(AdmissionRefusal::Unsited)?;
    let stamped = stamp(findings, &site)?;
    // Only an object carries `findings`, so the removal always reaches one.
    let _taken = result
        .as_object_mut()
        .and_then(|fields| fields.remove("findings"));
    Ok(Some(stamped))
}

/// Write the stamped `findings` into the `result` the journal records:
/// the one place an admitted finding leaves its type.
pub fn journal(result: &mut Value, findings: &[Finding]) {
    result["findings"] = serde_json::to_value(findings).expect("a finding serializes");
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && is_lower_hex(value)
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

mod covered;
mod marker;

pub use covered::{CheckedHead, Covered, CoveredHead};
pub use marker::{admit_live, by_the_engine, stamp_authored, EngineMarker, LiveRefusal, AUTHORED};

#[cfg(test)]
mod tests;
