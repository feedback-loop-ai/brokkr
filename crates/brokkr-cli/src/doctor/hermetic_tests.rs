//! A doctor test that reaches a real version probe, run again in a child
//! of this test binary whose `PATH` holds only the tools the test planted
//! (#1137). Doctor's tests once kept the host's `PATH` under a temporary
//! HOME, so they ran whatever `node`, `claude`, `codex` or `openspec` the
//! host had; a Volta shim under a temporary HOME resolves its tool to
//! itself, and one such run forked 72,000 processes. The process
//! environment is one per test binary, so the `PATH` is the child's, never
//! a write this binary's other threads could see mid-spawn.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[path = "../../tests/support/pidns.rs"]
mod pidns;

#[cfg(target_os = "linux")]
use pidns::Isolation;

/// Set on the child, so the test's body runs there and not here.
const CHILD: &str = "BROKKR_DOCTOR_HERMETIC_CHILD";

/// In the parent: run the calling test again in a child whose `PATH` is a
/// directory holding `tools` (each a name and the file it links to) and
/// nothing else, under a temporary HOME, with no `BROKKR_` or `DSH_`
/// variable that could name a host executable; fail unless the child ran
/// that one test and it passed; and answer `true`, so the caller returns.
/// In the child: answer `false`, so the caller's body runs.
pub(super) fn rerun(tools: &[(&str, &Path)]) -> bool {
    rerun_through(&[], tools)
}

/// Is this the child [`rerun`] started?
pub(super) fn rerunning() -> bool {
    std::env::var_os(CHILD).is_some()
}

/// [`rerun`], the child started through `launcher`: a program and its
/// arguments, which `exec` the test binary named after them, such as a
/// `perl` that ignores SIGCHLD first.
pub(super) fn rerun_through(launcher: &[&str], tools: &[(&str, &Path)]) -> bool {
    if rerunning() {
        return false;
    }
    reran(launcher, tools, &[]).passed_once("under a PATH of its own");
    true
}

/// Set on a child [`rerun_alone`] started: the pid of the sentinel the
/// parent holds outside the child's PID namespace.
#[cfg(target_os = "linux")]
const SENTINEL: &str = "BROKKR_DOCTOR_SENTINEL";

/// Set on a child [`rerun_alone`] started: the parent's PID namespace, as
/// [`pidns::identity`] reads it.
#[cfg(target_os = "linux")]
const OUTER: &str = "BROKKR_DOCTOR_OUTER_PIDNS";

/// [`rerun`], the child alone in a PID namespace of its own (#1137g): a
/// test that delivers a real signal runs its whole process tree there,
/// so whatever pid or group id it names, it can reach no process outside
/// the test. Linux only: macOS has no PID namespace, so a test that
/// delivers a real signal is `cfg(target_os = "linux")`, and macOS keeps
/// the tests whose seam is inert.
///
/// The child proves it is isolated before anything of it can deliver a
/// signal (#1137h, I1): it reads its own PID namespace and refuses, with
/// the reason, unless it differs from the one the parent handed it. A
/// host that cannot build the namespace fails or skips the test as
/// [`pidns::REQUIRE`] demands.
#[cfg(target_os = "linux")]
pub(super) fn rerun_alone(tools: &[(&str, &Path)]) -> bool {
    rerun_alone_through(&[], tools)
}

/// [`rerun_alone`], the child started through `launcher` inside the
/// namespace, as [`rerun_through`] starts it.
#[cfg(target_os = "linux")]
pub(super) fn rerun_alone_through(launcher: &[&str], tools: &[(&str, &Path)]) -> bool {
    if rerunning() {
        alone();
        return false;
    }
    let Some(bwrap) = pidns::isolating() else {
        return true;
    };
    let mut boxed = vec![bwrap];
    boxed.extend(pidns::ALONE);
    boxed.extend(launcher);
    guarded(&boxed, tools).passed_once("alone in a PID namespace");
    true
}

/// The re-run through `launcher`, told the parent's PID namespace, with a
/// sentinel held outside it. The parent starts the sentinel in a group of
/// its own and holds it; the child, once it has proved it is isolated,
/// sends SIGTERM to the sentinel's pid and to its group by number ([`alone`]),
/// standing for any id a test names that is not its own. The sentinel is
/// judged first, whatever the child ended with: it must still run and
/// have recorded no signal, so a kill that left the namespace is caught.
/// The sentinel records SIGTERM rather than dying of it, so a run without
/// the namespace harms nothing.
#[cfg(target_os = "linux")]
fn guarded(launcher: &[&str], tools: &[(&str, &Path)]) -> Reran {
    let outer = pidns::identity().unwrap_or_else(|refused| panic!("{}: {refused}", pidns::REFUSED));
    let outer = outer.to_string();
    let sentinel = Sentinel::started();
    let pid = sentinel.child.id().to_string();
    let reran = reran(launcher, tools, &[(SENTINEL, &pid), (OUTER, &outer)]);
    sentinel.unharmed();
    reran
}

/// In the child, before anything of it can deliver a signal: refuse
/// unless it is isolated ([`isolated`]), then stray at the sentinel.
#[cfg(target_os = "linux")]
fn alone() {
    if let Err(refused) = isolated() {
        panic!("{}: {refused}", pidns::REFUSED);
    }
    strayed();
}

/// Is this child in a PID namespace other than the one its parent named?
/// Its own is read and parsed as `pid:[N]` (#1137i, Z2).
#[cfg(target_os = "linux")]
fn isolated() -> Result<(), Isolation> {
    let outer = std::env::var(OUTER)
        .ok()
        .and_then(|outer| outer.parse().ok());
    let unnamed = || Isolation::Unreadable(format!("the parent named none in {OUTER}"));
    let own = pidns::identity()?;
    apart(outer.ok_or_else(unnamed)?, own)
}

/// The two namespaces apart, or the one they share.
#[cfg(target_os = "linux")]
fn apart(outer: u64, own: u64) -> Result<(), Isolation> {
    match outer == own {
        true => Err(Isolation::Shared(own)),
        false => Ok(()),
    }
}

/// In the child: SIGTERM to the sentinel's pid and group, by number.
#[cfg(target_os = "linux")]
fn strayed() {
    use rustix::process::{kill_process, kill_process_group, Pid, Signal};
    let named = std::env::var(SENTINEL).expect("rerun_alone names its sentinel");
    let pid = Pid::from_raw(named.parse().unwrap()).unwrap();
    // Neither answer is read: inside the namespace the number names no
    // process outside it, and the parent's sentinel is the judge.
    kill_process(pid, Signal::TERM).ok();
    kill_process_group(pid, Signal::TERM).ok();
}

/// Perl that records each SIGTERM in the file its first argument names,
/// marks the second `ready` once its handler is in place, and runs until
/// its standard input ends.
#[cfg(target_os = "linux")]
const RECORDS: &str = "$SIG{TERM} = sub { open(my $f, \">>\", $ARGV[0]) or die; \
    print $f \"TERM\\n\"; close $f }; \
    open(my $r, \">\", $ARGV[1]) or die; print $r \"ready\\n\"; close $r; \
    while (1) { my $n = sysread(STDIN, my $b, 1); last if defined $n && $n == 0 }";

/// The process [`guarded`] holds outside the namespace.
#[cfg(target_os = "linux")]
struct Sentinel {
    child: std::process::Child,
    told: tempfile::TempDir,
}

#[cfg(target_os = "linux")]
impl Sentinel {
    /// A sentinel in a group of its own, held by its standard input, and
    /// ready: its handler is in place, so a SIGTERM is recorded, never
    /// taken by the default action. Its record exists, empty.
    fn started() -> Sentinel {
        use std::os::unix::process::CommandExt;
        let told = tempfile::tempdir().unwrap();
        std::fs::write(told.path().join("told"), "").unwrap();
        let child = Command::new("/usr/bin/perl")
            .args(["-e", RECORDS])
            .arg(told.path().join("told"))
            .arg(told.path().join("ready"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("the sentinel runs");
        let mut sentinel = Sentinel { child, told };
        sentinel.readied();
        sentinel
    }

    /// Wait, at most PATIENCE, for the mark the sentinel writes once its
    /// handler is in place; fail if it ends or never marks.
    fn readied(&mut self) {
        let deadline = Instant::now() + PATIENCE;
        let ready = self.told.path().join("ready");
        while std::fs::read_to_string(&ready).ok().as_deref() != Some("ready\n") {
            let ended = self.child.try_wait().unwrap();
            assert!(
                ended.is_none() && Instant::now() < deadline,
                "the sentinel was not ready within {PATIENCE:?}: it ended {ended:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Fail unless the sentinel still runs and its record, which must
    /// read, holds no signal; it is then let go and reaped.
    fn unharmed(mut self) {
        let running = self.child.try_wait().unwrap().is_none();
        drop(self.child.stdin.take());
        let reaped = reaped_by(&mut self.child, Instant::now() + SETTLE);
        let told = std::fs::read_to_string(self.told.path().join("told"));
        let told = told.unwrap_or_else(|error| panic!("the sentinel's record reads: {error}"));
        assert!(
            running && reaped && told.is_empty(),
            "a signal left the namespace: the sentinel ran {running}, ended when let go {reaped}, and recorded {told:?}"
        );
    }
}

#[cfg(target_os = "linux")]
impl Drop for Sentinel {
    /// A failed re-run's unwind: the sentinel, this test's own held and
    /// unreaped child, is killed and reaped.
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            self.child.kill().ok();
            self.child.wait().ok();
        }
    }
}

/// How a re-run of one test ended.
struct Reran {
    /// The test's full libtest path.
    test: String,
    /// Whether the child exited 0.
    passed: bool,
    /// What it said on both its outputs.
    said: String,
}

impl Reran {
    /// Fail unless the child ran that one test and it passed.
    fn passed_once(&self, how: &str) {
        let Reran { test, passed, said } = self;
        assert!(
            *passed && said.contains("test result: ok. 1 passed"),
            "{test} {how}:\n{said}"
        );
    }
}

/// Run the calling test again as [`rerun_through`] does, with `env` set on
/// the child: how it ended.
fn reran(launcher: &[&str], tools: &[(&str, &Path)], env: &[(&str, &str)]) -> Reran {
    let thread = std::thread::current();
    let test = thread
        .name()
        .expect("libtest names a test's thread after it");
    let dir = tempfile::tempdir().unwrap();
    let (path, home) = (dir.path().join("path"), dir.path().join("home"));
    std::fs::create_dir_all(&path).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    for (name, target) in tools {
        std::os::unix::fs::symlink(target, path.join(name)).unwrap();
    }
    let exe = std::env::current_exe().unwrap();
    let mut child = match launcher.split_first() {
        None => Command::new(exe),
        Some((program, args)) => {
            let mut child = Command::new(program);
            child.args(args).arg(exe);
            child
        }
    };
    // The FULL libtest path under `--exact`: a filter that matches nothing
    // exits zero, and the count `passed_once` reads refuses that.
    child.args([test, "--exact", "--nocapture", "--test-threads=1"]);
    for (name, _) in std::env::vars_os() {
        let name = name.to_string_lossy();
        if name.starts_with("BROKKR_") || name.starts_with("DSH_") {
            child.env_remove(name.as_ref());
        }
    }
    child.env(CHILD, "1").env("PATH", &path).env("HOME", &home);
    child.env("XDG_STATE_HOME", home.join(".local/state"));
    child.envs(env.iter().copied());
    let (passed, said) = ran(&mut child, test);
    let test = test.to_owned();
    Reran { test, passed, said }
}

/// The report `doctor()` answers for no bundle, with `dir` as the
/// workspace and its `forge.db` as the journal.
#[cfg(target_os = "linux")]
pub(super) fn doctored_in(dir: &Path) -> super::Report {
    let defaults = crate::folder::DoctorDefaults::of(dir.into(), None);
    super::doctor(None, Some(&dir.join("forge.db")), &defaults, None)
}

/// [`rerun`] with the one tool a bundle's compile asks, the real `git`.
pub(super) fn rerun_with_git() -> bool {
    rerun(&[("git", &host_tool("git"))])
}

/// How long a re-run may take, spawn retries included, before it is
/// killed and the test fails instead of hanging.
const PATIENCE: Duration = Duration::from_secs(240);

/// Run `child`, a re-run of this test binary, for at most PATIENCE:
/// whether it exited 0, and what it said on both its outputs. Only
/// ETXTBSY is retried: a test that re-executes its own binary can reach
/// `exec` while another thread of this run still holds a write descriptor
/// to a file it staged, which is a fact about the moment rather than
/// about the test (#255).
pub(super) fn ran(child: &mut Command, what: &str) -> (bool, String) {
    let deadline = Instant::now() + PATIENCE;
    let said = tempfile::NamedTempFile::new().unwrap();
    let mut running = loop {
        child.stdin(Stdio::null());
        // Both outputs append, so neither writes over what the other said.
        let appending = || {
            let mut options = std::fs::OpenOptions::new();
            options.append(true).open(said.path()).unwrap()
        };
        child.stdout(appending()).stderr(appending());
        match child.spawn() {
            Ok(running) => break running,
            Err(error) if error.raw_os_error() == Some(26) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("{what}: the child test binary runs: {error}"),
        }
    };
    let status = loop {
        if let Some(status) = running.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            running.kill().ok();
            let reaped = reaped_by(&mut running, Instant::now() + SETTLE);
            panic!("{what}: the child test binary did not end within {PATIENCE:?} (reaped after its kill: {reaped})");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let said = std::fs::read(said.path()).unwrap();
    (
        status.success(),
        String::from_utf8_lossy(&said).into_owned(),
    )
}

/// How long a killed re-run has to be reaped before the test fails
/// without it.
const SETTLE: Duration = Duration::from_secs(5);

/// Was the killed `child` reaped by `until`? Never waited for past it.
fn reaped_by(child: &mut std::process::Child, until: Instant) -> bool {
    while Instant::now() < until {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// The real tool `name` a test must plant by name, such as `git` for a
/// compile: the file the `PATH` this process was started with finds, or
/// else the file in the system's own directories, so a run of these tests
/// under an empty `PATH` still finds it.
pub(super) fn host_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let system = ["/usr/bin", "/bin", "/usr/local/bin", "/opt/homebrew/bin"].map(PathBuf::from);
    std::env::split_paths(&path)
        .chain(system)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("`{name}` is on this host's PATH or in /usr/bin"))
}

/// The isolation proof itself (#1137h, I1 and I2), and the demand
/// [`pidns::REQUIRE`] makes (I3).
#[cfg(target_os = "linux")]
mod tests {
    use super::pidns::{demanded, identity_at, unisolated, Demand, Unavailable, Undeclared};
    use super::*;

    /// I1: the host's own namespace, compared with itself, reads as not
    /// isolated, and any other reads as apart. Each reason's text is
    /// pinned here once, through `std::error::Error`.
    #[test]
    fn the_hosts_own_pid_namespace_reads_as_not_isolated() {
        let own = pidns::identity().unwrap();
        assert_eq!(apart(own, own), Err(Isolation::Shared(own)));
        assert_eq!(apart(own ^ 1, own), Ok(()));
        let reasons = [
            Isolation::Shared(7),
            Isolation::Unreadable("/nowhere: entity not found".into()),
            Isolation::Unparsable("pid:[7a]".into()),
        ];
        assert_eq!(
            reasons.map(|reason| Box::<dyn std::error::Error>::from(reason).to_string()),
            [
                "it runs in its parent's PID namespace, pid:[7]",
                "a PID namespace identity is unreadable: /nowhere: entity not found",
                "a PID namespace identity reads \"pid:[7a]\", not pid:[N]",
            ]
        );
    }

    /// Z2: an identity is read and parsed as `pid:[N]`, N decimal digits:
    /// one unreadable or spelled any other way refuses, never reads as a
    /// namespace apart.
    #[test]
    fn an_identity_that_cannot_be_read_or_parsed_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let link = |name: &str, target: &str| {
            let link = dir.path().join(name);
            std::os::unix::fs::symlink(target, &link).unwrap();
            identity_at(&link)
        };
        assert_eq!(link("read", "pid:[4026531836]"), Ok(4026531836));
        for spelled in [
            "pid:[]",
            "pid:[+5]",
            "pid:[12a]",
            "net:[5]",
            "pid:[5",
            "pid:[99999999999999999999]",
        ] {
            let read = link(&spelled.replace(['/', ':'], "_"), spelled);
            assert_eq!(
                read,
                Err(Isolation::Unparsable(spelled.into())),
                "{spelled}"
            );
        }
        let missing = dir.path().join("missing");
        let unreadable = format!(
            "{}: No such file or directory (os error 2)",
            missing.display()
        );
        assert_eq!(
            identity_at(&missing),
            Err(Isolation::Unreadable(unreadable))
        );
    }

    /// I2: a child re-run with no namespace at all, in its parent's, is
    /// refused at the proof, before it strays: it fails with the reason,
    /// its body never runs, and the sentinel outside is untouched, which
    /// [`guarded`] judges first.
    #[test]
    fn a_child_in_its_parents_pid_namespace_refuses_before_it_delivers_anything() {
        if rerunning() {
            alone();
            panic!("the body ran in its parent's PID namespace");
        }
        let Reran { passed, said, .. } = guarded(&[], &[]);
        let own = pidns::identity().unwrap();
        let refused = format!("{}: {}", pidns::REFUSED, Isolation::Shared(own));
        assert!(!passed && said.contains(&refused), "{said}");
        assert!(!said.contains("the body ran"), "{said}");
    }

    /// I3: only `1` requires a namespace, and only absence skips.
    #[test]
    fn only_a_declared_one_requires_a_pid_namespace() {
        assert_eq!(demanded(None), Ok(Demand::Optional));
        assert_eq!(demanded(Some("1".as_ref())), Ok(Demand::Required));
        unisolated(Demand::Optional, &Unavailable::NoBwrap("no bwrap".into()));
    }

    /// I5: why no namespace can be built is typed, and each reason's text
    /// is pinned here once.
    #[test]
    fn each_reason_no_namespace_can_be_built_says_why() {
        use std::os::unix::process::ExitStatusExt;
        let bwrap = "/usr/bin/bwrap".to_owned();
        let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let status = std::process::ExitStatus::from_raw(1 << 8);
        let said = "No permissions to create new namespace".to_owned();
        let reasons = [
            Unavailable::NoBwrap("bwrap not on PATH".into()),
            Unavailable::DoesNotRun {
                bwrap: bwrap.clone(),
                error,
            },
            Unavailable::Refused {
                bwrap,
                status,
                said,
            },
        ];
        assert_eq!(
            reasons.map(|reason| Box::<dyn std::error::Error>::from(reason).to_string()),
            [
                "bwrap not on PATH",
                "/usr/bin/bwrap does not run: permission denied",
                "/usr/bin/bwrap cannot build one here (exit status: 1): No permissions to create \
                 new namespace",
            ]
        );
    }

    /// I3 and Z3: a value that is neither, empty included, is refused,
    /// never read as a skip, and says why.
    #[test]
    fn a_demand_that_is_neither_one_nor_absent_is_refused() {
        let refused = ["", "yes", "0"].map(|value| demanded(Some(value.as_ref())));
        assert_eq!(
            refused,
            ["", "yes", "0"].map(|value| Err(Undeclared(value.into())))
        );
        assert_eq!(
            Box::<dyn std::error::Error>::from(Undeclared("".into())).to_string(),
            "BROKKR_REQUIRE_PIDNS is `1` or absent, not \"\""
        );
    }

    /// I3: where a namespace is required, one this host cannot build fails
    /// the test with its reason.
    #[test]
    #[should_panic(
        expected = "BROKKR_REQUIRE_PIDNS is 1, so this host must build it: a test \
                               that delivers a real signal needs a PID namespace of its own, \
                               and none can be built here: bwrap not on PATH"
    )]
    fn a_required_namespace_this_host_cannot_build_fails_with_its_reason() {
        let unavailable = Unavailable::NoBwrap("bwrap not on PATH".into());
        unisolated(Demand::Required, &unavailable);
    }
}
