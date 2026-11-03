//! The coverage round's added sites (decision 0103 slice 4a, #430): each
//! test reaches one arm the slice introduced and names the refusal, answer
//! or report the driver makes there.

use super::*;
use crate::env_guard::EnvGuard;

/// One exec start driven through production's seat body, with every wire
/// body it sent. Both start-refusal cases below differ only in the start
/// they hand it.
fn exec_start(start: &Value) -> Vec<Body> {
    let mut messages = Vec::new();
    run_seat(AdapterKind::Exec, &[], start, None, &mut |body| {
        messages.push(body)
    });
    messages
}

/// A start's probe marker is typed before anything launches (decision 0103
/// ruling 4(a)): a present field that is not the marker is a refused start
/// with no `accepted`, never read as an ordinary attempt.
#[test]
fn a_start_with_a_false_probe_marker_refuses_before_it_launches() {
    let mut start = json!({"effect_id": "effect", "attempt_id": "attempt"});
    start["probe"] = json!(false);
    start["input"] = json!({"workdir": "."});
    let messages = exec_start(&start);
    assert!(
        !messages
            .iter()
            .any(|body| matches!(body, Body::Accepted { .. })),
        "a refused probe start admits no accepted: {messages:?}"
    );
    assert_eq!(messages.len(), 1, "{messages:?}");
    match &messages[0] {
        Body::Result {
            status: ResultStatus::Failed,
            error: Some(error),
            ..
        } => assert_eq!(
            error,
            "seat refused to start: the start's `probe` field is present but not true"
        ),
        other => panic!("a malformed probe marker is a failed result: {other:?}"),
    }
}

/// A probe's refusal is not an ordinary invocation (decision 0103 ruling
/// 4(a)): when its invocation cannot launch, it admits no `accepted` even
/// though an ordinary attempt does. The failing launch is exec's empty
/// command, so nothing spawns and the result is the driver's own failure.
#[test]
fn a_probe_that_cannot_launch_admits_no_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let mut start = json!({"effect_id": "effect", "attempt_id": "attempt", "probe": true});
    start["input"] = json!({"workdir": dir.path()});
    let messages = exec_start(&start);
    assert!(
        !messages
            .iter()
            .any(|body| matches!(body, Body::Accepted { .. })),
        "a probe admits no accepted: {messages:?}"
    );
    assert!(
        messages.iter().any(|body| matches!(
            body,
            Body::Result {
                status: ResultStatus::Failed,
                ..
            }
        )),
        "the failed invocation is reported: {messages:?}"
    );
}

/// A governed dsh launch is served only through its sealed record and
/// serving inputs (rebuild unit 15-fix-b; decision 0071 ruling 3): a plan
/// the engine wrote without them is refused at the driver's serving check,
/// before any spawn. `dsh_launch` itself succeeds — the check is the one
/// that refuses — so this reaches that error arm, not a planner refusal.
#[cfg(unix)]
#[test]
fn a_governed_dsh_launch_without_its_sealed_record_is_refused_at_the_serving_check() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = super::tests::executable(dir.path(), "dsh-governed", "#!/bin/sh\nexit 0\n");
    env.set("BROKKR_DSH_BIN", &shim);
    env.set("DSH_HOME", dir.path());
    env.set("HOME", dir.path());
    let mut input = json!({"workdir": dir.path()});
    input["native_controls"] = json!({
        "inventory": "unmeasured",
        "provider": "dsh",
        "harness": "dsh",
        "reason": "dsh declares its native inventory unmeasured",
    });
    input["launch_arguments"] = json!({"authored": [], "managed": []});
    let refused = match super::tests::invoke_case(
        AdapterKind::Dsh,
        &[],
        "prompt",
        &input,
        super::tests::Case {
            session: None,
            probe: false,
        },
        &[],
        &mut |_| {},
    ) {
        Ok(_) => panic!("a plan without its sealed pair is never served unchecked"),
        Err(error) => error,
    };
    assert_eq!(refused, super::mcp::UNSEALED);
}

/// Ruling 8's replacement is still an invocation (decision 0053). When the
/// cold replacement cannot spawn at all, its own error is returned instead
/// of the rejected rejoin's outcome being reported as if it had run.
#[cfg(unix)]
#[test]
fn a_rejected_rejoins_cold_replacement_that_cannot_spawn_fails_the_invocation() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let version =
        super::tests::version_preamble(&format!("codex-cli {}", super::tests::CODEX_VERSION));
    let shim = super::tests::executable(
        dir.path(),
        "codex-vanishes-before-its-replacement",
        &format!("#!/bin/sh\n{version}cat >/dev/null\nrm -f \"$0\"\nexit 1\n"),
    );
    env.set("BROKKR_CODEX_BIN", &shim);
    let input =
        super::tests::enabled_input(super::CODEX_SHAPE, super::tests::CODEX_VERSION, dir.path());
    let refused = match super::tests::invoke_case(
        AdapterKind::Codex,
        &["--sandbox".to_string(), "read-only".into()],
        "prompt",
        &input,
        super::tests::Case {
            session: Some(super::tests::THREAD),
            probe: false,
        },
        &[],
        &mut |_| {},
    ) {
        Ok(_) => panic!("the replacement has no executable to spawn"),
        Err(error) => error,
    };
    assert!(
        !shim.exists(),
        "the rejected rejoin ran and took its executable with it"
    );
    // The replacement fails at spawn because that path is gone. The same
    // missing path yields the OS's own typed error here, so the expected
    // text is the boundary's context plus that error's Display, never a
    // prefix that could hide a different cause (#430 M1; decision 0071
    // rulings 8 and 9).
    let missing = std::process::Command::new(&shim)
        .spawn()
        .expect_err("the replacement's path is gone");
    assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(
        refused,
        format!("could not invoke the agent CLI: {missing}")
    );
}

/// The stream-json route's wait is the same injected seam the shipped
/// route has (decision 0071 rulings 5 and 10): a child whose stdout closed
/// and whose wait then fails ends in the one `agent CLI did not conclude`
/// refusal instead of spinning on the transcript forever.
#[cfg(unix)]
#[test]
fn a_qualified_seat_whose_child_cannot_be_waited_on_concludes_instead_of_spinning() {
    let dir = tempfile::tempdir().unwrap();
    let shim = super::tests::executable(
        dir.path(),
        "dsh-stream-unwaitable",
        "#!/bin/sh\nexec 1>&-\nsleep 0.2\n",
    );
    let launch = super::tests::dsh_stream_launch(&shim, &dir.path().join("seat"), None, None);
    let command = [launch.command.clone(), vec!["the prompt".to_string()]].concat();
    let refused = match super::dsh_launch::invoke_dsh_launch_observed(
        launch,
        command,
        &[],
        &mut ProbeAnswer::new(false),
        &mut |_| {},
        |child| {
            // The child is reaped here rather than left behind: the refusal
            // is about what the driver does when it cannot ask.
            let _ = child.wait();
            Err(std::io::Error::other("no child processes"))
        },
        &mut |_| {},
    ) {
        Ok(_) => panic!("a child that cannot be waited on is not a success"),
        Err(error) => error,
    };
    // The seam's typed `io::Error` is retained and the ONE text boundary
    // prints its Display verbatim: pin the whole refusal, not fragments
    // that a different cause or context could share (#430 M1; decision
    // 0071 rulings 8 and 9).
    assert_eq!(refused, "agent CLI did not conclude: no child processes");
}
