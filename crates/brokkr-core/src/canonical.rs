//! Canonical JSON bytes: keys sorted lexicographically at every level,
//! compact separators, UTF-8. serde_json's default `Map` is a BTreeMap,
//! so `to_string` over a `Value` built without the `preserve_order`
//! feature already emits sorted keys; this module pins that property.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Canonical bytes of a JSON value (sorted keys, compact).
pub fn to_bytes(value: &Value) -> Vec<u8> {
    // Values pass through Value::Object = BTreeMap, so serialization is
    // key-sorted; compact is serde_json's default `to_string`.
    serde_json::to_vec(value).expect("JSON value serialization cannot fail")
}

/// SHA-256 hex digest of the canonical bytes.
pub fn sha256_hex(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(to_bytes(value));
    hex::encode(hasher.finalize())
}

/// SHA-256 hex digest of raw bytes (bundle files, artifacts).
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Is this text the shape a sha256 is written in — exactly 64 lowercase
/// hex characters? One spelling of "malformed digest" for every reader,
/// so a dispatch envelope's pins and a realm's crossing pins (decision
/// 0054 ruling 2) cannot drift apart on what a digest may look like.
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(test)]
mod tests;
