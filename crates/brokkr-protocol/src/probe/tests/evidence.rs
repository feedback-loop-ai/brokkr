//! The one rule at the verdict (#484): an admitting verdict stands only
//! when every line of every turn's streams was read and every fact it
//! rests on was measured. The chief's shapes on 891c3c9f, 9c899c39,
//! 8acbbecb and 25a0ea04, each run through the probe against a
//! Claude-like fake, and the clean streams that keep their admitting
//! verdicts.

use serde_json::{json, Value};

use super::listings::{PLANTED_INIT, PLANTED_SERVER};
use super::*;

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
            plain: format!(r#"{PLAIN_BASH}; printf '{{}}' >> "$dir/$sid.jsonl""#),
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
                &["in the boxed turn, the system/init event on line 1 of stdout holds an entry at \
                   /mcp_servers/0 the probe cannot name"
                    .to_string()],
            ),
        },
    ]
}

/// Stderr, read as text line by line: a line that is not UTF-8 is unread,
/// one that is a JSON object is an event, and any other is searched for a
/// reach.
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
            eligibility: reached_the_box(
                "line 1 of stderr names the planted user-scope MCP server brokkr-probe-user-scope",
            ),
        },
        Row {
            shape: "a boxed stderr warning naming another server's tool",
            plain: PLAIN_BASH.to_string(),
            boxed: boxed(
                r#"echo "warn: mcp__ prefixes mcp__brokkr__workspace and mcp__github__search""#,
            ),
            eligibility: reached_the_box(
                "line 1 of stderr names mcp__github__search of the MCP server github",
            ),
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

/// The chief's eight shapes on 25a0ea04 (#484): a line of the boxed
/// stderr is harmless only when the scanner recognises it whole, its
/// whitespace runs folded, and every name in it is checked.
fn found_by_the_chief_on_25a0ea04() -> Vec<Row> {
    let github = || reached_the_box("line 1 of stderr names the MCP server github");
    vec![
        row(
            "two spaces inside MCP server",
            r#"echo "MCP  server github connected""#,
            github(),
        ),
        row(
            "a tab inside MCP server",
            r#"printf 'MCP\tserver github connected\n'"#,
            github(),
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
/// scanner did not recognise, which reaches or refuses, never reads as
/// harmless (#484).
fn found_by_the_chief_on_8acbbecb() -> Vec<Row> {
    vec![
        row(
            "a boxed stderr line naming another MCP server connected",
            r#"echo "MCP server github connected""#,
            reached_the_box("line 1 of stderr names the MCP server github"),
        ),
        row(
            "a server listing cut short on the boxed stderr",
            r#"printf '%s\n' '{"mcp_servers":[{"name":"github"'"#,
            unrecognised(&[1]),
        ),
        row(
            "a server listing over several lines of the boxed stderr",
            r#"printf '%s\n' '{' '  "mcp_servers": [' '    {"name": "github"}' '  ]' '}'"#,
            // Line 3 is one JSON object, whose name no reader consumes.
            unrecognised(&[1, 2, 3]),
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
        // Its words normalised, the line names a server, which reaches.
        row(
            "an MCP server the boxed stderr does not name",
            r#"echo "MCP server: (unnamed) failed""#,
            reached_the_box("line 1 of stderr names the MCP server unnamed"),
        ),
    ]
}

/// The chief's shapes on 5f1623d9 (#484): neither JSON syntax, nor case,
/// nor punctuation makes a disclosure harmless. A string an event holds
/// that no reader consumes is read like a line of text, on stderr and on
/// stdout alike, and a tool's name is folded before it is compared.
fn found_by_the_chief_on_5f1623d9() -> Vec<Row> {
    let github = r#"{"level":"info","message":"MCP server github connected"}"#;
    vec![
        row(
            "p1: a JSON log line naming another server connected",
            &format!("printf '%s\\n' '{github}'"),
            reached_the_box("line 1 of stderr names the MCP server github"),
        ),
        row(
            "p1b: a JSON log line listing tools",
            r#"printf '%s\n' '{"level":"warn","msg":"Tools available: WebSearch, WebFetch"}'"#,
            unrecognised(&[1]),
        ),
        row(
            "p1c: a JSON log line naming the planted server",
            r#"printf '%s\n' '{"level":"info","message":"loaded user MCP server brokkr-probe-user-scope"}'"#,
            reached_the_box(
                "line 1 of stderr names the planted user-scope MCP server brokkr-probe-user-scope",
            ),
        ),
        Row {
            shape: "q5: the same JSON log line as an event of the boxed stdout",
            plain: PLAIN_BASH.to_string(),
            boxed: format!("{BOXED_CLEAN}; later='{github}'"),
            eligibility: reached_the_box("line 2 of stdout names the MCP server github"),
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

/// Clean CLIs that print on stderr, whose every line is read and whose
/// verdicts stand: ordinary warnings in both turns, and a box the CLI
/// refuses on stderr alone, whose refusal stays the measurement.
fn clean_with_stderr() -> Vec<Row> {
    let refused = "the CLI refused the adapter's hands argv: exit 1: error: unknown option \
                   '--strict-mcp-config'";
    let warnings = r#"printf '%s\n' 'warn: only mcp__brokkr__workspace is allowed' '' '{"level":"warn"}' 'MCP server brokkr connected' 'warn: 2 tools available' >&2"#;
    vec![
        Row {
            shape: "ordinary warnings on the stderr of both turns",
            plain: format!(r#"{PLAIN_BASH}; echo "Warning: no stdin data received" >&2"#),
            boxed: format!("{BOXED_CLEAN}; {warnings}"),
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

/// How a line on the plain turn's stderr is read beside [`INIT`]: a
/// reach, unread as unrecognised, or read.
fn stderr_reads(line: &str) -> String {
    let reading = plain_reading(INIT, line);
    let unrecognised = measure::Unread::Line {
        turn: measure::TurnName::Plain,
        source: "stderr".to_string(),
        line: 1,
        fault: measure::Fault::Unrecognised,
    };
    match (reading.facts.user_mcp_unboxed, reading.unread) {
        (
            Fact::Measured {
                value: true,
                evidence,
            },
            _,
        ) => format!("reach: {evidence}"),
        (_, unread) if unread == [unrecognised] => "unread".to_string(),
        (_, unread) if unread.is_empty() => "read".to_string(),
        (_, unread) => format!("{unread:?}"),
    }
}

/// Each form the scanner reads whole, and a line one word off each, which
/// it does not (#484): a level opens a form, every name in a form is the
/// hands server's or a refused flag, a word's edge punctuation and case
/// are folded away, and any line no form holds is unread, whatever it
/// names.
#[test]
fn a_stderr_line_is_read_only_in_a_form_read_whole_to_its_end() {
    let rows = [
        ("Warning: only mcp__brokkr__workspace is allowed", "read"),
        ("ONLY mcp__brokkr__workspace, is allowed.", "read"),
        ("only mcp__brokkr__workspace is now allowed", "unread"),
        ("only Bash is allowed", "unread"),
        ("Bash is ready", "unread"),
        ("Read 3 files", "unread"),
        ("Warning: no stdin data received", "read"),
        ("warn:", "read"),
        ("MCP server brokkr connected", "read"),
        ("MCP server brokkr pending", "unread"),
        ("MCP server brokkr connected and github connected", "unread"),
        (
            "MCP server (unnamed) failed",
            "reach: line 1 of stderr names the MCP server unnamed",
        ),
        (
            "MCP server `github` connected",
            "reach: line 1 of stderr names the MCP server github",
        ),
        ("error: unknown option '--mcp-config'", "read"),
        ("error: unknown option '-mcp'", "unread"),
        ("error: unknown option '--mcp,x'", "unread"),
        ("warn:   12 tools available", "read"),
        ("tool-less turn ok", "unread"),
        ("[1, 2]", "unread"),
        (
            "warn: mcp__other__search\tfailed",
            "reach: line 1 of stderr names mcp__other__search of the MCP server other",
        ),
        (plan::PROMPT, "read"),
        (
            "reply with exactly probe-ok, and nothing else: use no tool",
            "read",
        ),
        (
            "Reply with exactly PROBE-OK and nothing else. Use any tool.",
            "unread",
        ),
        ("PROBE-OK", "read"),
    ];
    for (line, read) in rows {
        assert_eq!((line, stderr_reads(line)), (line, read.to_string()));
    }
}

/// A listing entry is its reader's only in the name, and the status, it
/// is read by: any other string an entry holds, of a tool or of an MCP
/// server, is read like text, and a tag key holding prose is no tag
/// (#484).
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
        let line = measure::Unread::Line {
            turn: measure::TurnName::Plain,
            source: "stdout".to_string(),
            line: 1,
            fault: measure::Fault::Unrecognised,
        };
        reading.unread == [line]
    };
    assert_eq!(shapes.map(|stdout| unread(&stdout)), [true, true, true]);
    assert!(!unread(&init(r#"{"name":"Bash"}"#, "")));
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

/// `text` wrapped in JSON: under a key the readers know, a tag key among
/// them, under one they do not, and nested in an object and a list.
fn wrapped(text: &str) -> Vec<String> {
    let text = serde_json::to_string(text).unwrap();
    let keys = ["message", "text", "type", "brokkr_probe_unknown_key"];
    let flat = keys.map(|key| format!(r#"{{"level":"info","{key}":{text}}}"#));
    let nested = [
        format!(r#"{{"data":{{"lines":[{text}]}}}}"#),
        format!(r#"{{"type":"log","note":{{"body":{text}}}}}"#),
    ];
    flat.into_iter().chain(nested).collect()
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
