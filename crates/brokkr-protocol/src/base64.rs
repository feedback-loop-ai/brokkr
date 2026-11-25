//! One RFC 4648 base64 codec for the workspace (#1277, decision 0071 ruling 5).
//!
//! Previously `secret.rs` held the workspace's only base64 encoder; it moved
//! here so known-plaintext secret masking (`secret`) and the dsh hands
//! overlay / `hands serve` (`hands`) share one encoder and one strict
//! unpadded base64url decoder, with zero external dependency.

use thiserror::Error;

const B64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Why an unpadded base64url value was refused. No variant carries or echoes
/// the rejected value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Base64UrlError {
    #[error("contains '=' padding, which unpadded base64url forbids")]
    Padding,
    #[error("contains a byte outside the base64url alphabet")]
    InvalidAlphabet,
    #[error("has an invalid base64url length")]
    InvalidLength,
    #[error("has non-zero trailing base64url bits")]
    NonZeroTrailingBits,
    #[error("decoded bytes are not valid UTF-8")]
    NotUtf8,
}

fn encode(bytes: &[u8], alphabet: &[u8; 64], pad: bool) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(alphabet[(n >> 18) as usize & 63] as char);
        out.push(alphabet[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(alphabet[(n >> 6) as usize & 63] as char);
        } else if pad {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(alphabet[n as usize & 63] as char);
        } else if pad {
            out.push('=');
        }
    }
    out
}

pub(crate) fn encode_std(bytes: &[u8]) -> Vec<u8> {
    encode(bytes, B64_STD, true).into_bytes()
}

pub(crate) fn encode_std_nopad(bytes: &[u8]) -> Vec<u8> {
    encode(bytes, B64_STD, false).into_bytes()
}

pub(crate) fn encode_url(bytes: &[u8]) -> Vec<u8> {
    encode(bytes, B64_URL, true).into_bytes()
}

/// Encode `bytes` as RFC 4648 Section 5 base64url without `=` padding.
pub fn encode_base64url(bytes: &[u8]) -> String {
    encode(bytes, B64_URL, false)
}

fn sextet(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

/// Decode strict RFC 4648 Section 5 base64url without `=` padding.
/// Refuses padding, any byte outside `A-Z a-z 0-9 - _`, a `4k + 1` length,
/// and non-zero canonical trailing bits.
pub(crate) fn decode_url_nopad(input: &str) -> Result<Vec<u8>, Base64UrlError> {
    let bytes = input.as_bytes();
    if bytes.contains(&b'=') {
        return Err(Base64UrlError::Padding);
    }
    let sextets: Vec<u8> = bytes
        .iter()
        .map(|&byte| sextet(byte).ok_or(Base64UrlError::InvalidAlphabet))
        .collect::<Result<_, _>>()?;
    let (chunks, tail) = sextets.as_chunks::<4>();
    let mut out = Vec::with_capacity(sextets.len() * 3 / 4);
    for &[a, b, c, d] in chunks {
        out.push((a << 2) | (b >> 4));
        out.push((b << 4) | (c >> 2));
        out.push((c << 6) | d);
    }
    match tail {
        [] => Ok(out),
        [a, b] => {
            if b & 0x0f != 0 {
                return Err(Base64UrlError::NonZeroTrailingBits);
            }
            out.push((a << 2) | (b >> 4));
            Ok(out)
        }
        [a, b, c] => {
            if c & 0x03 != 0 {
                return Err(Base64UrlError::NonZeroTrailingBits);
            }
            out.push((a << 2) | (b >> 4));
            out.push((b << 4) | (c >> 2));
            Ok(out)
        }
        _ => Err(Base64UrlError::InvalidLength),
    }
}

/// Decode strict unpadded base64url into a UTF-8 string.
pub fn decode_base64url(input: &str) -> Result<String, Base64UrlError> {
    let bytes = decode_url_nopad(input)?;
    String::from_utf8(bytes).map_err(|_| Base64UrlError::NotUtf8)
}

#[cfg(test)]
mod tests;
