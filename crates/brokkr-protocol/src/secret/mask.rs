//! Masking for complete strings and projections with structural delimiters.

use std::ops::Range;

use super::{covered, needles, BoundSecret, Encoder, MIN_VALUE_BYTES, NEEDLE_ENCODINGS};
use crate::adapters::escapes::without_ansi;

impl BoundSecret {
    /// Mask stderr after escape removal, joining both kinds of match in
    /// one union. Stripping rejoins a value an escape split (U3), and the
    /// extra needle covers a value that itself holds an escape (#1242).
    /// Masking one spelling first could erase part of an overlapping match.
    pub(crate) fn mask_ansi_evidence(raw: &[u8], bindings: &[Self]) -> String {
        let stripped = without_ansi(&String::from_utf8_lossy(raw));
        let mut encodings = NEEDLE_ENCODINGS.to_vec();
        encodings.push(("ansi-stripped", stripped_needle));
        String::from_utf8_lossy(&mask_needles(stripped.as_bytes(), bindings, &encodings))
            .into_owned()
    }
}

fn stripped_needle(bytes: &[u8]) -> Vec<u8> {
    let stripped = without_ansi(&String::from_utf8_lossy(bytes));
    if stripped.len() < MIN_VALUE_BYTES {
        return Vec::new();
    }
    stripped.into_bytes()
}

#[cfg(test)]
#[test]
fn stripped_needles_respect_the_minimum_length() {
    assert_eq!(stripped_needle(b"ab\x1b[0mc"), b"");
    assert_eq!(stripped_needle(b"ab\x1b[0mcd"), b"abcd");
}

pub(super) fn mask_needles(
    bytes: &[u8],
    bindings: &[BoundSecret],
    encodings: &[(&str, Encoder)],
) -> Vec<u8> {
    write_mask(bytes, bindings, encodings, |_, _| {})
}

fn write_mask(
    bytes: &[u8],
    bindings: &[BoundSecret],
    encodings: &[(&str, Encoder)],
    mut written: impl FnMut(Range<usize>, usize),
) -> Vec<u8> {
    if bindings.is_empty() {
        return bytes.to_vec();
    }
    let needles = needles(bindings, encodings);
    let mut out = Vec::with_capacity(bytes.len());
    let mut clear = 0;
    for (start, end, name) in covered(bytes, &needles) {
        out.extend_from_slice(&bytes[clear..start]);
        out.extend_from_slice(format!("[secret:{name}]").as_bytes());
        written(start..end, out.len());
        clear = end;
    }
    out.extend_from_slice(&bytes[clear..]);
    out
}

/// Whether a run in the shared union mask touches a span of the raw text.
pub(crate) fn mask_intersects(text: &str, bindings: &[BoundSecret], span: Range<usize>) -> bool {
    let mut intersects = false;
    write_mask(text.as_bytes(), bindings, &NEEDLE_ENCODINGS, |run, _| {
        intersects |= run.start < span.end && span.start < run.end;
    });
    intersects
}

pub(crate) enum MaskedSplit {
    Parts { left: String, right: String },
    Whole(String),
}

/// Mask the whole string and map the original separator into that output.
/// A run touching the bytes of either delimiter makes the cut unusable.
pub(crate) fn mask_split(
    text: &str,
    bindings: &[BoundSecret],
    prefix_end: usize,
    separator: Range<usize>,
) -> MaskedSplit {
    let mut cut = Some(separator.clone());
    let bytes = write_mask(text.as_bytes(), bindings, &NEEDLE_ENCODINGS, |run, end| {
        if run.start < prefix_end || (run.start < separator.end && run.end > separator.start) {
            cut = None;
        } else if let Some(cut) = cut.as_mut().filter(|_| run.end <= separator.start) {
            let start = end + separator.start - run.end;
            *cut = start..start + separator.len();
        }
    });
    let decoded = |bytes| String::from_utf8_lossy(bytes).into_owned();
    match cut {
        Some(cut) => MaskedSplit::Parts {
            left: decoded(&bytes[prefix_end..cut.start]),
            right: decoded(&bytes[cut.end..]),
        },
        None => MaskedSplit::Whole(decoded(&bytes)),
    }
}
