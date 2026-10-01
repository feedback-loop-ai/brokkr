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

/// Decision 0065: authority data is read with a repeated key refused,
/// because `serde_json` would otherwise obey the second copy in silence.
#[test]
fn a_strict_parse_reads_every_json_shape_exactly_as_the_ordinary_parse_does() {
    let text = r#"{"a":[true,null,-3,4,1.5,"s"],"b":{"c":{}}}"#;
    assert_eq!(
        parse_strict(text).unwrap(),
        serde_json::from_str::<Value>(text).unwrap()
    );
}

#[test]
fn a_strict_parse_refuses_a_key_written_twice_at_any_depth() {
    assert_eq!(
        parse_strict(r#"{"a":1,"a":2}"#).unwrap_err(),
        "key 'a' is written twice at line 1 column 13"
    );
    assert_eq!(
        parse_strict(r#"{"outer":[{"k":1,"k":1}]}"#).unwrap_err(),
        "key 'k' is written twice at line 1 column 23"
    );
    // A malformed document is still serde_json's own refusal.
    assert_eq!(
        parse_strict("{").unwrap_err(),
        "EOF while parsing an object at line 1 column 1"
    );
}

#[test]
fn the_strict_visitor_says_what_it_expects() {
    struct Expecting;
    impl std::fmt::Display for Expecting {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            serde::de::Visitor::expecting(&StrictVisitor, formatter)
        }
    }
    assert_eq!(Expecting.to_string(), "a JSON value");
}
