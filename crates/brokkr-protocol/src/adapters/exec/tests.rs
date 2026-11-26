//! A dialect check's notes are its validator's stdout, its stderr named
//! only by evidence over the masked bytes captured (#1199).

use std::io::{self, Write};

use serde_json::json;

use super::super::{start, write_prompt, AdapterKind};
use super::dialect_notes;
use crate::{env_guard::EnvGuard, process::Evidence, Body, Message, ResultStatus};

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

#[test]
fn a_broken_pipe_on_the_exec_prompt_means_input_was_declined() {
    struct ClosedPipe;

    impl Write for ClosedPipe {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    assert_eq!(write_prompt(&mut ClosedPipe, "prompt"), Ok(()));
}

fn assert_validator_verdict(exit_code: u8, verdict: &str) {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let role = dir.path().join("role.md");
    // The validator cannot accept this whole prompt into the pipe buffer.
    std::fs::write(&role, "x".repeat(1_000_000)).unwrap();
    let (hello, start) = start::hello_and_start(json!({
        "role_path": role,
        "workdir": dir.path(),
        "result_path": dir.path().join("result.json"),
        "dialect_exec": {"success_result": "clear", "failure_result": "ambiguous"},
    }));
    let script = format!(
        "printf 'validator verdict\\n'; printf 'validator diagnostic\\n' >&2; exit {exit_code}"
    );
    let extra = ["sh".into(), "-c".into(), script];
    let input = format!("{hello}\n{start}\n");
    let mut output = Vec::new();
    start::serve_io(AdapterKind::Exec, &extra, input.as_bytes(), &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    let mut lines = output.lines().rev();
    let terminal: Message = serde_json::from_str(lines.next().expect("terminal message")).unwrap();
    let evidence = Evidence::of(b"validator diagnostic\n");
    let notes = format!("validator verdict\nvalidator stderr: {evidence}");
    assert!(
        matches!(&terminal.body, Body::Result {
            status: ResultStatus::Succeeded, result: Some(result), error: None, ..
        } if result["result"] == verdict && result["notes"] == notes),
        "expected validator verdict {verdict:?} with its output, got {terminal:?}"
    );
    let checkpoint: Message = serde_json::from_str(lines.next().expect("exit checkpoint")).unwrap();
    assert!(
        matches!(&checkpoint.body, Body::Checkpoint { data, .. }
            if data["step"] == "exec-session-finished" && data["exit_code"] == exit_code),
        "expected validator exit {exit_code}, got {checkpoint:?}"
    );
}

#[test]
fn a_validator_that_declines_its_prompt_delivers_its_success_verdict() {
    assert_validator_verdict(0, "clear");
}

#[test]
fn a_validator_that_declines_its_prompt_delivers_its_failure_verdict() {
    assert_validator_verdict(7, "ambiguous");
}
