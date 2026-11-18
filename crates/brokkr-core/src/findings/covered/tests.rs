//! Decision 0109 ruling 7's `head` arm, bound: both field sets, exactly
//! one of them, no `base`, and the digest-pinned projection.

use serde_json::{json, Value};

use super::*;

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

/// The record `covered` holds, if it reads as one.
fn read(covered: &Value) -> Option<Covered> {
    Covered::deserialize(covered).ok()
}

#[test]
fn both_head_field_sets_write_their_own_record_and_read_back() {
    let commit = Covered::commit(HEAD, true).unwrap();
    assert_eq!(
        serde_json::to_value(&commit).unwrap(),
        json!({"kind": "head", "head": HEAD, "dirty": true})
    );
    let digest = Covered::digest_of(HEAD, true);
    let pinned = sha256_bytes(HEAD.as_bytes());
    assert_eq!(
        serde_json::to_value(&digest).unwrap(),
        json!({"kind": "head", "head_digest": pinned, "dirty": true})
    );
    for covered in [commit, digest] {
        let written = serde_json::to_value(&covered).unwrap();
        assert_eq!(read(&written), Some(covered.clone()));
        assert!(covered.dirty());
    }
    assert_eq!(
        Covered::digest_of(HEAD, false).head(),
        &CoveredHead::Digest(pinned)
    );
    assert!(!Covered::digest_of(HEAD, false).dirty());
}

#[test]
fn a_record_names_exactly_one_head_and_no_base() {
    let digest = sha256_bytes(HEAD.as_bytes());
    let refused: [Value; 8] = [
        json!({"kind": "head", "head": HEAD, "head_digest": digest, "dirty": true}),
        json!({"kind": "head", "dirty": true}),
        json!({"kind": "head", "head": HEAD, "base": HEAD, "dirty": true}),
        json!({"kind": "head", "head": HEAD}),
        json!({"kind": "head", "head": "HEAD", "dirty": true}),
        json!({"kind": "head", "head_digest": HEAD, "dirty": true}),
        json!({"kind": "artifacts", "files": [{"path": "a.md", "sha256": digest}]}),
        json!({"head": HEAD, "dirty": true}),
    ];
    for record in refused {
        assert_eq!(read(&record), None, "{record}");
    }
    assert_eq!(Covered::commit("HEAD", true), None);
    assert_eq!(
        Covered::commit(&"a".repeat(64), true).map(|c| c.dirty()),
        Some(true)
    );
}

#[test]
fn a_record_projects_only_the_head_it_names() {
    let other = "fedcba9876543210fedcba9876543210fedcba98";
    for covered in [
        Covered::commit(HEAD, true).unwrap(),
        Covered::digest_of(HEAD, true),
    ] {
        assert!(covered.projects(HEAD), "{covered:?}");
        assert!(!covered.projects(other), "{covered:?}");
    }
    assert_eq!(
        CoveredError::Head.to_string(),
        "a covered head record names exactly one of a commit head or a SHA-256 head_digest"
    );
}
