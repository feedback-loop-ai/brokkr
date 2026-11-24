//! Masking for complete strings and projections with structural delimiters.

use std::ops::Range;

use super::{covered, needles, BoundSecret, Encoder, NEEDLE_ENCODINGS};

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
