use super::decision_0114a::{assert_refused_as, bindings, wire_rows, Driven, Stream};
use super::decision_0114b::{assert_bound_projection, assert_work, turns, words};
use super::*;
use crate::RefusalClass;

const LOCATOR: &str = "seat-1234";
const PROJECT: &str = "--w--";
const SESSION: &str = "root-1234";
const MODEL: &str = "echo-v2";
const EFFORT: &str = "xhigh";
const TOOL: &str = "mcp__srv__inspect";
const CALL: &str = "call-1";
const COUNTS: [(&str, u64); 4] = [
    ("input_tokens", 8),
    ("output_tokens", 4),
    ("cache_read_tokens", 3),
    ("reasoning_output_tokens", 2),
];

fn append_rows(home: &Path, rows: &[Value]) {
    use std::io::Write;

    let transcript = home
        .join(LOCATOR)
        .join(PROJECT)
        .join(SESSION)
        .join(DSH_TRANSCRIPT);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(transcript)
        .unwrap();
    for row in rows {
        writeln!(file, "{row}").unwrap();
    }
}

fn message(seq: u64, model: &str) -> Value {
    json!({"type":"assistant/message", "seq":seq, "data":{
        "message":{"source":{"model":model}, "content":"private"},
        "usage":{"inputTokens":5, "outputTokens":4, "cacheReadTokens":3, "reasoningTokens":2}
    }})
}

pub(in crate::adapters) fn dsh_call(call: &str, name: &str) -> Value {
    json!({"type": "tool/call", "seq": 7,
           "data": {"turn": 1, "step": 1, "callId": call, "name": name, "arguments": "{}"}})
}

#[test]
fn bound_dsh_transcript_keeps_current_usage_and_masks_the_model() {
    let streams = Stream::rows(&[], &[("STRUCTURE", "type"), ("MODEL", "private")]);
    let bound = bindings(&streams);
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(LOCATOR);
    plant_dsh_session(dir.path(), LOCATOR, PROJECT, SESSION, 2);
    append_rows(
        dir.path(),
        &[
            message(1, "old-model"),
            message(3, "valid-model"),
            message(4, "private-model"),
        ],
    );

    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted = Vec::new();
    drain_into(
        &root,
        &mut DshTail::default(),
        Some(2),
        &mut turns,
        &mut meta,
        &bound,
        &mut emitted,
    );
    assert_eq!(turns, 2);
    assert_eq!(emitted.len(), 2);
    assert_eq!(emitted[0]["model"], "valid-model");
    assert_eq!(emitted[1]["model"], MODEL_NOT_REPORTED);
    assert_eq!(meta["model"], "valid-model");
    for (key, count) in COUNTS {
        assert_eq!(emitted[0][key], count, "{key}");
        assert_eq!(emitted[1][key], count, "{key}");
        assert_eq!(meta[key], count * 2, "{key}");
    }
}

fn drive_dsh(
    home: &Path,
    rows: &[Value],
    secrets: &[(&str, &str)],
    effortless: bool,
) -> (Driven, Vec<secret::BoundSecret>) {
    let stream = Stream::rows_with_stderr(
        &[json!({"type":"system", "subtype":"init", "session_id":SESSION})],
        secrets,
        "dsh: reasoning:\ndsh: done",
    );
    let root = home.join(LOCATOR);
    plant_dsh_session(home, LOCATOR, PROJECT, SESSION, 0);
    append_rows(home, rows);
    let launch = DshLaunch {
        locator: LOCATOR.into(),
        effortless,
        ..dsh_stream_launch(&stream.shim, &root, None, None)
    };
    let driven = stream.drive_with(AdapterKind::Dsh, |prompt, _, bindings, mut emit| {
        invoke_dsh_launch(
            launch,
            prompt,
            home.to_str().unwrap(),
            bindings,
            &mut emit,
            wait_for_exit,
        )
    });
    (driven, bindings(&stream))
}

fn header(seq: u64, effort: &str, defaulted: bool) -> Value {
    json!({"type":"request/header", "seq":seq, "data":{"header":{
        "config":{"reasoningEffort":effort},
        "adapterDefaults":{"reasoningEffort":defaulted}
    }}})
}

fn steps(driven: &Driven) -> Vec<Step> {
    driven.emitted.iter().map(|(step, _)| *step).collect()
}

fn assert_plain_work(plain: &Driven) {
    assert_work(plain);
    assert_eq!(
        steps(plain),
        [
            Step::Transcript,
            Step::HarnessStarted,
            Step::SeatTurn,
            Step::SeatTurn,
            Step::SeatTurn,
            Step::SeatTurn,
            Step::SeatTurn,
            Step::SeatTurn
        ],
    );
    let transcript = &plain.emitted[0].1["transcript"];
    assert_eq!(transcript["locator"], LOCATOR);
    assert_eq!(plain.emitted[1].1["root_session"]["id"], SESSION);
    let meta = plain.meta.as_ref().unwrap();
    assert_eq!(meta["transcript"], *transcript);
    assert_eq!(meta["session_id"], SESSION);
    assert_eq!(meta["num_turns"], 5);
    assert_eq!(meta["model"], MODEL);
    assert_eq!(meta["effort"], EFFORT);

    let turns: Vec<_> = turns(plain).collect();
    assert_eq!(
        turns
            .iter()
            .map(|row| row["effort"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            EFFORT_NOT_APPLICABLE,
            "medium",
            EFFORT,
            EFFORT,
            EFFORT,
            EFFORT
        ]
    );
    assert!(turns.iter().all(|row| row["model"] == MODEL));
    for (index, turn) in [1, 2, 3, 3, 4, 5].into_iter().zip(&turns) {
        assert_eq!(turn["turn"], index);
    }
    let tool = turns[3];
    assert_eq!(tool["tool"], TOOL);
    assert_eq!(
        tool["observation"],
        json!({
            "format":"dsh", "call":CALL,
            "tool":{"kind":"mcp", "server":"srv", "tool":"inspect"}
        })
    );
    for (key, count) in COUNTS {
        assert_eq!(meta[key], count * 5, "{key}");
        for turn in [0, 1, 2, 4, 5] {
            assert_eq!(turns[turn][key], count, "{key}");
        }
    }
}

fn project_work(_: Step, row: &Value, word: &str) -> Value {
    let mut row = row.clone();
    if row.get("observation").is_some() {
        match word {
            "mcp__" | "mcp_" => {
                row["tool"] = json!(TOOL.replace(word, "[secret:S]"));
                row["observation"]["tool"] = json!({
                    "kind":"mcp_unidentified", "reported":row["tool"]
                });
            }
            "call" => row["observation"]["call"] = json!("[secret:S]-1"),
            _ => {}
        }
    }
    row
}

fn assert_bound_effort(home: &Path, rows: &[Value]) {
    let (driven, _) = drive_dsh(home, rows, &[("S", EFFORT)], true);
    assert_work(&driven);
    assert_eq!(
        turns(&driven)
            .map(|row| row["effort"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            EFFORT_NOT_APPLICABLE,
            "medium",
            EFFORT_NOT_REPORTED,
            EFFORT_NOT_REPORTED,
            EFFORT_NOT_REPORTED,
            "medium"
        ],
    );
    assert_eq!(driven.meta.unwrap()["effort"], "medium");

    let (driven, _) = drive_dsh(
        home,
        &[header(1, EFFORT, false), message(2, "private-v2")],
        &[("S", EFFORT), ("MODEL", "private")],
        true,
    );
    let turn = turns(&driven).next().unwrap();
    assert_eq!(turn["effort"], EFFORT_NOT_REPORTED);
    assert_eq!(turn["model"], MODEL_NOT_REPORTED);
    let meta = driven.meta.unwrap();
    assert_eq!(meta["effort"], EFFORT_NOT_APPLICABLE);
    assert_eq!(meta.get("model"), None);
}

fn assert_finished(driven: &Driven, bound: &[secret::BoundSecret], word: &str) {
    let wire: Vec<_> = wire_rows(&driven.bodies).collect();
    assert_eq!(wire.len(), driven.emitted.len() + 1, "{word}");
    let finished = wire.last().unwrap();
    assert_eq!(finished["step"], "deepseek-harness-session-finished");
    let guard = secret::Guard::new(bound);
    for (key, value) in finished.as_object().unwrap() {
        if key != "step" {
            assert!(guard.json(value.to_string().as_bytes()), "{word}: {key}");
        }
    }
}

#[test]
fn bound_word_dsh() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let mut call = dsh_call(CALL, TOOL);
    call["seq"] = json!(6);
    call["data"]["arguments"] = json!("private");
    let rows = [
        message(1, MODEL),
        header(2, "medium", false),
        message(3, MODEL),
        header(4, EFFORT, false),
        message(5, MODEL),
        call,
        message(7, MODEL),
        header(8, "higher", true),
        message(9, MODEL),
    ];
    let (plain, _) = drive_dsh(home, &rows, &[], true);
    assert_plain_work(&plain);
    // id and seq are shorter than MIN_VALUE_BYTES. Refusal-only terms
    // run through the refusal path below.
    let terms = [
        "type",
        "session",
        "request/header",
        "assistant/message",
        "tool/call",
        "usage",
        "inputTokens",
        "outputTokens",
        "cacheReadTokens",
        "reasoningTokens",
        "data",
        "model",
        "message",
        "source",
        "header",
        "config",
        "reasoningEffort",
        "adapterDefaults",
        "callId",
        "name",
        "mcp__",
        "system",
        "subtype",
        "init",
        "session_id",
        "delegationDepth",
        "dsh: reasoning:",
        "dsh: ",
        "private",
    ];
    assert_bound_projection(
        AdapterKind::Dsh,
        &terms,
        &plain,
        |word| {
            let (driven, bound) = drive_dsh(home, &rows, &[("S", word)], true);
            assert!(ProbeAnswer::new(true).answers(AdapterKind::Dsh, &Row::new(&rows[0], &bound)));
            assert_finished(&driven, &bound, word);
            (driven, bound)
        },
        project_work,
    );
    assert_bound_effort(home, &rows);
}

fn refusal(code: &str, status: Option<i64>) -> Value {
    let mut error = json!({"code":code, "message":"private refusal"});
    if let Some(status) = status {
        error["status"] = json!(status);
    }
    json!({"type":"turn/end", "seq":1, "data":{"reason":{"kind":"error", "error":error}}})
}

#[test]
fn bound_dsh_refusal_words_keep_their_class_and_do_not_accept() {
    use RefusalClass::{Other, RateLimit};

    let home = tempfile::tempdir().unwrap();
    let home = home.path();

    for (code, status, class, words_without_binding) in [
        ("QUOTA", None, RateLimit, "rate_limit: private refusal"),
        (
            "INTERNAL",
            Some(402),
            RateLimit,
            "rate_limit (HTTP 402): private refusal",
        ),
        (
            "INTERNAL",
            Some(429),
            RateLimit,
            "rate_limit (HTTP 429): private refusal",
        ),
        (
            "INTERNAL",
            Some(500),
            Other,
            "INTERNAL (HTTP 500): private refusal",
        ),
    ] {
        let rows = [refusal(code, status)];
        let (plain, _) = drive_dsh(home, &rows, &[], false);
        assert_refused_as(&plain.bodies, class, words_without_binding);
        assert_eq!(steps(&plain), [Step::Transcript, Step::HarnessStarted]);
        let terms = [
            "type", "turn/end", "data", "reason", "kind", "error", "code", "status", "message",
            code, "private",
        ];
        for word in words(&terms) {
            let (bound, bindings) = drive_dsh(home, &rows, &[("S", word)], false);
            let expected = words_without_binding.replace(word, "[secret:S]");
            assert_refused_as(&bound.bodies, class, &expected);
            assert_eq!(steps(&bound), steps(&plain), "{word}");
            assert!(
                secret::Guard::new(&bindings).text(expected.as_bytes()),
                "{word}"
            );
        }
    }
}
