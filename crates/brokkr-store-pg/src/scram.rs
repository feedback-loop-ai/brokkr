//! Pure-Rust RFC 5802 / RFC 7677 SCRAM-SHA-256 authentication using `sha2`,
//! with RFC 4013 SASLprep mapping and PostgreSQL `pg_saslprep` fallback.

use std::borrow::Cow;

use sha2::{Digest, Sha256};

use crate::PgError;

const BLOCK_SIZE: usize = 64;
const B64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const GS2_HEADER: &str = "n,,";
const GS2_HEADER_B64: &str = "biws";
/// Upper bound on PBKDF2 iterations accepted from a server (DoS boundary).
pub(crate) const MAX_SCRAM_ITERATIONS: u32 = 1_000_000;

/// Compute RFC 2104 HMAC-SHA256.
pub(crate) fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = Sha256::digest(key);
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

/// Compute single-block PBKDF2-HMAC-SHA256 (32-byte derived key for SCRAM-SHA-256).
pub(crate) fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut initial = Vec::with_capacity(salt.len() + 4);
    initial.extend_from_slice(salt);
    initial.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha256(password, &initial);
    let mut acc = u;
    for _ in 1..iterations {
        u = hmac_sha256(password, &u);
        for i in 0..32 {
            acc[i] ^= u[i];
        }
    }
    acc
}

/// Standard RFC 4648 base64 encoding with `=` padding.
pub(crate) fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(B64_TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64_TABLE[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64_TABLE[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Standard RFC 4648 base64 decoding, refusing non-canonical length or alphabet.
pub(crate) fn base64_decode(input: &str) -> Result<Vec<u8>, PgError> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return Err(PgError::Auth(
            "invalid base64 length in SCRAM message".into(),
        ));
    }
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    for (idx, chunk) in bytes.as_chunks::<4>().0.iter().enumerate() {
        let is_last = (idx + 1) * 4 == bytes.len();
        decode_quad(chunk, is_last, &mut out)?;
    }
    Ok(out)
}

fn decode_quad(chunk: &[u8], is_last: bool, out: &mut Vec<u8>) -> Result<(), PgError> {
    let c0 = b64_val(chunk[0])?;
    let c1 = b64_val(chunk[1])?;
    if chunk[2] == b'=' {
        if !is_last || chunk[3] != b'=' {
            return Err(PgError::Auth(
                "invalid base64 padding in SCRAM message".into(),
            ));
        }
        out.push((c0 << 2) | (c1 >> 4));
        return Ok(());
    }
    let c2 = b64_val(chunk[2])?;
    if chunk[3] == b'=' {
        if !is_last {
            return Err(PgError::Auth(
                "invalid base64 padding in SCRAM message".into(),
            ));
        }
        out.push((c0 << 2) | (c1 >> 4));
        out.push((c1 << 4) | (c2 >> 2));
        return Ok(());
    }
    let c3 = b64_val(chunk[3])?;
    out.push((c0 << 2) | (c1 >> 4));
    out.push((c1 << 4) | (c2 >> 2));
    out.push((c2 << 6) | c3);
    Ok(())
}

fn b64_val(byte: u8) -> Result<u8, PgError> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(PgError::Auth(
            "invalid base64 character in SCRAM message".into(),
        )),
    }
}

/// Constant-time equality for cryptographic MAC verification.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    std::hint::black_box(diff) == 0
}

/// Apply RFC 4013 SASLprep mapping (strip Table B.1 commonly-mapped-to-nothing,
/// map Table C.1.2 non-ASCII spaces to ASCII space), falling back to raw bytes
/// if any prohibited character is encountered (matching PostgreSQL `pg_saslprep`).
pub(crate) fn saslprep_password(password: &str) -> Cow<'_, str> {
    let mut out = String::with_capacity(password.len());
    for ch in password.chars() {
        if is_mapped_to_nothing(ch) {
            continue;
        }
        if is_non_ascii_space(ch) {
            out.push(' ');
            continue;
        }
        if is_prohibited_saslprep(ch) {
            return Cow::Borrowed(password);
        }
        out.push(ch);
    }
    Cow::Owned(out)
}

fn is_mapped_to_nothing(ch: char) -> bool {
    matches!(
        ch,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{1806}'
            | '\u{180B}'..='\u{180D}'
            | '\u{200B}'..='\u{200D}'
            | '\u{2060}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
    )
}

fn is_non_ascii_space(ch: char) -> bool {
    matches!(
        ch,
        '\u{00A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

fn is_prohibited_saslprep(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        ch,
        '\u{0000}'..='\u{001F}'
            | '\u{007F}'..='\u{009F}'
            | '\u{0340}'
            | '\u{0341}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{180E}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2061}'..='\u{2063}'
            | '\u{206A}'..='\u{206F}'
            | '\u{2FF0}'..='\u{2FFB}'
            | '\u{E000}'..='\u{F8FF}'
            | '\u{FDD0}'..='\u{FDEF}'
            | '\u{FFF9}'..='\u{FFFD}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
            | '\u{F0000}'..='\u{FFFFD}'
            | '\u{100000}'..='\u{10FFFD}'
    ) || (code & 0xFFFE) == 0xFFFE
}

fn escape_saslname(user: &str) -> String {
    user.replace('=', "=3D").replace(',', "=2C")
}

/// Client state machine for a single SCRAM-SHA-256 authentication exchange.
pub(crate) struct ScramClient {
    client_nonce: String,
    client_first_bare: String,
    expected_server_sig: Option<[u8; 32]>,
}

impl ScramClient {
    pub(crate) fn new(client_nonce: String) -> Self {
        Self::with_user("", client_nonce)
    }

    pub(crate) fn with_user(user: &str, client_nonce: String) -> Self {
        let escaped = escape_saslname(user);
        let client_first_bare = format!("n={escaped},r={client_nonce}");
        Self {
            client_nonce,
            client_first_bare,
            expected_server_sig: None,
        }
    }

    pub(crate) fn client_first_message(&self) -> String {
        format!("{GS2_HEADER}{}", self.client_first_bare)
    }

    pub(crate) fn process_server_first(
        &mut self,
        password: &str,
        server_first: &str,
    ) -> Result<String, PgError> {
        let (nonce, salt, iterations) = parse_server_first(server_first, &self.client_nonce)?;
        let prepared = saslprep_password(password);
        let salted = pbkdf2_hmac_sha256(prepared.as_bytes(), &salt, iterations);
        let client_key = hmac_sha256(&salted, b"Client Key");
        let stored_key: [u8; 32] = Sha256::digest(client_key).into();
        let final_no_proof = format!("c={GS2_HEADER_B64},r={nonce}");
        let auth_msg = format!("{},{server_first},{final_no_proof}", self.client_first_bare);
        let client_sig = hmac_sha256(&stored_key, auth_msg.as_bytes());
        let mut proof = [0u8; 32];
        for i in 0..32 {
            proof[i] = client_key[i] ^ client_sig[i];
        }
        let server_key = hmac_sha256(&salted, b"Server Key");
        self.expected_server_sig = Some(hmac_sha256(&server_key, auth_msg.as_bytes()));
        Ok(format!("{final_no_proof},p={}", base64_encode(&proof)))
    }

    pub(crate) fn verify_server_final(&self, server_final: &str) -> Result<(), PgError> {
        let expected = self
            .expected_server_sig
            .ok_or_else(|| PgError::Auth("SCRAM final received before server-first".into()))?;
        let mut verified = None;
        for part in server_final.split(',') {
            if let Some(err) = part.strip_prefix("e=") {
                return Err(PgError::Auth(format!("SCRAM server error: {err}")));
            }
            if let Some(val) = part.strip_prefix("v=") {
                verified = Some(base64_decode(val)?);
            }
        }
        let sig = verified
            .ok_or_else(|| PgError::Auth("missing server verifier in SCRAM final".into()))?;
        if !constant_time_eq(&sig, &expected) {
            return Err(PgError::Auth("SCRAM server signature mismatch".into()));
        }
        Ok(())
    }
}

#[derive(Default)]
struct ServerFirstAttrs<'a> {
    nonce: Option<&'a str>,
    salt_b64: Option<&'a str>,
    iter_str: Option<&'a str>,
}

fn parse_server_first_attrs(server_first: &str) -> Result<ServerFirstAttrs<'_>, PgError> {
    let mut attrs = ServerFirstAttrs::default();
    for part in server_first.split(',') {
        if part.starts_with("m=") {
            return Err(PgError::Auth(
                "unsupported mandatory SCRAM extension".into(),
            ));
        }
        if let Some(v) = part.strip_prefix("r=") {
            attrs.nonce = Some(v);
        } else if let Some(v) = part.strip_prefix("s=") {
            attrs.salt_b64 = Some(v);
        } else if let Some(v) = part.strip_prefix("i=") {
            attrs.iter_str = Some(v);
        }
    }
    Ok(attrs)
}

fn parse_server_first<'a>(
    server_first: &'a str,
    client_nonce: &str,
) -> Result<(&'a str, Vec<u8>, u32), PgError> {
    let attrs = parse_server_first_attrs(server_first)?;
    let nonce = attrs
        .nonce
        .ok_or_else(|| PgError::Auth("missing nonce in SCRAM server-first".into()))?;
    if !nonce.starts_with(client_nonce) || nonce.len() <= client_nonce.len() {
        return Err(PgError::Auth(
            "SCRAM server nonce does not extend client nonce".into(),
        ));
    }
    let salt = base64_decode(
        attrs
            .salt_b64
            .ok_or_else(|| PgError::Auth("missing salt in SCRAM server-first".into()))?,
    )?;
    let iterations: u32 = attrs
        .iter_str
        .ok_or_else(|| PgError::Auth("missing iterations in SCRAM server-first".into()))?
        .parse()
        .map_err(|_| PgError::Auth("invalid iterations in SCRAM server-first".into()))?;
    if !(1..=MAX_SCRAM_ITERATIONS).contains(&iterations) {
        return Err(PgError::Auth(format!(
            "SCRAM iteration count {iterations} outside 1..={MAX_SCRAM_ITERATIONS}"
        )));
    }
    Ok((nonce, salt, iterations))
}
