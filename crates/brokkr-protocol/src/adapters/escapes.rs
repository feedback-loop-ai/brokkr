//! A harness's escape sequences, ended where ECMA-48 ends them, and its
//! stderr evidence shared by the Qoder (#1178) and Antigravity (#1200)
//! drivers.

use std::ops::Range;

use crate::secret;

const ESC: u8 = b'\x1b';
const BEL: u8 = b'\x07';

type Bytes<'a> = std::iter::Peekable<std::iter::Copied<std::slice::Iter<'a, u8>>>;

/// The plain projection used by the driver's unmasked checks.
pub(crate) fn without_ansi(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    strip(text.as_bytes(), |range| kept.push_str(&text[range]));
    kept
}

/// The same projection of lossy UTF-8 stderr, with each output byte's raw
/// origin. All bytes of a replacement character share the replaced span.
pub(crate) fn without_ansi_with_offsets(raw: &[u8]) -> (String, Vec<Range<usize>>) {
    let mut kept = String::with_capacity(raw.len());
    let mut origins = Vec::with_capacity(raw.len());
    strip(raw, |range| {
        let mut start = range.start;
        for chunk in raw[range].utf8_chunks() {
            let end = start + chunk.valid().len();
            kept.push_str(chunk.valid());
            origins.extend((start..end).map(|at| at..at + 1));
            start = end + chunk.invalid().len();
            if end != start {
                kept.push('\u{fffd}');
                origins.extend(std::iter::repeat_n(end..start, '\u{fffd}'.len_utf8()));
            }
        }
    });
    (kept, origins)
}

/// Retain the ranges between escapes, each ended where ECMA-48 ends it
/// (V2). A CSI ends at its final byte, an OSC at BEL or ST, and any other
/// ESC followed by 0x40 to 0x5F consumes that byte. A lone ESC goes alone.
/// All cuts are next to ASCII delimiters, preserving UTF-8 decoding of
/// each kept range even when the raw capture is malformed.
fn strip(raw: &[u8], mut keep: impl FnMut(Range<usize>)) {
    let mut bytes = raw.iter().copied().peekable();
    let mut start = 0;
    while let Some(byte) = bytes.next() {
        if byte != ESC {
            continue;
        }
        keep(start..raw.len() - bytes.len() - 1);
        match bytes.next_if(|next| (0x40..=0x5f).contains(next)) {
            Some(b'[') => control_sequence(&mut bytes),
            Some(b']') => operating_system_command(&mut bytes),
            Some(_) | None => {}
        }
        start = raw.len() - bytes.len();
    }
    keep(start..raw.len());
}

/// A CSI after its ESC [: parameter bytes 0x30 to 0x3F, intermediate
/// bytes 0x20 to 0x2F, then one final byte 0x40 to 0x7E, consumed. Any
/// other byte ends a malformed sequence and is not consumed.
fn control_sequence(bytes: &mut Bytes<'_>) {
    while bytes.next_if(|c| (0x30..=0x3f).contains(c)).is_some() {}
    while bytes.next_if(|c| (0x20..=0x2f).contains(c)).is_some() {}
    bytes.next_if(|c| (0x40..=0x7e).contains(c));
}

/// An OSC after its ESC ]: everything up to BEL or ST, consumed with it.
fn operating_system_command(bytes: &mut Bytes<'_>) {
    while let Some(c) = bytes.next() {
        if c == BEL || (c == ESC && bytes.next_if_eq(&b'\\').is_some()) {
            return;
        }
    }
}

/// A harness's raw stderr as evidence, masked through the seat's bindings.
pub(super) fn plain_evidence(raw: &[u8], bindings: &[secret::BoundSecret]) -> String {
    secret::BoundSecret::mask_ansi_evidence(raw, bindings)
}

#[cfg(test)]
const EVIDENCE_CASES: [(&str, &str); 4] = [
    ("bloom\u{1b}[31mpond", "bloom\u{1b}[31mpond"),
    (
        "bloom\u{1b}]0;window\u{7}pond",
        "bloom\u{1b}]0;window\u{7}pond",
    ),
    ("bloompond", "bloom\u{1b}[31mpond"),
    ("bloompond", "bloompond"),
];

#[cfg(test)]
pub(super) fn assert_stderr_evidence(
    name: &str,
    evidence: impl Fn(&str, &[secret::BoundSecret]) -> String,
) {
    let partial_escapes = [
        ("ABCDEFGH", "\x1bABCDEFGH"),
        ("12345678abcdefgh", "\x1b[12345678abcdefgh"),
        ("[31mSECRETVALUE", "\x1b[31mSECRETVALUE"),
        ("sk-abcd1234", "\x1b[sk-abcd1234"),
        ("ABCD5678wxyz", "\x1bABCD5678wxyz"),
        ("ab\x1b[31mc", "ab\x1b[31mc"),
        (
            "window-title\x07SENSITIVE",
            "\x1b]window-title\x07SENSITIVE",
        ),
        ("ABCDEFGH", "\x1bQUJDREVGR0g="),
    ];
    for (value, written) in EVIDENCE_CASES.into_iter().chain(partial_escapes) {
        let bindings = [evidence_binding(name, value.as_bytes())];
        assert_eq!(
            evidence(&format!("diagnostic: {written}\n"), &bindings),
            format!("diagnostic: [secret:{name}]\n"),
            "{value:?} written as {written:?}"
        );
    }

    let short = [
        evidence_binding(name, b"ABC"),
        evidence_binding("ANSI", b"xy\x1b[0mz"),
    ];
    assert_eq!(
        evidence("diagnostic: ABC \x1bABC xyz\n", &short),
        "diagnostic: ABC BC xyz\n"
    );

    let overlapping = [
        evidence_binding(name, b"ABCDEFGH"),
        evidence_binding("OVERLAP", b"EFGHIJKL"),
        evidence_binding("ADJACENT", b"QRSTUVWX"),
    ];
    assert_eq!(
        evidence(
            "\x1b]ABCDEFGH\x07diagnostic: \x1bABCDEFGH\x1b[0mIJKLQR\x1b[0mSTUVWX \
             / QR\x1b[0mSTUVWX\x1bABCDEFGH / EFGH\x1b[0mIJKL\n",
            &overlapping,
        ),
        format!("diagnostic: [secret:{name}] / [secret:ADJACENT] / [secret:OVERLAP]\n")
    );

    let tied = [
        evidence_binding(name, b"ABCDEFGH"),
        evidence_binding("VISIBLE", b"BCDEFGH"),
    ];
    assert_eq!(
        evidence("diagnostic: \x1bABCDEFGH\n", &tied),
        "diagnostic: [secret:VISIBLE]\n"
    );
}

/// Bind directly from a handwritten store, including values too short
/// for the operator's set command and values that are not UTF-8.
#[cfg(test)]
pub(super) fn evidence_binding(name: &str, value: &[u8]) -> secret::BoundSecret {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("secrets.env");
    std::fs::write(&store, [name.as_bytes(), b"=", value, b"\n"].concat()).unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut bindings = secret::resolve_bindings(&store, &[name.to_string()]).unwrap();
    bindings.pop().unwrap()
}

#[cfg(test)]
#[test]
fn stripped_offsets_point_into_raw_stderr() {
    let raw = b"a\x1b[31mb\x1b]title\x07c\x1b]url\x1b\\d\x1bMe\xc3\xa9\x1b]unfinished";
    let (stripped, origins) = without_ansi_with_offsets(raw);
    assert_eq!(stripped, "abcdeé");
    assert_eq!(
        origins,
        [0..1, 6..7, 15..16, 23..24, 26..27, 27..28, 28..29]
    );

    let (stripped, origins) = without_ansi_with_offsets(b"\xc3\x1bM\xa9");
    assert_eq!(stripped, "\u{fffd}\u{fffd}");
    assert_eq!(origins, [0..1, 0..1, 0..1, 3..4, 3..4, 3..4]);
}
