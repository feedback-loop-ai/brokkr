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
/// stale command itself is still passed whole to the guard. `printf` writes
/// the bytes exactly, so `output` may end without a newline, which a
/// heredoc cannot express.
fn stub_cargo(dir: &Path, output: &str, status: u8) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("stub bin");
    let cargo = bin.join("cargo");
    let output = output.replace('\'', "'\\''");
    std::fs::write(
        &cargo,
        format!("#!/usr/bin/env bash\nprintf '%s' '{output}'\nexit {status}\n"),
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

/// The whole one-binary command, with a filter, the guard is given when
/// the stub stands in for the build.
const COMMAND: [&str; 8] = [
    "cargo",
    "test",
    "--locked",
    "-p",
    "brokkr-cli",
    "--test",
    "it",
    "packaging::",
];

/// The guard's exact refusal when a summary shows no executed test.
const NO_TEST_REFUSAL: &str = "run-it-tests: the command ran 0 tests, so a filter matched no test";

/// The guard's exact refusal when the run printed no summary.
const NO_SUMMARY_REFUSAL: &str = "run-it-tests: the command printed no test result: summary, so \
                                  no test is shown to have run";

/// The guard's exact refusal when a summary cannot be read.
const UNREAD_REFUSAL: &str = "run-it-tests: the command printed a test result: summary the guard \
                              cannot read, so no test is shown to have run";

/// A summary whose passed count is zero-padded: `00` is not execution.
const ZERO_PADDED_PASSED: &str = "running 1 test\n\ntest result: ok. 00 passed; 0 failed; 0 \
                                  ignored; 0 measured; 467 filtered out; finished in 0.00s\n";

/// A summary whose failed count is zero-padded: `00` is not execution.
const ZERO_PADDED_FAILED: &str = "running 1 test\n\ntest result: ok. 0 passed; 00 failed; 0 \
                                  ignored; 0 measured; 467 filtered out; finished in 0.00s\n";

/// `args` run through the guard with a stub `cargo` first on `PATH` that
/// prints `output` and exits `status`.
fn guard_over(args: &[&str], output: &str, status: u8) -> Output {
    let dir = tempfile::tempdir().expect("scratch");
    stub_cargo(dir.path(), output, status);
    run_guard(dir.path(), args)
}

/// Assert the guard refused `output` at exit 1, printing exactly
/// `refusal` and the command.
fn assert_refused(args: &[&str], output: &str, refusal: &str) {
    let run = guard_over(args, output, 0);
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

/// A stale filter through the guard fails on the run's own zero-executed
/// summary, in the module form and the renamed `--exact` form; the whole
/// command is the one a workflow or script would run.
#[test]
fn the_guard_refuses_a_filtered_run_that_executes_no_test() {
    let module = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "brokkr-cli",
        "--test",
        "it",
        "no_such_module::",
    ];
    let exact = [
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
    ];
    for args in [&module[..], &exact[..]] {
        assert_refused(args, NO_TEST, NO_TEST_REFUSAL);
    }
}

/// A run whose own summary shows no test executed refuses: an ignored-only
/// selection, a run that prints no `test result:` summary, and a summary
/// the guard cannot read each fail with the summary's own text.
#[test]
fn the_guard_refuses_a_run_that_shows_no_executed_test() {
    let rows: [(&str, &str); 3] = [
        (IGNORED_ONLY, NO_TEST_REFUSAL),
        (NO_SUMMARY, NO_SUMMARY_REFUSAL),
        (UNREAD_SUMMARY, UNREAD_REFUSAL),
    ];
    for (output, refusal) in rows {
        assert_refused(&COMMAND, output, refusal);
    }
}

/// The guard rules on every `test result:` summary a run prints, the
/// last line with no newline included, and reads the counts as numbers:
/// a passed summary followed by one that shows no test, one it cannot
/// read, in either order or without a terminator, and a zero-padded
/// count all refuse (#543).
#[test]
fn the_guard_rules_on_every_summary_and_the_unterminated_last_line() {
    let rows: [(String, &str); 7] = [
        (format!("{ONE_TEST}{NO_TEST}"), NO_TEST_REFUSAL),
        (format!("{NO_TEST}{ONE_TEST}"), NO_TEST_REFUSAL),
        (format!("{ONE_TEST}{UNREAD_SUMMARY}"), UNREAD_REFUSAL),
        (
            format!("{ONE_TEST}{}", NO_TEST.trim_end_matches('\n')),
            NO_TEST_REFUSAL,
        ),
        (
            format!("{ONE_TEST}{}", UNREAD_SUMMARY.trim_end_matches('\n')),
            UNREAD_REFUSAL,
        ),
        (ZERO_PADDED_PASSED.to_string(), NO_TEST_REFUSAL),
        (ZERO_PADDED_FAILED.to_string(), NO_TEST_REFUSAL),
    ];
    for (output, refusal) in rows {
        assert_refused(&COMMAND, &output, refusal);
    }
}

/// A run that executes a test passes through with its output unchanged,
/// terminated or with an unterminated last line, and a guard called with
/// no command is refused as usage.
#[test]
fn the_guard_passes_a_run_that_executes_a_test() {
    let dir = tempfile::tempdir().expect("scratch");
    for output in [ONE_TEST, ONE_TEST.trim_end_matches('\n')] {
        stub_cargo(dir.path(), output, 0);
        let run = run_guard(dir.path(), &COMMAND);
        assert_eq!(run.status.code(), Some(0), "{output:?}");
        assert_eq!(String::from_utf8_lossy(&run.stdout), output);
    }
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
    let args = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "brokkr-cli",
        "--test",
        "it",
    ];
    let output = guard_over(&args, FAILED_TEST, 101);
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), FAILED_TEST);
}
