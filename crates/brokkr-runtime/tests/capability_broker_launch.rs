//! Decision 0065 slice two, U8a2a (CR2, MB3): every dispatch takes its
//! worktree's writer lease before its first writer starts, and holds it
//! until its attempt has settled. Real engines, each in a process of its
//! own on one protected HOME (`writers`): an already-running writer
//! refuses a second run on its worktree, a disjoint worktree runs beside
//! it, and of two runs started together on one worktree exactly one runs.
//! Each is a production resolution proof, its root resolved from HOME
//! (M5): the same behaviour on an injected root, on every TMPDIR, is
//! `a_dispatch_on_a_held_worktree_fails_before_its_driver_starts`,
//! `separate_worktrees_hold_their_leases_side_by_side`,
//! `dispatch_holds_the_lease_while_its_driver_runs_and_releases_it_once_settled`
//! and `simultaneous_admissions_on_one_worktree_admit_exactly_one`.

use crate::writers::{held_by, until, Coordination};

/// An already-running writer refuses: while one run's driver works its
/// worktree, a second run there fails its attempt by CR2's cause before its
/// driver starts, and a run on another worktree completes beside it. Once
/// the first settles, its lease is gone and the next run on its worktree
/// completes, each reservation holding no attempt standing.
#[test]
fn a_running_writer_holds_its_worktree_and_a_disjoint_one_runs_beside_it() {
    let Some(host) = Coordination::new() else {
        return;
    };
    let (shared, apart) = (host.worktree("shared"), host.worktree("apart"));
    let (holding, refused) = (host.bundle("holding", true), host.bundle("refused", false));
    let (beside, after) = (host.bundle("beside", false), host.bundle("after", false));
    let first = host.start(&shared, &holding);
    let lease = host.reservation(&shared, &first.run);
    until("the first driver runs under its lease", || {
        host.mark("holding").exists() && lease.exists()
    });

    let second = host.start(&shared, &refused);
    let run = second.run.clone();
    assert_eq!(second.ended(), "awaiting_operator");
    assert_eq!(host.failures(&refused, &run), [held_by(&first.run)]);
    assert!(!host.mark("refused").exists(), "its driver never started");

    let third = host.start(&apart, &beside);
    assert_eq!(third.ended(), "completed");
    assert_eq!(
        host.standing(&shared, &first.run).len(),
        1,
        "the first holds its worktree"
    );

    std::fs::write(host.gate(), "").unwrap();
    let first_run = first.run.clone();
    assert_eq!(first.ended(), "completed");
    assert!(
        host.standing(&shared, &first_run).is_empty(),
        "released once settled"
    );
    let next = host.start(&shared, &after);
    let next_run = next.run.clone();
    assert_eq!(next.ended(), "completed");
    assert!(host.standing(&shared, &next_run).is_empty());
}

/// Simultaneous starts on one worktree admit exactly one: of two runs
/// whose drivers both wait, one holds the lease and runs, and the other
/// fails by CR2's cause naming it, its driver never started.
#[test]
fn of_two_runs_started_together_on_one_worktree_exactly_one_runs() {
    let Some(host) = Coordination::new() else {
        return;
    };
    let shared = host.worktree("shared");
    let (left, right) = (host.bundle("left", true), host.bundle("right", true));
    let started = [host.start(&shared, &left), host.start(&shared, &right)];
    let runs = [started[0].run.clone(), started[1].run.clone()];
    until("one of them is refused", || {
        let failed = |at: usize| !host.failures([&left, &right][at], &runs[at]).is_empty();
        failed(0) || failed(1)
    });
    std::fs::write(host.gate(), "").unwrap();
    let [first, second] = started;
    let ends = [first.ended(), second.ended()];
    let refused = usize::from(ends[0] == "completed");
    assert_eq!(ends[1 - refused], "completed", "{ends:?}");
    assert_eq!(ends[refused], "awaiting_operator");
    let bundles = [&left, &right];
    assert_eq!(
        host.failures(bundles[refused], &runs[refused]),
        [held_by(&runs[1 - refused])]
    );
    let marks = [host.mark("left"), host.mark("right")];
    assert!(!marks[refused].exists() && marks[1 - refused].exists());
}
