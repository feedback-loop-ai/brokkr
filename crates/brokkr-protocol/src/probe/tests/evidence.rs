//! The one rule at the verdict (#484): an admitting verdict stands only
//! when every line of every turn's streams was read and every fact it
//! rests on was measured. The chief's shapes on 891c3c9f, 9c899c39,
//! 8acbbecb and 25a0ea04, each run through the probe against a
//! Claude-like fake, and the clean streams that keep their admitting
//! verdicts.

use serde_json::{json, Value};

use super::listings::{PLANTED_INIT, PLANTED_SERVER};
use super::*;
use crate::probe::measure::read::Harness;

/// A clean plain turn: its own tool is `Bash`, and no MCP server reached
/// it.
const PLAIN_BASH: &str = r#"tools='"Bash"'"#;

/// The session transcript both turns write, and its path in evidence.
const SESSION: &str = "~/.claude/projects/{workdir}/{session}.jsonl";

/// A clean plain turn listing Claude Code's two network tools beside its
/// shell, which the declared OFF controls remove.
const PLAIN_WEB: &str = r#"tools='"Bash","WebSearch","WebFetch"'"#;

const BOXED: &str = "boxed offices";
const GRANTED: &str = "a seat in a realm that grants its capabilities";

/// One stream shape: the fake's shell on its plain turn and under the
/// hands argv, and the eligibility the probe must derive.
struct Row {
    shape: &'static str,
    plain: String,
    boxed: String,
    eligibility: Value,
}

fn refused(admits: &str, gaps: &[&str]) -> Value {
    json!({
        "verdict": "refused",
        "reason": format!("the evidence for {admits} is not complete: {}", gaps.join("; ")),
    })
}

const NOT_ONE_OBJECT: &str = "is not one JSON object naming each key once";

/// The refusal of a harness proposed for a granting realm, whose plain
/// turn listed `listed` tools, none of them a native capability's, with
/// `gaps` before its two off switches: no control was seen switching a
/// capability off that the plain turn never listed (#484), so neither
/// switch is measured.
fn ungranted(listed: usize, gaps: &[String]) -> Value {
    let switches = [("web-fetch", "WebFetch"), ("web-search", "WebSearch")].map(|(power, tool)| {
        format!(
            "{power}'s off switch is unmeasured: the plain turn listed none of {tool}, so no \
             control was seen switching it off: the system/init event on line 1 of stdout \
             listed tools: {listed}"
        )
    });
    let all = [gaps, &switches].concat();
    refused(GRANTED, &all.iter().map(String::as_str).collect::<Vec<_>>())
}

/// The refusal of a harness below the box, whose boxed turn's line `line`
/// of `source` went unread as `fault` says, and whose plain turn listed
/// `Bash` alone.
fn boxed_line_unread(line: usize, source: &str, fault: &str) -> Value {
    ungranted(
        1,
        &[format!("line {line} of the boxed turn's {source} {fault}")],
    )
}

/// The chief's four shapes on 891c3c9f.
fn found_by_the_chief_on_891c3c9f() -> Vec<Row> {
    vec![
        Row {
            shape: "a plain init with no tools key beside a clean box",
            plain: r#"printf '{"type":"system","subtype":"init","session_id":"%s","mcp_servers":[]}\n' "$sid"; exit 0"#.to_string(),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: refused(
                BOXED,
                &["tools is unmeasured: no event of the turn listed its tools"],
            ),
        },
        Row {
            shape: "a whitespace line after the boxed init, after a clean plain turn",
            plain: PLAIN_BASH.to_string(),
            boxed: format!("{BOXED_CLEAN}; later=' '"),
            eligibility: boxed_line_unread(2, "stdout", NOT_ONE_OBJECT),
        },
        Row {
            shape: "a boxed event joined to the plain turn's unfinished line",
            plain: format!(r#"{PLAIN_BASH}; printf '{{"type":"user"}}' >> "$dir/$sid.jsonl""#),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: boxed_line_unread(2, SESSION, NOT_ONE_OBJECT),
        },
        Row {
            shape: "a raw 0xE9 byte inside a string of the boxed init",
            plain: PLAIN_BASH.to_string(),
            boxed: r#"tools='"mcp__brokkr__workspace"'; servers=$(printf '{"name":"brokkr","status":"connected","note":"\351"}')"#.to_string(),
            eligibility: boxed_line_unread(1, "stdout", "is not UTF-8"),
        },
    ]
}

/// A clean plain turn beside a boxed turn that appends a whitespace line
/// to the session transcript, and the clean streams, read whole and
/// measured, that keep their admitting verdicts.
fn clean_plain_turns() -> Vec<Row> {
    vec![
        Row {
            shape: "a whitespace line the box appends to the session, after a clean plain turn",
            plain: PLAIN_BASH.to_string(),
            boxed: format!(r#"{BOXED_CLEAN}; printf '%s\n' ' ' >> "$dir/$sid.jsonl""#),
            eligibility: boxed_line_unread(3, SESSION, NOT_ONE_OBJECT),
        },
        Row {
            shape: "a clean stream, read whole and measured",
            plain: PLAIN_BASH.to_string(),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: json!({
                "verdict": "boxed",
                "reason": "its own tools switch off and the hands MCP server connects",
            }),
        },
        Row {
            shape: "a box that keeps Bash, its refusal read whole",
            plain: PLAIN_WEB.to_string(),
            boxed: r#"tools='"Bash","mcp__brokkr__workspace"'; servers='{"name":"brokkr","status":"connected"}'"#.to_string(),
            eligibility: unboxed(
                "the system/init event on line 1 of stdout listed tools: 2; the system/init event \
                 on line 1 of stdout listed mcp_servers: 1",
            ),
        },
    ]
}

/// A plain turn with no tool of its own, which no MCP server reached.
const PLAIN_TOOLLESS: &str = "tools=''";

/// The refusal of a harness whose boxed turn was read reaching `reach`.
fn reached_the_box(reach: &str) -> Value {
    json!({
        "verdict": "refused",
        "reason": format!("{NOT_ISOLATED}{OTHER_SERVER} reached the boxed turn: {reach}"),
    })
}

/// A harness whose box keeps `Bash`, or whose CLI refuses the box, as
/// `behind` evidences, and whose plain turn's native egress the declared
/// OFF controls remove: its own shell is no native capability a realm
/// grants (decision 0065 ruling 1), so it may hold unboxed offices.
fn unboxed(behind: &str) -> Value {
    json!({
        "verdict": "unboxed-only",
        "reason": format!(
            "it is not shown to stand behind the box ({behind}), and the declared OFF controls \
             removed WebSearch, WebFetch"
        ),
    })
}

/// The chief's shapes on 9c899c39: a boxed turn that failed after it
/// printed, each line of which is read whatever its exit, and a listing
/// value no reader names.
fn found_by_the_chief_on_9c899c39() -> Vec<Row> {
    let leak = format!("printf '%s\\n' '{PLANTED_INIT}'");
    let overloaded = format!(r#"{leak}; echo "API Error: 529 overloaded" >&2; exit 1"#);
    let listed =
        format!("{PLANTED_SERVER} the system/init event on line 1 of stdout at /mcp_servers/0");
    vec![
        Row {
            shape: "a boxed init listing the planted server, then exit 1, beside a plain Bash",
            plain: PLAIN_BASH.to_string(),
            boxed: overloaded.clone(),
            eligibility: reached_the_box(&listed),
        },
        Row {
            shape: "a boxed init listing the planted server, then exit 1, beside no plain tool",
            plain: PLAIN_TOOLLESS.to_string(),
            boxed: overloaded,
            eligibility: reached_the_box(&listed),
        },
        Row {
            shape: "a boxed JSON line cut short, then exit 1",
            plain: PLAIN_BASH.to_string(),
            boxed: r#"printf '%s\n' '{"type":"system","subtype":"init","mcp_ser'; exit 1"#
                .to_string(),
            eligibility: boxed_line_unread(1, "stdout", NOT_ONE_OBJECT),
        },
        Row {
            shape: "a boxed init listing the planted server, then killed, beside no plain tool",
            plain: PLAIN_TOOLLESS.to_string(),
            boxed: format!("{leak}; kill -KILL $$"),
            eligibility: reached_the_box(&listed),
        },
        Row {
            shape: "a boxed server listing whose entry is a bare string, beside no plain tool",
            plain: PLAIN_TOOLLESS.to_string(),
            boxed: r#"tools='"mcp__brokkr__workspace"'; servers='"brokkr"'"#.to_string(),
            eligibility: ungranted(
                0,
                &[format!("line 1 of the boxed turn's stdout {UNDECODED}")],
            ),
        },
    ]
}

/// Stderr, read as text line by line: a line that is not UTF-8 is unread,
/// one that is a JSON object is an event, and any other is unread unless
/// it is one of claude's forms, whatever it names (#484).
fn on_stderr() -> Vec<Row> {
    let boxed = |stderr: &str| format!("{BOXED_CLEAN}; {stderr} >&2");
    vec![
        Row {
            shape: "a boxed stderr line that is not UTF-8",
            plain: PLAIN_BASH.to_string(),
            boxed: boxed(r"printf 'warn: \351t\351\n'"),
            eligibility: boxed_line_unread(1, "stderr", "is not UTF-8"),
        },
        Row {
            shape: "an init naming the planted server on the boxed stderr",
            plain: PLAIN_BASH.to_string(),
            boxed: boxed(&format!("printf '%s\\n' '{PLANTED_INIT}'")),
            eligibility: reached_the_box(&format!(
                "{PLANTED_SERVER} the system/init event on line 1 of stderr at /mcp_servers/0"
            )),
        },
        Row {
            shape: "a plain stderr line that is not UTF-8",
            plain: format!(r"{PLAIN_BASH}; printf '\351\n' >&2"),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: refused(
                "boxed offices only",
                &[
                    "line 1 of the plain turn's stderr is not UTF-8",
                    "line 1 of the OFF turn's stderr is not UTF-8",
                    "tools is unmeasured: the turn's tools could not be read whole: line 1 of \
                     stderr is not UTF-8",
                ],
            ),
        },
        Row {
            shape: "a boxed stderr warning naming the planted server",
            plain: PLAIN_BASH.to_string(),
            boxed: boxed(r#"echo "warn: MCP server brokkr-probe-user-scope was skipped""#),
            eligibility: unrecognised(&[1]),
        },
        Row {
            shape: "a boxed stderr warning naming another server's tool",
            plain: PLAIN_BASH.to_string(),
            boxed: boxed(
                r#"echo "warn: mcp__ prefixes mcp__brokkr__workspace and mcp__github__search""#,
            ),
            eligibility: unrecognised(&[1]),
        },
    ]
}

/// A row whose plain turn lists `Bash` alone and whose boxed turn prints
/// what `stderr` echoes beside a clean init.
fn row(shape: &'static str, stderr: &str, eligibility: Value) -> Row {
    Row {
        shape,
        plain: PLAIN_BASH.to_string(),
        boxed: format!("{BOXED_CLEAN}; {stderr} >&2"),
        eligibility,
    }
}

/// The refusal of a harness whose boxed stderr's `lines` went unread as
/// unrecognised.
fn unrecognised(lines: &[usize]) -> Value {
    let gaps = lines
        .iter()
        .map(|line| format!("line {line} of the boxed turn's stderr {UNRECOGNISED}"));
    ungranted(1, &gaps.collect::<Vec<_>>())
}

/// The refusal of a harness whose boxed turn's line `line` of `source`
/// is a JSON object claude's reader does not decode.
fn undecoded(line: usize, source: &str) -> Value {
    boxed_line_unread(line, source, UNDECODED)
}

/// The chief's eight shapes on 25a0ea04 (#484): a line of the boxed
/// stderr is read only when it is one of claude's forms whole, its
/// whitespace runs folded, whatever it names.
fn found_by_the_chief_on_25a0ea04() -> Vec<Row> {
    vec![
        row(
            "two spaces inside MCP server",
            r#"echo "MCP  server github connected""#,
            unrecognised(&[1]),
        ),
        row(
            "a tab inside MCP server",
            r#"printf 'MCP\tserver github connected\n'"#,
            unrecognised(&[1]),
        ),
        row(
            "a tool listing in other words",
            r#"echo "Tools available: WebSearch, WebFetch""#,
            unrecognised(&[1]),
        ),
        row(
            "a hyphen inside MCP server",
            r#"echo "mcp-server github connected""#,
            unrecognised(&[1]),
        ),
        row(
            "MCP and a colon before the server",
            r#"echo "MCP: connected to github""#,
            unrecognised(&[1]),
        ),
        row(
            "an MCP integration loaded",
            r#"echo "Loaded 1 MCP integration (github)""#,
            unrecognised(&[1]),
        ),
        row(
            "enabled tools in a sentence",
            r#"echo "Enabled tools are WebSearch and WebFetch""#,
            unrecognised(&[1]),
        ),
        row(
            "the hands server named first beside another",
            r#"echo "MCP server brokkr and github connected""#,
            unrecognised(&[1]),
        ),
        row(
            "a declared tool named with no mention of tools",
            r#"echo "Enabled: WebSearch""#,
            unrecognised(&[1]),
        ),
    ]
}

/// The chief's shapes on 8acbbecb: a disclosure on stderr in a form the
/// reader does not recognise refuses, never reads as harmless (#484).
fn found_by_the_chief_on_8acbbecb() -> Vec<Row> {
    let over_lines = [
        (1, UNRECOGNISED),
        (2, UNRECOGNISED),
        // One JSON object, which is not one of claude's events.
        (3, UNDECODED),
        (4, UNRECOGNISED),
        (5, UNRECOGNISED),
    ];
    let over_lines =
        over_lines.map(|(line, fault)| format!("line {line} of the boxed turn's stderr {fault}"));
    vec![
        row(
            "a boxed stderr line naming another MCP server connected",
            r#"echo "MCP server github connected""#,
            unrecognised(&[1]),
        ),
        row(
            "a server listing cut short on the boxed stderr",
            r#"printf '%s\n' '{"mcp_servers":[{"name":"github"'"#,
            unrecognised(&[1]),
        ),
        row(
            "a server listing over several lines of the boxed stderr",
            r#"printf '%s\n' '{' '  "mcp_servers": [' '    {"name": "github"}' '  ]' '}'"#,
            ungranted(1, &over_lines),
        ),
        row(
            "a server listing as text on the boxed stderr",
            r#"echo "MCP servers: github, brokkr""#,
            unrecognised(&[1]),
        ),
        row(
            "a tool listing as text on the boxed stderr",
            r#"echo "tools: Bash, WebSearch""#,
            unrecognised(&[1]),
        ),
        row(
            "an MCP tool whose server the boxed stderr does not spell whole",
            r#"echo "warn: mcp__github is unavailable""#,
            unrecognised(&[1]),
        ),
        row(
            "an MCP server the boxed stderr does not name",
            r#"echo "MCP server: (unnamed) failed""#,
            unrecognised(&[1]),
        ),
    ]
}

/// The chief's shapes on 5f1623d9 (#484): neither JSON syntax, nor case,
/// nor punctuation makes a disclosure harmless. A JSON line is an event
/// claude's reader decodes, on stderr and on stdout alike, or unread.
fn found_by_the_chief_on_5f1623d9() -> Vec<Row> {
    let github = r#"{"level":"info","message":"MCP server github connected"}"#;
    vec![
        row(
            "p1: a JSON log line naming another server connected",
            &format!("printf '%s\\n' '{github}'"),
            undecoded(1, "stderr"),
        ),
        row(
            "p1b: a JSON log line listing tools",
            r#"printf '%s\n' '{"level":"warn","msg":"Tools available: WebSearch, WebFetch"}'"#,
            undecoded(1, "stderr"),
        ),
        row(
            "p1c: a JSON log line naming the planted server",
            r#"printf '%s\n' '{"level":"info","message":"loaded user MCP server brokkr-probe-user-scope"}'"#,
            undecoded(1, "stderr"),
        ),
        Row {
            shape: "q5: the same JSON log line as an event of the boxed stdout",
            plain: PLAIN_BASH.to_string(),
            boxed: format!("{BOXED_CLEAN}; later='{github}'"),
            eligibility: undecoded(2, "stdout"),
        },
        row(
            "q1: a declared tool ending a sentence",
            r#"echo "Enabled: WebSearch.""#,
            unrecognised(&[1]),
        ),
        row(
            "q1b: two declared tools, each ending a sentence",
            r#"echo "Loaded WebSearch. Loaded WebFetch.""#,
            unrecognised(&[1]),
        ),
        row(
            "q1d: declared tools hyphenated to a word",
            r#"echo "WebSearch-enabled, WebFetch-enabled""#,
            unrecognised(&[1]),
        ),
        row(
            "q1e: declared tools in lowercase",
            r#"echo "websearch and webfetch are enabled""#,
            unrecognised(&[1]),
        ),
    ]
}

/// The chief's rows on d77a2b6e (#484), each a JSON line beside a clean
/// boxed init: a type no reader knows, a key no struct names whatever its
/// value's type, and codex's own items on claude's stream. Each is
/// undecoded, on stderr and on stdout alike; no tag, id or key exempts it.
const ON_D77A2B6E: [(&str, &str); 16] = [
    ("t1", r#"{"type":"mcp.server.connected","id":"github"}"#),
    ("t2", r#"{"type":"mcp_server_connected","id":"github"}"#),
    ("t4", r#"{"type":"tool_enabled","id":"WebSearch"}"#),
    (
        "t5",
        r#"{"type":"log","body":{"role":"WebSearch-enabled"}}"#,
    ),
    ("t6", r#"{"message":"WebSearch-enabled"}"#),
    ("t7", r#"{"brokkr_probe_unknown_key":"WebSearch-enabled"}"#),
    (
        "u1",
        r#"{"type":"capabilities","WebSearch":true,"WebFetch":true}"#,
    ),
    (
        "u2",
        r#"{"type":"system","subtype":"features","features":{"web_search":{"enabled":true},"mcp":{"github":{"connected":true}}}}"#,
    ),
    ("u3", r#"{"mcp__github__search":true}"#),
    ("u4", r#"{"brokkr-probe-user-scope":{"connected":true}}"#),
    (
        "u5",
        r#"{"type":"mcp","servers_connected":2,"tools_enabled":14}"#,
    ),
    ("u6", r#"{"type":"connected","thread_id":"github"}"#),
    ("u7", r#"{"type":"github"}"#),
    ("u8", r#"{"role":"WebSearch"}"#),
    (
        "y1",
        r#"{"type":"item.started","item":{"id":"item_1","type":"web_search"}}"#,
    ),
    (
        "y2",
        r#"{"type":"item.completed","item":{"id":"item_2","type":"mcp_tool_call","exit_code":0}}"#,
    ),
];

/// [`ON_D77A2B6E`] on the boxed stderr and stdout, and the two lines of
/// text beside them: the bare disclosure (t8), and a count of tools
/// (z1, z2), which no listing read.
fn found_by_the_chief_on_d77a2b6e() -> Vec<Row> {
    let mut rows: Vec<Row> = ON_D77A2B6E
        .iter()
        .flat_map(|&(shape, event)| {
            [
                row(
                    shape,
                    &format!("printf '%s\\n' '{event}'"),
                    undecoded(1, "stderr"),
                ),
                Row {
                    shape,
                    plain: PLAIN_BASH.to_string(),
                    boxed: format!("{BOXED_CLEAN}; later='{event}'"),
                    eligibility: undecoded(2, "stdout"),
                },
            ]
        })
        .collect();
    rows.extend([
        row("t8", "echo 'WebSearch-enabled'", unrecognised(&[1])),
        row("z1", "echo 'warn: 12 tools available'", unrecognised(&[1])),
        Row {
            shape: "z2",
            plain: PLAIN_BASH.to_string(),
            boxed: format!(
                r#"{BOXED_CLEAN}; later='{{"level":"warn","message":"12 tools available"}}'"#
            ),
            eligibility: undecoded(2, "stdout"),
        },
    ]);
    rows
}

/// Clean CLIs that print on stderr, whose every line is read and whose
/// verdicts stand: blank lines, the reply and one of claude's events on
/// both turns, and a box the CLI refuses on stderr alone, whose refusal
/// stays the measurement.
fn clean_with_stderr() -> Vec<Row> {
    let refused = "the CLI refused the adapter's hands argv: exit 1: error: unknown option \
                   '--strict-mcp-config'";
    let read = r#"printf '%s\n' '' '   ' 'PROBE-OK' '{"type":"user"}' >&2"#;
    vec![
        Row {
            shape: "blank lines, the reply and an event on the stderr of both turns",
            plain: format!("{PLAIN_BASH}; {read}"),
            boxed: format!("{BOXED_CLEAN}; {read}"),
            eligibility: json!({
                "verdict": "boxed",
                "reason": "its own tools switch off and the hands MCP server connects",
            }),
        },
        Row {
            shape: "a hands argv refused on stderr alone",
            plain: PLAIN_WEB.to_string(),
            boxed: Boxed::Refuses.shell().to_string(),
            eligibility: unboxed(&format!("{refused}; {refused}")),
        },
    ]
}

/// A clean init listing `Bash`, which no MCP server reached.
const INIT: &str = r#"{"type":"system","subtype":"init","tools":["Bash"],"mcp_servers":[]}"#;

/// The reading of a plain turn that printed `stdout` and `stderr`, beside
/// other turns that printed [`INIT`] alone.
fn plain_reading(stdout: &str, stderr: &str) -> measure::Reading {
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    let observed = Observed {
        turn: observation(Some(0), stdout, stderr),
        ..observed(observation(Some(0), INIT, ""))
    };
    measure::reading(&plan, &observed, &[])
}

/// How a line on the plain turn's stderr is read beside [`INIT`]: unread
/// as unrecognised or undecoded, or read.
fn stderr_reads(line: &str) -> String {
    let reading = plain_reading(INIT, line);
    let unread = |fault| measure::Unread {
        turn: measure::TurnName::Plain,
        source: "stderr".to_string(),
        line: 1,
        fault,
    };
    match reading.unread.as_slice() {
        [] => "read".to_string(),
        [only] if *only == unread(measure::Fault::Unrecognised) => "unread".to_string(),
        [only] if *only == unread(measure::Fault::Undecoded(Harness::Claude)) => {
            "undecoded".to_string()
        }
        other => format!("{other:?}"),
    }
}

/// Each of claude's forms, read whole, and a line one word off each,
/// which is not (#484): a word's edge punctuation and case are folded
/// away, a slot holds one word and `{option}` a flag alone, a line of
/// punctuation alone is not blank, and any line no form holds is unread,
/// whatever it names.
#[test]
fn a_stderr_line_is_read_only_in_a_form_read_whole_to_its_end() {
    let tagged = "[claude-code:unrecognized_model] ";
    let rows = [
        ("error: unknown option '--mcp-config'", "read".to_string()),
        ("ERROR: Unknown Option `--mcp-config`.", "read".to_string()),
        ("error: unknown option '-m'", "read".to_string()),
        ("error: unknown option 'mcp-config'", "unread".to_string()),
        (
            "error: unknown option --mcp-config now",
            "unread".to_string(),
        ),
        ("Invalid API key · Please run /login", "read".to_string()),
        ("Invalid API key", "unread".to_string()),
        (
            "There's an issue with the selected model (x). It may not exist or you may not have \
             access to it. Run --model to pick a different model.",
            "read".to_string(),
        ),
        (
            "There's an issue with the selected model (x y). It may not exist or you may not \
             have access to it. Run --model to pick a different model.",
            "unread".to_string(),
        ),
        ("PROBE-OK", "read".to_string()),
        ("probe-ok.", "read".to_string()),
        ("PROBE-OK PROBE-OK", "unread".to_string()),
        ("   ", "read".to_string()),
        ("{", "unread".to_string()),
        ("[1, 2]", "unread".to_string()),
        (plan::PROMPT, "unread".to_string()),
        ("Warning: no stdin data received", "unread".to_string()),
        ("MCP server brokkr connected", "unread".to_string()),
        ("warn:   12 tools available", "unread".to_string()),
        ("warn:", "unread".to_string()),
        (r#"{"type":"user"}"#, "read".to_string()),
        (r#"{"type":"user","note":"x"}"#, "undecoded".to_string()),
    ];
    let tagged_rows = [
        (r#"{"model":"x","query_source":"sdk"}"#, "read"),
        (
            r#"{"model":"x","query_source":"sdk","scope":"user"}"#,
            "unread",
        ),
        (r#"{"model":"x""#, "unread"),
    ];
    let tagged_rows = tagged_rows.map(|(json, read)| (format!("{tagged}{json}"), read.to_string()));
    let rows = rows.map(|(line, read)| (line.to_string(), read));
    for (line, read) in rows.into_iter().chain(tagged_rows) {
        assert_eq!((stderr_reads(&line), &line), (read, &line));
    }
}

/// A listing entry is read only as its struct names it: a tool is a
/// name, and an MCP server its name, status and source, so any other
/// key an entry holds, or a subtype no reader knows, leaves the event
/// undecoded (#484).
#[test]
fn a_listing_entry_s_other_strings_and_a_tag_holding_prose_are_read_like_text() {
    let init = |tools: &str, servers: &str| {
        format!(
            r#"{{"type":"system","subtype":"init","tools":[{tools}],"mcp_servers":[{servers}]}}"#
        )
    };
    let shapes = [
        init(r#"{"name":"Bash","description":"Runs WebSearch"}"#, ""),
        init(
            r#""Bash""#,
            r#"{"name":"brokkr","status":"connected","scope":"user"}"#,
        ),
        r#"{"type":"system","subtype":"init: WebSearch enabled","tools":["Bash"],"mcp_servers":[]}"#
            .to_string(),
    ];
    let unread = |stdout: &str| {
        let reading = plain_reading(stdout, "");
        let line = measure::Unread {
            turn: measure::TurnName::Plain,
            source: "stdout".to_string(),
            line: 1,
            fault: measure::Fault::Undecoded(Harness::Claude),
        };
        reading.unread == [line]
    };
    assert_eq!(shapes.map(|stdout| unread(&stdout)), [true, true, true]);
    let read = init(
        r#""Bash""#,
        r#"{"name":"brokkr","status":"connected","source":"user"}"#,
    );
    assert_eq!(plain_reading(&read, "").unread, []);
}

/// The variants of one disclosure: the text in three cases, each bare
/// and punctuated at its edges.
fn variants(text: &str) -> Vec<String> {
    let cased = [text.to_string(), text.to_uppercase(), text.to_lowercase()];
    cased
        .iter()
        .flat_map(|text| {
            [
                text.clone(),
                format!("{text}."),
                format!("({text})"),
                format!("» {text}!"),
            ]
        })
        .collect()
}

/// `text` wrapped in JSON: in a log line under a key a reader knows
/// elsewhere, a tag or id key among them, and under one no reader knows;
/// in an otherwise clean init under each tag, id and session key; as a
/// type and a subtype; carried by a key, holding a bool and an object;
/// and nested in an object and a list.
fn wrapped(text: &str) -> Vec<String> {
    let quoted = serde_json::to_string(text).unwrap();
    let keys = [
        "message",
        "text",
        "type",
        "subtype",
        "role",
        "id",
        "session_id",
        "thread_id",
        "brokkr_probe_unknown_key",
    ];
    let logged = keys.map(|key| format!(r#"{{"level":"info","{key}":{quoted}}}"#));
    let in_init = keys
        .map(|key| format!(r#"{{"type":"system","subtype":"init","tools":[],"{key}":{quoted}}}"#));
    let shaped = [
        format!(r#"{{"type":{quoted}}}"#),
        format!(r#"{{"type":"system","subtype":{quoted}}}"#),
        format!(r#"{{"type":"system","subtype":"init",{quoted}:true}}"#),
        format!(r#"{{{quoted}:{{"connected":true}}}}"#),
        format!(r#"{{"data":{{"lines":[{quoted}]}}}}"#),
        format!(r#"{{"type":"log","note":{{"body":{quoted}}}}}"#),
    ];
    logged.into_iter().chain(in_init).chain(shaped).collect()
}

/// Every variant of the disclosures the chief found admitted (#484), as
/// a line of the plain turn's stderr and, wrapped in JSON, on its stderr
/// and as an event of its stdout: none is read as harmless, so the class
/// is closed, not the shapes. The clean init alone is read.
#[test]
fn no_variant_of_a_disclosure_in_case_punctuation_or_json_is_read_as_harmless() {
    let harmless = |reading: &measure::Reading| {
        reading.unread.is_empty() && reading.facts.user_mcp_unboxed.value() == Some(&false)
    };
    assert!(harmless(&plain_reading(INIT, "")));
    let disclosures = [
        "MCP server github connected",
        "Enabled: WebSearch",
        "Tools available: WebSearch, WebFetch",
        "loaded user MCP server brokkr-probe-user-scope",
        "websearch and webfetch are enabled",
        "12 tools available",
        "github",
        "WebSearch",
        "WebSearch-enabled",
        "mcp__github__search",
        "brokkr-probe-user-scope",
    ];
    let mut admitted = Vec::new();
    for variant in disclosures.into_iter().flat_map(variants) {
        let mut shapes = vec![(variant.clone(), plain_reading(INIT, &variant))];
        for event in wrapped(&variant) {
            let stdout = format!("{INIT}\n{event}");
            shapes.push((format!("stderr {event}"), plain_reading(INIT, &event)));
            shapes.push((format!("stdout {event}"), plain_reading(&stdout, "")));
        }
        let read = shapes.into_iter().filter(|(_, reading)| harmless(reading));
        admitted.extend(read.map(|(shape, _)| shape));
    }
    assert_eq!(admitted, Vec::<String>::new());
}

/// The chief's x1 and x2 on d77a2b6e (#484): the keys a shipped claude
/// stream ordinarily holds, its model, working directory, permission
/// mode, credential source and a message's model and stop reason, are
/// named by its reader, so a clean turn that carries them keeps its
/// verdict, where the scanner refused it.
#[test]
fn an_ordinary_key_a_shipped_harness_prints_is_read_and_keeps_a_clean_verdict() {
    let init = |more: &str, tools: &str, servers: &str| {
        format!(
            r#"{{"type":"system","subtype":"init",{more}"tools":[{tools}],"mcp_servers":[{servers}]}}"#
        )
    };
    let x1 = r#""model":"claude-sonnet-4-6","#;
    let x2 = r#""cwd":"/tmp/scratch","permissionMode":"default","apiKeySource":"none","#;
    let message = r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","stop_reason":"end_turn","content":[{"type":"text","text":"PROBE-OK"}],"usage":{"input_tokens":10,"output_tokens":2}}}"#;
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    let verdict = |more: &str| {
        let turn = |tools: &str, servers: &str| {
            let stdout = format!("{}\n{message}", init(more, tools, servers));
            Trial::Observed(observation(Some(0), &stdout, ""))
        };
        let observed = Observed {
            boxed: turn(
                r#""mcp__brokkr__workspace""#,
                r#"{"name":"brokkr","status":"connected"}"#,
            ),
            native_off: turn(r#""Bash""#, ""),
            ..observed(observation(
                Some(0),
                &format!("{}\n{message}", init(more, r#""Bash""#, "")),
                "",
            ))
        };
        let reading = measure::reading(&plan, &observed, &[]);
        (
            judge::eligibility(&reading.facts, &reading.unread),
            reading.unread,
        )
    };
    let boxed = Eligibility {
        verdict: Verdict::Boxed,
        reason: "its own tools switch off and the hands MCP server connects".to_string(),
    };
    assert_eq!(
        [verdict(x1), verdict(x2)],
        [(boxed.clone(), Vec::new()), (boxed, Vec::new())]
    );
}

#[test]
fn an_admitting_verdict_needs_every_line_of_both_turns_read_and_every_fact_it_rests_on_measured() {
    if !in_its_own_engine(
        "probe::tests::evidence::an_admitting_verdict_needs_every_line_of_both_turns_read_and_every_fact_it_rests_on_measured",
    ) {
        return;
    }
    let world = world();
    let rows = [
        found_by_the_chief_on_891c3c9f(),
        clean_plain_turns(),
        found_by_the_chief_on_9c899c39(),
        on_stderr(),
        found_by_the_chief_on_8acbbecb(),
        found_by_the_chief_on_25a0ea04(),
        found_by_the_chief_on_5f1623d9(),
        found_by_the_chief_on_d77a2b6e(),
        clean_with_stderr(),
    ];
    for (index, row) in rows.into_iter().flatten().enumerate() {
        let cli = world.fake(
            &format!("claude-{index}"),
            &claude_with("9.9.9", &row.plain, &row.boxed),
        );
        let report = probe(AdapterKind::Claude, &cli, &claude_declared(), &world);
        assert_eq!(
            (row.shape, &report["eligibility"]),
            (row.shape, &row.eligibility)
        );
    }
}
