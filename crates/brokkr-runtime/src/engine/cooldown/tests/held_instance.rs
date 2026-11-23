//! Fix round 0106b1r12, S3: a probe's hold names the driver instance that
//! holds it, its probe's own attempt, not only its run. An engine that
//! crashes after its probe's cleanup and before it ends the hold leaves
//! the hold standing; a later drive of the same run probes under its own
//! attempt, writes and ends its own hold, and never ends, revives or adopts
//! the crashed instance's, which stands, named by its attempt, until a
//! settlement ends it.

use super::crash::crashing_clock;
use super::held_route::{hold_end_of, holds_written, legs, probing, unended_holds};
use super::*;

/// The attempt every unended hold in the journal at `path` names.
fn unended_attempts(path: &std::path::Path) -> Vec<String> {
    rusqlite::Connection::open(path)
        .unwrap()
        .prepare("SELECT attempt_id FROM probe_holds WHERE ended_at IS NULL ORDER BY seq")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// The first engine's probe answers and its clock panics at the next read,
/// the answer's: a crash between the probe's cleanup and its hold's end.
/// The resumed drive's own probe answers, the retry succeeds, and of the
/// two holds written only the resumed instance's is ended: the crashed
/// instance's stands, and both production gates read the route cooling
/// down a day past its stored end, naming its attempt.
#[test]
fn a_resume_never_ends_or_adopts_a_crashed_instances_hold() {
    let (dir, mut crashed, now, probe_at, gates) = probing(Probed::Answer);
    crashed.cooldown.clock = crashing_clock(&dir.path().join("work.done"), &now);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crashed.drive()));
    assert!(panic.is_err(), "the clock did not crash after the probe");
    let (compiled, run) = (crashed.bundle.clone(), crashed.run_id.clone());
    let journal = crashed.store.path().to_path_buf();
    drop(crashed);
    let first = unended_attempts(&journal);
    assert_eq!(first.len(), 1, "the crash left no standing hold");

    let mut resumed = super::panel::resume_run(dir.path(), compiled, &run, host(dir.path(), 1, 2));
    resumed.cooldown.clock = Clock::advancing(probe_at).0;
    resumed.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"]
    );
    assert_eq!((holds_written(&journal), unended_holds(&journal)), (2, 1));
    assert_eq!(unended_attempts(&journal), first);
    let later = hold_end_of(probe_at) + time::Duration::days(1);
    for leg in legs(&gates, later) {
        assert!(
            matches!(&leg, Err(crate::admission::Capacity::CoolingDown { words, .. })
                if words.contains(&format!("(attempt {})", first[0]))),
            "{leg:?}"
        );
    }
}
