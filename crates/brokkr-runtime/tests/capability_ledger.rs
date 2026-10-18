//! Decision 0065 slice two, U8a2a (CR2, MB3): a writer lease is a durable
//! reservation in the protected inventory, released only once its
//! attempt's owned processes have durably settled. Real engines, each in a
//! process of its own on one protected HOME (`writers`): an engine killed
//! mid-attempt, its driver surviving it, keeps its worktree reserved
//! through the lock's release, the driver's own end and the run's resume,
//! for none of them proves the attempt's tree over. A production
//! resolution proof, its root resolved from HOME (M5): the same retention
//! on an injected root, on every TMPDIR, is
//! `a_lease_its_engine_never_released_keeps_refusing_with_the_lock_free`.

use crate::writers::{held_by, until, Coordination};

/// Engine death with surviving payload retains the reservation: the
/// killed engine's lease stands byte for byte, refusing a second run on
/// its worktree by CR2's cause while its driver still runs, after that
/// driver ends, and after the run is resumed and settles the attempt it
/// lost as indeterminate.
#[test]
fn an_engine_killed_mid_attempt_keeps_its_worktree_reserved() {
    let Some(host) = Coordination::new() else {
        return;
    };
    let shared = host.worktree("shared");
    let lost = host.bundle("lost", true);
    let first = host.start(&shared, &lost);
    let run = first.run.clone();
    let lease = host.reservation(&shared, &run);
    until("the first driver runs under its lease", || {
        host.mark("lost").exists() && lease.exists()
    });
    let sealed = std::fs::read(&lease).unwrap();
    first.killed();

    let refused = |name: &str| {
        let bundle = host.bundle(name, false);
        let second = host.start(&shared, &bundle);
        let second_run = second.run.clone();
        assert_eq!(second.ended(), "awaiting_operator", "{name}");
        assert_eq!(
            host.failures(&bundle, &second_run),
            [held_by(&run)],
            "{name}"
        );
        assert!(
            !host.mark(name).exists(),
            "{name}: its driver never started"
        );
        assert_eq!(
            std::fs::read(&lease).unwrap(),
            sealed,
            "{name}: the lease as sealed"
        );
    };
    refused("while-it-runs");
    // The surviving driver ends: its tree is gone, but nothing journaled
    // says so.
    std::fs::write(host.gate(), "").unwrap();
    refused("once-it-ended");
    assert_eq!(
        host.resume(&shared, &lost, &run).ended(),
        "awaiting_operator"
    );
    refused("after-the-resume");
}
