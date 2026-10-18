//! Sealing a reservation again (U8a2a; H1, M1; R1 and R2 of run four's
//! review): every operation that reseals one — a run's next attempt joining
//! it, the engine marking a driver's cleanup unresolved, the positive
//! settlement fact, the release, and recovery at a later admission — fails
//! or runs late at its final steps in each of three ways ([`finals`]): past
//! the deadline before its commit, past it once committed, and its
//! directory sync failing once committed. Each case begins with a
//! reservation already standing, its attempt unsettled; after each, the
//! reservation that stood is still there, the same bytes in the same inode,
//! and no staged copy is left beside it. A positive fact that cannot be
//! sealed keeps the reservation standing and, where the fence rule asks it,
//! fails the run that settles; a release without one is refused.

use std::os::unix::fs::MetadataExt;

use super::coordination::{died_after_its_end, settled_a1};
use super::*;
use crate::capabilities::McpFence;
use crate::engine::broker::settlement::Unreleased;

/// A fresh engine holding on `host` the lease of its run's attempt `a1` on
/// its own worktree, protecting `protects`, the reservation naming that
/// engine's own journal: the engine's directory, the engine, the worktree
/// and the run.
fn leasing(host: &Host, protects: Protects) -> (tempfile::TempDir, Engine, PathBuf, String) {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let (work, run) = (worktree(&engine), engine.run_id.clone());
    let mut leased = claim(&work, [&run, "a1"], protects, &[]);
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

/// `millis` from now.
fn within(millis: u64) -> Instant {
    Instant::now() + Duration::from_millis(millis)
}

/// The three ways a seal fails or runs late at its final steps, each
/// spending `millis` where it runs late: what arms it, the refusal it
/// meets, and its name.
fn finals(millis: u64) -> [(Late, Unknown, &'static str); 3] {
    let spent = Duration::from_millis(millis);
    [
        (
            Late::Sleep(spent),
            Unknown::Expired,
            "late before its commit",
        ),
        (
            Late::SleepCommitted(spent),
            Unknown::Expired,
            "late once committed",
        ),
        (
            Late::FailCommitted,
            Unknown::Unsealed,
            "its sync failed once committed",
        ),
    ]
}

/// R3, R1: a first seal that fails or would finish past the shared
/// deadline at its final steps leaves no record and nothing staged.
#[test]
fn a_first_seal_that_fails_or_runs_late_leaves_no_record() {
    for (armed, refused, how) in finals(150) {
        let host = Host::new();
        let budget = Budget::new(within(100));
        LATE.set(Some(armed));
        let sealed = seal_with(host.root.dir(), "probe.json", b"{}", &budget, late);
        assert_eq!(sealed, Err(refused), "{how}");
        let staged = host.root().join("probe.json.new");
        assert!(!host.root().join("probe.json").exists(), "{how}: no record");
        assert!(!staged.exists(), "{how}: nothing staged");
    }
}

/// R1: a reseal in a directory that will not exchange two names is
/// refused, and the record that stood is never replaced.
#[test]
fn a_reseal_that_cannot_exchange_keeps_the_reservation_that_stood() {
    let (host, at, before) = joining(Late::Deny);
    let joined = admit(
        &host.root,
        &open(&host.worktree("work"), ["run-a", "a2"]),
        soon(),
    );
    let run = at.parent().unwrap();
    std::fs::set_permissions(run, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(joined, mb3(Unknown::Unsealed));
    assert_eq!(standing(&at), before, "the reservation that stood");
}

/// A fresh host whose run `run-a` holds its attempt `a1` standing on the
/// worktree `work`, the next seal on this thread armed by `armed`: the
/// host, where the reservation stands, and how it stands.
fn joining(armed: Late) -> (Host, PathBuf, (Vec<u8>, (u64, u64))) {
    let host = Host::new();
    let work = host.worktree("work");
    drop(host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let at = host.reservation(&work, "run-a");
    let before = standing(&at);
    LATE.set(Some(armed));
    (host, at, before)
}

/// A run's next attempt whose seal fails or runs late at its final steps
/// is refused, and the reservation of its earlier, unsettled attempt stands
/// as it was.
#[test]
fn a_late_join_keeps_the_reservation_that_stood() {
    for (armed, refused, how) in finals(600) {
        let (host, at, before) = joining(armed);
        let next = open(&host.worktree("work"), ["run-a", "a2"]);
        assert_eq!(admit(&host.root, &next, within(400)), mb3(refused), "{how}");
        kept(&at, &before, how);
    }
}

/// The engine's mark of a driver's unresolved cleanup that fails or runs
/// late at its final steps leaves the reservation as it stood; the engine's
/// own copy is unproven all the same, so its settlement, the attempt's end
/// journaled, never releases the attempt.
#[test]
fn a_late_unresolved_mark_keeps_the_reservation_that_stood() {
    for (armed, _, how) in finals(600) {
        let host = Host::new();
        let (_dir, mut engine, work, run) = leasing(&host, Protects::Nothing);
        let at = host.reservation(&work, &run);
        let before = standing(&at);
        LATE.set(Some(armed));
        let unresolved = Cleanup::Unresolved {
            reason: Unsettled::Stdout,
        };
        witnessed_by(&mut engine.held, &unresolved, within(400));
        kept(&at, &before, how);
        settled_a1(&mut engine, &run);
        engine.settle("a1", Ok(())).unwrap();
        kept(&at, &before, &format!("{how}, settled after"));
    }
}

/// A release that fails or runs late at its final steps is refused, and
/// the reservation, its positive fact sealed, stands as it was.
#[test]
fn a_late_release_keeps_the_reservation_that_stood() {
    for (armed, refused, how) in finals(600) {
        let host = Host::new();
        let work = host.worktree("work");
        let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
        proved(&held, soon()).unwrap();
        let at = host.reservation(&work, "run-a");
        let before = standing(&at);
        LATE.set(Some(armed));
        let released = release(&held, within(400));
        assert_eq!(released, mb3(refused).map(drop), "{how}");
        kept(&at, &before, how);
    }
}

/// Recovery that fails or runs late at its final steps, at a later run's
/// admission, refuses that run, and the recovered run's reservation — its
/// positive fact sealed, its certified end in a protected journal — stands
/// as it was.
#[test]
fn a_late_recovery_keeps_the_reservation_that_stood() {
    for (armed, refused, how) in finals(1_500) {
        let host = Host::new();
        let (_dir, _engine, work, run) = died_after_its_end(&host);
        let at = host.reservation(&work, &run);
        let before = standing(&at);
        LATE.set(Some(armed));
        let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
        let admitted = admit(&host.root, &next, within(1_000));
        assert_eq!(admitted, mb3(refused), "{how}");
        kept(&at, &before, how);
        let sealed = host.reservation(&work, "run-b").exists();
        assert!(!sealed, "{how}: nothing of it sealed");
    }
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

/// M1, R2: an engine whose positive settlement fact cannot be sealed, its
/// attempt ended and its drivers settled, does not release the attempt:
/// its reservation stands as it was, and a later run on its worktree is
/// refused by its lease, the end in its protected journal proving nothing
/// without the fact. Under the fence rule the settlement fails the run
/// with the typed refusal: where evidence was at stake as it was admitted,
/// and on every run once the fence is lifted; with the fence standing and
/// nothing at stake it does not.
#[test]
fn a_positive_fact_that_cannot_be_sealed_keeps_the_reservation() {
    let unsealed = || Unreleased::from(Unadmitted::Identity(Unknown::Unsealed));
    for (protects, lifted, fails) in [
        (Protects::Nothing, false, false),
        (Protects::Evidence, false, true),
        (Protects::Nothing, true, true),
    ] {
        let how = format!("{protects:?}, the fence lifted: {lifted}");
        let host = Host::new();
        let (_dir, mut engine, work, run) = leasing(&host, protects);
        settled_a1(&mut engine, &run);
        let at = host.reservation(&work, &run);
        let before = standing(&at);
        let lift = lifted.then(McpFence::lift);
        LATE.set(Some(Late::Unstage));
        let settled = engine.settle("a1", Ok(())).map_err(|error| match error {
            EngineError::Unreleased(unreleased) => unreleased,
            other => panic!("{how}: {other}"),
        });
        drop(lift);
        assert_eq!(
            settled,
            if fails { Err(unsealed()) } else { Ok(()) },
            "{how}"
        );
        kept(&at, &before, &how);
        let later = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
        assert_eq!(host.admit(&later), cr2(Conflict::Held(run)), "{how}");
    }
}
