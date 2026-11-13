//! The one place a test re-executes its own binary to hold something in a
//! real child process (decision 0108's liveness bindings). Each test
//! binary includes this through `#[path]`; it is test code by its path,
//! so it stays outside the coverage denominator.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Run `test`, the child's full libtest path, as a re-executed copy of the
/// test binary with `vars` set, and wait until it writes `marker`. The
/// caller kills and waits for the child. Under `--exact`, a bare name
/// matches nothing, so the child would hold nothing.
pub(crate) fn spawn_holding(test: &str, vars: &[(&str, &OsStr)], marker: &Path) -> Child {
    let child = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--test-threads=1"])
        .envs(vars.iter().map(|(name, value)| (name, value)))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "the child never held its post");
        std::thread::sleep(Duration::from_millis(10));
    }
    child
}
