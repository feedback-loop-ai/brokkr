//! An absolute deadline is met at its instant (fix round 0106b1r6, D1).

use std::time::{Duration, Instant};

use serde_json::json;

use super::Deadline;
use crate::process::tests::command;
use crate::process::{DriverProcess, SpawnEnv};
use crate::AttemptOutcome;

/// A driver whose instant had already passed when its watchdog armed is
/// killed at once, not after the thirty seconds its deadline was armed
/// with: the watchdog waits to the instant, never a fresh length from
/// its own arming. The kill's words name the armed length.
#[test]
fn a_driver_whose_instant_passed_before_its_watchdog_armed_is_killed_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let armed = Duration::from_secs(30);
    let deadline = Deadline::At {
        kill: Instant::now(),
        armed,
    };
    let began = Instant::now();
    let report = DriverProcess::spawn_by(
        &command("read -r hello; sleep 60"),
        dir.path(),
        deadline,
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
