//! Sealing a reservation again (U8a2a; H1, M1): every operation that
//! reseals one — a run's next attempt joining it, the engine marking a
//! driver's cleanup unresolved, the positive settlement fact, the release,
//! and recovery at a later admission — commits only by its rename, after
//! its last deadline check. Each case below begins with a reservation
//! already standing, its attempt unsettled, and fails or runs past the
//! deadline at that final step; the reservation that stood is still there,
//! the same bytes in the same inode, and no staged copy is left beside it.
//! A positive fact that cannot be sealed keeps the reservation standing,
//! and a release without one is refused.

use std::os::unix::fs::MetadataExt;

use super::coordination::{died_after_its_end, settled_a1};
use super::*;

/// A fresh engine holding on `host` the lease of its run's attempt `a1` on
/// its own worktree, the reservation naming that engine's own journal: the
/// engine's directory, the engine, the worktree and the run.
fn leasing(host: &Host) -> (tempfile::TempDir, Engine, PathBuf, String) {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let (work, run) = (worktree(&engine), engine.run_id.clone());
    let mut leased = claim(&work, [&run, "a1"], Protects::Nothing, &[]);
    leased.journal = std::fs::canonicalize(engine.store.path()).unwrap();
    engine.held = Some(host.admit(&leased).unwrap());
    (dir, engine, work, run)
}

/// The bytes and the device and inode of the file at `at`.
fn standing(at: &Path) -> (Vec<u8>, (u64, u64)) {
    let found = std::fs::metadata(at).unwrap();
    (std::fs::read(at).unwrap(), (found.dev(), found.ino()))
}

/// The reservation at `at` stands as `before` did, and nothing is staged
/// beside it.
fn kept(at: &Path, before: &(Vec<u8>, (u64, u64)), what: &str) {
    assert_eq!(&standing(at), before, "{what}: the reservation that stood");
    let staged = at.with_file_name("reservation.json.new");
    assert!(!staged.exists(), "{what}: no staged copy left");
}

/// The next seal on this thread spends `millis` before its commit.
fn spends(millis: u64) {
    LATE.set(Some(Late::Sleep(Duration::from_millis(millis))));
}

/// `millis` from now.
fn within(millis: u64) -> Instant {
    Instant::now() + Duration::from_millis(millis)
}

/// R3: a first seal that would finish past the shared deadline is never
/// renamed in, so it names nothing.
#[test]
fn a_seal_that_would_finish_past_the_deadline_is_never_committed() {
    let host = Host::new();
    let budget = Budget::new(within(100));
    spends(150);
    let sealed = seal_with(host.root.dir(), "probe.json", b"{}", &budget, late);
    assert_eq!(sealed, Err(Unknown::Expired));
    let staged = host.root().join("probe.json.new");
    assert!(!host.root().join("probe.json").exists() & !staged.exists());
}

/// A run's next attempt whose seal runs past the deadline at its commit is
/// refused, and the reservation of its earlier, unsettled attempt stands
/// as it was.
#[test]
fn a_late_join_keeps_the_reservation_that_stood() {
    let host = Host::new();
    let work = host.worktree("work");
    drop(host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let at = host.reservation(&work, "run-a");
    let before = standing(&at);
    spends(600);
    let joined = admit(&host.root, &open(&work, ["run-a", "a2"]), within(400));
    assert_eq!(joined, mb3(Unknown::Expired));
    kept(&at, &before, "join");
}

/// The engine's mark of a driver's unresolved cleanup that runs past its
/// deadline at its commit leaves the reservation as it stood; the engine's
/// own copy is unproven all the same, so its settlement, the attempt's end
/// journaled, never releases the attempt.
#[test]
fn a_late_unresolved_mark_keeps_the_reservation_that_stood() {
    let host = Host::new();
    let (_dir, mut engine, work, run) = leasing(&host);
    let at = host.reservation(&work, &run);
    let before = standing(&at);
    spends(600);
    let unresolved = Cleanup::Unresolved {
        reason: Unsettled::Stdout,
    };
    witnessed_by(&mut engine.held, &unresolved, within(400));
    kept(&at, &before, "unresolved");
    settled_a1(&mut engine, &run);
    engine.settle("a1", Ok(())).unwrap();
    kept(&at, &before, "settled after");
}

/// A release that runs past its deadline at its commit is refused, and the
/// reservation, its positive fact sealed, stands as it was.
#[test]
fn a_late_release_keeps_the_reservation_that_stood() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    proved(&held, soon()).unwrap();
    let at = host.reservation(&work, "run-a");
    let before = standing(&at);
    spends(600);
    assert_eq!(release(&held, within(400)), mb3(Unknown::Expired).map(drop));
    kept(&at, &before, "release");
}

/// Recovery that runs past the deadline at its commit, at a later run's
/// admission, refuses that run, and the recovered run's reservation — its
/// positive fact sealed, its certified end in a protected journal — stands
/// as it was.
#[test]
fn a_late_recovery_keeps_the_reservation_that_stood() {
    let host = Host::new();
    let (_dir, _engine, work, run) = died_after_its_end(&host);
    let at = host.reservation(&work, &run);
    let before = standing(&at);
    spends(1_500);
    let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    assert_eq!(
        admit(&host.root, &next, within(1_000)),
        mb3(Unknown::Expired)
    );
    kept(&at, &before, "recovery");
    assert!(
        !host.reservation(&work, "run-b").exists(),
        "nothing of it sealed"
    );
}

/// M1: a release needs the attempt's positive settlement fact; without it
/// the release is refused, and the reservation stands as it was.
#[test]
fn a_release_without_its_positive_fact_is_refused() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    let at = host.reservation(&work, "run-a");
    let before = standing(&at);
    assert_eq!(release(&held, soon()), mb3(Unknown::Unproven).map(drop));
    kept(&at, &before, "unproven");
}

/// M1: an engine whose positive settlement fact cannot be sealed, its
/// attempt ended and its drivers settled, does not release the attempt:
/// its reservation stands as it was, and a later run on its worktree is
/// refused by its lease, the end in its protected journal proving nothing
/// without the fact.
#[test]
fn a_positive_fact_that_cannot_be_sealed_keeps_the_reservation() {
    let host = Host::new();
    let (_dir, mut engine, work, run) = leasing(&host);
    settled_a1(&mut engine, &run);
    let at = host.reservation(&work, &run);
    let before = standing(&at);
    LATE.set(Some(Late::Unstage));
    engine.settle("a1", Ok(())).unwrap();
    kept(&at, &before, "the fact");
    let later = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    assert_eq!(host.admit(&later), cr2(Conflict::Held(run)));
}
