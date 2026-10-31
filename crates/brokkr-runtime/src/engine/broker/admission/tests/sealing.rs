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

use std::os::unix::fs::MetadataExt;

use brokkr_protocol::broker::ledger::Outcome;

use super::coordination::{died_after_its_end, settled_a1};
use super::*;
use crate::capabilities::McpFence;
use crate::engine::artifacts::tests::{sha, Provisioned};
use crate::engine::artifacts::Unpublished;
use crate::engine::broker::root::Commit;
use crate::engine::broker::settlement::{Lease, Unreleased};

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
        let sealed = seal_with(host.root.dir(), "probe.json", b"{}", &budget, late);
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
    let mb3 = "dispatch refused: MCP server box filesystem identity is not protected: the \
               attempt's reservation was committed, but its directory was not proven synced";
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
