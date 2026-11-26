//! Masking for complete strings and projections with structural delimiters.

use std::ops::Range;

use super::{
    advance, covered, needles, projected_encodings, BoundSecret, Encoder, MIN_VALUE_BYTES,
    NEEDLE_ENCODINGS,
};
use crate::adapters::escapes::{without_ansi, without_ansi_with_offsets};

type Run<'a> = (usize, usize, &'a str);

impl BoundSecret {
    /// Mask both raw matches and matches visible after stripping (#1242).
    /// Stripping rejoins a value split by an escape (U3). Translate the
    /// raw runs before writing any marker, so an escape can
    /// consume a match's prefix and the rest still belongs to the union.
    pub(crate) fn mask_ansi_evidence(raw: &[u8], bindings: &[Self]) -> String {
        let raw_needles = needles(bindings, &NEEDLE_ENCODINGS);
        let (stripped, origins) = without_ansi_with_offsets(raw);
        let mut encodings = NEEDLE_ENCODINGS.to_vec();
        encodings.push(("ansi-stripped", stripped_needle));
        let stripped_needles = needles(bindings, &encodings);
        let raw_runs = projected_runs(&origins, covered(raw, &raw_needles));
        let stripped_runs = covered(stripped.as_bytes(), &stripped_needles);
        let runs = union_runs(raw_runs, stripped_runs);
        String::from_utf8_lossy(&write_runs(stripped.as_bytes(), runs, |_, _| {})).into_owned()
    }
}

/// A value's ANSI-stripped form, held to the masker's minimum needle
/// length: only this needle is floored, as ce633e1ff floored it, because a
/// short stripped form of a longer value would match ordinary text. The
/// value's own encodings keep no floor, as `mask_text` and `Guard` keep
/// none, so a short bound value is still masked (#1242). An empty needle
/// is dropped by `needles`.
fn stripped_needle(bytes: &[u8]) -> Vec<u8> {
    let stripped = without_ansi(&String::from_utf8_lossy(bytes)).into_bytes();
    match stripped.len() >= MIN_VALUE_BYTES {
        true => stripped,
        false => Vec::new(),
    }
}

/// Advance through the origins once, dropping raw runs the stripper
/// consumed completely. The resulting runs are in stripped byte order.
fn projected_runs<'a, 'n: 'a>(
    origins: &'a [Range<usize>],
    runs: impl Iterator<Item = Run<'n>> + 'a,
) -> impl Iterator<Item = Run<'n>> + 'a {
    let mut next = 0;
    runs.filter_map(move |(raw_start, raw_end, name)| {
        next += origins[next..]
            .iter()
            .take_while(|at| at.end <= raw_start)
            .count();
        let start = next;
        next += origins[next..]
            .iter()
            .take_while(|at| at.start < raw_end)
            .count();
        (start < next).then_some((start, next, name))
    })
}

/// Merge the two ordered run streams. At a shared start keep the stripped
/// run's name, which is what that projection would have written already.
fn union_runs<'a, 'n: 'a>(
    raw: impl Iterator<Item = Run<'n>> + 'a,
    stripped: impl Iterator<Item = Run<'n>> + 'a,
) -> impl Iterator<Item = Run<'n>> + 'a {
    let mut raw = raw.peekable();
    let mut stripped = stripped.peekable();
    let mut open: Option<Run<'n>> = None;
    std::iter::from_fn(move || loop {
        let next = match (raw.peek(), stripped.peek()) {
            (Some(raw_run), Some(stripped_run)) if raw_run.0 < stripped_run.0 => raw.next(),
            (_, Some(_)) => stripped.next(),
            (_, None) => raw.next(),
        };
        match (open.as_mut(), next) {
            (Some(run), Some(next)) if next.0 <= run.1 => run.1 = run.1.max(next.1),
            (_, Some(next)) => {
                if let Some(run) = open.replace(next) {
                    return Some(run);
                }
            }
            (_, None) => return open.take(),
        }
    })
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
    written: impl FnMut(Range<usize>, usize),
) -> Vec<u8> {
    if bindings.is_empty() {
        return bytes.to_vec();
    }
    let needles = needles(bindings, encodings);
    write_runs(bytes, covered(bytes, &needles), written)
}

fn write_runs<'a>(
    bytes: &[u8],
    runs: impl Iterator<Item = Run<'a>>,
    mut written: impl FnMut(Range<usize>, usize),
) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut clear = 0;
    for (start, end, name) in runs {
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

/// Streaming matcher for any bound secret across chunk boundaries (#1227).
/// Holds one KMP automaton per encoded needle (all [`NEEDLE_ENCODINGS`] plus
/// JSON-string escaping, matching [`super::mask_projected`]) so a bound value
/// split across two read buffers still matches without buffering whole files.
pub(crate) struct SecretMatcher {
    needles: Vec<NeedleState>,
}

struct NeedleState {
    needle: Vec<u8>,
    border: Vec<usize>,
    matched: usize,
}

impl SecretMatcher {
    pub(crate) fn new(bindings: &[BoundSecret]) -> Self {
        if bindings.is_empty() {
            return Self {
                needles: Vec::new(),
            };
        }
        let encodings = projected_encodings();
        let needles = needles(bindings, &encodings)
            .into_iter()
            .map(|(needle, _)| {
                let mut border = vec![0; needle.len()];
                let mut matched = 0;
                for i in 1..needle.len() {
                    matched = advance(&needle, &border, matched, needle[i]);
                    border[i] = matched;
                }
                NeedleState {
                    needle,
                    border,
                    matched: 0,
                }
            })
            .collect();
        Self { needles }
    }

    pub(crate) fn reset(&mut self) {
        for state in &mut self.needles {
            state.matched = 0;
        }
    }

    /// Feed the next chunk of bytes from the current file. Returns `true` as
    /// soon as any bound needle matches across the bytes fed since [`Self::reset`].
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> bool {
        if self.needles.is_empty() {
            return false;
        }
        for &byte in bytes {
            for state in &mut self.needles {
                state.matched = advance(&state.needle, &state.border, state.matched, byte);
                if state.matched == state.needle.len() {
                    return true;
                }
            }
        }
        false
    }
}
