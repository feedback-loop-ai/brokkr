//! Sealing a reservation again (U8a2a; H1, M1; R1 and R2 of run four's
//! review): every operation that reseals one — a run's next attempt joining
//! it, the engine marking a driver's cleanup unresolved, the positive
//! settlement fact, the release, and recovery at a later admission — fails
//! or runs late at its final steps before its commit in each of five ways
//! ([`finals`]): past the deadline before its rename, past it once renamed
//! in, its directory sync failing then, past it as it retires the record
//! that stood, and that record's prior name not removed; and one whose
//! last sync fails once committed reports itself committed (R1 of the
//! fourth run, U8a2c). Each case begins with a
//! reservation already standing, its attempt unsettled; after each, the
//! reservation that stood is still there, the same bytes in the same inode,
//! and no staged copy is left beside it. A positive fact that cannot be
//! sealed keeps the reservation standing and, where the fence rule asks it,
//! fails the run that settles; a release without one is refused.

use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;

use brokkr_protocol::broker::ledger::Outcome;

use super::coordination::{died_after_its_end, settled_a1};
use super::late::{
    failing_second_exchange, failing_second_exchange_over_the_name,
    failing_second_exchange_over_the_staged, failing_second_exchange_over_the_staged_and_the_hand,
    refusing, refusing_exchange, swaps_again, unlinking_before_second_exchange,
    unlinking_before_third_exchange,
};
use super::*;
use crate::capabilities::McpFence;
use crate::engine::artifacts::tests::{sha, Provisioned};
use crate::engine::artifacts::Unpublished;
use crate::engine::broker::root::{identified, Commit, Record, Root};
use crate::engine::broker::settlement::release::spawned;
use crate::engine::broker::settlement::{adopted, attested, ended, spawning, Lease, Unreleased};

/// A fresh engine holding on `host` the lease of its run's attempt `a1` on
/// its own worktree, protecting `protects`, the reservation naming that
/// engine's own journal: the engine's directory, the engine, the worktree
/// and the run.
fn leasing(host: &Host, protects: Protects) -> (tempfile::TempDir, Engine, PathBuf, String) {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let [work, _] = lease(host, &mut engine, "a1", protects);
    let run = engine.run_id.clone();
    (dir, engine, work, run)
}

/// The bytes and the device and inode of the file at `at`.
fn standing(at: &Path) -> (Vec<u8>, (u64, u64)) {
    let found = std::fs::metadata(at).unwrap();
    (std::fs::read(at).unwrap(), (found.dev(), found.ino()))
}

/// The reservation at `at` stands as `before` did, and nothing is staged
/// or kept under its prior name beside it.
fn kept(at: &Path, before: &(Vec<u8>, (u64, u64)), what: &str) {
    assert_eq!(&standing(at), before, "{what}: the reservation that stood");
    let staged = at.with_file_name("reservation.json.new");
    assert!(!staged.exists(), "{what}: no staged copy left");
    assert!(!prior(at).exists(), "{what}: no prior name left");
}

/// The prior name of the reservation at `at`.
fn prior(at: &Path) -> PathBuf {
    at.with_file_name("reservation.json.old")
}

/// `millis` from now.
fn within(millis: u64) -> Instant {
    Instant::now() + Duration::from_millis(millis)
}

/// The five ways a reseal fails or runs late at its final steps before its
/// commit, each spending `millis` where it runs late: what arms it, the
/// refusal it meets, and its name. A first seal meets the first three, its
/// commit the rename in (R1 of the fourth run; U8a2c).
fn finals(millis: u64) -> [(Late, Unknown, &'static str); 5] {
    let spent = Duration::from_millis(millis);
    [
        (
            Late::Sleep(spent),
            Unknown::Expired,
            "late before its rename",
        ),
        (
            Late::SleepCommitted(spent),
            Unknown::Expired,
            "late once renamed in",
        ),
        (
            Late::FailCommitted,
            Unknown::Unsealed,
            "its sync failed once renamed in",
        ),
        (
            Late::SleepRetiring(spent),
            Unknown::Expired,
            "late as it retires the record that stood",
        ),
        (
            Late::FailRetiring,
            Unknown::Unsealed,
            "the prior name of the record that stood not removed",
        ),
    ]
}

/// R3, R1: a first seal that fails or would finish past the shared
/// deadline at its final steps leaves no record and nothing staged.
#[test]
fn a_first_seal_that_fails_or_runs_late_leaves_no_record() {
    for (armed, refused, how) in finals(150).into_iter().take(3) {
        let host = Host::new();
        let budget = Budget::new(within(100));
        LATE.set(Some(armed));
        let sealed = host
            .root
            .seal(host.root.dir(), "probe.json", b"{}", &budget);
        assert_eq!(sealed, Err(refused), "{how}");
        let staged = host.root().join("probe.json.new");
        assert!(!host.root().join("probe.json").exists(), "{how}: no record");
        assert!(!staged.exists(), "{how}: nothing staged");
    }
}

/// R1, M3; R3 (U8a2c): a reseal that cannot give the record that stood a
/// second name — a file already at its prior name, the directory still
/// writable, so the rename after the link stays possible — is refused,
/// and that record is never replaced.
#[test]
fn a_reseal_that_cannot_link_the_record_that_stood_keeps_it() {
    let (host, at, before) = joining(Late::Occupy);
    let joined = admit(
        &host.root,
        &open(&host.worktree("work"), ["run-a", "a2"]),
        soon(),
    );
    assert_eq!(joined, mb3(Unknown::Unsealed));
    assert_eq!(standing(&at), before, "the reservation that stood");
    assert_eq!(
        std::fs::read(prior(&at)).unwrap(),
        b"",
        "the file planted, untouched"
    );
    std::fs::remove_file(prior(&at)).unwrap();
    kept(&at, &before, "its link refused");
}

/// The directory of the reservation at `at` writable by its owner again.
fn writable(at: &Path) {
    let run = at.parent().unwrap();
    std::fs::set_permissions(run, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// M2 (run four's second review): a reseal whose undoing itself fails —
/// the rename back refused, or the directory's sync after it failing — is
/// refused as unsealed, whatever cut it short. The reservation that stood
/// is the one every read finds: its very file under its prior name where
/// it could not be renamed back, or back at its name. The next seal puts
/// that file back at its name before anything else, so even cut short it
/// leaves it there; and the run's next attempt joins that reservation, not
/// the refused one, leaving no prior name behind.
#[test]
fn a_reseal_whose_undoing_fails_leaves_the_reservation_that_stood_read() {
    let spent = Duration::from_millis(600);
    for (armed, how) in [
        (Late::DenyCommitted(spent), "its rename back refused"),
        (Late::FailUndone(spent), "its undoing's sync failed"),
    ] {
        let (host, at, before) = joining(armed);
        let work = host.worktree("work");
        let joined = admit(&host.root, &open(&work, ["run-a", "a2"]), within(400));
        writable(&at);
        assert_eq!(joined, mb3(Unknown::Unsealed), "{how}");
        let stood = if prior(&at).exists() {
            prior(&at)
        } else {
            at.clone()
        };
        assert_eq!(
            standing(&stood),
            before,
            "{how}: the reservation that stood"
        );
        let key = open(&work, ["run-a", "a2"]).repo;
        let run = host.root.below(&[&key, "run-a"]).unwrap();
        let read: Reservation = read(&run, RESERVATION, &at).unwrap();
        assert_eq!(read, serde_json::from_slice(&before.0).unwrap(), "{how}");
        // The next seal first puts that very file back: cut short before
        // its own commit, it leaves the record that stood at its name.
        LATE.set(Some(Late::Sleep(spent)));
        let cut = admit(&host.root, &open(&work, ["run-a", "a3"]), within(400));
        assert_eq!(cut, mb3(Unknown::Expired), "{how}");
        kept(&at, &before, &format!("{how}, the next seal cut short"));
        drop(host.admit(&open(&work, ["run-a", "a3"])).unwrap());
        let joined = (vec!["a1".to_string(), "a3".to_string()], Vec::new());
        assert_eq!(host.attempts(&work, "run-a"), joined, "{how}");
        assert!(!prior(&at).exists(), "{how}: no prior name left");
    }
}

/// M2: a seal cut short between its link of the record that stood and its
/// rename, both names one file, is read as that record and undone by the
/// run's next seal, which joins it and leaves no prior name.
#[test]
fn a_seal_cut_short_between_its_link_and_its_rename_is_undone_by_the_next() {
    let (host, at, before) = stood();
    let work = host.worktree("work");
    std::fs::hard_link(&at, prior(&at)).unwrap();
    drop(host.admit(&open(&work, ["run-a", "a2"])).unwrap());
    let joined = (vec!["a1".to_string(), "a2".to_string()], Vec::new());
    assert_eq!(host.attempts(&work, "run-a"), joined);
    assert!(!prior(&at).exists(), "no prior name left");
    assert_eq!(std::fs::metadata(&at).unwrap().nlink(), 1, "one link");
    assert_ne!(standing(&at), before, "resealed");
}

/// M2: a prior name with no reservation beside it is no seal's, and is an
/// unknown participant.
#[test]
fn a_prior_name_with_no_record_beside_it_is_an_unknown_participant() {
    let (host, at, _) = stood();
    std::fs::rename(&at, prior(&at)).unwrap();
    let next = host.admit(&open(&host.worktree("work"), ["run-b", "b1"]));
    assert_eq!(next, mb3(Unknown::Participant(prior(&at))));
}

/// A fresh host whose run `run-a` holds its attempt `a1` standing on the
/// worktree `work`: the host, where the reservation stands, and how it
/// stands.
fn stood() -> (Host, PathBuf, (Vec<u8>, (u64, u64))) {
    let host = Host::new();
    let work = host.worktree("work");
    drop(host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let at = host.reservation(&work, "run-a");
    let before = standing(&at);
    (host, at, before)
}

/// [`stood`], the next seal on this thread armed by `armed`.
fn joining(armed: Late) -> (Host, PathBuf, (Vec<u8>, (u64, u64))) {
    let stood = stood();
    LATE.set(Some(armed));
    stood
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
        assert_eq!(proved(&held, soon()), Ok(Commit::Synced));
        let at = host.reservation(&work, "run-a");
        let before = standing(&at);
        LATE.set(Some(armed));
        let released = release(&held, within(400));
        assert_eq!(released, Err(Unadmitted::Identity(refused)), "{how}");
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
    let unproven = Err(Unadmitted::Identity(Unknown::Unproven));
    assert_eq!(release(&held, soon()), unproven);
    kept(&at, &before, "unproven");
}

/// M1, R2: an engine whose positive settlement fact cannot be sealed, its
/// attempt ended and its drivers settled, does not release the attempt:
/// its reservation stands as it was, and a later run on its worktree is
/// refused by its lease, the end in its protected journal proving nothing
/// without the fact. Under the fence rule the settlement fails the run
/// with the typed refusal where evidence was at stake as it was admitted;
/// with nothing at stake it does not, the fence standing or lifted.
#[test]
fn a_positive_fact_that_cannot_be_sealed_keeps_the_reservation() {
    for mut case in fenced() {
        let at = case.host.reservation(&case.work, &case.run);
        let before = standing(&at);
        LATE.set(Some(Late::Unstage));
        let how = case.how.clone();
        assert_eq!(
            case.settles(),
            case.refused(Unreleased::standing, Unknown::Unsealed),
            "{how}"
        );
        kept(&at, &before, &how);
        let later = claim(&case.work, ["run-b", "b1"], Protects::Nothing, &[]);
        let held = cr2(Conflict::Held(case.run.clone()));
        assert_eq!(case.host.admit(&later), held, "{how}");
    }
}

/// The fence rule's three cases: what the attempt protects, whether the
/// fence is lifted as it settles, and whether a settlement refused then
/// fails its run: only where evidence is at stake (operator ruling of
/// 2026-10-29 on U9b1's fourth run).
const FENCED: [(Protects, bool, bool); 3] = [
    (Protects::Nothing, false, false),
    (Protects::Evidence, false, true),
    (Protects::Nothing, true, false),
];

/// One case of [`FENCED`] on a fresh host: its name, an engine leasing
/// its run's attempt `a1`, which ended and whose drivers settled, its
/// worktree and run, whether the fence is lifted as it settles, and
/// whether a settlement refused then fails its run.
struct Fenced {
    how: String,
    host: Host,
    _dir: tempfile::TempDir,
    engine: Engine,
    work: PathBuf,
    run: String,
    lifted: bool,
    fails: bool,
}

/// Each case of [`FENCED`], made.
fn fenced() -> impl Iterator<Item = Fenced> {
    FENCED.into_iter().map(|(protects, lifted, fails)| {
        let host = Host::new();
        let (_dir, mut engine, work, run) = leasing(&host, protects);
        settled_a1(&mut engine, &run);
        Fenced {
            how: format!("{protects:?}, the fence lifted: {lifted}"),
            host,
            _dir,
            engine,
            work,
            run,
            lifted,
            fails,
        }
    })
}

impl Fenced {
    /// Its engine's settlement ([`settles`]).
    fn settles(&mut self) -> Result<(), Unreleased> {
        settles(&mut self.engine, self.lifted, &self.how)
    }

    /// What its settlement returns where a seal is refused by `unknown`,
    /// its lease left as `left` says (L1 of U8a2c's review): the run failed
    /// by MB3's identity cause where the fence rule asks it, otherwise
    /// settled.
    fn refused(
        &self,
        left: fn(Unadmitted) -> Unreleased,
        unknown: Unknown,
    ) -> Result<(), Unreleased> {
        match self.fails {
            true => Err(left(Unadmitted::Identity(unknown))),
            false => Ok(()),
        }
    }
}

/// `engine`'s settlement of its attempt `a1`, the fence lifted as it
/// settles where `lifted`: done, or the refusal it fails its run with.
fn settles(engine: &mut Engine, lifted: bool, how: &str) -> Result<(), Unreleased> {
    let lift = lifted.then(McpFence::lift);
    let settled = engine.settle("a1", Ok(())).map_err(|error| match error {
        EngineError::Unreleased(unreleased) => unreleased,
        other => panic!("{how}: {other}"),
    });
    drop(lift);
    settled
}

/// The reservation at `at`, read.
fn reserved(at: &Path) -> Reservation {
    serde_json::from_slice(&std::fs::read(at).unwrap()).unwrap()
}

/// R1 (U8a2a's fourth run, fixed by U8a2c): a reseal whose last directory
/// sync fails once the record that stood is retired reports itself
/// committed, never failed or late: its successor is the record every read
/// finds and no prior name is left, for a run's next attempt joining it,
/// the release, the engine marking a driver's cleanup unresolved, and
/// recovery at a later admission alike; each lease it leaves is held
/// unsynced.
#[test]
fn a_reseal_whose_last_sync_fails_reports_itself_committed() {
    let (host, at, before) = joining(Late::FailFinal);
    let work = host.worktree("work");
    let joined = admit(&host.root, &open(&work, ["run-a", "a2"]), soon()).unwrap();
    assert!(joined.unsynced, "joined: held unsynced");
    assert_ne!(standing(&at), before, "joined: its successor stands");
    let both = vec!["a1".to_string(), "a2".to_string()];
    assert_eq!(host.attempts(&work, "run-a"), (both, Vec::new()));
    assert!(!prior(&at).exists(), "joined: no prior name left");
    assert_eq!(proved(&joined, soon()), Ok(Commit::Synced));
    LATE.set(Some(Late::FailFinal));
    assert_eq!(release(&joined, soon()), Ok(Commit::Unsynced), "released");
    let released = (vec!["a1".to_string()], vec!["a2".to_string()]);
    assert_eq!(host.attempts(&work, "run-a"), released);
    assert!(!prior(&at).exists(), "released: no prior name left");
    let host = Host::new();
    let (_dir, mut engine, work, run) = leasing(&host, Protects::Evidence);
    LATE.set(Some(Late::FailFinal));
    let unresolved = Cleanup::Unresolved {
        reason: Unsettled::Stdout,
    };
    witnessed_by(&mut engine.held, &unresolved, soon());
    let marked = reserved(&host.reservation(&work, &run));
    assert_eq!(
        marked.attempts["a1"].cleanup,
        Witnessed::Unresolved,
        "marked"
    );
    assert!(
        engine.held.as_ref().is_some_and(|held| held.unsynced),
        "marked: held unsynced"
    );
    // Such a lease fails its run as it settles beside evidence, under the
    // fence rule.
    settled_a1(&mut engine, &run);
    let settled = settles(&mut engine, false, "marked");
    // Its lease was never released, its cleanup unresolved (L1).
    let unsynced = Unreleased::standing(Unadmitted::Identity(Unknown::Unsynced));
    assert_eq!(settled, Err(unsynced), "marked: its run failed");
    let host = Host::new();
    let (_dir, _engine, work, run) = died_after_its_end(&host);
    LATE.set(Some(Late::FailFinal));
    let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    let recovered = admit(&host.root, &next, soon()).unwrap();
    assert!(recovered.unsynced, "recovered: held unsynced");
    let settled = (Vec::new(), vec!["a1".to_string()]);
    assert_eq!(host.attempts(&work, &run), settled, "recovered");
    assert!(
        !prior(&host.reservation(&work, &run)).exists(),
        "recovered: no prior name left"
    );
}

/// L1 of U8a2c's review: a settlement that fails its run says whether its
/// lease stands or was released, in the operator's words.
#[test]
fn a_failed_settlement_says_whether_its_lease_was_released() {
    let cause = || Unadmitted::Identity(Unknown::Unsynced);
    let mb3 = format!(
        "dispatch refused: {}: the attempt's reservation was committed, but its directory was \
         not proven synced",
        brokkr_protocol::broker::Refusal::Identity
    );
    let standing = "the attempt's writer lease stands, for its settlement was not sealed";
    let released = "the attempt's writer lease was released, but a seal of its reservation \
                    was not proven durable";
    let cases = [
        (Unreleased::standing(cause()), standing),
        (Unreleased::released(cause()), released),
    ];
    for (unreleased, text) in cases {
        assert_eq!(unreleased.to_string(), format!("{text}: {mb3}"));
    }
    // U8c: one whose retained responses were not published says so, and
    // whether its lease stands or was released.
    let digest = "broker response digest mismatch";
    let unpublished = "the attempt's retained responses were not published";
    let cases = [
        (Lease::Standing, "and its writer lease stands"),
        (Lease::Released, "though its writer lease was released"),
    ];
    for (lease, said) in cases {
        let unreleased = Unreleased::unpublished(lease, Unpublished::Response);
        let text = format!("{unpublished}, {said}: {digest}");
        assert_eq!(unreleased.to_string(), text);
    }
}

/// R1: a settlement whose reseal commits without its directory proven
/// synced releases its lease and fails its run under the fence rule, with
/// MB3's identity cause where evidence was at stake as it was admitted;
/// with nothing at stake it does not, the fence standing or lifted.
#[test]
fn an_unsynced_settlement_fails_its_run_under_the_fence_rule() {
    // Its release committed, so its lease was released (L1).
    for mut case in fenced() {
        LATE.set(Some(Late::FailFinal));
        let how = case.how.clone();
        assert_eq!(
            case.settles(),
            case.refused(Unreleased::released, Unknown::Unsynced),
            "{how}"
        );
        let released = (Vec::new(), vec!["a1".to_string()]);
        let now = case.host.attempts(&case.work, &case.run);
        assert_eq!(now, released, "{how}: released");
    }
}

/// U8c (CR2): settlement publishes what its attempt's brokers retained
/// under its lease, before the release: a cited response verified against
/// its stage is published at its digest; one whose stage is not the cited
/// bytes publishes nothing and fails the run with CR2's cause, the lease
/// released all the same, its processes settled; and an attempt a driver
/// of which ended unresolved publishes nothing, its lease standing.
#[test]
fn settlement_publishes_what_its_attempt_retained_before_its_release() {
    let answer = br#"{"result":{}}"#;
    let (digest, other) = (sha(answer), sha(b"other"));
    let mismatch = Unreleased::unpublished(Lease::Released, Unpublished::Response);
    let unrooted = Unreleased::unpublished(Lease::Standing, Unpublished::Unbound);
    let cases = [
        ("verified", Protects::Evidence, &digest, false, Ok(())),
        ("mismatch", Protects::Evidence, &other, false, Err(mismatch)),
        ("unresolved", Protects::Evidence, &digest, true, Ok(())),
        ("unsealed", Protects::Nothing, &digest, false, Err(unrooted)),
    ];
    for (case, protects, cited, unresolved, expected) in cases {
        let host = Host::new();
        let (_dir, mut engine, work, run) = leasing(&host, protects);
        let repo = canonical::sha256_bytes(work.as_os_str().as_bytes());
        let at = host.root().join(&repo).join(&run);
        let attempt = Provisioned::new(&at, [&repo, &run, "a1"]);
        let plans = attempt.plans(&[true]);
        let answered = [(Some((&answer[..], cited.as_str())), Outcome::Succeeded)];
        attempt.served((0, &plans[0]), true, &answered);
        if unresolved {
            let reason = Unsettled::Stdout;
            witnessed_by(
                &mut engine.held,
                &Cleanup::Unresolved { reason },
                within(400),
            );
        }
        settled_a1(&mut engine, &run);
        let unsealed = case == "unsealed";
        if unsealed {
            LATE.set(Some(Late::Unstage));
        }
        let settled = engine.settle("a1", Ok(())).map_err(|error| match error {
            EngineError::Unreleased(unreleased) => unreleased,
            other => panic!("{case}: {other}"),
        });
        let published = work.join(".forge/artifacts/sha256").join(&digest).is_file();
        let released = host.attempts(&work, &run).1 == ["a1"];
        let found = (case, settled, published, released);
        let kept = !unresolved & !unsealed;
        assert_eq!(found, (case, expected, case == "verified", kept));
    }
}

/// A fresh host whose root exchanges by `exchange`, for the swap a reseal
/// makes after its proof (decision 0108 ruling 3, #440 fix round 440d6).
fn host_exchanging(exchange: Exchange) -> Host {
    let dir = fresh(&base());
    let home = dir.path().join("home");
    let root = Arc::new(super::opened_by(&home, Some(exchange)));
    Host { dir, home, root }
}

/// Nothing is staged beside the record at `at`.
fn nothing_staged(at: &Path) {
    let staged = at.with_file_name("reservation.json.new");
    assert!(!staged.exists(), "nothing staged left beside it");
}

/// The empty file a hand planted at `at` is still there.
fn planted_stays(at: &Path) {
    assert_eq!(
        std::fs::metadata(at).unwrap().len(),
        0,
        "the file planted at its name stays"
    );
}

/// [`spawning_attempt`] on `host`.
fn spawning_attempt_in(host: Host) -> (Host, PathBuf, Held, PathBuf) {
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    spawning(&held, "work", None).unwrap();
    let at = host.reservation(&work, "run-a");
    (host, work, held, at)
}

/// A fresh host whose run `run-a` holds its attempt `a1` standing on the
/// worktree `work`, with a driver record at its site: the lease, the host,
/// and where its reservation stands (#440 fix round 440d5).
fn spawning_attempt() -> (Host, PathBuf, Held, PathBuf) {
    spawning_attempt_in(Host::new())
}

/// The record at `at` with one more settled attempt: a valid reservation a
/// hand made in its place, different from the one that stood.
fn successor(at: &Path) -> Vec<u8> {
    let mut held: Reservation = serde_json::from_slice(&std::fs::read(at).unwrap()).unwrap();
    held.settled.insert("planted".into());
    serde_json::to_vec(&held).unwrap()
}

/// `bytes` made the file at `at`, replacing whatever stands there.
fn plant(at: &Path, bytes: &[u8]) {
    let temp = at.with_file_name("planted.json");
    std::fs::write(&temp, bytes).unwrap();
    std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::rename(&temp, at).unwrap();
}

/// #440 fix round 440d5 (decision 0108 ruling 3): a spawn record's
/// amendment never writes back a reservation that is gone. The driver
/// removes the run's reservation inside the seal's window; the amendment
/// writes nothing, and the reservation stays absent, so the next step's
/// door meets the lost lease and refuses it.
#[test]
fn a_spawn_record_never_writes_back_a_reservation_that_is_gone() {
    let (_host, _work, held, at) = spawning_attempt();
    LATE.set(Some(Late::Remove));
    spawned(&held, "work", 4242);
    assert!(!at.exists(), "the amendment wrote nothing back");
    nothing_staged(&at);
    assert!(!prior(&at).exists(), "no prior name left");
}

/// The reseal replaces only the very record it read: a different file
/// planted at its name inside the window is refused, never overwritten, so
/// a record another writer made is not silently discarded (#440 fix round
/// 440d5).
#[test]
fn a_reseal_never_replaces_a_record_it_did_not_read() {
    let (_host, _work, held, at) = spawning_attempt();
    LATE.set(Some(Late::Replace));
    spawned(&held, "work", 4242);
    planted_stays(&at);
    nothing_staged(&at);
}

/// An operator's release seals the attempt settled; a late driver's spawn
/// record after it leaves the released record exactly as the release sealed
/// it, and never makes one that is gone (#440 fix rounds 440d5 and 440d6).
#[test]
fn a_late_spawn_record_leaves_the_released_record_as_the_release_sealed_it() {
    let (host, work, held, at) = spawning_attempt();
    assert_eq!(proved(&held, soon()), Ok(Commit::Synced));
    assert_eq!(release(&held, soon()), Ok(Commit::Synced));
    let released = (Vec::<String>::new(), vec!["a1".to_string()]);
    assert_eq!(
        host.attempts(&work, "run-a"),
        released,
        "the release settled it"
    );
    let before = std::fs::read(&at).unwrap();
    spawned(&held, "work", 4242);
    assert_eq!(
        std::fs::read(&at).unwrap(),
        before,
        "the late record left the released record"
    );
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a reseal whose name is
/// removed after the record that stood was linked, at the last hook before
/// the atomic exchange, meets `ENOENT` and writes nothing. That is the
/// window 440d5's link-and-rename proof only narrowed.
#[test]
fn a_reseal_writes_nothing_when_the_name_is_removed_before_its_exchange() {
    let (_host, _work, held, at) = spawning_attempt();
    LATE.set(Some(Late::RemoveLinked));
    spawned(&held, "work", 4242);
    assert!(!at.exists(), "the reseal wrote nothing");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a reseal whose committed
/// record is removed before its post-rename sync fails never writes the old
/// record back. The removal stands, and the record that stood is removed.
#[test]
fn a_committed_reseals_undo_never_brings_back_a_removed_record() {
    let (_host, _work, held, at) = spawning_attempt();
    LATE.set(Some(Late::RemoveCommitted));
    spawned(&held, "work", 4242);
    assert!(!at.exists(), "the removal stands");
    assert!(!prior(&at).exists(), "the record that stood is removed");
    nothing_staged(&at);
    assert_eq!(
        recheck(&held),
        Err(Unadmitted::Identity(Unknown::Lost)),
        "the next door meets the lost lease"
    );
}

/// C2 (#1099; decision 0108 ruling 3; 0071 ruling 9): a reseal whose prior
/// name a hand removed is refused with `Unsealed` and leaves no successor
/// standing as the record. At its retirement, or as its sync fails, the
/// undo finds no record that stood to put back: the removal stands, its own
/// successor goes, and nothing is made at the name.
#[test]
fn a_reseal_whose_prior_name_is_gone_at_its_undo_leaves_no_successor() {
    for (late, how) in [
        (Late::RemovePriorRetiring, "at its retirement"),
        (Late::RemovePriorCommitted, "as its sync fails"),
    ] {
        let (_host, _work, held, at) = spawning_attempt();
        LATE.set(Some(late));
        let amended = amend(&held, soon(), |mut reservation| {
            reservation.settled.insert("planted".into());
            Ok(reservation)
        });
        assert_eq!(
            amended,
            Err(Unadmitted::Identity(Unknown::Unsealed)),
            "{how}"
        );
        assert!(!at.exists(), "{how}: no successor stands at the name");
        assert!(!prior(&at).exists(), "{how}: no prior name left");
        nothing_staged(&at);
        assert_eq!(
            recheck(&held),
            Err(Unadmitted::Identity(Unknown::Lost)),
            "{how}: the next door meets the lost lease"
        );
    }
}

/// C2 (#1099; decision 0108 ruling 3): a first seal over a standing record
/// whose prior name a hand removed is undone the same way: its successor goes
/// where it is still its own, and nothing is made at the name.
#[test]
fn a_first_seal_whose_prior_name_is_gone_at_its_undo_leaves_no_successor() {
    for (late, how) in [
        (Late::RemovePriorRetiring, "at its retirement"),
        (Late::RemovePriorCommitted, "as its sync fails"),
    ] {
        let host = Host::new();
        let (at, _) = probe(&host);
        LATE.set(Some(late));
        assert_eq!(sealing(&host.root, SECOND), Err(Unknown::Unsealed), "{how}");
        assert!(!at.exists(), "{how}: no successor stands at the name");
        assert!(!prior(&at).exists(), "{how}: no prior name left");
        nothing_staged(&at);
    }
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a seal's restore of a
/// record that must still stand puts the prior name back by the same
/// exchange, but where the name is gone the removal stands: it removes the
/// prior name and writes nothing at the name.
#[test]
fn a_restore_never_brings_back_a_removed_record() {
    let (_host, _work, held, at) = spawning_attempt();
    std::fs::hard_link(&at, prior(&at)).unwrap();
    std::fs::remove_file(&at).unwrap();
    spawned(&held, "work", 4242);
    assert!(!at.exists(), "the removal stands");
    assert!(!prior(&at).exists(), "the record that stood is removed");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): where the filesystem has
/// no atomic exchange, a reseal fails closed: it refuses by name and the
/// record that stood is unchanged, byte for byte and inode for inode.
#[test]
fn a_reseal_refused_by_its_filesystem_writes_nothing() {
    let (host, at, before) = stood();
    let root = opened_refusing(host.home());
    let reservation: Reservation = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let refused = resealed(&root, &reservation, &reservation, &Budget::new(soon()));
    assert_eq!(refused, Err(Unknown::Exchange));
    assert_eq!(
        Unknown::Exchange.to_string(),
        "the attempt's reservation could not be sealed: this filesystem has no atomic exchange"
    );
    assert_eq!(standing(&at), before, "the record that stood, unchanged");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a run's next attempt on a
/// standing reservation reseals the record its admission read. Removed
/// between that read and the seal, the record is refused and never made
/// again; it is the reproducer the two reviewers planted.
#[test]
fn a_second_attempts_admission_never_writes_back_a_record_removed_after_its_read() {
    let (host, at, _) = stood();
    let work = host.worktree("work");
    let next = open(&work, ["run-a", "a2"]);
    let between = || std::fs::remove_file(&at).unwrap();
    let refused = admit_with(&host.root, &next, soon(), &between);
    assert_eq!(refused, mb3(Unknown::Participant(at.clone())));
    assert!(!at.exists(), "the admission wrote nothing back");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): recovery at a later
/// admission reseals the reservation it read. Removed at the seal, the
/// record is refused and never made again.
#[test]
fn recovery_never_writes_back_a_reservation_removed_at_its_seal() {
    let host = Host::new();
    let (_dir, _engine, work, run) = died_after_its_end(&host);
    let at = host.reservation(&work, &run);
    LATE.set(Some(Late::Remove));
    let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    let admitted = admit(&host.root, &next, soon());
    assert_eq!(admitted, mb3(Unknown::Unsealed));
    assert!(!at.exists(), "recovery wrote nothing back");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a restarted engine's
/// re-adoption reseals the reservation it read. Removed at the seal, the
/// record is refused and never made again.
#[test]
fn a_re_adoption_never_writes_back_a_reservation_removed_at_its_seal() {
    let host = Host::new();
    let (_dir, mut engine, work, run) = leasing(&host, Protects::Nothing);
    let held = engine.held.clone().unwrap();
    spawning(&held, "work", None).unwrap();
    ended(&mut engine.held, "work", &Cleanup::Settled);
    let at = host.reservation(&work, &run);
    let repo = canonical::sha256_bytes(work.as_os_str().as_bytes());
    LATE.set(Some(Late::Remove));
    let readopted = adopted(&host.root, [&repo, &run, "a1"], soon());
    assert_eq!(
        readopted.map(|adoption| adoption.is_some()),
        Err(Unadmitted::Identity(Unknown::Unsealed))
    );
    assert!(!at.exists(), "the adoption wrote nothing back");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): the operator's release
/// reseals the reservation it read. Removed at the seal, the record is
/// refused and never made again.
#[test]
fn an_operators_release_never_writes_back_a_reservation_removed_at_its_seal() {
    let host = Host::new();
    let (_dir, mut engine, work, run) = leasing(&host, Protects::Nothing);
    let payload = serde_json::json!({"effect_id": "e", "attempt_id": "a1", "reason": "killed"});
    engine
        .store
        .append_next(
            &run,
            EventType::EffectIndeterminate,
            payload,
            None,
            Some("a1".into()),
        )
        .unwrap();
    let at = host.reservation(&work, &run);
    LATE.set(Some(Late::Remove));
    let released = attested(&host.root, &run, &mut engine.store, |_, _| Ok(()));
    match released {
        Err(EngineError::Unreleased(unreleased)) => assert_eq!(
            unreleased,
            Unreleased::standing(Unadmitted::Identity(Unknown::Unsealed))
        ),
        other => panic!("not a settlement refusal: {other:?}"),
    }
    assert!(!at.exists(), "the release wrote nothing back");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): the reseal proves the
/// record it read is still the one at its name. A valid record a hand puts
/// there after the admission's read is refused, never replaced, so the
/// record another writer made stands byte for byte.
#[test]
fn a_second_attempts_admission_refuses_a_record_planted_after_its_read() {
    let (host, at, _) = stood();
    let work = host.worktree("work");
    let next = open(&work, ["run-a", "a2"]);
    let planted = successor(&at);
    let between = || plant(&at, &planted);
    let refused = admit_with(&host.root, &next, soon(), &between);
    assert_eq!(refused, mb3(Unknown::Participant(at.clone())));
    assert_eq!(
        std::fs::read(&at).unwrap(),
        planted,
        "the record planted at its name stays"
    );
    assert!(!prior(&at).exists(), "no prior name left");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a restore whose name is
/// removed at its last point before the exchange meets it gone and writes
/// nothing: the removal stands, not the record the restore would put back.
#[test]
fn a_restore_writes_nothing_when_the_name_is_removed_before_its_exchange() {
    let (_host, _work, held, at) = spawning_attempt();
    std::fs::hard_link(&at, prior(&at)).unwrap();
    let planted = successor(&at);
    plant(&at, &planted);
    LATE.set(Some(Late::RemoveRestored));
    spawned(&held, "work", 4242);
    assert!(!at.exists(), "the restore wrote nothing");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// #440 fix round 440d6 (decision 0108 ruling 3): an undo cut short after
/// its exchange leaves the record that stood at its name and the successor
/// under the staged name, never the reverse. The next seal removes the
/// successor and writes nothing over the record that stood.
#[test]
fn a_restore_keeps_the_record_that_stood_after_an_undo_is_cut_short() {
    let (_host, _work, held, at) = spawning_attempt();
    let before = standing(&at);
    LATE.set(Some(Late::CutUndo));
    spawned(&held, "work", 4242);
    assert_eq!(
        standing(&at),
        before,
        "the record that stood is back at its name"
    );
    let staged = at.with_file_name("reservation.json.new");
    assert!(staged.exists(), "the successor is under the staged name");
    assert!(
        prior(&at).exists(),
        "the record that stood keeps its prior name"
    );
    LATE.set(Some(Late::Unstage));
    spawned(&held, "work", 4242);
    assert_eq!(
        standing(&at),
        before,
        "the next seal kept the record that stood"
    );
    assert!(!prior(&at).exists(), "no prior name left");
    assert!(!staged.exists(), "the successor is gone");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): a committed reseal's undo
/// leaves a file a hand planted at the name: the exchange that put it out
/// of the way is put back, and the record that stood is removed rather than
/// written over the hand's file.
#[test]
fn a_committed_undo_leaves_a_file_planted_at_the_name() {
    let (_host, _work, held, at) = spawning_attempt();
    LATE.set(Some(Late::ReplaceCommitted));
    spawned(&held, "work", 4242);
    planted_stays(&at);
    assert!(!prior(&at).exists(), "the record that stood is removed");
    nothing_staged(&at);
}

/// A fresh host whose reseals exchange by `seam` and a reseal of its record
/// with `late` armed, a hand's file at the name once it returns: the host,
/// the lease and where the record stands.
fn a_reseal_over_a_hand(seam: Exchange, late: Late) -> (Host, Held, PathBuf) {
    swaps_again();
    let host = host_exchanging(seam);
    let (host, _work, held, at) = spawning_attempt_in(host);
    swaps_again();
    LATE.set(Some(late));
    spawned(&held, "work", 4242);
    (host, held, at)
}

/// [`a_reseal_over_a_hand`]'s state after a failed exchange back: no name
/// is made, the record that stood goes, and the hand's planted file stays
/// under the staged name.
fn a_failed_exchange_back_leaves_no_name(seam: Exchange, late: Late) {
    let (_host, _held, at) = a_reseal_over_a_hand(seam, late);
    assert!(!at.exists(), "no name is made");
    assert!(!prior(&at).exists(), "the record that stood is removed");
    let staged = at.with_file_name("reservation.json.new");
    assert_eq!(
        std::fs::metadata(&staged).unwrap().len(),
        0,
        "the planted file stays under the staged name"
    );
}

/// #440 fix rounds 440d6 and 440d8 (decision 0108 ruling 3): where the
/// exchange back a foreign file needs fails for a reason other than the
/// name's removal, no plain rename runs either: a rename could make a name
/// the removal took. This seal's successor is removed, and the planted file
/// stays under the staged name.
#[test]
fn a_reseal_whose_exchange_back_fails_leaves_the_planted_file() {
    a_failed_exchange_back_leaves_no_name(failing_second_exchange, Late::Replace);
}

/// C1 (decision 0108 ruling 3; 0071 ruling 9): a reseal's exchange back
/// meets a name a hand removed and lets the removal stand: no rename makes
/// the name again.
#[test]
fn a_reseals_exchange_back_never_makes_a_name_a_removal_took() {
    a_failed_exchange_back_leaves_no_name(unlinking_before_second_exchange, Late::Replace);
}

/// C1 (decision 0108 ruling 3; 0071 ruling 9): an undo's exchange back meets
/// a name a hand removed before the third swap and lets the removal stand:
/// the name stays absent.
#[test]
fn a_committed_undos_exchange_back_never_makes_a_name_a_removal_took() {
    a_failed_exchange_back_leaves_no_name(unlinking_before_third_exchange, Late::ReplaceCommitted);
}

/// F1 (0071 ruling 9; S5): the `ours()` clause is what keeps a hand's file
/// at the name. Where the exchange back fails and the name holds a hand's
/// regular file, it stays, byte for byte, and the record that stood goes.
#[test]
fn a_reseal_whose_exchange_back_fails_leaves_a_file_a_hand_planted_at_the_name() {
    let (_host, _held, at) = a_reseal_over_a_hand(
        failing_second_exchange_over_the_staged_and_the_hand,
        Late::Replace,
    );
    assert_eq!(
        std::fs::read(&at).unwrap(),
        b"a hand's file",
        "the name reads the hand's bytes"
    );
    assert!(!prior(&at).exists(), "the record that stood is removed");
}

/// #440 fix round 440d6 (decision 0108 ruling 3): every way a filesystem
/// says it has no atomic exchange refuses by name, and the record that
/// stood is unchanged byte for byte and inode for inode.
#[test]
fn a_reseal_refused_by_each_exchange_refusal_writes_nothing() {
    let refusals: [(rustix::io::Errno, &str); 4] = [
        (rustix::io::Errno::INVAL, "EINVAL"),
        (rustix::io::Errno::NOTSUP, "ENOTSUP"),
        (rustix::io::Errno::OPNOTSUPP, "EOPNOTSUPP"),
        (rustix::io::Errno::NOSYS, "ENOSYS"),
    ];
    for (error, why) in refusals {
        refusing(error);
        let (host, at, before) = stood();
        let root = super::opened_by(host.home(), Some(refusing_exchange));
        let reservation: Reservation =
            serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
        let refused = resealed(&root, &reservation, &reservation, &Budget::new(soon()));
        assert_eq!(refused, Err(Unknown::Exchange), "{why}");
        assert_eq!(
            standing(&at),
            before,
            "{why}: the record that stood, unchanged"
        );
        assert!(!prior(&at).exists(), "{why}: no prior name left");
        let staged = at.with_file_name("reservation.json.new");
        assert!(!staged.exists(), "{why}: nothing staged left beside it");
    }
}

/// The run `run`'s directory below `host`'s root, and where its reservation
/// stands: what a test that calls the root's own reseal reads by.
fn run_at(host: &Host, work: &Path, run: &str) -> (std::os::fd::OwnedFd, PathBuf) {
    let key = canonical::sha256_bytes(work.as_os_str().as_bytes());
    (
        host.root.below(&[&key, run]).unwrap(),
        host.reservation(work, run),
    )
}

/// The bytes of each first, second and third seal of the reservation in a
/// test's own root.
const FIRST: &[u8] = b"{\"first\":1}";
const SECOND: &[u8] = b"{\"second\":2}";
const THIRD: &[u8] = b"{\"third\":3}";

/// `bytes` sealed as the reservation in `root` before `deadline`.
fn sealing_by(root: &Root, bytes: &[u8], deadline: Instant) -> Result<Commit, Unknown> {
    root.seal(root.dir(), RESERVATION, bytes, &Budget::new(deadline))
}

/// [`sealing_by`] within the house's five seconds.
fn sealing(root: &Root, bytes: &[u8]) -> Result<Commit, Unknown> {
    sealing_by(root, bytes, soon())
}

/// A first seal of the reservation at `at`, and how it stood.
fn first_seal(root: &Root, at: &Path) -> (Vec<u8>, (u64, u64)) {
    assert_eq!(sealing(root, FIRST), Ok(Commit::Synced));
    standing(at)
}

/// A fresh host whose root holds the reservation `RESERVATION` from a first
/// seal, and how it stood.
fn probe(host: &Host) -> (PathBuf, (Vec<u8>, (u64, u64))) {
    let at = host.root().join(RESERVATION);
    let before = first_seal(&host.root, &at);
    (at, before)
}

/// [`probe`] on a root whose reseals exchange by the refusing seam.
fn refusing_probe(host: &Host) -> (Root, PathBuf, (Vec<u8>, (u64, u64))) {
    let root = opened_refusing(host.home());
    let at = host.root().join(RESERVATION);
    let before = first_seal(&root, &at);
    (root, at, before)
}

/// [`probe`], its record linked under its prior name as a seal cut short
/// between its link and its exchange leaves it.
fn linked_probe(host: &Host) -> (PathBuf, (Vec<u8>, (u64, u64))) {
    let (at, before) = probe(host);
    std::fs::hard_link(&at, prior(&at)).unwrap();
    (at, before)
}

/// A standing reservation read whole, with a file planted at its name and
/// at its prior name, for a test that calls the root's own reseal.
struct Restoring {
    host: Host,
    run: std::os::fd::OwnedFd,
    at: PathBuf,
    old: Vec<u8>,
    next: Vec<u8>,
    stood: Record,
    read: Vec<u8>,
}

/// A fresh host whose run `run-a` holds its attempt `a1` standing, with the
/// record and bytes a reseal reads.
fn restoring() -> Restoring {
    let (host, work, _held, at) = spawning_attempt();
    let (run, _) = run_at(&host, &work, "run-a");
    let old = std::fs::read(&at).unwrap();
    let (_, stood, read) = identified::<Reservation>(&run, RESERVATION, &at).unwrap();
    let next = successor(&at);
    Restoring {
        host,
        run,
        at,
        old,
        next,
        stood,
        read,
    }
}

impl Restoring {
    /// The name holds a valid successor, and the prior name the record that
    /// stood: a seal cut short after its exchange.
    fn replace(&self) {
        plant(&self.at, &self.next);
        plant(&prior(&self.at), &self.old);
    }

    /// The reservation resealed in `root` with `bytes`, over the record this
    /// case read.
    fn reseal(&self, root: &Root, bytes: &[u8]) -> Result<Commit, Unknown> {
        root.reseal(
            &self.run,
            RESERVATION,
            bytes,
            &Budget::new(soon()),
            &self.stood,
            &self.read,
        )
    }

    /// [`reseal`] in the case's own root.
    fn reseal_in(&self, bytes: &[u8]) -> Result<Commit, Unknown> {
        self.reseal(&self.host.root, bytes)
    }

    /// Both names are left as `name` and `at_prior` say, and nothing is
    /// staged.
    fn leaves(&self, name: &[u8], at_prior: &[u8]) {
        assert_eq!(std::fs::read(&self.at).unwrap(), name, "the name is left");
        assert_eq!(
            std::fs::read(prior(&self.at)).unwrap(),
            at_prior,
            "the prior name is left"
        );
        nothing_staged(&self.at);
    }

    /// The name holds `name`, the prior name goes, and nothing is staged.
    fn keeps(&self, name: &[u8]) {
        assert_eq!(std::fs::read(&self.at).unwrap(), name, "the name stays");
        assert!(!prior(&self.at).exists(), "the prior name goes");
        nothing_staged(&self.at);
    }
}

/// A fresh host whose root exchanges by `seam`, its reservation resealed
/// once with a hand's file at its name: the seam's second swap is the
/// exchange back a foreign file needs.
fn put_backs_failed(seam: Exchange) -> (Host, PathBuf, PathBuf) {
    swaps_again();
    let host = host_exchanging(seam);
    let (host, _work, held, at) = spawning_attempt_in(host);
    swaps_again();
    let staged = at.with_file_name("reservation.json.new");
    LATE.set(Some(Late::Replace));
    spawned(&held, "work", 4242);
    (host, at, staged)
}

/// Run four's review, S1: a first seal that may create, replacing a record
/// that stood, undoes after its commit by a plain rename even on a
/// filesystem with no atomic exchange: the record that stood is back, the
/// same file, and the next seal succeeds.
#[test]
fn a_first_seals_undo_puts_the_record_that_stood_back_without_an_exchange() {
    let host = Host::new();
    let (root, at, before) = refusing_probe(&host);
    LATE.set(Some(Late::FailCommitted));
    assert_eq!(
        sealing(&root, SECOND),
        Err(Unknown::Unsealed),
        "its sync failed once renamed in"
    );
    assert_eq!(standing(&at), before, "the record that stood is back");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
    assert_eq!(
        sealing(&root, THIRD),
        Ok(Commit::Synced),
        "the next seal succeeds"
    );
}

/// Run four's review, S1: a first seal over a standing record whose name a
/// hand replaced inside its window is refused, and the hand's file stays.
#[test]
fn a_first_seals_undo_leaves_a_file_planted_at_the_name() {
    let host = Host::new();
    let (at, _) = probe(&host);
    LATE.set(Some(Late::ReplaceCommitted));
    assert_eq!(sealing(&host.root, SECOND), Err(Unknown::Unsealed));
    planted_stays(&at);
    assert!(!prior(&at).exists(), "the record that stood is removed");
    nothing_staged(&at);
}

/// Run four's review, S1: a first seal whose staged copy cannot be written
/// leaves no record and nothing staged.
#[test]
fn a_first_seal_whose_staged_copy_cannot_be_written_leaves_no_record() {
    let host = Host::new();
    let at = host.root().join(RESERVATION);
    LATE.set(Some(Late::FailStaged));
    assert_eq!(sealing(&host.root, FIRST), Err(Unknown::Unsealed));
    assert!(!at.exists(), "no record");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// Run four's review, S1: a first seal's undo whose name a hand removed
/// lets the removal stand and removes the record that stood's prior name.
#[test]
fn a_first_seals_undo_lets_a_removed_name_stand() {
    let host = Host::new();
    let (at, _) = probe(&host);
    LATE.set(Some(Late::RemoveCommitted));
    assert_eq!(sealing(&host.root, SECOND), Err(Unknown::Unsealed));
    assert!(!at.exists(), "the removal stands");
    assert!(!prior(&at).exists(), "the prior name goes");
    nothing_staged(&at);
}

/// Run four's review, S1: a first seal that made the record removes it again
/// where it is still that seal's file, and leaves a hand's file where it is
/// not.
#[test]
fn a_first_seal_with_nothing_standing_removes_what_it_made() {
    for (armed, planted) in [
        (Late::RemoveCommitted, false),
        (Late::ReplaceCommitted, true),
    ] {
        let host = Host::new();
        let at = host.root().join(RESERVATION);
        LATE.set(Some(armed));
        assert_eq!(sealing(&host.root, FIRST), Err(Unknown::Unsealed));
        if planted {
            planted_stays(&at);
        } else {
            assert!(!at.exists(), "{armed:?}: the record it made is removed");
        }
        assert!(!prior(&at).exists(), "{armed:?}: no prior name left");
        nothing_staged(&at);
    }
}

/// Run four's review, S1: a first seal cut short between its link and its
/// rename is restored by the next seal, whether it left the record that
/// stood under both names or a hand removed its name: the removal stands
/// and the prior name goes.
#[test]
fn a_first_seal_cut_short_between_its_link_and_its_rename_is_restored() {
    for removed in [false, true] {
        let host = Host::new();
        let (at, before) = linked_probe(&host);
        if removed {
            std::fs::remove_file(&at).unwrap();
        }
        LATE.set(Some(Late::Unstage));
        assert_eq!(sealing(&host.root, SECOND), Err(Unknown::Unsealed));
        if removed {
            assert!(!at.exists(), "the removal stands");
        } else {
            assert_eq!(standing(&at), before, "the record that stood stays");
        }
        assert!(!prior(&at).exists(), "its second link goes");
        nothing_staged(&at);
    }
}

/// Run four's review, S1: a first seal cut short after its commit, with a
/// hand's file at the name, is put back by its restore: the record that
/// stood returns and the hand's file goes.
#[test]
fn a_first_seals_restore_puts_the_record_that_stood_back_over_a_replacement() {
    let host = Host::new();
    let (at, before) = linked_probe(&host);
    plant(&at, b"{\"a hand\":true}");
    LATE.set(Some(Late::Unstage));
    assert_eq!(sealing(&host.root, SECOND), Err(Unknown::Unsealed));
    assert_eq!(standing(&at), before, "the record that stood is back");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// Run four's review, S1a: a first seal that may create, cut short as it
/// retires the record that stood, undoes without an exchange. On a
/// filesystem with no atomic exchange it still refuses with the refusal
/// that cut it short and leaves that record back at its name.
#[test]
fn a_first_seal_late_as_it_retires_undoes_without_an_exchange() {
    let cases = [
        (Late::FailRetiring, Unknown::Unsealed, soon()),
        (
            Late::SleepRetiring(Duration::from_millis(1_200)),
            Unknown::Expired,
            Instant::now() + Duration::from_millis(800),
        ),
    ];
    for (armed, refused, deadline) in cases {
        let host = Host::new();
        let (root, at, before) = refusing_probe(&host);
        LATE.set(Some(armed));
        assert_eq!(
            sealing_by(&root, SECOND, deadline),
            Err(refused),
            "{armed:?}"
        );
        assert_eq!(
            standing(&at),
            before,
            "{armed:?}: the record that stood is back"
        );
        assert!(!prior(&at).exists(), "{armed:?}: no prior name left");
        nothing_staged(&at);
    }
}

/// Run four's review, S2: a reseal's undo whose exchange fails unlinks only
/// its own staged copy and leaves the record that stood under its prior
/// name.
#[test]
fn a_committed_undos_exchange_failure_keeps_the_record_that_stood() {
    swaps_again();
    let host = host_exchanging(failing_second_exchange);
    let (_host, _work, held, at) = spawning_attempt_in(host);
    swaps_again();
    let before = std::fs::read(&at).unwrap();
    LATE.set(Some(Late::FailCommitted));
    spawned(&held, "work", 4242);
    assert_eq!(
        std::fs::read(prior(&at)).unwrap(),
        before,
        "the record that stood is under its prior name"
    );
    assert_ne!(
        std::fs::read(&at).unwrap(),
        before,
        "its successor is at the name"
    );
    nothing_staged(&at);
}

/// Run four's review, S3: a restore whose prior name is a file that is
/// neither the record the name holds nor the bytes the path read refuses,
/// leaving both names as they are.
#[test]
fn a_restore_refuses_a_prior_name_that_is_neither_its_record_nor_the_names() {
    let case = restoring();
    case.replace();
    plant(&prior(&case.at), b"neither the record nor its successor");
    assert_eq!(case.reseal_in(&case.next), Err(Unknown::Unsealed));
    case.leaves(&case.next, b"neither the record nor its successor");
}

/// Run four's review, S3: a restore whose name already holds the record the
/// path read removes a file planted at its prior name and writes nothing at
/// the name.
#[test]
fn a_restore_leaves_the_record_that_stands_over_a_file_at_its_prior_name() {
    let case = restoring();
    plant(&prior(&case.at), b"a file planted at the prior name");
    LATE.set(Some(Late::Unstage));
    assert_eq!(case.reseal_in(&case.old), Err(Unknown::Unsealed));
    case.keeps(&case.old);
}

/// Run four's review, S3: a restore whose proof step fails leaves both
/// names as the step found them.
#[test]
fn a_restores_failed_step_leaves_both_names() {
    let case = restoring();
    case.replace();
    LATE.set(Some(Late::FailRestored));
    assert_eq!(case.reseal_in(&case.next), Err(Unknown::Unsealed));
    case.leaves(&case.next, &case.old);
}

/// Run four's review, S3: a restore whose exchange the filesystem refuses
/// leaves both names as they are and refuses by name.
#[test]
fn a_restores_refused_exchange_leaves_both_names() {
    let case = restoring();
    case.replace();
    let root = super::opened_by(case.host.home(), Some(refusing_exchange));
    assert_eq!(case.reseal(&root, &case.next), Err(Unknown::Exchange));
    case.leaves(&case.next, &case.old);
}

/// Run four's review, S4: a lease whose record a hand removed, with only its
/// prior name beside it, is lost at the door: the inventory refuses the same
/// state as an unknown participant.
#[test]
fn a_door_refuses_a_record_whose_name_a_hand_removed_beside_its_prior() {
    let (_host, _work, held, at) = spawning_attempt();
    let old = std::fs::read(&at).unwrap();
    plant(&at, &successor(&at));
    plant(&prior(&at), &old);
    std::fs::remove_file(&at).unwrap();
    assert_eq!(
        recheck(&held),
        Err(Unadmitted::Identity(Unknown::Lost)),
        "the door meets the lost lease"
    );
}

/// F3 (0071 ruling 5): a lone prior name is no record at the door, in
/// settlement's read or at the amend's seal. All three refuse and none
/// writes the name; the amend's restore removes the stale prior name.
#[test]
fn a_lone_prior_name_is_refused_by_the_door_the_amend_and_settlement() {
    let (_host, _work, held, at) = spawning_attempt();
    let old = std::fs::read(&at).unwrap();
    std::fs::remove_file(&at).unwrap();
    std::fs::write(prior(&at), &old).unwrap();
    std::fs::set_permissions(prior(&at), std::fs::Permissions::from_mode(0o600)).unwrap();

    assert_eq!(
        recheck(&held),
        Err(Unadmitted::Identity(Unknown::Lost)),
        "the door meets the lost lease"
    );
    assert!(!at.exists(), "the door writes nothing");
    assert!(prior(&at).exists(), "the door leaves the lone prior name");

    let read = crate::engine::broker::settlement::standing(
        &held.coordination,
        [held.repo.as_str(), held.run.as_str()],
    );
    assert_eq!(
        read,
        Err(Unknown::Lost),
        "settlement's read refuses the same state"
    );
    assert!(!at.exists(), "settlement's read writes nothing");
    assert!(
        prior(&at).exists(),
        "settlement's read leaves the lone prior name"
    );

    assert_eq!(
        amend(&held, soon(), Ok),
        Err(Unadmitted::Identity(Unknown::Unsealed)),
        "the amend refuses"
    );
    assert!(!at.exists(), "the amend writes no name");
    assert!(
        !prior(&at).exists(),
        "the restore removes the lone prior name"
    );
}

/// Run four's review, S5: where a reseal's exchange back fails and the
/// name is a directory, the planted file stays under the staged name and the
/// record that stood is removed, rather than the planted file being deleted.
#[test]
fn a_reseal_whose_exchange_back_and_rename_both_fail_leaves_the_planted_file() {
    let (_host, at, staged) = put_backs_failed(failing_second_exchange_over_the_name);
    assert!(
        std::fs::metadata(&at).unwrap().is_dir(),
        "the hand's entry at the name stays"
    );
    assert_eq!(
        std::fs::metadata(&staged).unwrap().len(),
        0,
        "the planted file stays under the staged name"
    );
    assert!(!prior(&at).exists(), "the record that stood is removed");
}

/// Run four's review, S5: where neither put-back can run and the name still
/// holds this seal's own successor, that successor is removed: a refused
/// reseal never leaves a record standing.
#[test]
fn a_reseals_exchange_back_that_fails_removes_its_successor() {
    let (_host, at, staged) = put_backs_failed(failing_second_exchange_over_the_staged);
    assert!(!at.exists(), "the successor is removed");
    assert!(
        std::fs::metadata(&staged).unwrap().is_dir(),
        "the hand's entry under the staged name stays"
    );
    assert!(!prior(&at).exists(), "the record that stood is removed");
}

/// Run four's review, S6: a reseal whose link of the record that stood
/// cannot be proven writes nothing: the name stays and no prior name is
/// left.
#[test]
fn a_reseal_whose_linked_name_cannot_be_proven_writes_nothing() {
    let (_host, _work, held, at) = spawning_attempt();
    let before = std::fs::read(&at).unwrap();
    LATE.set(Some(Late::FailLinked));
    spawned(&held, "work", 4242);
    assert_eq!(std::fs::read(&at).unwrap(), before, "the name stays");
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// Run four's review, S6: a record a crash left under its staged name, with
/// the record that stood still beside its prior name, reads as the record
/// that stood, not as an unknown participant.
#[test]
fn a_record_under_its_staged_name_beside_its_prior_is_the_record_that_stood() {
    let (host, work, _held, at) = spawning_attempt();
    let (run, _) = run_at(&host, &work, "run-a");
    let before: Reservation = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    std::fs::hard_link(&at, prior(&at)).unwrap();
    let staged = at.with_file_name("reservation.json.new");
    std::fs::hard_link(&at, &staged).unwrap();
    plant(&at, &successor(&at));
    let read: Reservation = read(&run, RESERVATION, &at).unwrap();
    assert_eq!(
        read, before,
        "the record that stood is what every read finds"
    );
}

/// Run four's review, S6: a first seal whose rename of its staged copy
/// fails after it linked the record that stood removes that link again.
#[test]
fn a_first_seals_failed_rename_removes_the_link_it_made() {
    let host = Host::new();
    let (at, before) = probe(&host);
    LATE.set(Some(Late::Unstage));
    assert_eq!(sealing(&host.root, SECOND), Err(Unknown::Unsealed));
    assert_eq!(standing(&at), before, "the record that stood is unchanged");
    assert!(!prior(&at).exists(), "the link it made is removed");
    nothing_staged(&at);
}

/// A reservation standing in another build's byte form, with the successor
/// a seal cut short after its exchange would leave at the name: the name,
/// the prior name holding the record that stood as that build wrote it, the
/// record read, and the lease that stands on it.
fn another_byte_form() -> (Host, PathBuf, Held, Reservation) {
    let (host, _work, held, at) = spawning_attempt();
    let record: Reservation = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let written = serde_json::to_vec_pretty(&record).unwrap();
    assert_ne!(
        written,
        serde_json::to_vec(&record).unwrap(),
        "another build's byte form"
    );
    plant(&at, &written);
    plant(&prior(&at), &written);
    plant(&at, &successor(&at));
    (host, at, held, record)
}

/// Run four's review, S7: a reseal restores the record that stood from its
/// own bytes, not a re-serialisation, so a cut-short state another build
/// wrote in another byte form is undone rather than refused.
#[test]
fn a_reseal_restores_a_record_written_in_another_byte_form() {
    let (host, at, _held, record) = another_byte_form();
    assert_eq!(
        resealed(&host.root, &record, &record, &Budget::new(soon())),
        Ok(Commit::Synced),
        "the cut-short state is restored"
    );
    assert_eq!(
        std::fs::read(&at).unwrap(),
        serde_json::to_vec(&record).unwrap(),
        "the resealed record stands"
    );
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}

/// Run four's review, S7: an amend reads and restores a record another build
/// wrote in another byte form by the bytes it read, not a re-serialisation.
#[test]
fn an_amend_restores_a_record_written_in_another_byte_form() {
    let (_host, at, held, record) = another_byte_form();
    let amended = amend(&held, soon(), Ok);
    assert_eq!(amended, Ok(Commit::Synced), "the amend restores it");
    assert_eq!(
        std::fs::read(&at).unwrap(),
        serde_json::to_vec(&record).unwrap(),
        "the amended record stands"
    );
    assert!(!prior(&at).exists(), "no prior name left");
    nothing_staged(&at);
}
