//! A decoded value that contradicts the verdict resting on its turn is
//! never inert (#484): a turn is read only when it holds the reply and
//! states no refusal and no failure, whatever its exit, and a tool it
//! ran that its listing shows absent, or ran unnamed, leaves the listing
//! unmeasured. The
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

/// A hook's response, its `key` holding the reply, then an init event
/// listing `tools` and `servers`; the turn exits 0 there, unanswered,
/// unless `answers`.
fn hook_reply(key: &str, tools: &str, servers: &str, answers: bool) -> String {
    let hook = format!(
        r#"{{"type":"system","subtype":"hook_response","session_id":"{CLAUDE_SESSION}","hook_id":"11111111-2222-3333-4444-555555555555","hook_name":"SessionStart:startup","hook_event":"SessionStart","uuid":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","{key}":"PROBE-OK"}}"#
    );
    let shown = format!("tools='{tools}'; servers='{servers}'; printf '%s\\n' '{hook}'");
    if answers {
        shown
    } else {
        format!("{shown}; {}; exit 0", init_only(tools, servers))
    }
}

/// An init event listing `tools` and `servers`, printed alone.
fn init_only(tools: &str, servers: &str) -> String {
    format!(
        r#"printf '{{"type":"system","subtype":"init","session_id":"{CLAUDE_SESSION}","tools":[%s],"mcp_servers":[%s]}}\n' '{tools}' '{servers}'"#
    )
}

const BROKKR: &str = r#"{"name":"brokkr","status":"connected"}"#;

/// `shell` on the OFF turn alone, the plain turn listing `tools`.
fn on_off(tools: &str, shell: &str) -> String {
    format!(r#"tools='{tools}'; case " $* " in *" --disallowedTools "*) {shell} ;; esac"#)
}

/// `shell` on the plain turn alone.
fn on_plain(shell: &str) -> String {
    format!(r#"{PLAIN_WEB}; case " $* " in *" --disallowedTools "*) ;; *) {shell} ;; esac"#)
}

/// A boxed turn listing no tool beside the hands server, its result
/// carrying `extra`.
fn boxed_result(extra: &str) -> String {
    format!(
        r#"tools=''; servers='{BROKKR}'; {}"#,
        later(&[&format!(
            r#"{{"type":"result","subtype":"success","is_error":false,"result":"PROBE-OK"{extra}}}"#
        )])
    )
}

const SPAWNED: &str = r#","subagent_stats":{"spawned":@N@,"requested":{"background":0,"foreground":@N@,"unset":0},"started_in_background":0,"max_depth":1,"spawned_by_subagents":0,"completed":@N@,"failed":0,"killed":{"parent":0,"user":0,"system":0},"refused":{"depth_limit":0,"concurrency_limit":0,"budget":0},"by_type":{}}"#;

/// A row: `turns` the shells of its plain turn and of its boxed one.
fn row(shape: &'static str, turns: (String, String), fact: &'static str, expected: Value) -> Row {
    let (plain, boxed) = turns;
    Row {
        shape,
        plain,
        boxed,
        fact,
        expected,
    }
}

/// The turns of a row whose plain turn lists the network tools and whose
/// boxed turn runs `boxed`.
fn in_box(boxed: String) -> (String, String) {
    (PLAIN_WEB.to_string(), boxed)
}

/// The turns of a row whose plain turn runs `plain` and whose boxed turn
/// is the clean one.
fn out_of_box(plain: String) -> (String, String) {
    (plain, BOXED_CLEAN.to_string())
}

const BASH: &str = r#""Bash""#;

/// The chief's rows on 8316ad4d for finding 1, each beside its control:
/// only the turn's own answer is its reply, on each of the three turns.
fn reply_origins() -> Vec<Row> {
    let unanswered = |turn: &str, tools: &str, servers: &str| {
        format!(
            r#"{turn} exited 0, but no line of its stdout or a transcript is the reply PROBE-OK: exit 0: {{"type":"system","subtype":"init","session_id":"{CLAUDE_SESSION}","tools":[{tools}],"mcp_servers":[{servers}]}}"#
        )
    };
    let in_the_box = unanswered("the boxed turn", "", BROKKR);
    let headless = unanswered("the headless turn", BASH, "");
    let off_turn = unanswered("the turn under the declared OFF controls", BASH, "");
    let (bash, web) = (BASH, r#""Bash","WebSearch","WebFetch""#);
    vec![
        row(
            "f1-hook-reply: a hook's stdout is the reply on a boxed turn that never answered",
            in_box(hook_reply("stdout", "", BROKKR, false)),
            "boxed_tools",
            boxed_unread(&in_the_box),
        ),
        row(
            "f1-no-hook: its control, the hook removed",
            in_box(format!("{}; exit 0", init_only("", BROKKR))),
            "boxed_tools",
            boxed_unread(&in_the_box),
        ),
        row(
            "m3: a hook's output is the reply on a plain turn that never answered",
            out_of_box(on_plain(&hook_reply("output", bash, "", false))),
            "tools",
            json!({
                "fact": unmeasured(&headless),
                "eligibility": {
                    "verdict": "refused",
                    "reason": format!(
                        "its user-scope configuration is not shown to be isolated: {headless}"
                    ),
                },
            }),
        ),
        row(
            "m3 control: the same hook, and the turn answers",
            out_of_box(on_plain(&hook_reply("output", bash, "", true))),
            "tools",
            boxed_beside(measured(
                json!(["Bash"]),
                "the system/init event on line 2 of stdout listed tools: 1",
            )),
        ),
        row(
            "m4: a hook's stderr is the reply on the OFF turn that never answered",
            out_of_box(on_off(web, &hook_reply("stderr", bash, "", false))),
            "user_mcp_off",
            json!({
                "fact": unmeasured(&off_turn),
                "eligibility": {
                    "verdict": "boxed-only",
                    "reason": format!(
                        "its own tools switch off and the hands MCP server connects, but its \
                         turn under the declared OFF controls, the launch a seat granted nothing \
                         uses, is not shown to keep another MCP server out (#467): {off_turn}, \
                         so it may hold boxed offices only"
                    ),
                },
            }),
        ),
        row(
            "m4 control: the same hook, and the turn answers",
            out_of_box(on_off(web, &hook_reply("stderr", bash, "", true))),
            "user_mcp_off",
            boxed_beside(measured(
                json!(false),
                "the system/init event on line 2 of stdout listed mcp_servers: 0",
            )),
        ),
    ]
}

/// The chief's rows on 8316ad4d for findings 2 and 6, each beside its
/// control: an OFF turn not read leaves egress_off so, and a subagent
/// spawned or more than one turn taken is a tool run.
fn early_returns() -> Vec<Row> {
    let ran = |at: &str| {
        let ran = format!(
            "the turn's tools listed none, but the result/success event on line 2 of stdout ran \
             a tool at {at}"
        );
        let servers = "the system/init event on line 1 of stdout listed mcp_servers: 1";
        unboxed(unmeasured(&ran), &ran, servers)
    };
    let searched = r#"later='{"type":"result","subtype":"success","usage":{"server_tool_use":{"web_search_requests":1,"web_fetch_requests":0}}}'"#;
    let off_unread = "the turn under the declared OFF controls was not read: the turn's tools \
                      listed Bash, but the result/success event on line 2 of stdout ran \
                      WebSearch at /usage/server_tool_use/web_search_requests";
    let none_spawned = format!(r#"{},"num_turns":1"#, SPAWNED.replace("@N@", "0"));
    let bash = BASH;
    vec![
        row(
            "f2-off-search-plain-empty: a search counted on the OFF turn, the plain turn listing \
             no egress",
            out_of_box(on_off(bash, searched)),
            "egress_off",
            boxed_beside(unmeasured(off_unread)),
        ),
        row(
            "f2-control-plain-empty: its control, no search counted",
            out_of_box(on_off(bash, ":")),
            "egress_off",
            boxed_beside(measured(
                json!(true),
                "the plain turn listed no native egress tool",
            )),
        ),
        row(
            "m1: a subagent spawned on a boxed turn that listed no tool",
            in_box(boxed_result(&SPAWNED.replace("@N@", "1"))),
            "boxed_tools",
            ran("/subagent_stats/spawned"),
        ),
        row(
            "m2: three turns taken on a boxed turn that listed no tool",
            in_box(boxed_result(r#","num_turns":3"#)),
            "boxed_tools",
            ran("/num_turns"),
        ),
        row(
            "m1 and m2 control: none spawned, one turn taken",
            in_box(boxed_result(&none_spawned)),
            "boxed_tools",
            boxed_beside(measured(
                json!([]),
                "the system/init event on line 1 of stdout listed tools: 0",
            )),
        ),
    ]
}

/// The turns of a row whose boxed turn runs `boxed`, then writes claude's
/// recorded cost-state row, `from` replaced by `to`, as a transcript.
fn cost_written(boxed: &str, from: &str, to: &str) -> (String, String) {
    let recorded = CLAUDE_PLAIN_LOG.lines().nth(23).unwrap();
    assert!(recorded.contains(from), "{recorded}");
    let row = recorded.replace(from, to);
    let written = format!(r#"printf '%s\n' '{row}' > "$dir/cost.jsonl""#);
    in_box(format!("{boxed}; {written}"))
}

/// The chief's H3 on dcf4344f beside its controls: the boxed turn that
/// lists no tool writes claude's recorded cost-state row as a transcript,
/// and each total only a tool run makes positive, raised, is a tool run,
/// as a web search the row counts is; the row as recorded keeps the turn
/// boxed.
fn worked_in_tools() -> Vec<Row> {
    let cost = |from: &str, to: &str| cost_written(&boxed_result(""), from, to);
    let ran = |what: &str, at: &str| {
        let ran = format!(
            "the turn's tools listed none, but the cost-state event on line 1 of \
             ~/.claude/projects/{{workdir}}/cost.jsonl ran {what} at {at}"
        );
        let servers = "the system/init event on line 1 of stdout listed mcp_servers: 1";
        unboxed(unmeasured(&ran), &ran, servers)
    };
    let raised = |shape, key: &str| {
        let at = format!("/{key}");
        row(
            shape,
            cost(&format!(r#""{key}":0,"#), &format!(r#""{key}":3,"#)),
            "boxed_tools",
            ran("a tool", &at),
        )
    };
    vec![
        row(
            "h3 control: the recorded row",
            cost("", ""),
            "boxed_tools",
            boxed_beside(measured(
                json!([]),
                "the system/init event on line 1 of stdout listed tools: 0",
            )),
        ),
        row(
            "h3 comparator: a web search the row counts",
            cost(r#""webSearchRequests":0"#, r#""webSearchRequests":1"#),
            "boxed_tools",
            ran("WebSearch", "/modelUsage/*/webSearchRequests"),
        ),
        raised("h3a: time spent in tools", "totalToolDuration"),
        raised("h3b: lines added", "totalLinesAdded"),
        raised("h3c: lines removed", "totalLinesRemoved"),
    ]
}

/// A result event carrying `extra`.
fn result_with(extra: &str) -> String {
    format!(r#"{{"type":"result","subtype":"success"{extra}}}"#)
}

/// What a turn listing `listed` reads when the `event` event on its line
/// 2 ran a tool unnamed at `at`.
fn ran_unnamed(listed: &str, event: &str, at: &str) -> String {
    format!(
        "the turn's tools listed {listed}, but the {event} event on line 2 of stdout ran a tool \
         at {at}"
    )
}

/// The chief's H5 on 0fa0ed56 beside its controls: a tool run that names
/// no tool on the OFF turn, which lists Bash, leaves that listing
/// unmeasured, so neither off switch is measured and the harness, whose
/// CLI refuses the hands argv, is refused where its control is held to
/// unboxed offices.
fn unnamed_runs_off() -> Vec<Row> {
    let stop = |reason: &str| {
        format!(r#"{{"type":"assistant","message":{{"content":[],"stop_reason":"{reason}"}}}}"#)
    };
    let refused = "the CLI refused the adapter's hands argv: exit 1: error: unknown option \
                   '--strict-mcp-config'";
    let control = unboxed(
        measured(
            json!(true),
            "the declared OFF controls removed WebSearch, WebFetch",
        ),
        refused,
        refused,
    );
    let unread = |event: &str, at: &str| {
        let why = format!(
            "the turn under the declared OFF controls was not read: {}",
            ran_unnamed("Bash", event, at)
        );
        json!({
            "fact": unmeasured(&why),
            "eligibility": {
                "verdict": "refused",
                "reason": format!(
                    "the evidence for a seat in a realm that grants its capabilities is not \
                     complete: web-fetch's off switch is unmeasured: {why}; web-search's off \
                     switch is unmeasured: {why}"
                ),
            },
        })
    };
    let off_row = |shape, event: String, expected| {
        let turns = (off(&[&event]), Boxed::Refuses.shell().to_string());
        row(shape, turns, "egress_off", expected)
    };
    let spawned = |n: &str| result_with(&SPAWNED.replace("@N@", n));
    let result = "result/success";
    vec![
        off_row(
            "h5-off-num-turns-2",
            result_with(r#","num_turns":2"#),
            unread(result, "/num_turns"),
        ),
        off_row(
            "h5-off-num-turns-1 control",
            result_with(r#","num_turns":1"#),
            control.clone(),
        ),
        off_row(
            "h5-off-stop-tool-use",
            stop("tool_use"),
            unread("assistant", "/message/stop_reason"),
        ),
        off_row(
            "h5-off-stop-end-turn control",
            stop("end_turn"),
            control.clone(),
        ),
        off_row(
            "h5-off-spawned-1",
            spawned("1"),
            unread(result, "/subagent_stats/spawned"),
        ),
        off_row("h5-off-spawned-0 control", spawned("0"), control),
    ]
}

/// The chief's H5 widened on 0fa0ed56, beside its controls: a tool run
/// that names no tool on the boxed turn, which lists only the hands tool,
/// or on the plain turn, which lists the network tools, or a transcript's
/// total that only a tool run makes positive beside the hands tool, leaves
/// that listing unmeasured, and the box or the evidence with it.
fn unnamed_runs_listed() -> Vec<Row> {
    let (two, one) = (
        result_with(r#","num_turns":2"#),
        result_with(r#","num_turns":1"#),
    );
    let servers = "the system/init event on line 1 of stdout listed mcp_servers: 1";
    let in_the_box = ran_unnamed("mcp__brokkr__workspace", "result/success", "/num_turns");
    let plain = ran_unnamed("Bash, WebSearch, WebFetch", "result/success", "/num_turns");
    let boxed_control = || {
        boxed_beside(measured(
            json!([]),
            "the system/init event on line 1 of stdout listed tools: 1",
        ))
    };
    let totalled = "the turn's tools listed mcp__brokkr__workspace, but the cost-state event on \
                    line 1 of ~/.claude/projects/{workdir}/cost.jsonl ran a tool at \
                    /totalToolDuration";
    let duration = r#""totalToolDuration":"#;
    vec![
        row(
            "w-boxed-hands-num-turns-2",
            in_box(boxed(&[&two])),
            "boxed_tools",
            unboxed(unmeasured(&in_the_box), &in_the_box, servers),
        ),
        row(
            "w-boxed-hands control",
            in_box(boxed(&[&one])),
            "boxed_tools",
            boxed_control(),
        ),
        row(
            "w-plain-num-turns-2",
            out_of_box(on_plain(&later(&[&two]))),
            "tools",
            json!({
                "fact": unmeasured(&plain),
                "eligibility": {
                    "verdict": "refused",
                    "reason": format!(
                        "the evidence for boxed offices is not complete: tools is unmeasured: \
                         {plain}"
                    ),
                },
            }),
        ),
        row(
            "w-plain control",
            out_of_box(on_plain(&later(&[&one]))),
            "tools",
            boxed_beside(measured(
                json!(["Bash", "WebSearch", "WebFetch"]),
                "the system/init event on line 1 of stdout listed tools: 3",
            )),
        ),
        row(
            "h5-transcript-tool-duration beside the hands tool",
            cost_written(
                BOXED_CLEAN,
                &format!("{duration}0,"),
                &format!("{duration}3,"),
            ),
            "boxed_tools",
            unboxed(unmeasured(totalled), totalled, servers),
        ),
        row(
            "h5-transcript control: the recorded row beside the hands tool",
            cost_written(BOXED_CLEAN, "", ""),
            "boxed_tools",
            boxed_control(),
        ),
    ]
}

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

/// The chief's f3-stderr-only-exit0 on 8316ad4d, beside its control
/// f3-stderr-with-init-exit0: a boxed turn that exits 0 printing only the
/// CLI's refusal of a hands flag, no event and no transcript, is the CLI
/// refusing the hands argv, as it is beside an event.
#[test]
fn a_hands_flag_refused_by_a_turn_printing_no_event_is_unsupported() {
    let init = r#"{"type":"system","subtype":"init","tools":["Bash"],"mcp_servers":[]}"#;
    let refused = "error: unknown option '--strict-mcp-config'";
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    let boxed_tools = |stdout: &str| {
        let observed = Observed {
            boxed: Trial::Observed(observation(Some(0), stdout, refused)),
            ..observed(observation(Some(0), &replied(init), ""))
        };
        measure::reading(&plan, &observed, &[]).facts.boxed_tools
    };
    let unsupported = Fact::Unsupported {
        evidence: format!("the CLI refused the adapter's hands argv: exit 0: {refused}"),
    };
    assert_eq!(
        [boxed_tools(""), boxed_tools(init)],
        [unsupported.clone(), unsupported]
    );
}

/// The chief's codex-erritem on 8316ad4d, beside its control
/// codex-agentmsg: an error item holding the reply leaves the plain turn
/// unread, where an agent message holding it is the turn's answer.
#[test]
fn a_codex_error_item_holding_the_reply_is_not_the_turn_s_answer() {
    if !in_its_own_engine(
        "probe::tests::answered::a_codex_error_item_holding_the_reply_is_not_the_turn_s_answer",
    ) {
        return;
    }
    let world = world();
    let erritem = CODEX_LIKE.replace(
        r#""type":"agent_message","text":"PROBE-OK""#,
        r#""type":"error","message":"PROBE-OK""#,
    );
    let events = |name: &str, body: &str| {
        let cli = world.fake(name, body);
        probe(AdapterKind::Codex, &cli, &codex_declared(), &world)["facts"]["events"].clone()
    };
    let completed = r#"{"type":"turn.completed","usage":{"input_tokens":20,"cached_input_tokens":5,"output_tokens":3,"reasoning_output_tokens":1}}"#;
    assert_eq!(
        [
            events("codex-erritem", &erritem),
            events("codex-agentmsg", CODEX_LIKE)
        ],
        [
            unmeasured(&format!(
                "the headless turn exited 0, but no line of its stdout or a transcript is the \
                 reply PROBE-OK: exit 0: {completed}"
            )),
            measured(
                json!({
                    "source": "stdout",
                    "format": "ndjson",
                    "non_json_lines": 0,
                    "types": ["thread.started", "turn.started", "item.completed", "turn.completed"],
                }),
                "4 events read from stdout",
            ),
        ]
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
    let rows: Vec<Row> = [
        failed_turns(),
        tools_that_ran(),
        transcript_rows(),
        reply_origins(),
        early_returns(),
        worked_in_tools(),
        unnamed_runs_off(),
        unnamed_runs_listed(),
    ]
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
