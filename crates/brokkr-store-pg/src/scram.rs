//! Pure-Rust RFC 5802 / RFC 7677 SCRAM-SHA-256 authentication using `sha2`.

use sha2::{Digest, Sha256};

use crate::PgError;

const BLOCK_SIZE: usize = 64;
const B64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const GS2_HEADER: &str = "n,,";
const GS2_HEADER_B64: &str = "biws";

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
        let client_first_bare = format!("n={user},r={client_nonce}");
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
        let salted = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iterations);
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

fn parse_server_first<'a>(
    server_first: &'a str,
    client_nonce: &str,
) -> Result<(&'a str, Vec<u8>, u32), PgError> {
    let (mut nonce, mut salt_b64, mut iter_str) = (None, None, None);
    for part in server_first.split(',') {
        if let Some(v) = part.strip_prefix("r=") {
            nonce = Some(v);
        } else if let Some(v) = part.strip_prefix("s=") {
            salt_b64 = Some(v);
        } else if let Some(v) = part.strip_prefix("i=") {
            iter_str = Some(v);
        }
    }
    let nonce = nonce.ok_or_else(|| PgError::Auth("missing nonce in SCRAM server-first".into()))?;
    if !nonce.starts_with(client_nonce) || nonce.len() <= client_nonce.len() {
        return Err(PgError::Auth(
            "SCRAM server nonce does not extend client nonce".into(),
        ));
    }
    let salt = base64_decode(
        salt_b64.ok_or_else(|| PgError::Auth("missing salt in SCRAM server-first".into()))?,
    )?;
    let iterations: u32 = iter_str
        .ok_or_else(|| PgError::Auth("missing iterations in SCRAM server-first".into()))?
        .parse()
        .map_err(|_| PgError::Auth("invalid iterations in SCRAM server-first".into()))?;
    if iterations == 0 {
        return Err(PgError::Auth(
            "SCRAM iteration count must be positive".into(),
        ));
    }
    Ok((nonce, salt, iterations))
}
