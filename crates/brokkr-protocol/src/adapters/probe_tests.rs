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

/// A dsh shim that writes one `assistant/message` into the transcript
/// under the seat overlay's root, exactly as the shipped headless runner
/// does. The caller appends how the child ends.
#[cfg(unix)]
fn dsh_probe_answer_script() -> String {
    let mut script = String::from(
        r#"#!/bin/sh
while [ "$1" != --patch ] && [ $# -gt 0 ]; do shift; done
shift
root=$(awk -F"'" '/^    root: /{print $2}' "$1")
d="$root/--project--/session-fake"
mkdir -p "$d"
f="$d/session.v3.jsonl"
printf '{"type":"session","version":0,"id":"session-fake-1","cwd":"/w"}\n' > "$f"
"#,
    );
    script.push_str(
        r#"printf '{"type":"assistant/message","data":{"turn":1,"step":1,"message":{"source":{"model":"served-by-dsh"}},"usage":{"inputTokens":10,"outputTokens":2}}}\n' >> "$f"
"#,
    );
    script
}

/// A harness that answers its adapter once and then hangs until `release`
/// exists. The file exists only so a probe that fails to stop can be let
/// go and its failure asserted, rather than hanging the suite.
#[cfg(unix)]
fn answer_then_hang_script(kind: AdapterKind, release: &std::path::Path) -> String {
    let head = match kind {
        AdapterKind::Claude => {
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"answered"}]}}'
"#
            .to_string()
        }
        AdapterKind::Codex => {
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"thread.started","thread_id":"t1"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.completed","item":{"type":"agent_message","text":"answered"}}'
"#
            .to_string()
        }
        AdapterKind::Dsh => dsh_probe_answer_script(),
        AdapterKind::Lanetally | AdapterKind::Exec => {
            unreachable!("the probe answers on claude, codex and dsh")
        }
    };
    format!(
        "{head}while [ ! -f '{}' ]; do sleep 0.02; done\n",
        release.display()
    )
}

/// Run one probe in a thread and wait no longer than `bound` for it. On a
/// timeout the release file lets the hung shim end, the thread is joined,
/// and the caller's elapsed assertion fails with the late report in hand.
#[cfg(unix)]
fn run_probe_before(
    kind: AdapterKind,
    extra: &[String],
    start: &Value,
    bound: std::time::Duration,
    release: &std::path::Path,
) -> (Vec<Body>, std::time::Duration) {
    let (tx, rx) = std::sync::mpsc::channel();
    let start = start.clone();
    let extra = extra.to_vec();
    let handle = std::thread::spawn(move || {
        let began = std::time::Instant::now();
        let mut messages = Vec::new();
        run_seat(kind, &extra, &start, None, &mut |body| messages.push(body));
        let _ = tx.send((messages, began.elapsed()));
    });
    match rx.recv_timeout(bound) {
        Ok(done) => {
            handle.join().unwrap();
            done
        }
        Err(_) => {
            std::fs::write(release, b"").unwrap();
            handle.join().unwrap();
            rx.recv().unwrap()
        }
    }
}

/// A probe's answer is its whole report: the harness is stopped there, so
/// an answer followed by a hang is reported succeeded, on every adapter
/// whose stream can answer (decision 0103 ruling 4(a)).
#[cfg(unix)]
#[test]
fn a_probe_that_answers_then_hangs_reports_its_answer() {
    let mut env = EnvGuard::lock();
    let bound = std::time::Duration::from_secs(5);
    for kind in [AdapterKind::Claude, AdapterKind::Codex, AdapterKind::Dsh] {
        let dir = tempfile::tempdir().unwrap();
        let release = dir.path().join("release");
        let shim = executable(
            dir.path(),
            &format!("{kind:?}-answer-then-hang"),
            &answer_then_hang_script(kind, &release),
        );
        let extra = match kind {
            AdapterKind::Claude => {
                env.set("BROKKR_CLAUDE_BIN", &shim);
                vec!["--model".to_string(), "anthropic/claude-x".to_string()]
            }
            AdapterKind::Codex => {
                env.set("BROKKR_CODEX_BIN", &shim);
                Vec::new()
            }
            AdapterKind::Dsh => {
                env.set("BROKKR_DSH_BIN", &shim);
                env.set("DSH_HOME", dir.path());
                Vec::new()
            }
            AdapterKind::Lanetally | AdapterKind::Exec => {
                unreachable!("the probe answers on claude, codex and dsh")
            }
        };
        let mut start = seat_start(dir.path());
        start["probe"] = json!(true);
        let (messages, elapsed) = run_probe_before(kind, &extra, &start, bound, &release);
        assert!(
            elapsed < bound,
            "{kind:?}: the probe waited for the harness to end: {elapsed:?}; {messages:?}"
        );
        let Body::Result {
            status: ResultStatus::Succeeded,
            result: Some(answer),
            refusal: None,
            ..
        } = messages.last().unwrap()
        else {
            panic!("{kind:?}: the answer is the whole report: {messages:?}")
        };
        assert_eq!(answer["turn"], json!(1), "{kind:?}: {answer}");
    }
}

/// #430 H1: a probe stopped at its answer keeps the stderr the harness
/// already wrote. The stop removed the hang, not the evidence; a route
/// that discarded the bytes lost what the journal is owed.
#[cfg(unix)]
#[test]
fn a_stopped_probe_keeps_its_stderr() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let cases = [
        (
            AdapterKind::Claude,
            r#"#!/bin/sh
cat >/dev/null
printf 'wrapper said this\n' >&2
printf '%s\n' '{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"answered"}]}}'
"#,
        ),
        (
            AdapterKind::Codex,
            r#"#!/bin/sh
cat >/dev/null
printf 'wrapper said this\n' >&2
printf '%s\n' '{"type":"thread.started","thread_id":"t1"}'
printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.completed","item":{"type":"agent_message","text":"answered"}}'
"#,
        ),
    ];
    for (kind, script) in cases {
        let shim = executable(dir.path(), &format!("{kind:?}-stopped-stderr"), script);
        match kind {
            AdapterKind::Claude => env.set("BROKKR_CLAUDE_BIN", &shim),
            AdapterKind::Codex => env.set("BROKKR_CODEX_BIN", &shim),
            AdapterKind::Lanetally | AdapterKind::Dsh | AdapterKind::Exec => {
                unreachable!("the stop's stderr is proven on claude and codex")
            }
        }
        let invocation =
            invoke_probe(kind, &[], "prompt", &json!({"workdir": dir.path()}), &[]).unwrap();
        assert!(invocation.answer.is_some(), "{kind:?}: the probe answered");
        assert!(
            invocation.stderr.contains("wrapper said this"),
            "{kind:?}: the stopped probe kept its stderr: {}",
            invocation.stderr
        );
    }
}

/// #430 H1: a stopped probe whose stderr reader failed records that
/// failure in its evidence through the same `evidence` the claude and
/// codex stops call, never as a clean end.
#[test]
fn review_an_unreadable_stderr_capture_is_named_in_a_stopped_probes_evidence() {
    struct Failed;
    impl std::io::Read for Failed {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "stderr seam failed",
            ))
        }
    }
    let evidence = super::stderr::capture_reader(Failed)
        .take(true)
        .evidence(|_| String::new());
    assert!(
        evidence.contains("stderr capture ended unreadable"),
        "the stopped capture's failure is named: {evidence:?}"
    );
}

/// A probe refused `rate_limit` before any answer carries that refusal,
/// typed, exactly as an ordinary attempt does (decision 0103 rulings 1(a)
/// and 4(a)); it is never erased into "ended without an answer".
#[cfg(unix)]
#[test]
fn a_probe_refused_rate_limit_reports_that_refusal() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "claude-probe-refused",
        r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"rate_limit_event","apiErrorStatus":429,"message":"You have reached your limit"}'
"#,
    );
    env.set("BROKKR_CLAUDE_BIN", &shim);
    let extra = ["--model".to_string(), "anthropic/claude-x".to_string()];
    let mut start = seat_start(dir.path());
    start["probe"] = json!(true);
    let mut messages = Vec::new();
    run_seat(AdapterKind::Claude, &extra, &start, None, &mut |body| {
        messages.push(body)
    });
    assert!(
        !messages
            .iter()
            .any(|body| matches!(body, Body::Accepted { .. } | Body::Checkpoint { .. })),
        "a probe refusal accepts or checkpoints nothing: {messages:?}"
    );
    let result = messages.last().unwrap();
    let Body::Result {
        status,
        refusal,
        error,
        ..
    } = result
    else {
        panic!("the probe reports its typed refusal: {messages:?}")
    };
    assert_eq!(*status, ResultStatus::Failed);
    let refusal = refusal
        .as_ref()
        .expect("the probe reports its typed refusal");
    let error = error.as_deref().expect("the probe states why it ended");
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert_eq!(refusal.provider(), "claude");
    assert_eq!(refusal.route(), Some("anthropic"));
    assert!(refusal.words().contains("HTTP 429"), "{refusal}");
    assert!(
        refusal.words().contains("You have reached your limit"),
        "{refusal}"
    );
    assert_eq!(error, "probe ended without an answer");
}

/// What a Claude seat pinned to `model` reports when its harness answers
/// only a 429 `rate_limit_event`.
#[cfg(unix)]
fn rate_limited_claude_seat(model: &str) -> Vec<Body> {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "claude-rate-limited",
        r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"rate_limit_event","apiErrorStatus":429,"message":"You have reached your limit"}'
"#,
    );
    env.set("BROKKR_CLAUDE_BIN", &shim);
    let extra = ["--model".to_string(), model.to_string()];
    let mut messages = Vec::new();
    run_seat(
        AdapterKind::Claude,
        &extra,
        &seat_start(dir.path()),
        None,
        &mut |body| messages.push(body),
    );
    messages
}

/// #430 L2 (decision 0071 ruling 3): a typed refusal the producer attributed
/// is always one its own reader accepts. A Claude seat pinned to
/// `featherless:qwen/qwen3` derives the route `featherless:qwen` (decision
/// 0036 ruling 2: the id's prefix before its first `/`), and the serialized
/// `Body::Result` refusal must read back as the identical typed object, not
/// as `RouteForm`.
#[cfg(unix)]
#[test]
fn chief_review_attributed_refusal_must_roundtrip_its_own_reader() {
    let messages = rate_limited_claude_seat("featherless:qwen/qwen3");
    let Body::Result {
        refusal: Some(refusal),
        ..
    } = messages.last().expect("the seat reports its refusal")
    else {
        panic!("a classified refusal is carried on the result: {messages:?}");
    };
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert_eq!(refusal.provider(), "claude");
    assert_eq!(refusal.route(), Some("featherless:qwen"));
    let serialized = serde_json::to_value(refusal).expect("the refusal serializes");
    assert_eq!(
        crate::PreSessionRefusal::read(&serialized),
        Ok(refusal.clone()),
        "the producer's refusal survives its own reader: {serialized}"
    );
}

/// A refusal whose route the wire's one form cannot carry is refused before
/// it is emitted: the invocation fails by name, and because the harness was
/// spawned no result is sent (the engine reads the attempt as lost after its
/// start), so no refusal reaches the engine rewritten or unreadable (fix
/// round 430s4a6 L2).
#[cfg(unix)]
#[test]
fn a_refusal_whose_route_the_wire_cannot_carry_fails_the_invocation_by_name() {
    let messages = rate_limited_claude_seat("bad!route/qwen3");
    assert!(
        !messages
            .iter()
            .any(|body| matches!(body, Body::Result { .. })),
        "an unformed route sends no result, and so no refusal: {messages:?}"
    );
}

/// dsh's one model-output event answers a probe and the answer is carried
/// on the report; dsh still classifies no refusal (decision 0103 ruling
/// 4(a); decision 0053's dsh gap stays).
#[cfg(unix)]
#[test]
fn a_dsh_probe_that_answers_reports_succeeded_with_that_answer() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "dsh-probe-answer",
        &format!("{}exit 0\n", dsh_probe_answer_script()),
    );
    env.set("BROKKR_DSH_BIN", &shim);
    env.set("DSH_HOME", dir.path());
    let mut start = seat_start(dir.path());
    start["probe"] = json!(true);
    let mut messages = Vec::new();
    run_seat(AdapterKind::Dsh, &[], &start, None, &mut |body| {
        messages.push(body)
    });
    let report = messages.last().unwrap();
    let Body::Result {
        status,
        result: answer,
        refusal,
        ..
    } = report
    else {
        panic!("a dsh probe reports its answer: {messages:?}")
    };
    assert_eq!(*status, ResultStatus::Succeeded);
    assert!(refusal.is_none(), "dsh still classifies no refusal");
    let answer = answer.as_ref().expect("the dsh answer is carried");
    assert_eq!(answer["step"], json!("seat-turn"), "{answer}");
    assert_eq!(answer["turn"], json!(1), "{answer}");
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
    let raw = b"429 Too Many Requests: rate limit".to_vec();
    let captured = super::stderr::capture_reader(std::io::Cursor::new(raw)).take(false);
    let run = finish_dsh(&mut Map::new(), 1, captured, None, &[]).unwrap();
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

/// The valid session header and one `assistant/message` answer appended to
/// `file`, as the headless runner writes them once turn 1 completes.
#[cfg(unix)]
fn dsh_append_answer(file: &std::path::Path) -> String {
    format!(
        "printf '{{\"type\":\"session\",\"version\":3,\"id\":\"session-1\",\"delegationDepth\":0}}\\n' > '{file}'\n\
         printf '{{\"type\":\"assistant/message\",\"seq\":0,\"data\":{{\"message\":{{\"source\":{{\"model\":\"deepseek-flash\"}}}},\"usage\":{{\"inputTokens\":5,\"outputTokens\":2}}}}}}\\n' >> '{file}'\n",
        file = file.display(),
    )
}

/// One qualified probe shim: the init event, then `before_answer` (what
/// stdout does before the transcript moves), then the answer and a hang.
#[cfg(unix)]
fn dsh_probe_body(file: &std::path::Path, before_answer: &str) -> String {
    format!(
        "#!/bin/sh\n\
         printf '%s\\n' '{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\"}}'\n\
         {before_answer}\
         {answer}\
         sleep 2\n",
        answer = dsh_append_answer(file),
    )
}

/// Drive one qualified cold probe whose shim is `body` and report what it
/// answered beside the wall time.
#[cfg(unix)]
fn run_qualified_probe(
    dir: &std::path::Path,
    name: &str,
    requested: bool,
    body: impl FnOnce(&std::path::Path) -> String,
) -> (Invocation, std::time::Duration) {
    let root = dir.join("seat");
    let transcript = root.join("--w--").join("session-1").join(DSH_TRANSCRIPT);
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    let shim = executable(dir, name, &body(&transcript));
    let launch = super::tests::dsh_stream_launch(&shim, &root, None, None);
    let command = [launch.command.clone(), vec!["the prompt".to_string()]].concat();
    let began = std::time::Instant::now();
    let invocation = super::dsh_launch::invoke_dsh_launch_observed(
        launch,
        command,
        &[],
        &mut ProbeAnswer::new(requested),
        &mut |_| {},
        super::tests::wait_for_exit,
        &mut |_| {},
    )
    .expect("the qualified probe reports");
    (invocation, began.elapsed())
}

/// A qualified probe must report its turn-1 answer inside `bound`; the
/// `waited` phrase names what stdout was doing so a failure reads as the
/// delay it was.
#[cfg(unix)]
fn assert_answered_under(
    (invocation, elapsed): (Invocation, std::time::Duration),
    bound: std::time::Duration,
    waited: &str,
) {
    assert!(
        elapsed < bound,
        "the probe waited for {waited}: {elapsed:?}"
    );
    let answer = invocation.answer.expect("the probe reports its answer");
    assert_eq!(answer["step"], json!("seat-turn"), "{answer}");
    assert_eq!(answer["turn"], json!(1), "{answer}");
}

/// #430 M2 (decision 0103 ruling 4(a); decision 0071 ruling 9): a probe
/// whose answer is appended while stdout stays silent must report at the
/// answer, not wait for the child to end.
#[cfg(unix)]
#[test]
fn a_qualified_dsh_probe_that_answers_then_hangs_reports_its_answer() {
    let dir = tempfile::tempdir().unwrap();
    let report = run_qualified_probe(dir.path(), "dsh-qualified-probe-answer", true, |file| {
        dsh_probe_body(file, "sleep 0.2\n")
    });
    assert_answered_under(report, std::time::Duration::from_secs(1), "stdout silence");
}

/// #430 M2 (review finding, decision 0103 ruling 4(a)): stdout closing
/// before the child does must not delay the transcript's answer. The old
/// route broke its loop at stdout EOF and then waited out the child.
#[cfg(unix)]
#[test]
fn review_qualified_probe_stdout_eof_does_not_delay_transcript_answer() {
    let dir = tempfile::tempdir().unwrap();
    let report = run_qualified_probe(dir.path(), "dsh-qualified-probe-eof", true, |file| {
        dsh_probe_body(file, "exec 1>&-\nsleep 0.2\n")
    });
    assert_answered_under(report, std::time::Duration::from_secs(1), "stdout EOF");
}

/// #430 M2 (review finding, decision 0103 ruling 4(a)): stdout noise must
/// not delay the transcript's answer either. The old route polled only on
/// a receive timeout, so a line every 20 ms held the transcript off.
#[cfg(unix)]
#[test]
fn review_qualified_probe_stdout_noise_does_not_delay_transcript_answer() {
    let dir = tempfile::tempdir().unwrap();
    let report = run_qualified_probe(dir.path(), "dsh-qualified-probe-noise", true, |file| {
        dsh_probe_body(
            file,
            "( i=0; while [ $i -lt 100 ]; do printf 'not a stream line\\n'; sleep 0.02; i=$((i+1)); done ) &\nsleep 0.2\n",
        )
    });
    assert_answered_under(report, std::time::Duration::from_secs(1), "stdout noise");
}

/// #430 M1 (returned review, decision 0103 ruling 4(a); decision 0071
/// ruling 9): a stdout line the driver cannot decode ends DECODING, not
/// the transcript's wait. The old route broke its loop at the unreadable
/// line and then waited out the child, so an answer appended while that
/// child slept was reported only at its exit.
#[cfg(unix)]
#[test]
fn review_qualified_probe_unreadable_stdout_does_not_delay_transcript_answer() {
    let dir = tempfile::tempdir().unwrap();
    let report = run_qualified_probe(dir.path(), "dsh-qualified-probe-unreadable", true, |file| {
        dsh_probe_body(file, "printf '\\377\\n'\nsleep 0.2\n")
    });
    assert_answered_under(
        report,
        std::time::Duration::from_secs(1),
        "stdout's unreadable line",
    );
}

/// #430 M1 (decision 0103 ruling 4(a); decision 0071 ruling 9): an
/// unreadable stdout line closes stdout for the reader too. The old reader
/// kept reading and filling a channel no one drained, so the child ran on
/// to its sleep; now its next write meets a closed pipe and it exits.
#[cfg(unix)]
#[test]
fn review_ordinary_qualified_dsh_closes_stdout_after_an_unreadable_line() {
    let dir = tempfile::tempdir().unwrap();
    let (invocation, elapsed) = run_qualified_probe(
        dir.path(),
        "dsh-qualified-probe-closes-stdout",
        false,
        |file| {
            dsh_probe_body(
                file,
                "set -e\nprintf '\\377\\n'\nsleep 0.1\n\
                 i=0\nwhile [ $i -lt 10000 ]; do printf 'progress %s\\n' \"$i\"; i=$((i+1)); done\n\
                 sleep 2\n",
            )
        },
    );
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "the reader did not close stdout at the unreadable line: {elapsed:?}"
    );
    assert_eq!(invocation.answer, None, "no probe was requested");
}

/// The probe's bound at its own edge (decision 0103 ruling 4(a)): an
/// unrequested invocation, an event that is not an answer, and a second
/// answer after the first each admit nothing, and Exec has no model turn to
/// answer with. The first answer is the whole report.
#[test]
fn a_probe_admits_only_the_first_answer_and_exec_has_none() {
    let answer = json!({
        "type": "assistant",
        "message": {"content": [{"type": "text", "text": "answered"}]},
    });
    let row = json!({"step": "seat-turn", "turn": 1});

    let mut unrequested = ProbeAnswer::new(false);
    unrequested.admit(AdapterKind::Claude, &answer, std::slice::from_ref(&row));
    assert!(
        !unrequested.answered(),
        "an ordinary invocation admits no answer"
    );

    let mut probe = ProbeAnswer::new(true);
    probe.admit(
        AdapterKind::Claude,
        &json!({"type": "system"}),
        std::slice::from_ref(&row),
    );
    assert!(!probe.answered(), "a non-answer event admits nothing");
    probe.admit(AdapterKind::Claude, &answer, std::slice::from_ref(&row));
    assert!(probe.answered(), "the first answer is admitted");
    probe.admit(AdapterKind::Claude, &answer, &[json!({"step": "second"})]);
    assert_eq!(
        probe.take(),
        Some(row),
        "a later answer never replaces the first"
    );

    assert!(
        !super::probe::is_answer(AdapterKind::Exec, &answer),
        "Exec has no model turn to answer a probe"
    );
}

/// A refusal always keeps a bounded, non-empty evidence line (decision
/// 0103 ruling 1(a); decision 0071 ruling 9): a provider that gave no token
/// at all is the explicit `api_error`, and the adapter fills its own name.
#[test]
fn a_refusal_with_no_token_keeps_a_default_and_each_adapter_names_itself() {
    let refusal = refusal_reason(" \n\t ", None, None);
    assert_eq!(refusal.words(), "api_error");
    assert_eq!(refusal.class(), RefusalClass::Other);
    for (kind, provider) in [
        (AdapterKind::Claude, "claude"),
        (AdapterKind::Lanetally, "lanetally"),
        (AdapterKind::Codex, "codex"),
        (AdapterKind::Dsh, "dsh"),
        (AdapterKind::Exec, "exec"),
    ] {
        assert_eq!(kind.provider_name(), provider, "{kind:?}");
    }
}
