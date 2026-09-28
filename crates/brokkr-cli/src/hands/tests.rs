use super::*;

/// A session that cannot lock stops `hands serve` before it serves, and
/// the one line the binary prints names the lock and the kernel's words.
#[test]
fn a_session_that_cannot_lock_prints_the_lock_and_the_errno() {
    let tmp = tempfile::tempdir().unwrap();
    let nolck = || std::io::Error::from_raw_os_error(rustix::io::Errno::NOLCK.raw_os_error());
    let tree = tmp.path().join("brokkr-hands-serve-tree");
    let refused = |label: &str| {
        assert_eq!(label, "serve");
        Err(SessionError::Lock {
            path: tree.join(".owner.lock"),
            cause: nolck(),
        })
    };
    let serve = HandsCommand::Serve {
        workdir: tmp.path().to_path_buf(),
        spec: "\"workspace\"".to_string(),
    };
    let error = run_with(serve, refused).unwrap_err();
    let mut stderr = Vec::new();
    assert_eq!(crate::report_to(&error, &mut stderr), ExitCode::from(1));
    assert_eq!(
        String::from_utf8(stderr).unwrap(),
        format!(
            "error: hands session: cannot lock {}/.owner.lock: {}\n",
            tree.display(),
            nolck()
        )
    );
}
