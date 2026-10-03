//! One test played in an engine of its own (#403, #484).

/// Set in the re-executed test binary: the one test it runs is already in
/// an engine of its own.
const OWN_ENGINE: &str = "BROKKR_TEST_OWN_ENGINE";

/// Whether this test runs in an engine of its own. An in-process launch is
/// one of the test binary's engine's attempts, and every test in the binary
/// shares that engine as its subreaper. An orphan another test hands it (a
/// hands box's `bwrap` namespace init, which outlives the `bwrap` that
/// started it) can read as a stray of this test's launches and refuse them,
/// so such a test re-executes this binary on `test`, its full path, alone.
/// True in the re-executed child, which runs the body; false in the parent,
/// once it has asserted that exactly one test passed there. The child's
/// own output is shown when it did not.
pub fn in_its_own_engine(test: &str) -> bool {
    if std::env::var_os(OWN_ENGINE).is_some() {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test])
        .env(OWN_ENGINE, "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let passed = shown.contains("test result: ok. 1 passed;");
    assert_eq!((output.status.code(), passed), (Some(0), true), "{shown}");
    false
}
