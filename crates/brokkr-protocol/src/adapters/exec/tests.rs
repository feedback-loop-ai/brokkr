//! A dialect check's notes are its validator's stdout, its stderr named
//! only by evidence (#1199).

use super::dialect_notes;

/// The SHA-256 prefix the evidence line names for `stderr`.
fn digest(stderr: &str) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(stderr.as_bytes()))[..16].to_string()
}

#[test]
fn a_dialect_checks_notes_are_its_stdout_and_its_stderr_by_evidence() {
    let said = "UNBOUND-SENTINEL-1199\n";
    let evidence = format!(
        "validator stderr: 22 bytes (first byte: letter; sha256: {})",
        digest(said)
    );
    assert_eq!(dialect_notes("", said), evidence);
    assert_eq!(dialect_notes("  diag\n", said), format!("diag\n{evidence}"));
    assert_eq!(
        dialect_notes(" diag \n", ""),
        "diag",
        "empty stderr adds nothing"
    );
    assert_eq!(dialect_notes("\n", ""), "");
}
