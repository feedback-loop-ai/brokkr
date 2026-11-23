//! Fix round 0106b1r12, S3: a probe's hold names the driver instance that
//! holds it, its probe's own attempt, not only its run. An engine that
//! crashes after its probe's cleanup and before it ends the hold leaves
//! the hold standing; a later drive of the same run probes under its own
//! attempt, writes and ends its own hold, and never ends, revives or adopts
//! the crashed instance's, which stands, named by its attempt, until a
//! settlement ends it.

use super::crash::{crashing_clock, Crashed};
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
/// No coordination root can be protected, so no record proves its tree
/// settled. The resumed drive's own probe answers, the retry succeeds, and
/// of the two holds written only the resumed instance's is ended: the
/// crashed instance's stands, and both production gates read the route
/// cooling down a day past its stored end, naming its attempt.
#[test]
fn a_resume_never_ends_or_adopts_a_crashed_instances_hold() {
    let (dir, mut crashed, now, probe_at, gates) = probing(Probed::Answer);
    let home = dir.path().join("unrooted");
    let _unrooted = crate::engine::broker::admission::tests::Coordinated::unrooted(&home);
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

/// The reasons of every row and hold the in-force read of `store` holds a
/// day past the fixture's base instant.
fn in_force_words(store: &brokkr_store::Store) -> Vec<String> {
    let rows = store.cooldowns_in_force(base() + time::Duration::days(1));
    rows.unwrap().into_iter().map(|row| row.reason).collect()
}

/// Fix round 0106b1r12, S2: the restarted engine's reaper settles the tree
/// a crashed probe's hold names. The first engine's probe answers, its
/// driver is sealed Settled, and it crashes before it ends its hold or
/// releases its attempt; the hold stands in the in-force read, naming that
/// attempt. The resume's recovery, before any probe, proves the attempt by
/// its sealed record and releases it, and the reaper ends the hold naming
/// it. A hold of the run naming an attempt no record proves still stands.
/// It runs in an engine of its own, as the crash test it extends does.
#[test]
fn the_restarted_engines_reaper_ends_the_hold_of_a_crashed_probe_its_records_prove() {
    let test = "engine::cooldown::tests::held_instance::the_restarted_engines_reaper_ends_the_hold_of_a_crashed_probe_its_records_prove";
    let Some(crashed) = Crashed::in_its_own_engine(test) else {
        return;
    };
    let journal = crashed.dir.path().join("forge.db");
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    let unproven = brokkr_store::ProbeHold {
        run_id: &crashed.run,
        attempt_id: "unproven",
        site: "",
        provider: "codex",
        route: "second",
        deadline: base(),
    };
    store.hold_probe(&unproven, base()).unwrap();
    let standing = in_force_words(&store);
    assert_eq!(standing.len(), 2, "{standing:?}");
    let named = format!("(attempt {})", crashed.first);
    assert!(standing[0].contains(&named), "{standing:?}");
    let both = [crashed.first.clone(), "unproven".into()];
    assert_eq!(unended_attempts(&journal), both);
    drop(store);

    crashed.resumed();
    let probed = ["probe", "probe", "attempt:succeed"];
    assert_eq!(logs(crashed.dir.path(), "work"), probed);
    assert_eq!(unended_attempts(&journal), ["unproven"]);
    let standing = in_force_words(&brokkr_store::Store::open(&journal).unwrap());
    assert_eq!(standing.len(), 1, "{standing:?}");
    assert!(standing[0].contains("(attempt unproven)"), "{standing:?}");
}

/// The reaper's refusal: a store that cannot end the crashed probe's
/// proven hold (every hold update forced to fail) parks the resume in the
/// store's words before any probe, and the hold stands.
#[test]
fn a_hold_the_reaper_cannot_end_parks_the_resume_and_stands() {
    let test = "engine::cooldown::tests::held_instance::a_hold_the_reaper_cannot_end_parks_the_resume_and_stands";
    let Some(crashed) = Crashed::in_its_own_engine(test) else {
        return;
    };
    let journal = crashed.dir.path().join("forge.db");
    rusqlite::Connection::open(&journal)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER unended BEFORE UPDATE ON probe_holds
             BEGIN SELECT RAISE(ABORT, 'forced probe_holds failure'); END;",
        )
        .unwrap();
    let reason = park_reason(&crashed.resumed());
    assert_eq!(logs(crashed.dir.path(), "work"), ["probe"]);
    let unreaped = format!(
        "the cool-down probe holds of run {} whose attempts are settled could not be ended",
        crashed.run
    );
    assert!(
        reason.contains(&unreaped) && reason.contains("forced probe_holds failure"),
        "{reason}"
    );
    assert_eq!(unended_attempts(&journal), [crashed.first]);
}

/// The return of fix round 0106b1r12: a probe's hold is written only once
/// its reservation names its attempt, so a crash at any instant leaves a
/// hold the reservation names, or none. Another run's claim taken after the
/// first wait slice refuses the probe's admission: the probe parks
/// unadmitted, and no hold was ever written, so none could have stood
/// without its reservation.
#[cfg(target_os = "linux")]
#[test]
fn a_probe_whose_admission_is_refused_writes_no_hold() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let home = super::lease::home(dir.path());
    let worktree = dir.path().join("work");
    let claimed = super::lease::claim_after_first_slice(home, worktree, &now);
    engine.cooldown.clock = claimed.clock;
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let reason = park_reason(&engine);
    assert!(
        reason.contains("was not admitted") && reason.contains("run 'run-p' holds this worktree"),
        "{reason}"
    );
    assert_eq!(holds_written(engine.store.path()), 0);
}

/// The return of fix round 0106b1r12: the reaper runs at a resumed
/// engine's first turn, whatever its cursor, not only where a recorded
/// wait would probe. The first engine's probe answers, its driver is
/// sealed Settled, and it crashes before it ends its hold; the operator's
/// stop is then journaled, so the resume stops and reaches no wait. Its
/// first turn proves the crashed attempt by its sealed record, releases it
/// and ends the hold naming it: no second probe runs, and both gates read
/// the route open.
#[test]
fn a_resume_that_stops_before_any_wait_still_reaps_the_crashed_probes_hold() {
    let test = "engine::cooldown::tests::held_instance::a_resume_that_stops_before_any_wait_still_reaps_the_crashed_probes_hold";
    let Some(crashed) = Crashed::in_its_own_engine(test) else {
        return;
    };
    let journal = crashed.dir.path().join("forge.db");
    assert_eq!(
        unended_attempts(&journal),
        std::slice::from_ref(&crashed.first)
    );
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    let cited = brokkr_store::Store::open_read_only(&journal).unwrap();
    let named = crate::engine::Named {
        cause: crate::engine::Cause {
            ending: brokkr_core::ending::Ending::OperatorStopped,
            by_realm: None,
            by_run: None,
            by_ref: None,
        },
        reason: "the operator stopped it",
        operator: "operator",
    };
    crate::engine::operator_stop(&mut store, &cited, &crashed.run, &named).unwrap();
    drop((store, cited));
    let resumed = crashed.resumed();
    assert_eq!(logs(crashed.dir.path(), "work"), ["probe"]);
    assert_eq!(unended_attempts(&journal), Vec::<String>::new());
    let mut gates = super::held_route::Gates::over(&journal, base());
    assert_eq!(gates.legs(base()), [Ok(()), Ok(())]);
    assert_eq!(super::held_wait::standing(&resumed).0, Status::Stopped);
}
