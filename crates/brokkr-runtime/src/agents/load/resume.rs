//! The steps of one named execution shape in an adapter's `resume`
//! assessment, in the order `resume_assessment` reads them: the shape's
//! fields, its status, its limitations and reason, and the two rules that
//! keep a declaration from enabling itself. Every refusal stays in the
//! order it was written, so a shape failing several checks reports the
//! same first one.

use serde_json::{Map, Value};

use super::{
    bounded_reason, named, object, only_keys, resume_evidence, resume_identity, string,
    string_array, LibraryError, ResumeEvidence, ResumeIdentity, ResumeShape, ResumeStatus,
};

/// One named shape, read whole.
pub(super) fn shape(value: &Value, what: &str) -> Result<ResumeShape, LibraryError> {
    let raw = object(value, what)?;
    only_keys(
        raw,
        &[
            "status",
            "identity",
            "classes",
            "boundaries",
            "hands",
            "evidence",
            "limitations",
            "reason",
        ],
        what,
    )?;
    let status = status(raw, what).map_err(refused)?;
    let identity = resume_identity(raw, what)?;
    let classes = string_array(raw, "classes", what)?;
    named(&classes, "classes", what)?;
    let boundaries = string_array(raw, "boundaries", what)?;
    let hands = string(raw, "hands", what)?;
    let evidence = resume_evidence(raw, what)?;
    let limitations = limitations(raw, what)?;
    let reason = reason(raw, what)?;
    enablement(status, &identity, &evidence, reason.as_deref(), what).map_err(refused)?;
    Ok(ResumeShape {
        status,
        identity,
        classes,
        boundaries,
        hands,
        evidence,
        limitations,
        reason,
    })
}

/// Why a shape's status, or one of its two enablement rules, refused it:
/// one variant per check, so a test can tell which one fired. Its text is
/// the operator's, and `shape` turns it into `LibraryError::Invalid`.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum ResumeRefusal<'a> {
    #[error("{what} needs 'status': one of unmeasured, unsupported, supported")]
    Status { what: &'a str },
    #[error(
        "{what} is 'supported' with an unknown identity; only a MEASURED identity \
         can support enablement, because the per-invocation version check needs a \
         pinned version to compare the observed one against"
    )]
    UnknownIdentity { what: &'a str },
    #[error(
        "{what} is 'supported' and must name all four evidence references — \
         interface, restrictions, root, accounting; one of them does not imply \
         the others"
    )]
    IncompleteEvidence { what: &'a str },
    #[error(
        "{what} is 'unsupported' and needs its measured 'reason'; \"we looked and it \
         cannot\" is a different statement from \"nobody looked\""
    )]
    NoReason { what: &'a str },
}

/// The one place a refusal becomes the library's text.
fn refused(refusal: ResumeRefusal<'_>) -> LibraryError {
    LibraryError::Invalid(refusal.to_string())
}

/// One of the three tokens, and nothing else.
fn status<'a>(raw: &Map<String, Value>, what: &'a str) -> Result<ResumeStatus, ResumeRefusal<'a>> {
    match raw.get("status").and_then(Value::as_str) {
        Some("unmeasured") => Ok(ResumeStatus::Unmeasured),
        Some("unsupported") => Ok(ResumeStatus::Unsupported),
        Some("supported") => Ok(ResumeStatus::Supported),
        _ => Err(ResumeRefusal::Status { what }),
    }
}

/// The measured limitations, each a bounded line; absent is none.
fn limitations(raw: &Map<String, Value>, what: &str) -> Result<Vec<String>, LibraryError> {
    let limitations = match raw.get("limitations") {
        None => Vec::new(),
        Some(_) => string_array(raw, "limitations", what)?,
    };
    for limitation in &limitations {
        bounded_reason(limitation, "limitations", what)?;
    }
    Ok(limitations)
}

/// The measured reason, a bounded line, where the shape gives one.
fn reason(raw: &Map<String, Value>, what: &str) -> Result<Option<String>, LibraryError> {
    let reason = match raw.get("reason") {
        None => None,
        Some(_) => {
            let reason = string(raw, "reason", what)?;
            bounded_reason(&reason, "reason", what)?;
            Some(reason)
        }
    };
    Ok(reason)
}

/// The two rules that keep a declaration from enabling itself.
fn enablement<'a>(
    status: ResumeStatus,
    identity: &ResumeIdentity,
    evidence: &ResumeEvidence,
    reason: Option<&str>,
    what: &'a str,
) -> Result<(), ResumeRefusal<'a>> {
    if status == ResumeStatus::Supported {
        if !identity.is_measured() {
            return Err(ResumeRefusal::UnknownIdentity { what });
        }
        if !evidence.complete() {
            return Err(ResumeRefusal::IncompleteEvidence { what });
        }
    }
    if status == ResumeStatus::Unsupported && reason.is_none() {
        return Err(ResumeRefusal::NoReason { what });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
