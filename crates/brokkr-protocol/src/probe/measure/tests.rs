//! The typed readers against the streams the controller recorded from
//! the real CLIs on 2026-10-03 (`streams/`, sanitised: every MCP server,
//! tool, plugin, skill, command and agent name is a numbered
//! placeholder), and every variant of a known event one key or one value
//! type off it (#484).

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use super::forms::Origin;
use super::read::{Class, Counted, Harness, Refused, Said, Server};
use super::tools::mcp_server;
use super::{parse_lines, Captured, Event, Fault, Lines, Reader, Stream, Streams};
use crate::probe::facts::Fact;
use crate::probe::plan::NO_SUCH_MODEL;

const CLAUDE_PLAIN: &str = include_str!("streams/claude-plain.stdout");
const CLAUDE_NOMODEL: &str = include_str!("streams/claude-nomodel.stdout");
const CLAUDE_NOMODEL_ERR: &str = include_str!("streams/claude-nomodel.stderr");
const CODEX_PLAIN: &str = include_str!("streams/codex-plain.stdout");
const CODEX_PLAIN_ERR: &str = include_str!("streams/codex-plain.stderr");
const CODEX_NOMODEL: &str = include_str!("streams/codex-nomodel.stdout");
const DSH_NOMODEL_ERR: &str = include_str!("streams/dsh-nomodel.stderr");
const DSH_PLAIN: &str = include_str!("streams/dsh-plain.stdout");
const DSH_PLAIN_ERR: &str = include_str!("streams/dsh-plain.stderr");
const DSH_STREAM_ERR: &str = include_str!("streams/dsh-stream.stderr");

/// The transcripts the recorded launches left, as skeletons (2026-10-04).
const CLAUDE_PLAIN_LOG: &str = include_str!("streams/claude-plain.jsonl");
const CLAUDE_NOMODEL_LOG: &str = include_str!("streams/claude-nomodel.jsonl");
const CODEX_PLAIN_LOG: &str = include_str!("streams/codex-plain.jsonl");
const CODEX_OFF_LOG: &str = include_str!("streams/codex-off.jsonl");
const CODEX_NOMODEL_LOG: &str = include_str!("streams/codex-nomodel.jsonl");
const DSH_PLAIN_LOG: &str = include_str!("streams/dsh-plain.jsonl");

/// Each event a stream held, by its type, with what it said.
type Events = Vec<(String, Vec<Said>)>;

/// What `text` reads to, as `harness`'s `lines`: each event's type and
/// what it said, and each line left unread and why.
fn read(harness: Harness, lines: Lines, text: &str) -> (Events, Vec<(usize, Fault)>) {
    let captured = Captured {
        text: text.to_string(),
        not_utf8: Vec::new(),
    };
    let stream = parse_lines(&captured, "stdout", 1, Reader { harness, lines });
    let events = stream
        .events
        .into_iter()
        .map(|event| (event.label, event.said));
    (events.collect(), stream.unread)
}

/// What each line of `text` says, read as `harness`'s text.
fn text_said(harness: Harness, text: &str) -> Vec<Option<Vec<Said>>> {
    text.lines().map(|line| harness.text(line)).collect()
}

fn refusal(class: Class, object: &str) -> Said {
    Said::Refusal(Refused {
        class,
        object: object.to_string(),
    })
}

/// The `init` event of a recorded claude stream, as the oracle a reading
/// is held to: its tools, and its MCP servers by name and status.
fn claude_init(stream: &str) -> (Vec<String>, Vec<Server>) {
    let init: Value = stream
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|event| event["subtype"] == "init")
        .unwrap();
    let tools = init["tools"].as_array().unwrap().iter();
    let servers = init["mcp_servers"].as_array().unwrap().iter();
    let statuses = ["connected", "failed", "needs-auth", "pending"];
    (
        tools
            .map(|tool| tool.as_str().unwrap().to_string())
            .collect(),
        servers
            .map(|server| Server {
                name: server["name"].as_str().unwrap().to_string(),
                status: statuses
                    .into_iter()
                    .find(|status| server["status"] == *status)
                    .unwrap(),
            })
            .collect(),
    )
}

#[test]
fn claude_s_recorded_plain_turn_decodes_whole_and_yields_its_tools_servers_and_reply() {
    let (events, unread) = read(Harness::Claude, Lines::Events, CLAUDE_PLAIN);
    assert_eq!(unread, []);
    let session = || Said::Session {
        key: "session_id",
        id: "4d68b632-8be3-4e0d-9b5d-46dc3c5464ce".to_string(),
    };
    let (tools, servers) = claude_init(CLAUDE_PLAIN);
    assert_eq!((tools.len(), servers.len()), (144, 24));
    let assistant_usage = Counted::of(
        "/message/usage",
        Some("msg_011CffxTjvpVstmKsAkKfcLk".to_string()),
        [
            ("input_tokens", Some(2)),
            ("cache_creation_input_tokens", Some(18466)),
            ("cache_read_input_tokens", Some(10341)),
            ("output_tokens", Some(8)),
        ],
    );
    let result_usage = Counted::of(
        "/usage",
        None,
        [
            ("input_tokens", Some(2)),
            ("cache_creation_input_tokens", Some(18466)),
            ("cache_read_input_tokens", Some(10341)),
            ("output_tokens", Some(9)),
        ],
    );
    let cost = Said::Cost {
        at: "/total_cost_usd",
    };
    assert_eq!(
        events,
        vec![
            ("system/hook_started".to_string(), vec![session()]),
            ("system/hook_response".to_string(), vec![session()]),
            (
                "system/init".to_string(),
                vec![
                    session(),
                    Said::Model("claude-opus-5-5[1m]".to_string()),
                    Said::Tools {
                        at: "/tools",
                        names: tools,
                    },
                    Said::Servers {
                        at: "/mcp_servers",
                        servers,
                    },
                ]
            ),
            (
                "assistant".to_string(),
                vec![
                    session(),
                    Said::Reply,
                    Said::Model("claude-opus-5-5".to_string()),
                    assistant_usage
                ]
            ),
            ("rate_limit_event".to_string(), vec![session()]),
            (
                "result/success".to_string(),
                vec![session(), Said::Reply, result_usage, cost]
            ),
        ]
    );
}

#[test]
fn claude_s_recorded_refusal_of_an_unknown_model_is_a_configuration_refusal_naming_it() {
    let (events, unread) = read(Harness::Claude, Lines::Events, CLAUDE_NOMODEL);
    assert_eq!(unread, []);
    let refusals: Vec<&Said> = events
        .iter()
        .flat_map(|(_, said)| said)
        .filter(|said| matches!(said, Said::Refusal(_)))
        .collect();
    let config = refusal(Class::Config, NO_SUCH_MODEL);
    assert_eq!(refusals, [&config, &config]);
    assert_eq!(
        text_said(Harness::Claude, CLAUDE_NOMODEL_ERR),
        [Some(vec![config])]
    );
}

#[test]
fn codex_s_recorded_turns_decode_whole_and_yield_the_reply_usage_and_refusal() {
    let (events, unread) = read(Harness::Codex, Lines::Events, CODEX_PLAIN);
    assert_eq!(unread, []);
    let usage = Counted::of(
        "/usage",
        None,
        [
            ("input_tokens", Some(14177)),
            ("cached_input_tokens", Some(12288)),
            ("cache_write_input_tokens", Some(0)),
            ("output_tokens", Some(8)),
            ("reasoning_output_tokens", Some(0)),
        ],
    );
    let thread = Said::Session {
        key: "thread_id",
        id: "01a10383-b8fc-7683-a19b-6c41efeae3a0".to_string(),
    };
    assert_eq!(
        events,
        vec![
            ("thread.started".to_string(), vec![thread]),
            ("turn.started".to_string(), Vec::new()),
            ("item.completed".to_string(), vec![Said::Reply]),
            ("turn.completed".to_string(), vec![usage]),
        ]
    );
    assert_eq!(
        text_said(Harness::Codex, CODEX_PLAIN_ERR),
        [Some(Vec::new())]
    );
    let (events, unread) = read(Harness::Codex, Lines::Events, CODEX_NOMODEL);
    assert_eq!(unread, []);
    let config = refusal(Class::Config, NO_SUCH_MODEL);
    let labels: Vec<(&str, &[Said])> = events
        .iter()
        .map(|(label, said)| (label.as_str(), said.as_slice()))
        .collect();
    assert_eq!(
        labels[1..],
        [
            ("item.completed", &[][..]),
            ("turn.started", &[][..]),
            (
                "error",
                &[config.clone(), Said::Failed { at: "/message" }][..]
            ),
            (
                "turn.failed",
                &[
                    config,
                    Said::Failed {
                        at: "/error/message"
                    }
                ][..]
            ),
        ]
    );
}

/// A string a reader consumes is read whole by its forms, so one that is
/// none of them leaves its event unrecognised, whichever key holds it;
/// and an event otherwise decoded says only what it holds (#484).
#[test]
fn a_string_no_form_reads_leaves_its_event_unread_wherever_a_reader_consumes_it() {
    let sid = "5d0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e11";
    let hook = format!(
        r#"{{"type":"system","subtype":"hook_response","hook_id":"{sid}","hook_name":"h","hook_event":"e","uuid":"{sid}","session_id":"{sid}","output":"MCP server github connected"}}"#
    );
    // The provider's error, as codex prints it inside a message.
    let api = |error: &str| {
        let error: Value = serde_json::from_str(error).unwrap();
        let inner = json!({"type": "error", "status": 400, "error": error});
        json!({"type": "error", "message": inner.to_string()}).to_string()
    };
    let rows = [
        (Harness::Claude, hook),
        (
            Harness::Claude,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"github"}]}}"#
                .to_string(),
        ),
        (
            Harness::Claude,
            r#"{"type":"result","subtype":"success","result":"WebSearch enabled"}"#.to_string(),
        ),
        (
            Harness::Codex,
            r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"github"}}"#
                .to_string(),
        ),
        (Harness::Codex, r#"{"type":"error","message":"github"}"#.to_string()),
        (
            Harness::Codex,
            api(r#"{"type":"invalid_request_error","message":"github"}"#),
        ),
    ];
    let unread = rows.map(|(harness, line)| read(harness, Lines::Events, &line).1);
    assert_eq!(unread, [(); 6].map(|_| vec![(1, Fault::Unrecognised)]));
    let api_unknown_key = api(r#"{"type":"t","message":"m","code":"x"}"#);
    assert_eq!(
        read(Harness::Codex, Lines::Events, &api_unknown_key).1,
        [(1, Fault::Undecoded(Harness::Codex))]
    );
    let bare = [
        r#"{"type":"assistant","message":{"content":[]}}"#,
        r#"{"type":"result","subtype":"success"}"#,
    ];
    assert_eq!(
        bare.map(|line| read(Harness::Claude, Lines::Events, line)),
        [
            (vec![("assistant".to_string(), Vec::new())], Vec::new()),
            (vec![("result/success".to_string(), Vec::new())], Vec::new()),
        ]
    );
}

/// Only the turn's own answer is its reply (#484, the chief's finding 1
/// on 8316ad4d): claude's assistant text and result, and codex's agent
/// message, are; a hook's output, stdout or stderr, and codex's error
/// item, error event and failed turn, each holding the reply alone, read
/// and say no reply, a failure still stating its failure.
#[test]
fn only_the_turn_s_own_answer_is_its_reply() {
    let sid = "5d0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e11";
    let hook = |key: &str| {
        format!(
            r#"{{"type":"system","subtype":"hook_response","hook_id":"{sid}","hook_name":"h","hook_event":"e","uuid":"{sid}","session_id":"{sid}","{key}":"PROBE-OK"}}"#
        )
    };
    let item = |kind: &str, key: &str| {
        format!(
            r#"{{"type":"item.completed","item":{{"id":"item_0","type":"{kind}","{key}":"PROBE-OK"}}}}"#
        )
    };
    let rows = [
        (Harness::Claude, hook("output")),
        (Harness::Claude, hook("stdout")),
        (Harness::Claude, hook("stderr")),
        (
            Harness::Claude,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"PROBE-OK"}]}}"#
                .to_string(),
        ),
        (
            Harness::Claude,
            r#"{"type":"result","subtype":"success","result":"PROBE-OK"}"#.to_string(),
        ),
        (Harness::Codex, item("agent_message", "text")),
        (Harness::Codex, item("error", "message")),
        (
            Harness::Codex,
            r#"{"type":"error","message":"PROBE-OK"}"#.to_string(),
        ),
        (
            Harness::Codex,
            r#"{"type":"turn.failed","error":{"message":"PROBE-OK"}}"#.to_string(),
        ),
    ];
    let session = Said::Session {
        key: "session_id",
        id: sid.to_string(),
    };
    let said = rows.map(|(harness, line)| read(harness, Lines::Events, &line));
    let one = |label: &str, said: Vec<Said>| (vec![(label.to_string(), said)], Vec::new());
    assert_eq!(
        said,
        [
            one("system/hook_response", vec![session.clone()]),
            one("system/hook_response", vec![session.clone()]),
            one("system/hook_response", vec![session]),
            one("assistant", vec![Said::Reply]),
            one("result/success", vec![Said::Reply]),
            one("item.completed", vec![Said::Reply]),
            one("item.completed", Vec::new()),
            one("error", vec![Said::Failed { at: "/message" }]),
            one(
                "turn.failed",
                vec![Said::Failed {
                    at: "/error/message"
                }]
            ),
        ]
    );
}

/// A session is a canonical UUID, and a line that names one in any other
/// shape is not one of claude's events; each harness names itself in the
/// line it does not decode.
#[test]
fn a_session_that_is_not_a_uuid_is_undecoded_and_each_harness_names_itself() {
    let init = |id: &str| format!(r#"{{"type":"system","subtype":"init","session_id":"{id}"}}"#);
    let ids = [
        "5d0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e11",
        "5d0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e1",
        "zd0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e11",
    ];
    let unread = ids.map(|id| read(Harness::Claude, Lines::Events, &init(id)).1);
    let undecoded = vec![(1, Fault::Undecoded(Harness::Claude))];
    assert_eq!(unread, [Vec::new(), undecoded.clone(), undecoded.clone()]);
    // A key a struct requires is not optional, and one it may lack is.
    let sid = ids[0];
    let lines = [
        (
            Harness::Claude,
            format!(r#"{{"type":"system","subtype":"hook_started","hook_name":"h","hook_event":"e","uuid":"{sid}","session_id":"{sid}"}}"#),
        ),
        (
            Harness::Claude,
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1,"rateLimitType":"five_hour","utilization":0.5,"isUsingOverage":false}}"#
                .to_string(),
        ),
        (Harness::Dsh, r#"{"type":"session","version":3}"#.to_string()),
        (
            Harness::Dsh,
            r#"{"type":"assistant/message","data":{"message":{"source":{"model":"m"}}}}"#.to_string(),
        ),
    ];
    assert_eq!(
        lines.map(|(harness, line)| read(harness, Lines::Events, &line).1),
        [
            undecoded,
            Vec::new(),
            vec![(1, Fault::Undecoded(Harness::Dsh))],
            Vec::new()
        ]
    );
    let named = [Harness::Claude, Harness::Codex, Harness::Dsh]
        .map(|harness| Fault::Undecoded(harness).to_string());
    let reads = "event the probe decodes whole: its type, a key or a value's type is not one the \
                 reader names";
    assert_eq!(
        named,
        ["claude", "codex", "dsh"].map(|harness| format!("is not a {harness} {reads}"))
    );
}

#[test]
fn dsh_s_recorded_refusal_of_model_is_a_refused_control_not_a_model_refusal() {
    assert_eq!(
        text_said(Harness::Dsh, DSH_NOMODEL_ERR),
        [Some(vec![refusal(Class::Control, "--model")])]
    );
}

/// What one reading says, in a few words: a listing by its pointer and
/// length, a server listing with its first status.
fn summary(said: &Said) -> String {
    match said {
        Said::Tools { at, names } => format!("tools {at}: {}", names.len()),
        Said::Servers { at, servers } => {
            format!("servers {at}: {} {}", servers.len(), servers[0].status)
        }
        Said::Session { key, .. } => format!("session {key}"),
        Said::Usage(counted) => format!("usage {}", counted.at),
        Said::Cost { at } => format!("cost {at}"),
        Said::Reply => "reply".to_string(),
        Said::Refusal(refused) => format!("refusal {}", refused.object),
        Said::Levels(levels) => format!("levels {}", levels.refused),
        Said::Failed { at } => format!("failed {at}"),
        Said::Ran { at, .. } => format!("ran {at}"),
        Said::Model(model) => format!("model {model}"),
    }
}

/// What a recorded transcript of `harness` yields, each line read whole.
fn yields(harness: Harness, log: &str) -> Vec<String> {
    let (events, unread) = read(harness, Lines::Transcript, log);
    assert_eq!(unread, [], "{log}");
    let said = events.iter().flat_map(|(_, said)| said);
    said.map(summary).collect()
}

/// Every transcript the recorded launches left decodes whole, as its
/// harness's rows, and yields what it shows: the reply, the model, the
/// tools and MCP servers it lists, and a failed turn's failure; a real
/// claude `user` row with its message, and a codex `session_meta` with
/// its fields, among them (#484).
#[test]
fn every_recorded_transcript_decodes_whole_and_yields_its_reply_model_and_listings() {
    let listed = |pending: usize, servers: usize| {
        [
            "tools /attachment/surfacedNames: 3".to_string(),
            "tools /attachment/surfacedDefinitions: 3".to_string(),
            format!("servers /attachment/pendingMcpServers: {pending} pending"),
            "servers /attachment/needsAuthMcpServers: 8 needs-auth".to_string(),
            "servers /attachment/failedMcpServers: 3 failed".to_string(),
            format!("servers /attachment/addedNames: {servers} connected"),
        ]
    };
    let claude_plain = [
        vec!["model claude-opus-5-5[1m]".to_string()],
        vec!["tools /attachment/addedNames: 133".to_string()],
        listed(6, 3).to_vec(),
        ["reply", "model claude-opus-5-5", "usage /message/usage"]
            .map(String::from)
            .to_vec(),
        vec!["tools /attachment/tools: 11".to_string()],
    ];
    assert_eq!(
        yields(Harness::Claude, CLAUDE_PLAIN_LOG),
        claude_plain.concat()
    );
    let failed =
        ["/error", "/isApiErrorMessage", "/apiErrorStatus"].map(|at| format!("failed {at}"));
    let claude_nomodel = [
        vec!["model <modelId>".to_string()],
        vec!["tools /attachment/addedNames: 198".to_string()],
        listed(2, 5).to_vec(),
        ["model <model>", "usage /message/usage"]
            .map(String::from)
            .to_vec(),
        failed.to_vec(),
    ];
    assert_eq!(
        yields(Harness::Claude, CLAUDE_NOMODEL_LOG),
        claude_nomodel.concat()
    );
    let codex = ["model gpt-6.1-sol", "reply", "reply", "reply"];
    assert_eq!(
        [CODEX_PLAIN_LOG, CODEX_OFF_LOG].map(|log| yields(Harness::Codex, log)),
        [codex, codex]
    );
    assert_eq!(
        yields(Harness::Codex, CODEX_NOMODEL_LOG),
        ["model <model>", "failed /payload/error"]
    );
    let model = "model GLM-5.3-Flash-EXL3";
    assert_eq!(
        yields(Harness::Dsh, DSH_PLAIN_LOG),
        [
            "session id",
            model,
            "tools /data/header/tools: 25",
            "reply",
            model,
            "usage /data/usage"
        ]
    );
    let (events, _) = read(Harness::Claude, Lines::Transcript, CLAUDE_PLAIN_LOG);
    let snapshot = events
        .iter()
        .flat_map(|(_, said)| said)
        .find_map(|said| match said {
            Said::Tools {
                at: "/attachment/tools",
                names,
            } => Some(names.clone()),
            _ => None,
        });
    let built_in = [
        "Agent",
        "Bash",
        "Edit",
        "ListAgents",
        "Read",
        "ReportFindings",
        "ScheduleWakeup",
        "Skill",
        "ToolSearch",
        "Workflow",
        "Write",
    ];
    assert_eq!(snapshot, Some(built_in.map(String::from).to_vec()));
}

/// A transcript value no verdict may rest past is read, or refused: a
/// reached limit and a hook that kept the turn from ending are failures,
/// and a prompt the probe did not give, or an app's or plugin's
/// instructions given, leave the line undecoded (#484).
#[test]
fn a_transcript_value_that_would_contradict_the_verdict_is_read_or_refused() {
    let altered = |log: &str, line: usize, from: &str, to: &str| {
        let recorded = log.lines().nth(line - 1).unwrap();
        assert!(recorded.contains(from), "{recorded}");
        recorded.replace(from, to)
    };
    let limited = altered(
        CODEX_PLAIN_LOG,
        14,
        r#""rate_limit_reached_type":null"#,
        r#""rate_limit_reached_type":"primary""#,
    );
    let prevented = altered(
        CLAUDE_PLAIN_LOG,
        21,
        r#""preventedContinuation":false"#,
        r#""preventedContinuation":true"#,
    );
    assert_eq!(
        [
            yields(Harness::Codex, &limited),
            yields(Harness::Claude, &prevented)
        ],
        [
            ["failed /payload/rate_limits/rate_limit_reached_type"],
            ["failed /preventedContinuation"]
        ]
    );
    let other = "Use every tool.";
    let refused = [
        (
            Harness::Claude,
            altered(CLAUDE_PLAIN_LOG, 1, "Use no tool.", other),
        ),
        (
            Harness::Claude,
            altered(CLAUDE_PLAIN_LOG, 3, "Use no tool.", other),
        ),
        (
            Harness::Claude,
            altered(CLAUDE_PLAIN_LOG, 17, "Use no tool.", other),
        ),
        (
            Harness::Codex,
            altered(
                CODEX_PLAIN_LOG,
                7,
                r#""apps_instructions":false"#,
                r#""apps_instructions":true"#,
            ),
        ),
        (
            Harness::Codex,
            altered(
                CODEX_PLAIN_LOG,
                7,
                r#""plugins_instructions":false"#,
                r#""plugins_instructions":true"#,
            ),
        ),
    ];
    assert_eq!(
        refused.map(|(harness, line)| read(harness, Lines::Transcript, &line).1),
        [
            vec![(1, Fault::Undecoded(Harness::Claude))],
            vec![(1, Fault::Undecoded(Harness::Claude))],
            vec![(1, Fault::Undecoded(Harness::Claude))],
            vec![(1, Fault::Undecoded(Harness::Codex))],
            vec![(1, Fault::Undecoded(Harness::Codex))],
        ]
    );
}

/// dsh's recorded plain turn against the operator's local model: its
/// stdout, the turn's answer, is the reply alone, which on stderr is not
/// one (#484); and its stderr, the one line of reasoning after each
/// `dsh: reasoning:` header, is read whole and says nothing. A refusal
/// inside the reasoning is still read, the reply there is not one, and
/// the line after the reasoning's one is read like any other.
#[test]
fn dsh_s_recorded_plain_turn_reads_its_reply_and_its_reasoning_whole() {
    let read = |origin, text: &str| {
        let captured = Captured {
            text: text.to_string(),
            not_utf8: Vec::new(),
        };
        let reader = Reader {
            harness: Harness::Dsh,
            lines: Lines::Text(origin),
        };
        let stream = parse_lines(&captured, "stderr", 1, reader);
        (stream.text, stream.unread)
    };
    let text = |text: &str| read(Origin::Aside, text);
    assert_eq!(
        read(Origin::Answer, DSH_PLAIN),
        (vec![(1, Said::Reply)], Vec::new())
    );
    assert_eq!(text(DSH_PLAIN), (Vec::new(), Vec::new()));
    assert_eq!(text(DSH_PLAIN_ERR), (Vec::new(), Vec::new()));
    // dsh 0.1.5-rc.1's headless profile refuses a stream format.
    assert_eq!(
        text(DSH_STREAM_ERR),
        (
            vec![(1, refusal(Class::Control, "--output-format"))],
            Vec::new()
        )
    );
    let reasoning = "dsh: reasoning:\nI think.\nPROBE-OK\nerror: unknown option '--model'";
    assert_eq!(
        text(reasoning),
        (vec![(4, refusal(Class::Control, "--model"))], Vec::new())
    );
    assert_eq!(
        read(Origin::Answer, "dsh: reasoning:\nPROBE-OK"),
        (Vec::new(), Vec::new())
    );
    // The chief's dsh-reason-a on 8316ad4d: a warning after the
    // reasoning's one line is unread, as it is without the header.
    let loaded = "warning: loaded MCP server github from /etc/dsh/config.yml";
    assert_eq!(
        [
            text(&format!("dsh: reasoning:\nI think.\n{loaded}")),
            text(loaded)
        ],
        [
            (Vec::new(), vec![(3, Fault::Unrecognised)]),
            (Vec::new(), vec![(1, Fault::Unrecognised)])
        ]
    );
    assert_eq!(
        text("I think.\ndsh: reasoning:"),
        (Vec::new(), vec![(1, Fault::Unrecognised)])
    );
}

/// One value of each JSON type, the key's value in an added key.
fn of_each_type() -> [Value; 6] {
    [
        json!("WebSearch"),
        json!(true),
        json!(14),
        json!({"github": {"connected": true}}),
        json!(["mcp__github__search"]),
        Value::Null,
    ]
}

/// A value of each JSON type other than `value`'s; null among them unless
/// the recordings show `key` null, as some event of theirs does.
fn other_types(value: &Value, key: &str, nullable: &BTreeSet<String>) -> Vec<Value> {
    let kind = |value: &Value| std::mem::discriminant(value);
    let each = of_each_type()
        .into_iter()
        .filter(|other| !other.is_null() || !nullable.contains(key));
    each.filter(|other| kind(other) != kind(value)).collect()
}

/// Every variant of `event` one key or one value type off it: at each
/// object it holds, an added key of each JSON type; and at each key and
/// list entry whose value is not null, a value of each other type.
fn variants(event: &Value, nullable: &BTreeSet<String>) -> Vec<Value> {
    let mut found = Vec::new();
    nodes(event, &mut Vec::new(), &mut found);
    let mut variants = Vec::new();
    for (path, value) in found {
        if let Value::Object(_) = value {
            for added in of_each_type() {
                let mut variant = event.clone();
                let object = at(&mut variant, &path).as_object_mut().unwrap();
                object.insert("brokkr_probe_unknown_key".to_string(), added);
                variants.push(variant);
            }
        }
        let Some(key) = path.last().filter(|_| !value.is_null()) else {
            continue;
        };
        for other in other_types(&value, key, nullable) {
            let mut variant = event.clone();
            *at(&mut variant, &path) = other;
            variants.push(variant);
        }
    }
    variants
}

/// Every key some recorded event of `streams` holds `null` under.
fn recorded_null(streams: &[&str]) -> BTreeSet<String> {
    let mut found = Vec::new();
    for line in streams.iter().flat_map(|stream| stream.lines()) {
        nodes(
            &serde_json::from_str(line).unwrap(),
            &mut Vec::new(),
            &mut found,
        );
    }
    let null = found.into_iter().filter(|(_, value)| value.is_null());
    null.filter_map(|(path, _)| path.last().cloned()).collect()
}

/// Each value inside `value`, `value` among them, by its path.
fn nodes(value: &Value, path: &mut Vec<String>, found: &mut Vec<(Vec<String>, Value)>) {
    found.push((path.clone(), value.clone()));
    let children: Vec<(String, &Value)> = match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| (key.clone(), value))
            .collect(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item))
            .collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => Vec::new(),
    };
    for (step, child) in children {
        path.push(step);
        nodes(child, path, found);
        path.pop();
    }
}

fn at<'a>(value: &'a mut Value, path: &[String]) -> &'a mut Value {
    path.iter().fold(value, |value, step| match value {
        Value::Array(items) => &mut items[step.parse::<usize>().unwrap()],
        other => &mut other[step.as_str()],
    })
}

/// Every variant of every event of every recorded stream refuses: an
/// added key of any JSON type at any depth, and a known key holding a
/// value of another type, null where no recording shows it null, leave
/// the line unread (#484).
#[test]
fn every_event_one_key_or_one_value_type_off_a_recorded_one_is_unread() {
    let claude = [CLAUDE_PLAIN, CLAUDE_NOMODEL];
    let codex = [CODEX_PLAIN, CODEX_NOMODEL];
    let recorded = [
        (Harness::Claude, &claude, recorded_null(&claude)),
        (Harness::Codex, &codex, recorded_null(&codex)),
    ];
    assert_eq!(
        recorded.each_ref().map(|(_, _, nullable)| nullable.len()),
        [14, 0]
    );
    let mut tried = 0;
    let mut admitted = Vec::new();
    for (harness, streams, nullable) in &recorded {
        let harness = *harness;
        for line in streams.iter().flat_map(|stream| stream.lines()) {
            let event: Value = serde_json::from_str(line).unwrap();
            let fields = |value: Value| -> Map<String, Value> {
                let Value::Object(fields) = value else {
                    unreachable!("a recorded event is an object")
                };
                fields
            };
            assert!(harness.event(fields(event.clone())).is_ok(), "{line}");
            for variant in variants(&event, nullable) {
                tried += 1;
                if harness.event(fields(variant.clone())).is_ok() {
                    admitted.push(variant.to_string());
                }
            }
        }
    }
    assert_eq!((tried, admitted), (7712, Vec::<String>::new()));
}

/// The hands server's status when the turn's every line was read, a
/// listing named servers, and none of them the hands server: the exact
/// `not listed` spelling the report carries (the review rounds' carried
/// L7; ruling 9's tests that bind).
#[test]
fn mcp_server_listing_whole_without_the_hands_server_reads_not_listed() {
    let stream = Stream {
        source: "stdout".to_string(),
        lines: Lines::Events,
        events: vec![Event {
            line: 1,
            label: "system/init".to_string(),
            said: vec![Said::Servers {
                at: "/mcp_servers",
                servers: vec![Server {
                    name: "github".to_string(),
                    status: "connected",
                }],
            }],
        }],
        text: Vec::new(),
        unread: Vec::new(),
        empty: false,
    };
    let streams = Streams {
        all: vec![stream],
        primary: 0,
    };
    assert_eq!(
        mcp_server(&streams),
        Fact::Measured {
            value: "not listed".to_string(),
            evidence: "the system/init event on line 1 of stdout listed mcp_servers: 1".to_string(),
        }
    );
}
