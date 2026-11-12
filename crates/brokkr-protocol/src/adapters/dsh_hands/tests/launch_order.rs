//! The refusal order F4 moved (#1052): the hands sandbox decision, which
//! makes the launch's one `--version` read, now runs before the resume
//! gate, the fresh root and the planned locator. A seat that owes more
//! than one refusal reports the sandbox one.

#![cfg(unix)]

use super::super::Refusal;
use super::sealed;
use crate::env_guard::EnvGuard;
use crate::native_controls::SealedBoundary;

/// The sandbox decision reports before the later root the launch would
/// otherwise fail to stage: with `DSH_HOME` a regular file, the fresh
/// transcript root is a refusal both orders can reach, and this seat also
/// names hands with no access. The sandbox refusal is the one reported, so
/// the order F4 introduced is a fact the test reads.
#[test]
fn a_hands_seat_reports_its_sandbox_refusal_before_a_later_root_refusal() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let not_a_home = dir.path().join("not-a-home");
    std::fs::write(&not_a_home, b"not a directory").unwrap();
    env.set("DSH_HOME", &not_a_home);
    let dsh = super::super::super::tests::dsh_version_shim(dir.path(), "dsh-order", "0.2.1-rc.2");
    let refused = super::super::super::dsh_launch_with(
        dsh.to_str().unwrap(),
        &[],
        dir.path().to_str().unwrap(),
        None,
        &sealed(SealedBoundary::Namespace, None),
        |_| panic!("a refused launch recomputes no composite"),
    )
    .err()
    .expect("the seat owes its sandbox refusal");
    assert_eq!(refused, Refusal::NoAccess.to_string());
}
