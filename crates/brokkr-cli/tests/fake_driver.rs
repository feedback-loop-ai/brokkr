//! Regression for #1245. The child's stdin is `/dev/null`, so the test
//! finishes even when the test runner's stdin is a terminal.
//!
//! `--effort` rides the same argv as `--model` (decision 0035 ruling 5),
//! so a proof can assert that either pin reached the driver. The verb
//! returns an error when its script is missing.

use std::path::Path;
use std::process::Stdio;

#[test]
fn the_fake_driver_takes_both_pins_and_reports_a_script_it_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let run = |script: &Path| {
        crate::built::brokkr(["fake-driver", "--script"])
            .arg(script)
            .arg("--state")
            .arg(&state)
            .args(["--model", "model-x", "--effort", "xhigh"])
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };

    let refused = run(&dir.path().join("no-such-script.json"));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert_eq!(refused.status.code(), Some(1), "missing script: {stderr}");
    assert_eq!(stderr, "error: No such file or directory (os error 2)\n");

    let script = dir.path().join("script.json");
    std::fs::write(&script, r#"{"seats": {}}"#).unwrap();
    let accepted = run(&script);
    assert_eq!(
        accepted.status.code(),
        Some(0),
        "readable script: {}",
        String::from_utf8_lossy(&accepted.stderr)
    );
}
