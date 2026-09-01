use super::*;
use serde_json::json;

fn command(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}

fn wire(body: Body) -> String {
    serde_json::to_string(&Message::new(body)).unwrap()
}

fn run(script: &str) -> AttemptReport {
    DriverProcess::spawn(&command(script), std::path::Path::new("."), None)
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
    let mut process =
        DriverProcess::spawn(&command(&script), std::path::Path::new("."), None).unwrap();
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
        DriverProcess::spawn(&[], std::path::Path::new("."), None),
        Err(SpawnError::EmptyCommand)
    ));
    let error = match DriverProcess::spawn(
        &["forge-certainly-does-not-exist".into()],
        std::path::Path::new("."),
        None,
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
        format!("read line; printf '\\n%s\\n' '{}'", wrong_proto),
        format!(
            "read line; printf '%s\\n' '{}'; read line",
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

    let before_accept = format!("read line; printf '%s\\n' '{}'; read line", capabilities());
    assert!(matches!(
        run(&before_accept).outcome,
        AttemptOutcome::Indeterminate { reason } if reason.contains("before accepting")
    ));
    let after_accept = format!(
        "read line; printf '%s\\n' '{}'; read line; printf '%s\\n' '{}'",
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
            "read line; printf '%s\\n' '{}'; read line; printf '%s\\n' '{}'",
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
        "read line; printf '%s\\n' '{}'; read line; printf '%s\\n' '{}'",
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
        "read line; printf '%s\\n' '{}'; read line; printf '%s\\n' '{}'",
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
    let invalid_utf8 = "read line; printf '\\377'";
    assert!(matches!(
        run(invalid_utf8).outcome,
        AttemptOutcome::Failed { error } if error.contains("stdout read failed")
    ));

    assert!(matches!(
        run("read line").outcome,
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
#[cfg(unix)]
fn stalling_tree(
    _dir: &std::path::Path,
    born: &std::path::Path,
    survived: &std::path::Path,
) -> Vec<String> {
    command(&format!(
        "read -r hello\n\
         (: > '{}'; sleep 4; : > '{}') </dev/null >/dev/null 2>&1 &\n\
         read -r stall\n",
        born.display(),
        survived.display()
    ))
}

/// The Windows twin: `cmd` blocking on `set /p` with a detached-console
/// `cmd` child of its own. Written as batch files rather than nested
/// `cmd /C` quoting because the quoting is the part most likely to be
/// wrong from a machine that cannot run it — and a quoting mistake that
/// leaves the child unspawned is exactly what the `born` control is
/// there to turn into a red test rather than a silent green one.
#[cfg(windows)]
fn stalling_tree(
    dir: &std::path::Path,
    born: &std::path::Path,
    survived: &std::path::Path,
) -> Vec<String> {
    let child = dir.join("stall-child.bat");
    std::fs::write(
        &child,
        format!(
            "@echo off\r\n\
             echo born>\"{}\"\r\n\
             ping -n 5 127.0.0.1 >NUL\r\n\
             echo alive>\"{}\"\r\n",
            born.display(),
            survived.display()
        ),
    )
    .unwrap();
    let parent = dir.join("stall-parent.bat");
    std::fs::write(
        &parent,
        format!(
            "@echo off\r\n\
             set /p hello=\r\n\
             start \"\" /B cmd /C \"{}\" >NUL 2>&1\r\n\
             set /p stall=\r\n",
            child.display()
        ),
    )
    .unwrap();
    vec![
        "cmd".into(),
        "/C".into(),
        parent.to_string_lossy().into_owned(),
    ]
}

/// The deadline kill must unblock the harness, not merely signal the
/// one process the harness holds a handle to. Both platforms have to
/// come back with a determinate `Failed(deadline)` inside the deadline
/// plus a bounded margin, and on both the driver really did have a
/// child of its own — the twenty-minute CI job timeout is the hang
/// backstop, never the assertion.
///
/// Windows carries the extra claim, because it is the platform where a
/// surviving grandchild holds the inherited pipes open and the harness
/// waits forever: the whole tree has to be gone, which a single-process
/// stall could not tell apart from "killed the child". The author
/// cannot run Windows — windows-latest in CI is the judge of that half.
#[test]
fn the_deadline_kill_unblocks_a_stalled_driver_tree() {
    let dir = tempfile::tempdir().unwrap();
    let born = dir.path().join("the-child-was-born");
    let survived = dir.path().join("the-child-outlived-the-kill");
    let driver = stalling_tree(dir.path(), &born, &survived);
    // Long enough that the driver has unmistakably reached its own
    // spawn before the watchdog fires: a deadline that raced the tree
    // into existence would let the Windows half pass without ever
    // having a tree to kill.
    let deadline = Duration::from_secs(2);

    let started = std::time::Instant::now();
    let report = DriverProcess::spawn(&driver, dir.path(), Some(deadline))
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

    // The child was scheduled to announce its survival well after the
    // deadline. Nothing may announce itself.
    #[cfg(windows)]
    {
        std::thread::sleep(Duration::from_secs(6));
        assert!(
            !survived.exists(),
            "the deadline kill took the driver's whole tree, not only \
             the process the harness held a handle to"
        );
    }
}

fn poison_child_lock(process: &DriverProcess) {
    let child = Arc::clone(&process.child);
    assert!(std::thread::spawn(move || {
        let _guard = child.lock().unwrap();
        panic!("poison child mutex for refusal-path proof");
    })
    .join()
    .is_err());
}

#[test]
fn poisoned_child_lock_never_panics_the_watchdog_or_finish_path() {
    let process = DriverProcess::spawn(
        &command("sleep 1"),
        std::path::Path::new("."),
        Some(Duration::from_millis(10)),
    )
    .unwrap();
    poison_child_lock(&process);
    let wait_until = std::time::Instant::now() + Duration::from_secs(1);
    while !process.timed_out.load(Ordering::SeqCst) && std::time::Instant::now() < wait_until {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(process.timed_out.load(Ordering::SeqCst));
    {
        let mut child = process.child.lock().unwrap_err().into_inner();
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(process);

    let process =
        DriverProcess::spawn(&command("sleep 0.05"), std::path::Path::new("."), None).unwrap();
    poison_child_lock(&process);
    let report = process.finish(
        AttemptOutcome::Failed {
            error: "test".into(),
        },
        None,
        Vec::new(),
        false,
    );
    assert!(matches!(report.outcome, AttemptOutcome::Failed { .. }));
}
