//! A decoded value that contradicts the verdict resting on its turn is
//! never inert (#484): a turn is read only when it holds the reply and
//! states no refusal and no failure, whatever its exit, and a tool it
//! ran that its listing shows absent leaves the listing unmeasured. The
//! chief's rows on 07db5279, each run through the probe against a
//! Claude-like fake.

use serde_json::{json, Value};

use super::*;

const CLAUDE_NOMODEL: &str = include_str!("../measure/streams/claude-nomodel.stdout");

/// One row: the fake's shell on its plain and OFF turns and under the
/// hands argv, the fact the row is about, and what the probe must read.
struct Row {
    shape: &'static str,
    plain: String,
    boxed: String,
    fact: &'static str,
    expected: Value,
}

/// A shell setting `later`, an event printed after the init event, to
/// `events`, one per line, quoted for the shell.
fn later(events: &[&str]) -> String {
    format!("later='{}'", events.join("\n").replace('\'', r"'\''"))
}

/// The clean boxed turn, then `events`.
fn boxed(events: &[&str]) -> String {
    format!("{BOXED_CLEAN}; {}", later(events))
}

/// A plain shell listing Claude Code's two network tools beside its shell
/// on every turn, and printing `events` on the OFF turn alone.
fn off(events: &[&str]) -> String {
    format!(
        r#"{PLAIN_WEB}; case " $* " in *" --disallowedTools "*) {} ;; esac"#,
        later(events)
    )
}

const PLAIN_WEB: &str = r#"tools='"Bash","WebSearch","WebFetch"'"#;

/// The fake's last line, which a turn's excerpt quotes when stderr holds
/// none.
const LAST: &str = r#"{"type":"result","subtype":"success","is_error":false,"session_id":"5d0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e11","total_cost_usd":0.0125,"usage":{"input_tokens":10,"cache_read_input_tokens":4,"output_tokens":2}}"#;

/// The view of a row whose boxed turn reads as `tools` says, its server
/// listing as `server`, below the box, its OFF controls removing both
/// network tools.
fn unboxed(fact: Value, tools: &str, server: &str) -> Value {
    json!({
        "fact": fact,
        "eligibility": {
            "verdict": "unboxed-only",
            "reason": format!(
                "it is not shown to stand behind the box ({tools}; {server}), and the declared \
                 OFF controls removed WebSearch, WebFetch"
            ),
        },
    })
}

/// The view of a row whose boxed turn was unread as `why` says.
fn boxed_unread(why: &str) -> Value {
    unboxed(unmeasured(why), why, why)
}

/// The view of a row whose boxed turn the CLI refused, as `refused` says.
fn boxed_refused(refused: &str) -> Value {
    let refused = format!("the CLI refused the adapter's hands argv: exit 0: {refused}");
    unboxed(unsupported(&refused), &refused, &refused)
}

/// The view of a row whose `fact` is as given beside a boxed verdict.
fn boxed_beside(fact: Value) -> Value {
    json!({
        "fact": fact,
        "eligibility": {
            "verdict": "boxed",
            "reason": "its own tools switch off and the hands MCP server connects",
        },
    })
}

/// How the boxed turn fails when its line 2 says `what`.
fn boxed_failed(what: &str) -> String {
    format!("the boxed turn exited 0, but line 2 of stdout {what}: exit 0: {LAST}")
}

/// The chief's finding 1: a turn's own failure, at a clean exit.
fn failed_turns() -> Vec<Row> {
    let nomodel_result = CLAUDE_NOMODEL.lines().last().unwrap();
    let plain_failed =
        format!("the headless turn exited 0, but line 2 of stdout states a failure at /is_error: exit 0: {LAST}");
    vec![
        Row {
            shape: "f1a: an error result naming a hands flag",
            plain: PLAIN_WEB.to_string(),
            boxed: boxed(&[
                r#"{"type":"result","subtype":"success","is_error":true,"result":"error: unknown option --strict-mcp-config"}"#,
            ]),
            fact: "boxed_tools",
            expected: boxed_refused(LAST),
        },
        Row {
            shape: "f1c: the recorded unknown-model result",
            plain: PLAIN_WEB.to_string(),
            boxed: boxed(&[nomodel_result]),
            fact: "boxed_tools",
            expected: boxed_unread(&boxed_failed(
                "refuses the model brokkr-probe-no-such-model",
            )),
        },
        Row {
            shape: "f1e: an error result with no text and an overload status",
            plain: PLAIN_WEB.to_string(),
            boxed: boxed(&[
                r#"{"type":"result","subtype":"success","is_error":true,"api_error_status":529}"#,
            ]),
            fact: "boxed_tools",
            expected: boxed_unread(&boxed_failed("states a failure at /is_error")),
        },
        Row {
            shape: "h2: a hands flag refused on stderr at exit 0",
            plain: PLAIN_WEB.to_string(),
            boxed: format!(
                r#"{BOXED_CLEAN}; echo "error: unknown option '--strict-mcp-config'" >&2"#
            ),
            fact: "boxed_tools",
            expected: boxed_refused("error: unknown option '--strict-mcp-config'"),
        },
        Row {
            shape: "h3: a rejected rate limit, then no turn taken",
            plain: PLAIN_WEB.to_string(),
            boxed: boxed(&[
                r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1,"rateLimitType":"five_hour","utilization":1.0,"isUsingOverage":false}}"#,
                r#"{"type":"result","subtype":"success","is_error":true,"num_turns":0}"#,
            ]),
            fact: "boxed_tools",
            expected: boxed_unread(&boxed_failed("states a failure at /rate_limit_info/status")),
        },
        Row {
            shape: "h5: an error result on the plain turn",
            plain: format!(
                r#"{PLAIN_WEB}; case " $* " in *" --disallowedTools "*) ;; *) {} ;; esac"#,
                later(&[r#"{"type":"result","subtype":"success","is_error":true}"#])
            ),
            boxed: BOXED_CLEAN.to_string(),
            fact: "tools",
            expected: json!({
                "fact": unmeasured(&plain_failed),
                "eligibility": {
                    "verdict": "refused",
                    "reason": format!(
                        "its user-scope configuration is not shown to be isolated: {plain_failed}"
                    ),
                },
            }),
        },
    ]
}

/// The chief's finding 2: a count of native egress a turn ran.
fn tools_that_ran() -> Vec<Row> {
    let searched = r#"{"type":"result","subtype":"success","usage":{"server_tool_use":{"web_search_requests":1,"web_fetch_requests":0}}}"#;
    let servers = "the system/init event on line 1 of stdout listed mcp_servers: 1";
    let ran = |listed: &str, event: &str, what: &str, at: &str| {
        format!(
            "the turn's tools listed {listed}, but the {event} event on line 2 of stdout ran \
             {what} at {at}"
        )
    };
    let off_unread = |at: &str| {
        format!(
            "the turn under the declared OFF controls was not read: {}",
            ran("Bash", "result/success", "WebSearch", at)
        )
    };
    let counted = "/usage/server_tool_use/web_search_requests";
    let switch = |capability: &str, tool: &str| json!({"capability": capability, "off": unmeasured(&off_unread(counted)), "tools": [tool]});
    let in_box = ran(
        "mcp__brokkr__workspace",
        "result/success",
        "WebSearch",
        counted,
    );
    let stopped = ran("none", "assistant", "a tool", "/message/stop_reason");
    vec![
        Row {
            shape: "f2a: a search counted on the boxed turn",
            plain: PLAIN_WEB.to_string(),
            boxed: boxed(&[searched]),
            fact: "boxed_tools",
            expected: unboxed(unmeasured(&in_box), &in_box, servers),
        },
        Row {
            shape: "f2b: a search counted on the OFF turn",
            plain: off(&[searched]),
            boxed: BOXED_CLEAN.to_string(),
            fact: "capabilities",
            expected: boxed_beside(measured(
                json!([
                    switch("web-fetch", "WebFetch"),
                    switch("web-search", "WebSearch")
                ]),
                "the system/init event on line 1 of stdout listed tools: 3",
            )),
        },
        Row {
            shape: "f2c: a model's searches counted on the OFF turn",
            plain: off(&[
                r#"{"type":"result","subtype":"success","modelUsage":{"m":{"inputTokens":1,"outputTokens":1,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"webSearchRequests":3,"costUSD":0.1,"thinkingTokens":0}}}"#,
            ]),
            boxed: BOXED_CLEAN.to_string(),
            fact: "egress_off",
            expected: boxed_beside(unmeasured(&off_unread("/modelUsage/*/webSearchRequests"))),
        },
        Row {
            shape: "a tool use stopping a boxed turn that listed no tool",
            plain: PLAIN_WEB.to_string(),
            boxed: format!(
                r#"tools=''; servers='{{"name":"brokkr","status":"connected"}}'; {}"#,
                later(&[
                    r#"{"type":"assistant","message":{"content":[],"stop_reason":"tool_use"}}"#
                ])
            ),
            fact: "boxed_tools",
            expected: unboxed(unmeasured(&stopped), &stopped, servers),
        },
    ]
}

/// The chief's t0 and t1: the plain turn's transcript holds a bare `user`
/// row, as the fake writes on every launch, or the real one claude
/// writes, its message the probe's prompt; both read, and keep a clean
/// verdict.
fn transcript_rows() -> Vec<Row> {
    let user_row = CLAUDE_PLAIN_LOG.lines().nth(2).unwrap();
    let kept_out = || {
        boxed_beside(measured(
            json!(false),
            "the system/init event on line 1 of stdout listed mcp_servers: 0",
        ))
    };
    let prompt = "mcp__github__search of the MCP server github was listed by the system/init \
                  event on line 2 of stdout at /slash_commands/0";
    vec![
        Row {
            shape: "h1: an MCP server's prompt among the boxed turn's slash commands",
            plain: PLAIN_WEB.to_string(),
            boxed: boxed(&[
                r#"{"type":"system","subtype":"init","slash_commands":["compact","mcp__github__search"],"plugins":[{"name":"github","path":"p","source":"s"}]}"#,
            ]),
            fact: "user_mcp_boxed",
            expected: json!({
                "fact": measured(json!(true), prompt),
                "eligibility": {
                    "verdict": "refused",
                    "reason": format!(
                        "its user-scope configuration is not shown to be isolated: an MCP \
                         server other than brokkr reached the boxed turn: {prompt}"
                    ),
                },
            }),
        },
        Row {
            shape: "t0: a bare user row",
            plain: PLAIN_WEB.to_string(),
            boxed: BOXED_CLEAN.to_string(),
            fact: "user_mcp_unboxed",
            expected: kept_out(),
        },
        Row {
            shape: "t1: a recorded user row with its message",
            plain: format!(r#"{PLAIN_WEB}; printf '%s\n' '{user_row}' >> "$dir/$sid.jsonl""#),
            boxed: BOXED_CLEAN.to_string(),
            fact: "user_mcp_unboxed",
            expected: kept_out(),
        },
    ]
}

const CLAUDE_PLAIN_LOG: &str = include_str!("../measure/streams/claude-plain.jsonl");

/// A clean exit is not an answer (#484): a turn holding a refusal of any
/// class on any stream, or no reply on its stdout or a transcript, is
/// unread, its stderr's reply not being one, and says why.
#[test]
fn a_clean_exit_holding_a_refusal_or_no_reply_is_not_read() {
    let init = r#"{"type":"system","subtype":"init","tools":["Bash"],"mcp_servers":[]}"#;
    let why = |stdout: &str, stderr: &str| {
        let facts = facts_of(observation(Some(0), stdout, stderr));
        facts.tools.account().to_string()
    };
    let failed = |what: &str, ended: &str| {
        format!("the headless turn exited 0, but {what}: exit 0: {ended}")
    };
    let effort = "error: option '--effort <level>' argument 'max' is invalid. Allowed choices \
                  are low.";
    let rows = [
        (
            "Invalid API key · Please run /login",
            "line 1 of stderr refuses the credential API key",
        ),
        (
            "error: unknown option '--frobnicate'",
            "line 1 of stderr refuses the flag or key --frobnicate",
        ),
        (effort, "line 1 of stderr refuses the effort max"),
    ];
    for (stderr, what) in rows {
        assert_eq!(why(&replied(init), stderr), failed(what, stderr));
    }
    let no_reply = "no line of its stdout or a transcript is the reply PROBE-OK";
    assert_eq!(
        [why(init, ""), why(init, "PROBE-OK")],
        [failed(no_reply, init), failed(no_reply, "PROBE-OK")]
    );
}

/// dsh's recorded plain turn against the operator's local model, its
/// session log beside it, is read whole: its reply, the model it ran,
/// which the headless launch's evidence names, and the tools its request
/// offered, whose 25 names the skeleton holds as one placeholder.
#[test]
fn dsh_s_recorded_plain_turn_is_read_whole_and_names_its_model() {
    let log = Transcript {
        path: "~/.dsh/sessions/{workdir}/s/session.v3.jsonl".to_string(),
        written: Written::Created,
        text: Captured {
            text: include_str!("../measure/streams/dsh-plain.jsonl").to_string(),
            not_utf8: Vec::new(),
        },
    };
    let turn = Observation {
        transcripts: vec![log],
        ..observation(
            Some(0),
            include_str!("../measure/streams/dsh-plain.stdout"),
            include_str!("../measure/streams/dsh-plain.stderr"),
        )
    };
    let plan = plan::plan(AdapterKind::Dsh, &dsh_declared()).unwrap();
    let reading = measure::reading(&plan, &observed(turn), &["DSH_KEY"]);
    let headless = reading.facts.headless.account().to_string();
    assert_eq!(
        (
            reading.unread,
            headless.rsplit_once("; ").map(|(_, ran)| ran),
            reading.facts.tools.value()
        ),
        (
            Vec::new(),
            Some("it ran GLM-5.3-Flash-EXL3"),
            Some(&strings(&["<name>"]))
        )
    );
}

#[test]
fn a_decoded_failure_or_tool_run_never_rests_quietly_under_a_verdict() {
    if !in_its_own_engine(
        "probe::tests::answered::a_decoded_failure_or_tool_run_never_rests_quietly_under_a_verdict",
    ) {
        return;
    }
    let world = world();
    let rows: Vec<Row> = [failed_turns(), tools_that_ran(), transcript_rows()]
        .into_iter()
        .flatten()
        .collect();
    let mut seen = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let report = claude_report(&world, index, &row.plain, &row.boxed);
        let view = json!({
            "fact": report["facts"][row.fact],
            "eligibility": report["eligibility"],
        });
        seen.push((row.shape, view));
    }
    let expected: Vec<(&str, Value)> = rows
        .iter()
        .map(|row| (row.shape, row.expected.clone()))
        .collect();
    assert_eq!(seen, expected, "{:#}", json!(seen));
}
