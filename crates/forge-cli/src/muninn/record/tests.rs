use super::*;

use serde_json::json;

#[test]
fn the_record_appends_and_reads_back_in_order() {
    let dir = tempfile::tempdir().unwrap();
    // A nested path proves the parent is created on the way.
    let path = dir.path().join(".forge/muninn.ndjson");
    assert!(read(&path).unwrap().is_empty(), "an absent record is empty");
    append(&path, &json!({"n": 1})).unwrap();
    append(&path, &json!({"n": 2})).unwrap();
    assert_eq!(read(&path).unwrap(), vec![json!({"n": 1}), json!({"n": 2})]);

    // Appending never rewrites: entry one survives entry two verbatim.
    let raw = std::fs::read_to_string(&path).unwrap();
    assert_eq!(raw.lines().count(), 2);
    assert!(raw.starts_with("{\"n\":1}\n"));
}

#[test]
fn a_blank_line_is_skipped_and_a_broken_line_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("muninn.ndjson");
    std::fs::write(&path, "{\"n\":1}\n\n   \nnot-json\n").unwrap();
    let error = read(&path).unwrap_err().to_string();
    assert!(error.contains("line 4"), "{error}");

    std::fs::write(&path, "{\"n\":1}\n\n").unwrap();
    assert_eq!(read(&path).unwrap(), vec![json!({"n": 1})]);
}

#[test]
fn an_unwritable_path_and_an_unreadable_one_are_plain_errors() {
    let error = append(Path::new(""), &json!({})).unwrap_err().to_string();
    assert!(error.contains("opening"), "{error}");

    // A directory is readable as a path but not as a record.
    let dir = tempfile::tempdir().unwrap();
    let error = read(dir.path()).unwrap_err().to_string();
    assert!(error.contains("reading"), "{error}");
}
