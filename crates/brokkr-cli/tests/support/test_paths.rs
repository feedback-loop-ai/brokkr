//! The exact coverage gate's test-path vocabulary, for the test binaries
//! that sort the repository's files into test and production. Its one home
//! is `scripts/coverage-exact.sh` (`test_dirs`, `test_files`); the
//! suppressions test holds this reading to that script, line for line.

/// Whether a path is test harness to the exact gate: under a `tests/`,
/// `examples/` or `benches/` directory, or named `tests.rs`, `*_tests.rs`
/// or `*-tests.rs`.
pub(crate) fn is_gate_test_path(path: &str) -> bool {
    let mut parts = path.split('/');
    let name = parts.next_back().unwrap_or("");
    parts.any(|dir| matches!(dir, "tests" | "examples" | "benches"))
        || name == "tests.rs"
        || name
            .strip_suffix("tests.rs")
            .is_some_and(|stem| stem.ends_with(['_', '-']))
}
