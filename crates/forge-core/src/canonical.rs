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

pub const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_sort_at_every_level() {
        let v = json!({"b": {"z": 1, "a": 2}, "a": [{"y": 1, "x": 2}]});
        assert_eq!(
            String::from_utf8(to_bytes(&v)).unwrap(),
            r#"{"a":[{"x":2,"y":1}],"b":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn digest_is_stable() {
        let a = json!({"k": 1, "j": 2});
        let b = json!({"j": 2, "k": 1});
        assert_eq!(sha256_hex(&a), sha256_hex(&b));
    }
}
