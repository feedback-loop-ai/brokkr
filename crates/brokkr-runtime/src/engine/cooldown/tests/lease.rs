//! The cool-down probe as an attempt of its own (decision 0103 slice 4c1,
//! fix round 430s4c5): its admission seals the reach its box is built from
//! and takes the writer lease its door rechecks, its settlement releases
//! that lease before the retry admits, and a refused admission parks
//! without spending an effect attempt.

use std::os::unix::fs::PermissionsExt;
use std::time::{Duration as StdDuration, Instant};

use super::*;
use crate::engine::broker::admission::tests::{protecting_refused, Coordinated};

/// A home a test's engine and its competing claim both resolve their
/// coordination root under, owner-only as admission opens it.
fn home(dir: &Path) -> PathBuf {
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).unwrap();
    home
}

/// A boxed seat's probe on a resume is composed from the reach its own
/// admission sealed, so the box is built, the probe answers and the retry
/// runs. At HEAD the fresh engine sealed no reach and the probe parked
/// "did not spawn".
#[test]
fn a_boxed_seats_probe_on_a_resume_spawns_in_its_claims_reach_and_retries() {
    let (dir, mut engine, _now) = resumed_at(
        &["succeed"],
        base() + time::Duration::seconds(1),
        1,
        base(),
        true,
    );
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["probe", "attempt:succeed"]);
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert!(started[1]["probe"].is_object());
}

/// While the probe's own driver runs, its admission's lease stands at the
/// protected root: another run's claim is refused by name, and after the
/// probe's settlement the retry's own admission succeeds and runs.
#[test]
fn a_probe_holds_the_writer_lease_at_a_protected_root() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Barrier, None, 2);
    let home = home(dir.path());
    let worktree = dir.path().join("work");
    let started = dir.path().join("work.started");
    let release = dir.path().join("work.release");
    let standing = home.clone();
    let driver = std::thread::spawn(move || {
        let _stand = Coordinated::at(&standing);
        engine.drive().unwrap();
        engine
    });
    let deadline = Instant::now() + StdDuration::from_secs(20);
    while !started.exists() {
        assert!(Instant::now() < deadline, "the probe never started");
        std::thread::sleep(StdDuration::from_millis(10));
    }
    let refused = protecting_refused(&home, &worktree);
    std::fs::write(&release, "").unwrap();
    let engine = driver.join().unwrap();
    assert!(
        refused.as_ref().is_err_and(|refusal| refusal
            .to_string()
            .contains("holds this worktree, its writers not proven settled")),
        "the probe held no writer lease: {refused:?}"
    );
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(starts(&engine).len(), 2);
    let after = protecting_refused(&home, &worktree);
    assert!(
        after.is_ok(),
        "the probe's lease was not released after its settlement: {after:?}"
    );
}

/// A refused admission at the probe parks naming the refusal: it writes no
/// `effect/failed` under the probe's attempt, the cap's count is unchanged,
/// and no probe or retry follows.
#[test]
fn a_refused_admission_at_the_probe_parks_naming_the_refusal() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let home = home(dir.path());
    let worktree = dir.path().join("work");
    let _stand = Coordinated::at(&home);
    let standing = home.clone();
    let claimed = worktree.clone();
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        protecting_refused(&standing, &claimed).expect("the first attempt's lease was released");
    });
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let started = starts(&engine);
    assert_eq!(started.len(), 1);
    let failed = failed(&engine);
    assert_eq!(
        failed.len(),
        1,
        "the probe wrote an effect/failed: {failed:?}"
    );
    assert_eq!(
        failed[0].attempt_id.as_deref(),
        started[0]["attempt_id"].as_str(),
        "the probe's attempt is the failed event's"
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("not admitted"), "{reason}");
    assert!(
        reason.contains("run 'run-p' holds this worktree"),
        "{reason}"
    );
}

/// A probe at a root the engine cannot protect (C1) holds no lease, so its
/// settlement releases nothing, and the same answer and retry follow.
#[test]
fn a_probes_unprotected_root_settles_nothing_and_answers() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let home = home(dir.path());
    let _stand = Coordinated::unrooted(&home);
    drove(
        &mut engine,
        dir.path(),
        &["attempt:refuse", "probe", "attempt:succeed"],
    );
}

/// The cap counts the effect's failures alone: at two attempts a refusal,
/// its answer and a second refusal still park with 0006's text, two
/// `effect/failed` events and two starts, none of them the probe's.
#[test]
fn the_cap_counts_the_effects_failures_and_not_the_probes_journals() {
    let (dir, mut engine, _now) = seated!(&["refuse", "refuse"], Probed::Answer, None, 2);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:refuse"]
    );
    let reason = park_reason(&engine);
    assert!(
        reason.starts_with("effect ") && reason.contains("failed 2 of 2 attempt(s); last error:"),
        "{reason}"
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2);
    let failed = failed(&engine);
    assert_eq!(
        failed.len(),
        2,
        "a probe wrote an effect/failed: {failed:?}"
    );
    let failures: Vec<_> = failed
        .iter()
        .map(|event| event.attempt_id.clone().unwrap())
        .collect();
    let starts: Vec<_> = started
        .iter()
        .map(|started| started["attempt_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(failures, starts);
}
