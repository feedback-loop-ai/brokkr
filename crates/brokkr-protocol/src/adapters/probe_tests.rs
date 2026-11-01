//! The typed pre-session refusal and the probe's adapter bound
//! (decision 0103 ruling 1(a) and ruling 4(a)), at the adapter edge: a
//! fake harness refuses and then answers, a probe ends only on the first
//! model-output event, and dsh still classifies nothing.

use super::tests::executable;
use super::*;
use crate::env_guard::EnvGuard;
use crate::RefusalClass;

/// The same invocation, marked as a one-turn probe (decision 0103 ruling
/// 4(a)): a probe reads no session offer and streams no ordinary rows.
fn invoke_probe(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
) -> Result<Invocation, String> {
    super::tests::invoke_case(
        kind,
        extra,
        prompt,
        input,
        super::tests::Case {
            session: None,
            probe: true,
        },
        bindings,
        &mut |_| {},
    )
}

fn seat_start(dir: &std::path::Path) -> Value {
    json!({
        "effect_id": "effect",
        "attempt_id": "attempt",
        "input": {
            "workdir": dir,
            "result_path": dir.join("result.json"),
            "allowed_results": ["complete"],
            "feature": "f",
            "phase": "work",
            "role_path": "/dev/null",
        }
    })
}

#[cfg(unix)]
#[test]
fn a_fake_adapter_refuses_with_a_rate_limit_and_answers_on_the_next_attempt() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let once = dir.path().join("once");
    let shim = executable(
        dir.path(),
        "claude-refuse-then-answer",
        &format!(
            r#"#!/bin/sh
cat >/dev/null
if [ ! -f '{once}' ]; then
  : > '{once}'
  printf '%s\n' '{{"type":"system","subtype":"init","session_id":"refused-1"}}'
  printf '%s\n' '{{"type":"rate_limit_event","apiErrorStatus":429,"message":"You have reached your limit"}}'
  exit 1
fi
printf '%s\n' '{{"type":"system","subtype":"init","session_id":"answered-2"}}'
printf '%s\n' '{{"type":"assistant","message":{{"id":"m1","content":[{{"type":"text","text":"done"}}]}}}}'
printf '%s\n' '{{"type":"result","session_id":"answered-2","result":"ok"}}'
printf '%s\n' '{{"result":"complete"}}' > '{result}'
"#,
            once = once.display(),
            result = dir.path().join("result.json").display()
        ),
    );
    env.set("BROKKR_CLAUDE_BIN", &shim);
    let extra = ["--model".to_string(), "anthropic/claude-x".to_string()];
    let start = seat_start(dir.path());

    let mut first = Vec::new();
    run_seat(AdapterKind::Claude, &extra, &start, None, &mut |body| {
        first.push(body)
    });
    // Decision 0053's structural fail-to-start: no accepted, no
    // checkpoint, one failed result carrying the typed refusal.
    assert!(
        !first
            .iter()
            .any(|body| matches!(body, Body::Accepted { .. } | Body::Checkpoint { .. })),
        "a refusal must not accept or checkpoint: {first:?}"
    );
    let Body::Result {
        status: ResultStatus::Failed,
        refusal: Some(refusal),
        error: Some(error),
        ..
    } = first.last().unwrap()
    else {
        panic!("expected one failed result with a refusal: {first:?}")
    };
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert_eq!(refusal.provider(), "claude");
    assert_eq!(refusal.route(), Some("anthropic"));
    assert!(refusal.words().contains("rate_limit"), "{refusal}");
    assert!(refusal.words().contains("HTTP 429"), "{refusal}");
    assert!(
        refusal.words().contains("You have reached your limit"),
        "{refusal}"
    );
    assert!(error.contains("provider refused before the first turn: rate_limit"));

    // The next attempt proceeds: the fake harness answers, and no
    // refusal is read.
    let mut second = Vec::new();
    run_seat(AdapterKind::Claude, &extra, &start, None, &mut |body| {
        second.push(body)
    });
    assert!(
        second
            .iter()
            .any(|body| matches!(body, Body::Accepted { .. })),
        "the second attempt accepts: {second:?}"
    );
    let Body::Result {
        status: ResultStatus::Succeeded,
        refusal: None,
        ..
    } = second.last().unwrap()
    else {
        panic!("expected the second attempt to proceed: {second:?}")
    };
}

#[cfg(unix)]
#[test]
fn a_probe_ends_only_on_an_answer_and_admits_no_second() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "claude-two-answers",
        r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"system","subtype":"init","session_id":"s"}'
printf '%s\n' '{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"first"}]}}'
printf '%s\n' '{"type":"assistant","message":{"id":"m2","content":[{"type":"text","text":"second"}]}}'
printf '%s\n' '{"type":"result","session_id":"s","result":"ok"}'
"#,
    );
    env.set("BROKKR_CLAUDE_BIN", &shim);
    let invocation = invoke_probe(
        AdapterKind::Claude,
        &[],
        "prompt",
        &json!({"workdir": dir.path()}),
        &[],
    )
    .unwrap();
    let answer = invocation.answer.expect("the first answer");
    assert_eq!(answer["turn"], json!(1), "the first model output: {answer}");
}

/// A codex shim that starts a thread and a turn, then runs `tail`. The
/// stream before it is the same for every "started, then ended without an
/// answer" probe.
#[cfg(unix)]
fn codex_started_script(tail: &str) -> String {
    format!(
        r#"#!/bin/sh
read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"c","type":"capabilities","driver":"codex","version":"1","supports":[]}}'
read -r start
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"t","type":"thread.started","thread_id":"t1"}}'
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"s","type":"turn.started"}}'
{tail}
"#
    )
}

/// The answer a codex probe reports when the stream starts a turn and
/// then runs `tail` to its end.
#[cfg(unix)]
fn answer_after_codex_start(tail: &str) -> Option<Value> {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(dir.path(), "codex-probe", &codex_started_script(tail));
    env.set("BROKKR_CODEX_BIN", &shim);
    invoke_probe(
        AdapterKind::Codex,
        &[],
        "prompt",
        &json!({"workdir": dir.path()}),
        &[],
    )
    .unwrap()
    .answer
}

#[cfg(unix)]
#[test]
fn a_probe_that_starts_then_hangs_has_no_answer() {
    assert_eq!(
        answer_after_codex_start("sleep 0.2"),
        None,
        "turn.started is not an answer"
    );
}

#[cfg(unix)]
#[test]
fn a_probe_that_starts_then_errors_has_no_answer() {
    assert_eq!(
        answer_after_codex_start(
            "printf '%s\\n' '{\"proto\":\"forge-driver/v1\",\"msg_id\":\"e\",\"type\":\"error\",\"message\":\"boom\"}'; exit 1",
        ),
        None,
        "an error after turn.started is not an answer"
    );
}

#[test]
fn dsh_s_refusal_stays_none() {
    let stderr = std::thread::spawn(|| b"429 Too Many Requests: rate limit".to_vec());
    let run = finish_dsh(&mut Map::new(), 1, stderr, &[]).unwrap();
    assert_eq!(run.invocation.refusal, None);
}

#[test]
fn the_probe_advertisement_is_a_capabilities_field_not_a_supports_entry() {
    let hello = serde_json::to_string(&Message::new(Body::Hello {
        engine_version: "test".into(),
    }))
    .unwrap();
    for (kind, advertised) in [(AdapterKind::Claude, true), (AdapterKind::Exec, false)] {
        let mut out = Vec::new();
        start::serve_io(
            kind,
            &[],
            std::io::Cursor::new(format!("{hello}\n")),
            &mut out,
        )
        .unwrap();
        let line = String::from_utf8(out).unwrap();
        let message: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(
            message.get("probe").is_some(),
            advertised,
            "{kind:?}: {message}"
        );
        let supports = message["supports"].as_array().unwrap();
        assert!(
            !supports.iter().any(|entry| entry == "probe"),
            "the probe is never a supports entry: {message}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_probe_start_reports_succeeded_with_the_answer_checkpoint() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "claude-probe-answer",
        r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"answered"}]}}'
"#,
    );
    env.set("BROKKR_CLAUDE_BIN", &shim);
    let mut start = seat_start(dir.path());
    start["probe"] = json!(true);
    let mut messages = Vec::new();
    run_seat(AdapterKind::Claude, &[], &start, None, &mut |body| {
        messages.push(body)
    });
    let Body::Result {
        status: ResultStatus::Succeeded,
        result: Some(answer),
        refusal: None,
        ..
    } = messages.last().unwrap()
    else {
        panic!("a probe reports its answer: {messages:?}")
    };
    assert_eq!(answer["step"], json!("seat-turn"));
    assert_eq!(answer["turn"], json!(1));
    assert!(
        messages
            .iter()
            .any(|body| matches!(body, Body::Checkpoint { data, .. } if data == answer)),
        "the answer rides one checkpoint: {messages:?}"
    );
}

/// Decision 0053: the reason is one bounded line, never empty and never
/// the raw multi-line prose.
#[test]
fn a_refusal_reason_is_one_bounded_line() {
    let reason = refusal_reason(
        "rate_limit",
        Some(429),
        Some("You've reached your\n  limit\nfor this model"),
    )
    .to_string();
    assert!(reason.contains("rate_limit"));
    assert!(reason.contains("(HTTP 429)"));
    assert!(reason.contains("You've reached your limit for this model"));
    assert!(!reason.contains('\n'));
    let long = refusal_reason("api_error", None, Some(&"x".repeat(400))).to_string();
    assert!(long.len() <= "provider refused before the first turn: api_error: ".len() + 160);
    assert_eq!(
        refusal_reason("api_error", None, None).to_string(),
        "provider refused before the first turn: api_error"
    );
}

/// Decision 0053: a refusal is classified ONLY before the first counted
/// turn. The same record after a turn is decision 0016's mid-session
/// failure, which nothing here narrows.
#[test]
fn a_refusal_is_classified_only_before_the_first_counted_turn() {
    let refusal = json!({"type":"assistant","isApiErrorMessage":true,"error":"rate_limit",
        "message":{"content":[{"type":"text","text":"limit"}]}});
    let mut turns = ClaudeFold::default();
    let mut meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::None).unwrap();
    let mut emitted: Vec<Value> = Vec::new();
    assert!(
        fold_stream_event(&refusal, &mut turns, &mut meta, &mut transcript, &mut |c| {
            emitted.push(c.clone())
        })
        .is_some()
    );
    assert!(
        emitted.is_empty(),
        "a refusal emits no checkpoint: {emitted:?}"
    );
    fold_stream_event(
        &json!({"type":"assistant","message":{"content":[]}}),
        &mut turns,
        &mut meta,
        &mut transcript,
        &mut |c| emitted.push(c.clone()),
    );
    assert_eq!(turns.turns, 1);
    assert!(
        fold_stream_event(&refusal, &mut turns, &mut meta, &mut transcript, &mut |c| {
            emitted.push(c.clone())
        })
        .is_none()
    );

    let mut turn = 0;
    let mut meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::None).unwrap();
    let mut echo = CodexThreadEcho::default();
    let mut emitted: Vec<Value> = Vec::new();
    assert!(fold_codex_event(
        &json!({"type":"error","message":"rate limit"}),
        &mut turn,
        &mut meta,
        &mut transcript,
        &mut echo,
        &mut |c| emitted.push(c.clone())
    )
    .is_some());
    assert!(emitted.is_empty());
    fold_codex_event(
        &json!({"type":"turn.started"}),
        &mut turn,
        &mut meta,
        &mut transcript,
        &mut echo,
        &mut |c| emitted.push(c.clone()),
    );
    assert_eq!(turn, 1);
    assert!(fold_codex_event(
        &json!({"type":"error","message":"rate limit"}),
        &mut turn,
        &mut meta,
        &mut transcript,
        &mut echo,
        &mut |c| emitted.push(c.clone())
    )
    .is_none());
}
