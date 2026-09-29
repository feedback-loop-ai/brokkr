//! Every stream shape a review found the listings misread, and those the
//! listing reader's invariant implies, each run through the probe against
//! a Claude-like fake and held to its exact facts and verdict (#484).

use serde_json::{json, Value};

use super::*;

const INIT: &str = "the system/init event on line 1 of stdout";
const LATER: &str = "the system/init event on line 2 of stdout";
const TRANSCRIPT: &str = "the system/init event on line 1 of ~/.claude/projects/{workdir}";
const PLANTED_TOOL: &str =
    "mcp__brokkr-probe-user-scope__probe of the MCP server brokkr-probe-user-scope";
const PLANTED_SERVER: &str = "the MCP server brokkr-probe-user-scope was listed connected by";

/// A plain turn that lists no MCP server at all.
const PLAIN_CLEAN: &str = ":";

/// One stream shape: the fake's shell on its plain turn and under the
/// hands argv, and what the probe must read of them.
struct Row {
    shape: &'static str,
    plain: &'static str,
    boxed: String,
    expected: Value,
}

fn listed(at: &str, key: &str, count: usize) -> String {
    format!("{at} listed {key}: {count}")
}

fn unread(key: &str, why: &str) -> String {
    format!("the turn's {key} could not be read whole: {why}")
}

fn no_reach_but(unread: &str) -> String {
    format!("no MCP server other than brokkr was read reaching the turn, but {unread}")
}

/// The clean boxed turn's shell, then `more`.
fn boxed(more: &str) -> String {
    format!("{BOXED_CLEAN}; {more}")
}

/// A boxed turn printing `event` after its init event.
fn later(event: &str) -> String {
    boxed(&format!("later='{event}'"))
}

/// A boxed turn writing `event` to the transcript `file` beside the
/// session's.
fn written(file: &str, event: &str) -> String {
    format!("printf '%s\\n' '{event}' > \"$dir/{file}\"")
}

/// The facts a row shows, the verdict among them.
fn view(report: &Value) -> Value {
    let facts = &report["facts"];
    json!({
        "boxed_tools": facts["boxed_tools"],
        "mcp_server": facts["mcp_server"],
        "user_mcp_unboxed": facts["user_mcp_unboxed"],
        "user_mcp_boxed": facts["user_mcp_boxed"],
        "config_isolation": facts["config_isolation"],
        "eligibility": report["eligibility"],
    })
}

/// A row's view: its two listings, and the MCP servers each turn was read
/// to reach, the isolation and verdict they imply being `rest`.
fn expect(boxed_tools: Value, mcp_server: Value, unboxed: Value, rest: Value) -> Value {
    let mut view = json!({
        "boxed_tools": boxed_tools,
        "mcp_server": mcp_server,
        "user_mcp_unboxed": unboxed,
    });
    view.as_object_mut()
        .unwrap()
        .extend(rest.as_object().unwrap().clone());
    view
}

fn tools_emptied(evidence: &str) -> Value {
    measured(json!([]), evidence)
}

fn connected(evidence: &str) -> Value {
    measured(json!("connected"), evidence)
}

/// The rest of a view whose boxed turn another MCP server was read
/// reaching at `reach`: refused.
fn reached_in_the_box(reach: &str) -> Value {
    let leaked = format!("{OTHER_SERVER} reached the boxed turn: {reach}");
    json!({
        "user_mcp_boxed": measured(json!(true), reach),
        "config_isolation": measured(json!(false), &leaked),
        "eligibility": {"verdict": "refused", "reason": format!("{NOT_ISOLATED}{leaked}")},
    })
}

/// The rest of a view whose boxed turn was not read whole, as `why`
/// says, and whose plain turn loaded the planted server: refused.
fn unread_in_the_box(why: &str) -> Value {
    let leaked = format!(
        "{OTHER_SERVER} reached the plain turn, and no boxed turn showed it kept out: {why}"
    );
    json!({
        "user_mcp_boxed": unmeasured(why),
        "config_isolation": measured(json!(false), &leaked),
        "eligibility": {"verdict": "refused", "reason": format!("{NOT_ISOLATED}{leaked}")},
    })
}

/// The rest of a view whose boxed turn was read whole, as `servers`
/// evidences, and reached by no other MCP server, with `eligibility`.
fn kept_out_of_the_box(servers: &str, eligibility: Value) -> Value {
    json!({
        "user_mcp_boxed": measured(json!(false), servers),
        "config_isolation": measured(
            json!(true),
            &format!("{NO_OTHER_SERVER} reached the boxed turn: {servers}"),
        ),
        "eligibility": eligibility,
    })
}

/// The plain turn's reading of the default fake, which loads the planted
/// server.
fn leaked_plain() -> Value {
    measured(json!(true), PLAIN_REACH)
}

/// The planted server's tool, read in the boxed init event's tool listing.
fn tool_at_init() -> String {
    format!("{PLANTED_TOOL} was listed by {INIT} at /tools/1")
}

/// The evidence of a boxed turn that listed its MCP servers twice.
fn two_server_listings() -> String {
    format!(
        "{}, and {}",
        listed(INIT, "mcp_servers", 1),
        listed(LATER, "mcp_servers", 1)
    )
}

/// A boxed turn whose later `event` lists the planted server's tool at
/// `pointer`, after a clean init event: refused.
fn planted_tool_later(shape: &'static str, event: &str, pointer: &str) -> Row {
    Row {
        shape,
        plain: PLAIN_READS_THE_PLANT,
        boxed: later(event),
        expected: expect(
            tools_emptied(&format!(
                "{}, and {}",
                listed(INIT, "tools", 1),
                listed(LATER, "tools", 1)
            )),
            connected(&listed(INIT, "mcp_servers", 1)),
            leaked_plain(),
            reached_in_the_box(&format!(
                "{PLANTED_TOOL} was listed by {LATER} at {pointer}"
            )),
        ),
    }
}

/// The chief's three shapes on 93b3793f.
fn found_by_the_chief() -> Vec<Row> {
    let no_name = format!("{LATER} holds an entry at /tools/0 the probe cannot name");
    let object = unread(
        "tools",
        &format!(
            r#"{LATER} holds at /tools a value that is not a list: {{"mcp__brokkr-probe-user-scope__probe":{{}}}}"#
        ),
    );
    vec![
        Row {
            shape: "a reach beside a sibling listing the probe cannot name",
            plain: PLAIN_READS_THE_PLANT,
            boxed: r#"tools='"mcp__brokkr__workspace","mcp__brokkr-probe-user-scope__probe"'; servers='{"name":"brokkr","status":"connected"}'; later='{"type":"system","subtype":"init","tools":[{"id":1}]}'"#.to_string(),
            expected: expect(
                unmeasured(&unread("tools", &no_name)),
                connected(&listed(INIT, "mcp_servers", 1)),
                leaked_plain(),
                reached_in_the_box(&tool_at_init()),
            ),
        },
        Row {
            shape: "an object where a tool listing belongs",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(
                r#"{"type":"system","subtype":"init","tools":{"mcp__brokkr-probe-user-scope__probe":{}}}"#,
            ),
            expected: expect(
                unmeasured(&object),
                connected(&listed(INIT, "mcp_servers", 1)),
                leaked_plain(),
                unread_in_the_box(&no_reach_but(&object)),
            ),
        },
        Row {
            shape: "a reach only in the boxed turn's transcript",
            plain: PLAIN_READS_THE_PLANT,
            boxed: boxed(&written(
                "boxed.jsonl",
                r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr-probe-user-scope","status":"connected"}]}"#,
            )),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                connected(&format!(
                    "{}, and {}",
                    listed(INIT, "mcp_servers", 1),
                    listed(&format!("{TRANSCRIPT}/boxed.jsonl"), "mcp_servers", 1)
                )),
                leaked_plain(),
                reached_in_the_box(&format!(
                    "{PLANTED_SERVER} {TRANSCRIPT}/boxed.jsonl at /mcp_servers/0"
                )),
            ),
        },
    ]
}

/// The reaches earlier reviews of #484 found read past.
fn found_earlier() -> Vec<Row> {
    vec![
        Row {
            shape: "a tool of another MCP server listed without its server",
            plain: PLAIN_READS_THE_PLANT,
            boxed: r#"tools='"mcp__brokkr__workspace","mcp__brokkr-probe-user-scope__probe"'; servers='{"name":"brokkr","status":"connected"}'"#.to_string(),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 2)),
                connected(&listed(INIT, "mcp_servers", 1)),
                leaked_plain(),
                reached_in_the_box(&tool_at_init()),
            ),
        },
        planted_tool_later(
            "a later tool listing holding another server's tool",
            r#"{"type":"system","subtype":"init","tools":["mcp__brokkr-probe-user-scope__probe"]}"#,
            "/tools/0",
        ),
        Row {
            shape: "a later server listing naming the planted server",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr-probe-user-scope","status":"connected"}]}"#),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                connected(&two_server_listings()),
                leaked_plain(),
                reached_in_the_box(&format!("{PLANTED_SERVER} {LATER} at /mcp_servers/0")),
            ),
        },
        Row {
            shape: "the planted server listed beside the hands server",
            plain: PLAIN_READS_THE_PLANT,
            boxed: r#"tools='"mcp__brokkr__workspace"'; servers='{"name":"brokkr","status":"connected"},{"name":"brokkr-probe-user-scope","status":"connected"}'"#.to_string(),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                connected(&listed(INIT, "mcp_servers", 2)),
                leaked_plain(),
                reached_in_the_box(&format!("{PLANTED_SERVER} {INIT} at /mcp_servers/1")),
            ),
        },
    ]
}

/// A review's shape that reaches nothing but disagrees with itself, and
/// the clean stream beside it.
fn disagreeing_and_clean() -> Vec<Row> {
    let two_listings = two_server_listings();
    vec![
        Row {
            shape: "two server listings giving the hands server two statuses",
            plain: PLAIN_CLEAN,
            boxed: later(
                r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr","status":"failed"}]}"#,
            ),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                unmeasured(&format!(
                    "the turn's listings gave brokkr the statuses connected, failed: {two_listings}"
                )),
                measured(json!(false), &listed(INIT, "mcp_servers", 0)),
                kept_out_of_the_box(
                    &two_listings,
                    json!({
                        "verdict": "unboxed-only",
                        "reason": format!(
                            "it is not shown to stand behind the box ({}; the turn's listings \
                             gave brokkr the statuses connected, failed: {two_listings}), and \
                             the hands argv removed WebSearch, WebFetch",
                            listed(INIT, "tools", 1)
                        ),
                    }),
                ),
            ),
        },
        Row {
            shape: "a clean stream",
            plain: PLAIN_CLEAN,
            boxed: BOXED_CLEAN.to_string(),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                connected(&listed(INIT, "mcp_servers", 1)),
                measured(json!(false), &listed(INIT, "mcp_servers", 0)),
                kept_out_of_the_box(
                    &listed(INIT, "mcp_servers", 1),
                    json!({
                        "verdict": "boxed",
                        "reason": "its own tools switch off and the hands MCP server connects",
                    }),
                ),
            ),
        },
    ]
}

/// The lines and streams no review named that the invariant implies.
fn implied_lines_and_streams() -> Vec<Row> {
    let twice = unread(
        "tools",
        "line 2 of stdout names tools but is not one JSON object naming each key once",
    );
    let cut = unread(
        "mcp_servers",
        "line 2 of stdout names mcp_servers but is not one JSON object naming each key once",
    );
    let (a, b) = (
        format!("{TRANSCRIPT}/a.jsonl"),
        format!("{TRANSCRIPT}/b.jsonl"),
    );
    let disagreeing = format!(
        "{}, and {}, and {}",
        listed(INIT, "mcp_servers", 1),
        listed(&a, "mcp_servers", 1),
        listed(&b, "mcp_servers", 2)
    );
    let plain_transcript_reach = format!(
        "{PLANTED_SERVER} the system/init event on line 2 of \
         ~/.claude/projects/{{workdir}}/{{session}}.jsonl at /mcp_servers/0"
    );
    vec![
        Row {
            shape: "a key given twice in one event",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(
                r#"{"type":"system","subtype":"init","tools":["mcp__brokkr-probe-user-scope__probe"],"tools":[]}"#,
            ),
            expected: expect(
                unmeasured(&twice),
                connected(&listed(INIT, "mcp_servers", 1)),
                leaked_plain(),
                unread_in_the_box(&no_reach_but(&twice)),
            ),
        },
        Row {
            shape: "a listing line cut short",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(
                r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr-probe-user-scope""#,
            ),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                unmeasured(&cut),
                leaked_plain(),
                unread_in_the_box(&no_reach_but(&cut)),
            ),
        },
        Row {
            shape: "a reach only in a transcript of the plain turn",
            plain: r#"printf '%s\n' '{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr-probe-user-scope","status":"connected"}]}' >> "$dir/$sid.jsonl""#,
            boxed: BOXED_CLEAN.to_string(),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                connected(&listed(INIT, "mcp_servers", 1)),
                measured(json!(true), &plain_transcript_reach),
                kept_out_of_the_box(
                    &listed(INIT, "mcp_servers", 1),
                    json!({
                        "verdict": "boxed-only",
                        "reason": format!(
                            "its own tools switch off and the hands MCP server connects, but \
                             {PLAIN_LEAK}{plain_transcript_reach}, so it may hold boxed offices \
                             only"
                        ),
                    }),
                ),
            ),
        },
        Row {
            shape: "two transcripts of the boxed turn that disagree",
            plain: PLAIN_READS_THE_PLANT,
            boxed: boxed(&format!(
                "{}; {}",
                written(
                    "a.jsonl",
                    r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr","status":"connected"}]}"#
                ),
                written(
                    "b.jsonl",
                    r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"brokkr","status":"failed"},{"name":"brokkr-probe-user-scope","status":"connected"}]}"#
                ),
            )),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                unmeasured(&format!(
                    "the turn's listings gave brokkr the statuses connected, failed: {disagreeing}"
                )),
                leaked_plain(),
                reached_in_the_box(&format!("{PLANTED_SERVER} {b} at /mcp_servers/1")),
            ),
        },
    ]
}

/// The empty, null and nested values no review named that the invariant
/// implies.
fn implied_values() -> Vec<Row> {
    let null = unread(
        "mcp_servers",
        &format!("{LATER} holds at /mcp_servers a value that is not a list: null"),
    );
    let nested = unread(
        "tools",
        &format!("{LATER} holds an entry at /tools/0 the probe cannot name"),
    );
    vec![
        Row {
            shape: "an empty list beside a reach",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(
                r#"{"type":"system","subtype":"init","tools":[],"mcp_servers":[{"name":"brokkr-probe-user-scope","status":"connected"}]}"#,
            ),
            expected: expect(
                tools_emptied(&format!(
                    "{}, and {}",
                    listed(INIT, "tools", 1),
                    listed(LATER, "tools", 0)
                )),
                connected(&format!(
                    "{}, and {}",
                    listed(INIT, "mcp_servers", 1),
                    listed(LATER, "mcp_servers", 1)
                )),
                leaked_plain(),
                reached_in_the_box(&format!("{PLANTED_SERVER} {LATER} at /mcp_servers/0")),
            ),
        },
        Row {
            shape: "a null where a server listing belongs",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(r#"{"type":"system","subtype":"init","mcp_servers":null}"#),
            expected: expect(
                tools_emptied(&listed(INIT, "tools", 1)),
                unmeasured(&null),
                leaked_plain(),
                unread_in_the_box(&no_reach_but(&null)),
            ),
        },
        Row {
            shape: "a list nested in a tool listing",
            plain: PLAIN_READS_THE_PLANT,
            boxed: later(
                r#"{"type":"system","subtype":"init","tools":[["mcp__brokkr-probe-user-scope__probe"]]}"#,
            ),
            expected: expect(
                unmeasured(&nested),
                connected(&listed(INIT, "mcp_servers", 1)),
                leaked_plain(),
                unread_in_the_box(&no_reach_but(&nested)),
            ),
        },
        planted_tool_later(
            "a tool listing nested in an array of objects",
            r#"{"type":"system","subtype":"init","agents":[{"tools":["mcp__brokkr-probe-user-scope__probe"]}]}"#,
            "/agents/0/tools/0",
        ),
    ]
}

#[test]
fn every_listing_a_turn_gives_is_read_whole_and_a_reach_read_anywhere_refuses() {
    let world = world();
    let rows = [
        found_by_the_chief(),
        found_earlier(),
        disagreeing_and_clean(),
        implied_values(),
        implied_lines_and_streams(),
    ];
    for (index, row) in rows.into_iter().flatten().enumerate() {
        let cli = world.fake(
            &format!("claude-{index}"),
            &claude_with("9.9.9", row.plain, &row.boxed),
        );
        let report = probe(AdapterKind::Claude, &cli, &claude_declared(), &world);
        assert_eq!((row.shape, view(&report)), (row.shape, row.expected));
    }
}
