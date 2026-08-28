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

    let close_before_start = format!(
        "read line; exec 0<&-; printf '%s\\n' '{}'; sleep 1",
        capabilities()
    );
    assert!(matches!(
        run(&close_before_start).outcome,
        AttemptOutcome::Failed { error } if error.contains("could not send start")
    ));

    let process = DriverProcess::spawn(
        &command("exec 0<&-; sleep 1"),
        std::path::Path::new("."),
        None,
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(25));
    let report = process.run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {});
    assert!(matches!(
        report.outcome,
        AttemptOutcome::Failed { error } if error.contains("could not greet driver")
    ));
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
    );
    assert!(matches!(report.outcome, AttemptOutcome::Failed { .. }));
}
