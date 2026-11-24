use super::decision_0114a::{assert_refused, bindings, wire_rows, Driven, Stream};
use super::decision_0114b::{assert_bound_projection, assert_work, words};
use super::*;

const THREAD: &str = "local-1234";
const MODEL: &str = "actual-v2";
const PIN: &str = "gpt-5.6-sol";

fn turn(model: &str) -> Value {
    json!({"type":"turn.completed","model":model,"usage":{
        "input_tokens":11,"cached_input_tokens":7,"output_tokens":5,
        "cache_write_input_tokens":3,"reasoning_output_tokens":2
    }})
}

fn completed(driven: &Driven) -> &Value {
    &driven
        .emitted
        .iter()
        .find(|(step, _)| *step == Step::TurnCompleted)
        .unwrap()
        .1
}

fn check_model_fallback(sessions: &Path) {
    let rollout = sessions.join(format!("rollout-{THREAD}.jsonl"));
    std::fs::write(&rollout, turn_context("from-rollout", "private-high")).unwrap();
    let rows = [
        json!({"type":"thread.started","thread_id":THREAD}),
        json!({"type":"turn.started"}),
        turn("private-v2"),
        json!({"type":"result","effort":"private-low"}),
    ];
    let pin = ["--model".to_string(), PIN.to_string()];
    for (header, expected, turn_model) in [
        (PIN, Some("from-rollout"), "from-rollout"),
        (PIN, Some(PIN), MODEL_NOT_REPORTED),
        ("gpt-5.6-mini", None, MODEL_NOT_REPORTED),
    ] {
        let stream =
            Stream::rows_with_stderr(&rows, &[("S", "private")], &format!("model: {header}"));
        let driven = stream.drive(AdapterKind::Codex, &pin);
        assert_work(&driven);
        assert_eq!(completed(&driven)["model"], turn_model);
        assert_eq!(completed(&driven)["effort"], EFFORT_NOT_REPORTED);
        let finished = wire_rows(&driven.bodies).last().unwrap();
        assert_eq!(finished["model"], expected.unwrap_or(MODEL_NOT_REPORTED));
        assert_eq!(finished["effort"], EFFORT_NOT_REPORTED);
        let meta = driven.meta.unwrap();
        assert_eq!(meta.get("model").and_then(Value::as_str), expected);
        assert_eq!(meta.get("effort"), None);
        if expected == Some("from-rollout") {
            std::fs::remove_file(&rollout).unwrap();
        }
    }
}

fn assert_plain(plain: &Driven) {
    assert_work(plain);
    assert_eq!(
        plain
            .emitted
            .iter()
            .map(|(step, _)| *step)
            .collect::<Vec<_>>(),
        [
            Step::Transcript,
            Step::HarnessStarted,
            Step::TurnStarted,
            Step::ItemStarted,
            Step::ItemCompleted,
            Step::TurnCompleted,
        ],
    );
    let transcript = &plain.emitted[0].1["transcript"];
    let meta = plain.meta.as_ref().unwrap();
    assert_eq!(transcript["locator"], THREAD);
    assert_eq!(meta["transcript"], *transcript);
    let call = &plain.emitted[3].1;
    assert_eq!(call["tool"], "mcp_tool_call");
    assert_eq!(
        call["observation"],
        json!({"format":"codex","call":"call-1",
            "tool":{"kind":"mcp","server":"sandbox","tool":"check"}})
    );
    let completed = completed(plain);
    assert_eq!(completed["turn"], 1);
    assert_eq!(completed["model"], MODEL);
    for (key, value) in [
        ("input_tokens", 11),
        ("cache_read_tokens", 7),
        ("output_tokens", 5),
        ("cache_write_tokens", 3),
        ("reasoning_output_tokens", 2),
    ] {
        assert_eq!(completed[key], value);
        assert_eq!(meta[key], value);
    }
    assert_eq!(meta["num_turns"], 1);
    assert_eq!(meta["model"], MODEL);
    assert_eq!(meta["effort"], "medium");
    assert_eq!(meta["total_cost_usd"], 0.25);
}

fn mask_item_projection(step: Step, row: &Value, word: &str) -> Value {
    let mut projected = row.clone();
    if matches!(step, Step::ItemStarted | Step::ItemCompleted) {
        projected["tool"] = json!(row["tool"].as_str().unwrap().replace(word, "[secret:S]"));
        if step == Step::ItemCompleted {
            projected["observation"]["tool"]["name"] = projected["tool"].clone();
        }
    }
    projected
}

fn check_stream_refusals() {
    let terms = [
        "type",
        "error",
        "turn.failed",
        "status",
        "message",
        "codex_error_info",
        "usage_limit_exceeded",
        "rate_limit_exceeded",
        "rate_limit",
        "quota",
    ];
    for (row, prefix) in [
        (
            json!({"type":"error","status":429,"message":"quota exhausted"}),
            "rate_limit (HTTP 429): ",
        ),
        (
            json!({"type":"turn.failed","error":{"codex_error_info":"usage_limit_exceeded",
            "message":"quota exhausted"}}),
            "rate_limit: ",
        ),
        (
            json!({"type":"error","codex_error_info":"rate_limit_exceeded",
            "message":"quota exhausted"}),
            "rate_limit: ",
        ),
    ] {
        let rows = [row];
        let plain = Stream::rows(&rows, &[]).drive(AdapterKind::Codex, &[]);
        assert_refused(&plain.bodies, &format!("{prefix}quota exhausted"));
        for word in words(&terms) {
            let bound = Stream::rows(&rows, &[("S", word)]).drive(AdapterKind::Codex, &[]);
            let excerpt = "quota exhausted".replace(word, "[secret:S]");
            assert_refused(
                &bound.bodies,
                &format!("{prefix}{excerpt}").replace(word, "[secret:S]"),
            );
            assert_eq!(
                bound
                    .emitted
                    .iter()
                    .map(|(step, _)| *step)
                    .collect::<Vec<_>>(),
                [Step::Transcript, Step::HarnessStarted],
            );
        }
    }
}

#[test]
fn bound_word_codex() {
    let mut env = EnvGuard::lock();
    let home = tempfile::tempdir().unwrap();
    env.set("CODEX_HOME", home.path());
    let sessions = home.path().join("sessions");
    std::fs::create_dir(&sessions).unwrap();
    let rollout = sessions.join(format!("rollout-{THREAD}.jsonl"));
    std::fs::write(&rollout, turn_context("from-rollout", "medium")).unwrap();
    let rows = [
        json!({"type":"thread.started","thread_id":THREAD}),
        json!({"type":"turn.started"}),
        json!({"type":"item.started","item":{"type":"mcp_tool_call",
            "id":"call-1","server":"sandbox","tool":"check","arguments":"private"}}),
        json!({"type":"item.completed","item":{"type":"agent_message","text":"private"}}),
        turn(MODEL),
        json!({"type":"result","num_turns":1,"total_cost_usd":0.25,
            "turn_context":{"effort":"medium"},"result":"private"}),
    ];
    let plain = Stream::rows(&rows, &[]).drive(AdapterKind::Codex, &[]);
    assert_plain(&plain);
    let terms = [
        "type",
        "item",
        "item.started",
        "item.completed",
        "agent_message",
        "thread.started",
        "thread_id",
        "turn.started",
        "turn.completed",
        "usage",
        "input_tokens",
        "cached_input_tokens",
        "output_tokens",
        "cache_write_input_tokens",
        "reasoning_output_tokens",
        "mcp_tool_call",
        "server",
        "tool",
        "result",
        "model",
        "message",
        "data",
        "source",
        "session_id",
        "effort",
        "turn_context",
        "num_turns",
        "total_cost_usd",
        "payload",
        "thread_settings",
        "reasoning_effort",
        "private",
    ];
    assert_bound_projection(
        AdapterKind::Codex,
        &terms,
        &plain,
        |word| {
            let stream = Stream::rows(&rows, &[("S", word)]);
            let bindings = bindings(&stream);
            assert!(
                ProbeAnswer::new(true).answers(AdapterKind::Codex, &Row::new(&rows[3], &bindings))
            );
            (stream.drive(AdapterKind::Codex, &[]), bindings)
        },
        mask_item_projection,
    );
    check_model_fallback(&sessions);
    check_stream_refusals();
}

/// Codex's `exec --json` stream carries usage on `turn.completed` and no
/// model at all (verified against codex-cli 0.153.0), so the served
/// model has to come off the thread record or a codex seat reaches every
/// cost and audit surface saying `not reported` — dsh's sentinel, on a
/// harness that does echo what served it (decision 0031 ruling 1).
#[test]
fn a_codex_turn_takes_its_model_from_the_thread_record_when_the_stream_omits_it() {
    let home = tempfile::tempdir().unwrap();
    let dated = home.path().join("sessions/2026/09/04");
    std::fs::create_dir_all(&dated).unwrap();
    std::fs::write(
        dated.join("rollout-01a06c05-c0a6-7991-a277-652d17ace6a6.jsonl"),
        thread_settings("gpt-5.6-sol", "medium") + &turn_context("gpt-5.6-sol", "medium"),
    )
    .unwrap();

    let mut echo = CodexThreadEcho::default();
    echo.locate_unbound(home.path(), "01a06c05-c0a6-7991-a277-652d17ace6a6");
    let mut turn = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    let mut transcript = Transcript::resolve(TranscriptKind::CodexThread).unwrap();
    fold_codex_into(
        &Row::new(
            &json!({"type": "turn.completed", "usage": {"input_tokens": 10}}),
            &[],
        ),
        &mut turn,
        &mut meta,
        &mut transcript,
        &mut echo,
        &mut emitted,
    );
    assert_eq!(meta["model"], "gpt-5.6-sol");
    assert_eq!(meta["effort"], "medium");
    let checkpoint = emitted.last().unwrap();
    assert_eq!(checkpoint["model"], "gpt-5.6-sol");
    assert_eq!(checkpoint["effort"], "medium");

    // A stream that DOES name a model outranks the record: it is the
    // more direct report of the turn that just ran.
    fold_codex_into(
        &Row::new(
            &json!({"type": "turn.completed", "model": "gpt-5.6-sol-preview", "usage": {}}),
            &[],
        ),
        &mut turn,
        &mut meta,
        &mut transcript,
        &mut echo,
        &mut emitted,
    );
    assert_eq!(emitted.last().unwrap()["model"], "gpt-5.6-sol-preview");
}
