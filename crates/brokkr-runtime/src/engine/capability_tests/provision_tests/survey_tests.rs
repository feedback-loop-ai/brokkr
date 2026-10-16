//! U7c (MB3, SD4; operator rulings 2026-10-15 and 2026-10-16): each plan's
//! box is surveyed by the current Brokkr binary's `broker survey`, a child
//! in a process group of its own under the startup deadline, sent one
//! request line. The real binary surveys from an ordinary, unconfined
//! process, and `broker serve` binds the plan the engine wrote from its
//! reply, and refuses it substituted or without its inventory. A stand-in
//! that never answers is killed with its whole group at the deadline, only
//! one closed reply of the bootstrap it was started as is an answer, and a
//! real engine ended mid-survey, by SIGTERM or SIGKILL, leaves nothing of
//! the survey running.

use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use brokkr_protocol::broker::{
    Bootstrap, Network, Plan, Provisioned, Reach, Refusal, Sources, Survey, Surveyed, Tree, Unread,
};
use brokkr_protocol::hands::{self, ServerProgram};

use super::super::super::broker::{surveyed, Host, Unprovisioned};
use super::{admitted, scratch, sealed, Fixture};
use super::{Path, PathBuf, PermissionsExt};

/// What `broker serve` says of a plan the attempt's inventory does not pin.
const UNBOUND: &str = "error: broker plan is not bound to this attempt\n";

/// Whether the test `test` of this module runs here, in an engine of its
/// own (#403): the test binary's engine tracks every other test's live
/// attempts, and a survey, its child outside its group, would read as an
/// orphan one of them left and be killed with it. False in the parent,
/// once the re-executed test passed alone.
fn alone(test: &str) -> bool {
    let module = "engine::capability_tests::provision_tests::survey_tests";
    brokkr_protocol::process::in_its_own_engine(&format!("{module}::{test}"))
}

/// An owner-only directory under the fixtures' base.
fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir_in(scratch()).unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

/// `from` installed as the executable `name` in `dir`, singly linked: a
/// child `cp` writes it and it is renamed in, so no fork of this process
/// holds it open for writing when it runs (#255). Its canonical path.
fn installed(dir: &Path, name: &str, from: &Path) -> PathBuf {
    let staged = dir.join(format!("{name}.staged"));
    let cp = Command::new("/bin/cp").arg(from).arg(&staged).status();
    assert_eq!(cp.unwrap().code(), Some(0), "copying {}", from.display());
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = dir.join(name);
    std::fs::rename(&staged, &path).unwrap();
    std::fs::canonicalize(path).unwrap()
}

/// A survey's stand-in in `dir`: a shell script running `body`.
fn stand_in(dir: &Path, body: &str) -> PathBuf {
    let source = dir.join("stand-in.sh");
    std::fs::write(&source, format!("#!/bin/sh\n{body}\n")).unwrap();
    installed(dir, "stand-in", &source)
}

/// A request for the system `true` under `home`.
fn request(home: &Path) -> Survey {
    Survey {
        launch: "true".into(),
        home: home.to_path_buf(),
        reach: Reach {
            writable: Vec::new(),
            readable: Vec::new(),
        },
        network: Network::Isolated,
        writers: vec![hands::ids().0],
    }
}

/// The state, parent and process group `/proc` shows for `pid`, none once
/// it is gone.
#[cfg(target_os = "linux")]
fn shown(pid: i32) -> Option<(char, i32, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, rest) = stat.rsplit_once(") ")?;
    let fields: Vec<&str> = rest.split(' ').take(3).collect();
    let [state, parent, group] = fields[..] else {
        return None;
    };
    let state = state.chars().next()?;
    Some((state, parent.parse().ok()?, group.parse().ok()?))
}

/// Whether the process `pid` still runs: a zombie, ended but not yet
/// reaped by whoever adopted it, does not.
#[cfg(target_os = "linux")]
fn running(pid: i32) -> bool {
    shown(pid).is_some_and(|(state, _, _)| state != 'Z')
}

/// Off Linux, whether a signal still reaches `pid`.
#[cfg(not(target_os = "linux"))]
fn running(pid: i32) -> bool {
    let pid = rustix::process::Pid::from_raw(pid).unwrap();
    rustix::process::test_kill_process(pid).is_ok()
}

/// A stand-in that records its own pid and its child's, in `pids`, and
/// waits on that child, which never ends: it reads nothing and answers
/// nothing.
fn stalled(dir: &Path) -> (PathBuf, PathBuf) {
    let pids = dir.join("pids");
    let body = format!(
        "echo $$ >> '{0}'\nsleep 600 &\necho $! >> '{0}'\nwait",
        pids.display()
    );
    (stand_in(dir, &body), pids)
}

/// Every process the stalled stand-in recorded has ended within a few
/// seconds: the survey and the child in its group.
fn none_survives(pids: &Path) {
    let recorded = std::fs::read_to_string(pids).unwrap();
    let recorded: Vec<i32> = recorded.lines().map(|pid| pid.parse().unwrap()).collect();
    assert_eq!(recorded.len(), 2, "the stand-in and its child");
    let settled = Instant::now() + Duration::from_secs(5);
    while recorded.iter().any(|pid| running(*pid)) && Instant::now() < settled {
        std::thread::sleep(Duration::from_millis(20));
    }
    let survivors: Vec<&i32> = recorded.iter().filter(|pid| running(**pid)).collect();
    assert_eq!(survivors, Vec::<&i32>::new());
}

/// M2: a survey that never answers refuses at the deadline, and neither it
/// nor anything of its group survives.
#[test]
fn a_silent_survey_is_killed_with_its_group_at_the_deadline() {
    if !alone("a_silent_survey_is_killed_with_its_group_at_the_deadline") {
        return;
    }
    let home = scratch();
    let expiring = private_dir();
    let (script, pids) = stalled(expiring.path());
    let begun = Instant::now();
    let deadline = begun + Duration::from_millis(1500);
    assert_eq!(
        surveyed(&script, &request(&home), deadline),
        Err(Unprovisioned::Expired)
    );
    assert!(begun.elapsed() < Duration::from_secs(10));
    none_survives(&pids);
}

/// The line a replying stand-in writes, given the path it is installed at.
type Line<'l> = &'l dyn Fn(&Path) -> String;

/// Only one closed reply line of the bootstrap the survey was started as
/// is an answer: its box, MB3's cause it refused with, or why it read no
/// request. A reply naming another bootstrap, two lines, a line left
/// open, and a bootstrap that does not start answer nothing.
#[test]
fn only_one_closed_reply_of_the_surveys_own_bootstrap_is_an_answer() {
    if !alone("only_one_closed_reply_of_the_surveys_own_bootstrap_is_an_answer") {
        return;
    }
    let home = scratch();
    let deadline = Instant::now() + Duration::from_secs(60);
    let provisioned = |path: &Path| Provisioned {
        executable: PathBuf::from("/usr/bin/true"),
        tree: Tree::System {},
        bootstrap: Bootstrap {
            path: path.to_path_buf(),
            digest: "b".repeat(64),
        },
        sources: Sources {
            entries: 1,
            mounts: 1,
            digest: "5".repeat(64),
        },
    };
    let replying = |reply: &str, with: Line<'_>| {
        let dir = private_dir();
        let path = std::fs::canonicalize(dir.path()).unwrap().join("stand-in");
        // It reads its one request line, then replies with its input open.
        let body = format!("read -r _\nprintf '{reply}' '{}'", with(&path));
        let script = stand_in(dir.path(), &body);
        assert_eq!(script, path);
        (surveyed(&script, &request(&home), deadline), path)
    };
    let line = |surveyed: Surveyed| serde_json::to_string(&surveyed).unwrap();
    let own = |path: &Path| line(Surveyed::Provisioned(provisioned(path)));
    let (answer, path) = replying("%s\\n", &own);
    assert_eq!(answer, Ok(provisioned(&path)));
    let other = |_: &Path| own(Path::new("/opt/brokkr/bin/brokkr"));
    let refused = |_: &Path| line(Surveyed::Refused(Refusal::Linked));
    let unread = |_: &Path| line(Surveyed::Unread(Unread::Truncated));
    let cases: [(&str, Line<'_>, Unprovisioned); 6] = [
        ("%s\\n", &refused, Unprovisioned::Box(Refusal::Linked)),
        ("%s\\n", &unread, Unprovisioned::Unread(Unread::Truncated)),
        ("%s\\n", &other, Unprovisioned::Unanswered),
        ("%s\\n%s\\n", &refused, Unprovisioned::Unanswered),
        ("%s\\n\\n", &refused, Unprovisioned::Unanswered),
        ("%s", &refused, Unprovisioned::Unanswered),
    ];
    for (reply, with, answer) in cases {
        assert_eq!(replying(reply, with).0, Err(answer), "{reply}");
    }
    let missing = scratch().join("no-such-brokkr");
    assert_eq!(
        surveyed(&missing, &request(&home), deadline),
        Err(Unprovisioned::Unanswered)
    );
}

/// The binary cargo built beside this test binary, the one `brokkr` a
/// workspace build makes, where it is current: no file its dep-info names,
/// and no workspace source a library built in its profile names
/// ([`library_sources`]), is newer than it, so it reads and writes as the
/// sources beside this test do. Otherwise why it is not run. A relative
/// source is the workspace root's, where cargo runs the compiler.
fn built_brokkr() -> Result<PathBuf, &'static str> {
    let stale = "the brokkr beside this test binary is older than its sources: build it";
    let this = std::env::current_exe().ok();
    let built = (this.as_deref().and_then(profile))
        .map(|dir| dir.join("brokkr"))
        .filter(|built| built.is_file());
    let built =
        built.ok_or("no brokkr binary beside this test binary: a workspace build makes it")?;
    let modified = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    let made = modified(&built).ok_or(stale)?;
    let info = std::fs::read_to_string(dep_info(&built)).unwrap_or_default();
    let sources = first_rule(&info).ok_or(stale)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2);
    let root = root.ok_or(stale)?;
    let mut sources = sources.split_whitespace().map(|source| root.join(source));
    let current = sources.all(|source| modified(&source).is_some_and(|at| at <= made));
    let mut linked = library_sources(&built).into_iter();
    let linked = linked.all(|source| modified(&root.join(source)).is_none_or(|at| at <= made));
    (current && linked).then_some(built).ok_or(stale)
}

/// The sources the first rule of the dep-info `info` names.
fn first_rule(info: &str) -> Option<&str> {
    let (_, sources) = info.lines().next()?.split_once(": ")?;
    Some(sources)
}

/// Every workspace source the compiler's dep-info of a library built in the
/// profile of the uplifted binary `built` names. The binary's own dep-info
/// names only its own file, while it links the workspace's libraries, and a
/// test build that needs none of it relinks nothing: a library source newer
/// than the binary is one it may not have been built from. Every library
/// cargo built here counts, since which one the binary linked is not
/// recorded, so one it did not link can only make it read stale, never
/// current; a source missing here is another tree's, a kept target's, whose
/// change moved a source this tree has. A relative source is the
/// workspace's; a registry's is absolute.
fn library_sources(built: &Path) -> BTreeSet<PathBuf> {
    let libraries = (outputs(built).flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some("rlib".as_ref()));
    let infos = libraries.filter_map(|library| {
        let crate_name = library.file_stem()?.to_str()?.strip_prefix("lib")?;
        std::fs::read_to_string(library.with_file_name(format!("{crate_name}.d"))).ok()
    });
    let mut sources = BTreeSet::new();
    for info in infos {
        let named = first_rule(&info).unwrap_or_default().split_whitespace();
        let named = named.map(PathBuf::from);
        sources.extend(named.filter(|source| source.is_relative()));
    }
    sources
}

/// The directories cargo writes the profile of the uplifted binary `built`'s
/// compiled units in: `deps`, and, under the build-dir layout, each
/// package's `build/<package>/<hash>/out`.
fn outputs(built: &Path) -> impl Iterator<Item = PathBuf> {
    let packages = std::fs::read_dir(built.with_file_name("build"));
    let hashes = (packages.into_iter().flatten().flatten())
        .flat_map(|package| std::fs::read_dir(package.path()).into_iter().flatten())
        .flatten();
    let outs = hashes.map(|hash| hash.path().join("out"));
    std::iter::once(built.with_file_name("deps")).chain(outs)
}

/// The profile directory the test binary `test` was built in. Cargo keeps
/// a test binary in the profile's `deps`, or, under the build-dir layout
/// the exact coverage gate's toolchain uses, in `build/<package>/<hash>/out`;
/// either way it uplifts the `brokkr` binary into the profile directory.
fn profile(test: &Path) -> Option<&Path> {
    let dir = test.parent()?;
    if dir.ends_with("deps") {
        return dir.parent();
    }
    let build = dir.ancestors().nth(3)?;
    (dir.ends_with("out") && build.ends_with("build"))
        .then(|| build.parent())
        .flatten()
}

/// The dep-info of the uplifted binary `built`: cargo's beside it, naming
/// every workspace source it was built from, which cargo writes only for a
/// binary built on its own (`cargo build`); or, for one built only because
/// a test needs it, as the suites and the exact gate build it, the
/// compiler's beside the copy it is a hard link of ([`outputs`]), naming
/// only the binary's own file, the libraries it links being read apart
/// ([`library_sources`]). Cargo's is trusted only where it is no older
/// than the binary: a target a later test build relinked the binary in
/// keeps another build's. Where none is found, the one beside it, which
/// reads as stale.
fn dep_info(built: &Path) -> PathBuf {
    let beside = built.with_extension("d");
    let Ok(binary) = std::fs::metadata(built) else {
        return beside;
    };
    let written = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    if written(&beside) >= binary.modified().ok() {
        return beside;
    }
    let same = (binary.dev(), binary.ino());
    let identity = |path: &Path| std::fs::metadata(path).map(|meta| (meta.dev(), meta.ino()));
    let copies = outputs(built)
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten())
        .flatten()
        .map(|entry| entry.path())
        .filter(|copy| copy.extension().is_none());
    let mut linked = copies.filter(|copy| identity(copy).is_ok_and(|other| other == same));
    linked
        .next()
        .map_or(beside, |copy| copy.with_extension("d"))
}

/// `broker serve` of the plan `locator` pins at `digest`, by `brokkr`
/// under `home`: its exit code and standard error.
fn served(brokkr: &Path, home: &Path, locator: &Path, digest: &str) -> (Option<i32>, String) {
    let ran = Command::new(brokkr)
        .args(["broker", "serve", "--plan"])
        .arg(locator)
        .args(["--plan-digest", digest])
        .env("HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&ran.stderr).into_owned();
    (ran.status.code(), said)
}

/// M1, M3, M4 and L2 in the production arrangement: the real binary, as a
/// singly linked copy, surveys a system entry from this ordinary process,
/// unconfined, judging the sealed writers as confined, and the engine
/// seals its reply — the copy and its own digest as the bootstrap, the
/// resolved program — beside a store spelled through `..`, sealed normal.
/// The same binary's `broker serve`, under the same HOME, refuses the plan
/// substituted and the attempt without its inventory, and binds the plan
/// the engine wrote: its ledger opens beside it, which no unbound plan
/// has. Off Linux the survey refuses as no box stands.
#[test]
fn the_real_survey_provisions_from_an_unconfined_engine_and_the_broker_binds_its_plan() {
    if !alone("the_real_survey_provisions_from_an_unconfined_engine_and_the_broker_binds_its_plan")
    {
        return;
    }
    let required = hands::boundary_evidence_required();
    if std::env::var_os(hands::HANDS_BOX_ENV).is_some() {
        hands::skip_boundary_proof(required, "a seat's box prepares no server box");
        return;
    }
    let built = match built_brokkr() {
        Ok(built) => built,
        Err(reason) => return hands::skip_boundary_proof(required, reason),
    };
    let test = "the_real_survey_provisions_from_an_unconfined_engine_and_the_broker_binds_its_plan";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let copies = private_dir();
    let brokkr = installed(copies.path(), "brokkr", &built);
    fixture.hold("status", admitted("status", &["true"], false));
    let spelled = fixture.root.join("work/../private/secrets.env");
    fixture.engine.secrets_file = Some(spelled);
    let host = Host {
        bootstrap: brokkr.clone(),
        ..fixture.host()
    };
    let provisioned = fixture.provisioned_by("attempt-1", Ok(host), &surveyed);
    if !cfg!(target_os = "linux") {
        assert_eq!(provisioned, Err(Unprovisioned::Box(Refusal::Unavailable)));
        return;
    }
    let control = provisioned.unwrap().unwrap();
    let (inventory, plans) = sealed(&control);
    let plan: Plan = serde_json::from_value(plans[0].clone()).unwrap();
    let digest = brokkr_core::canonical::sha256_bytes(&std::fs::read(&brokkr).unwrap());
    let bootstrap = Bootstrap {
        path: brokkr.clone(),
        digest,
    };
    assert_eq!(plan.intent.bootstrap, bootstrap);
    let home = fixture.root.join("home");
    let program = ServerProgram::resolve("true", &home).unwrap();
    assert_eq!(plan.intent.executable, program.executable());
    assert_eq!(plan.intent.tree, Tree::System {});
    let store = fixture.root.join("private/secrets.env");
    assert_eq!(plan.intent.excluded.store, store);
    let pin = &inventory.plans[0];
    let serve = || served(&brokkr, &home, &pin.locator, &pin.digest);
    let unbound = (Some(1), UNBOUND.to_string());
    let bytes = std::fs::read(&pin.locator).unwrap();
    std::fs::write(&pin.locator, [&bytes[..], b" "].concat()).unwrap();
    assert_eq!(serve(), unbound, "a substituted plan");
    std::fs::write(&pin.locator, &bytes).unwrap();
    let pins = control.join("inventory.json");
    let pinned = std::fs::read(&pins).unwrap();
    std::fs::remove_file(&pins).unwrap();
    assert_eq!(serve(), unbound, "an attempt without its inventory");
    std::fs::write(&pins, pinned).unwrap();
    std::fs::set_permissions(&pins, std::fs::Permissions::from_mode(0o600)).unwrap();
    let (_, said) = serve();
    assert_ne!(said, UNBOUND);
    let ledger = control.join("plan-0.json.ledger");
    assert!(ledger.exists(), "the bound plan's ledger: {said}");
}

/// Set in the engine [`an_engine_ended_mid_survey_leaves_nothing_of_the_survey_running`]
/// runs: the `brokkr` it surveys with.
#[cfg(target_os = "linux")]
const SURVEYING: &str = "BROKKR_TEST_SURVEYING_WITH";

/// Set beside [`SURVEYING`]: the HOME that engine's survey resolves.
#[cfg(target_os = "linux")]
const HELD_HOME: &str = "BROKKR_TEST_SURVEY_HELD_HOME";

/// The tracer's arguments, the held HOME named next: every process of the
/// run followed, stopped by a seccomp filter at file calls alone, nothing
/// written, and each file call naming the held HOME held at its entry for
/// ten minutes. Only the survey names it, resolving it as its first act
/// once its request is read and its watcher armed, so the survey stalls
/// there while its watcher runs on.
#[cfg(target_os = "linux")]
const HOLDING: [&str; 10] = [
    "-f",
    "--seccomp-bpf",
    "-qqq",
    "-o",
    "/dev/null",
    "-e",
    "trace=%file",
    "-e",
    "inject=%file:delay_enter=600000000",
    "-P",
];

/// Every process `/proc` shows whose state, parent and group `keep` holds
/// to.
#[cfg(target_os = "linux")]
fn listed(keep: impl Fn((char, i32, i32)) -> bool) -> Vec<i32> {
    let entries = std::fs::read_dir("/proc").unwrap();
    let pids = entries.filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok());
    pids.filter(|pid| shown(*pid).is_some_and(&keep)).collect()
}

/// The one child of `parent` once it stands, within half a minute.
#[cfg(target_os = "linux")]
fn child_of(parent: i32) -> i32 {
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        if let [child] = listed(|(_, of, _)| of == parent)[..] {
            return child;
        }
        assert!(Instant::now() < until, "no child of {parent} started");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Wait until the survey `pid` is held by the tracer, its watcher beside
/// it: its first thread stopped ('t') a whole second through, which no
/// passing file call is, and a second thread standing.
#[cfg(target_os = "linux")]
fn held(pid: i32) {
    let until = Instant::now() + Duration::from_secs(30);
    let mut since = Instant::now();
    loop {
        let threads = std::fs::read_dir(format!("/proc/{pid}/task")).map_or(0, Iterator::count);
        let stopped = shown(pid).is_some_and(|(state, _, _)| state == 't');
        if !(stopped & (threads == 2)) {
            since = Instant::now();
        }
        if since.elapsed() >= Duration::from_secs(1) {
            return;
        }
        assert!(Instant::now() < until, "the survey {pid} was never held");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Whether the process `pid` runs on unkilled: it is neither gone nor a
/// zombie, and no SIGKILL pends on it, as one does on a killed process
/// only while a tracer holds it at its exit.
#[cfg(target_os = "linux")]
fn unkilled(pid: i32) -> bool {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let pending = |name: &str| {
        let mask = status.lines().find_map(|line| line.strip_prefix(name));
        mask.and_then(|mask| u64::from_str_radix(mask.trim(), 16).ok())
    };
    let kill = 1 << (rustix::process::Signal::KILL.as_raw() - 1);
    let killed = (pending("SigPnd:").unwrap_or(0) | pending("ShdPnd:").unwrap_or(0)) & kill != 0;
    running(pid) & !killed
}

/// The members of the group `group` that `stands` holds to, once none is
/// or ten seconds pass.
#[cfg(target_os = "linux")]
fn standing(group: i32, stands: fn(i32) -> bool) -> Vec<i32> {
    let left = || -> Vec<i32> {
        let members = listed(|(_, _, of)| of == group);
        members.into_iter().filter(|pid| stands(*pid)).collect()
    };
    let until = Instant::now() + Duration::from_secs(10);
    while !left().is_empty() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    left()
}

/// The operator ruling of 2026-10-16: a real engine ended mid-survey ends
/// the survey with its whole group, by SIGTERM before any stop handler
/// exists (this test binary installs none, as the engine installs none
/// before its first driver) and by SIGKILL. The engine is this test,
/// re-executed under a tracer that holds its survey of the real binary in
/// the survey's resolution of its HOME, past its request and with its
/// watcher armed; the engine's end closes the survey's input, and nothing
/// of its group survives.
#[cfg(target_os = "linux")]
#[test]
fn an_engine_ended_mid_survey_leaves_nothing_of_the_survey_running() {
    let test = "an_engine_ended_mid_survey_leaves_nothing_of_the_survey_running";
    if let (Some(brokkr), Some(home)) = (std::env::var_os(SURVEYING), std::env::var_os(HELD_HOME)) {
        let deadline = Instant::now() + Duration::from_secs(600);
        let answer = surveyed(Path::new(&brokkr), &request(Path::new(&home)), deadline);
        panic!("the engine was not ended mid-survey: {answer:?}");
    }
    if !alone(test) {
        return;
    }
    let required = hands::boundary_evidence_required();
    if std::env::var_os(hands::HANDS_BOX_ENV).is_some() {
        return hands::skip_boundary_proof(required, "a seat's box traces no survey");
    }
    let built = match built_brokkr() {
        Ok(built) => built,
        Err(reason) => return hands::skip_boundary_proof(required, reason),
    };
    let traces = Command::new("strace")
        .args(["-f", "-qqq", "-o", "/dev/null", "/bin/true"])
        .stderr(Stdio::null())
        .status();
    if !traces.is_ok_and(|status| status.success()) {
        let reason = "no strace on PATH traces a child here, so no survey can be held";
        return hands::skip_boundary_proof(required, reason);
    }
    let module = "engine::capability_tests::provision_tests::survey_tests";
    for signal in [rustix::process::Signal::TERM, rustix::process::Signal::KILL] {
        let home = private_dir();
        let home = std::fs::canonicalize(home.path()).unwrap();
        let mut tracer = Command::new("strace")
            .args(HOLDING)
            .arg(&home)
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{module}::{test}")])
            .env(SURVEYING, &built)
            .env(HELD_HOME, &home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let engine = child_of(tracer.id() as i32);
        let survey = child_of(engine);
        held(survey);
        let pid = |pid: i32| rustix::process::Pid::from_raw(pid).unwrap();
        rustix::process::kill_process(pid(engine), signal).unwrap();
        let unkilled = standing(survey, unkilled);
        // The tracer let go, a member it held at its exit ends.
        tracer.kill().ok();
        tracer.wait().unwrap();
        let left = standing(survey, running);
        rustix::process::kill_process_group(pid(survey), rustix::process::Signal::KILL).ok();
        let none = Vec::<i32>::new();
        assert_eq!((signal, unkilled, left), (signal, none.clone(), none));
        assert!(!running(engine));
    }
}
