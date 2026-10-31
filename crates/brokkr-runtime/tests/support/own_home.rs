//! An engine driven in a test's own process admits each attempt in a
//! coordination root under that process's HOME (decision 0065 slice two,
//! U8a2a). A test binary's HOME is the operator's, and only the house's
//! guard may write the process environment (decision 0071 ruling 9), so a
//! test that drives one runs again, alone, in a child process whose HOME
//! is a fresh owner-only directory of its own, and whose XDG_CONFIG_HOME,
//! which decision 0083 gives precedence over HOME for the personal shelf
//! and host configuration, is that HOME's own `.config` ([`in_own_home`]),
//! whatever the parent inherited; and every helper that drives one first
//! checks it stands in such a process ([`required`]), so a later test that
//! calls the helper is held to it too.

use std::os::unix::fs::DirBuilderExt;
use std::process::Command;
use std::time::{Duration, Instant};

/// Set in the child: its HOME is its own.
const OWN: &str = "BROKKR_TEST_OWN_HOME";

/// The longest the child is waited on before it is killed and reaped.
const PATIENCE: Duration = Duration::from_secs(300);

/// Whether this process is the child that runs `test` under a HOME of its
/// own, where the caller runs its body; otherwise `test` was run so, within
/// [`PATIENCE`], and passed, and the caller returns.
pub(crate) fn in_own_home(test: &str) -> bool {
    if std::env::var_os(OWN).is_some() {
        return true;
    }
    let dir = tempfile::tempdir().unwrap();
    // HOME's parent protects it whatever the umask made it: past the
    // fence, lifted on Linux (U9b1), a coordination root below a
    // group-writable ancestor refuses every dispatch (U8a2a; MB3).
    let private = std::os::unix::fs::PermissionsExt::from_mode(0o700);
    std::fs::set_permissions(dir.path(), private).unwrap();
    let home = dir.path().join("home");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&home)
        .unwrap();
    let out = std::fs::File::create(dir.path().join("out")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env(OWN, "1")
        .stderr(out.try_clone().unwrap())
        .stdout(out)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + PATIENCE;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().ok();
            child.wait().ok();
            panic!("{test} never ended under a HOME of its own");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let printed = std::fs::read_to_string(dir.path().join("out")).unwrap();
    let ran = status.success() & printed.contains("test result: ok. 1 passed");
    assert!(ran, "{test} under a HOME of its own: {status}\n{printed}");
    false
}

/// Refuses to go on unless this process is such a child.
pub(crate) fn required() {
    assert!(
        std::env::var_os(OWN).is_some(),
        "an engine is driven only in a process whose HOME is its own (support/own_home.rs)"
    );
}
