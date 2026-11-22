//! A harness's escape sequences, ended where ECMA-48 ends them, and its
//! stderr as evidence: stripped first, then masked. The one home Qoder's
//! (#1178) and Antigravity's (#1200) drivers read through, moved here from
//! the qoder driver unchanged.

use crate::secret;

const ESC: char = '\u{1b}';
const BEL: char = '\u{7}';

/// The characters an escape sequence is read from.
type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// `text` without its escape sequences, each ended where ECMA-48 ends it
/// (V2): a CSI (`ESC [`) at its final byte, an OSC (`ESC ]`) at BEL or ST
/// (`ESC \`), and any other `ESC` followed by a byte 0x40 to 0x5F as a
/// two-byte sequence. An `ESC` before any other byte goes alone, and that
/// byte is kept. Qoder colours its refusals. This is the one stripper of
/// the checks and of the evidence.
pub(super) fn without_ansi(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != ESC {
            kept.push(c);
            continue;
        }
        match chars.next_if(|next| ('\u{40}'..='\u{5f}').contains(next)) {
            Some('[') => control_sequence(&mut chars),
            Some(']') => operating_system_command(&mut chars),
            Some(_) | None => {}
        }
    }
    kept
}

/// A CSI after its `ESC [`: parameter bytes 0x30 to 0x3F, intermediate
/// bytes 0x20 to 0x2F, then one final byte 0x40 to 0x7E, consumed. Any
/// other byte ends a malformed sequence and is not consumed.
fn control_sequence(chars: &mut Chars<'_>) {
    while chars
        .next_if(|c| ('\u{30}'..='\u{3f}').contains(c))
        .is_some()
    {}
    while chars
        .next_if(|c| ('\u{20}'..='\u{2f}').contains(c))
        .is_some()
    {}
    chars.next_if(|c| ('\u{40}'..='\u{7e}').contains(c));
}

/// An OSC after its `ESC ]`: everything up to BEL or ST, consumed with it.
fn operating_system_command(chars: &mut Chars<'_>) {
    while let Some(c) = chars.next() {
        if c == BEL || (c == ESC && chars.next_if_eq(&'\\').is_some()) {
            return;
        }
    }
}

/// A harness's raw stderr as evidence: lossy UTF-8, stripped first, so a
/// value an escape split is whole when the mask reads it (U3), then masked
/// through the seat's `bindings`. It is evidence, no word of the driver's.
pub(super) fn plain_evidence(raw: &[u8], bindings: &[secret::BoundSecret]) -> String {
    let stripped = without_ansi(&String::from_utf8_lossy(raw));
    secret::mask_text(stripped.as_bytes(), bindings)
}
