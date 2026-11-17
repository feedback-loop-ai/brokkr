//! `until` as RFC 3339 on the wire, an `OffsetDateTime` in the type: the
//! one format the provider-wait extension already reads (decision 0103),
//! kept here so the cause's field is typed past the edge.

use serde::Serializer;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// `at` as the wire spells it, for a sentence an operator reads.
pub(super) fn spell(at: &OffsetDateTime) -> String {
    at.format(&Rfc3339)
        .unwrap_or_else(|_| "an instant the journal cannot spell".to_string())
}

/// The instant `text` spells, where it is RFC 3339.
pub(super) fn parse(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

pub(super) fn serialize<S: Serializer>(
    at: &OffsetDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let text = at.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&text)
}
