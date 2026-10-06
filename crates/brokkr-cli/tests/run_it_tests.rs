//! The checked entry point the workflows and scripts run their one-binary
//! test commands through (#543). `cargo test` and libtest pass a run that
//! selected nothing, so a stale filter must fail at the runtime, not by a
//! reading of the command text. `scripts/run-it-tests.sh` is that entry
//! point, and `layering::test_targets` refuses a workflow or script
//! command that bypasses it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::workspace_root::workspace;

/// The checked entry point, from the workspace root.
const GUARD: &str = "scripts/run-it-tests.sh";

/// The runtime's output when a filter matches no test: the line the guard
/// reads, and the summary beside it.
const NO_TEST: &str = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; \
                       0 measured; 468 filtered out; finished in 0.00s\n";

/// The runtime's output when one test runs.
const ONE_TEST: &str = "running 1 test\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; \
                        0 measured; 467 filtered out; finished in 0.00s\n";

/// A `cargo` stub first on the child's `PATH`, standing in for the build so
/// the test needs no nested one: it prints `output` and exits 0. The stale
/// command itself is still passed whole to the guard.
fn stub_cargo(dir: &Path, output: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("stub bin");
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        format!("#!/usr/bin/env bash\ncat <<'BROKKR_STUB'\n{output}BROKKR_STUB\n"),
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
    stub_cargo(dir.path(), NO_TEST);
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

/// A run that executes a test passes through with its output unchanged,
/// and a guard called with no command is refused as usage.
#[test]
fn the_guard_passes_a_run_that_executes_a_test() {
    let dir = tempfile::tempdir().expect("scratch");
    stub_cargo(dir.path(), ONE_TEST);
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
