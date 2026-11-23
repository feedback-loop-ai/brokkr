//! An absolute deadline is met at its instant (fix rounds 0106b1r6, D1,
//! and 0106b1r9, E1), and one whose monotonic instant has passed is never
//! spawned (the return of fix round 0106b1r12).

use std::time::{Duration, Instant};

use super::By;
use crate::process::tests::command;
use crate::process::{DriverProcess, SpawnEnv, SpawnError};

/// A driver whose monotonic instant had already passed at the fork is not
/// spawned, though the caller's clock still finds thirty seconds left: the
/// fork reads the instant its kill is carried on as well as the caller's
/// clock, and refuses once either has passed.
#[test]
fn a_driver_whose_instant_passed_before_the_fork_is_never_spawned() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran");
    let left = || Some(Duration::from_secs(30));
    let by = By {
        kill: Instant::now(),
        left: &left,
    };
    let refused = DriverProcess::spawn_by(
        &command(&format!("touch {}", marker.display())),
        dir.path(),
        by,
        &SpawnEnv::Inherit,
    )
    .err()
    .expect("a driver past its instant was spawned");
    assert!(matches!(refused, SpawnError::Late { .. }), "{refused:?}");
    std::thread::sleep(Duration::from_millis(200));
    assert!(!marker.exists(), "the driver ran");
}
