//! An absolute deadline is met at its instant (fix rounds 0106b1r6, D1,
//! and 0106b1r9, E1).

use std::time::{Duration, Instant};

use serde_json::json;

use super::By;
use crate::process::tests::command;
use crate::process::{DriverProcess, SpawnEnv};
use crate::AttemptOutcome;

/// A driver whose instant had already passed when its watchdog armed is
/// killed at once, not after the thirty seconds the fork found left on
/// the caller's clock: the watchdog waits to the instant, never a fresh
/// length from its own arming. The kill's words name the length the fork
/// found.
#[test]
fn a_driver_whose_instant_passed_before_its_watchdog_armed_is_killed_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let left = || Some(Duration::from_secs(30));
    let by = By {
        kill: Instant::now(),
        left: &left,
    };
    let began = Instant::now();
    let report = DriverProcess::spawn_by(
        &command("read -r hello; sleep 60"),
        dir.path(),
        by,
        &SpawnEnv::Inherit,
    )
    .unwrap()
    .run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {});
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "{:?}",
        began.elapsed()
    );
    assert!(report.deadline_killed);
    assert_eq!(
        report.outcome,
        AttemptOutcome::Indeterminate {
            reason: "attempt exceeded its 30s deadline and was killed; its process tree is \
                     gone, and what it did is unknown"
                .into(),
        }
    );
}
