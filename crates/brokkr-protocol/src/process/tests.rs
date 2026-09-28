use super::*;
use rustix::process::Pid;
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

fn process_with_writer(writer: impl std::io::Write + Send + 'static) -> DriverProcess {
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

    // #403: a table that cannot be read before the spawn refuses it, and
    // nothing runs. An attempt on the real host starts the tracker first,
    // so the blind one serves only this spawn.
    run("exit 0");
    let dir = tempfile::tempdir().unwrap();
    let ran = dir.path().join("ran");
    let driver = command(&format!("printf x > '{}'", ran.display()));
    let blind = Host {
        table: || Ok(Vec::new()),
        ..Host::REAL
    };
    let refused = DriverProcess::spawn_with(&driver, dir.path(), None, &SpawnEnv::Inherit, blind);
    let Err(error) = refused else {
        panic!("a spawn without the read before it must be refused")
    };
    let source = Unsettled::Table {
        error: format!(
            "the table has no row for the engine itself (pid {})",
            std::process::id()
        ),
    };
    assert_eq!(
        error.to_string(),
        format!("refused to spawn driver {}: {source}", driver.join(" "))
    );
    assert!(matches!(error, SpawnError::Unwatched { source: found, .. } if found == source));
    std::thread::sleep(Duration::from_millis(100));
    assert!(!ran.exists(), "the refused driver ran");
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
    assert_eq!(report.cleanup, Cleanup::Settled);
}

/// Is `pid` gone? A zombie is: it has exited, whoever still holds its
/// pid (#403).
fn gone(pid: i32) -> bool {
    !table::snapshot()
        .unwrap()
        .iter()
        .any(|entry| entry.id.pid == pid && !entry.zombie)
}

/// Poll until every pid of `tree` is gone, for up to three seconds.
fn all_gone(tree: [i32; 2]) -> bool {
    let until = Instant::now() + Duration::from_secs(3);
    while !tree.into_iter().all(gone) {
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// The environment variable that names the part this test binary plays
/// when a test re-executes it, and the files that part is handed.
const ROLE: &str = "BROKKR_PROCESS_TEST_ROLE";
const ROLE_PID: &str = "BROKKR_PROCESS_TEST_PID";
const ROLE_MARKER: &str = "BROKKR_PROCESS_TEST_MARKER";
const ROLE_DIR: &str = "BROKKR_PROCESS_TEST_DIR";
const ROLE_GROUP: &str = "BROKKR_PROCESS_TEST_GROUP";

/// Seat stubs for #403, in one directory as concurrent runs in one
/// checkout would be. A stub records its own pid, answers the handshake,
/// then forks a grandchild that records its pid and keeps appending to a
/// marker. The grandchild's stdio points away from the harness's pipes,
/// so a kill that misses it fails an assertion rather than hanging.
struct Seats {
    dir: std::path::PathBuf,
    _owned: Option<tempfile::TempDir>,
}

impl Seats {
    fn new() -> Seats {
        let owned = tempfile::tempdir().unwrap();
        Seats {
            dir: owned.path().to_path_buf(),
            _owned: Some(owned),
        }
    }

    /// The seats another test process owns, from inside a role.
    fn at(dir: &str) -> Seats {
        Seats {
            dir: dir.into(),
            _owned: None,
        }
    }

    fn file(&self, tag: &str, what: &str) -> std::path::PathBuf {
        self.dir.join(format!("{tag}.{what}"))
    }

    /// The stub `tag`: `handshake` after the greeting, then the
    /// grandchild, then `then` once the grandchild has recorded itself.
    /// The grandchild stops when the directory goes, so a kill that
    /// misses it leaks nothing past the test.
    fn driver(&self, tag: &str, handshake: &str, then: &str) -> Vec<String> {
        self.stub(
            tag,
            handshake,
            "sh -c 'printf \"%s\\n\" \"$$\" > \"$1\"; \
             while [ -d \"$3\" ]; do printf x >> \"$2\"; sleep 0.05; done' \
             tree \"$GRANDCHILD\" \"$MARKER\" \"$DIR\"",
            then,
        )
    }

    /// The same stub, whose grandchild is this test binary re-executed in
    /// `role`, which leaves the group before it records itself: `detached`
    /// leaves the session too (`setsid`, as Node's `detached` spawn does);
    /// `job` moves to a group of its own in the same session, as shell job
    /// control does; `joiner` tries to join the engine's own group, whose
    /// id it is handed. The shell that starts either exits at once, so it
    /// is orphaned before the tracker can see its parent.
    fn role_driver(&self, tag: &str, role: &str, handshake: &str, then: &str) -> Vec<String> {
        let exe = std::env::current_exe().unwrap();
        let background = if role == "detached" { "" } else { " &" };
        let group = rustix::process::getpgrp().as_raw_pid();
        self.stub(
            tag,
            handshake,
            &format!(
                "{ROLE}={role} {ROLE_PID}=\"$GRANDCHILD\" {ROLE_MARKER}=\"$MARKER\" \
                 {ROLE_DIR}=\"$DIR\" {ROLE_GROUP}={group} sh -c '\"$0\" --exact \
                 process::tests::role --ignored{background}' '{}'",
                exe.display()
            ),
            then,
        )
    }

    fn stub(&self, tag: &str, handshake: &str, grandchild: &str, then: &str) -> Vec<String> {
        command(&format!(
            "GRANDCHILD='{grandchild_file}' MARKER='{marker}' DIR='{dir}'\n\
             printf '%s\\n' \"$$\" > '{driver}'\n\
             read -r hello\n\
             {handshake}\n\
             {grandchild} </dev/null >/dev/null 2>&1 &\n\
             while [ ! -s \"$GRANDCHILD\" ]; do sleep 0.01; done\n\
             {then}\n",
            driver = self.file(tag, "driver").display(),
            grandchild_file = self.file(tag, "grandchild").display(),
            marker = self.file(tag, "marker").display(),
            dir = self.dir.display(),
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
    let report = attempt(spawned(&driver, &seats.dir, Some(Duration::from_secs(1))));
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
    let report = attempt(spawned(&driver, &seats.dir, None));
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
            &seats.dir,
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

/// An end that cannot be proven — a pipe something outside the tree
/// still holds, a group the kernel would not signal, a leader that would
/// not be reaped, a table that cannot be read — leaves the outcome the
/// driver reached as it reached it, typed, and the cleanup unresolved
/// beside it: the report never certifies the attempt (#403).
#[test]
fn an_attempt_whose_end_cannot_be_proven_parks() {
    let held = |process: &mut DriverProcess| process.bounds.drain = Duration::from_millis(200);
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

    let answering = format!("read -r hello; {}", capabilities());
    let mut process = spawned(&command(&answering), std::path::Path::new("."), None);
    process.host.kill_group = |_| Err(rustix::io::Errno::PERM);
    let group = i32::try_from(process.child.id()).unwrap();
    let kill_refused = attempt(process);

    let mut process = spawned(&command("exit 0"), std::path::Path::new("."), None);
    process.host.table = || Ok(Vec::new());
    process.bounds.settle = Duration::from_millis(50);
    let unreadable = attempt(process);

    let seats = Seats::new();
    let with_descendants = seats.driver("seat", &accepting(), "exit 0");
    let missing = || {
        Some(Unsettled::Subreaper(
            rustix::io::Errno::INVAL.raw_os_error(),
        ))
    };
    let mut process = spawned(&with_descendants, &seats.dir, None);
    process.host.missing = missing;
    let unwatched = attempt(process);
    let mut process = spawned(&command(&answering), std::path::Path::new("."), None);
    process.host.missing = missing;
    let alone = attempt(process);

    assert!(
        matches!(&stdout_held.outcome, AttemptOutcome::Failed { error }
        if error == "attempt exceeded its 1s deadline and was killed")
    );
    assert_eq!(unresolved(&stdout_held), Some(&Unsettled::Stdout));
    assert!(
        matches!(&stderr_held.outcome, AttemptOutcome::Indeterminate { reason }
        if reason == "driver exited before accepting the attempt")
    );
    assert_eq!(unresolved(&stderr_held), Some(&Unsettled::Stderr));
    assert_eq!(
        unresolved(&kill_refused),
        Some(&Unsettled::Kill {
            group,
            errno: rustix::io::Errno::PERM.raw_os_error(),
        })
    );
    assert_eq!(
        unresolved(&unreadable),
        Some(&Unsettled::Table {
            error: format!(
                "the table has no row for the engine itself (pid {})",
                std::process::id()
            )
        })
    );
    assert_eq!(unresolved(&unwatched), missing().as_ref());
    assert_eq!(seats.pids("seat").map(gone), [true, true]);
    assert_eq!(alone.cleanup, Cleanup::Settled, "no descendant, no doubt");
}

fn unresolved(report: &AttemptReport) -> Option<&Unsettled> {
    match &report.cleanup {
        Cleanup::Settled => None,
        Cleanup::Unresolved { reason } => Some(reason),
    }
}

/// A leader the kill did not reach is not waited on without bound: the
/// reap gives up at the settle bound, and the result the driver sent is
/// kept, typed, beside the unresolved cleanup (#403).
#[test]
fn a_leader_that_outlives_the_kill_is_not_waited_on_past_the_settle_bound() {
    let driver = format!(
        "read -r hello; {}; printf '%s\\n' '{}'; exec sleep 30",
        accepting(),
        succeeded()
    );
    let mut process = spawned(&command(&driver), std::path::Path::new("."), None);
    process.host.kill_group = |_| Ok(());
    process.bounds.grace = Duration::from_millis(100);
    process.bounds.settle = Duration::from_millis(200);
    // The leader holds the pipes too; the reap is the first bound it meets.
    process.bounds.drain = Duration::from_millis(200);
    let leader = i32::try_from(process.child.id()).unwrap();
    let started = Instant::now();
    let report = attempt(process);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(3), "took {elapsed:?}");
    tree::kill_group(Pid::from_raw(leader).unwrap()).unwrap();
    assert!(all_gone([leader, leader]), "the lingering leader {leader}");
    assert!(
        matches!(&report.outcome, AttemptOutcome::Succeeded { result }
        if *result == json!({"result": "complete"}))
    );
    assert_eq!(unresolved(&report), Some(&Unsettled::Reap { pid: leader }));
}

/// What a caller acts on (#403): the outcome reached once the tree is
/// settled, and `Indeterminate` naming that outcome while it is not; the
/// terminal event's evidence carries the outcome typed beside the cleanup.
#[test]
fn an_unresolved_cleanup_is_acted_on_as_indeterminate() {
    let report = |outcome: AttemptOutcome, cleanup: Cleanup| AttemptReport {
        outcome,
        refused: None,
        cleanup,
        session_ref: None,
        checkpoints: Vec::new(),
        stderr: String::new(),
        accepted: true,
        deadline_killed: false,
    };
    let stdout = || Cleanup::Unresolved {
        reason: Unsettled::Stdout,
    };
    let succeeded = || AttemptOutcome::Succeeded {
        result: json!({"result": "complete"}),
    };
    let settled = report(succeeded(), Cleanup::Settled);
    assert!(matches!(
        settled.settled_outcome(),
        AttemptOutcome::Succeeded { .. }
    ));
    assert!(settled.cleanup_evidence().is_empty());
    // A refusal replaces the outcome acted on, never the one received.
    let refused = AttemptReport {
        refused: Some(AttemptOutcome::Failed {
            error: "refused".into(),
        }),
        ..settled
    };
    assert!(matches!(
        refused.settled_outcome(),
        AttemptOutcome::Failed { error } if error == "refused"
    ));
    assert!(matches!(refused.outcome, AttemptOutcome::Succeeded { .. }));
    let not_over = "the attempt is not proven over: \
                    a process outside its tree still held the driver's stdout";
    for (outcome, reached) in [
        (succeeded(), "the driver reported success"),
        (
            AttemptOutcome::Failed {
                error: "boom".into(),
            },
            "boom",
        ),
        (
            AttemptOutcome::Indeterminate {
                reason: "lost".into(),
            },
            "lost",
        ),
    ] {
        let unresolved = report(outcome, stdout());
        assert!(
            matches!(unresolved.settled_outcome(), AttemptOutcome::Indeterminate { reason }
            if reason == format!("{reached}; {not_over}"))
        );
    }
    assert_eq!(
        Value::Object(report(succeeded(), stdout()).cleanup_evidence()),
        json!({
            "received": {"status": "succeeded", "result": {"result": "complete"}},
            "cleanup": {
                "state": "unresolved",
                "reason": "a process outside its tree still held the driver's stdout",
            },
        })
    );
}

/// Termination is by the attempt's own group, never by directory: a
/// concurrent run in the same checkout keeps its whole tree through the
/// other's deadline kill, and then finishes on its own terms.
#[test]
fn a_concurrent_run_in_the_same_directory_is_never_signalled() {
    let seats = Seats::new();
    let dir = seats.dir.clone();
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
        &attempt(spawned(&killed, &seats.dir, Some(Duration::from_secs(1)))),
        1,
    );
    assert_eq!(seats.pids("a").map(gone), [true, true]);
    assert_eq!(other_tree.map(gone), [false, false]);

    std::fs::write(&done, "").unwrap();
    let report = other.join().unwrap();
    assert!(matches!(report.outcome, AttemptOutcome::Succeeded { .. }));
    assert_eq!(other_tree.map(gone), [true, true]);
}

/// The parts this test binary plays when a test re-executes it (#403).
/// Run on its own, it plays none.
#[test]
#[ignore = "played only when a test re-executes this binary"]
fn role() {
    let var = |name: &str| std::env::var(name).unwrap_or_default();
    let dir = var(ROLE_DIR);
    let detached =
        |seats: &Seats| seats.role_driver("seat", "detached", &accepting(), "read -r never");
    let in_group = |seats: &Seats| seats.driver("seat", &accepting(), "read -r never");
    let blind = Host {
        table: blind_after_the_spawn,
        ..Host::REAL
    };
    match var(ROLE).as_str() {
        "detached" => left(setsid, &var(ROLE_PID), &var(ROLE_MARKER), &dir),
        "job" => left(own_group, &var(ROLE_PID), &var(ROLE_MARKER), &dir),
        "joiner" => left(join_the_engine, &var(ROLE_PID), &var(ROLE_MARKER), &dir),
        "engine" => stopped_engine(&dir, libc::SIG_DFL, detached, Host::REAL),
        "nohup-engine" => stopped_engine(&dir, libc::SIG_IGN, detached, Host::REAL),
        "blind-engine" => stopped_engine(&dir, libc::SIG_DFL, in_group, blind),
        "job-engine" => orphaning_engine(&dir, "job"),
        "joiner-engine" => orphaning_engine(&dir, "joiner"),
        _ => {}
    }
}

/// The table the spawn reads, and nothing after it: a spawn without the
/// read before is refused. The first read is the spawn's, because the
/// tracker reads only once an attempt is registered.
fn blind_after_the_spawn() -> Result<Vec<table::Entry>, table::TableError> {
    static READ: AtomicBool = AtomicBool::new(false);
    if READ.swap(true, Ordering::SeqCst) {
        Ok(Vec::new())
    } else {
        table::snapshot()
    }
}

fn setsid() {
    rustix::process::setsid().expect("a detached descendant leaves the session");
}

fn own_group() {
    rustix::process::setpgid(None, None).expect("a job leads a group of its own");
}

/// Try to move into the engine's own group, and write down the errno the
/// kernel answered with, or `joined`.
fn join_the_engine() {
    let var = |name: &str| std::env::var(name).unwrap_or_default();
    let group = var(ROLE_GROUP).parse().ok().and_then(Pid::from_raw);
    let answer = match rustix::process::setpgid(None, group) {
        Ok(()) => "joined".to_string(),
        Err(errno) => errno.raw_os_error().to_string(),
    };
    std::fs::write(Seats::at(&var(ROLE_DIR)).file("seat", "joined"), answer).unwrap();
}

/// A descendant that leaves the attempt's group by `leave`, its stdio
/// already pointed away from the driver's pipes. It records its pid only
/// once it has left, then appends to its marker for as long as the seats'
/// directory lasts.
fn left(leave: fn(), pid_file: &str, marker: &str, dir: &str) {
    leave();
    std::fs::write(pid_file, format!("{}\n", std::process::id())).unwrap();
    while std::path::Path::new(dir).is_dir() {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(marker)
            .unwrap();
        file.write_all(b"x").unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// An engine running one attempt of `driver` on `host`, as its first,
/// with the stop signals at their defaults whatever launched the test and
/// hangup at `hangup` (`SIG_IGN` as under `nohup`). It ends only by a
/// signal.
fn stopped_engine(
    dir: &str,
    hangup: libc::sighandler_t,
    driver: fn(&Seats) -> Vec<String>,
    host: Host,
) {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    for signal in [SIGINT, SIGTERM, SIGHUP] {
        // SAFETY: setting a default disposition, before any handler exists.
        unsafe { libc::signal(signal, libc::SIG_DFL) };
    }
    // SAFETY: as above.
    unsafe { libc::signal(SIGHUP, hangup) };
    let seats = Seats::at(dir);
    let process =
        DriverProcess::spawn_with(&driver(&seats), &seats.dir, None, &SpawnEnv::Inherit, host);
    attempt(process.unwrap());
    unreachable!("the driver never ends on its own");
}

/// An engine whose attempt's grandchild is an orphaned `role`, and whose
/// driver exits as soon as the grandchild has recorded itself, so the
/// grandchild comes to the engine. It writes down the cleanup, the outcome
/// and what of the tree it still reads running once the report has
/// returned.
fn orphaning_engine(dir: &str, role: &str) {
    let seats = Seats::at(dir);
    let driver = seats.role_driver("seat", role, &accepting(), "exit 0");
    let report = attempt(spawned(&driver, &seats.dir, None));
    let tree = seats.pids("seat");
    let survivors: Vec<i32> = tree.into_iter().filter(|pid| !gone(*pid)).collect();
    let written = format!("{:?} {:?} {survivors:?}", report.cleanup, report.outcome);
    std::fs::write(seats.file("seat", "report"), written).unwrap();
}

/// This test binary as an engine playing `role` over `seats`.
fn engine(seats: &Seats, role: &str) -> Command {
    let mut engine = Command::new(std::env::current_exe().unwrap());
    engine
        .args(["--exact", "process::tests::role", "--ignored"])
        .env(ROLE, role)
        .env(ROLE_DIR, &seats.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    engine
}

/// The engine, in a session and group of its own, once its attempt's tree
/// has formed and the tracker has had time to record the detached
/// grandchild. It is registered as one of this process's attempts: in a
/// group of its own and this process's child, it would otherwise read, to
/// another test's attempt, as an orphan this process adopted.
fn engine_in_its_own_group(seats: &Seats, role: &str) -> (Child, Attempt, [i32; 2]) {
    let (engine, registered) = Attempt::spawn(&mut engine(seats, role), Host::REAL).unwrap();
    let tree = seats.pids("seat");
    std::thread::sleep(Duration::from_millis(500));
    (engine, registered, tree)
}

fn signal_group(engine: &Child, signal: i32) {
    let group = Pid::from_raw(i32::try_from(engine.id()).unwrap()).unwrap();
    rustix::process::kill_process_group(
        group,
        rustix::process::Signal::from_named_raw(signal).unwrap(),
    )
    .unwrap();
}

/// The engine's exit status, waited on for up to ten seconds.
fn exit_code(engine: &mut Child) -> Option<i32> {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = engine.try_wait().unwrap() {
            return status.code();
        }
        if Instant::now() >= until {
            engine.kill().unwrap();
            engine.wait().unwrap();
            panic!("the engine did not exit on its stop signal");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The attempt's driver and its grandchild are gone, and the marker has
/// stopped.
fn assert_ended(seats: &Seats, tree: [i32; 2], signal: i32) {
    assert!(all_gone(tree), "signal {signal}: tree {tree:?} survived");
    let marker = seats.marker("seat");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        seats.marker("seat"),
        marker,
        "signal {signal}: the marker moved"
    );
}

/// #403: the driver leads a session of its own, so a signal to the
/// engine's group (a terminal's Ctrl-C or hangup, a supervisor's SIGTERM)
/// does not reach its tree. The engine ends every live attempt, the detached
/// grandchild included, and exits 128 plus the signal.
#[test]
fn a_stopped_engine_ends_its_attempts_before_it_exits() {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    for signal in [SIGINT, SIGTERM, SIGHUP] {
        let seats = Seats::new();
        let (mut engine, _registered, tree) = engine_in_its_own_group(&seats, "engine");
        signal_group(&engine, signal);
        assert_eq!(
            exit_code(&mut engine),
            Some(128 + signal),
            "signal {signal}"
        );
        assert_ended(&seats, tree, signal);
    }
}

/// #403 finding 3: a stop that cannot read the process table still kills
/// what its group signal reaches, then waits out the settle bound for a
/// table that never reads its attempts gone, and exits 125 rather than
/// 128 plus the signal: it cannot say they are over.
#[test]
fn a_stop_that_cannot_prove_its_attempts_over_exits_distinctly() {
    use signal_hook::consts::SIGTERM;
    let seats = Seats::new();
    let (mut engine, _registered, tree) = engine_in_its_own_group(&seats, "blind-engine");
    signal_group(&engine, SIGTERM);
    assert_eq!(exit_code(&mut engine), Some(125));
    assert_ended(&seats, tree, SIGTERM);
}

/// A hangup the engine inherited as ignored, as under `nohup`, stays
/// ignored: its attempt runs on through one, and SIGTERM still ends it.
#[test]
fn an_engine_that_inherited_hangup_ignored_runs_on_through_one() {
    use signal_hook::consts::{SIGHUP, SIGTERM};
    let seats = Seats::new();
    let (mut engine, _registered, tree) = engine_in_its_own_group(&seats, "nohup-engine");
    signal_group(&engine, SIGHUP);
    let marker = seats.marker("seat");
    std::thread::sleep(Duration::from_millis(500));
    assert!(engine.try_wait().unwrap().is_none(), "the engine hung up");
    assert!(seats.marker("seat") > marker, "the attempt stopped");
    signal_group(&engine, SIGTERM);
    assert_eq!(exit_code(&mut engine), Some(128 + SIGTERM));
    assert_ended(&seats, tree, SIGTERM);
}

/// #403: a grandchild that calls `setsid` and points its stdio away from
/// the driver's pipes is out of the group signal's reach and invisible on
/// the pipes; it is ended by its recorded identity, before the report
/// returns, and the kill is certified only because it is gone.
#[test]
fn a_deadline_kill_ends_a_descendant_that_left_the_group_and_its_pipes() {
    let seats = Seats::new();
    let driver = seats.role_driver("seat", "detached", &accepting(), "read -r never");
    let report = attempt(spawned(&driver, &seats.dir, Some(Duration::from_secs(3))));
    let tree = seats.pids("seat");
    let survivors: Vec<i32> = tree.into_iter().filter(|pid| !gone(*pid)).collect();
    assert_eq!(survivors, Vec::<i32>::new(), "tree {tree:?}");
    deadline_failure(&report, 3);
    let marker = seats.marker("seat");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        seats.marker("seat"),
        marker,
        "the detached grandchild kept writing"
    );
}

/// #403: a running driver is its own tree's subreaper, so an orphan of the
/// tree goes to the driver, where the tracker records it, and not to the
/// engine, where another attempt could be taken for its source.
#[test]
fn a_running_driver_adopts_its_trees_orphans() {
    let seats = Seats::new();
    let orphan = seats.file("seat", "orphan");
    let driver = command(&format!(
        "read -r hello; {}; sh -c 'sleep 30 & printf \"%s\\n\" \"$!\" > \"$1\"' orphan '{}'; \
         read -r never",
        accepting(),
        orphan.display()
    ));
    let process = spawned(&driver, &seats.dir, Some(Duration::from_secs(2)));
    let leader = i32::try_from(process.child.id()).unwrap();
    let ended = std::thread::spawn(move || attempt(process));
    let until = Instant::now() + Duration::from_secs(2);
    let pid = loop {
        let text = std::fs::read_to_string(&orphan).unwrap_or_default();
        if let Ok(pid) = text.trim().parse::<i32>() {
            break pid;
        }
        assert!(Instant::now() < until, "the orphan never recorded itself");
        std::thread::sleep(Duration::from_millis(10));
    };
    let parent = |pid| {
        let rows = table::snapshot().unwrap();
        rows.into_iter()
            .find(|entry| entry.id.pid == pid)
            .map(|entry| entry.ppid)
    };
    while parent(pid) != Some(leader) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    let adopted = parent(pid);
    let report = ended.join().unwrap();
    assert_eq!(adopted, Some(leader), "the orphan's parent");
    deadline_failure(&report, 2);
    assert!(gone(pid), "the adopted orphan outlived the attempt");
}

/// #403 finding 1: a grandchild that shell job control moves to a group of
/// its own, in the driver's session, whose shell exits at once and whose
/// driver exits next, comes to the engine before the tracker can record
/// it. The engine adopts it as a stray, whatever its session, attributes
/// it to the attempt whose leader is gone, and ends it before the report
/// returns, which certifies the end only because it is gone. The engine
/// is a child process, so that no other test's attempt is live beside it.
#[test]
fn a_job_orphaned_before_the_tracker_saw_it_is_ended_before_the_report() {
    orphaned("job");
}

/// #403: a grandchild that tries to join the engine's own group, the one
/// group whose members are never strays, is refused by the kernel: its
/// driver leads a session of its own, and `setpgid` refuses a group in
/// another session. So it stays in the attempt's group, and is ended
/// with it before the report returns, though its shell and its driver
/// exit before the tracker can see it.
#[test]
fn a_grandchild_cannot_join_the_engines_group_to_outlive_the_report() {
    let seats = orphaned("joiner");
    let answer = std::fs::read_to_string(seats.file("seat", "joined")).unwrap();
    assert_eq!(answer, rustix::io::Errno::PERM.raw_os_error().to_string());
}

/// The engine, a child process so that no other test's attempt is live
/// beside it, playing an `orphaning_engine` of `role`: the report settled
/// only because the tree is gone, and the grandchild's marker stopped.
fn orphaned(role: &str) -> Seats {
    let seats = Seats::new();
    let mut engine = engine(&seats, &format!("{role}-engine")).spawn().unwrap();
    assert_eq!(exit_code(&mut engine), Some(0));
    let written = std::fs::read_to_string(seats.file("seat", "report")).unwrap();
    assert_eq!(
        written,
        "Settled Indeterminate { reason: \"driver exited after accepting, before a result — \
         attempt cannot be established as complete\" } []"
    );
    let tree = seats.pids("seat");
    assert_eq!(tree.map(gone), [true, true], "tree {tree:?}");
    let marker = seats.marker("seat");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(seats.marker("seat"), marker, "the {role} kept writing");
    seats
}

/// A pipe, and how many bytes it holds before a write blocks, handed back
/// empty. Measured on the pipe itself: the kernel sizes each pipe as it
/// is made, smaller once a user's pipes pass their soft limit.
fn measured_pipe() -> (std::io::PipeReader, std::io::PipeWriter, usize) {
    let (mut reader, mut writer) = std::io::pipe().unwrap();
    let written = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let (counter, stopped) = (Arc::clone(&written), Arc::clone(&stop));
    let filler = std::thread::spawn(move || {
        while !stopped.load(Ordering::SeqCst) {
            writer.write_all(b"x").unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
        }
        writer
    });
    let mut capacity = usize::MAX;
    while written.load(Ordering::SeqCst) != capacity {
        capacity = written.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(200));
    }
    // One read frees the byte the filler is blocked on; it then stops.
    stop.store(true, Ordering::SeqCst);
    let mut chunk = vec![0; capacity];
    let mut drained = reader.read(&mut chunk).unwrap();
    let writer = filler.join().unwrap();
    while drained < written.load(Ordering::SeqCst) {
        drained += reader.read(&mut chunk).unwrap();
    }
    (reader, writer, capacity)
}

/// #403: a driver that answered without draining its stdin cannot hold
/// the report on the `shutdown` write. The start fills the pipe so the
/// write blocks, and the report still returns within the grace.
#[test]
fn a_driver_that_stops_reading_cannot_hold_the_shutdown_write() {
    let line = |body: Body| wire(body).len() + 1;
    let start = |pad: &str| {
        line(Body::Start {
            effect_id: "effect".into(),
            attempt_id: "attempt".into(),
            seat: "seat".into(),
            input: json!({"pad": pad}),
        })
    };
    // The driver's stdin is a pipe measured here. The greeting is read
    // off it, and then nothing: the start leaves half a shutdown line of
    // room.
    let (reader, writer, capacity) = measured_pipe();
    let pad = capacity - start("") - line(Body::Shutdown) / 2;
    let dir = tempfile::tempdir().unwrap();
    let greeted = dir.path().join("greeted");
    // The read end closes once the test is done, or after ten seconds, so
    // a write left blocking on it fails the test rather than hanging it.
    let (done, closing) = mpsc::channel::<()>();
    let reading = {
        let greeted = greeted.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            reader.read_line(&mut String::new()).unwrap();
            std::fs::write(greeted, "").unwrap();
            let _ = closing.recv_timeout(Duration::from_secs(10));
        })
    };
    let driver = command(&format!(
        "while [ ! -e '{}' ]; do sleep 0.01; done; printf '%s\\n' '{}' '{}' '{}'; sleep 10",
        greeted.display(),
        capabilities(),
        accepted(),
        succeeded()
    ));
    let mut process = spawned(&driver, std::path::Path::new("."), None);
    process.stdin = Box::new(writer);
    process.bounds.grace = Duration::from_millis(500);
    let group = i32::try_from(process.child.id()).unwrap();
    let started = Instant::now();
    let report = process.run_attempt(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({"pad": "x".repeat(pad)}),
        |_| {},
    );
    let elapsed = started.elapsed();
    assert!(
        matches!(report.outcome, AttemptOutcome::Succeeded { .. }),
        "{:?} after {elapsed:?}",
        report.outcome
    );
    assert!(elapsed < Duration::from_secs(3), "took {elapsed:?}");
    assert_eq!(report.cleanup, Cleanup::Settled);
    assert!(gone(group));
    // Closing the read end releases the blocked write's thread.
    drop(done);
    reading.join().unwrap();
}

/// #403: once the stdout bound has passed, nothing more is read, however
/// much a writer outside the tree keeps ready: the next read gives up and
/// the drain reports the pipe held without consuming what is ready.
#[test]
fn stdout_is_not_read_past_its_bound_whatever_is_ready() {
    let mut process = spawned(
        &command("read -r hello"),
        std::path::Path::new("."),
        Some(Duration::from_secs(1)),
    );
    let (lines, stdout) = mpsc::sync_channel(8);
    process.stdout = stdout;
    for _ in 0..2 {
        lines.send(Stdout::Line("x\n".into())).unwrap();
    }
    process.started = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
    process.bounds.drain = Duration::ZERO;
    assert!(
        process.next_stdout().is_none(),
        "a ready line was read past the bound"
    );
    assert_eq!(
        process.stdout_closed(Instant::now()),
        Err(Unsettled::Stdout)
    );
    assert!(
        matches!(process.stdout.try_recv(), Ok(Stdout::Line(_))),
        "the drain read a ready line past its bound"
    );
    // Something outside the tree holding stdout open parks the attempt
    // within the drain bound.
    process.bounds.drain = Duration::from_millis(100);
    let started = Instant::now();
    let report = attempt(process);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(unresolved(&report), Some(&Unsettled::Stdout));
    drop(lines);
}

/// #403: a flooding driver meets backpressure: its reader holds at most
/// `STDOUT_LINES` unread, and once the engine stops listening, it reads
/// nothing more.
#[test]
fn a_flooding_driver_meets_backpressure() {
    struct Flood(Arc<std::sync::atomic::AtomicUsize>);
    impl Read for Flood {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.fetch_add(1, Ordering::SeqCst);
            buf[..2].copy_from_slice(b"x\n");
            Ok(2)
        }
    }
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stdout = read_stdout(Flood(Arc::clone(&reads)));
    // The count once it stops moving, or after two seconds of moving.
    let still = || {
        let until = Instant::now() + Duration::from_secs(2);
        let mut last = usize::MAX;
        while reads.load(Ordering::SeqCst) != last && Instant::now() < until {
            last = reads.load(Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(100));
        }
        reads.load(Ordering::SeqCst)
    };
    assert_eq!(still(), STDOUT_LINES + 1);
    drop(stdout);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        still(),
        STDOUT_LINES + 1,
        "the reader went on after the refusal"
    );
}
