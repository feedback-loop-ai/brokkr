//! A dialect check's notes are its validator's stdout, its stderr named
//! only by evidence over the masked bytes captured (#1199).

use super::dialect_notes;

/// The SHA-256 prefix the evidence line names for `stderr`.
fn digest(stderr: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(stderr))[..16].to_string()
}

#[test]
fn a_dialect_checks_notes_are_its_stdout_and_its_stderr_by_evidence() {
    let said = b"UNBOUND-SENTINEL-1199\n";
    let evidence = format!(
        "validator stderr: 22 bytes (first byte: letter; sha256: {})",
        digest(said)
    );
    assert_eq!(dialect_notes(b"", said), evidence);
    assert_eq!(
        dialect_notes(b"  diag\n", said),
        format!("diag\n{evidence}")
    );
    assert_eq!(
        dialect_notes(b" diag \n", b""),
        "diag",
        "empty stderr adds nothing"
    );
    assert_eq!(dialect_notes(b"\n", b""), "");
}

/// A byte that is not UTF-8 is counted and digested as the one byte it
/// is, never as the three its lossy reading would be.
#[test]
fn a_dialect_checks_stderr_evidence_is_over_its_raw_bytes() {
    assert_eq!(
        dialect_notes(b"", &[0xff]),
        "validator stderr: 1 bytes (first byte: non-ASCII; sha256: a8100ae6aa1940d0)"
    );
    assert_eq!(digest(&[0xff]), "a8100ae6aa1940d0");
}
