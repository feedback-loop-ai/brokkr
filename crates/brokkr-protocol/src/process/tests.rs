use super::*;
use serde_json::json;

fn command(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}

fn wire(body: Body) -> String {
    serde_json::to_string(&Message::new(body)).unwrap()
}

fn run(script: &str) -> AttemptReport {
    DriverProcess::spawn(
        &command(script),
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    )
    .unwrap()
    .run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {})
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("injected writer fault"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct BrokenWriter;

impl std::io::Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "injected closed driver stdin",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "injected closed driver stdin",
        ))
    }
}

#[derive(Default)]
struct BreakAfterFirstFlush {
    flushed: bool,
}

impl std::io::Write for BreakAfterFirstFlush {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.flushed {
            return BrokenWriter.write(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.flushed {
            return BrokenWriter.flush();
        }
        self.flushed = true;
        Ok(())
    }
}

fn process_with_writer(writer: impl std::io::Write + 'static) -> DriverProcess {
    let script = format!("printf '%s\\n' '{}'", capabilities());
    let mut process = DriverProcess::spawn(
        &command(&script),
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    )
    .unwrap();
    process.stdin = Box::new(writer);
    process
}

fn capabilities() -> String {
    wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: Vec::new(),
    })
}

#[test]
fn spawn_refuses_empty_and_missing_commands() {
    assert!(matches!(
        DriverProcess::spawn(&[], std::path::Path::new("."), None, &SpawnEnv::Inherit),
        Err(SpawnError::EmptyCommand)
    ));
    let error = match DriverProcess::spawn(
        &["forge-certainly-does-not-exist".into()],
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    ) {
        Ok(_) => panic!("missing executable must fail"),
        Err(error) => error,
    };
    assert!(matches!(error, SpawnError::Spawn { .. }));
}

#[test]
fn handshake_and_eof_defects_fail_closed() {
    let wrong_proto = json!({
        "proto": "other/v1",
        "msg_id": "m",
        "type": "capabilities",
        "driver": "test",
        "version": "1",
        "supports": [],
    });
    for script in [
        format!("read -r line; printf '\\n%s\\n' '{}'", wrong_proto),
        format!(
            "read -r line; printf '%s\\n' '{}'; read -r line",
            wire(Body::Accepted {
                effect_id: "effect".into(),
                attempt_id: "attempt".into(),
                session_ref: None,
            })
        ),
    ] {
        assert!(matches!(
            run(&script).outcome,
            AttemptOutcome::Failed { .. }
        ));
    }

    let before_accept = format!(
        "read -r line; printf '%s\\n' '{}'; read -r line",
        capabilities()
    );
    assert!(matches!(
        run(&before_accept).outcome,
        AttemptOutcome::Indeterminate { reason } if reason.contains("before accepting")
    ));
    let after_accept = format!(
        "read -r line; printf '%s\\n' '{}'; read -r line; printf '%s\\n' '{}'",
        capabilities(),
        wire(Body::Accepted {
            effect_id: "effect".into(),
            attempt_id: "attempt".into(),
            session_ref: Some("session".into()),
        })
    );
    assert!(matches!(
        run(&after_accept).outcome,
        AttemptOutcome::Indeterminate { reason } if reason.contains("after accepting")
    ));
}

#[test]
fn attempt_loop_refuses_foreign_and_malformed_terminal_messages() {
    let cases = [
        Body::Accepted {
            effect_id: "foreign".into(),
            attempt_id: "attempt".into(),
            session_ref: None,
        },
        Body::Checkpoint {
            effect_id: "foreign".into(),
            attempt_id: "attempt".into(),
            data: json!({}),
        },
        Body::Result {
            effect_id: "foreign".into(),
            attempt_id: "attempt".into(),
            status: ResultStatus::Succeeded,
            result: Some(json!({})),
            error: None,
        },
        Body::Cancelled {
            effect_id: "effect".into(),
        },
    ];
    for body in cases {
        let script = format!(
            "read -r line; printf '%s\\n' '{}'; read -r line; printf '%s\\n' '{}'",
            capabilities(),
            wire(body)
        );
        assert!(matches!(
            run(&script).outcome,
            AttemptOutcome::Failed { .. }
        ));
    }

    let no_payload = Body::Result {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        status: ResultStatus::Succeeded,
        result: None,
        error: None,
    };
    let script = format!(
        "read -r line; printf '%s\\n' '{}'; read -r line; printf '%s\\n' '{}'",
        capabilities(),
        wire(no_payload)
    );
    assert!(matches!(
        run(&script).outcome,
        AttemptOutcome::Failed { .. }
    ));

    let failed = Body::Result {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        status: ResultStatus::Failed,
        result: None,
        error: None,
    };
    let script = format!(
        "read -r line; printf '%s\\n' '{}'; read -r line; printf '%s\\n' '{}'",
        capabilities(),
        wire(failed)
    );
    assert!(matches!(
        run(&script).outcome,
        AttemptOutcome::Failed { error } if error == "driver reported failure"
    ));
}

#[test]
fn pipe_and_stdout_failures_are_terminal_reports() {
    let invalid_utf8 = "read -r line; printf '\\377'";
    assert!(matches!(
        run(invalid_utf8).outcome,
        AttemptOutcome::Failed { error } if error.contains("stdout read failed")
    ));

    assert!(matches!(
        run("read -r line").outcome,
        AttemptOutcome::Indeterminate { reason } if reason.contains("before accepting")
    ));

    let report = process_with_writer(BreakAfterFirstFlush::default()).run_attempt(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({}),
        |_| {},
    );
    assert!(matches!(
        report.outcome,
        AttemptOutcome::Failed { error } if error.contains("could not send start")
    ));

    // A pipe broken at the greeting is the driver already gone: the
    // same indeterminate arm as an exit before accepting, never a race
    // over which error text the operator reads.
    let report = process_with_writer(BrokenWriter).run_attempt(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({}),
        |_| {},
    );
    assert!(matches!(
        report.outcome,
        AttemptOutcome::Indeterminate { reason } if reason.contains("before accepting")
    ));

    // Any OTHER write failure at the greeting keeps its own words.
    let report = process_with_writer(FailingWriter).run_attempt(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({}),
        |_| {},
    );
    assert!(matches!(
        report.outcome,
        AttemptOutcome::Failed { error } if error.contains("could not greet driver")
    ));
}

/// A driver that stalls on a read that never comes and leaves a child
/// of its own running behind it — the shape the deadline kill has to
/// take down. The child's stdio is pointed away from the harness's
/// pipes so that a kill which misses it still lets the harness see EOF:
/// this test must fail with an assertion when the tree survives, never
/// hang waiting on the CI job timeout to notice.
///
/// The child writes `born` the moment it starts and `survived` only
/// well after the deadline. `born` is the positive control: without it
/// an absent `survived` would prove nothing, because a tree that never
/// formed also never announces itself.
fn stalling_tree(born: &std::path::Path, survived: &std::path::Path) -> Vec<String> {
    command(&format!(
        "read -r hello\n\
         (: > '{}'; sleep 4; : > '{}') </dev/null >/dev/null 2>&1 &\n\
         read -r stall\n",
        born.display(),
        survived.display()
    ))
}

/// The deadline kill must unblock the harness, not merely signal the
/// one process the harness holds a handle to. It has to come back with a
/// determinate `Failed(deadline)` inside the deadline plus a bounded
/// margin, and the driver really did have a child of its own — the
/// twenty-minute CI job timeout is the hang backstop, never the
/// assertion.
#[test]
fn the_deadline_kill_unblocks_a_stalled_driver_tree() {
    let dir = tempfile::tempdir().unwrap();
    let born = dir.path().join("the-child-was-born");
    let survived = dir.path().join("the-child-outlived-the-kill");
    let driver = stalling_tree(&born, &survived);
    // Long enough that the driver has unmistakably reached its own
    // spawn before the watchdog fires, so `born` is a real control.
    let deadline = Duration::from_secs(2);

    let started = std::time::Instant::now();
    let report = DriverProcess::spawn(&driver, dir.path(), Some(deadline), &SpawnEnv::Inherit)
        .unwrap()
        .run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {});
    let elapsed = started.elapsed();

    assert!(
        matches!(&report.outcome, AttemptOutcome::Failed { error } if error.contains("deadline")),
        "{:?}",
        report.outcome
    );
    assert!(
        elapsed < deadline + Duration::from_secs(10),
        "the kill must unblock the harness inside the deadline plus a \
         bounded margin, took {elapsed:?}"
    );
    assert!(
        born.exists(),
        "the stalled driver must really have had a child of its own, \
         or the kill had no tree to prove anything about"
    );
    // Decision 0053 ruling 5: the stalled driver never accepted and
    // never checkpointed, so `Failed` alone would read as a failure to
    // START. The report says who ended it, and that is the fact the
    // engine's predicate reads instead.
    assert!(!report.accepted);
    assert!(report.checkpoints.is_empty());
    assert!(
        report.deadline_killed,
        "the watchdog's kill is on the report, not only in its prose"
    );
}

/// The session offer reaches a driver that DECLARED it can rejoin one,
/// ahead of the start it belongs to — and reaches no other driver at
/// all. A session handle belongs to the client that opened it, so a
/// driver that never said it knows what to do with one is never handed
/// one, whatever the engine found in the journal (decision 0030).
#[test]
fn an_offered_session_reaches_only_a_driver_that_declared_resume() {
    let advertising = wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec!["resume".into()],
    });
    let succeeded = succeeded();
    let dir = tempfile::tempdir().unwrap();
    for (case, capabilities, offered, expected) in [
        ("declared", &advertising, Some("thread-1"), 2),
        ("silent", &capabilities(), Some("thread-1"), 1),
        ("nothing offered", &advertising, None, 1),
    ] {
        let log = dir.path().join(case.replace(' ', "-"));
        // The path rides inside an `sh` script: Windows spells it with
        // backslashes, which `sh` eats — forward-slashed and quoted, the
        // same spelling works on every leg.
        let script = format!(
            "read -r line; printf '%s\\n' '{capabilities}'; \
             while read -r line; do printf '%s\\n' \"$line\" >> '{log}'; \
             case \"$line\" in *start*) break ;; esac; done; \
             printf '%s\\n' '{succeeded}'; read -r line",
            log = log.display().to_string().replace('\\', "/")
        );
        let report = DriverProcess::spawn(
            &command(&script),
            std::path::Path::new("."),
            None,
            &SpawnEnv::Inherit,
        )
        .unwrap()
        .run_attempt_resuming(
            "test",
            "effect",
            "attempt",
            "seat",
            json!({}),
            offered.map(str::to_string),
            |_| {},
        );
        assert!(
            matches!(report.outcome, AttemptOutcome::Succeeded { .. }),
            "{case}"
        );
        let received: Vec<Value> = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(received.len(), expected, "{case}: {received:?}");
        if expected == 2 {
            assert_eq!(received[0]["type"], "resume", "{case}");
            assert_eq!(received[0]["session_ref"], "thread-1", "{case}");
            assert_eq!(received[0]["effect_id"], "effect", "{case}");
            assert_eq!(received[0]["attempt_id"], "attempt", "{case}");
        }
        assert_eq!(received.last().unwrap()["type"], "start", "{case}");
    }
}

/// A pipe that breaks between the greeting and the offer fails the
/// attempt in its own words, like every other send on this path.
#[test]
fn a_broken_pipe_at_the_offer_is_a_determinate_failure() {
    let script = format!(
        "printf '%s\\n' '{}'",
        wire(Body::Capabilities {
            driver: "test".into(),
            version: "1".into(),
            supports: vec!["resume".into()],
        })
    );
    let mut process = DriverProcess::spawn(
        &command(&script),
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    )
    .unwrap();
    process.stdin = Box::new(BreakAfterFirstFlush::default());
    let report = process.run_attempt_resuming(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({}),
        Some("thread-1".into()),
        |_| {},
    );
    assert!(matches!(
        report.outcome,
        AttemptOutcome::Failed { error } if error.contains("could not send resume")
    ));
}

#[test]
fn a_driver_inherits_or_receives_exactly_the_selected_environment() {
    // No global environment mutation: a child test process reports the
    // presence of the normal PATH and the explicitly supplied probe value.
    let dir = tempfile::tempdir().unwrap();
    let shell =
        command("printf '%s\\n%s\\n' \"${PATH:+inherited}\" \"${BROKKR_ENV_PROBE:-absent}\"");
    let inherited = DriverProcess::spawn(&shell, dir.path(), None, &SpawnEnv::Inherit).unwrap();
    assert_eq!(stdout_text(&inherited), "inherited\nabsent\n");
    let table = [("BROKKR_ENV_PROBE".to_string(), "declared".to_string())].into();
    // The shell may install its own default PATH. Inspect an inherited
    // variable it does not synthesize to prove env_clear at the spawn door.
    let exact = DriverProcess::spawn(
        &command("printf '%s\\n%s\\n' \"${CARGO_MANIFEST_DIR:-cleared}\" \"$BROKKR_ENV_PROBE\""),
        dir.path(),
        None,
        &SpawnEnv::Exactly(table),
    )
    .unwrap();
    assert_eq!(stdout_text(&exact), "cleared\ndeclared\n");
}

/// Everything the driver printed, up to EOF.
fn stdout_text(process: &DriverProcess) -> String {
    let mut text = String::new();
    while let Some(Stdout::Line(line)) = process.next_stdout() {
        text.push_str(&line);
    }
    text
}

fn accepted() -> String {
    wire(Body::Accepted {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        session_ref: None,
    })
}

fn succeeded() -> String {
    wire(Body::Result {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        status: ResultStatus::Succeeded,
        result: Some(json!({"result": "complete"})),
        error: None,
    })
}

/// The handshake of a driver that accepts the attempt.
fn accepting() -> String {
    format!(
        "printf '%s\\n' '{}'; read -r start; printf '%s\\n' '{}'",
        capabilities(),
        accepted()
    )
}

fn spawned(driver: &[String], dir: &std::path::Path, deadline: Option<Duration>) -> DriverProcess {
    DriverProcess::spawn(driver, dir, deadline, &SpawnEnv::Inherit).unwrap()
}

fn attempt(process: DriverProcess) -> AttemptReport {
    process.run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {})
}

fn deadline_failure(report: &AttemptReport, secs: u64) {
    assert!(
        matches!(&report.outcome, AttemptOutcome::Failed { error }
            if *error == format!("attempt exceeded its {secs}s deadline and was killed")),
        "{:?}",
        report.outcome
    );
    assert!(report.deadline_killed);
}

/// Is `pid` gone? A zombie is not: the kernel still answers for it.
fn gone(pid: i32) -> bool {
    rustix::process::test_kill_process(Pid::from_raw(pid).unwrap()) == Err(rustix::io::Errno::SRCH)
}

/// Seat stubs for #403, in one directory as concurrent runs in one
/// checkout would be. A stub records its own pid, answers the handshake,
/// then forks a grandchild that records its pid and keeps appending to a
/// marker. The grandchild's stdio points away from the harness's pipes,
/// so a kill that misses it fails an assertion rather than hanging.
struct Seats {
    dir: tempfile::TempDir,
}

impl Seats {
    fn new() -> Seats {
        Seats {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn file(&self, tag: &str, what: &str) -> std::path::PathBuf {
        self.dir.path().join(format!("{tag}.{what}"))
    }

    /// The stub `tag`: `handshake` after the greeting, then the
    /// grandchild, then `then` once the grandchild has recorded itself.
    /// The grandchild stops when the directory goes, so a kill that
    /// misses it leaks nothing past the test.
    fn driver(&self, tag: &str, handshake: &str, then: &str) -> Vec<String> {
        let grandchild = self.file(tag, "grandchild");
        command(&format!(
            "printf '%s\\n' \"$$\" > '{driver}'\n\
             read -r hello\n\
             {handshake}\n\
             sh -c 'printf \"%s\\n\" \"$$\" > \"$1\"; \
             while [ -d \"$3\" ]; do printf x >> \"$2\"; sleep 0.05; done' \
             tree '{grandchild}' '{marker}' '{dir}' </dev/null >/dev/null 2>&1 &\n\
             while [ ! -s '{grandchild}' ]; do sleep 0.01; done\n\
             {then}\n",
            driver = self.file(tag, "driver").display(),
            grandchild = grandchild.display(),
            marker = self.file(tag, "marker").display(),
            dir = self.dir.path().display(),
        ))
    }

    /// The stub's tree, driver then grandchild, once both have recorded
    /// themselves: the positive control that there was a tree to end.
    fn pids(&self, tag: &str) -> [i32; 2] {
        let read = |what| {
            let text = std::fs::read_to_string(self.file(tag, what)).unwrap_or_default();
            text.trim().parse::<i32>().ok()
        };
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            if let (Some(driver), Some(grandchild)) = (read("driver"), read("grandchild")) {
                return [driver, grandchild];
            }
            assert!(Instant::now() < until, "stub {tag} never formed its tree");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn marker(&self, tag: &str) -> u64 {
        std::fs::metadata(self.file(tag, "marker")).unwrap().len()
    }
}

/// The acceptance of #403: a timed-out attempt's driver AND the
/// grandchild it forked are gone at the moment the report returns, which
/// is the moment the engine may start a retry, so the retry cannot
/// overlap them; and the grandchild's marker stops moving.
#[test]
fn a_deadline_kill_ends_every_descendant_before_the_report_returns() {
    let seats = Seats::new();
    let driver = seats.driver("seat", &accepting(), "read -r never");
    let report = attempt(spawned(
        &driver,
        seats.dir.path(),
        Some(Duration::from_secs(1)),
    ));
    let tree = seats.pids("seat");
    let survivors: Vec<i32> = tree.into_iter().filter(|pid| !gone(*pid)).collect();
    assert_eq!(survivors, Vec::<i32>::new(), "tree {tree:?}");
    deadline_failure(&report, 1);
    assert!(report.accepted);
    let marker = seats.marker("seat");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(seats.marker("seat"), marker, "the grandchild kept writing");
}

/// A driver that dies on its own, mid-attempt, takes its descendants
/// with it: the harness it started does not keep working unowned.
#[test]
fn an_abrupt_driver_exit_takes_its_descendants_with_it() {
    let seats = Seats::new();
    let driver = seats.driver("seat", &accepting(), "exit 0");
    let report = attempt(spawned(&driver, seats.dir.path(), None));
    let tree = seats.pids("seat");
    assert_eq!(tree.map(gone), [true, true], "tree {tree:?}");
    assert!(
        matches!(&report.outcome, AttemptOutcome::Indeterminate { reason }
            if reason == "driver exited after accepting, before a result — attempt \
                          cannot be established as complete"),
        "{:?}",
        report.outcome
    );
}

/// The issue's regression matrix: a driver that lingers after a result,
/// or after a malformed message, is ended within the shutdown grace and
/// not waited on for as long as it cares to live, and what it says after
/// its last word is not read as protocol.
#[test]
fn a_driver_that_lingers_after_its_last_word_is_ended_within_the_grace() {
    let malformed = serde_json::from_str::<Message>("not json\n").unwrap_err();
    for (case, last_word, expected) in [
        (
            "a result",
            format!("printf '%s\\n' '{}' 'after the result'", succeeded()),
            None,
        ),
        (
            "a malformed message",
            "printf 'not json\\n'".to_string(),
            Some(format!(
                "unreadable driver message: {malformed}: not json\n"
            )),
        ),
    ] {
        let seats = Seats::new();
        let handshake = format!("{}; {last_word}", accepting());
        let mut process = spawned(
            &seats.driver("seat", &handshake, "sleep 10"),
            seats.dir.path(),
            None,
        );
        process.bounds.grace = Duration::from_millis(200);
        let started = Instant::now();
        let report = attempt(process);
        let elapsed = started.elapsed();
        assert!(elapsed < Duration::from_secs(5), "{case}: took {elapsed:?}");
        assert_eq!(seats.pids("seat").map(gone), [true, true], "{case}");
        match (&report.outcome, &expected) {
            (AttemptOutcome::Succeeded { .. }, None) => {}
            (AttemptOutcome::Failed { error }, Some(expected)) => assert_eq!(error, expected),
            (outcome, _) => panic!("{case}: {outcome:?}"),
        }
    }
}

/// The issue's second probe: a grandchild that inherited the driver's
/// stdout and stderr is killed with it, so the pipes close at the
/// deadline and not when the grandchild would have finished.
#[test]
fn a_grandchild_holding_the_pipes_does_not_outlast_the_deadline() {
    let driver = command(&format!(
        "read -r hello; {}; sleep 10 & sleep 10",
        accepting()
    ));
    let started = Instant::now();
    let report = attempt(spawned(
        &driver,
        std::path::Path::new("."),
        Some(Duration::from_secs(1)),
    ));
    let elapsed = started.elapsed();
    deadline_failure(&report, 1);
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

/// An end that cannot be proven — a group that will not empty, or a pipe
/// something outside the group still holds — parks the attempt with the
/// outcome it reached in the reason; it never certifies the attempt.
#[test]
fn an_attempt_whose_end_cannot_be_proven_parks() {
    let held = |process: &mut DriverProcess| process.bounds.drain = Duration::from_millis(200);
    let mut process = spawned(
        &command(&format!(
            "read -r hello; {}; printf '%s\\n' '{}'",
            accepting(),
            succeeded()
        )),
        std::path::Path::new("."),
        None,
    );
    process.alive = |_| true;
    process.bounds.settle = Duration::from_millis(50);
    let group = process.child.id();
    let group_survived = attempt(process);

    let mut process = spawned(
        &command("read -r hello; read -r never"),
        std::path::Path::new("."),
        Some(Duration::from_secs(1)),
    );
    held(&mut process);
    let (_stdout_holder, stdout) = mpsc::channel();
    process.stdout = stdout;
    let started = Instant::now();
    let stdout_held = attempt(process);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");

    let mut process = spawned(&command("exit 0"), std::path::Path::new("."), None);
    held(&mut process);
    let (_stderr_holder, stderr) = mpsc::channel();
    process.stderr = stderr;
    let stderr_held = attempt(process);

    for (report, expected) in [
        (
            group_survived,
            format!(
                "the driver reported success; the attempt is not proven over: \
                 its process group {group} still had members after the kill"
            ),
        ),
        (
            stdout_held,
            "attempt exceeded its 1s deadline and was killed; the attempt is not proven \
             over: a process outside its group still held the driver's stdout"
                .to_string(),
        ),
        (
            stderr_held,
            "driver exited before accepting the attempt; the attempt is not proven over: \
             a process outside its group still held the driver's stderr"
                .to_string(),
        ),
    ] {
        assert!(
            matches!(&report.outcome, AttemptOutcome::Indeterminate { reason } if *reason == expected),
            "{:?}",
            report.outcome
        );
    }
}

/// Termination is by the attempt's own group, never by directory: a
/// concurrent run in the same checkout keeps its whole tree through the
/// other's deadline kill, and then finishes on its own terms.
#[test]
fn a_concurrent_run_in_the_same_directory_is_never_signalled() {
    let seats = Seats::new();
    let dir = seats.dir.path().to_path_buf();
    let done = seats.file("a", "done");
    let other = seats.driver(
        "b",
        &accepting(),
        &format!(
            "while [ -d '{}' ] && [ ! -e '{}' ]; do sleep 0.05; done; \
             printf '%s\\n' '{}'; read -r shutdown",
            dir.display(),
            done.display(),
            succeeded()
        ),
    );
    let other =
        std::thread::spawn(move || attempt(spawned(&other, &dir, Some(Duration::from_secs(30)))));
    let other_tree = seats.pids("b");

    let killed = seats.driver("a", &accepting(), "read -r never");
    deadline_failure(
        &attempt(spawned(
            &killed,
            seats.dir.path(),
            Some(Duration::from_secs(1)),
        )),
        1,
    );
    assert_eq!(seats.pids("a").map(gone), [true, true]);
    assert_eq!(other_tree.map(gone), [false, false]);
    assert!(tree::group_alive(Pid::from_raw(other_tree[0]).unwrap()));

    std::fs::write(&done, "").unwrap();
    let report = other.join().unwrap();
    assert!(matches!(report.outcome, AttemptOutcome::Succeeded { .. }));
    assert_eq!(other_tree.map(gone), [true, true]);
}
