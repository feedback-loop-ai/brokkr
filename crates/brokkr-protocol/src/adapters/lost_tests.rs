//! The built-in driver's lost results (decision 0006's 2026-10-11
//! addendum, rulings 1 and 2, `lost`): a command that was spawned and left
//! no result of its own gets no `failed` in its place, and the start that
//! sent nothing ends the driver. A refusal before the spawn, and a result
//! the command did leave, are reported as they always were. Every child
//! here is a real `sh`.

use super::start::serve_io;
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

/// The thread a codex rejoin is offered.
const OFFERED: &str = tests::THREAD;

/// The refusal a codex stream reports before its first turn.
const REFUSAL: &str = r#"printf '{"type":"error","message":"stream error: rate limit"}\n'"#;

/// A codex seat in `dir` on an enabled work shape, whose shim answers
/// its version and then runs `script`, rejoining `session` when it is
/// one: every body it sent, whether it answered, and what the shim
/// appended to its marker, once per run that reached it.
fn codex(dir: &Path, script: &str, session: Option<&str>) -> (Vec<Body>, bool, String) {
    codex_in(dir, dir, script, session)
}

/// [`codex`], the seat working in `workdir`.
fn codex_in(
    dir: &Path,
    workdir: &Path,
    script: &str,
    session: Option<&str>,
) -> (Vec<Body>, bool, String) {
    let mut env = EnvGuard::lock();
    let marker = dir.join("marker");
    std::fs::remove_file(&marker).ok();
    let version = tests::version_preamble(&format!("codex-cli {}", tests::CODEX_VERSION));
    let script = format!(
        "#!/bin/sh\n{version}cat >/dev/null\nprintf x >> '{}'\n{script}\n",
        marker.display()
    );
    env.set("BROKKR_CODEX_BIN", tests::executable(dir, "codex", &script));
    let mut input = tests::enabled_input(CODEX_SHAPE, tests::CODEX_VERSION, dir);
    for (key, value) in exec_start(dir, "feature", None)["input"]
        .as_object()
        .unwrap()
    {
        input[key] = value.clone();
    }
    input["workdir"] = json!(workdir);
    let start = json!({"effect_id": "effect", "attempt_id": "attempt", "input": input});
    let extra = ["--sandbox".to_string(), "read-only".into()];
    let mut bodies = Vec::new();
    let answered = run_seat(AdapterKind::Codex, &extra, &start, session, &mut |body| {
        bodies.push(body);
    });
    let marked = std::fs::read_to_string(&marker).unwrap_or_default();
    (bodies, answered, marked)
}

/// A refusal read from a command that a signal then ended, or from a
/// rejoin nobody confirmed, is no refusal to start: the command ran, so
/// no `failed` is sent that a retry would repeat. The same stream from a
/// cold launch that exited on its own is the refusal it always was.
#[test]
fn a_refusal_ended_by_a_signal_or_an_unsettled_rejoin_ends_lost() {
    let dir = tempfile::tempdir().unwrap();
    let ended = |script: &str, session| {
        let (bodies, answered, marked) = codex(dir.path(), script, session);
        (result(&bodies), answered, marked)
    };
    let lost = (None, false, "x".to_string());
    assert_eq!(ended(&format!("{REFUSAL}\nkill -9 $$"), None), lost);
    assert_eq!(ended(REFUSAL, Some(OFFERED)), lost);
    let refused = "provider refused before the first turn: api_error: stream error: rate limit";
    let failed = Some((ResultStatus::Failed, None, Some(refused.into())));
    assert_eq!(ended(REFUSAL, None), (failed, true, "x".into()));
}

/// The version probe observes the harness, and is not the seat's spawn: a
/// spawn refused after it, in a workdir that is not there, is the refusal
/// before the session it always was, and a retry may repeat it.
#[test]
fn a_spawn_refused_after_the_version_probe_is_still_sent() {
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("absent");
    let (bodies, answered, marked) = codex_in(dir.path(), &absent, "", None);
    let refused = "could not invoke the agent CLI: No such file or directory (os error 2)";
    let failed = Some((ResultStatus::Failed, None, Some(refused.into())));
    assert_eq!(
        (result(&bodies), answered, marked),
        (failed, true, String::new())
    );
}

/// A rejoin a signal ended is never replaced by a cold spawn within the
/// attempt: it may have acted, so the attempt ends lost with the command
/// run once. A rejoin the harness refused with a code of its own is still
/// replaced, once (proposed decision 0056 ruling 8).
#[test]
fn a_rejoin_ended_by_a_signal_is_never_replaced_cold() {
    let dir = tempfile::tempdir().unwrap();
    let rejoin = |refused: &str| {
        let script = format!(
            "case \"$*\" in *resume*) {refused} ;; esac\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{OFFERED}\"}}\\n'"
        );
        let (bodies, answered, marked) = codex(dir.path(), &script, Some(OFFERED));
        let replaced: Vec<Value> = bodies
            .iter()
            .filter_map(|body| match body {
                Body::Checkpoint { data, .. } if data["step"] == "harness-started" => {
                    Some(data["resume_refusal"].clone())
                }
                _ => None,
            })
            .collect();
        (result(&bodies), answered, marked, replaced)
    };
    assert_eq!(rejoin("kill -9 $$"), (None, false, "x".into(), Vec::new()));
    assert_eq!(
        rejoin("exit 1"),
        (None, false, "xx".into(), vec![json!("harness-refused")])
    );
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
