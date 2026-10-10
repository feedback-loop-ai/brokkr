//! What a refusal says of the driver bytes it refused (#433): their
//! length, the class of their first byte and a prefix of their SHA-256,
//! enough to correlate a refusal with what the driver sent, and none of
//! its content. A refusal is journaled, append-only, and a secret a
//! driver printed by accident must not be copied there (decision 0012).

use std::fmt;

use serde_json::error::Category;
use sha2::{Digest, Sha256};

/// Hex digits of the SHA-256 a refusal names: 64 bits.
const DIGEST_HEX: usize = 16;

/// Refused bytes, described without their content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Evidence {
    bytes: usize,
    first: FirstByte,
    digest: String,
}

impl Evidence {
    pub(super) fn of(bytes: &[u8]) -> Self {
        let mut digest = hex::encode(Sha256::digest(bytes));
        digest.truncate(DIGEST_HEX);
        Evidence {
            bytes: bytes.len(),
            first: bytes
                .first()
                .map_or(FirstByte::None, |&byte| FirstByte::of(byte)),
            digest,
        }
    }
}

impl fmt::Display for Evidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} bytes (first byte: {}; sha256: {})",
            self.bytes, self.first, self.digest
        )
    }
}

/// The class of the first refused byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstByte {
    None,
    Letter,
    Digit,
    Punctuation,
    Whitespace,
    Control,
    NonAscii,
}

impl FirstByte {
    fn of(byte: u8) -> Self {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' => FirstByte::Letter,
            b'0'..=b'9' => FirstByte::Digit,
            0x21..=0x2f | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e => FirstByte::Punctuation,
            b'\t' | b'\n' | 0x0c | b'\r' | b' ' => FirstByte::Whitespace,
            0x00..=0x08 | 0x0b | 0x0e..=0x1f | 0x7f => FirstByte::Control,
            0x80..=0xff => FirstByte::NonAscii,
        }
    }
}

impl fmt::Display for FirstByte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FirstByte::None => "none",
            FirstByte::Letter => "letter",
            FirstByte::Digit => "digit",
            FirstByte::Punctuation => "punctuation",
            FirstByte::Whitespace => "whitespace",
            FirstByte::Control => "control",
            FirstByte::NonAscii => "non-ASCII",
        })
    }
}

/// Why a stdout line was not a protocol message: the kind and place of
/// the JSON error, and the line as [`Evidence`]. A JSON error's own
/// text can quote the input (an unknown variant, a string of the wrong
/// type), so it is never used.
pub(super) fn unreadable(error: &serde_json::Error, line: &str) -> String {
    let kind = match error.classify() {
        Category::Io => "read",
        Category::Syntax => "syntax",
        Category::Data => "data",
        Category::Eof => "end-of-input",
    };
    format!(
        "unreadable driver message: a JSON {kind} error at line {} column {} of {}",
        error.line(),
        error.column(),
        Evidence::of(line.as_bytes())
    )
}

#[cfg(test)]
mod tests;
