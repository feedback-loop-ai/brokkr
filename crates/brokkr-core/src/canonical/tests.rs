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
