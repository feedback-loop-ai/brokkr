//! The crashed cool-down probe's recovery refusals (decision 0103 ruling 4,
//! fix round 430s4c6): the admission lock, the reservation and its seal each
//! park with the cleanup refusal naming where the probe stands, never as a
//! pass over a reservation whose processes are not proven settled.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::*;
use crate::engine::broker::admission::probe::settle_orphaned;
use crate::engine::broker::settlement::Driver;

/// A wait the cleanup refusal names.
fn wait() -> brokkr_core::ProviderWait {
    brokkr_core::ProviderWait {
        site: None,
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit".into(),
        retry_after_seconds: None,
        seconds: 1,
        until: OffsetDateTime::parse("2026-11-01T00:00:01Z", &Rfc3339).unwrap(),
        rounds: 1,
    }
}

/// A run's one standing attempt `a1` on `work`, no journal event naming it:
/// the probe's reservation, and the worktree's key.
fn planted(host: &Host, work: &Path, run: &str) -> (String, PathBuf) {
    let held = host.admit(&open(work, [run, "a1"])).unwrap();
    drop(held);
    let repo = canonical::sha256_bytes(work.as_os_str().as_bytes());
    (repo, host.reservation(work, run))
}

/// The lock a recovery must take is held past its deadline: the cleanup
/// refusal names the reservation and says it cannot be read.
#[test]
fn a_lock_that_cannot_be_taken_parks_naming_the_reservation() {
    let host = Host::new();
    let work = host.worktree("work");
    let (repo, at) = planted(&host, &work, "run-a");
    let _held = taken(&host.root().join(LOCK));
    let deadline = Instant::now() + Duration::from_millis(50);
    let refused = settle_orphaned(&host.root, [&repo, "run-a"], &[], &wait(), deadline)
        .expect_err("a held lock was passed over");
    assert!(
        refused.contains("did not prove its process tree gone"),
        "{refused}"
    );
    assert!(refused.contains("cannot be read"), "{refused}");
    assert!(refused.contains(&at.display().to_string()), "{refused}");
}

/// A reservation that does not read is no reservation: the cleanup refusal
/// names where it stands, never as no probe.
#[test]
fn a_reservation_that_does_not_read_parks_naming_where_it_stands() {
    let host = Host::new();
    let work = host.worktree("work");
    let (repo, at) = planted(&host, &work, "run-a");
    std::fs::write(&at, "not a reservation").unwrap();
    let refused = settle_orphaned(&host.root, [&repo, "run-a"], &[], &wait(), soon())
        .expect_err("a malformed reservation was passed over");
    assert!(refused.contains("cannot be read"), "{refused}");
    assert!(refused.contains(&at.display().to_string()), "{refused}");
}

/// A proven orphan's release whose seal cannot commit parks naming the
/// attempt, so a later run never reads a release that did not stand.
#[test]
fn a_release_that_cannot_be_sealed_parks_naming_the_attempt() {
    let host = Host::new();
    let work = host.worktree("work");
    let (repo, at) = planted(&host, &work, "run-a");
    let mut held: Reservation = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let reserved = held.attempts.get_mut("a1").unwrap();
    let driver = Driver {
        site: "work".into(),
        begun: None,
        cleanup: Witnessed::Settled,
    };
    reserved.drivers.push(driver);
    std::fs::write(&at, serde_json::to_vec(&held).unwrap()).unwrap();
    LATE.set(Some(Late::FailCommitted));
    let refused = settle_orphaned(&host.root, [&repo, "run-a"], &[], &wait(), soon())
        .expect_err("a seal that could not commit was passed over");
    assert!(refused.contains("'a1'"), "{refused}");
}

/// A worktree that does not resolve has no reservation to find: the cleanup
/// refusal says so and no probe follows. The root is protected; only the
/// key cannot be taken.
#[test]
fn a_worktree_that_does_not_resolve_parks_without_a_reservation() {
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let (_dir, engine) = engine(single_body(vec!["unused".into()]));
    std::fs::remove_dir_all(worktree(&engine)).unwrap();
    let observed = observed().expect("a coordinated host");
    let refused = engine
        .recovered_probe(&[], &wait(), &observed)
        .expect_err("a vanished worktree found a reservation");
    assert!(refused.contains("cannot be located"), "{refused}");
}

/// An engine whose worktree's reservation holds one standing attempt `a1`
/// that no journal event names, as a crashed cool-down probe's does: the
/// engine's own directory, the engine, and its canonical worktree.
pub(super) fn crashed_probe(host: &Host) -> (tempfile::TempDir, Engine, PathBuf) {
    let (dir, mut engine) = engine(single_body(vec!["unused".into()]));
    let work = worktree(&engine);
    let [_, _] = lease(host, &mut engine, "a1", Protects::Nothing);
    drop(engine.held.take());
    (dir, engine, work)
}

/// The probe admission for a new attempt `a2` of `engine`'s run, under the
/// host `observed`: the one call the two "leaves the admission its own
/// refusal" tests share.
fn probe_admission(engine: &mut Engine, observed: &Observed) -> Result<(), Unadmitted> {
    let events = engine.store.load(&engine.run_id).unwrap();
    let body = single_body(vec!["unused".into()]);
    let (selected, _) = body.selected(None).unwrap();
    engine
        .admit_for_probe(
            &events,
            ["a2", "work"],
            selected,
            &Selection::new(),
            (Instant::now(), Duration::from_secs(5)),
            observed,
        )
        .map(drop)
}

/// The probe's own admission still fails closed where the recovery could
/// not check: a lock that is not this user's own single file leaves
/// `orphan_at` nothing, and the admission itself refuses by the lock's
/// name. The attempt it would admit is never sealed.
#[test]
fn an_invalid_lock_leaves_the_probe_admission_its_own_refusal() {
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let (_dir, mut engine, _work) = crashed_probe(&host);
    std::fs::hard_link(host.root().join(LOCK), host.dir.path().join("second-link")).unwrap();
    let observed = observed().expect("a coordinated host");
    assert_eq!(
        probe_admission(&mut engine, &observed),
        Err(Unadmitted::Identity(Unknown::Lock))
    );
}

/// A worktree that no longer resolves leaves `orphan_at` nothing to find,
/// and the admission that follows refuses by the worktree's own name: no
/// second probe is admitted over a reservation it could not read.
#[test]
fn a_vanished_worktree_leaves_the_probe_admission_its_own_refusal() {
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let (_dir, mut engine, work) = crashed_probe(&host);
    std::fs::remove_dir_all(&work).unwrap();
    let observed = observed().expect("a coordinated host");
    assert_eq!(
        probe_admission(&mut engine, &observed),
        Err(Unadmitted::Identity(Unknown::Worktree))
    );
}
