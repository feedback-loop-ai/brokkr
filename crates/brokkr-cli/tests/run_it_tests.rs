//! The checked entry point the workflows and scripts run their one-binary
//! test commands through (#543). `cargo test` and libtest pass a run that
//! selected nothing, so a stale filter must fail at the runtime, not by a
//! reading of the command text. `scripts/run-it-tests.sh` is that entry
//! point, and `layering::test_targets` refuses a workflow or script
//! command that bypasses it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::workspace_root::{workspace, GUARD};

/// The runtime's output when a filter matches no test: a summary whose
/// passed and failed counts are both zero.
const NO_TEST: &str = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; \
                       0 measured; 468 filtered out; finished in 0.00s\n";

/// The runtime's output when the only selected test is `#[ignore]`d: the
/// summary shows an ignored test, and ignoring is not execution.
const IGNORED_ONLY: &str = "running 1 test\ntest a_renamed_test ... ignored\n\ntest result: ok. \
                             0 passed; 0 failed; 1 ignored; 0 measured; 467 filtered out; \
                             finished in 0.00s\n";

/// A run that prints no `test result:` summary at all: the guard has no
/// evidence that any test ran.
const NO_SUMMARY: &str = "error: could not compile `brokkr-cli` (test \"it\")\n";

/// A `test result:` line in a shape the guard cannot read.
const UNREAD_SUMMARY: &str = "test result: everything is fine\n";

/// The runtime's output when one test runs.
const ONE_TEST: &str = "running 1 test\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; \
                        0 measured; 467 filtered out; finished in 0.00s\n";

/// The runtime's output when a test fails: the run executed a test, and
/// the command exits non-zero.
const FAILED_TEST: &str = "running 1 test\ntest a_failing_test ... FAILED\n\ntest result: \
                           FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 467 filtered \
                           out; finished in 0.00s\n";

/// A `cargo` stub first on the child's `PATH`, standing in for the build so
/// the test needs no nested one: it prints `output` and exits `status`. The
/// stale command itself is still passed whole to the guard.
fn stub_cargo(dir: &Path, output: &str, status: u8) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("stub bin");
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        format!("#!/usr/bin/env bash\ncat <<'BROKKR_STUB'\n{output}BROKKR_STUB\nexit {status}\n"),
    )
    .expect("stub cargo");
    let mut mode = std::fs::metadata(&cargo).expect("stub").permissions();
    mode.set_mode(0o755);
    std::fs::set_permissions(&cargo, mode).expect("stub mode");
    cargo
}

/// `args` run through the guard, with a `cargo` stub first on `PATH`.
fn run_guard(dir: &Path, args: &[&str]) -> Output {
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").expect("PATH")
    );
    Command::new("bash")
        .arg(workspace().join(GUARD))
        .args(args)
        .env("PATH", path)
        .output()
        .expect("the guard runs")
}

/// A stale filter through the guard fails with the line the run printed,
/// in the module form and the renamed `--exact` form; the whole command is
/// the one a workflow or script would run.
#[test]
fn the_guard_refuses_a_filtered_run_that_executes_no_test() {
    let dir = tempfile::tempdir().expect("scratch");
    stub_cargo(dir.path(), NO_TEST, 0);
    let rows: [&[&str]; 2] = [
        &[
            "cargo",
            "test",
            "--locked",
            "-p",
            "brokkr-cli",
            "--test",
            "it",
            "no_such_module::",
        ],
        &[
            "cargo",
            "test",
            "--locked",
            "-p",
            "brokkr-cli",
            "--test",
            "it",
            "--",
            "--exact",
            "packaging::a_renamed_away_test",
        ],
    ];
    for args in rows {
        let output = run_guard(dir.path(), args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "run-it-tests: the command ran 0 tests, so a filter matched no test: {}\n",
                args.join(" ")
            ),
            "{}",
            args.join(" ")
        );
    }
}

/// A run whose own summary shows no test executed refuses: an ignored-only
/// selection, a run that prints no `test result:` summary, and a summary
/// the guard cannot read each fail with the summary's own text.
#[test]
fn the_guard_refuses_a_run_that_shows_no_executed_test() {
    let args = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "brokkr-cli",
        "--test",
        "it",
        "packaging::",
    ];
    let rows: [(&str, &str); 3] = [
        (
            IGNORED_ONLY,
            "run-it-tests: the command ran 0 tests, so a filter matched no test",
        ),
        (
            NO_SUMMARY,
            "run-it-tests: the command printed no test result: summary, so no test is shown to \
             have run",
        ),
        (
            UNREAD_SUMMARY,
            "run-it-tests: the command printed a test result: summary the guard cannot read, so \
             no test is shown to have run",
        ),
    ];
    for (output, refusal) in rows {
        let dir = tempfile::tempdir().expect("scratch");
        stub_cargo(dir.path(), output, 0);
        let run = run_guard(dir.path(), &args);
        assert_eq!(
            run.status.code(),
            Some(1),
            "{output:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run.stderr),
            format!("{refusal}: {}\n", args.join(" ")),
            "{output:?}"
        );
    }
}

/// A run that executes a test passes through with its output unchanged,
/// and a guard called with no command is refused as usage.
#[test]
fn the_guard_passes_a_run_that_executes_a_test() {
    let dir = tempfile::tempdir().expect("scratch");
    stub_cargo(dir.path(), ONE_TEST, 0);
    let args = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "brokkr-cli",
        "--test",
        "it",
        "packaging::",
    ];
    let output = run_guard(dir.path(), &args);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), ONE_TEST);
    let usage = run_guard(dir.path(), &[]);
    assert_eq!(usage.status.code(), Some(2), "{usage:?}");
    assert_eq!(
        String::from_utf8_lossy(&usage.stderr),
        "run-it-tests: no command given; pass a whole test command\n"
    );
}

/// A command that fails with a readable summary passes through with its own
/// status, exactly: the guard's own refusal is the only exit that replaces
/// it.
#[test]
fn the_guard_passes_a_failing_command_with_the_commands_own_status() {
    let dir = tempfile::tempdir().expect("scratch");
    stub_cargo(dir.path(), FAILED_TEST, 101);
    let args = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "brokkr-cli",
        "--test",
        "it",
    ];
    let output = run_guard(dir.path(), &args);
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), FAILED_TEST);
}
