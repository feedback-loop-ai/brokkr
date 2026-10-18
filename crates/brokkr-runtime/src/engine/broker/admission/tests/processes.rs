//! The writer lease's lifecycle across real engines, each in a process of
//! its own (decision 0065 slice two, U8a2a; CR2, MB3; M4 of run four's
//! second review): an already-running writer refuses a second run on its
//! worktree and a disjoint worktree runs beside it; of two runs started
//! together on one worktree exactly one runs; and an engine killed
//! mid-attempt, its driver surviving it, keeps its worktree reserved
//! through the lock's release, the driver's own end and the run's resume.
//! Moved here from `capability_broker_launch` and `capability_ledger`,
//! which now hold the production resolution proofs from HOME alone.
//!
//! Each engine is this unit-test binary run again on [`engine_child`], told
//! its journal, bundle, worktree and HOME through its environment, which
//! that test entry reads and stands in on its own thread as every fixture
//! here stands one in ([`Coordinated::at`]): a coordination root opened
//! with no ancestor judged, in a fresh owner-only directory on any TMPDIR,
//! so none of these is ever skipped. Production reads no such variable.
//! Nothing here reads or writes the host's HOME. Every wait on a child is
//! bounded, and a child still running when its test unwinds is killed and
//! reaped (M6).

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};

use brokkr_core::EventEnvelope;
use brokkr_store::Store;

use super::*;
use crate::{Bundle, Engine};

const DB: &str = "BROKKR_WRITERS_DB";
const BUNDLE: &str = "BROKKR_WRITERS_BUNDLE";
const WORKTREE: &str = "BROKKR_WRITERS_WORKTREE";
const RUN: &str = "BROKKR_WRITERS_RUN";
const HOME: &str = "BROKKR_WRITERS_HOME";

/// The child entry every engine here is run on.
const CHILD: &str = "engine::broker::admission::tests::processes::engine_child";

/// A `work` phase, the protected `review` and done.
const POLICY: &str = r#"{"phases": ["work", "review", "done"], "initial": "work",
  "terminal": ["done"],
  "rules": [{"id": "WORK-DONE", "from": "work", "result": "complete", "next": "review",
             "reason": "worked"},
            {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
             "reason": "clean"}]}"#;

/// CR2's refusal of a worktree another run holds, as dispatch journals it.
fn held_by(run: &str) -> String {
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
enum Unnamed {
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
struct Running {
    run: String,
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
    fn ended(mut self) -> String {
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
    fn killed(mut self) {
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

/// A HOME its engines' coordination root is opened in, the worktrees
/// beside it, and the bundles and journals its engines run.
struct Coordination {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Coordination {
    /// A fresh owner-only directory in TMPDIR, canonical, whatever TMPDIR's
    /// ancestors are, holding an owner-only `home` ([`fresh`]).
    fn new() -> Coordination {
        let dir = fresh(&base());
        let root = dir.path().to_path_buf();
        Coordination { _dir: dir, root }
    }

    /// Where `run` of the worktree `worktree` keeps its reservation.
    fn reservation(&self, worktree: &Path, run: &str) -> PathBuf {
        let key = canonical::sha256_bytes(worktree.as_os_str().as_bytes());
        let root = coordination(&self.root.join("home"));
        root.join(key).join(run).join(RESERVATION)
    }

    /// The worktree `name`, made.
    fn worktree(&self, name: &str) -> PathBuf {
        let worktree = self.root.join(name);
        std::fs::create_dir_all(&worktree).unwrap();
        worktree
    }

    /// Where the driver of the bundle `name` leaves its mark once it
    /// starts.
    fn mark(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.ran"))
    }

    /// The file a waiting driver waits for.
    fn gate(&self) -> PathBuf {
        self.root.join("gate")
    }

    /// The bundle `name`: a `work` seat whose driver leaves its mark,
    /// waits for the gate where `waits`, then succeeds, and a review that
    /// is clean; one attempt each.
    fn bundle(&self, name: &str, waits: bool) -> PathBuf {
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
    fn start(&self, worktree: &Path, bundle: &Path) -> Running {
        self.engine(worktree, bundle, None)
    }

    /// The engine of `run` resumed, as [`Coordination::start`] runs it.
    fn resume(&self, worktree: &Path, bundle: &Path, run: &str) -> Running {
        self.engine(worktree, bundle, Some(run))
    }

    fn engine(&self, worktree: &Path, bundle: &Path, run: Option<&str>) -> Running {
        let journal = bundle.with_extension("db");
        let home = self.root.join("home");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", CHILD, "--nocapture"]);
        command.env("HOME", &home).env(HOME, &home);
        command.env(DB, journal).env(BUNDLE, bundle);
        command.env(WORKTREE, worktree).env_remove(RUN);
        run.map(|run| command.env(RUN, run));
        Running::started(command, PATIENCE).unwrap()
    }

    /// Every event `run` of `bundle` journaled.
    fn events(&self, bundle: &Path, run: &str) -> Vec<EventEnvelope> {
        Store::open(&bundle.with_extension("db"))
            .unwrap()
            .load(run)
            .unwrap()
    }

    /// The error of every attempt `run` of `bundle` failed.
    fn failures(&self, bundle: &Path, run: &str) -> Vec<String> {
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
    fn standing(&self, worktree: &Path, run: &str) -> Vec<String> {
        let bytes = std::fs::read(self.reservation(worktree, run)).unwrap();
        let held: Reservation = serde_json::from_slice(&bytes).unwrap();
        held.attempts.into_keys().collect()
    }
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
fn until(what: &str, holds: impl Fn() -> bool) {
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

/// One engine, started or resumed, in its own process, its coordination
/// root opened in the HOME it is told, as every fixture here stands one in:
/// inert without [`DB`], so a plain run of the suite passes over it.
#[test]
fn engine_child() {
    let (Ok(db), Ok(bundle), Ok(worktree), Ok(home)) = (
        std::env::var(DB),
        std::env::var(BUNDLE),
        std::env::var(WORKTREE),
        std::env::var(HOME),
    ) else {
        return;
    };
    let _stood = Coordinated::at(Path::new(&home));
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

/// An already-running writer refuses: while one run's driver works its
/// worktree, a second run there fails its attempt by CR2's cause before its
/// driver starts, and a run on another worktree completes beside it. Once
/// the first settles, its lease is gone and the next run on its worktree
/// completes, each reservation holding no attempt standing.
#[test]
fn a_running_writer_holds_its_worktree_and_a_disjoint_one_runs_beside_it() {
    let host = Coordination::new();
    let (shared, apart) = (host.worktree("shared"), host.worktree("apart"));
    let (holding, refused) = (host.bundle("holding", true), host.bundle("refused", false));
    let (beside, after) = (host.bundle("beside", false), host.bundle("after", false));
    let first = host.start(&shared, &holding);
    let lease = host.reservation(&shared, &first.run);
    until("the first driver runs under its lease", || {
        host.mark("holding").exists() && lease.exists()
    });

    let second = host.start(&shared, &refused);
    let run = second.run.clone();
    assert_eq!(second.ended(), "awaiting_operator");
    assert_eq!(host.failures(&refused, &run), [held_by(&first.run)]);
    assert!(!host.mark("refused").exists(), "its driver never started");

    let third = host.start(&apart, &beside);
    assert_eq!(third.ended(), "completed");
    assert_eq!(
        host.standing(&shared, &first.run).len(),
        1,
        "the first holds its worktree"
    );

    std::fs::write(host.gate(), "").unwrap();
    let first_run = first.run.clone();
    assert_eq!(first.ended(), "completed");
    assert!(
        host.standing(&shared, &first_run).is_empty(),
        "released once settled"
    );
    let next = host.start(&shared, &after);
    let next_run = next.run.clone();
    assert_eq!(next.ended(), "completed");
    assert!(host.standing(&shared, &next_run).is_empty());
}

/// Simultaneous starts on one worktree admit exactly one: of two runs
/// whose drivers both wait, one holds the lease and runs, and the other
/// fails by CR2's cause naming it, its driver never started.
#[test]
fn of_two_runs_started_together_on_one_worktree_exactly_one_runs() {
    let host = Coordination::new();
    let shared = host.worktree("shared");
    let (left, right) = (host.bundle("left", true), host.bundle("right", true));
    let started = [host.start(&shared, &left), host.start(&shared, &right)];
    let runs = [started[0].run.clone(), started[1].run.clone()];
    until("one of them is refused", || {
        let failed = |at: usize| !host.failures([&left, &right][at], &runs[at]).is_empty();
        failed(0) || failed(1)
    });
    std::fs::write(host.gate(), "").unwrap();
    let [first, second] = started;
    let ends = [first.ended(), second.ended()];
    let refused = usize::from(ends[0] == "completed");
    assert_eq!(ends[1 - refused], "completed", "{ends:?}");
    assert_eq!(ends[refused], "awaiting_operator");
    let bundles = [&left, &right];
    assert_eq!(
        host.failures(bundles[refused], &runs[refused]),
        [held_by(&runs[1 - refused])]
    );
    let marks = [host.mark("left"), host.mark("right")];
    assert!(!marks[refused].exists() && marks[1 - refused].exists());
}

/// Engine death with surviving payload retains the reservation: the
/// killed engine's lease stands byte for byte, refusing a second run on
/// its worktree by CR2's cause while its driver still runs, after that
/// driver ends, and after the run is resumed and settles the attempt it
/// lost as indeterminate.
#[test]
fn an_engine_killed_mid_attempt_keeps_its_worktree_reserved() {
    let host = Coordination::new();
    let shared = host.worktree("shared");
    let lost = host.bundle("lost", true);
    let first = host.start(&shared, &lost);
    let run = first.run.clone();
    let lease = host.reservation(&shared, &run);
    until("the first driver runs under its lease", || {
        host.mark("lost").exists() && lease.exists()
    });
    let sealed = std::fs::read(&lease).unwrap();
    first.killed();

    let refused = |name: &str| {
        let bundle = host.bundle(name, false);
        let second = host.start(&shared, &bundle);
        let second_run = second.run.clone();
        assert_eq!(second.ended(), "awaiting_operator", "{name}");
        assert_eq!(
            host.failures(&bundle, &second_run),
            [held_by(&run)],
            "{name}"
        );
        assert!(
            !host.mark(name).exists(),
            "{name}: its driver never started"
        );
        assert_eq!(
            std::fs::read(&lease).unwrap(),
            sealed,
            "{name}: the lease as sealed"
        );
    };
    refused("while-it-runs");
    // The surviving driver ends: its tree is gone, but nothing journaled
    // says so.
    std::fs::write(host.gate(), "").unwrap();
    refused("once-it-ended");
    assert_eq!(
        host.resume(&shared, &lost, &run).ended(),
        "awaiting_operator"
    );
    refused("after-the-resume");
}
