//! Content-free evidence (#433): each byte's class, the text of each
//! class and of each JSON error kind, pinned once.

use super::*;

#[test]
fn evidence_names_the_length_the_first_byte_class_and_a_digest_prefix() {
    assert_eq!(
        Evidence::of(b"not json\n").to_string(),
        "9 bytes (first byte: letter; sha256: 3c48773b404d8500)"
    );
    assert_eq!(
        Evidence::of(b"").to_string(),
        "0 bytes (first byte: none; sha256: e3b0c44298fc1c14)"
    );
}

#[test]
fn every_byte_has_the_class_its_ascii_predicates_give_it() {
    for byte in 0..=u8::MAX {
        let expected = if byte.is_ascii_alphabetic() {
            "letter"
        } else if byte.is_ascii_digit() {
            "digit"
        } else if byte.is_ascii_punctuation() {
            "punctuation"
        } else if byte.is_ascii_whitespace() {
            "whitespace"
        } else if byte.is_ascii_control() {
            "control"
        } else {
            "non-ASCII"
        };
        assert_eq!(
            FirstByte::of(byte).to_string(),
            expected,
            "byte {byte:#04x}"
        );
    }
}

/// Each JSON error kind is named, with its place, and the line only as
/// evidence; a read error, which a string never meets, included.
#[test]
fn an_unreadable_line_names_the_error_kind_and_place_and_no_content() {
    let read = serde_json::Error::io(std::io::Error::other("injected"));
    for (line, error, expected) in [
        ("{\n", None, "end-of-input error at line 2 column 0 of 2 bytes (first byte: punctuation; sha256: a6fb08fda1acb957)"),
        ("{\"proto\":\"forge-driver/v1\"}\n", None, "data error at line 1 column 27 of 28 bytes (first byte: punctuation; sha256: be653b846244c6ec)"),
        ("not json\n", None, "syntax error at line 1 column 2 of 9 bytes (first byte: letter; sha256: 3c48773b404d8500)"),
        ("x", Some(read), "read error at line 0 column 0 of 1 bytes (first byte: letter; sha256: 2d711642b726b044)"),
    ] {
        let error =
            error.unwrap_or_else(|| serde_json::from_str::<crate::Message>(line).unwrap_err());
        assert_eq!(
            unreadable(&error, line),
            format!("unreadable driver message: a JSON {expected}")
        );
    }
}
