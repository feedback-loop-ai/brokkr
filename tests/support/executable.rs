//! A script installed as an executable that no descriptor of the test
//! process ever wrote. The suite forks constantly, and a child forked
//! while this process holds a write descriptor on the inode keeps it
//! until the child execs, so `exec` of the script meets ETXTBSY (#255).
//! The body is written to a sibling source, and `install`, a child that
//! writes the destination and exits, delivers the executable.

use std::path::{Path, PathBuf};

pub(crate) fn install(dir: &Path, name: &str, body: &str) -> PathBuf {
    let source = dir.join(format!("{name}.source"));
    std::fs::write(&source, body).unwrap();
    let path = dir.join(name);
    let status = std::process::Command::new("install")
        .args(["-m", "0755"])
        .arg(&source)
        .arg(&path)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "install {} exited {status}",
        path.display()
    );
    path
}
