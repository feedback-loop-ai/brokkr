//! U8a2a (CR2, MB3): the writer lease and its durable reservation, proved
//! on a coordination root of each test's own, opened and handed to
//! admission's core as production's resolution would hand it, on any
//! TMPDIR (R5): the admission operation itself, then dispatch's own calls
//! of it, before any driver starts and after the attempt settles. Only
//! the tests of that resolution from HOME itself ([`Host::resolving`]) need
//! a location whose every ancestor protects. The coordination comparison is
//! in [`coordination`]; settlement from an engine's own sealed journal in
//! [`recovery`].

use std::cell::{Cell, RefCell};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_core::envelope::EventType;
use brokkr_protocol::hands::{Bind, BindMode, HandsSpec};
use brokkr_protocol::process::Unsettled;
use brokkr_protocol::Cleanup;
use rustix::fs::{flock, AtFlags, FlockOperation};
use serde_json::json;

use super::*;
use crate::bundle::{SeatBody, SequenceStep};
use crate::engine::broker::inventory::Budget;
use crate::engine::broker::root::{coordination, seal_with, Commit, Step, PRIOR};
use crate::engine::broker::settlement::{proved, witnessed, witnessed_by};
use crate::engine::broker::{
    owned, surveyed, Host as Hosting, Surveyor, Unprovisioned, PROTECTED_ROOT,
};
use crate::engine::tests::{
    capturing_driver_command, draft_then_finish, engine, execute_work, in_its_own_engine, member,
    requested, set_site_hands, single_body,
};
use crate::engine::{spawn_site, Door, SiteSpawn};

mod coordination;
mod foreign;
mod processes;
mod reach;
mod recovery;
mod sealing;
// U8a2b's proofs read the mount table, which only Linux has (decision 0063).
#[cfg(target_os = "linux")]
mod sources;
#[cfg(not(target_os = "linux"))]
use crate::engine::broker::sources::observed as shown;
#[cfg(target_os = "linux")]
use sources::shown;
#[cfg(target_os = "linux")]
pub(in crate::engine) use sources::{
    leased, proven_settled, staked, standing_writer, unsynced_next,
};

/// Where admission reads its coordination on a thread under test.
#[derive(Clone)]
enum Stand {
    /// The provisioning stand-in's HOME, or else one of this thread's own,
    /// each with a root opened in it.
    Own,
    /// The HOME a test stood in, with a root opened in it.
    At(PathBuf),
    /// The HOME a test stood in, with no root that could be protected.
    Unrooted(PathBuf),
    /// The host's, as production reads it.
    Host,
}

/// What the next seal on a root this thread opened does at its final steps
/// (H1; R1 of run four's review): before its commit, spends the time given,
/// removes its staged copy so its commit fails, or plants a file at the
/// record's prior name so it cannot link the record that stood there while
/// the rename after the link stays possible (R3, U8a2c); once renamed in,
/// spends the time given, fails as its directory sync failing would, or
/// spends it with the directory's write permission taken, or spends it and
/// then fails the sync of its undoing (M2 of run four's second review); as
/// it retires the record that stood, spends the time given or fails as the
/// removal of its prior name failing would; and once committed, fails as
/// its last directory sync failing would (R1, U8a2c).
#[derive(Debug, Clone, Copy)]
enum Late {
    Sleep(Duration),
    Unstage,
    Occupy,
    SleepCommitted(Duration),
    FailCommitted,
    DenyCommitted(Duration),
    FailUndone(Duration),
    Undoing,
    SleepRetiring(Duration),
    FailRetiring,
    FailFinal,
}

thread_local! {
    static STAND: RefCell<Stand> = const { RefCell::new(Stand::Own) };
    /// The HOME of this thread's own a test that stands none in admits
    /// under, removed as the thread ends.
    static OWN: tempfile::TempDir = fresh(&base());
    /// What the next seal does before its commit, once ([`late`]).
    static LATE: Cell<Option<Late>> = const { Cell::new(None) };
}

/// The host as dispatch observes it under test, the house's seam as
/// provisioning's is ([`brokered_tests::stood_in`]): admission's HOME the
/// one a test stood in on this thread, none where it asked for the host's,
/// or else the one provisioning's stand-in names, or one of this thread's
/// own, so no two tests share a coordination root unless they ask to, its
/// root opened with no ancestor judged; provisioning's host and survey its
/// stand-in's, or the host's. Never the host's HOME unasked, and never a
/// root resolved from it.
///
/// [`brokered_tests::stood_in`]: crate::engine::capability_tests::provision_tests::brokered_tests::stood_in
pub(in crate::engine) fn observed() -> Option<Observed> {
    let provisioning = crate::engine::capability_tests::provision_tests::brokered_tests::stood_in();
    type Open = fn(&Path) -> Option<Root>;
    let (home, open): (PathBuf, Open) = match STAND.with_borrow(Clone::clone) {
        Stand::At(home) => (home, |home| Some(opened(home))),
        Stand::Unrooted(home) => (home, |_| None),
        Stand::Host => return None,
        Stand::Own => {
            let named = provisioning.as_ref().map(|(host, _)| host.home.clone());
            let home = named.unwrap_or_else(|| OWN.with(|own| own.path().join("home")));
            (home, |home| Some(opened(home)))
        }
    };
    let (host, survey): (Result<Hosting, Unprovisioned>, Surveyor<'static>) = match provisioning {
        Some((host, survey)) => (Ok(host), survey),
        None => (
            Hosting::of(std::env::vars_os(), std::env::current_exe()),
            &surveyed,
        ),
    };
    let home = Some(home);
    Some(Observed {
        host,
        survey,
        home,
        open,
    })
}

/// The coordination root under `home`, made owner-only where it is missing
/// and opened, as production's resolution hands it, but with no ancestor
/// judged, its seals handed to [`late`]: the handle every fixture admits in
/// (R5).
pub(in crate::engine) fn opened(home: &Path) -> Root {
    let path = coordination(home);
    let mut made = std::fs::DirBuilder::new();
    made.recursive(true).mode(0o700).create(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY;
    let dir = rustix::fs::open(&path, flags, rustix::fs::Mode::empty()).unwrap();
    Root::opened(dir, path, (late, shown)).expect("an owner-only root")
}

/// The hook of every root a test opens: what [`LATE`] arms, at the step of
/// the next seal on this thread it names, and then nothing.
fn late(step: Step, dir: &OwnedFd, staged: &str) -> std::io::Result<()> {
    match (step, LATE.take()) {
        (Step::Staged, Some(Late::Sleep(spent)))
        | (Step::Committed, Some(Late::SleepCommitted(spent)))
        | (Step::Retiring, Some(Late::SleepRetiring(spent))) => std::thread::sleep(spent),
        (Step::Retiring, Some(Late::FailRetiring)) => {
            return Err(std::io::Error::other("the prior name could not be removed"))
        }
        (Step::Staged, Some(Late::Unstage)) => {
            rustix::fs::unlinkat(dir, staged, AtFlags::empty()).unwrap()
        }
        (Step::Staged, Some(Late::Occupy)) => occupied(dir, staged),
        (Step::Committed, Some(Late::DenyCommitted(spent))) => {
            denied(dir);
            std::thread::sleep(spent)
        }
        (Step::Committed, Some(Late::FailUndone(spent))) => {
            LATE.set(Some(Late::Undoing));
            std::thread::sleep(spent)
        }
        (Step::Committed, Some(Late::FailCommitted))
        | (Step::Undone, Some(Late::Undoing))
        | (Step::Final, Some(Late::FailFinal)) => {
            return Err(std::io::Error::other("the directory sync failed"))
        }
        (_, armed) => LATE.set(armed),
    }
    Ok(())
}

/// An empty file of this user's own made at the prior name of the record
/// `staged` stages in `dir`: a link of the record that stood to that name
/// fails, and the directory stays writable.
fn occupied(dir: &OwnedFd, staged: &str) {
    let name = staged.strip_suffix(STAGED).unwrap();
    let flags = rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE | rustix::fs::OFlags::EXCL;
    let mode = rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR;
    drop(rustix::fs::openat(dir, format!("{name}{PRIOR}").as_str(), flags, mode).unwrap());
}

/// The directory `dir` left readable and searchable, but not writable.
fn denied(dir: &OwnedFd) {
    let kept = rustix::fs::Mode::RUSR | rustix::fs::Mode::XUSR;
    rustix::fs::fchmod(dir, kept).unwrap()
}

/// `held`'s positive settlement fact sealed, and then its lease released,
/// as its engine settles a proven attempt.
fn freed(held: &Held) {
    assert_eq!(proved(held, soon()), Ok(Commit::Synced));
    assert_eq!(release(held, soon()), Ok(Commit::Synced));
}

/// TMPDIR, canonical: where every fixture of these suites is made.
fn base() -> PathBuf {
    std::fs::canonicalize(std::env::temp_dir()).unwrap()
}

/// Whether `base` and every ancestor of it protect what lies below them
/// by the writer's own rule (`owned`), as D6 asks of a HOME's.
fn protects(base: &Path) -> bool {
    let user = brokkr_protocol::hands::ids().0;
    let judged = |at: &Path| rustix::fs::stat(at).is_ok_and(|stat| owned(&stat, user, false));
    base.ancestors().all(judged)
}

/// A fresh owner-only directory in `base`, canonical, holding an
/// owner-only `home`, whatever the umask made them.
fn fresh(base: &Path) -> tempfile::TempDir {
    made_in(base).unwrap()
}

/// [`fresh`], or none where nothing can be made in `base`.
fn made_in(base: &Path) -> Option<tempfile::TempDir> {
    let mut made = tempfile::Builder::new();
    let made = made.prefix("brokkr-admission-").tempdir_in(base).ok()?;
    std::fs::set_permissions(made.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let home = made.path().join("home");
    std::fs::DirBuilder::new().mode(0o700).create(home).unwrap();
    Some(made)
}

/// A HOME stood in for this thread's admissions until dropped, on return
/// or unwind.
pub(in crate::engine) struct Coordinated;

impl Coordinated {
    pub(in crate::engine) fn at(home: &Path) -> Coordinated {
        STAND.set(Stand::At(home.to_path_buf()));
        Coordinated
    }

    pub(in crate::engine) fn unrooted(home: &Path) -> Coordinated {
        STAND.set(Stand::Unrooted(home.to_path_buf()));
        Coordinated
    }

    fn host() -> Coordinated {
        STAND.set(Stand::Host);
        Coordinated
    }
}

impl Drop for Coordinated {
    fn drop(&mut self) {
        STAND.set(Stand::Own);
    }
}

/// A HOME of a test's own, its coordination root opened, and the worktrees
/// it makes beside it.
struct Host {
    dir: tempfile::TempDir,
    home: PathBuf,
    root: Arc<Root>,
}

impl Host {
    /// A fresh one in TMPDIR, whatever TMPDIR's ancestors are: its root is
    /// handed to admission opened, as production's resolution would.
    fn new() -> Host {
        Host::in_dir(fresh(&base()))
    }

    /// The one made in `dir`.
    fn in_dir(dir: tempfile::TempDir) -> Host {
        let home = dir.path().join("home");
        let root = Arc::new(opened(&home));
        Host { dir, home, root }
    }

    /// A fresh one where every ancestor protects a HOME, for the tests of
    /// production's resolution from HOME itself: TMPDIR, else `/tmp`, else
    /// `/var/tmp`. None where no such location exists or none can be made
    /// in, the test then skipped with its reason (or failed where this host
    /// must produce the evidence).
    fn resolving() -> Option<Host> {
        let candidates = [base(), PathBuf::from("/tmp"), PathBuf::from("/var/tmp")];
        let canonical = candidates
            .iter()
            .filter_map(|at| std::fs::canonicalize(at).ok());
        let made = canonical
            .filter(|at| protects(at))
            .find_map(|at| made_in(&at));
        if made.is_none() {
            let required = brokkr_protocol::hands::boundary_evidence_required();
            let reason = "no protected location for a HOME (TMPDIR, /tmp, /var/tmp; D6)";
            brokkr_protocol::hands::skip_boundary_proof(required, reason);
        }
        made.map(Host::in_dir)
    }

    /// The worktree `name` beside the HOME, made.
    fn worktree(&self, name: &str) -> PathBuf {
        let worktree = self.dir.path().join(name);
        std::fs::create_dir_all(&worktree).unwrap();
        worktree
    }

    /// The protected root's path.
    fn root(&self) -> PathBuf {
        coordination(&self.home)
    }

    /// Where the run `run` of the worktree `worktree` keeps its
    /// reservation.
    fn reservation(&self, worktree: &Path, run: &str) -> PathBuf {
        let repo = canonical::sha256_bytes(worktree.as_os_str().as_bytes());
        self.root().join(repo).join(run).join(RESERVATION)
    }

    /// The reservation `run` of `worktree` holds: its standing attempts,
    /// and its settled ones.
    fn attempts(&self, worktree: &Path, run: &str) -> (Vec<String>, Vec<String>) {
        let bytes = std::fs::read(self.reservation(worktree, run)).unwrap();
        let held: Reservation = serde_json::from_slice(&bytes).unwrap();
        (
            held.attempts.into_keys().collect(),
            held.settled.into_iter().collect(),
        )
    }

    /// `claim` admitted in this HOME's root within five seconds.
    fn admit(&self, claim: &Claim) -> Result<Held, Unadmitted> {
        admit(&self.root, claim, soon())
    }
}

/// The lock at `lock` taken exclusively, within five seconds: a process
/// another test forks may carry its open description until it execs.
fn taken(lock: &Path) -> std::fs::File {
    let file = std::fs::File::open(lock).unwrap();
    let deadline = soon();
    while flock(file.as_fd(), FlockOperation::NonBlockingLockExclusive).is_err() {
        assert!(Instant::now() < deadline, "the lock was never free");
        std::thread::sleep(Duration::from_millis(10));
    }
    file
}

/// How long the seal-in-flight test holds the lock after the door's call,
/// so a door that reads without the lock (the binding mutation) shows
/// itself by answering. It bounds only that: the fixed door waits on the
/// lock to its own startup deadline, and never on this (#835).
const HOLD: Duration = Duration::from_millis(500);

/// Five seconds from now.
pub(in crate::engine) fn soon() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

/// A run of `worktree` that protects its evidence and starts no writer,
/// its attempt standing in `home`'s coordination root, as its reservation
/// shows it to every later admission: its lease.
pub(in crate::engine) fn protecting(home: &Path, worktree: &Path) -> Held {
    let root = Arc::new(opened(home));
    let claim = claim(worktree, ["run-p", "p1"], Protects::Evidence, &[]);
    admit(&root, &claim, soon()).unwrap()
}

/// `engine` holding on `host` the lease of its run's attempt `attempt` on
/// its own worktree, protecting `protects`, the reservation naming the
/// engine's own journal: the worktree, and the run's directory in the
/// host's root, where the attempt's evidence stands.
fn lease(host: &Host, engine: &mut Engine, attempt: &str, protects: Protects) -> [PathBuf; 2] {
    let (work, run) = (worktree(engine), engine.run_id.clone());
    let mut leased = claim(&work, [&run, attempt], protects, &[]);
    leased.journal = std::fs::canonicalize(engine.store.path()).unwrap();
    engine.held = Some(host.admit(&leased).unwrap());
    let at = host.root().join(&leased.repo).join(run);
    [work, at]
}

/// [`lease`] on a fresh host protecting the attempt's evidence (U8d): the
/// host's directory, and the run's directory in its root.
pub(in crate::engine) fn evidence_leased(
    engine: &mut Engine,
    attempt: &str,
) -> (tempfile::TempDir, PathBuf) {
    fresh_lease(engine, attempt, Protects::Evidence)
}

/// [`lease`] on a fresh host protecting nothing, so no evidence is at stake
/// (U8e3): the host's directory, and the run's directory in its root.
pub(in crate::engine) fn unstaked_lease(
    engine: &mut Engine,
    attempt: &str,
) -> (tempfile::TempDir, PathBuf) {
    fresh_lease(engine, attempt, Protects::Nothing)
}

/// [`lease`] on a fresh host, protecting `protects`.
fn fresh_lease(
    engine: &mut Engine,
    attempt: &str,
    protects: Protects,
) -> (tempfile::TempDir, PathBuf) {
    let host = Host::new();
    let [_, at] = lease(&host, engine, attempt, protects);
    (host.dir, at)
}

/// The attempt `attempt` of the run `run` on `worktree`, protecting
/// `protects` with `writers`; its artifact root is the worktree's
/// `.forge/artifacts`, made where it protects it, and its journal the
/// worktree's `forge.db`.
fn claim(
    worktree: &Path,
    [run, attempt]: [&str; 2],
    protects: Protects,
    writers: &[Writer],
) -> Claim {
    let artifacts = worktree.join(".forge/artifacts");
    if protects == Protects::Evidence {
        std::fs::create_dir_all(&artifacts).unwrap();
    }
    Claim {
        repo: canonical::sha256_bytes(worktree.as_os_str().as_bytes()),
        run: run.to_string(),
        attempt: attempt.to_string(),
        workdir: worktree.to_path_buf(),
        artifacts,
        journal: worktree.join("forge.db"),
        reserved: Reserved {
            protects,
            writers: writers.to_vec(),
            cleanup: Witnessed::Settling,
            serves: Vec::new(),
            drivers: Vec::new(),
        },
        authored: Authored::default(),
    }
}

/// [`claim`] of a run on `worktree` that protects nothing, its one `work`
/// site outside any box.
fn open(worktree: &Path, ids: [&str; 2]) -> Claim {
    claim(worktree, ids, Protects::Nothing, &[unbounded("work")])
}

/// [`claim`] of a run on `worktree` that protects its evidence, its one
/// `work` site boxed to that worktree.
fn guarded(worktree: &Path, ids: [&str; 2]) -> Claim {
    let work = [boxed("work", &[worktree])];
    claim(worktree, ids, Protects::Evidence, &work)
}

/// The site `site`'s writer, boxed to the writable `roots`, binding no
/// read-only source.
fn boxed(site: &str, roots: &[&Path]) -> Writer {
    boxed_binding(site, roots, &[])
}

/// The site `site`'s writer, boxed to the writable `roots` and binding the
/// read-only sources `readable`, each source as authored what it names.
fn boxed_binding(site: &str, roots: &[&Path], readable: &[&Path]) -> Writer {
    let paths = |ps: &[&Path]| ps.iter().map(|p| p.to_path_buf()).collect::<Vec<_>>();
    Writer {
        site: site.to_string(),
        reach: Reach::Boxed {
            writable: paths(roots),
            readable: paths(readable),
            sources: [paths(roots), paths(readable)].concat(),
            sealed: Box::default(),
        },
    }
}

impl Engine {
    /// The reach `spec`'s boxes are built from, sealed under `home` as
    /// admission seals it ([`boxed_reach`]) and held for the attempt in
    /// flight (U8a2c): what a fixture that composes a box without
    /// admitting its attempt stands in.
    pub(in crate::engine) fn sealed_for(&mut self, home: &Path, spec: &HandsSpec) {
        let workdir = std::fs::canonicalize(self.workdir()).unwrap();
        let store = self.kept_store().clone();
        let git = hands::git_facts(&workdir);
        let reach = boxed_reach(spec, home, &workdir, &git, store.sealed());
        self.reach.boxes.push((spec.clone(), reach));
    }

    /// A reach for `spec`'s boxes whose seal names no store, explicitly
    /// null, as doctor's and the probe's own seals do and no admission
    /// seals a boxed writer's: what U8m's refusal stands against.
    pub(in crate::engine) fn sealed_storeless(&mut self, home: &Path, spec: &HandsSpec) {
        let workdir = std::fs::canonicalize(self.workdir()).unwrap();
        let sealed = hands::seal(spec, home, &workdir, &hands::git_facts(&workdir), None);
        let (writable, readable, sources) = (vec![workdir.clone()], Vec::new(), vec![workdir]);
        let sealed = Box::new(sealed);
        let reach = Reach::Boxed {
            writable,
            readable,
            sources,
            sealed,
        };
        self.reach.boxes.push((spec.clone(), reach));
    }
}

/// The site `site`'s writer outside any box.
fn unbounded(site: &str) -> Writer {
    Writer {
        site: site.to_string(),
        reach: Reach::Unbounded,
    }
}

/// CR2's refusal of `conflict`, as a variant.
fn cr2(conflict: Conflict) -> Result<Held, Unadmitted> {
    Err(Unadmitted::Evidence(conflict))
}

/// MB3's refusal of `unknown`, as a variant.
fn mb3(unknown: Unknown) -> Result<Held, Unadmitted> {
    Err(Unadmitted::Identity(unknown))
}

/// CR2's refusal of the site `site` of the run `run` reaching `root`.
fn reaches(site: &str, run: &str, root: &Path) -> Conflict {
    Conflict::Reaches {
        site: site.into(),
        run: run.into(),
        root: root.to_path_buf(),
    }
}

/// MB3's refusal of the site `site` of the run `run` able to write
/// `served`.
fn exposing(site: &str, run: &str, served: &Path) -> Unknown {
    Unknown::Serving {
        site: site.into(),
        run: run.into(),
        served: served.to_path_buf(),
    }
}

/// [`exposing`], as admission refuses with it.
fn exposed(site: &str, run: &str, served: &Path) -> Result<Held, Unadmitted> {
    mb3(exposing(site, run, served))
}

/// The reservation of `run` on `worktree` as it stands, its attempt `attempt`'s
/// writers rewritten to `writers`, in the root's own shape and ownership: what
/// an earlier build sealed, or a writer reaching the root could have left.
#[cfg(target_os = "linux")]
fn planted_writers(host: &Host, worktree: &Path, [run, attempt]: [&str; 2], writers: Vec<Writer>) {
    let at = host.reservation(worktree, run);
    let bytes = std::fs::read(&at).unwrap();
    let mut whole: Reservation = serde_json::from_slice(&bytes).unwrap();
    whole.attempts.get_mut(attempt).unwrap().writers = writers;
    std::fs::write(&at, serde_json::to_vec(&whole).unwrap()).unwrap();
}

/// CR2's refusal of the site `site` of the run `run` outside any box.
fn unboxed(site: &str, run: &str) -> Conflict {
    Conflict::Unboxed {
        site: site.into(),
        run: run.into(),
    }
}

/// Whether the test `test` of this module runs here, in an engine of its
/// own (#403, #811): a test whose engine launches a process, a driver, a
/// member or a site's command, otherwise shares the test binary's engine
/// as its subreaper, and an orphan another test leaves there reads as a
/// stray its attempt cannot be attributed. False in the parent, once the
/// re-executed test passed alone.
fn alone(test: &str) -> bool {
    in_its_own_engine(&format!("engine::broker::admission::tests::{test}"))
}

/// Every refusal reads as its cause and its detail: CR2's for evidence and
/// coordination, MB3's identity cause for participation, in the operator's
/// words.
#[test]
fn every_refusal_reads_as_its_cause() {
    let (cr2, mb3) = (
        "dispatch refused: capability evidence root cannot be protected from every managed writer",
        "dispatch refused: MCP server box filesystem identity is not protected",
    );
    let path = PathBuf::from("/r");
    let cases = [
        (Conflict::Held("a".into()).into(), format!("{cr2}: run 'a' holds this worktree, its writers not proven settled")),
        (unboxed("work", "a").into(), format!("{cr2}: site 'work' of run 'a' writes outside a workspace box, which cannot be kept from protected evidence or the coordination root")),
        (reaches("work:m", "a", &path).into(), format!("{cr2}: site 'work:m' of run 'a' can reach '/r', which evidence or coordination depends on")),
        (Conflict::Replaceable { site: "work".into(), run: "a".into(), bind: path.clone() }.into(), format!("{cr2}: site 'work' of run 'a' binds '/r', which a managed writable root could replace to redirect the box it is rebuilt into")),
        (Conflict::Shown(path.clone()).into(), format!("{cr2}: '/r' is visible at a second mount point, or its mount cannot be read")),
        (Conflict::Corrupt(path.clone()).into(), format!("{cr2}: the artifact '/r' is not one file of one link holding the bytes its name digests")),
        (Conflict::Replaced(path.clone()).into(), format!("{cr2}: the artifact root '/r' was replaced since its attempt was admitted")),
        (Conflict::Redirected(path.clone()).into(), format!("{cr2}: '/r', which a box of the attempt is built from, no longer resolves as when its attempt was admitted")),
        (Unknown::Root.into(), format!("{mb3}: the capability coordination root is not protected")),
        (Unknown::Worktree.into(), format!("{mb3}: the operated worktree does not resolve")),
        (Unknown::Lock.into(), format!("{mb3}: the admission lock is not this user's own single file")),
        (Unknown::Expired.into(), format!("{mb3}: admission did not complete before its startup deadline")),
        (Unknown::Bounds.into(), format!("{mb3}: the protected inventory exceeds its entry bound")),
        (Unknown::Participant(path).into(), format!("{mb3}: the protected inventory holds an unknown participant at '/r'")),
        (Unknown::Unsealed.into(), format!("{mb3}: the attempt's reservation could not be sealed")),
        (Unknown::Unsynced.into(), format!("{mb3}: the attempt's reservation was committed, but its directory was not proven synced")),
        (Unknown::Lost.into(), format!("{mb3}: the attempt's reservation no longer stands")),
        (Unknown::Unproven.into(), format!("{mb3}: the attempt's reservation holds no proof that its owned processes settled")),
        (Unknown::Serving { site: "work:m".into(), run: "a".into(), served: "/s".into() }.into(), format!("{mb3}: site 'work:m' of run 'a' can write '/s', which an MCP server's box or lease rests on")),
        (Unknown::Unconfined { site: "work".into(), run: "a".into() }.into(), format!("{mb3}: site 'work' of run 'a' writes outside a workspace box, its privilege not confined, beside an MCP server's sources")),
        (Unknown::Unmounted("/s".into()).into(), format!("{mb3}: the mount table does not prove where '/s' lies beside a managed writer")),
        (Unknown::Unrouted("s".into(), Unroutable::Relative).into(), format!("{mb3}: the route of 's', which an MCP server's box is built from, is not absolute, as a relative or empty PATH entry or a relative TMPDIR names it")),
        (Unknown::Unrouted("/s".into(), Unroutable::Unread).into(), format!("{mb3}: the route of '/s', which an MCP server's box is built from, meets a component that cannot be read")),
        (Unknown::Unrouted("/s".into(), Unroutable::Hops).into(), format!("{mb3}: the route of '/s', which an MCP server's box is built from, meets more than 40 symlinks")),
    ];
    for (refusal, text) in cases {
        let refusal: Unadmitted = refusal;
        assert_eq!(refusal.to_string(), text);
    }
}

/// An already-running writer refuses: a second run on a worktree whose
/// lease stands is refused by the run holding it, before anything of it is
/// sealed, until that run's release; the next run is then admitted, and
/// each run's reservation keeps its settled attempt's name.
#[test]
fn a_second_run_on_a_held_worktree_waits_for_the_first_to_settle() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    let boxed_in = [boxed("work", &[&work])];
    let second = claim(&work, ["run-b", "b1"], Protects::Nothing, &boxed_in);
    assert_eq!(host.admit(&second), cr2(Conflict::Held("run-a".into())));
    let sealed = host.reservation(&work, "run-b").exists();
    assert!(!sealed, "nothing of it sealed");
    freed(&held);
    freed(&host.admit(&second).unwrap());
    let settled = (Vec::new(), vec!["a1".to_string()]);
    assert_eq!(host.attempts(&work, "run-a"), settled);
    assert_eq!(host.attempts(&work, "run-b").1, ["b1"]);
}

/// Engine death or a free advisory lock does not release a lease: a
/// reservation whose engine never released it keeps refusing with the lock
/// held by no one, and stands byte for byte as it was sealed.
#[test]
fn a_lease_its_engine_never_released_keeps_refusing_with_the_lock_free() {
    let host = Host::new();
    let work = host.worktree("work");
    drop(host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let lease = host.reservation(&work, "run-a");
    let sealed = std::fs::read(&lease).unwrap();
    drop(taken(&host.root().join(LOCK)));
    let refused = host.admit(&open(&work, ["run-b", "b1"]));
    assert_eq!(refused, cr2(Conflict::Held("run-a".into())));
    assert_eq!(std::fs::read(&lease).unwrap(), sealed);
}

/// Separate worktrees that protect nothing keep their leases side by side,
/// whatever their writers reach, while the fence stands.
#[test]
fn separate_worktrees_hold_their_leases_side_by_side() {
    let host = Host::new();
    let (one, two) = (host.worktree("one"), host.worktree("two"));
    let first = host.admit(&open(&one, ["run-a", "a1"])).unwrap();
    let second = host.admit(&open(&two, ["run-b", "b1"])).unwrap();
    for (worktree, run) in [(&one, "run-a"), (&two, "run-b")] {
        let holds = host.reservation(worktree, run).exists();
        assert!(holds, "{run} holds its own");
    }
    [first, second].iter().for_each(freed);
}

/// Admission is atomic: an admission that starts while another holds its
/// comparison open waits on the lock and then sees the other's sealed
/// reservation, so of two simultaneous runs on one worktree exactly one is
/// admitted. A first admission that fails ends the wait at once.
#[test]
fn simultaneous_admissions_on_one_worktree_admit_exactly_one() {
    let host = Host::new();
    let work = host.worktree("work");
    let (first, second) = (open(&work, ["run-a", "a1"]), open(&work, ["run-b", "b1"]));
    let outcomes = std::thread::scope(|scope| {
        let root = &host.root;
        let (compared, told) = mpsc::channel();
        let holding = scope.spawn(move || {
            let between = || {
                compared.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(300));
            };
            // The sender goes with this thread, so a failure here ends
            // the wait below rather than holding it.
            admit_with(root, &first, soon(), &between)
        });
        let racing = told.recv().map(|()| admit(root, &second, soon()));
        (holding.join().unwrap().map(drop), racing)
    });
    assert_eq!(outcomes.0, Ok(()));
    assert_eq!(outcomes.1, Ok(cr2(Conflict::Held("run-a".into()))));
}

/// Participation nothing can vouch for refuses by MB3's identity cause
/// before anything is sealed (H6): a stray file in the protected root, in
/// a worktree's directory or in a run's, a link, a directory no worktree's
/// key names, an attempt's directory the reservation does not name, a
/// directory that cannot be entered, this run's own reservation with a
/// field no record has, and another run's naming a third. A named
/// attempt's directory and a record staged beside its reservation are
/// passed over, and so is a field no record has in another run's, which
/// is read through the view every build writes (decision 0090).
#[test]
fn an_unknown_participant_refuses_before_anything_is_sealed() {
    let host = Host::new();
    let (work, other) = (host.worktree("work"), host.worktree("other"));
    let (planted, incoming) = (open(&other, ["run-p", "p1"]), open(&work, ["run-a", "a1"]));
    let (planted_at, own_at) = (
        host.reservation(&other, "run-p"),
        host.reservation(&work, "run-a"),
    );
    let widen = |at: &Path| {
        let sealed = std::fs::read_to_string(at).unwrap();
        std::fs::write(at, sealed.replacen('{', r#"{"extra":1,"#, 1)).unwrap();
    };
    let (root, run_dir) = (host.root(), planted_at.parent().unwrap().to_path_buf());
    let repo_dir = run_dir.parent().unwrap().to_path_buf();
    let key = "f".repeat(64);
    let held = host.admit(&planted).unwrap();
    std::fs::create_dir(run_dir.join("p1")).unwrap();
    std::fs::write(run_dir.join("reservation.json.new"), "").unwrap();
    freed(&host.admit(&incoming).unwrap());
    drop(held);
    let private = |at: &Path, mode: u32| std::fs::DirBuilder::new().mode(mode).create(at).unwrap();
    type Plant<'a> = Box<dyn Fn() + 'a>;
    let cases: [(Plant<'_>, PathBuf); 9] = [
        (
            Box::new(|| std::fs::write(root.join("stray"), "").unwrap()),
            root.join("stray"),
        ),
        (
            Box::new(|| std::fs::write(repo_dir.join("stray"), "").unwrap()),
            repo_dir.join("stray"),
        ),
        (
            Box::new(|| std::fs::write(run_dir.join("stray"), "").unwrap()),
            run_dir.join("stray"),
        ),
        (
            Box::new(|| std::os::unix::fs::symlink(&root, root.join(&key)).unwrap()),
            root.join(&key),
        ),
        // Owner-only, so only its name is what refuses it.
        (
            Box::new(|| private(&root.join("not-a-key"), 0o700)),
            root.join("not-a-key"),
        ),
        (
            Box::new(|| private(&run_dir.join("p2"), 0o700)),
            run_dir.join("p2"),
        ),
        (
            Box::new(|| private(&root.join(&key), 0o000)),
            root.join(&key),
        ),
        (Box::new(|| widen(&own_at)), own_at.clone()),
        (
            Box::new(|| {
                let sealed = std::fs::read_to_string(&planted_at).unwrap();
                std::fs::write(&planted_at, sealed.replace("run-p", "run-q")).unwrap();
            }),
            planted_at.clone(),
        ),
    ];
    let (sealed, own) = (
        std::fs::read(&planted_at).unwrap(),
        std::fs::read(&own_at).unwrap(),
    );
    for (plant, at) in cases {
        plant();
        let stood = std::fs::read(&own_at).unwrap();
        assert_eq!(host.admit(&incoming), mb3(Unknown::Participant(at.clone())));
        assert_eq!(
            std::fs::read(&own_at).unwrap(),
            stood,
            "{at:?}: nothing sealed"
        );
        if std::fs::symlink_metadata(&at).is_ok_and(|found| found.is_dir()) {
            std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o700)).unwrap();
            std::fs::remove_dir(&at).unwrap();
        } else if (at != planted_at) & (at != own_at) {
            std::fs::remove_file(&at).unwrap();
        }
        for (at, bytes) in [(&planted_at, &sealed), (&own_at, &own)] {
            std::fs::write(at, bytes).unwrap();
            std::fs::set_permissions(at, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    widen(&planted_at);
    let widened = std::fs::read(&planted_at).unwrap();
    freed(&host.admit(&incoming).unwrap());
    assert_eq!(std::fs::read(&planted_at).unwrap(), widened, "never sealed");
}

/// A name that is not UTF-8 is unknown too. Linux only: APFS refuses such
/// a name before anything under test runs.
#[cfg(target_os = "linux")]
#[test]
fn a_name_that_is_not_utf8_is_an_unknown_participant() {
    use std::os::unix::ffi::OsStrExt;
    let host = Host::new();
    let work = host.worktree("work");
    freed(&host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    std::fs::write(host.root().join(std::ffi::OsStr::from_bytes(b"\xff")), "").unwrap();
    let refused = host.admit(&open(&work, ["run-a", "a2"]));
    assert_eq!(refused, mb3(Unknown::Participant(host.root())));
}

/// A record is read only once renamed in: admission returns, and so a
/// writer starts, only after its reservation was renamed in and synced,
/// nothing staged left; a reservation staged by an admission cut short
/// before its rename admitted nothing, and is passed over.
#[test]
fn a_writer_starts_only_after_its_reservation_is_renamed_in() {
    let host = Host::new();
    let work = host.worktree("work");
    let lease = host.reservation(&work, "run-a");
    let staged = lease.with_file_name("reservation.json.new");
    let seen = RefCell::new(None);
    let between = || *seen.borrow_mut() = Some((lease.exists(), staged.exists()));
    let held = admit_with(&host.root, &open(&work, ["run-a", "a1"]), soon(), &between).unwrap();
    assert_eq!(
        *seen.borrow(),
        Some((false, false)),
        "nothing sealed while compared"
    );
    assert!(lease.exists() && !staged.exists(), "renamed in on return");
    freed(&held);
    std::fs::rename(&lease, &staged).unwrap();
    let other = host.worktree("other");
    let admitted = host.admit(&open(&other, ["run-b", "b1"])).map(drop);
    assert_eq!(admitted, Ok(()), "a staged record admitted nothing");
}

/// The lock is the user's own single file, held only within the startup
/// deadline: a second link of it refuses, as does a lock held past the
/// deadline, and nothing of either attempt is sealed.
#[test]
fn an_unprotected_lock_refuses() {
    let host = Host::new();
    let work = host.worktree("work");
    let incoming = open(&work, ["run-a", "a1"]);
    freed(&host.admit(&incoming).unwrap());
    let incoming = open(&work, ["run-a", "a2"]);
    let lock = host.root().join(LOCK);
    std::fs::hard_link(&lock, host.dir.path().join("second-link")).unwrap();
    assert_eq!(host.admit(&incoming), mb3(Unknown::Lock));
    std::fs::remove_file(host.dir.path().join("second-link")).unwrap();
    let holder = taken(&lock);
    let shortly = Instant::now() + Duration::from_millis(50);
    assert_eq!(admit(&host.root, &incoming, shortly), mb3(Unknown::Expired));
    drop(holder);
    assert_eq!(host.attempts(&work, "run-a"), (vec![], vec!["a1".into()]));
}

/// Production's resolution of the coordination root from HOME (R5): the
/// root, made where it is missing, resolves only where HOME's every
/// ancestor and every directory on the way protects what lies below it,
/// and the root itself is this user's alone; a root another user's group
/// may write, or a HOME under a group-writable parent, resolves none.
#[test]
fn the_root_resolves_from_home_only_where_every_ancestor_protects() {
    let Some(host) = Host::resolving() else {
        return;
    };
    let resolved = Root::of(&host.home).map(|root| root.path().to_path_buf());
    assert_eq!(resolved, Some(host.root()));
    std::fs::set_permissions(host.root(), std::fs::Permissions::from_mode(0o770)).unwrap();
    assert_eq!(Root::of(&host.home), None, "a root its group may write");
    let group = host.dir.path().join("group");
    std::fs::DirBuilder::new()
        .mode(0o775)
        .create(&group)
        .unwrap();
    std::fs::set_permissions(&group, std::fs::Permissions::from_mode(0o775)).unwrap();
    let home = group.join("home");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&home)
        .unwrap();
    assert_eq!(
        Root::of(&home),
        None,
        "a HOME under a group-writable parent"
    );
    assert!(
        !coordination(&home).exists(),
        "nothing made past the parent"
    );
}

/// Admission and provisioning share one startup deadline (M8): an
/// observation that blocks past it, under the lock, ends in MB3's identity
/// cause with nothing sealed.
#[test]
fn an_observation_blocked_past_the_deadline_seals_nothing() {
    let host = Host::new();
    let work = host.worktree("work");
    let deadline = Instant::now() + Duration::from_millis(200);
    let blocked = || std::thread::sleep(Duration::from_millis(300));
    let refused = admit_with(
        &host.root,
        &open(&work, ["run-a", "a1"]),
        deadline,
        &blocked,
    );
    assert_eq!(refused, mb3(Unknown::Expired));
    assert!(!host.reservation(&work, "run-a").exists());
}

/// The inventory reads within its entry bound: one entry more than the
/// bound is refused, the bound itself read whole, and an expired deadline
/// reads nothing.
#[test]
fn the_inventory_reads_within_its_entry_bound() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host
        .admit(&claim(&work, ["run-a", "a1"], Protects::Nothing, &[]))
        .unwrap();
    // The lock, the worktree's directory, its run's, and its reservation.
    let within = |entries, deadline| {
        let mut budget = Budget { entries, deadline };
        let own = ["", ""];
        let found =
            super::super::inventory::inventory(host.root.dir(), &host.root(), own, &mut budget);
        found.map(|found| found.standing.len())
    };
    assert_eq!(within(4, soon()), Ok(1));
    assert_eq!(within(3, soon()), Err(Unknown::Bounds));
    assert_eq!(within(4, Instant::now()), Err(Unknown::Expired));
    freed(&held);
}

/// Every directory production's resolution makes on the way to the
/// protected root and below it is synced in its parent before admission
/// returns, and so before anything spawns (M9). A test of the resolution
/// from HOME itself (R5).
#[test]
fn every_directory_admission_makes_is_synced_in_its_parent() {
    use std::os::unix::fs::MetadataExt;
    let Some(host) = Host::resolving() else {
        return;
    };
    std::fs::remove_dir_all(host.home.join(".local")).unwrap();
    let synced = RefCell::new(Vec::new());
    // Read through std's metadata, whose device is a u64 on every
    // platform: rustix's `st_dev` is an i32 on macOS.
    let witness = |dir: &std::os::fd::OwnedFd| {
        let found = std::fs::File::from(dir.try_clone().unwrap()).metadata();
        let found = found.unwrap();
        synced.borrow_mut().push((found.dev(), found.ino()));
        Ok(())
    };
    super::super::protected_with(&host.home, &["repo", "run"], &witness).unwrap();
    let parents = ["", ".local", ".local/state", ".local/state/brokkr"];
    let root = PROTECTED_ROOT.join("/");
    let parents = parents.iter().map(|at| host.home.join(at));
    let parents = parents.chain([host.home.join(&root), host.home.join(&root).join("repo")]);
    let identity = |at: PathBuf| {
        let found = std::fs::metadata(at).unwrap();
        (found.dev(), found.ino())
    };
    assert_eq!(*synced.borrow(), parents.map(identity).collect::<Vec<_>>());
    synced.borrow_mut().clear();
    super::super::protected_with(&host.home, &["repo", "run"], &witness).unwrap();
    assert!(synced.borrow().is_empty(), "nothing gained a name");
}

/// C1 (operator ruling 2026-10-18): a coordination root that cannot be
/// protected refuses only where the claim protects evidence; with nothing
/// at stake the dispatch proceeds holding no lease, the fence standing or
/// lifted, and nothing is sealed.
#[test]
fn an_unprotectable_root_proceeds_unreserved_only_where_nothing_is_at_stake() {
    if !alone("an_unprotectable_root_proceeds_unreserved_only_where_nothing_is_at_stake") {
        return;
    }
    let host = Host::new();
    let work = host.worktree("work");
    let plain = open(&work, ["run-a", "a1"]);
    let boxed_in = [boxed("work", &[&work])];
    let protecting = claim(&work, ["run-a", "a1"], Protects::Evidence, &boxed_in);
    assert_eq!(coordinated(None, &plain), Ok(None));
    assert_eq!(coordinated(None, &protecting), Err(Unknown::Root));
    // Dispatch on a HOME whose root could not be protected: the driver
    // runs, holding no lease, and no reservation is made.
    let _home = Coordinated::unrooted(&host.home);
    let touching = || {
        let (dir, mut engine) = engine(single_body(Vec::new()));
        let ran = dir.path().join("ran");
        let touch = driver(&format!("touch '{}'", ran.display()));
        engine.bundle.seats.get_mut("work").unwrap().body = single_body(touch);
        (dir, engine, ran)
    };
    let (_dir, mut unreserved, ran) = touching();
    let (event, payload) = executed(&mut unreserved);
    assert_eq!(event, EventType::EffectSucceeded, "{payload}");
    assert!(ran.exists(), "the driver ran unreserved");
    let reserved = host.reservation(&worktree(&unreserved), &unreserved.run_id);
    assert!(!reserved.exists(), "no reservation was taken");
    // Evidence at stake: the attempt is refused with MB3's identity cause
    // before its driver starts.
    let (_dir, mut beside, ran) = touching();
    let artifacts = crate::engine::boxed::artifact_root(&beside.run_local());
    std::fs::create_dir_all(artifacts.join("sha256")).unwrap();
    refused_with(&mut beside, Unknown::Root.into());
    assert!(!ran.exists(), "no driver started beside evidence");
    // Fence lifted: nothing at stake still proceeds unreserved (operator
    // ruling of 2026-10-29 on U9b1's fourth run).
    let _lift = crate::capabilities::McpFence::lift();
    assert_eq!(coordinated(None, &plain), Ok(None));
    let (_dir, mut lifted, ran) = touching();
    let (event, payload) = executed(&mut lifted);
    assert_eq!(event, EventType::EffectSucceeded, "{payload}");
    assert!(
        ran.exists(),
        "the driver ran unreserved with the fence lifted"
    );
}

/// The dispatch door admits only a lease that still stands: a reservation
/// removed, or an artifact root replaced since admission, refuses there.
#[test]
fn the_door_refuses_a_lease_that_no_longer_stands() {
    let host = Host::new();
    let work = host.worktree("work");
    // Off Linux no box is built and the mount table is never read, so a
    // protecting boxed admission refuses at the coordination root before
    // there is any lease to recheck (exclusion's `shown_once`, decision
    // 0063). On Linux it is admitted, and the door's recheck is proved.
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        host.admit(&guarded(&work, ["run-a", "a1"])),
        cr2(Conflict::Shown(host.root()))
    );
    #[cfg(target_os = "linux")]
    {
        let held = host.admit(&guarded(&work, ["run-a", "a1"])).unwrap();
        assert_eq!(recheck(&held), Ok(()));
        let artifacts = work.join(".forge/artifacts");
        std::fs::rename(&artifacts, work.join(".forge/moved")).unwrap();
        std::fs::create_dir(&artifacts).unwrap();
        let replaced = Err(Unadmitted::Evidence(Conflict::Replaced(artifacts.clone())));
        assert_eq!(recheck(&held), replaced);
        std::fs::remove_dir(&artifacts).unwrap();
        assert_eq!(recheck(&held), replaced, "a root that went");
        std::fs::write(&artifacts, "").unwrap();
        assert_eq!(recheck(&held), replaced, "a file in its place");
        std::fs::remove_file(host.reservation(&work, "run-a")).unwrap();
        assert_eq!(recheck(&held), Err(Unadmitted::Identity(Unknown::Lost)));
    }
}

/// R1 (door side): the dispatch door observes each boxed writer's sealed
/// sources again from its authored hands, as hands rebuilds its box, and
/// refuses a lease whose bind source now resolves elsewhere than when it
/// was sealed — another directory put at its path, or the source swapped
/// for a symlink, after admission — before anything spawns, under the
/// fence rule: where evidence was at stake when it was admitted, or once
/// the fence is lifted. An unchanged reach rechecks clean.
#[test]
fn the_door_refuses_a_lease_whose_reach_was_redirected() {
    let host = Host::new();
    let (work, src) = (host.worktree("work"), host.worktree("src"));
    let elsewhere = host.worktree("elsewhere");
    let bind = Bind {
        path: src.to_string_lossy().into_owned(),
        mode: BindMode::Ro,
        mask: Vec::new(),
    };
    let spec = HandsSpec {
        network: false,
        binds: vec![bind],
    };
    let store = host.home.join("secrets.env");
    let reach = boxed_reach(
        &spec,
        &host.home,
        &work,
        &hands::git_facts(&work),
        (&store, &store),
    );
    let writer = Writer {
        site: "work".into(),
        reach: reach.clone(),
    };
    let mut incoming = claim(&work, ["run-a", "a1"], Protects::Nothing, &[writer]);
    incoming.authored = Authored {
        boxes: vec![(spec, reach)],
    };
    // A disjoint, unreplaceable source is admitted with nothing at stake.
    let held = host.admit(&incoming).unwrap();
    let staked = Held {
        staked: true,
        ..held.clone()
    };
    assert_eq!(recheck(&staked), Ok(()));
    let redirected = Err(Unadmitted::Evidence(Conflict::Redirected(src.clone())));
    std::fs::rename(&src, host.dir.path().join("was")).unwrap();
    std::fs::create_dir(&src).unwrap();
    assert_eq!(
        recheck(&staked),
        redirected,
        "another directory at its path"
    );
    std::fs::remove_dir(&src).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &src).unwrap();
    assert_eq!(recheck(&held), Ok(()), "fence standing, nothing at stake");
    assert_eq!(recheck(&staked), redirected, "evidence at stake");
    let _lift = crate::capabilities::McpFence::lift();
    assert_eq!(recheck(&held), Ok(()), "fence lifted, nothing at stake");
    assert_eq!(
        recheck(&staked),
        redirected,
        "fence lifted, evidence at stake"
    );
}

/// #835: the door reads the reservation under the admission lock, so a
/// sibling's seal in flight is waited for and never read as a loss. The
/// seal in flight is modelled by what every seal holds, the lock, and what
/// a reader descheduled between its link and its unlink finds, the record
/// of two links and no prior, which `standing` refuses. Without the lock
/// the door answers that refusal at once; with it, nothing until the record
/// is whole and the lock released, and then admission.
#[test]
fn the_door_waits_for_a_seal_in_flight() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    let mid_seal = host.dir.path().join("mid-seal");
    std::fs::hard_link(host.reservation(&work, "run-a"), &mid_seal).unwrap();
    std::thread::scope(|scope| {
        let holder = taken(&host.root().join(LOCK));
        let (door, answered) = mpsc::channel();
        scope.spawn(move || {
            let _ = door.send(recheck(&held));
        });
        let early = answered.recv_timeout(HOLD);
        assert_eq!(
            early,
            Err(mpsc::RecvTimeoutError::Timeout),
            "read through the held lock"
        );
        std::fs::remove_file(&mid_seal).unwrap();
        drop(holder);
        assert_eq!(answered.recv().unwrap(), Ok(()));
    });
}

/// #835: the door's take of the lock is `locked`'s, so a lock file that is
/// not this user's single regular file refuses the dispatch by the lock's
/// own name, as it refuses admission, and nothing is read; with the second
/// link gone the same lease is admitted.
#[test]
fn the_door_refuses_a_lock_file_by_its_own_name() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    let second = host.dir.path().join("second-link");
    std::fs::hard_link(host.root().join(LOCK), &second).unwrap();
    assert_eq!(recheck(&held), Err(Unadmitted::Identity(Unknown::Lock)));
    std::fs::remove_file(&second).unwrap();
    assert_eq!(recheck(&held), Ok(()));
}

/// A boxed writer held with no sealed reach, as no box admission builds
/// is, is redirected at the door by the first path its box is built from
/// now, its worktree: what was never sealed never rechecks clean.
#[test]
fn a_box_held_with_no_sealed_reach_is_redirected_by_its_worktree() {
    let host = Host::new();
    let work = host.worktree("work");
    let incoming = claim(&work, ["run-a", "a1"], Protects::Nothing, &[]);
    let mut held = host.admit(&incoming).unwrap();
    let spec = HandsSpec {
        network: false,
        binds: Vec::new(),
    };
    held.authored = Authored {
        boxes: vec![(spec, Reach::Unbounded)],
    };
    let worktree = std::fs::canonicalize(&work).unwrap();
    assert_eq!(redirected(&held), Some(worktree));
}

/// A claim names its artifact root and journal as far as each resolves:
/// canonical where it stands, its parent's canonical path with its own
/// name where only the parent stands, and as spelled where neither does.
#[test]
fn a_claims_paths_resolve_as_far_as_they_stand() {
    let host = Host::new();
    let parent = host.worktree("parent");
    let canonical = std::fs::canonicalize(&parent).unwrap();
    assert_eq!(beside(&parent), canonical, "where it stands");
    assert_eq!(
        beside(&parent.join("unmade")),
        canonical.join("unmade"),
        "where only its parent stands"
    );
    let neither = parent.join("gone").join("unmade");
    assert_eq!(beside(&neither), neither, "where neither stands");
}

/// Two handles are one coordination root where they were opened at one
/// path, and two where at two.
#[test]
fn two_handles_are_one_root_only_where_opened_at_one_path() {
    let (one, two) = (Host::new(), Host::new());
    assert!(opened(&one.home) == opened(&one.home), "one path");
    assert!(opened(&one.home) != opened(&two.home), "two paths");
}

/// R2: a lease's reservation is sealed unresolved the first time a driver
/// of its attempt ends unresolved, and not again at a second such end; a
/// lease whose attempt its reservation no longer holds standing is sealed
/// again unchanged.
#[test]
fn an_unresolved_end_is_sealed_once_and_changes_no_attempt_not_standing() {
    use std::os::unix::fs::MetadataExt;
    let host = Host::new();
    let work = host.worktree("work");
    let incoming = claim(&work, ["run-a", "a1"], Protects::Nothing, &[]);
    let mut held = Some(host.admit(&incoming).unwrap());
    let unresolved = Cleanup::Unresolved {
        reason: Unsettled::Stdout,
    };
    let at = host.reservation(&work, "run-a");
    let sealed = |at: &Path| {
        let found = std::fs::metadata(at).unwrap();
        (std::fs::read(at).unwrap(), found.ino())
    };
    witnessed(&mut held, &unresolved);
    let first = sealed(&at);
    let read: Reservation = serde_json::from_slice(&first.0).unwrap();
    assert_eq!(read.attempts["a1"].cleanup, Witnessed::Unresolved);
    witnessed(&mut held, &unresolved);
    assert_eq!(sealed(&at), first, "sealed once");
    let mut gone = held.map(|held| Held {
        attempt: "a9".into(),
        unproven: false,
        ..held
    });
    witnessed(&mut gone, &unresolved);
    assert_eq!(sealed(&at).0, first.0, "no attempt not standing changed");
    assert_eq!(gone.map(|held| held.unproven), Some(true));
}

/// `git` run with `args` in `repo`, which must succeed.
fn git(repo: &Path, args: &[&str]) {
    let ran = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .status();
    assert!(ran.unwrap().success(), "git {args:?}");
}

/// H2 (operator ruling 2026-10-18, after U8a2a's third run): a linked
/// worktree's Git directory and its Git common directory outside it are
/// sealed with its writer's reach, the common directory a writable root
/// of its own as hands binds it, and the dispatch door asks Git for both
/// again before every spawn. Once admitted, the unchanged worktree
/// dispatches; once its `commondir` names another repository's Git
/// directory, the door refuses before anything spawns (evidence at stake,
/// so the rule applies). Holding a running driver's later calls to the
/// sealed reach is U8a2c's.
#[test]
fn the_door_refuses_a_worktree_whose_git_names_another_repository() {
    if !alone("the_door_refuses_a_worktree_whose_git_names_another_repository") {
        return;
    }
    let host = Host::new();
    let (main, other) = (host.worktree("main"), host.worktree("other"));
    crate::engine::tests::git_commit(&main, "base");
    crate::engine::tests::git_commit(&other, "other");
    let linked = host.dir.path().join("linked");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &linked.to_string_lossy(),
        ],
    );
    let (spec, common) = (hands(&[]), main.join(".git"));
    let store = host.home.join("secrets.env");
    let reach = boxed_reach(
        &spec,
        &host.home,
        &linked,
        &hands::git_facts(&linked),
        (&store, &store),
    );
    let Reach::Boxed {
        writable, sealed, ..
    } = &reach
    else {
        panic!("a boxed reach")
    };
    assert_eq!(writable, &[linked.clone(), common.clone()]);
    let git_dir = common.join("worktrees/linked");
    let named = hands::GitFacts {
        git_dir: Some(git_dir.clone()),
        common_dir: Some(common.clone()),
        identity: Vec::new(),
    };
    let held = hands::seal(&spec, &host.home, &linked, &named, Some((&store, &store)));
    assert_eq!(**sealed, held);
    let writer = Writer {
        site: "work".into(),
        reach: reach.clone(),
    };
    let mut incoming = claim(&linked, ["run-a", "a1"], Protects::Nothing, &[writer]);
    incoming.authored.boxes = vec![(spec, reach)];
    let held = Held {
        staked: true,
        ..host.admit(&incoming).unwrap()
    };
    let (dir, engine) = engine(single_body(Vec::new()));
    let door = |name: &str| {
        let ran = dir.path().join(name);
        let spawn = SiteSpawn::inherit(vec!["touch".into(), ran.to_string_lossy().into_owned()]);
        let deadline = Duration::from_secs(5);
        let spawned = spawn_site(
            &engine.bundle,
            &spawn,
            &json!({}),
            &linked,
            deadline,
            Some(at_work(&held)),
        );
        spawned.map(|(process, _)| drop(process))
    };
    assert_eq!(door("unchanged"), Ok(()));
    let named = format!("{}\n", other.join(".git").display());
    std::fs::write(git_dir.join("commondir"), named).unwrap();
    let refused = Unadmitted::Evidence(Conflict::Redirected(common)).to_string();
    assert_eq!(door("redirected"), Err(refused));
    assert!(!dir.path().join("redirected").exists(), "nothing spawned");
}

/// The common dispatch door rechecks the lease before it spawns: a lease
/// whose attempt its reservation no longer names standing — released, or
/// the reservation gone — refuses there and starts nothing, and one that
/// stands spawns.
#[test]
fn the_dispatch_door_spawns_only_under_a_lease_that_stands() {
    if !alone("the_dispatch_door_spawns_only_under_a_lease_that_stands") {
        return;
    }
    let host = Host::new();
    let (dir, engine) = engine(single_body(Vec::new()));
    let work = worktree(&engine);
    let attempt = |id: &str| claim(&work, ["run-a", id], Protects::Nothing, &[]);
    let ran = dir.path().join("ran");
    let spawn = SiteSpawn::inherit(vec!["touch".into(), ran.to_string_lossy().into_owned()]);
    let door = |held| {
        let deadline = Duration::from_secs(5);
        let held = Some(at_work(held));
        let spawned = spawn_site(&engine.bundle, &spawn, &json!({}), &work, deadline, held);
        spawned.map(|(process, _)| drop(process))
    };
    let lost = Err(Unadmitted::Identity(Unknown::Lost).to_string());
    let released = host.admit(&attempt("a0")).unwrap();
    freed(&released);
    assert_eq!(door(&released), lost, "released");
    let held = host.admit(&attempt("a1")).unwrap();
    std::fs::remove_file(host.reservation(&work, "run-a")).unwrap();
    assert_eq!(door(&held), lost, "the reservation gone");
    assert!(!ran.exists(), "nothing spawned");
    let held = host.admit(&attempt("a2")).unwrap();
    assert_eq!(door(&held), Ok(()));
}

/// Another run's attempt on `engine`'s worktree, its one `work` site
/// outside any box, admitted in the coordination root under `home`: the
/// lease, or the refusal a reservation standing there makes (U8e2).
pub(in crate::engine) fn conflicting(home: &Path, engine: &Engine) -> Result<Held, Unadmitted> {
    let incoming = open(&worktree(engine), ["another-run", "b1"]);
    admit(&Arc::new(opened(home)), &incoming, soon())
}

/// The door of the `work` site, under the lease `held`, of a driver whose
/// site was sealed no broker.
fn at_work(held: &Held) -> Door<'_> {
    Door::of(held, "work", None)
}

/// A run's later attempt joins its reservation beside an earlier one not
/// proven settled, and releasing the later one leaves the earlier standing.
#[test]
fn a_runs_later_attempt_joins_its_reservation_beside_an_unsettled_one() {
    let host = Host::new();
    let work = host.worktree("work");
    drop(host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    freed(&host.admit(&open(&work, ["run-a", "a2"])).unwrap());
    let held = (vec!["a1".to_string()], vec!["a2".to_string()]);
    assert_eq!(host.attempts(&work, "run-a"), held);
    let refused = host.admit(&open(&work, ["run-b", "b1"]));
    assert_eq!(refused, cr2(Conflict::Held("run-a".into())));
}

/// A driver's command that runs `before` and then succeeds with
/// `complete`, echoing the ids its start names.
pub(in crate::engine) fn driver(before: &str) -> Vec<String> {
    driven(before, "complete")
}

/// A driver's command that runs `before` and then succeeds with `result`,
/// echoing the ids its start names.
fn driven(before: &str, result: &str) -> Vec<String> {
    let result = json!({ "result": result });
    let answer = capturing_driver_command("effect", "attempt", Path::new("/dev/null"), result);
    vec!["sh".into(), "-c".into(), format!("{before}; {}", answer[2])]
}

/// Workspace hands, with a read-write bind of each of `binds`.
pub(in crate::engine) fn hands(binds: &[&Path]) -> HandsSpec {
    let bind = |path: &&Path| Bind {
        path: path.to_string_lossy().into_owned(),
        mode: BindMode::Rw,
        mask: Vec::new(),
    };
    HandsSpec {
        network: false,
        binds: binds.iter().map(bind).collect(),
    }
}

/// The engine's run driven to its effect's attempt `attempt` opened, as
/// dispatch opens it, so the journal shows it in flight: the effect's id.
pub(in crate::engine) fn in_flight(engine: &mut Engine, attempt: &str) -> String {
    engine.drive_once().unwrap();
    engine.drive_once().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let effect = events.last().unwrap().payload["effect_id"].clone();
    let started = serde_json::json!({"effect_id": effect, "attempt_id": attempt, "driver": "none"});
    let attempt = Some(attempt.to_string());
    engine
        .append(EventType::EffectStarted, started, attempt)
        .unwrap();
    effect.as_str().unwrap().to_string()
}

/// The last event the engine journaled.
pub(in crate::engine) fn last(engine: &Engine) -> (EventType, serde_json::Value) {
    let events = engine.store.load(&engine.run_id).unwrap();
    let last = events.last().unwrap();
    (last.event_type, last.payload.clone())
}

/// The engine's own worktree, canonical.
fn worktree(engine: &Engine) -> PathBuf {
    std::fs::canonicalize(engine.workdir()).unwrap()
}

/// A host of the test's own, stood in as HOME for this thread's admissions
/// until its stand-in drops, and an engine whose work seat the test writes:
/// the host, its stand-in, the engine's directory and the engine.
fn stood_in() -> (Host, Coordinated, tempfile::TempDir, Engine) {
    let host = Host::new();
    let home = Coordinated::at(&host.home);
    let (dir, engine) = engine(single_body(Vec::new()));
    (host, home, dir, engine)
}

/// Dispatch holds the lease from before its driver starts until the
/// attempt's terminal event, and releases it once its driver is proven
/// settled: the driver saw its run's reservation, and once the attempt
/// succeeded the reservation names it settled.
#[test]
fn dispatch_holds_the_lease_while_its_driver_runs_and_releases_it_once_settled() {
    if !alone("dispatch_holds_the_lease_while_its_driver_runs_and_releases_it_once_settled") {
        return;
    }
    let (host, _home, dir, mut engine) = stood_in();
    let reservation = host.reservation(&worktree(&engine), &engine.run_id);
    let saw = dir.path().join("saw");
    let probe = format!(
        "test -f '{}' && touch '{}'",
        reservation.display(),
        saw.display()
    );
    engine.bundle.seats.get_mut("work").unwrap().body = single_body(driver(&probe));
    set_site_hands(&mut engine.bundle, "work", hands(&[]));
    let effect = requested(&engine, "effect");
    execute_work(&mut engine, &[effect]).unwrap();
    let (event, payload) = last(&engine);
    assert_eq!(event, EventType::EffectSucceeded, "{payload}");
    assert!(saw.exists(), "the driver ran under its run's reservation");
    let (standing, settled) = host.attempts(&worktree(&engine), &engine.run_id);
    assert_eq!((standing.len(), settled.len()), (0, 1));
}

/// A dispatch whose worktree another run holds is refused by CR2's cause
/// before any driver starts, and the attempt fails with it.
#[test]
fn a_dispatch_on_a_held_worktree_fails_before_its_driver_starts() {
    if !alone("a_dispatch_on_a_held_worktree_fails_before_its_driver_starts") {
        return;
    }
    let (host, _home, dir, mut engine) = stood_in();
    let ran = dir.path().join("ran");
    let touch = format!("touch '{}'", ran.display());
    engine.bundle.seats.get_mut("work").unwrap().body = single_body(driver(&touch));
    let standing = claim(&worktree(&engine), ["run-s", "s1"], Protects::Nothing, &[]);
    let _held = host.admit(&standing).unwrap();
    refused_with(&mut engine, Conflict::Held("run-s".into()).into());
    assert!(!ran.exists(), "no driver started");
}

/// The work seat's effect requested and executed as dispatch executes it:
/// the last event the engine journaled.
fn executed(engine: &mut Engine) -> (EventType, serde_json::Value) {
    let effect = requested(engine, "effect");
    execute_work(engine, &[effect]).unwrap();
    last(engine)
}

/// [`executed`], its attempt failed by `refusal`.
fn refused_with(engine: &mut Engine, refusal: Unadmitted) {
    let (event, payload) = executed(engine);
    assert_eq!(event, EventType::EffectFailed, "{payload}");
    assert_eq!(payload["error"], refusal.to_string());
}

/// The production seam reads the host's HOME as provisioning reads it: a
/// dispatch whose artifact root is no directory is refused by U8a's cause
/// as its claim is made, before that HOME is touched.
#[test]
fn the_hosts_home_is_read_only_where_nothing_stood_in() {
    if !alone("the_hosts_home_is_read_only_where_nothing_stood_in") {
        return;
    }
    let _home = Coordinated::host();
    let (_dir, mut engine) = engine(single_body(driver(":")));
    let artifacts = crate::engine::boxed::artifact_root(&engine.run_local());
    std::fs::create_dir_all(artifacts.parent().unwrap()).unwrap();
    std::fs::write(&artifacts, "").unwrap();
    refused_with(&mut engine, Unrooted::Foreign(artifacts).into());
}

/// An engine whose work seat is a sequence, a boxed draft step and then a
/// finishing panel whose member `m1` binds `bound` read-write, every
/// step's driver touching the file it returns beside the engine's
/// directory, which no step's driver must start.
fn sequenced(bound: &Path) -> (tempfile::TempDir, Engine, PathBuf) {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let ran = dir.path().join("ran");
    let touch = driver(&format!("touch '{}'", ran.display()));
    let members = vec![member("m1", touch.clone())];
    let steps: Vec<SequenceStep> = draft_then_finish("role.md", touch, &["drafted"], members);
    engine.bundle.seats.get_mut("work").unwrap().body = SeatBody::Sequence { steps };
    set_site_hands(&mut engine.bundle, "work:draft", hands(&[]));
    set_site_hands(&mut engine.bundle, "work:finish:m1", hands(&[bound]));
    (dir, engine, ran)
}

/// The union of a sequence's writer reach is reserved before its first
/// step: a panel member of its last step whose bind reaches another run's
/// protected evidence refuses the whole attempt before the first step's
/// driver starts. The run holding that evidence starts no writer of its
/// own, so it stands on every host: no mount table is read to admit it.
#[test]
fn a_sequence_reserves_every_steps_reach_before_its_first() {
    if !alone("a_sequence_reserves_every_steps_reach_before_its_first") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let kept = host.worktree("kept");
    let protecting = claim(&kept, ["run-s", "s1"], Protects::Evidence, &[]);
    let _held = host.admit(&protecting).unwrap();
    let (_dir, mut engine, ran) = sequenced(&kept.join(".forge"));
    let effect = requested(&engine, "effect");
    execute_work(&mut engine, &[effect]).unwrap();
    let artifacts = kept.join(".forge/artifacts");
    let refused = reaches("work:finish:m1", &engine.run_id, &artifacts);
    let (event, payload) = last(&engine);
    assert_eq!(event, EventType::EffectFailed);
    assert_eq!(payload["error"], Unadmitted::Evidence(refused).to_string());
    assert!(!ran.exists(), "no step's driver started");
}

/// The sequence of [`every_steps_door_rechecks_the_lease_its_attempt_holds`]:
/// a first step whose driver removes the run's reservation, then a single
/// or a panel step that would touch `ran`.
fn removing(engine: &mut Engine, reservation: &Path, ran: &Path, panel: bool) {
    let remove = driven(&format!("rm -f '{}'", reservation.display()), "drafted");
    let touch = driver(&format!("touch '{}'", ran.display()));
    let members = vec![member("m1", touch.clone())];
    let mut steps = draft_then_finish("role.md", remove, &["drafted"], members);
    if !panel {
        steps[1].body = crate::bundle::StepBody::Invocation(crate::bundle::Invocation::Single {
            role_path: "role.md".into(),
            command: touch,
            candidates: Vec::new(),
            written: Vec::new(),
            class: (),
        });
    }
    engine.bundle.seats.get_mut("work").unwrap().body = SeatBody::Sequence { steps };
}

/// Every step's door, a single's and a panel member's, rechecks the lease
/// its attempt was admitted under: while the fence stands and nothing is
/// at stake, a first step outside any box may run, and once its driver
/// removed the run's reservation, the next step's driver does not start.
/// With the fence lifted the same holds: nothing at stake, the driver
/// outside a box runs (operator ruling of 2026-10-29 on U9b1's fourth run).
#[test]
fn every_steps_door_rechecks_the_lease_its_attempt_holds() {
    if !alone("every_steps_door_rechecks_the_lease_its_attempt_holds") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    // Standing, a single and a panel member; lifted, a single.
    for (lifted, panel) in [(false, false), (false, true), (true, false)] {
        let _lift = lifted.then(crate::capabilities::McpFence::lift);
        let (dir, mut engine) = engine(single_body(Vec::new()));
        let reservation = host.reservation(&worktree(&engine), &engine.run_id);
        let ran = dir.path().join("ran");
        removing(&mut engine, &reservation, &ran, panel);
        let effect = requested(&engine, "effect");
        execute_work(&mut engine, &[effect]).unwrap();
        let lost = Unadmitted::Identity(Unknown::Lost).to_string();
        let (event, payload) = last(&engine);
        assert_eq!(event, EventType::EffectFailed, "{payload}");
        let refused = match panel {
            false => format!("sequence step 'finish': driver did not spawn: {lost}"),
            true => format!(
                "sequence step 'finish': panel members failed — m1: member driver did not spawn: \
                 {lost}"
            ),
        };
        assert_eq!(payload["error"], refused, "lifted {lifted}");
        assert!(!reservation.exists(), "the first step ran and removed it");
        assert!(!ran.exists(), "panel {panel}: the next step never started");
    }
}

/// The panel `keeper` and `sibling`, each driver running `before` first.
fn panel(keeper: &str, sibling: &str) -> SeatBody {
    SeatBody::Invocation(crate::bundle::Invocation::Panel {
        members: vec![
            member("keeper", driver(keeper)),
            member("sibling", driver(sibling)),
        ],
        aggregate: crate::bundle::Aggregate::UnanimousPass,
    })
}

/// Existing evidence alone requires protection, a later run with no
/// grants included: a panel whose zero-grant member would overwrite it
/// from outside a box is refused before any member starts, the evidence's
/// bytes unchanged; with hands throughout, the panel runs on Linux, and
/// off it, where no mount table proves its evidence shown once (decision
/// 0063), is refused at the coordination root.
#[test]
fn historical_evidence_refuses_a_panel_member_that_cannot_be_kept_from_it() {
    if !alone("historical_evidence_refuses_a_panel_member_that_cannot_be_kept_from_it") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let artifacts = crate::engine::boxed::artifact_root(&engine.run_local());
    let digest = artifacts
        .join("sha256")
        .join(canonical::sha256_bytes(b"retained"));
    std::fs::create_dir_all(digest.parent().unwrap()).unwrap();
    std::fs::write(&digest, "retained").unwrap();
    let ran = dir.path().join("ran");
    let touch = format!("touch '{}'", ran.display());
    let forge = format!("printf forged > '{}'", digest.display());
    engine.bundle.seats.get_mut("work").unwrap().body = panel(&touch, &forge);
    set_site_hands(&mut engine.bundle, "work:keeper", hands(&[]));
    let effect = requested(&engine, "effect");
    execute_work(&mut engine, std::slice::from_ref(&effect)).unwrap();
    let refused = unboxed("work:sibling", &engine.run_id);
    assert_eq!(
        last(&engine).1["error"],
        Unadmitted::Evidence(refused).to_string()
    );
    assert_eq!(std::fs::read(&digest).unwrap(), b"retained");
    assert!(!ran.exists(), "no member started");
    engine.bundle.seats.get_mut("work").unwrap().body = panel(&touch, &touch);
    set_site_hands(&mut engine.bundle, "work:sibling", hands(&[]));
    execute_work(&mut engine, &[effect]).unwrap();
    #[cfg(target_os = "linux")]
    assert!(ran.exists(), "with hands throughout, the panel ran");
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        last(&engine).1["error"],
        Unadmitted::Evidence(Conflict::Shown(host.root())).to_string()
    );
    assert_eq!(std::fs::read(&digest).unwrap(), b"retained");
}

/// The lease is released only on positive proof that the attempt's owned
/// processes settled (#403): an end in error, no terminal event, or any
/// driver whose cleanup is unresolved keeps the reservation standing,
/// whatever terminal event the attempt journals; a terminal event after
/// drivers that all settled, or none, releases it.
#[test]
fn a_lease_is_released_only_on_proof_its_processes_settled() {
    let host = Host::new();
    let (_dir, mut engine) = engine(single_body(Vec::new()));
    let (work, run) = (worktree(&engine), engine.run_id.clone());
    // Each lease names the engine's own journal, which every later
    // admission's recovery reads: what it releases there it may (R2).
    let journal = std::fs::canonicalize(engine.store.path()).unwrap();
    let leased = |engine: &mut Engine, attempt: &str, cleanup: Option<Cleanup>| {
        let mut claim = claim(&work, [&run, attempt], Protects::Nothing, &[]);
        claim.journal = journal.clone();
        engine.held = Some(host.admit(&claim).unwrap());
        if let Some(cleanup) = cleanup {
            witnessed(&mut engine.held, &cleanup);
        }
    };
    let ended = |engine: &mut Engine, event: EventType, attempt: &str| {
        let payload = json!({"effect_id": "e", "attempt_id": attempt, "error": "x", "reason": "r"});
        let id = engine.run_id.clone();
        let appended = engine
            .store
            .append_next(&id, event, payload, None, Some(attempt.into()));
        appended.unwrap();
    };
    let unresolved = || {
        Some(Cleanup::Unresolved {
            reason: Unsettled::Stdout,
        })
    };
    leased(&mut engine, "a1", None);
    let error = EngineError::Invariant("test", "an attempt that ended in error".into());
    let ran = engine.settle("a1", Err(error));
    assert!(matches!(ran, Err(EngineError::Invariant("test", _))));
    leased(&mut engine, "a2", Some(Cleanup::Settled));
    engine.settle("a2", Ok(())).unwrap();
    for (attempt, event) in [
        ("a3", EventType::EffectIndeterminate),
        ("a4", EventType::EffectSucceeded),
    ] {
        leased(&mut engine, attempt, unresolved());
        witnessed(&mut engine.held, &Cleanup::Settled);
        ended(&mut engine, event, attempt);
        engine.settle(attempt, Ok(())).unwrap();
    }
    let kept = ["a1", "a2", "a3", "a4"].map(String::from).to_vec();
    assert_eq!(host.attempts(&work, &run), (kept.clone(), vec![]));
    for (attempt, event) in [
        ("a5", EventType::EffectFailed),
        ("a6", EventType::EffectIndeterminate),
    ] {
        leased(&mut engine, attempt, Some(Cleanup::Settled));
        ended(&mut engine, event, attempt);
        engine.settle(attempt, Ok(())).unwrap();
    }
    leased(&mut engine, "a7", None);
    ended(&mut engine, EventType::EffectIndeterminate, "a7");
    engine.settle("a7", Ok(())).unwrap();
    let released = ["a5", "a6", "a7"].map(String::from).to_vec();
    assert_eq!(host.attempts(&work, &run), (kept, released));
    assert_eq!(engine.held, None, "the door's copy goes with the attempt");
}

/// A moved-HEAD gate whose driver's cleanup is unresolved keeps its
/// reservation: the rewrite journals a bare indeterminate end, with no
/// cleanup facts, and the lease reads the report's own. Something outside
/// the driver's tree holds its standard output open, which #403 cannot
/// prove over. Linux only: the holder opens the driver's `/proc` entry.
#[cfg(target_os = "linux")]
#[test]
fn a_moved_head_gate_with_unresolved_cleanup_keeps_its_reservation() {
    if !alone("a_moved_head_gate_with_unresolved_cleanup_keeps_its_reservation") {
        return;
    }
    let (host, _home, dir, mut engine) = stood_in();
    let repo = engine.workdir();
    crate::engine::tests::git_commit(&repo, "base");
    let moves = "git commit -q --allow-empty -m gate-moved-head";
    let (before, holder) = held_open(dir.path(), moves);
    // Boxed to its worktree, so a fresh admission reads its journal.
    engine.boundary = brokkr_core::realms::Boundary::Namespace;
    set_site_hands(&mut engine.bundle, "work", hands(&[]));
    let work = engine.bundle.seats.get_mut("work").unwrap();
    (work.body, work.gate) = (
        single_body(driver(&before)),
        crate::bundle::GateAssessment::AllGate,
    );
    let payload = unsettled(&host, &mut engine, holder);
    let reason = payload["reason"].as_str().unwrap();
    assert!(reason.starts_with("GATE-MOVED-HEAD "), "{payload}");
    let cleanup = payload.get("cleanup");
    assert_eq!(cleanup, None, "the rewrite carries no cleanup");
}

/// [`executed`] with its driver's output [`held_open`] until `holder` lets
/// it go: the attempt ended indeterminate, its reservation standing, its
/// engine having sealed the unresolved cleanup into it as it witnessed it
/// (R2), and a fresh admission of another run on its worktree, reading its
/// own sealed journal, refused. Its terminal event's payload.
#[cfg(target_os = "linux")]
fn unsettled(host: &Host, engine: &mut Engine, holder: impl FnOnce()) -> serde_json::Value {
    let (event, payload) = executed(engine);
    holder();
    assert_eq!(event, EventType::EffectIndeterminate, "{payload}");
    let work = worktree(engine);
    let (standing, settled) = host.attempts(&work, &engine.run_id);
    assert_eq!((standing.len(), settled.len()), (1, 0));
    let bytes = std::fs::read(host.reservation(&work, &engine.run_id)).unwrap();
    let sealed: Reservation = serde_json::from_slice(&bytes).unwrap();
    let witnessed = sealed.attempts.values().map(|reserved| reserved.cleanup);
    assert_eq!(witnessed.collect::<Vec<_>>(), [Witnessed::Unresolved]);
    let fresh = [boxed("work", &[&work])];
    let refused = host.admit(&claim(&work, ["run-b", "b1"], Protects::Nothing, &fresh));
    assert_eq!(refused, cr2(Conflict::Held(engine.run_id.clone())));
    payload
}

/// A driver's `before`, run in `dir`: `first`, and then its standard
/// output held open by something outside its tree, which #403 cannot
/// prove over; and how the holder is let go once the attempt ended.
#[cfg(target_os = "linux")]
fn held_open(dir: &Path, first: &str) -> (String, impl FnOnce()) {
    let (pid, go) = (dir.join("pid"), dir.join("go"));
    let before = format!(
        "{first}; echo $$ > '{}'; while [ ! -e '{}' ]; do sleep 0.02; done",
        pid.display(),
        go.display()
    );
    let (release_holder, held_until) = mpsc::channel::<()>();
    let holder = std::thread::spawn(move || {
        let deadline = soon();
        let pid = loop {
            let read = std::fs::read_to_string(&pid).ok();
            if let Some(pid) = read.and_then(|pid| pid.trim().parse::<u32>().ok()) {
                break pid;
            }
            assert!(Instant::now() < deadline, "the driver never started");
            std::thread::sleep(Duration::from_millis(10));
        };
        let stdout = std::fs::OpenOptions::new()
            .write(true)
            .open(format!("/proc/{pid}/fd/1"));
        std::fs::write(go, "").unwrap();
        held_until.recv_timeout(Duration::from_secs(60)).ok();
        drop(stdout.unwrap());
    });
    let ended = move || {
        release_holder.send(()).ok();
        holder.join().unwrap();
    };
    (before, ended)
}

/// A panel member whose cleanup is unresolved keeps its attempt's
/// reservation, the panel's own end indeterminate. Linux only, as
/// [`held_open`] is.
#[cfg(target_os = "linux")]
#[test]
fn a_panel_member_with_unresolved_cleanup_keeps_its_reservation() {
    if !alone("a_panel_member_with_unresolved_cleanup_keeps_its_reservation") {
        return;
    }
    let (host, _home, dir, mut engine) = stood_in();
    let (before, holder) = held_open(dir.path(), ":");
    engine.bundle.seats.get_mut("work").unwrap().body = panel(&before, ":");
    let payload = unsettled(&host, &mut engine, holder);
    let member = payload["unresolved_members"].get("keeper");
    assert!(member.is_some(), "{payload}");
}

/// Whether an artifact root holds evidence, as admission observes it:
/// missing or empty holds none, anything in it does, and a root that is
/// not a directory of its own, or cannot be observed, refuses by U8a's
/// cause.
#[test]
fn the_artifact_root_holds_evidence_only_where_anything_stands_in_it() {
    let host = Host::new();
    let work = host.worktree("work");
    let root = work.join("artifacts");
    assert_eq!(evidence(&root), Ok(false));
    std::fs::create_dir(&root).unwrap();
    assert_eq!(evidence(&root), Ok(false));
    std::fs::write(root.join("sha256"), "").unwrap();
    assert_eq!(evidence(&root), Ok(true));
    let file = work.join("file");
    std::fs::write(&file, "").unwrap();
    assert_eq!(evidence(&file), Err(Unrooted::Foreign(file.clone())));
    let below = file.join("artifacts");
    let unmade = Unrooted::Unmade(below.clone(), std::io::ErrorKind::NotADirectory);
    assert_eq!(evidence(&below), Err(unmade));
}

/// Off Linux no mount table is read, so no boxed writer is kept from an
/// MCP server's inputs (U8a2b; decision 0063): the extension of a lease
/// held by one is refused by MB3's identity cause naming the first input.
#[cfg(not(target_os = "linux"))]
#[test]
fn off_linux_no_boxed_writer_is_kept_from_a_servers_inputs() {
    use crate::engine::broker::sources::{extended, fixed};
    let host = Host::new();
    let work = host.worktree("a");
    let writers = [boxed("work", &[&work])];
    let held = host.admit(&claim(&work, ["run-a", "a1"], Protects::Nothing, &writers));
    let bootstrap = host.dir.path().join("brokkr");
    let commits = Cell::new(Commit::Synced);
    let extension = extended(&held.unwrap(), fixed(&bootstrap), soon(), &commits);
    assert_eq!(extension, Err(Unknown::Unmounted(bootstrap).into()));
}
