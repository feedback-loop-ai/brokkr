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
/// 0057 ruling 2) cannot drift apart on what a digest may look like.
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// A JSON value read with duplicate object keys REFUSED, at every depth.
///
/// `serde_json` keeps the last of two equal keys and says nothing, which
/// is harmless for data nobody rules by and is not harmless for authority
/// data (decision 0065): a grant written twice would be granted as
/// whichever copy came second, and the file a reviewer read would not be
/// the file the engine obeyed. So the new authority documents — a v6
/// realms map, an abstract capability definition, a tool dialect, an
/// adapter's native capabilities — are read through here, and a repeated
/// key is a refusal naming the key.
struct Strict(Value);

struct StrictVisitor;

impl<'de> serde::de::Visitor<'de> for StrictVisitor {
    type Value = Strict;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Strict, E> {
        Ok(Strict(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Strict, E> {
        Ok(Strict(Value::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Strict, E> {
        Ok(Strict(Value::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Strict, E> {
        Ok(Strict(Value::from(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Strict, E> {
        Ok(Strict(Value::String(value.to_string())))
    }

    fn visit_unit<E>(self) -> Result<Strict, E> {
        Ok(Strict(Value::Null))
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
        let mut items = Vec::new();
        while let Some(Strict(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Strict(Value::Array(items)))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
        let mut object = serde_json::Map::new();
        while let Some((key, Strict(value))) = map.next_entry::<String, Strict>()? {
            if object.insert(key.clone(), value).is_some() {
                return Err(serde::de::Error::custom(format!(
                    "key '{key}' is written twice"
                )));
            }
        }
        Ok(Strict(Value::Object(object)))
    }
}

impl<'de> serde::Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Strict, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

/// Parse JSON text, refusing a key written twice in one object. The error
/// is `serde_json`'s own words, position included.
pub fn parse_strict(text: &str) -> Result<Value, String> {
    serde_json::from_str::<Strict>(text)
        .map(|Strict(value)| value)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
