//! The built-in driver's lost results (decision 0006's 2026-10-11
//! addendum, rulings 1 and 2, `lost`): a command that was spawned and left
//! no result of its own gets no `failed` in its place, and the start that
//! sent nothing ends the driver. A refusal before the spawn, and a result
//! the command did leave, are reported as they always were. Every child
//! here is a real `sh`.

use super::*;
use crate::env_guard::EnvGuard;

/// An exec seat running `script` in `dir`, on `feature`, as a dialect
/// step when `dialect` is one.
fn exec_start(dir: &Path, feature: &str, dialect: Option<Value>) -> Value {
    let mut input = json!({
        "feature": feature, "phase": "work", "workdir": dir,
        "result_path": dir.join("result.json"), "allowed_results": ["complete"],
    });
    if let Some(dialect) = dialect {
        input["dialect_exec"] = dialect;
    }
    json!({"effect_id": "effect", "attempt_id": "attempt", "input": input})
}

/// Every body the seat sent, and whether `run_seat` said it answered.
fn exec(extra: &[&str], start: &Value) -> (Vec<Body>, bool) {
    let extra: Vec<String> = extra.iter().map(|part| part.to_string()).collect();
    let mut bodies = Vec::new();
    let answered = run_seat(AdapterKind::Exec, &extra, start, None, &mut |body| {
        bodies.push(body);
    });
    (bodies, answered)
}

/// The result the seat sent, if it sent one: its status, payload and error.
fn result(bodies: &[Body]) -> Option<(ResultStatus, Option<Value>, Option<String>)> {
    let mut results = bodies.iter().filter_map(|body| match body {
        Body::Result {
            status,
            result,
            error,
            ..
        } => Some((*status, result.clone(), error.clone())),
        _ => None,
    });
    let first = results.next();
    assert!(results.next().is_none(), "one result at most: {bodies:?}");
    first
}

#[test]
fn a_command_that_exits_zero_without_a_result_ends_lost() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let start = exec_start(dir.path(), "feature", None);

    let (bodies, answered) = exec(&["sh", "-c", "cat >/dev/null"], &start);
    assert_eq!(result(&bodies), None);
    assert!(!answered);
    assert!(
        matches!(bodies.first(), Some(Body::Accepted { .. })),
        "{bodies:?}"
    );

    // The command's own report is still read: a non-zero exit is its
    // failure, and a result file is its result.
    let (bodies, answered) = exec(&["sh", "-c", "cat >/dev/null; exit 3"], &start);
    let failed = (
        ResultStatus::Failed,
        None,
        Some("agent CLI exited 3".into()),
    );
    assert_eq!(result(&bodies), Some(failed));
    assert!(answered);
    let wrote = "cat >/dev/null; printf '{\"result\":\"complete\"}' > result.json";
    let (bodies, answered) = exec(&["sh", "-c", wrote], &start);
    let (status, payload, _) = result(&bodies).unwrap();
    assert_eq!(status, ResultStatus::Succeeded);
    assert_eq!(payload.unwrap()["result"], "complete");
    assert!(answered);
}

#[test]
fn a_prompt_write_failure_after_the_spawn_ends_lost() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    // A prompt larger than a pipe holds, to a command that closes its
    // stdin before it reads any: the write fails after the spawn.
    let start = exec_start(dir.path(), &"x".repeat(1 << 20), None);
    let (bodies, answered) = exec(&["sh", "-c", "exec 0<&-; sleep 1"], &start);
    assert_eq!(result(&bodies), None);
    assert!(!answered);

    // A command that cannot be spawned never ran, so its refusal is still
    // sent, and a retry may repeat it.
    let absent = dir.path().join("absent");
    let (bodies, answered) = exec(&[absent.to_str().unwrap()], &start);
    let refused = "could not invoke the agent CLI: No such file or directory (os error 2)";
    assert_eq!(
        result(&bodies),
        Some((ResultStatus::Failed, None, Some(refused.into())))
    );
    assert!(answered);
}

/// A command ended by a signal returned no exit status, so it left no
/// result, a dialect step's included: the signal is read before the
/// dialect's words. A dialect step's own non-zero exit keeps its failure
/// word.
#[test]
fn a_command_ended_by_a_signal_ends_lost_even_as_a_dialect_step() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let killed = ["sh", "-c", "cat >/dev/null; kill -9 $$"];
    for dialect in [None, Some(json!({}))] {
        let start = exec_start(dir.path(), "feature", dialect.clone());
        let (bodies, answered) = exec(&killed, &start);
        assert_eq!(result(&bodies), None, "{dialect:?}");
        assert!(!answered, "{dialect:?}");
    }
    let start = exec_start(dir.path(), "feature", Some(json!({})));
    let (bodies, answered) = exec(&["sh", "-c", "cat >/dev/null; exit 7"], &start);
    let (status, payload, _) = result(&bodies).unwrap();
    assert_eq!(status, ResultStatus::Succeeded);
    assert_eq!(payload.unwrap()["result"], "fail");
    assert!(answered);
}

/// A start that sent no result ends the serve loop, so the engine reads
/// the result lost at once rather than at its deadline. One that sent a
/// result serves on.
#[test]
fn a_start_that_sent_no_result_ends_the_driver() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let start = exec_start(dir.path(), "feature", None);
    let line = |body: Body| serde_json::to_string(&Message::new(body)).unwrap();
    let hello = line(Body::Hello {
        engine_version: "test".into(),
    });
    let start = line(Body::Start {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        seat: "work".into(),
        input: start["input"].clone(),
    });
    let served = |script: &str| {
        let extra = ["sh".to_string(), "-c".into(), script.into()];
        let input = format!("{hello}\n{start}\n{hello}\n");
        let mut output = Vec::new();
        serve_io(AdapterKind::Exec, &extra, input.as_bytes(), &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        let types: Vec<String> = output
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap()["type"].to_string())
            .collect();
        types
    };
    let lost = served("cat >/dev/null");
    assert_eq!(lost.first().map(String::as_str), Some("\"capabilities\""));
    assert_eq!(
        lost.iter()
            .filter(|kind| *kind == "\"capabilities\"")
            .count(),
        1,
        "{lost:?}"
    );
    assert!(!lost.contains(&"\"result\"".to_string()), "{lost:?}");
    let answered = served("cat >/dev/null; exit 3");
    assert_eq!(
        answered.last().map(String::as_str),
        Some("\"capabilities\""),
        "{answered:?}"
    );
    assert!(answered.contains(&"\"result\"".to_string()), "{answered:?}");
}

/// An unsettled rejoin's reason, which now goes to stderr in place of a
/// result, in the words it was always refused in.
#[test]
fn an_unsettled_rejoin_keeps_its_reason() {
    assert_eq!(
        lost::unsettled_reason(LaunchTerminal::Unconfirmed),
        "provider never confirmed the offered session; refusing to accept the invocation"
    );
    assert_eq!(
        lost::unsettled_reason(LaunchTerminal::Mismatch),
        "provider named a different session than the offered root; refusing to accept the \
         invocation"
    );
}
