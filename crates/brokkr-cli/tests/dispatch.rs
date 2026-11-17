//! Decision 0110 rulings 3 and 4 through the built binary, on Linux and
//! macOS: `brokkr dispatch --once` hands each run it starts to this binary
//! as `dispatch drive`, a process in a session of its own whose stdin is
//! `/dev/null` and whose stdout and stderr are the run's own log; and a
//! dispatcher whose binary was replaced claims nothing and ends in ruling
//! 4's words. Every journal, HOME, host file and recipe is the test's own.
//!
//! Each test runs in a process of its own (`in_its_own_engine`), so the
//! driver a pass leaves behind is never adopted by a test binary another
//! test made a subreaper.

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use brokkr_protocol::process::in_its_own_engine;
use serde_json::json;

/// The longest a test waits on a driver.
const PATIENCE: Duration = Duration::from_secs(120);

/// Work, then the review every bundle keeps, then done.
fn policy() -> serde_json::Value {
    let rule = |id: &str, from: &str, result: &str, next: &str| json!({"id": id, "from": from, "result": result, "next": next, "reason": id});
    json!({"phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
           "rules": [rule("worked", "work", "complete", "review"),
                     rule("reviewed", "review", "clean", "done")]})
}

/// A temporary machine: its workspace, a provider-free bundle whose one
/// seat marks `working`, then waits for `released` before it answers, its HOME, state, temp
/// directory and host file, and a journal in it.
struct Machine {
    _dir: tempfile::TempDir,
    base: PathBuf,
    work: PathBuf,
    bundle: PathBuf,
    journal: PathBuf,
}

impl Machine {
    fn new() -> Machine {
        let dir = tempfile::tempdir().unwrap();
        // macOS hands temporary paths under /var, a link to /private/var.
        let base = std::fs::canonicalize(dir.path()).unwrap();
        for owned in ["home", "state", "tmp"] {
            std::fs::create_dir_all(base.join(owned)).unwrap();
            let mode = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(base.join(owned), mode).unwrap();
        }
        let work = base.join("work");
        std::fs::create_dir_all(work.join("recipes/mark")).unwrap();
        std::fs::write(work.join("recipes/mark/bundle.json"), "{}").unwrap();
        let machine = Machine {
            bundle: base.join("bundle"),
            journal: base.join("forge.db"),
            work,
            base,
            _dir: dir,
        };
        machine.host();
        machine.seat();
        machine
    }

    /// A `forge.host/v4` file admitting the `exec` provider.
    fn host(&self) {
        let host = json!({"schema": "forge.host/v4", "providers": {"exec": {"ceiling": 4}},
                          "floors": [{"place": "temp", "floor_bytes": 1}],
                          "boxed_builds": {"ceiling": 1}, "runs": {"ceiling": 3}});
        let at = self.base.join("config/brokkr");
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("host.json"), host.to_string()).unwrap();
    }

    /// The bundle: one unboxed `exec` seat that marks `working`, waits for
    /// `released`, then
    /// answers `complete` in the result file its prompt names, and a
    /// review seat that answers `clean` at once.
    fn seat(&self) {
        std::fs::create_dir_all(&self.bundle).unwrap();
        std::fs::write(self.bundle.join("policy.json"), policy().to_string()).unwrap();
        std::fs::write(self.bundle.join("role.md"), "# seat\n").unwrap();
        let wait = format!(
            ": > '{}'; until [ -e '{}' ]; do sleep 0.05; done",
            self.working().display(),
            self.released().display()
        );
        let (work, review) = (answering(&wait, "complete"), answering(":", "clean"));
        let bundle = json!({"name": "dispatched", "policy": "policy.json",
                            "seats": {"work": work, "review": review}});
        std::fs::write(self.bundle.join("bundle.json"), bundle.to_string()).unwrap();
    }

    /// The file whose presence lets the seat answer.
    fn released(&self) -> PathBuf {
        self.base.join("released")
    }

    /// The file the seat makes as it starts: the driver running it holds
    /// the run's lock.
    fn working(&self) -> PathBuf {
        self.base.join("working")
    }

    /// Wait up to the patience for the seat to start.
    fn seat_runs(&self) {
        let began = Instant::now();
        while !self.working().exists() {
            assert!(began.elapsed() < PATIENCE, "the seat never started");
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// `program` given `args` in the workspace, under this machine's own
    /// HOME, state, temp directory and host file.
    fn command(&self, program: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&self.work)
            .env("HOME", self.base.join("home"))
            .env("XDG_STATE_HOME", self.base.join("state"))
            .env("XDG_CONFIG_HOME", self.base.join("config"))
            .env("TMPDIR", self.base.join("tmp"))
            .stdin(Stdio::null());
        command
    }

    /// `brokkr` given `args`, run to its end.
    fn brokkr(&self, args: &[&str]) -> Output {
        let brokkr = Path::new(env!("CARGO_BIN_EXE_brokkr"));
        self.command(brokkr, args).output().unwrap()
    }

    fn db(&self) -> &str {
        self.journal.to_str().unwrap()
    }

    /// One entry queued, of the bundle.
    fn queued(&self) {
        let bundle = self.bundle.to_str().unwrap();
        let args = [
            "queue",
            "add",
            "--feature",
            "x",
            "--reason",
            "queued",
            "--bundle",
            bundle,
            "--db",
            self.db(),
        ];
        let added = self.brokkr(&args);
        assert!(added.status.success(), "{added:?}");
    }

    /// The commands the queue journaled, by word.
    fn commands(&self) -> Vec<String> {
        let conn = rusqlite::Connection::open(&self.journal).unwrap();
        let mut rows = conn
            .prepare("SELECT command FROM queue_commands ORDER BY seq")
            .unwrap();
        let read = rows.query_map([], |row| row.get(0)).unwrap();
        read.collect::<Result<_, _>>().unwrap()
    }

    /// The run's driver log.
    fn log(&self, run: &str) -> PathBuf {
        self.base.join(format!("forge.db.runs/{run}.log"))
    }
}

/// An unboxed `exec` seat that runs `before`, then answers `result` in the
/// result file its prompt names.
fn answering(before: &str, result: &str) -> serde_json::Value {
    let script = format!(
        "{before}; answer=$(grep -o '/[^ ]*/results/[^ ]*\\.json' \"$1\" | tail -n 1); \
         printf '{{\"result\":\"{result}\",\"inputs\":{{}},\"notes\":\"n\"}}' > \"$answer\""
    );
    let driver = [
        "{brokkr}", "driver", "exec", "--", "sh", "-c", &script, "seat",
    ];
    let mut command = json!(driver);
    command.as_array_mut().unwrap().push(json!("{prompt_file}"));
    json!({"results": [result], "role": "role.md", "driver": {"command": command}})
}

/// The run, entry, claim and driver pid a pass's `started:` line names.
fn started(stdout: &str) -> (String, String, String, i32) {
    let line = (stdout.lines())
        .find(|line| line.starts_with("started: "))
        .unwrap_or_else(|| panic!("no started line in {stdout}"));
    let words: Vec<&str> = line.split([' ', ',']).filter(|w| !w.is_empty()).collect();
    // started: run R under claim S of entry E driver pid P
    let pid = words[words.len() - 1].parse().unwrap();
    (
        words[2].to_string(),
        words[8].to_string(),
        words[5].to_string(),
        pid,
    )
}

/// Wait up to the patience for the driver's `log` to say its run
/// completed.
fn completes(log: &Path) {
    let began = Instant::now();
    loop {
        let said = std::fs::read_to_string(log).unwrap_or_default();
        if said.contains("\"status\": \"completed\"") {
            return;
        }
        assert!(
            began.elapsed() < PATIENCE,
            "the run never completed: {said}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// The session `pid` stands in.
fn session(pid: i32) -> i32 {
    let pid = rustix::process::Pid::from_raw(pid);
    rustix::process::getsid(pid).unwrap().as_raw_nonzero().get()
}

#[test]
fn a_pass_hands_its_run_to_a_driver_of_its_own_that_completes_it() {
    if !in_its_own_engine("dispatch::a_pass_hands_its_run_to_a_driver_of_its_own_that_completes_it")
    {
        return;
    }
    let machine = Machine::new();
    machine.queued();
    let pass = machine.brokkr(&["dispatch", "--once", "--db", machine.db()]);
    let stdout = String::from_utf8_lossy(&pass.stdout).into_owned();
    assert!(pass.status.success(), "{pass:?}");
    assert_eq!(machine.commands(), ["add", "claim", "link", "spawn"]);
    let (run, entry, claim, pid) = started(&stdout);
    assert!(stdout.ends_with("queue: 0 waiting, 0 held\n"), "{stdout}");

    // The driver leads a session of its own, not the dispatcher's, which
    // was this test's.
    assert_eq!(session(pid), pid);
    assert_ne!(session(pid), session(std::process::id() as i32));

    // Its log is this user's own, mode 0600, one link.
    let log = machine.log(&run);
    let read = std::fs::symlink_metadata(&log).unwrap();
    assert_eq!(
        (read.mode() & 0o777, read.uid(), read.nlink()),
        (0o600, rustix::process::getuid().as_raw(), 1)
    );
    #[cfg(target_os = "linux")]
    descriptors(pid, &log, &machine.journal);

    // A second driver of the claim, once the first holds the run (its seat
    // is working), is refused in decision 0108's words, and writes nothing.
    machine.seat_runs();
    let args = [
        "dispatch",
        "drive",
        "--db",
        machine.db(),
        "--entry",
        &entry,
        "--claim",
        &claim,
    ];
    let second = machine.brokkr(&args);
    let said = String::from_utf8_lossy(&second.stderr);
    assert_eq!(second.status.code(), Some(1), "{second:?}");
    assert!(
        said.contains(&format!("run '{run}' is driven by pid {pid}")),
        "{said}"
    );

    std::fs::write(machine.released(), "").unwrap();
    completes(&log);
    assert_eq!(machine.commands(), ["add", "claim", "link", "spawn"]);
}

/// On Linux, the driver's open descriptors: its standard streams, stdin
/// `/dev/null` and stdout and stderr its log, and its own, none of them
/// the dispatcher's lock.
#[cfg(target_os = "linux")]
fn descriptors(pid: i32, log: &Path, journal: &Path) {
    let fds = PathBuf::from(format!("/proc/{pid}/fd"));
    let target = |fd: &str| std::fs::read_link(fds.join(fd)).unwrap();
    assert_eq!(target("0"), Path::new("/dev/null"));
    assert_eq!((target("1"), target("2")), (log.to_owned(), log.to_owned()));
    let lock = PathBuf::from(format!("{}.dispatch.lock", journal.display()));
    for fd in std::fs::read_dir(&fds).unwrap() {
        let at = std::fs::read_link(fd.unwrap().path());
        assert_ne!(
            at.ok(),
            Some(lock.clone()),
            "the driver holds the dispatcher's lock"
        );
    }
}

#[test]
fn a_driver_of_a_claim_with_no_spawn_row_refuses_and_writes_nothing() {
    if !in_its_own_engine(
        "dispatch::a_driver_of_a_claim_with_no_spawn_row_refuses_and_writes_nothing",
    ) {
        return;
    }
    let machine = Machine::new();
    machine.queued();
    let before = std::fs::read(&machine.journal).unwrap();
    let args = [
        "dispatch",
        "drive",
        "--db",
        machine.db(),
        "--entry",
        "1",
        "--claim",
        "2",
    ];
    let refused = machine.brokkr(&args);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert_eq!(
        String::from_utf8_lossy(&refused.stderr),
        "error: dispatch drive: queue entry 1 is not linked under claim 2 with a `spawn` row, \
         so this driver resumes nothing and wrote nothing (decision 0110)\n"
    );
    assert_eq!(std::fs::read(&machine.journal).unwrap(), before);
    assert_eq!(machine.commands(), ["add"]);
}

/// Decision 0110 ruling 4: the dispatcher started from a copy of the
/// binary, the copy replaced while the dispatcher waits on the journal's
/// write lock, claims nothing and ends in the ruling's words.
#[test]
fn a_dispatcher_whose_binary_was_replaced_claims_nothing_and_ends() {
    if !in_its_own_engine(
        "dispatch::a_dispatcher_whose_binary_was_replaced_claims_nothing_and_ends",
    ) {
        return;
    }
    let machine = Machine::new();
    machine.queued();
    let copy = machine.base.join("bin/brokkr");
    stage(Path::new(env!("CARGO_BIN_EXE_brokkr")), &copy);

    // The dispatcher records its binary as it starts, then waits on the
    // journal's write lock, which the test holds until the copy is
    // replaced.
    let holder = rusqlite::Connection::open(&machine.journal).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    let dispatcher = machine
        .command(&copy, &["dispatch", "--once", "--db", machine.db()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let lock = machine.base.join("forge.db.dispatch.lock");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !lock.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    let other = machine.base.join("bin/other");
    std::fs::write(&other, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::rename(&other, &copy).unwrap();
    holder.execute_batch("ROLLBACK").unwrap();

    let ended = dispatcher.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&ended.stdout);
    assert_eq!(ended.status.code(), Some(1), "{ended:?}");
    let words = format!(
        "ended: dispatch: the brokkr at {} is not the one this dispatcher started from; it \
         claimed nothing after; start the dispatcher again\n",
        copy.display()
    );
    assert!(stdout.ends_with(&words), "{stdout}");
    assert_eq!(machine.commands(), ["add"], "nothing claimed");
}

/// A standing dispatcher this test started, killed and waited on if the
/// test fails before it ends, so no dispatcher outlives its test.
struct Standing(Option<std::process::Child>);

impl Standing {
    /// `brokkr dispatch` given `args` on `machine`, its output piped.
    fn start(machine: &Machine, args: &[&str]) -> Standing {
        let brokkr = Path::new(env!("CARGO_BIN_EXE_brokkr"));
        let child = machine
            .command(brokkr, args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        Standing(Some(child))
    }

    /// Send `signal` to the dispatcher's own pid, not this test's group.
    fn signal(&self, signal: rustix::process::Signal) {
        let pid = self.0.as_ref().unwrap().id() as i32;
        let pid = rustix::process::Pid::from_raw(pid).unwrap();
        rustix::process::kill_process(pid, signal).unwrap();
    }

    /// Wait up to the patience for the dispatcher to end, and read what it
    /// said.
    fn ended(mut self) -> Output {
        let mut child = self.0.take().unwrap();
        let began = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if began.elapsed() > PATIENCE {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the dispatcher did not end within the patience");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        child.wait_with_output().unwrap()
    }
}

impl Drop for Standing {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Whether `pid` names a process still there.
fn alive(pid: i32) -> bool {
    let pid = rustix::process::Pid::from_raw(pid).unwrap();
    rustix::process::test_kill_process(pid).is_ok()
}

/// Decision 0110 ruling 5: the standing loop starts the queued entry's
/// driver, and a SIGTERM stops it with exit 0 once that tick is reported.
/// The driver, in a session of its own, is never signalled: it runs on and
/// completes its run, and no claim follows the signal.
#[test]
fn a_standing_dispatcher_stops_on_sigterm_and_its_driver_runs_on() {
    if !in_its_own_engine("dispatch::a_standing_dispatcher_stops_on_sigterm_and_its_driver_runs_on")
    {
        return;
    }
    let machine = Machine::new();
    machine.queued();
    let args = ["dispatch", "--every", "5", "--db", machine.db()];
    let dispatcher = Standing::start(&machine, &args);
    machine.seat_runs();
    dispatcher.signal(rustix::process::Signal::TERM);
    let ended = dispatcher.ended();
    let stdout = String::from_utf8_lossy(&ended.stdout).into_owned();
    assert_eq!(ended.status.code(), Some(0), "{ended:?}");
    let (run, _, _, pid) = started(&stdout);
    let last = format!(
        "queue: 0 waiting, 0 held\nleft running: run {run}, driver pid {pid}\nstopped: a signal \
         asked the dispatcher to stop; it claimed nothing after, and its drivers run on\n"
    );
    assert!(stdout.ends_with(&last), "{stdout}");
    assert!(alive(pid), "the driver was not signalled");
    assert_eq!(machine.commands(), ["add", "claim", "link", "spawn"]);

    std::fs::write(machine.released(), "").unwrap();
    completes(&machine.log(&run));
    assert_eq!(machine.commands(), ["add", "claim", "link", "spawn"]);
}

/// Decision 0110 ruling 5: a second signal while the first's stop is asked
/// ends the dispatcher at once, 128 plus SIGTERM's number. The dispatcher
/// is held on the journal's write lock, so the first stop cannot end it
/// before the second arrives.
#[test]
fn a_second_sigterm_ends_the_dispatcher_at_once() {
    if !in_its_own_engine("dispatch::a_second_sigterm_ends_the_dispatcher_at_once") {
        return;
    }
    let machine = Machine::new();
    machine.queued();
    let holder = rusqlite::Connection::open(&machine.journal).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    let args = ["dispatch", "--every", "5", "--db", machine.db()];
    let dispatcher = Standing::start(&machine, &args);
    // Its handlers are installed before it takes the lock file.
    let lock = machine.base.join("forge.db.dispatch.lock");
    let began = Instant::now();
    while !lock.exists() {
        assert!(
            began.elapsed() < PATIENCE,
            "the dispatcher never took its lock"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    dispatcher.signal(rustix::process::Signal::TERM);
    std::thread::sleep(Duration::from_millis(300));
    dispatcher.signal(rustix::process::Signal::TERM);
    let ended = dispatcher.ended();
    assert_eq!(ended.status.code(), Some(143), "{ended:?}");
    holder.execute_batch("ROLLBACK").unwrap();
    assert_eq!(machine.commands(), ["add"], "nothing claimed");
}

/// `from` copied to `to` by a staging name renamed into place, so no
/// writer of `to` can make its exec busy (#255).
fn stage(from: &Path, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    let staged = to.with_extension("staged");
    std::fs::copy(from, &staged).unwrap();
    std::fs::rename(&staged, to).unwrap();
}
