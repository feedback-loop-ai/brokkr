//! A probe's kill is its deadline's own instant (fix round 0106b1r9, E1).

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use time::OffsetDateTime;

use super::ProbeDeadline;

/// A pause between the fork's reading of the engine's clock and the
/// watchdog's arming moves nothing: the kill is the instant the probe took
/// as it began, whatever the clock read since, and only what the fork
/// found left, the length the kill's words name, shrinks with it.
#[test]
fn a_pause_between_the_clock_samples_leaves_the_kill_at_the_deadline() {
    let began = Instant::now();
    let start = OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_000);
    let deadline = ProbeDeadline::begun(began, start, Duration::from_secs(2));
    assert_eq!(deadline.ends, start + time::Duration::seconds(2));
    assert_eq!(deadline.kill, began + Duration::from_secs(2));
    // Each reading of the engine's clock comes 700 ms after the last.
    let reads = AtomicI64::new(0);
    let now = || start + time::Duration::milliseconds(700 * reads.fetch_add(1, Ordering::SeqCst));
    let left = || deadline.left(now());
    let by = deadline.watched(&left);
    assert_eq!((by.left)(), Some(Duration::from_secs(2)));
    assert_eq!((by.left)(), Some(Duration::from_millis(1_300)));
    assert_eq!(by.kill, began + Duration::from_secs(2));
    assert_eq!((by.left)(), Some(Duration::from_millis(600)));
    assert_eq!((by.left)(), None);
    assert_eq!(by.kill, deadline.kill);
}
