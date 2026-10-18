//! Engines run in processes of their own on one protected HOME, each
//! operating a worktree beside it, for the writer lease's two suites
//! (decision 0065 slice two, U8a2a; CR2, MB3): `capability_broker_launch`
//! and `capability_ledger`. Each engine is this test binary run again on
//! [`engine_child`], told its journal, bundle and worktree through its
//! environment, with the coordination HOME as its own. Nothing here reads
//! or writes the host's HOME. Every wait on a child is bounded, and a
//! child still running when its test unwinds is killed and reaped (M6).
//!
//! These are the suites' production resolution proofs (M5): an engine in
//! a process of its own is the public engine, which resolves its
//! coordination root from HOME through the protected walk, judging every
//! ancestor, as an installed `brokkr` does; no seam reaches across the
//! process. So each needs a HOME whose ancestors protect, and is skipped
//! with its reason where this host has none. The behaviour each proves is
//! proved again, on every TMPDIR and never skipped, by the in-crate suites
//! on an injected coordination root (`engine::broker::admission::tests`).

use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_runtime::{Bundle, Engine};
use brokkr_store::Store;
use serde_json::json;

const DB: &str = "BROKKR_WRITERS_DB";
const BUNDLE: &str = "BROKKR_WRITERS_BUNDLE";
const WORKTREE: &str = "BROKKR_WRITERS_WORKTREE";
const RUN: &str = "BROKKR_WRITERS_RUN";

/// A `work` phase, the protected `review` and done.
const POLICY: &str = r#"{"phases": ["work", "review", "done"], "initial": "work",
  "terminal": ["done"],
  "rules": [{"id": "WORK-DONE", "from": "work", "result": "complete", "next": "review",
             "reason": "worked"},
            {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
             "reason": "clean"}]}"#;

/// CR2's refusal of a worktree another run holds, as dispatch journals it.
pub(crate) fn held_by(run: &str) -> String {
    format!(
        "dispatch refused: capability evidence root cannot be protected from every managed \
         writer: run '{run}' holds this worktree, its writers not proven settled"
    )
}

/// The longest any child engine is waited on, for its run's name or its
/// end: far past what one run here takes, and far short of forever.
const PATIENCE: Duration = Duration::from_secs(120);

/// Why a child engine was given up on before it named its run (R7 of run
/// four's review), with what it printed until then; the child is killed
/// and reaped either way.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Unnamed {
    #[error("the engine named no run within its patience: {0}")]
    Timeout(String),
    #[error("the engine ended before it named its run: {0}")]
    Ended(String),
}

/// One run of an engine in a process of its own: the run, the child still
/// running or ended, the lines of its standard output as a thread of
/// their own reads them, and how long any wait on it may last. Dropped
/// while the child still runs — a test that unwinds — it is killed and
/// reaped.
pub(crate) struct Running {
    pub(crate) run: String,
    child: Child,
    lines: Receiver<String>,
    patience: Duration,
}

impl Running {
    /// `command` started, its standard output read line by line on a
    /// thread of its own, and read until a line names its run, all within
    /// `patience`; why not where it names none in time or ends first, the
    /// child then killed and reaped.
    fn started(mut command: Command, patience: Duration) -> Result<Running, Unnamed> {
        let mut child = (command.stdout(Stdio::piped()).stderr(Stdio::null()))
            .spawn()
            .unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let (sent, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                if sent.send(line).is_err() {
                    break;
                }
            }
        });
        let run = String::new();
        let mut running = Running {
            run,
            child,
            lines,
            patience,
        };
        let deadline = Instant::now() + patience;
        let mut seen = String::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = running
                .lines
                .recv_timeout(left)
                .map_err(|error| match error {
                    RecvTimeoutError::Timeout => Unnamed::Timeout(seen.clone()),
                    RecvTimeoutError::Disconnected => Unnamed::Ended(seen.clone()),
                })?;
            if let Some(run) = line.strip_prefix("run_id=") {
                running.run = run.to_string();
                return Ok(running);
            }
            seen.push_str(&line);
            seen.push('\n');
        }
    }

    /// Wait, within its patience, for the engine to end on its own; the
    /// word its run's status ended as.
    pub(crate) fn ended(mut self) -> String {
        let deadline = Instant::now() + self.patience;
        let mut lines = String::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => {
                    lines.push_str(&line);
                    lines.push('\n');
                }
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {
                    panic!("the engine of {} never ended: {lines}", self.run)
                }
            }
        }
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            let late = Instant::now() >= deadline;
            assert!(!late, "the engine of {} never exited: {lines}", self.run);
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(
            status.success(),
            "the engine of {} exited {status}: {lines}",
            self.run
        );
        let ended = lines.lines().find_map(|line| line.strip_prefix("status="));
        let ended = ended.unwrap_or_else(|| panic!("{} printed no end: {lines}", self.run));
        ended.to_string()
    }

    /// Kill the engine outright, as a host that loses it would.
    pub(crate) fn killed(mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

impl Drop for Running {
    /// A child still running is killed; every child is reaped.
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// A protected HOME, the worktrees beside it, and the bundles and journals
/// its engines run.
pub(crate) struct Coordination {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Coordination {
    /// A temporary root, owner-only and canonical, holding an owner-only
    /// `home`, made where every ancestor protects a HOME, as production's
    /// resolution of the coordination root from HOME asks (D6): the
    /// target's own temporary directory, whatever TMPDIR names, else
    /// TMPDIR, `/tmp` or `/var/tmp`. None where no such location exists or
    /// none can be made in, the test then skipped with its reason (or
    /// failed where this host must produce the evidence): an engine here
    /// would hold no lease, and nothing these suites prove would be.
    pub(crate) fn new() -> Option<Coordination> {
        let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let candidates = [
            target,
            std::env::temp_dir(),
            "/tmp".into(),
            "/var/tmp".into(),
        ];
        let canonical = candidates
            .iter()
            .filter_map(|at| std::fs::canonicalize(at).ok());
        let mut protecting = canonical.filter(|at| protects(at));
        let Some(dir) = protecting.find_map(|at| tempfile::tempdir_in(at).ok()) else {
            let required = brokkr_protocol::hands::boundary_evidence_required();
            let reason = "no protected location for a HOME (TMPDIR, /tmp, /var/tmp; D6)";
            brokkr_protocol::hands::skip_boundary_proof(required, reason);
            return None;
        };
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let private = || std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&root, private()).unwrap();
        std::fs::create_dir(root.join("home")).unwrap();
        std::fs::set_permissions(root.join("home"), private()).unwrap();
        Some(Coordination { _dir: dir, root })
    }

    /// The protected root under the HOME.
    pub(crate) fn capabilities(&self) -> PathBuf {
        self.root.join("home/.local/state/brokkr/capabilities")
    }

    /// Where `run` of the worktree `worktree` keeps its reservation.
    pub(crate) fn reservation(&self, worktree: &Path, run: &str) -> PathBuf {
        let key = brokkr_core::canonical::sha256_bytes(worktree.as_os_str().as_encoded_bytes());
        self.capabilities()
            .join(key)
            .join(run)
            .join("reservation.json")
    }

    /// The worktree `name`, made.
    pub(crate) fn worktree(&self, name: &str) -> PathBuf {
        let worktree = self.root.join(name);
        std::fs::create_dir_all(&worktree).unwrap();
        worktree
    }

    /// Where the driver of the bundle `name` leaves its mark once it
    /// starts.
    pub(crate) fn mark(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.ran"))
    }

    /// The file a waiting driver waits for.
    pub(crate) fn gate(&self) -> PathBuf {
        self.root.join("gate")
    }

    /// The bundle `name`: a `work` seat whose driver leaves its mark,
    /// waits for the gate where `waits`, then succeeds, and a review that
    /// is clean; one attempt each.
    pub(crate) fn bundle(&self, name: &str, waits: bool) -> PathBuf {
        let bundle = self.root.join(format!("bundles/{name}"));
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
        // A driver whose test unwound, its root removed, stops waiting.
        let gate = self.gate();
        let wait = format!(
            "while [ ! -e '{}' ] && [ -d '{}' ]; do sleep 0.05; done",
            gate.display(),
            self.root.display()
        );
        let before = format!(
            "touch '{}'; {}",
            self.mark(name).display(),
            if waits { &wait } else { ":" }
        );
        let seat = |result: &str, before: &str| {
            json!({"role": "roles/role.md", "results": [result],
                   "limits": {"max_attempts": 1, "timeout_seconds": 120},
                   "driver": {"command": ["sh", "-c", driver(before, result)]}})
        };
        let seats = json!({"work": seat("complete", &before), "review": seat("clean", ":")});
        let document = json!({"name": name, "policy": "policy.json", "seats": seats});
        std::fs::write(bundle.join("bundle.json"), document.to_string()).unwrap();
        bundle
    }

    /// The engine of a new run of `bundle` on `worktree`, journaled in a
    /// journal of its bundle's own, started in a process of its own and
    /// read until it names its run.
    pub(crate) fn start(&self, worktree: &Path, bundle: &Path) -> Running {
        self.engine(worktree, bundle, None)
    }

    /// The engine of `run` resumed, as [`Coordination::start`] runs it.
    pub(crate) fn resume(&self, worktree: &Path, bundle: &Path, run: &str) -> Running {
        self.engine(worktree, bundle, Some(run))
    }

    fn engine(&self, worktree: &Path, bundle: &Path, run: Option<&str>) -> Running {
        let journal = bundle.with_extension("db");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "writers::engine_child", "--nocapture"]);
        command
            .env("HOME", self.root.join("home"))
            .env(DB, journal)
            .env(BUNDLE, bundle);
        command.env(WORKTREE, worktree).env_remove(RUN);
        run.map(|run| command.env(RUN, run));
        Running::started(command, PATIENCE).unwrap()
    }

    /// Every event `run` of `bundle` journaled.
    pub(crate) fn events(&self, bundle: &Path, run: &str) -> Vec<EventEnvelope> {
        Store::open(&bundle.with_extension("db"))
            .unwrap()
            .load(run)
            .unwrap()
    }

    /// The error of every attempt `run` of `bundle` failed.
    pub(crate) fn failures(&self, bundle: &Path, run: &str) -> Vec<String> {
        let events = self.events(bundle, run);
        let failed = events
            .iter()
            .filter(|event| event.event_type == EventType::EffectFailed);
        failed
            .map(|event| event.payload["error"].as_str().unwrap().to_string())
            .collect()
    }

    /// The attempts `run` of `worktree` holds standing, by its
    /// reservation, which keeps its settled attempts' names beside them.
    pub(crate) fn standing(&self, worktree: &Path, run: &str) -> Vec<String> {
        let bytes = std::fs::read(self.reservation(worktree, run)).unwrap();
        let held: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let attempts = held["attempts"].as_object().unwrap();
        attempts.keys().cloned().collect()
    }
}

/// Whether `at` and every ancestor of it protect what lies below them:
/// this user's or root's, writable by no one else unless root-owned and
/// sticky, as the broker judges a HOME's ancestors.
fn protects(at: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let user = brokkr_protocol::hands::ids().0;
    let judged = |at: &Path| {
        std::fs::metadata(at).is_ok_and(|found| {
            let (owner, mode) = (found.uid(), found.mode());
            let sticky = (owner == 0) & (mode & 0o1000 != 0);
            ((owner == user) | (owner == 0)) & ((mode & 0o022 == 0) | sticky)
        })
    };
    at.ancestors().all(judged)
}

/// M6: supervision is bounded and kills what it gives up on. A child that
/// names no run within its patience is given up on at once, killed and
/// reaped; one that named its run and then never ends fails the wait for
/// its end, and its test's unwinding kills and reaps it too.
#[test]
fn a_stalled_engine_is_given_up_on_killed_and_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let pid = dir.path().join("pid");
    let stalled = |named: &str| {
        let script = format!("echo $$ > '{}'; {named} exec sleep 600", pid.display());
        let mut command = Command::new("sh");
        command.args(["-c", &script]);
        command
    };
    let patience = Duration::from_millis(300);
    let began = Instant::now();
    let refused = Running::started(stalled(""), patience).err();
    assert_eq!(refused, Some(Unnamed::Timeout(String::new())));
    let said = Unnamed::Timeout("seen\n".into()).to_string();
    assert_eq!(said, "the engine named no run within its patience: seen\n");
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "within its patience"
    );
    assert!(reaped(&pid), "killed and reaped");
    let running = Running::started(stalled("echo run_id=r;"), patience).unwrap();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| running.ended()));
    let why = unwound.unwrap_err().downcast::<String>().unwrap();
    assert_eq!(*why, "the engine of r never ended: ");
    assert!(reaped(&pid), "killed and reaped on unwind");
}

/// Whether the process whose id the file `pid` holds is gone, reaped: no
/// process holds that id (L5 of run four's review), not merely one this
/// test may not signal.
fn reaped(pid: &Path) -> bool {
    let pid = std::fs::read_to_string(pid).unwrap();
    let pid = rustix::process::Pid::from_raw(pid.trim().parse().unwrap()).unwrap();
    rustix::process::test_kill_process(pid) == Err(rustix::io::Errno::SRCH)
}

/// Wait up to thirty seconds for `holds`.
pub(crate) fn until(what: &str, holds: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !holds() {
        assert!(Instant::now() < deadline, "{what} never held");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A driver's script that runs `before` once started and then succeeds
/// with `result`, echoing the ids its start names.
fn driver(before: &str, result: &str) -> String {
    let capabilities = json!({"proto": "forge-driver/v1", "msg_id": "cap", "type": "capabilities",
                              "driver": "test", "version": "1", "supports": []});
    format!(
        r#"read -r hello
printf '%s\n' '{capabilities}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
{before}
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{{"result":"{result}"}},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#
    )
}

/// One engine, started or resumed, in its own process: inert without
/// [`DB`], so a plain run of the suite passes over it.
#[test]
fn engine_child() {
    let (Ok(db), Ok(bundle), Ok(worktree)) = (
        std::env::var(DB),
        std::env::var(BUNDLE),
        std::env::var(WORKTREE),
    ) else {
        return;
    };
    let bundle = Bundle::compile(Path::new(&bundle)).unwrap();
    let store = Store::open(Path::new(&db)).unwrap();
    let worktree = Some(PathBuf::from(worktree));
    let mut engine = match std::env::var(RUN) {
        Ok(run) => Engine::resume(store, bundle, &run, worktree).unwrap(),
        Err(_) => Engine::start(store, bundle, "Feature: writers", worktree).unwrap(),
    };
    println!("run_id={}", engine.run_id);
    let end = engine.drive().unwrap();
    println!("status={}", end.state.run.status().as_str());
}
