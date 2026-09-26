//! The CPU budgets for the shipped binary (#342): instruction counts under
//! Callgrind for startup and for `inspect` over a real exported run. CI runs
//! these on Linux against the merge-base in the same job and fails a
//! regression over 2%.

use std::path::PathBuf;
use std::process::Command;

use gungraun::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Sandbox, Stdio,
};

const BINARY: &str = env!("CARGO_BIN_EXE_brokkr");

/// A real exported run, with its manifest beside it, as `import` requires.
const RUN: &str = "tui-graph-the-selection-box-gets-80f98deb";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/journals")
        .join(format!("{RUN}.ndjson"))
}

/// Adopt the fixture into `journal.db` in the sandbox, outside the measure.
fn import_fixture() {
    let status = Command::new(BINARY)
        .args(["import", "--db", "journal.db", "--from"])
        .arg(fixture())
        .stdout(std::process::Stdio::null())
        .status()
        .expect("brokkr import starts");
    assert!(status.success(), "brokkr import adopts the fixture run");
}

#[binary_benchmark]
#[bench::startup()]
fn version() -> gungraun::Command {
    gungraun::Command::new(BINARY)
        .arg("--version")
        .stdout(Stdio::Null)
        .build()
}

#[binary_benchmark]
#[bench::fixture_run(
    setup = import_fixture(),
    config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(true))
)]
fn inspect() -> gungraun::Command {
    gungraun::Command::new(BINARY)
        .args(["inspect", "--db", "journal.db", "--run", RUN])
        .stdout(Stdio::Null)
        .build()
}

binary_benchmark_group!(name = binary; benchmarks = version, inspect);

main!(binary_benchmark_groups = binary);
